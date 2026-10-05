//! EventFetcher: reads one stream to its last page, retrying throttling and
//! network failures (U3:BR2.1-BR2.3, BR3.2, BR3.3).

use serde::Serialize;

use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::gateway::{CloudWatchLogsGateway, GetLogEventsRequest};
use crate::paging::{PageCursor, PageDecision};
use crate::request::FetchRequest;
use crate::retry::{CallResult, Retrier, RetryCounter, StreamEnding};
use crate::time_range::TimeRange;

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
    /// Retries made for this stream, over all its requests (BR2.2).
    pub retry_count: u32,
    /// Completed or Failed.
    pub status: StreamStatus,
    /// The failure, only when `status` is Failed.
    pub failure: Option<ApiFailure>,
}

impl StreamFetchOutcome {
    /// A stream failed without calling the API, because the retries ran
    /// out on too many streams in a row (BR2.1, review R-09).
    pub fn skipped(log_stream_name: &str, failure: ApiFailure) -> Self {
        Self {
            log_stream_name: log_stream_name.to_string(),
            event_count: 0,
            page_count: 0,
            retry_count: 0,
            status: StreamStatus::Failed,
            failure: Some(failure),
        }
    }
}

/// How the fetch of one stream ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFetchEnd {
    /// Read to the last page.
    Completed,
    /// Failed for good; `exhausted` when the retries were used up.
    Failed {
        /// Whether a retryable failure used up every retry.
        exhausted: bool,
    },
    /// The abort was requested; no further API call was made.
    Aborted,
}

impl StreamFetchEnd {
    /// The ending as counted by the exhaustion streak; `None` when aborted.
    pub fn streak_ending(self) -> Option<StreamEnding> {
        match self {
            StreamFetchEnd::Completed => Some(StreamEnding::Succeeded),
            StreamFetchEnd::Failed { exhausted: true } => Some(StreamEnding::Exhausted),
            StreamFetchEnd::Failed { exhausted: false } => {
                Some(StreamEnding::FailedWithoutExhaustion)
            }
            StreamFetchEnd::Aborted => None,
        }
    }
}

/// Outcome of one stream and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamFetch {
    /// Counts, status and failure.
    pub outcome: StreamFetchOutcome,
    /// Completed, Failed or Aborted.
    pub end: StreamFetchEnd,
}

/// Reads `log_stream_name` page by page until the end of pages (U1:BR3.2)
/// with no limit on the number of events (U1:BR3.3).
///
/// Every call passes `startTime = startInstant`, `endTime = endInstant + 1`
/// and `startFromHead = true` (U1:BR3.1). Each page's events are numbered
/// in API order within the stream, from 0 (BR3.2), and handed to `on_page`
/// as soon as the page arrives. A failure is retried by `retrier`
/// (BR2.1, BR2.2); a failure that remains stops the stream, keeping the
/// pages already delivered (BR3.3). An abort stops without another call
/// (BR2.3).
pub async fn fetch_stream<G, F>(
    gateway: &G,
    request: &FetchRequest,
    range: &TimeRange,
    log_stream_name: &str,
    retrier: &Retrier<'_>,
    mut on_page: F,
) -> StreamFetch
where
    G: CloudWatchLogsGateway,
    F: FnMut(Vec<LogEvent>) + Send,
{
    let mut cursor = PageCursor::new();
    let mut counter = RetryCounter::new();
    let mut next_sequence: u64 = 0;
    let mut event_count: u64 = 0;
    loop {
        let call = GetLogEventsRequest {
            profile_name: request.profile_name().map(str::to_string),
            region: request.region().map(str::to_string),
            log_group_name: request.log_group_name().to_string(),
            log_stream_name: log_stream_name.to_string(),
            start_time: range.start_instant(),
            end_time: range.api_end_time(),
            start_from_head: true,
            next_token: cursor.token_to_send().map(str::to_string),
        };
        let page = match retrier
            .call(&mut counter, || gateway.get_log_events(call.clone()))
            .await
        {
            CallResult::Succeeded(page) => page,
            CallResult::Failed { failure, exhausted } => {
                let outcome = StreamFetchOutcome {
                    log_stream_name: log_stream_name.to_string(),
                    event_count,
                    page_count: cursor.page_count(),
                    retry_count: counter.total(),
                    status: StreamStatus::Failed,
                    failure: Some(failure),
                };
                return StreamFetch {
                    outcome,
                    end: StreamFetchEnd::Failed { exhausted },
                };
            }
            CallResult::Aborted => {
                let outcome = StreamFetchOutcome {
                    log_stream_name: log_stream_name.to_string(),
                    event_count,
                    page_count: cursor.page_count(),
                    retry_count: counter.total(),
                    status: StreamStatus::Failed,
                    failure: None,
                };
                return StreamFetch {
                    outcome,
                    end: StreamFetchEnd::Aborted,
                };
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
                    log_stream_name: log_stream_name.to_string(),
                    sequence,
                }
            })
            .collect();
        event_count += events.len() as u64;
        if !events.is_empty() {
            on_page(events);
        }
        if decision == PageDecision::Finished {
            let outcome = StreamFetchOutcome {
                log_stream_name: log_stream_name.to_string(),
                event_count,
                page_count: cursor.page_count(),
                retry_count: counter.total(),
                status: StreamStatus::Completed,
                failure: None,
            };
            return StreamFetch {
                outcome,
                end: StreamFetchEnd::Completed,
            };
        }
    }
}
