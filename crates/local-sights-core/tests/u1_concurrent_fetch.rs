//! 261012-parallel-fetch U1: reading a log group's streams side by side
//! (functional-spec 5). Up to eight streams at once (BR1.7), in the selected
//! order (BR1.8), under the per-region ceiling of `GetLogEvents` calls
//! (BR1.1-BR1.5), pages handed on one at a time in arrival order and merged
//! by time (BR1.10-BR1.12), partial failures kept (BR1.14), the window close
//! (BR1.16) and the one-line summary (NFR2.2).
//!
//! Every test runs against the fake gateway on tokio's paused clock, so the
//! call durations, permit spacing and retry waits pass instantly and
//! nothing connects to AWS (NFR6.1).

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{
    BatchProgress, FetchJob, FetchSink, TimelineStore, run_fetch,
};
use local_sights_core::event::LogEvent;
use local_sights_core::failure::FailureKind;
use local_sights_core::fetch_stats::FetchSummary;
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{
    FixedJitter, Retrier, RetryPolicy, StopHandle, StopReason, StopSignal, stop_pair,
};
use local_sights_core::streams::StreamListingStatus;
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page, stream_page};
use tokio::time::Instant;

/// 2024/01/02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;
/// 2024/01/02 03:05:05.999 UTC (end of the range).
const END: i64 = START + 60_999;
/// One millisecond older than the listing margin (U3:BR1.2).
const TOO_OLD: i64 = START - 3_600_000 - 1;
/// Tokyo: 25 calls per second at AWS, 12 for the app (BR1.1).
const TOKYO: Option<&str> = Some("ap-northeast-1");

fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

/// The U3 range for `dev`, in `region` (`None`: no region in the request).
fn validated(region: Option<&str>) -> ValidatedFetch {
    let validated = validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024/01/02 03:04:05".to_string(),
        end_text: "2024/01/02 03:05:05".to_string(),
    })
    .expect("test input is valid");
    match region {
        Some(region) => ValidatedFetch {
            request: validated.request.with_connection(
                ProfileSelector::Named("dev".to_string()),
                region.to_string(),
            ),
            range: validated.range,
        },
        None => validated,
    }
}

fn names(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("s{i:02}")).collect()
}

/// A group whose listing is `names` in one page, every stream scripted with
/// one page holding one event at `START + i`.
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

/// Pages added and streams ended, with when and which stream, in order.
type OrderLog = Arc<Mutex<Vec<(Instant, String, &'static str)>>>;

/// One delivery to the sink, with when it happened.
#[derive(Debug, Clone, PartialEq)]
enum Seen {
    Batch(BatchProgress),
    StreamFinished(String),
    Finished(JobStatus),
}

/// Records every delivery with its time, the summary, and optionally a log
/// shared with [`LoggedTimeline`]. With `stop_after_last_stream` it closes
/// the fetch as soon as the last stream is reported, before the job ends.
#[derive(Default)]
struct TimedSink {
    seen: Vec<(Instant, Seen)>,
    summary: Option<FetchSummary>,
    order: Option<OrderLog>,
    stop_after_last_stream: Option<StopHandle>,
}

impl TimedSink {
    fn first_batch_at(&self) -> Option<Instant> {
        self.seen
            .iter()
            .find(|(_, seen)| matches!(seen, Seen::Batch(_)))
            .map(|(at, _)| *at)
    }

    fn batches(&self) -> usize {
        self.seen
            .iter()
            .filter(|(_, seen)| matches!(seen, Seen::Batch(_)))
            .count()
    }
}

impl FetchSink for TimedSink {
    fn on_started(&mut self, job: &FetchJob, _timeline_version: u64) {
        assert_eq!(job.status, JobStatus::Running);
    }

    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}

    fn on_planned(&mut self, _job: &FetchJob) {}

    fn on_batch(&mut self, _job_id: u64, progress: BatchProgress) {
        self.seen.push((Instant::now(), Seen::Batch(progress)));
    }

    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        let name = outcome.log_stream_name.clone();
        if let Some(order) = &self.order {
            order
                .lock()
                .unwrap()
                .push((Instant::now(), name.clone(), "end"));
        }
        self.seen.push((Instant::now(), Seen::StreamFinished(name)));
        if job.finished_stream_count == job.planned_stream_count {
            if let Some(handle) = &self.stop_after_last_stream {
                handle.stop(StopReason::Close);
            }
        }
    }

    fn on_summary(&mut self, summary: &FetchSummary) {
        assert!(self.summary.is_none(), "one summary per fetch");
        self.summary = Some(*summary);
    }

    fn on_finished(&mut self, job: &FetchJob, _timeline_version: u64) {
        self.seen.push((Instant::now(), Seen::Finished(job.status)));
    }
}

/// A timeline that logs which stream each added page came from.
struct LoggedTimeline {
    inner: Mutex<EventTimeline>,
    order: OrderLog,
}

impl TimelineStore for LoggedTimeline {
    fn discard(&self) -> u64 {
        self.inner.discard()
    }

    fn add(&self, events: Vec<LogEvent>) -> u64 {
        if let Some(event) = events.first() {
            self.order.lock().unwrap().push((
                Instant::now(),
                event.log_stream_name.clone(),
                "page",
            ));
        }
        self.inner.add(events)
    }
}

async fn run<T: TimelineStore>(
    gateway: &FakeGateway,
    fetch: &ValidatedFetch,
    timeline: &T,
    signal: &StopSignal,
    sink: &mut TimedSink,
) -> FetchJob {
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, signal);
    run_fetch(gateway, fetch, timeline, &retrier, sink).await
}

async fn run_plain(
    gateway: &FakeGateway,
    region: Option<&str>,
    timeline: &Mutex<EventTimeline>,
) -> (FetchJob, TimedSink) {
    let mut sink = TimedSink::default();
    let (_handle, signal) = stop_pair();
    let job = run(gateway, &validated(region), timeline, &signal, &mut sink).await;
    (job, sink)
}

fn held(timeline: &Mutex<EventTimeline>) -> Vec<(String, String)> {
    timeline
        .lock()
        .unwrap()
        .events()
        .iter()
        .map(|e| (e.log_stream_name.clone(), e.message.clone()))
        .collect()
}

fn pair(stream: &str, message: &str) -> (String, String) {
    (stream.to_string(), message.to_string())
}

/// The most call starts in any one-second window `[t, t + 1 s)`.
fn busiest_second(starts: &[Instant]) -> usize {
    let mut starts = starts.to_vec();
    starts.sort();
    (0..starts.len())
        .map(|i| {
            starts[i..]
                .iter()
                .take_while(|at| **at - starts[i] < Duration::from_secs(1))
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// AC1.1.1: a stream's call starts while another stream's call is still
/// under way, and never more than eight are under way.
#[tokio::test(start_paused = true)]
async fn streams_are_called_side_by_side() {
    let streams = names(10);
    let gateway = group_of(&streams).with_call_duration(millis(200));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 10);
    let calls = gateway.timed_calls();
    assert_eq!(calls.len(), 10);
    let overlapping = calls
        .windows(2)
        .any(|pair| pair[0].stream != pair[1].stream && pair[1].started < pair[0].ended);
    assert!(overlapping, "{calls:?}");
    assert!(gateway.max_concurrent_calls() >= 2);
    assert!(gateway.max_concurrent_calls() <= 8);
    assert_eq!(sink.summary.map(|s| s.max_in_flight), Some(8));
}

/// AC1.1.2, NFR1.1: 100 streams of one 200 ms call each in Tokyo take at
/// most half of the 20 s that one stream after another takes. The ceiling
/// of 12 calls a second is what bounds it (about 8.5 s).
#[tokio::test(start_paused = true)]
async fn a_hundred_streams_take_at_most_half_the_one_by_one_time() {
    let streams = names(100);
    let gateway = group_of(&streams).with_call_duration(millis(200));
    let timeline = Mutex::new(EventTimeline::new());
    let started = Instant::now();
    let (job, _) = run_plain(&gateway, TOKYO, &timeline).await;
    let elapsed = started.elapsed();

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 100);
    assert!(elapsed <= Duration::from_secs(10), "{elapsed:?}");
    assert!(
        elapsed > Duration::from_secs(8),
        "the ceiling binds: {elapsed:?}"
    );
}

/// NFR3.2 (AC1.1.2's premise): with retries after throttling too, no
/// one-second window holds more than 12 `GetLogEvents` calls in Tokyo.
#[tokio::test(start_paused = true)]
async fn no_second_holds_more_than_twelve_calls_retries_included() {
    let streams = names(100);
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = streams
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, name) in streams.iter().enumerate() {
        let mut script = Vec::new();
        if i % 10 == 0 {
            script.push(Err(failure(FailureKind::Throttled)));
        }
        script.push(page(&[(START, "x")], None));
        gateway = gateway.script_stream(name, script);
    }
    let gateway = gateway.with_call_duration(millis(200));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    let starts: Vec<Instant> = gateway.timed_calls().iter().map(|c| c.started).collect();
    assert_eq!(starts.len(), 110, "100 first calls and 10 retries");
    assert!(busiest_second(&starts) <= 12, "{}", busiest_second(&starts));
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.calls, summary.throttled), (110, 10));
}

/// AC1.1.3: the streams selected, and the order they are started in, are
/// the same as before U1 (the U3 listing rules).
#[tokio::test(start_paused = true)]
async fn the_same_streams_are_selected_and_started_in_listing_order() {
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(
            &[
                ("b", Some(START), Some(END)),
                ("skip", Some(END + 1), Some(END + 2)),
            ],
            Some("n1"),
        ),
        stream_page(
            &[
                ("a", None, None),
                ("old", Some(0), Some(TOO_OLD)),
                ("no-times", None, None),
            ],
            Some("n2"),
        ),
    ])
    .script_stream("b", vec![page(&[(START, "b0")], None)])
    .script_stream("a", vec![page(&[(START + 1, "a0")], None)])
    .script_stream("no-times", vec![page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.listing_status, Some(StreamListingStatus::StoppedEarly));
    assert_eq!(job.planned_stream_count, 3);
    assert_eq!(gateway.stream_calls().len(), 2, "no third listing page");
    assert_eq!(gateway.fetched_streams(), vec!["b", "a", "no-times"]);
    assert_eq!(job.status, JobStatus::Completed);
}

/// AC1.2.1, AC1.2.4: when only the fifth stream onwards has logs in the
/// range, the first page arrives sooner than reading one stream after
/// another (about 5 × 200 ms = 1 s).
#[tokio::test(start_paused = true)]
async fn the_first_logs_arrive_before_the_one_by_one_time() {
    let streams = names(20);
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = streams
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, name) in streams.iter().enumerate() {
        let script = if i < 4 {
            page(&[], None)
        } else {
            page(&[(START + i as i64, "x")], None)
        };
        gateway = gateway.script_stream(name, vec![script]);
    }
    let gateway = gateway.with_call_duration(millis(200));
    let timeline = Mutex::new(EventTimeline::new());
    let started = Instant::now();
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.event_count, 16);
    let first = sink.first_batch_at().expect("a batch") - started;
    assert!(first < Duration::from_secs(1), "{first:?}");
}

/// AC1.2.2, BR1.12: an older page that arrives later is still placed by
/// time, then stream name, then the order inside the stream.
#[tokio::test(start_paused = true)]
async fn the_rows_stay_in_time_order_whatever_order_the_pages_arrive_in() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[
            ("slow", None, None),
            ("fast", None, None),
            ("tie", None, None),
        ],
        None,
    )])
    .script_stream(
        "slow",
        vec![page(&[(START + 1, "slow-1"), (START + 3, "slow-3")], None)],
    )
    .script_stream(
        "fast",
        vec![page(&[(START + 2, "fast-2"), (START + 4, "fast-4")], None)],
    )
    .script_stream("tie", vec![page(&[(START + 1, "tie-1")], None)])
    .stream_call_duration("slow", millis(900))
    .stream_call_duration("fast", millis(50))
    .stream_call_duration("tie", millis(50));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(sink.batches(), 3);
    assert_eq!(
        held(&timeline),
        vec![
            pair("slow", "slow-1"),
            pair("tie", "tie-1"),
            pair("fast", "fast-2"),
            pair("slow", "slow-3"),
            pair("fast", "fast-4"),
        ]
    );
}

/// AC1.3.1, AC1.3.2, BR1.14: a stream denied access is listed with a safe
/// detail only, and the other streams' logs are kept.
#[tokio::test(start_paused = true)]
async fn a_failed_stream_is_listed_and_the_others_are_kept() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[
            ("s00", None, None),
            ("s01", None, None),
            ("s02", None, None),
        ],
        None,
    )])
    .script_stream("s00", vec![page(&[(START, "zero")], None)])
    .script_stream("s01", vec![Err(failure(FailureKind::AccessDenied))])
    .script_stream("s02", vec![page(&[(START + 2, "two")], None)])
    .with_call_duration(millis(100));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(job.failed_streams.len(), 1);
    assert_eq!(job.failed_streams[0].log_stream_name, "s01");
    assert_eq!(
        job.failed_streams[0].failure.safe_detail(),
        "kind=AccessDenied; api=GetLogEvents"
    );
    assert_eq!(
        held(&timeline),
        vec![pair("s00", "zero"), pair("s02", "two")]
    );
    assert_eq!(job.finished_stream_count, 3);
}

/// BR1.5: a close while streams wait for their permit stops them without
/// calling AWS, and the fetch ends Aborted.
#[tokio::test(start_paused = true)]
async fn a_close_during_the_permit_wait_calls_nothing_more() {
    let streams = names(8);
    let gateway = group_of(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = stop_pair();
    let mut sink = TimedSink::default();
    let fetch = validated(TOKYO);
    let fetching = run(&gateway, &fetch, &timeline, &signal, &mut sink);
    let close_later = async {
        tokio::time::sleep(millis(100)).await;
        handle.stop(StopReason::Close);
    };
    let (job, ()) = tokio::join!(fetching, close_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert_eq!(
        gateway.fetched_streams(),
        vec!["s00", "s01"],
        "permits at 0 and 84 ms only; the other six were waiting"
    );
    assert!(timeline.lock().unwrap().is_empty());
    assert_eq!(
        sink.seen.last().map(|(_, s)| s),
        Some(&Seen::Finished(JobStatus::Aborted))
    );
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.status, summary.calls), (JobStatus::Aborted, 2));
}

/// BR1.16: no page under way at the close is handed on. Since U5 (NFR4.3,
/// nfr-requirements Q1) the calls under way are cut off instead of awaited.
#[tokio::test(start_paused = true)]
async fn pages_that_arrive_after_the_close_are_dropped() {
    let streams = names(2);
    let gateway = group_of(&streams).with_call_duration(millis(500));
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = stop_pair();
    let mut sink = TimedSink::default();
    let fetch = validated(TOKYO);
    let started = Instant::now();
    let fetching = run(&gateway, &fetch, &timeline, &signal, &mut sink);
    let close_later = async {
        tokio::time::sleep(millis(100)).await;
        handle.stop(StopReason::Close);
    };
    let (job, ()) = tokio::join!(fetching, close_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert_eq!(job.event_count, 0);
    assert_eq!(sink.batches(), 0, "no page after the close");
    assert!(
        !sink
            .seen
            .iter()
            .any(|(_, s)| matches!(s, Seen::StreamFinished(_))),
        "no stream is reported after the close"
    );
    assert!(timeline.lock().unwrap().is_empty());
    assert_eq!(
        gateway.calls().len(),
        2,
        "both calls were already under way"
    );
    assert!(
        gateway.timed_calls().is_empty(),
        "U5: neither call was awaited to its answer"
    );
    assert_eq!(
        started.elapsed(),
        millis(100),
        "U5: the fetch ended at the close"
    );
}

/// BR1.16: a close after the last stream but before the job finishes still
/// makes the job Aborted and discards the logs.
#[tokio::test(start_paused = true)]
async fn a_close_after_the_last_stream_but_before_the_end_is_aborted() {
    let streams = names(2);
    let gateway = group_of(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = stop_pair();
    let mut sink = TimedSink {
        stop_after_last_stream: Some(handle),
        ..TimedSink::default()
    };
    let job = run(&gateway, &validated(TOKYO), &timeline, &signal, &mut sink).await;

    assert_eq!(job.status, JobStatus::Aborted);
    assert_eq!(job.event_count, 0);
    assert!(
        timeline.lock().unwrap().is_empty(),
        "the logs are discarded"
    );
    assert_eq!(sink.batches(), 2, "both pages had been handed on");
    assert_eq!(
        sink.seen.last().map(|(_, s)| s),
        Some(&Seen::Finished(JobStatus::Aborted))
    );
}

/// BR1.10, BR1.11: each stream's pages come before its end, and a stream's
/// next call starts only after its previous page was handed on, so at most
/// eight pages wait at once.
#[tokio::test(start_paused = true)]
async fn each_streams_pages_come_before_its_end_and_one_at_a_time() {
    let streams = names(10);
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = streams
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, name) in streams.iter().enumerate() {
        let t = START + i as i64;
        gateway = gateway
            .script_stream(
                name,
                vec![
                    page(&[(t, "p0")], Some("f/1")),
                    page(&[(t + 10, "p1")], Some("f/2")),
                    page(&[(t + 20, "p2")], None),
                ],
            )
            .stream_call_duration(name, millis(100 + 37 * i as u64));
    }
    let order = Arc::new(Mutex::new(Vec::new()));
    let timeline = LoggedTimeline {
        inner: Mutex::new(EventTimeline::new()),
        order: Arc::clone(&order),
    };
    let (_handle, signal) = stop_pair();
    let mut sink = TimedSink {
        order: Some(Arc::clone(&order)),
        ..TimedSink::default()
    };
    let job = run(&gateway, &validated(TOKYO), &timeline, &signal, &mut sink).await;
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 30);

    let order = order.lock().unwrap().clone();
    let calls = gateway.timed_calls();
    for stream in &streams {
        let own: Vec<&(Instant, String, &str)> =
            order.iter().filter(|(_, s, _)| s == stream).collect();
        let kinds: Vec<&str> = own.iter().map(|(_, _, kind)| *kind).collect();
        assert_eq!(kinds, ["page", "page", "page", "end"], "{stream}");
        let pages: Vec<Instant> = own.iter().take(3).map(|(at, _, _)| *at).collect();
        let starts: Vec<Instant> = calls
            .iter()
            .filter(|call| &call.stream == stream)
            .map(|call| call.started)
            .collect();
        assert_eq!(starts.len(), 3, "{stream}");
        assert!(starts[1] >= pages[0] && starts[2] >= pages[1], "{stream}");
    }
    assert!(gateway.max_concurrent_calls() <= 8);
    assert_eq!(sink.summary.map(|s| s.max_in_flight), Some(8));
}

/// BR1.9: a group with no stream for the range is not read and completes.
#[tokio::test(start_paused = true)]
async fn no_selected_stream_completes_without_reading() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.planned_stream_count, 0);
    assert!(gateway.calls().is_empty());
    assert_eq!(
        sink.summary.map(|s| s.line()),
        Some(
            "local-sights: fetch summary streams=0 calls=0 throttled=0 elapsed_ms=0 \
             max_in_flight=0 min_rate=12 status=Completed"
                .to_string()
        )
    );
}

/// NFR2.2, NFR2.4: the summary counts the streams, every attempt, the
/// throttled ones, the read time and the most streams read at once.
#[tokio::test(start_paused = true)]
async fn the_summary_counts_streams_calls_throttles_and_time() {
    let streams = names(5);
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = streams
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)])
        .script_stream(
            "s00",
            vec![
                Err(failure(FailureKind::Throttled)),
                Err(failure(FailureKind::Throttled)),
                page(&[(START, "late")], None),
            ],
        )
        .script_stream("s01", vec![Err(failure(FailureKind::AccessDenied))])
        .script_stream("s02", vec![page(&[(START + 2, "x")], None)])
        .script_stream("s03", vec![page(&[(START + 3, "x")], None)])
        .script_stream("s04", vec![page(&[(START + 4, "x")], None)])
        .with_call_duration(millis(100));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    let summary = sink.summary.expect("summary");
    assert_eq!(
        (
            summary.streams,
            summary.calls,
            summary.throttled,
            summary.max_in_flight,
            summary.status
        ),
        (5, 7, 2, 5, JobStatus::CompletedWithFailures)
    );
    // Three 100 ms calls and the 1 s and 2 s retry waits of s00.
    assert!(summary.elapsed_ms >= 3_300, "{}", summary.elapsed_ms);
    assert!(
        summary
            .line()
            .starts_with("local-sights: fetch summary streams=5 calls=7 ")
    );
}

/// NFR1.3: with 700 ms calls, the read stage keeps exactly eight calls
/// under way at its busiest.
#[tokio::test(start_paused = true)]
async fn slow_calls_reach_exactly_eight_at_once() {
    let streams = names(20);
    let gateway = group_of(&streams).with_call_duration(millis(700));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run_plain(&gateway, TOKYO, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(gateway.max_concurrent_calls(), 8);
    assert_eq!(sink.summary.map(|s| s.max_in_flight), Some(8));
}
