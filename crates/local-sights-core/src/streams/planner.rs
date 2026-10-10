//! Listing flow: reads `DescribeLogStreams` pages, newest last event first,
//! and builds the [`StreamPlan`] (U3:BR1.1-BR1.5). Each page is reported
//! to the caller as it arrives (BR5.4, review R-06). Failures are retried
//! (BR2.1); a failure that remains ends the listing as Partial (BR1.5).

use crate::gateway::{CloudWatchLogsGateway, DescribeLogStreamsRequest};
use crate::paging::PageDecision;
use crate::request::FetchRequest;
use crate::retry::{CallResult, Retrier, RetryCounter};
use crate::time_range::TimeRange;

use super::{StreamPlan, StreamSelection};

/// Progress after one listing page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListingProgress {
    /// Streams seen so far.
    pub seen_stream_count: u32,
    /// Streams selected so far.
    pub selected_stream_count: u32,
}

/// How the listing ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOutcome {
    /// The listing ended (Complete, StoppedEarly or Partial).
    Planned(StreamPlan),
    /// The abort was requested; no further call was made (BR5.5).
    Aborted,
}

/// Lists the streams of the requested log group and selects those that
/// matter for `range`. `on_page` receives the progress after every page.
pub async fn plan_streams<G, F>(
    gateway: &G,
    request: &FetchRequest,
    range: &TimeRange,
    retrier: &Retrier<'_>,
    mut on_page: F,
) -> PlanOutcome
where
    G: CloudWatchLogsGateway,
    F: FnMut(ListingProgress) + Send,
{
    let mut selection = StreamSelection::new(*range);
    loop {
        let call = DescribeLogStreamsRequest {
            profile_name: request.profile_name().map(str::to_string),
            region: request.region().map(str::to_string),
            log_group_name: request.log_group_name().to_string(),
            next_token: selection.token_to_send().map(str::to_string),
        };
        // Retries are counted per request (BR2.2).
        let mut counter = RetryCounter::new();
        let page = match retrier
            .call(&mut counter, || gateway.describe_log_streams(call.clone()))
            .await
        {
            CallResult::Succeeded(page) => page,
            CallResult::Failed { failure, .. } => {
                return PlanOutcome::Planned(selection.fail(failure));
            }
            CallResult::Aborted => return PlanOutcome::Aborted,
        };
        let decision = selection.apply_page(page.streams, page.next_token.as_deref());
        on_page(ListingProgress {
            seen_stream_count: selection.seen_count(),
            selected_stream_count: selection.selected_count(),
        });
        if decision == PageDecision::Finished {
            return PlanOutcome::Planned(selection.into_plan());
        }
    }
}
