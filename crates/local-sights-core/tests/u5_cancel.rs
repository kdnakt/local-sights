//! 261012-parallel-fetch U5: [Cancel] cuts a fetch off (u5-cancel,
//! nfr-design security-design 2.1). After the cancel no call starts
//! (NFR4.2), the waits and the calls under way end at once without the clock
//! moving (NFR4.3), the streams being read and those not started yet are
//! the unfinished streams N (BR5.4, NFR4.4, NFR4.6), a cancel before the
//! listing ended is Cancelled before listing and writes nothing to the
//! cache (BR5.5, BR5.7), a cancel after every stream was read is Completed
//! and written (NFR4.5, NFR6.18), a cancel while the cache is looked up
//! (NFR6.19), and a close after the cancel is still Aborted (NFR6.15).
//!
//! Every test runs against the fake gateway on tokio's paused clock with a
//! stop signal sent from the test; nothing connects to AWS (NFR6.13).

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use local_sights_core::cache::plan::{CacheKey, CacheOutcome, CachedEvent, CoveredRange};
use local_sights_core::cache::{HeaderRead, LogCache};
use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, run_fetch_with_cache};
use local_sights_core::failure::FailureKind;
use local_sights_core::fetch_stats::FetchSummary;
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{
    FixedJitter, Retrier, RetryPolicy, StopHandle, StopReason, stop_pair,
};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page, stream_page};
use support::recording_sink::{Ending, RecordingSink};
use tokio::time::Instant;

/// 2024/01/02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;
/// A fetch started a day later: the whole range is recordable (U6:BR3.1).
const A_DAY_LATER: i64 = START + 86_400_000;
/// Long enough that no test waits for it: a fetch that is not cut off
/// fails the test instead of hanging it.
const NEVER: Duration = Duration::from_secs(3_600);

fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

/// The U3 range for `dev` in Tokyo (12 `GetLogEvents` calls per second).
fn validated() -> ValidatedFetch {
    let validated = validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024/01/02 03:04:05".to_string(),
        end_text: "2024/01/02 03:05:05".to_string(),
    })
    .expect("test input is valid");
    ValidatedFetch {
        request: validated.request.with_connection(
            ProfileSelector::Named("dev".to_string()),
            "ap-northeast-1".to_string(),
        ),
        range: validated.range,
    }
}

fn names(count: usize, width: usize) -> Vec<String> {
    (0..count).map(|i| format!("s{i:0width$}")).collect()
}

/// A group whose listing is `names` in one page, every stream scripted with
/// one last page holding one event at `START + i`.
fn group_of(names: &[String]) -> FakeGateway {
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = names
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, name) in names.iter().enumerate() {
        let message = format!("{name}-0");
        gateway = gateway.script_stream(name, vec![page(&[(START + i as i64, &message)], None)]);
    }
    gateway
}

/// What one fetch did and when the test stopped it.
struct Run {
    job: FetchJob,
    sink: RecordingSink,
    /// When the stop was sent.
    stopped_at: Instant,
    /// Calls that had reached the fake when the stop was sent.
    calls_at_stop: usize,
    /// When the fetch returned.
    ended_at: Instant,
}

/// Runs one fetch (with the cache in front when `cache` is given) and sends
/// `reasons`, one after the other, `after` the start.
async fn fetch_and_stop(
    gateway: &FakeGateway,
    timeline: &Arc<Mutex<EventTimeline>>,
    cache: Option<&LogCache>,
    after: Duration,
    reasons: &[StopReason],
) -> Run {
    let (handle, signal) = stop_pair();
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, &signal);
    let mut sink = RecordingSink::default();
    let fetching = async {
        let job = run_fetch_with_cache(
            gateway,
            &validated(),
            timeline,
            &retrier,
            cache,
            A_DAY_LATER,
            &mut sink,
        )
        .await;
        (job, Instant::now())
    };
    let stopping = async {
        tokio::time::sleep(after).await;
        let calls = gateway.api_log().len();
        for reason in reasons {
            handle.stop(*reason);
        }
        (Instant::now(), calls)
    };
    let joined = tokio::time::timeout(NEVER, async { tokio::join!(fetching, stopping) });
    let ((job, ended_at), (stopped_at, calls_at_stop)) =
        joined.await.expect("the fetch ends once it is stopped");
    Run {
        job,
        sink,
        stopped_at,
        calls_at_stop,
        ended_at,
    }
}

/// Runs one fetch whose stop (`reason`) was sent before it started
/// (261020-carryover-fixes FR1.1, OQ1). `run_fetch_with_cache` does not look
/// at the stop before the cache lookup answers, so a stop sent beforehand
/// is, for the fetch, a stop that arrived while the cache was looked up and
/// before it answered. The order is then fixed by the test itself, not by
/// the clock or by how fast the lookup's blocking thread runs.
async fn fetch_already_stopped(
    gateway: &FakeGateway,
    timeline: &Arc<Mutex<EventTimeline>>,
    cache: Option<&LogCache>,
    reason: StopReason,
) -> (FetchJob, RecordingSink) {
    let (handle, signal) = stop_pair();
    handle.stop(reason);
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, &signal);
    let mut sink = RecordingSink::default();
    let fetch = validated();
    let fetching = run_fetch_with_cache(
        gateway,
        &fetch,
        timeline,
        &retrier,
        cache,
        A_DAY_LATER,
        &mut sink,
    );
    let job = tokio::time::timeout(NEVER, fetching)
        .await
        .expect("a fetch stopped beforehand ends");
    (job, sink)
}

fn ending(run: &Run) -> Ending {
    run.sink.ending.expect("the fetch finished")
}

fn summary(run: &Run) -> FetchSummary {
    run.sink.summary.expect("the summary was reported")
}

fn held(timeline: &Mutex<EventTimeline>) -> usize {
    timeline.lock().unwrap().len()
}

/// NFR4.2-NFR4.4: five streams waiting for their answers are cut off: no
/// call after the cancel, the fetch ends when the cancel is sent, the five
/// are unfinished and none of their pages is shown.
#[tokio::test(start_paused = true)]
async fn five_answers_awaited_are_cut_off_and_counted_unfinished() {
    let streams = names(5, 2);
    let gateway = group_of(&streams).with_call_duration(millis(500));
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(400),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(
        run.calls_at_stop, 6,
        "the listing and five reads were under way"
    );
    assert_eq!(
        gateway.api_log().len(),
        run.calls_at_stop,
        "no call after the cancel"
    );
    assert!(gateway.timed_calls().is_empty(), "no answer was awaited");
    assert_eq!(run.ended_at, run.stopped_at, "the clock did not move");
    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert_eq!(run.job.unfinished_stream_count, 5);
    assert!(!run.job.cancelled_before_listing);
    assert_eq!(run.job.event_count, 0);
    assert_eq!(held(&timeline), 0, "no page of a cut-off call");
    assert!(!run.sink.kinds().contains(&"Batch"));
    assert!(
        !run.sink.kinds().contains(&"StreamFinished"),
        "no give-up, no finish"
    );
    assert_eq!(ending(&run).status, JobStatus::Cancelled);
    assert_eq!(ending(&run).unfinished_stream_count, 5);
    let summary = summary(&run);
    assert_eq!(summary.status, JobStatus::Cancelled);
    assert_eq!(summary.calls, 0, "cut-off calls are not counted");
}

/// BR5.4: N counts the streams being read and those not started yet, not
/// the finished ones, failed ones included; the fetched pages stay.
#[tokio::test(start_paused = true)]
async fn unfinished_streams_are_the_ones_being_read_and_not_started() {
    let streams = names(20, 2);
    let gateway = group_of(&streams)
        .with_call_duration(Duration::from_secs(10))
        .stream_call_duration("s00", Duration::ZERO)
        .stream_call_duration("s01", Duration::ZERO)
        .script_stream("s02", vec![Err(failure(FailureKind::AccessDenied))])
        .stream_call_duration("s02", Duration::ZERO);
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(1_000),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert_eq!(run.job.finished_stream_count, 3);
    assert_eq!(run.job.failed_stream_count(), 1);
    assert_eq!(run.job.failed_streams[0].log_stream_name, "s02");
    assert_eq!(
        run.job.unfinished_stream_count, 17,
        "8 being read + 9 not started"
    );
    assert_eq!(run.job.event_count, 2);
    assert_eq!(held(&timeline), 2, "the fetched pages stay");
    assert_eq!(gateway.api_log().len(), run.calls_at_stop);
    assert_eq!(run.ended_at, run.stopped_at);
}

/// NFR4.6: the count holds for a thousand streams.
#[tokio::test(start_paused = true)]
async fn a_thousand_streams_count_every_unfinished_one() {
    let streams = names(1_000, 3);
    let mut gateway = group_of(&streams).with_call_duration(Duration::from_secs(10));
    for name in &streams[..10] {
        gateway = gateway.stream_call_duration(name, Duration::ZERO);
    }
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(1_000),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert_eq!(run.job.planned_stream_count, 1_000);
    assert_eq!(run.job.finished_stream_count, 10);
    assert_eq!(run.job.unfinished_stream_count, 990);
    assert_eq!(held(&timeline), 10);
}

/// NFR4.3: a cancel during a retry wait ends the fetch at once.
#[tokio::test(start_paused = true)]
async fn a_cancel_during_a_retry_wait_ends_at_once() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[("s00", None, None)], None)])
        .script_stream(
            "s00",
            vec![
                Err(failure(FailureKind::Throttled)),
                page(&[(START, "late")], None),
            ],
        );
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(500),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(
        run.ended_at, run.stopped_at,
        "the one-second wait did not run out"
    );
    assert_eq!(gateway.fetched_streams(), ["s00"], "not called again");
    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert_eq!(run.job.unfinished_stream_count, 1);
    assert_eq!(
        summary(&run).throttled,
        1,
        "the throttled attempt was answered"
    );
}

/// NFR4.3: a cancel while streams wait for their call permit ends at once,
/// and the waiting stream is never called.
#[tokio::test(start_paused = true)]
async fn a_cancel_while_waiting_for_a_permit_ends_at_once() {
    let streams = names(3, 2);
    let gateway = group_of(&streams).with_call_duration(Duration::from_secs(10));
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(100),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(
        gateway.fetched_streams(),
        ["s00", "s01"],
        "s02 still waited"
    );
    assert_eq!(run.ended_at, run.stopped_at);
    assert_eq!(run.job.unfinished_stream_count, 3);
}

/// NFR4.3: a call that never answers is cut off, while reading and while
/// listing; a cut-off listing is a cancel before the listing ended.
#[tokio::test(start_paused = true)]
async fn calls_that_never_answer_are_cut_off() {
    let gateway = group_of(&names(1, 2)).stream_never_answers("s00");
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(&gateway, &timeline, None, millis(50), &[StopReason::Cancel]).await;
    assert_eq!(run.ended_at, run.stopped_at);
    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert_eq!(run.job.unfinished_stream_count, 1);

    let silent_listing = FakeGateway::default().with_listing_that_never_answers();
    let run = fetch_and_stop(
        &silent_listing,
        &timeline,
        None,
        millis(50),
        &[StopReason::Cancel],
    )
    .await;
    assert_eq!(run.ended_at, run.stopped_at);
    assert_eq!(silent_listing.api_log(), ["DescribeLogStreams"]);
    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert!(run.job.cancelled_before_listing);
}

/// BR5.5, BR5.7: a cancel between two listing pages is Cancelled before
/// listing with no logs, no stream read and nothing written to the cache.
#[tokio::test(start_paused = true)]
async fn a_cancel_while_listing_is_cancelled_before_listing_and_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(&[("s00", None, None)], Some("next")),
        stream_page(&[("s01", None, None)], None),
    ])
    .with_listing_duration(millis(500));
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        Some(&cache),
        millis(700),
        &[StopReason::Cancel],
    )
    .await;

    assert_eq!(
        gateway.api_log(),
        ["DescribeLogStreams", "DescribeLogStreams"]
    );
    assert_eq!(run.ended_at, run.stopped_at, "the second page was cut off");
    assert_eq!(run.job.status, JobStatus::Cancelled);
    assert!(run.job.cancelled_before_listing);
    assert_eq!(run.job.unfinished_stream_count, 0);
    assert_eq!(run.job.event_count, 0);
    assert_eq!(run.sink.kinds(), ["Started", "Listing", "Finished"]);
    assert_eq!(
        ending(&run),
        Ending {
            status: JobStatus::Cancelled,
            event_count: 0,
            unfinished_stream_count: 0,
            cancelled_before_listing: true,
        }
    );
    assert_eq!(summary(&run).status, JobStatus::Cancelled);
    assert!(!run.sink.saving);
    assert_eq!(run.job.cache_outcome, CacheOutcome::NotSaved);
    let key = CacheKey::from_request(&validated().request).unwrap();
    assert_eq!(
        cache.read_header(&key),
        HeaderRead::Missing,
        "nothing written"
    );
}

/// A sink that sends a stop from inside the fetch: at the first page, or
/// once the last stream ended.
struct StopFromSink {
    inner: RecordingSink,
    handle: StopHandle,
    at_first_batch: bool,
}

impl FetchSink for StopFromSink {
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64) {
        self.inner.on_started(job, timeline_version);
    }
    fn on_listing_progress(&mut self, job_id: u64, progress: ListingProgress) {
        self.inner.on_listing_progress(job_id, progress);
    }
    fn on_planned(&mut self, job: &FetchJob) {
        self.inner.on_planned(job);
    }
    fn on_batch(&mut self, job_id: u64, progress: BatchProgress) {
        self.inner.on_batch(job_id, progress);
        if self.at_first_batch {
            self.handle.stop(StopReason::Cancel);
        }
    }
    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        self.inner.on_stream_finished(job, outcome);
        if !self.at_first_batch && job.finished_stream_count == job.planned_stream_count {
            self.handle.stop(StopReason::Cancel);
        }
    }
    fn on_saving(&mut self, job_id: u64) {
        self.inner.on_saving(job_id);
    }
    fn on_summary(&mut self, summary: &FetchSummary) {
        self.inner.on_summary(summary);
    }
    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64) {
        self.inner.on_finished(job, timeline_version);
    }
}

async fn fetch_stopped_by_sink(
    gateway: &FakeGateway,
    timeline: &Arc<Mutex<EventTimeline>>,
    cache: Option<&LogCache>,
    at_first_batch: bool,
) -> (FetchJob, RecordingSink) {
    let (handle, signal) = stop_pair();
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, &signal);
    let mut sink = StopFromSink {
        inner: RecordingSink::default(),
        handle,
        at_first_batch,
    };
    let job = run_fetch_with_cache(
        gateway,
        &validated(),
        timeline,
        &retrier,
        cache,
        A_DAY_LATER,
        &mut sink,
    )
    .await;
    (job, sink.inner)
}

/// NFR4.5, NFR6.18: a cancel after every stream was read but before the
/// fetch ended is Completed, keeps the logs and writes the cache.
#[tokio::test(start_paused = true)]
async fn a_cancel_after_every_stream_was_read_completes_and_saves() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    let gateway = group_of(&names(3, 2));
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let (job, sink) = fetch_stopped_by_sink(&gateway, &timeline, Some(&cache), false).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.unfinished_stream_count, 0);
    assert_eq!(job.event_count, 3);
    assert_eq!(held(&timeline), 3);
    assert!(sink.saving);
    assert_eq!(job.cache_outcome, CacheOutcome::Saved);
    let key = CacheKey::from_request(&validated().request).unwrap();
    assert!(matches!(cache.read_header(&key), HeaderRead::Found(_)));
}

/// BR5.4: a stream whose pages were all handed on but whose end was not
/// recorded yet is unfinished, and its page stays.
#[tokio::test(start_paused = true)]
async fn a_stream_with_only_its_end_left_is_unfinished() {
    let gateway = group_of(&names(1, 2));
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let (job, sink) = fetch_stopped_by_sink(&gateway, &timeline, None, true).await;

    assert_eq!(job.status, JobStatus::Cancelled);
    assert_eq!(job.unfinished_stream_count, 1);
    assert_eq!(job.finished_stream_count, 0);
    assert_eq!(job.event_count, 1);
    assert_eq!(held(&timeline), 1);
    assert!(!sink.kinds().contains(&"StreamFinished"));
}

fn cached(timestamp: i64) -> CachedEvent {
    CachedEvent {
        timestamp,
        ingestion_time: None,
        message: "cached".to_string(),
        log_stream_name: "s00".to_string(),
        sequence: 0,
    }
}

/// NFR6.19: a cancel that is already there when the cache answers: a hit
/// is shown whole and Completed; a miss is cancelled before listing,
/// without any call.
///
/// The cancel is sent before the fetch starts (`fetch_already_stopped`,
/// 261020-carryover-fixes FR1.1). The lookup runs on a real blocking
/// thread, so a cancel sent from the paused clock (even after
/// `Duration::ZERO`) could come after the lookup had answered; a miss then
/// went on to the listing and the unscripted fake failed the test. Sent
/// beforehand, the cancel is always there before the lookup answers.
#[tokio::test(start_paused = true)]
async fn a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    let key = CacheKey::from_request(&validated().request).unwrap();
    let range = validated().range;
    let whole = CoveredRange {
        start_ms: range.start_instant(),
        end_ms: range.end_instant(),
    };
    cache
        .write(&key, whole, vec![cached(START + 1), cached(START + 2)])
        .unwrap();
    let gateway = FakeGateway::default();
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let (hit, _) =
        fetch_already_stopped(&gateway, &timeline, Some(&cache), StopReason::Cancel).await;
    assert_eq!(hit.status, JobStatus::Completed);
    assert!(hit.served_from_cache);
    assert_eq!(hit.event_count, 2);
    assert_eq!(held(&timeline), 2);

    let empty = tempfile::tempdir().unwrap();
    let no_cache = LogCache::new(empty.path().join("cache"));
    let (miss, _) =
        fetch_already_stopped(&gateway, &timeline, Some(&no_cache), StopReason::Cancel).await;
    assert_eq!(miss.status, JobStatus::Cancelled);
    assert!(miss.cancelled_before_listing);
    assert_eq!(miss.event_count, 0);
    assert!(gateway.api_log().is_empty(), "no AWS API was called");
    assert_eq!(held(&timeline), 0);
}

/// NFR6.15, BR5.6: a close after the cancel, before the fetch ended, is
/// Aborted and discards the logs, as a close alone.
#[tokio::test(start_paused = true)]
async fn a_close_after_the_cancel_is_aborted() {
    let streams = names(3, 2);
    let gateway = group_of(&streams)
        .with_call_duration(millis(500))
        .stream_call_duration("s00", Duration::ZERO);
    let timeline = Arc::new(Mutex::new(EventTimeline::new()));
    let run = fetch_and_stop(
        &gateway,
        &timeline,
        None,
        millis(300),
        &[StopReason::Cancel, StopReason::Close],
    )
    .await;

    assert_eq!(run.job.status, JobStatus::Aborted);
    assert_eq!(run.job.event_count, 0);
    assert_eq!(held(&timeline), 0, "the fetched page was discarded");
    assert_eq!(summary(&run).status, JobStatus::Aborted);
    assert_eq!(run.ended_at, run.stopped_at);
}
