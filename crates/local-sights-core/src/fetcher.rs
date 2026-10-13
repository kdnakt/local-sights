//! EventFetcher: reads one stream page by page, retrying throttling and
//! network failures (U3:BR2.1-BR2.3, BR3.2, BR3.3).
//!
//! Since 261012-parallel-fetch U1 a stream is read one page at a time
//! ([`next_page`], functional-spec 3): the reading state lives in a
//! [`StreamReader`] value that each read takes and gives back, so the
//! FetchCoordinator can keep up to eight streams' next pages going side by
//! side in one task and hand each page on before asking for the next one
//! (U1:BR1.10, BR1.11). Every `GetLogEvents` attempt waits for a permit of
//! the fetch's [`CallPacer`] first (U1:BR1.4).

use serde::Serialize;

use crate::event::LogEvent;
use crate::failure::{ApiFailure, FailureKind};
use crate::fetch_stats::FetchStats;
use crate::gateway::{CloudWatchLogsGateway, GatewayEvent, GetLogEventsPage, GetLogEventsRequest};
use crate::pacing::CallPacer;
use crate::paging::PageCursor;
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

/// The reading state of one stream between two pages (functional-spec 3):
/// where the paging is, the next number inside the stream and the counts.
#[derive(Debug, Clone)]
pub struct StreamReader {
    log_stream_name: String,
    cursor: PageCursor,
    counter: RetryCounter,
    next_sequence: u64,
    event_count: u64,
}

impl StreamReader {
    /// A reader at the start of `log_stream_name`.
    pub fn new(log_stream_name: &str) -> Self {
        Self {
            log_stream_name: log_stream_name.to_string(),
            cursor: PageCursor::new(),
            counter: RetryCounter::new(),
            next_sequence: 0,
            event_count: 0,
        }
    }

    /// Stream name.
    pub fn log_stream_name(&self) -> &str {
        &self.log_stream_name
    }

    /// Events read so far.
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// Retries, attempts and throttled attempts so far.
    pub fn counter(&self) -> &RetryCounter {
        &self.counter
    }

    /// The stream's end, with its counts.
    fn finish(
        &self,
        status: StreamStatus,
        failure: Option<ApiFailure>,
        end: StreamFetchEnd,
    ) -> StreamFetch {
        StreamFetch {
            outcome: StreamFetchOutcome {
                log_stream_name: self.log_stream_name.clone(),
                event_count: self.event_count,
                page_count: self.cursor.page_count(),
                retry_count: self.counter.total(),
                status,
                failure,
            },
            end,
        }
    }

    /// Numbers a page's events in API order inside the stream (BR3.2).
    fn number(&mut self, events: Vec<GatewayEvent>) -> Vec<LogEvent> {
        let events: Vec<LogEvent> = events
            .into_iter()
            .map(|event| {
                let sequence = self.next_sequence;
                self.next_sequence += 1;
                LogEvent {
                    timestamp: event.timestamp,
                    ingestion_time: event.ingestion_time,
                    message: event.message,
                    log_stream_name: self.log_stream_name.clone(),
                    sequence,
                }
            })
            .collect();
        self.event_count += events.len() as u64;
        events
    }
}

/// Result of reading the next page of one stream (entities.md PageStep).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageStep {
    /// One page of numbered events, never empty.
    Page(Vec<LogEvent>),
    /// The stream ended: Completed, Failed or Aborted.
    Finished(StreamFetch),
}

/// What every page read of one fetch shares: the gateway, the request and
/// range, the retry rules with the stop signal, the fetch's call pacer and
/// its counts.
#[derive(Clone, Copy)]
pub struct ReadContext<'a, G> {
    /// The AWS boundary.
    pub gateway: &'a G,
    /// The fetch's request (profile, region, log group).
    pub request: &'a FetchRequest,
    /// The fetch's range.
    pub range: &'a TimeRange,
    /// Retry rules and stop signal.
    pub retrier: &'a Retrier<'a>,
    /// The per-region ceiling shared by every stream of the fetch.
    pub pacer: &'a CallPacer,
    /// The fetch's counts (calls and throttled calls).
    pub stats: &'a FetchStats,
}

impl<G> std::fmt::Debug for ReadContext<'_, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadContext")
            .field("request", self.request)
            .field("range", self.range)
            .field("pacer", self.pacer)
            .finish_non_exhaustive()
    }
}

/// Reads the next page of `reader`'s stream (functional-spec 3) and gives
/// the reader back with the result.
///
/// Every call passes `startTime = startInstant`, `endTime = endInstant + 1`
/// and `startFromHead = true` (U1:BR3.1). Before every attempt, the first
/// one and every retry, a permit of the fetch's [`CallPacer`] is awaited
/// (261012-parallel-fetch U1:BR1.4, contract C1); each attempt is counted in
/// the fetch's [`FetchStats`] (NFR2.4). A failure is retried by the
/// retrier (BR2.1, BR2.2); a failure that remains ends the stream as
/// Failed, keeping the pages already returned (BR3.3). A stop, during a
/// permit wait or a retry wait, ends the stream as Aborted without another
/// call (U1:BR1.5).
///
/// The events of a page are numbered in API order within the stream, from
/// 0 (BR3.2). A page without events is not returned: the reader goes on to
/// the next page, or ends the stream when the pages end (U1:BR3.2). The
/// stream is read with no limit on the number of events (U1:BR3.3).
pub async fn next_page<G>(
    context: &ReadContext<'_, G>,
    mut reader: StreamReader,
) -> (StreamReader, PageStep)
where
    G: CloudWatchLogsGateway,
{
    let stop = context.retrier.stop_signal();
    loop {
        if reader.cursor.is_finished() {
            // The last page came with the previous page's events.
            let fetch = reader.finish(StreamStatus::Completed, None, StreamFetchEnd::Completed);
            return (reader, PageStep::Finished(fetch));
        }
        let call = GetLogEventsRequest {
            profile_name: context.request.profile_name().map(str::to_string),
            region: context.request.region().map(str::to_string),
            log_group_name: context.request.log_group_name().to_string(),
            log_stream_name: reader.log_stream_name.clone(),
            start_time: context.range.start_instant(),
            end_time: context.range.api_end_time(),
            start_from_head: true,
            next_token: reader.cursor.token_to_send().map(str::to_string),
        };
        let result = context
            .retrier
            .call(
                &mut reader.counter,
                || context.pacer.permit(stop),
                || counted_call(context, call.clone()),
            )
            .await;
        let page = match result {
            CallResult::Succeeded(page) => page,
            CallResult::Failed { failure, exhausted } => {
                let fetch = reader.finish(
                    StreamStatus::Failed,
                    Some(failure),
                    StreamFetchEnd::Failed { exhausted },
                );
                return (reader, PageStep::Finished(fetch));
            }
            CallResult::Aborted => {
                let fetch = reader.finish(StreamStatus::Failed, None, StreamFetchEnd::Aborted);
                return (reader, PageStep::Finished(fetch));
            }
        };
        reader
            .cursor
            .record_response(page.next_forward_token.as_deref());
        let events = reader.number(page.events);
        if !events.is_empty() {
            // When this was the last page, the next read ends the stream
            // without calling (the check at the top of the loop).
            return (reader, PageStep::Page(events));
        }
    }
}

/// One `GetLogEvents` attempt, counted in the fetch's stats (NFR2.4).
async fn counted_call<G>(
    context: &ReadContext<'_, G>,
    call: GetLogEventsRequest,
) -> Result<GetLogEventsPage, ApiFailure>
where
    G: CloudWatchLogsGateway,
{
    let result = context.gateway.get_log_events(call).await;
    let throttled = matches!(&result, Err(failure) if failure.kind() == FailureKind::Throttled);
    context.stats.record_attempt(throttled);
    result
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

    use tokio::time::Instant;

    use super::*;
    use crate::catalog::ProfileSelector;
    use crate::failure::SafeDetailFields;
    use crate::gateway::{
        DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
        DescribeLogStreamsRequest,
    };
    use crate::retry::{FixedJitter, RetryPolicy, StopReason, StopSignal, stop_pair};

    /// 2024/01/02 03:04:05 UTC.
    const START: i64 = 1_704_164_645_000;

    type Script = VecDeque<Result<GetLogEventsPage, ApiFailure>>;

    /// Answers `GetLogEvents` from a script and records when each call
    /// came and with which token. Never connects to AWS.
    #[derive(Default)]
    struct ScriptedLogs {
        script: Mutex<Script>,
        calls: Mutex<Vec<(Duration, Option<String>)>>,
        started: Option<Instant>,
    }

    impl ScriptedLogs {
        fn new(script: Vec<Result<GetLogEventsPage, ApiFailure>>) -> Self {
            Self {
                script: Mutex::new(script.into()),
                calls: Mutex::default(),
                started: Some(Instant::now()),
            }
        }

        fn calls(&self) -> Vec<(Duration, Option<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CloudWatchLogsGateway for ScriptedLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            let at = self
                .started
                .map(|started| started.elapsed())
                .unwrap_or_default();
            self.calls.lock().unwrap().push((at, request.next_token));
            self.script
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| panic!("more GetLogEvents calls than scripted"))
        }

        async fn describe_log_groups(
            &self,
            _request: DescribeLogGroupsRequest,
        ) -> Result<DescribeLogGroupsPage, ApiFailure> {
            panic!("a stream read never lists log groups");
        }

        async fn describe_log_streams(
            &self,
            _request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            panic!("a stream read never lists streams");
        }
    }

    fn page(events: &[(i64, &str)], token: Option<&str>) -> Result<GetLogEventsPage, ApiFailure> {
        Ok(GetLogEventsPage {
            events: events
                .iter()
                .map(|(timestamp, message)| GatewayEvent {
                    timestamp: *timestamp,
                    ingestion_time: None,
                    message: (*message).to_string(),
                })
                .collect(),
            next_forward_token: token.map(str::to_string),
        })
    }

    fn failure(kind: FailureKind) -> ApiFailure {
        ApiFailure::new(kind, &SafeDetailFields::default())
    }

    fn request(region: Option<&str>) -> FetchRequest {
        FetchRequest::new(
            ProfileSelector::Named("dev".to_string()),
            region.map(str::to_string),
            "/aws/lambda/orders".to_string(),
        )
    }

    /// Reads `stream` page by page to its end with the given stop signal;
    /// returns the pages (as messages) and the end.
    async fn read_all(
        gateway: &ScriptedLogs,
        region: Option<&str>,
        stop: &StopSignal,
        stats: &FetchStats,
    ) -> (Vec<Vec<(String, u64)>>, StreamFetch) {
        let policy = RetryPolicy::default();
        let jitter = FixedJitter(0.0);
        let retrier = Retrier::new(&policy, &jitter, stop);
        let request = request(region);
        let range = TimeRange::from_seconds(START / 1_000, START / 1_000 + 60).unwrap();
        let pacer = CallPacer::for_region(region);
        let context = ReadContext {
            gateway,
            request: &request,
            range: &range,
            retrier: &retrier,
            pacer: &pacer,
            stats,
        };
        let mut reader = StreamReader::new("s01");
        let mut pages = Vec::new();
        loop {
            let (next, step) = next_page(&context, reader).await;
            reader = next;
            match step {
                PageStep::Page(events) => pages.push(
                    events
                        .into_iter()
                        .map(|event| (event.message, event.sequence))
                        .collect(),
                ),
                PageStep::Finished(fetch) => return (pages, fetch),
            }
        }
    }

    fn texts(page: &[(&str, u64)]) -> Vec<(String, u64)> {
        page.iter().map(|(m, s)| ((*m).to_string(), *s)).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn pages_come_one_at_a_time_in_order_then_the_end() {
        let gateway = ScriptedLogs::new(vec![
            page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
            page(&[(START + 2, "c")], Some("f/2")),
            page(&[], Some("f/2")),
        ]);
        let stats = FetchStats::new();
        let (pages, fetch) = read_all(&gateway, None, &StopSignal::never(), &stats).await;
        assert_eq!(
            pages,
            vec![texts(&[("a", 0), ("b", 1)]), texts(&[("c", 2)])],
            "numbered inside the stream from 0 (BR3.2)"
        );
        assert_eq!(fetch.end, StreamFetchEnd::Completed);
        assert_eq!(fetch.outcome.status, StreamStatus::Completed);
        assert_eq!(fetch.outcome.event_count, 3);
        assert_eq!(fetch.outcome.page_count, 3);
        let tokens: Vec<Option<String>> = gateway.calls().into_iter().map(|(_, t)| t).collect();
        assert_eq!(
            tokens,
            vec![None, Some("f/1".to_string()), Some("f/2".to_string())]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_empty_page_is_not_handed_on_and_a_last_page_with_events_ends_without_a_call() {
        let gateway = ScriptedLogs::new(vec![page(&[], Some("f/1")), page(&[(START, "a")], None)]);
        let stats = FetchStats::new();
        let (pages, fetch) = read_all(&gateway, None, &StopSignal::never(), &stats).await;
        assert_eq!(pages, vec![texts(&[("a", 0)])]);
        assert_eq!(fetch.end, StreamFetchEnd::Completed);
        assert_eq!(gateway.calls().len(), 2, "no call after the last page");
        assert_eq!(stats.calls(), 2);

        let empty = ScriptedLogs::new(vec![page(&[], Some("f/1")), page(&[], Some("f/1"))]);
        let (pages, fetch) = read_all(&empty, None, &StopSignal::never(), &stats).await;
        assert!(pages.is_empty());
        assert_eq!(fetch.outcome.page_count, 2);
        assert_eq!(fetch.end, StreamFetchEnd::Completed);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_that_remains_ends_the_stream_as_failed_after_its_pages() {
        let gateway = ScriptedLogs::new(vec![
            page(&[(START, "a")], Some("f/1")),
            Err(failure(FailureKind::AccessDenied)),
        ]);
        let stats = FetchStats::new();
        let (pages, fetch) = read_all(&gateway, None, &StopSignal::never(), &stats).await;
        assert_eq!(pages.len(), 1, "the page before the failure is kept");
        assert_eq!(fetch.end, StreamFetchEnd::Failed { exhausted: false });
        assert_eq!(fetch.outcome.status, StreamStatus::Failed);
        assert_eq!(
            fetch.outcome.failure.as_ref().map(ApiFailure::kind),
            Some(FailureKind::AccessDenied)
        );
        assert_eq!(fetch.outcome.event_count, 1);

        let mut throttles: Vec<_> = (0..6)
            .map(|_| Err(failure(FailureKind::Throttled)))
            .collect();
        throttles.insert(0, page(&[(START, "a")], Some("f/1")));
        let exhausted = ScriptedLogs::new(throttles);
        let stats = FetchStats::new();
        let (_, fetch) = read_all(&exhausted, None, &StopSignal::never(), &stats).await;
        assert_eq!(fetch.end, StreamFetchEnd::Failed { exhausted: true });
        assert_eq!(fetch.outcome.retry_count, 5);
        assert_eq!((stats.calls(), stats.throttled()), (7, 6));
    }

    #[tokio::test(start_paused = true)]
    async fn a_stop_ends_the_stream_as_aborted_without_another_call() {
        let gateway = ScriptedLogs::new(vec![
            page(&[(START, "a")], Some("f/1")),
            Err(failure(FailureKind::Throttled)),
        ]);
        let (handle, signal) = stop_pair();
        let stats = FetchStats::new();
        let reading = read_all(&gateway, None, &signal, &stats);
        let stop_later = async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            handle.stop(StopReason::Close);
        };
        let ((pages, fetch), ()) = tokio::join!(reading, stop_later);
        assert_eq!(pages.len(), 1);
        assert_eq!(fetch.end, StreamFetchEnd::Aborted);
        assert_eq!(fetch.outcome.failure, None);
        assert_eq!(gateway.calls().len(), 2, "stopped during the retry wait");

        let untouched = ScriptedLogs::new(Vec::new());
        let (_, fetch) = read_all(&untouched, None, &signal, &stats).await;
        assert_eq!(fetch.end, StreamFetchEnd::Aborted);
        assert!(untouched.calls().is_empty(), "no call once stopped");
    }

    #[tokio::test(start_paused = true)]
    async fn every_attempt_waits_for_a_permit_first() {
        // No region: 5 calls per second, one permit every 200 ms. The
        // retry after the throttle waits its 1 s and then its permit.
        let gateway = ScriptedLogs::new(vec![
            page(&[(START, "a")], Some("f/1")),
            page(&[(START + 1, "b")], Some("f/2")),
            Err(failure(FailureKind::Throttled)),
            page(&[], Some("f/2")),
        ]);
        let stats = FetchStats::new();
        let (_, fetch) = read_all(&gateway, None, &StopSignal::never(), &stats).await;
        assert_eq!(fetch.end, StreamFetchEnd::Completed);
        let times: Vec<Duration> = gateway.calls().into_iter().map(|(at, _)| at).collect();
        assert_eq!(
            times,
            vec![
                Duration::ZERO,
                Duration::from_millis(200),
                Duration::from_millis(400),
                Duration::from_millis(1_400),
            ]
        );
        assert_eq!((stats.calls(), stats.throttled()), (4, 1));
    }
}
