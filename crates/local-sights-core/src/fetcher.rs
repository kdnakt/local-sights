//! EventFetcher: reads one stream to its last page (U1).

use serde::Serialize;

use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::gateway::{CloudWatchLogsGateway, GetLogEventsRequest};
use crate::paging::{PageCursor, PageDecision};
use crate::request::ValidatedFetch;

/// Result status of one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StreamStatus {
    /// Read to the last page.
    Completed,
    /// Stopped by an error.
    Failed,
}

/// Result and paging progress of one stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamFetchOutcome {
    /// Stream name.
    pub log_stream_name: String,
    /// Number of events received.
    pub event_count: u64,
    /// Number of responses received, including the final same-token one.
    pub page_count: u32,
    /// Completed or Failed.
    pub status: StreamStatus,
    /// The failure, only when `status` is Failed.
    pub failure: Option<ApiFailure>,
}

/// Reads one stream page by page until the end of pages (BR3.2), with no
/// limit on the number of events (BR3.3).
///
/// Every call passes `startTime = startInstant`, `endTime = endInstant + 1`
/// and `startFromHead = true` (BR3.1). Each page's events are numbered in
/// API order (`sequence`, from 0 across the whole fetch) and handed to
/// `on_page` as soon as the page arrives. On an error the fetch stops and a
/// Failed outcome with the counts so far is returned; already delivered
/// pages stay delivered (BR4.1).
pub async fn fetch_stream<G, F>(
    gateway: &G,
    fetch: &ValidatedFetch,
    mut on_page: F,
) -> StreamFetchOutcome
where
    G: CloudWatchLogsGateway,
    F: FnMut(Vec<LogEvent>) + Send,
{
    let request = &fetch.request;
    let mut cursor = PageCursor::new();
    let mut next_sequence: u64 = 0;
    let mut event_count: u64 = 0;
    loop {
        let call = GetLogEventsRequest {
            profile_name: request.profile_name().map(str::to_string),
            log_group_name: request.log_group_name().to_string(),
            log_stream_name: request.log_stream_name().to_string(),
            start_time: fetch.range.start_instant(),
            end_time: fetch.range.api_end_time(),
            start_from_head: true,
            next_token: cursor.token_to_send().map(str::to_string),
        };
        let page = match gateway.get_log_events(call).await {
            Ok(page) => page,
            Err(failure) => {
                return outcome(fetch, event_count, &cursor, Some(failure));
            }
        };
        let decision = cursor.record_response(page.next_forward_token.as_deref());
        let events: Vec<LogEvent> = page
            .events
            .into_iter()
            .map(|event| {
                let sequence = next_sequence;
                next_sequence += 1;
                LogEvent {
                    timestamp: event.timestamp,
                    ingestion_time: event.ingestion_time,
                    message: event.message,
                    log_stream_name: request.log_stream_name().to_string(),
                    sequence,
                }
            })
            .collect();
        event_count += events.len() as u64;
        on_page(events);
        if decision == PageDecision::Finished {
            return outcome(fetch, event_count, &cursor, None);
        }
    }
}

fn outcome(
    fetch: &ValidatedFetch,
    event_count: u64,
    cursor: &PageCursor,
    failure: Option<ApiFailure>,
) -> StreamFetchOutcome {
    StreamFetchOutcome {
        log_stream_name: fetch.request.log_stream_name().to_string(),
        event_count,
        page_count: cursor.page_count(),
        status: if failure.is_some() {
            StreamStatus::Failed
        } else {
            StreamStatus::Completed
        },
        failure,
    }
}
