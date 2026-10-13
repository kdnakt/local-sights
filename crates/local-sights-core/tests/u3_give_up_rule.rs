//! 261012-parallel-fetch U3 (u3-give-up-rule): the give-up rule through the
//! whole fetch (functional-spec 3, security-design.md "確かめること").
//!
//! The StreamSchedule counts the streams that used up their retries in a
//! row, in the order they end (BR3.1); at three, the streams not started yet
//! are failed without calling `GetLogEvents`, with a copy of the third
//! stream's failure (BR3.2, NFR3.7, NFR8.9); the streams being read are read
//! to their end (BR3.3); a failure that is not retried resets the count
//! (BR3.4); and a close still ends the fetch as Aborted (BR3.5, U1:BR1.16).
//!
//! The finishing order is made different from the reading order by giving
//! each stream's calls their own duration on the fake gateway. Exhausting a
//! request takes six calls and 1 + 2 + 4 + 8 + 16 = 31 s of retry waits, so
//! a stream of calls of `d` ends at about 31 s + 6 × `d`; the durations are
//! chosen so the planned ends are tens of seconds apart, far more than the
//! permit spacing adds. Everything runs on tokio's paused clock and nothing
//! connects to AWS (NFR6.1).

mod support;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use local_sights_core::coordinator::{FetchJob, run_fetch};
use local_sights_core::failure::{ApiFailure, FailureKind};
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::{StreamFetchOutcome, StreamStatus};
use local_sights_core::gateway::GetLogEventsPage;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{
    FixedJitter, Retrier, RetryPolicy, StopReason, StopSignal, stop_pair,
};
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page, stream_page};
use support::recording_sink::{Delivered, RecordingSink};

/// 2024/01/02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;

type Answer = Result<GetLogEventsPage, ApiFailure>;

/// What one stream's `GetLogEvents` calls answer.
enum Script {
    /// Fails with `kind` on the first call and on all five retries.
    Exhausts(FailureKind),
    /// Denied on the first call (not retried).
    Denied,
    /// Answers `pages` pages of one event each, the last without a token.
    Pages(usize),
}

impl Script {
    fn answers(&self, stream: &str, at: i64) -> Vec<Answer> {
        match self {
            Script::Exhausts(kind) => (0..6).map(|_| Err(failure(*kind))).collect(),
            Script::Denied => vec![Err(failure(FailureKind::AccessDenied))],
            Script::Pages(count) => (0..*count)
                .map(|n| {
                    let message = format!("{stream}-{n}");
                    let token = format!("f/{}", n + 1);
                    let last = n + 1 == *count;
                    page(
                        &[(at + n as i64, &message)],
                        (!last).then_some(token.as_str()),
                    )
                })
                .collect(),
        }
    }
}

/// One stream of the group: its name, script and per-call duration in ms.
struct Stream {
    name: String,
    script: Script,
    call_ms: u64,
}

fn stream(index: usize, script: Script, call_ms: u64) -> Stream {
    Stream {
        name: format!("s{index:02}"),
        script,
        call_ms,
    }
}

/// A group whose listing is `streams` in one page, in that order (the
/// selected order), each scripted with its own call duration.
fn group(streams: &[Stream]) -> FakeGateway {
    let listed: Vec<(&str, Option<i64>, Option<i64>)> = streams
        .iter()
        .map(|s| (s.name.as_str(), None, None))
        .collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for (i, s) in streams.iter().enumerate() {
        gateway = gateway
            .script_stream(&s.name, s.script.answers(&s.name, START + i as i64 * 10))
            .stream_call_duration(&s.name, Duration::from_millis(s.call_ms));
    }
    gateway
}

fn validated() -> ValidatedFetch {
    validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024/01/02 03:04:05".to_string(),
        end_text: "2024/01/02 03:05:05".to_string(),
    })
    .expect("test input is valid")
}

async fn run_with(
    gateway: &FakeGateway,
    timeline: &Mutex<EventTimeline>,
    signal: &StopSignal,
) -> (FetchJob, RecordingSink) {
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, signal);
    let mut sink = RecordingSink::default();
    let job = run_fetch(gateway, &validated(), timeline, &retrier, &mut sink).await;
    (job, sink)
}

async fn run(gateway: &FakeGateway, timeline: &Mutex<EventTimeline>) -> (FetchJob, RecordingSink) {
    let (_handle, signal) = stop_pair();
    run_with(gateway, timeline, &signal).await
}

fn finishing_order(job: &FetchJob) -> Vec<&str> {
    job.outcomes
        .iter()
        .map(|outcome| outcome.log_stream_name.as_str())
        .collect()
}

fn outcome<'a>(job: &'a FetchJob, name: &str) -> &'a StreamFetchOutcome {
    job.outcomes
        .iter()
        .find(|outcome| outcome.log_stream_name == name)
        .unwrap_or_else(|| panic!("no outcome for {name}"))
}

fn calls_per_stream(gateway: &FakeGateway) -> HashMap<String, usize> {
    let mut calls = HashMap::new();
    for name in gateway.fetched_streams() {
        *calls.entry(name).or_insert(0) += 1;
    }
    calls
}

fn names(range: std::ops::RangeInclusive<usize>) -> Vec<String> {
    range.map(|i| format!("s{i:02}")).collect()
}

/// AC2.3.1, AC2.3.2, FR3.1: the streams end in another order than they
/// were started (s02 before s01), and the count follows the finishing
/// order: s02, s01 exhausted (2), s03 succeeds (0), s04, s05 exhausted (2),
/// and s06 makes three in a row. Without the reset s04 would have been the
/// third. s14-s16 are not started then and are skipped in the selected
/// order with s06's failure.
#[tokio::test(start_paused = true)]
async fn the_count_follows_the_finishing_order_and_gives_up_at_three() {
    let mut streams = vec![
        stream(1, Script::Exhausts(FailureKind::Throttled), 5_000),
        stream(2, Script::Exhausts(FailureKind::Throttled), 100),
        stream(3, Script::Pages(1), 75_000),
        stream(4, Script::Exhausts(FailureKind::Throttled), 10_000),
        stream(5, Script::Exhausts(FailureKind::Throttled), 15_000),
        stream(6, Script::Exhausts(FailureKind::Network), 20_000),
        stream(7, Script::Pages(2), 200_000),
        stream(8, Script::Pages(2), 200_000),
    ];
    streams.extend((9..=16).map(|i| stream(i, Script::Pages(1), 300_000)));
    let gateway = group(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    let order = finishing_order(&job);
    assert_eq!(
        &order[..9],
        [
            "s02", "s01", "s03", "s04", "s05", "s06", "s14", "s15", "s16"
        ],
        "ended in this order, the skipped ones right after s06"
    );
    let third = outcome(&job, "s06").failure.clone().expect("s06 failed");
    assert_eq!(third.kind(), FailureKind::Network);
    for name in names(14..=16) {
        let skipped = outcome(&job, &name);
        assert_eq!(skipped.status, StreamStatus::Failed);
        assert_eq!(skipped.failure.as_ref(), Some(&third), "{name}");
        assert_eq!((skipped.page_count, skipped.retry_count), (0, 0));
    }
    let failed: Vec<&str> = job
        .failed_streams
        .iter()
        .map(|failed| failed.log_stream_name.as_str())
        .collect();
    assert_eq!(
        failed,
        ["s02", "s01", "s04", "s05", "s06", "s14", "s15", "s16"]
    );
    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(
        (job.finished_stream_count, job.planned_stream_count),
        (16, 16)
    );
    let calls = calls_per_stream(&gateway);
    for name in names(14..=16) {
        assert!(!calls.contains_key(&name), "{name} is never called");
    }
    for name in names(9..=13) {
        assert_eq!(calls.get(&name), Some(&1), "{name} was started and read");
    }
}

/// NFR3.7, NFR8.9, BR3.7: after the give-up no `GetLogEvents` call is made
/// for the streams not started, and each skipped stream is reported one by
/// one, in the selected order, right after the stream that reached three,
/// with exactly that stream's failure (no new text).
#[tokio::test(start_paused = true)]
async fn skipped_streams_are_never_called_and_carry_the_third_failure() {
    let mut streams = vec![
        stream(1, Script::Exhausts(FailureKind::Throttled), 100),
        stream(2, Script::Exhausts(FailureKind::Throttled), 5_000),
        stream(3, Script::Exhausts(FailureKind::Network), 10_000),
    ];
    streams.extend((4..=10).map(|i| stream(i, Script::Pages(1), 200_000)));
    streams.extend((11..=13).map(|i| stream(i, Script::Pages(1), 100)));
    let gateway = group(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;

    assert!(
        gateway.api_log().iter().all(|call| ![
            "GetLogEvents:s11",
            "GetLogEvents:s12",
            "GetLogEvents:s13"
        ]
        .contains(&call.as_str())),
        "{:?}",
        gateway.api_log()
    );
    let finished: Vec<(String, u32)> = sink
        .delivered
        .iter()
        .filter_map(|delivered| match delivered {
            Delivered::StreamFinished { name, finished, .. } => Some((name.clone(), *finished)),
            _ => None,
        })
        .collect();
    let expected: Vec<(String, u32)> = ["s01", "s02", "s03", "s11", "s12", "s13"]
        .iter()
        .zip(1..)
        .map(|(name, count)| ((*name).to_string(), count))
        .collect();
    assert_eq!(&finished[..6], expected.as_slice());
    let third = outcome(&job, "s03").failure.clone().expect("s03 failed");
    for name in names(11..=13) {
        let skipped = outcome(&job, &name);
        assert_eq!(skipped.failure.as_ref(), Some(&third));
        assert_eq!(
            skipped.failure.as_ref().map(ApiFailure::safe_detail),
            Some("kind=Network; api=GetLogEvents"),
            "only the safe detail of the third stream"
        );
    }
    assert_eq!(job.event_count, 7, "s04-s10 were read");
    assert_eq!(job.failed_streams.len(), 6);
}

/// AC2.3.3, FR3.3, BR3.3: the streams being read at the give-up are read to
/// their end and recorded as they end: a stream of three pages completes
/// with all its events, one that later uses up its retries and one that is
/// denied are listed with their own failures, and no second give-up
/// happens (the two skipped streams are reported once).
#[tokio::test(start_paused = true)]
async fn streams_being_read_at_the_give_up_are_read_to_their_end() {
    let streams = vec![
        stream(1, Script::Exhausts(FailureKind::Throttled), 100),
        stream(2, Script::Exhausts(FailureKind::Throttled), 5_000),
        stream(3, Script::Exhausts(FailureKind::Throttled), 10_000),
        stream(4, Script::Pages(3), 40_000),
        stream(5, Script::Exhausts(FailureKind::Throttled), 30_000),
        stream(6, Script::Denied, 150_000),
        stream(7, Script::Pages(1), 120_000),
        stream(8, Script::Pages(1), 120_000),
        stream(9, Script::Pages(1), 100_000),
        stream(10, Script::Pages(1), 100_000),
        stream(11, Script::Pages(1), 100),
        stream(12, Script::Pages(1), 100),
    ];
    let gateway = group(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(
        &finishing_order(&job)[..5],
        ["s01", "s02", "s03", "s11", "s12"]
    );
    let long = outcome(&job, "s04");
    assert_eq!(long.status, StreamStatus::Completed);
    assert_eq!((long.event_count, long.page_count), (3, 3));
    let held: Vec<String> = timeline
        .lock()
        .unwrap()
        .events()
        .iter()
        .filter(|event| event.log_stream_name == "s04")
        .map(|event| event.message.clone())
        .collect();
    assert_eq!(held, ["s04-0", "s04-1", "s04-2"], "every page is kept");

    let late = outcome(&job, "s05");
    assert_eq!(late.status, StreamStatus::Failed);
    assert_eq!(late.retry_count, 5, "read to its own end");
    assert_eq!(
        late.failure.as_ref().map(ApiFailure::kind),
        Some(FailureKind::Throttled)
    );
    assert_eq!(
        outcome(&job, "s06").failure.as_ref().map(ApiFailure::kind),
        Some(FailureKind::AccessDenied)
    );
    assert_eq!(
        job.outcomes.len(),
        12,
        "each stream once, no second give-up"
    );
    let skipped = job
        .outcomes
        .iter()
        .filter(|o| o.status == StreamStatus::Failed && o.page_count == 0 && o.retry_count == 0)
        .filter(|o| o.failure.as_ref().map(ApiFailure::kind) == Some(FailureKind::Throttled))
        .count();
    assert_eq!(skipped, 2, "s11 and s12 only");
    assert_eq!(job.event_count, 3 + 4, "s04's three and s07-s10");
    assert_eq!(job.status, JobStatus::CompletedWithFailures);
}

/// AC2.3.4, FR3.4, BR3.4: a denied stream between exhausted ones resets the
/// count, so s01, s02, (s03 denied), s04, s05 never make three in a row and
/// every selected stream is called, also the ones started last.
#[tokio::test(start_paused = true)]
async fn a_denied_stream_between_exhausted_ones_resets_the_count() {
    let mut streams = vec![
        stream(1, Script::Exhausts(FailureKind::Throttled), 100),
        stream(2, Script::Exhausts(FailureKind::Throttled), 5_000),
        stream(3, Script::Denied, 75_000),
        stream(4, Script::Exhausts(FailureKind::Throttled), 10_000),
        stream(5, Script::Exhausts(FailureKind::Throttled), 15_000),
    ];
    streams.extend((6..=13).map(|i| stream(i, Script::Pages(1), 200_000)));
    let gateway = group(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(
        &finishing_order(&job)[..5],
        ["s01", "s02", "s03", "s04", "s05"]
    );
    let calls = calls_per_stream(&gateway);
    for name in names(1..=13) {
        assert!(calls.contains_key(&name), "{name} was called");
    }
    let failed: Vec<&str> = job
        .failed_streams
        .iter()
        .map(|failed| failed.log_stream_name.as_str())
        .collect();
    assert_eq!(
        failed,
        ["s01", "s02", "s03", "s04", "s05"],
        "nothing skipped"
    );
    assert_eq!(job.event_count, 8);
    assert_eq!(job.status, JobStatus::CompletedWithFailures);
}

/// BR3.5, U1:BR1.16: closing the window after two exhausted streams ends the
/// fetch as Aborted; the streams stopped by the close are not counted and
/// nothing is skipped or called afterwards.
#[tokio::test(start_paused = true)]
async fn a_close_after_two_exhausted_streams_is_still_aborted() {
    let mut streams = vec![
        stream(1, Script::Exhausts(FailureKind::Throttled), 100),
        stream(2, Script::Exhausts(FailureKind::Throttled), 5_000),
        stream(3, Script::Exhausts(FailureKind::Throttled), 10_000),
    ];
    streams.extend((4..=12).map(|i| stream(i, Script::Pages(1), 200_000)));
    let gateway = group(&streams);
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = stop_pair();
    let fetching = run_with(&gateway, &timeline, &signal);
    let close_later = async {
        tokio::time::sleep(Duration::from_secs(70)).await;
        handle.stop(StopReason::Close);
    };
    let ((job, sink), ()) = tokio::join!(fetching, close_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert_eq!(
        finishing_order(&job),
        ["s01", "s02"],
        "nothing after the close"
    );
    let reported: Vec<&str> = sink
        .delivered
        .iter()
        .filter_map(|delivered| match delivered {
            Delivered::StreamFinished { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reported, ["s01", "s02"]);
    let calls = calls_per_stream(&gateway);
    assert!(!calls.contains_key("s11") && !calls.contains_key("s12"));
    assert!(
        calls.get("s03").is_some_and(|count| *count < 6),
        "s03 stopped during a retry wait"
    );
    assert!(
        timeline.lock().unwrap().is_empty(),
        "the logs are discarded"
    );
}
