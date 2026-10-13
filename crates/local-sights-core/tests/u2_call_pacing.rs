//! 261012-parallel-fetch U2: lowering the `GetLogEvents` rate on throttling
//! and raising it again (functional-spec 4). Throttled attempts halve the
//! fetch's rate and widen the spacing of the calls (BR2.1-BR2.3, AC2.2.1,
//! AC2.2.3); quiet time brings it back to the ceiling (BR2.4); a close at the
//! lowest rate still ends at once (U1:NFR4.1); the retry waits do not
//! change (BR2.8, AC2.2.4); without throttling the ceiling is kept (U1:NFR1.1);
//! and the one-line summary carries the lowest rate as `min_rate` (BR2.7).
//!
//! Every test runs against the fake gateway on tokio's paused clock, so the
//! permit spacing and retry waits pass instantly and nothing connects to AWS
//! (NFR6.1).

mod support;

use std::sync::Mutex;
use std::time::Duration;

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, run_fetch};
use local_sights_core::failure::{ApiFailure, FailureKind};
use local_sights_core::fetch_stats::FetchSummary;
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::gateway::GetLogEventsPage;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{
    FixedJitter, Retrier, RetryPolicy, StopReason, StopSignal, stop_pair,
};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page, stream_page};
use tokio::time::Instant;

/// 2024/01/02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;
/// Tokyo: 25 calls per second at AWS, a ceiling of 12 for the app.
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
/// `script(i)`.
fn group_with(
    names: &[String],
    script: impl Fn(usize) -> Vec<Result<GetLogEventsPage, ApiFailure>>,
) -> FakeGateway {
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = names
        .iter()
        .map(|name| (name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, name) in names.iter().enumerate() {
        gateway = gateway.script_stream(name, script(i));
    }
    gateway
}

/// Keeps the summary and the final status; nothing else is shown for the
/// rate (FR2.6), so nothing else is recorded.
#[derive(Default)]
struct SummarySink {
    summary: Option<FetchSummary>,
    finished: Option<JobStatus>,
}

impl FetchSink for SummarySink {
    fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {}

    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}

    fn on_planned(&mut self, _job: &FetchJob) {}

    fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}

    fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {}

    fn on_summary(&mut self, summary: &FetchSummary) {
        assert!(self.summary.is_none(), "one summary per fetch");
        self.summary = Some(*summary);
    }

    fn on_finished(&mut self, job: &FetchJob, _timeline_version: u64) {
        self.finished = Some(job.status);
    }
}

async fn run(
    gateway: &FakeGateway,
    region: Option<&str>,
    signal: &StopSignal,
) -> (FetchJob, SummarySink) {
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, signal);
    let timeline = Mutex::new(EventTimeline::new());
    let mut sink = SummarySink::default();
    let job = run_fetch(gateway, &validated(region), &timeline, &retrier, &mut sink).await;
    (job, sink)
}

async fn run_plain(gateway: &FakeGateway, region: Option<&str>) -> (FetchJob, SummarySink) {
    run(gateway, region, &StopSignal::never()).await
}

/// The start of every timed `GetLogEvents` call, in order.
fn starts(gateway: &FakeGateway) -> Vec<Instant> {
    gateway
        .timed_calls()
        .iter()
        .map(|call| call.started)
        .collect()
}

/// The time between neighbouring call starts, in whole milliseconds.
fn gaps_ms(starts: &[Instant]) -> Vec<u128> {
    starts
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).as_millis())
        .collect()
}

/// The most call starts in any one-second window `[t, t + 1 s)`.
fn busiest_second(starts: &[Instant]) -> usize {
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

/// Asserts every gap in `gaps` is the interval of `rate` per second, give or
/// take the 1 ms of tokio's timer rounding.
fn assert_spaced_at(gaps: &[u128], rate: u128, what: &str) {
    let interval = 1_000 / rate;
    for gap in gaps {
        assert!(
            (interval..=interval + 1).contains(gap),
            "{what}: gap {gap} ms, not {interval} ms ({gaps:?})"
        );
    }
}

/// AC2.2.1, AC2.2.3, BR2.2, BR2.3: while every first attempt is throttled,
/// the calls are spaced wider and wider: 12, then 6, 3 and 1 per second. No
/// second ever holds more than the ceiling of 12.
#[tokio::test(start_paused = true)]
async fn throttling_widens_the_spacing_of_the_calls() {
    let streams = names(24);
    let gateway = group_with(&streams, |_| {
        vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            page(&[(START, "x")], None),
        ]
    })
    .with_call_duration(millis(10));
    let (job, sink) = run_plain(&gateway, TOKYO).await;

    assert_eq!(
        job.status,
        JobStatus::Completed,
        "three throttles are retried"
    );
    let starts = starts(&gateway);
    assert_eq!(starts.len(), 96);
    let gaps = gaps_ms(&starts);
    // The first eight readers at the ceiling, before any report.
    assert_spaced_at(&gaps[..7], 12, "at 12 per second");
    // The first report (at 10 ms) halves to 6; the retries after the 1 s
    // wait are spaced for 6, then 3 once the hold of one second is over.
    assert_spaced_at(&gaps[8..9], 6, "at 6 per second");
    assert_spaced_at(&gaps[9..16], 3, "at 3 per second");
    // Then 1 per second while the reports keep coming.
    assert_spaced_at(&gaps[16..29], 1, "at 1 per second");
    assert!(busiest_second(&starts) <= 12, "{}", busiest_second(&starts));
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.calls, summary.throttled), (96, 72));
    assert_eq!(summary.min_rate, 1);
}

/// AC2.2.3, BR2.4, NFR3.6: after one throttle the rate comes back by one
/// every five quiet seconds, and the spacing shrinks back to the ceiling's
/// 83 ms, never below it.
#[tokio::test(start_paused = true)]
async fn once_the_throttling_stops_the_spacing_comes_back_to_the_ceiling() {
    const PAGES: usize = 330;
    let mut script = vec![Err(failure(FailureKind::Throttled))];
    for i in 0..PAGES {
        let token = (i + 1 < PAGES).then(|| format!("f/{i}"));
        script.push(page(&[(START + i as i64, "x")], token.as_deref()));
    }
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[("s00", None, None)], None)])
        .script_stream("s00", script)
        .with_call_duration(millis(1));
    let (job, sink) = run_plain(&gateway, TOKYO).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, PAGES as u64);
    let starts = starts(&gateway);
    let gaps = gaps_ms(&starts);
    assert_eq!(gaps[0], 1_001, "the retry waits 1 s after the 1 ms call");
    assert_spaced_at(&gaps[1..10], 6, "right after the throttle");
    for pair in gaps[1..].windows(2) {
        assert!(pair[1] <= pair[0] + 1, "the spacing only shrinks: {pair:?}");
    }
    let first_at_ceiling = gaps
        .iter()
        .position(|gap| *gap <= 84)
        .expect("the ceiling is reached again");
    assert_spaced_at(&gaps[first_at_ceiling..], 12, "back at the ceiling");
    let back_at_ceiling = starts[first_at_ceiling] - starts[0];
    assert!(
        (Duration::from_secs(30)..Duration::from_secs(31)).contains(&back_at_ceiling),
        "6 → 12 takes six steps of 5 s after the throttle: {back_at_ceiling:?}"
    );
    assert!(busiest_second(&starts) <= 12);
    assert_eq!(sink.summary.map(|s| s.min_rate), Some(6));
}

/// U1:NFR4.1 at the lowest rate (NFR requirements review R-03): with the
/// rate down to 1, eight readers would wait about eight seconds for their
/// permits; a close ends the waits at once and no call is started.
#[tokio::test(start_paused = true)]
async fn a_close_at_the_lowest_rate_ends_the_permit_waits_at_once() {
    let streams = names(24);
    let gateway = group_with(&streams, |_| {
        vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            page(&[(START, "x")], None),
        ]
    })
    .with_call_duration(millis(10));
    let (handle, signal) = stop_pair();
    let started = Instant::now();
    let close_at = started + Duration::from_secs(6);
    let fetching = run(&gateway, TOKYO, &signal);
    let close_later = async {
        tokio::time::sleep_until(close_at).await;
        handle.stop(StopReason::Close);
    };
    let ((job, sink), ()) = tokio::join!(fetching, close_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert_eq!(Instant::now(), close_at, "no wait after the close");
    let starts = starts(&gateway);
    assert!(
        starts.iter().all(|at| *at < close_at),
        "no call after the close"
    );
    // At 1 per second since about 3.5 s: the calls of 4.5 s and 5.5 s.
    assert_eq!(gaps_ms(&starts).last(), Some(&1_000));
    let summary = sink.summary.expect("summary");
    assert_eq!(summary.status, JobStatus::Aborted);
    assert_eq!(summary.min_rate, 1);
    assert_eq!(sink.finished, Some(JobStatus::Aborted));
}

/// AC2.2.4, BR2.8: a call throttled twice and then answered waits 1 s and
/// 2 s, as before U2, whatever the rate came down to; each attempt still
/// waits for its own permit.
#[tokio::test(start_paused = true)]
async fn the_retry_waits_stay_one_and_two_seconds() {
    for (region, lowest) in [(TOKYO, 3), (None, 1)] {
        let gateway =
            FakeGateway::with_stream_pages(vec![stream_page(&[("s00", None, None)], None)])
                .script_stream(
                    "s00",
                    vec![
                        Err(failure(FailureKind::Throttled)),
                        Err(failure(FailureKind::Throttled)),
                        page(&[(START, "x")], None),
                    ],
                )
                .with_call_duration(millis(10));
        let (job, sink) = run_plain(&gateway, region).await;

        assert_eq!(job.status, JobStatus::Completed, "{region:?}");
        let calls = gateway.timed_calls();
        let waits: Vec<Duration> = calls
            .windows(2)
            .map(|pair| pair[1].started - pair[0].ended)
            .collect();
        assert_eq!(
            waits,
            [Duration::from_secs(1), Duration::from_secs(2)],
            "{region:?}"
        );
        let summary = sink.summary.expect("summary");
        assert_eq!(
            (summary.calls, summary.throttled, summary.min_rate),
            (3, 2, lowest),
            "{region:?}"
        );
    }
}

/// U1:NFR1.1, NFR1.3: without throttling the rate stays at the ceiling: 100
/// streams of one 200 ms call each still take between 8 s and 10 s in Tokyo,
/// with every call start at least the ceiling's 83 ms apart.
#[tokio::test(start_paused = true)]
async fn without_throttling_the_rate_stays_at_the_ceiling() {
    let streams = names(100);
    let gateway = group_with(&streams, |i| vec![page(&[(START + i as i64, "x")], None)])
        .with_call_duration(millis(200));
    let started = Instant::now();
    let (job, sink) = run_plain(&gateway, TOKYO).await;
    let elapsed = started.elapsed();

    assert_eq!(job.status, JobStatus::Completed);
    assert!(
        (Duration::from_secs(8)..=Duration::from_secs(10)).contains(&elapsed),
        "{elapsed:?}"
    );
    let starts = starts(&gateway);
    assert!(gaps_ms(&starts).iter().all(|gap| *gap >= 83));
    assert_eq!(busiest_second(&starts), 12, "the ceiling is reached");
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.throttled, summary.min_rate), (0, 12));
}

/// BR2.7, NFR8.7: the summary line carries the lowest rate after
/// max_in_flight; a fetch that ended before the pacer was made, and one
/// whose only throttle was on the listing (BR2.1), report the ceiling.
#[tokio::test(start_paused = true)]
async fn the_summary_line_reports_the_lowest_rate() {
    // Lowered once: 12 → 6.
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[("s00", None, None)], None)])
        .script_stream(
            "s00",
            vec![
                Err(failure(FailureKind::Throttled)),
                page(&[(START, "x")], None),
            ],
        );
    let (_, sink) = run_plain(&gateway, TOKYO).await;
    assert_eq!(
        sink.summary.map(|s| s.line()),
        Some(
            "local-sights: fetch summary streams=1 calls=2 throttled=1 elapsed_ms=1000 \
             max_in_flight=1 min_rate=6 status=Completed"
                .to_string()
        )
    );

    // Closed during the listing's retry wait, before the pacer is made: the
    // region's ceiling, 12 in Tokyo and 5 without a region.
    for (region, ceiling) in [(TOKYO, 12), (None, 5)] {
        let gateway = FakeGateway::with_stream_pages(vec![
            Err(failure(FailureKind::Throttled)),
            stream_page(&[("s00", None, None)], None),
        ]);
        let (handle, signal) = stop_pair();
        let close_later = async {
            tokio::time::sleep(millis(500)).await;
            handle.stop(StopReason::Close);
        };
        let ((job, sink), ()) = tokio::join!(run(&gateway, region, &signal), close_later);
        assert_eq!(job.status, JobStatus::Aborted);
        assert!(gateway.calls().is_empty());
        assert_eq!(
            sink.summary.map(|s| s.line()),
            Some(format!(
                "local-sights: fetch summary streams=0 calls=0 throttled=0 elapsed_ms=0 \
                 max_in_flight=0 min_rate={ceiling} status=Aborted"
            )),
            "{region:?}"
        );
    }

    // The listing's first page failed: nothing was read, the ceiling.
    let gateway = FakeGateway::with_stream_pages(vec![Err(failure(FailureKind::AccessDenied))]);
    let (job, sink) = run_plain(&gateway, TOKYO).await;
    assert_eq!(job.status, JobStatus::Failed);
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.min_rate, summary.status), (12, JobStatus::Failed));

    // A throttled listing does not lower the rate (BR2.1).
    let gateway = FakeGateway::with_stream_pages(vec![
        Err(failure(FailureKind::Throttled)),
        stream_page(&[("s00", None, None)], None),
    ])
    .script_stream("s00", vec![page(&[(START, "x")], None)]);
    let (job, sink) = run_plain(&gateway, TOKYO).await;
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(gateway.stream_calls().len(), 2, "the listing was retried");
    let summary = sink.summary.expect("summary");
    assert_eq!((summary.throttled, summary.min_rate), (0, 12));
}
