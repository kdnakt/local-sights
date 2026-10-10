//! U3 fetch flow against a fake gateway: listing and selecting the streams
//! (BR1.1-BR1.5), retries (BR2.1-BR2.3), one stream after another (BR3.1-
//! BR3.3), merging by time (BR4.1, BR4.2) and the job result, progress and
//! abort (BR5.1-BR5.5), including the three FR4.10 acceptance criteria.
//! Waits run on tokio's paused clock, so no test waits in real time, and
//! nothing here connects to AWS.

mod support;

use std::sync::Mutex;
use std::time::Duration;

use local_sights_core::coordinator::{FetchJob, JobStatus, run_fetch};
use local_sights_core::failure::FailureKind;
use local_sights_core::fetcher::StreamStatus;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{AbortSignal, FixedJitter, Retrier, RetryPolicy, abort_pair};
use local_sights_core::streams::StreamListingStatus;
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page, stream_page};
use support::recording_sink::{Delivered, RecordingSink};
use tokio::time::Instant;

/// 2024-01-02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;
/// 2024-01-02 03:05:05.999 UTC (end of the range).
const END: i64 = START + 60_999;
/// One millisecond older than the listing margin (BR1.2).
const TOO_OLD: i64 = START - 3_600_000 - 1;

fn validated() -> ValidatedFetch {
    validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024-01-02 03:04:05".to_string(),
        end_text: "2024-01-02 03:05:05".to_string(),
    })
    .expect("test input is valid")
}

async fn run_with(
    gateway: &FakeGateway,
    timeline: &Mutex<EventTimeline>,
    jitter: f64,
    signal: &AbortSignal,
) -> (FetchJob, RecordingSink) {
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(jitter);
    let retrier = Retrier::new(&policy, &jitter, signal);
    let mut sink = RecordingSink::default();
    let job = run_fetch(gateway, &validated(), timeline, &retrier, &mut sink).await;
    (job, sink)
}

async fn run(gateway: &FakeGateway, timeline: &Mutex<EventTimeline>) -> (FetchJob, RecordingSink) {
    let (_handle, signal) = abort_pair();
    run_with(gateway, timeline, 0.0, &signal).await
}

/// `(stream, message)` of every held event, in timeline order.
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

fn throttled_times(
    count: usize,
) -> Vec<Result<local_sights_core::gateway::GetLogEventsPage, local_sights_core::failure::ApiFailure>>
{
    (0..count)
        .map(|_| Err(failure(FailureKind::Throttled)))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn streams_are_fetched_one_by_one_and_merged_by_time_then_stream_name() {
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(
            &[("b", Some(START), Some(END)), ("a", Some(START), Some(END))],
            Some("n1"),
        ),
        stream_page(&[("c", None, None)], None),
    ])
    .script_stream(
        "b",
        vec![
            page(&[(START + 1, "b0"), (START + 3, "b1")], Some("f/1")),
            page(&[(START + 5, "b2")], Some("f/1")),
        ],
    )
    .script_stream("a", vec![page(&[(START, "a0"), (START + 3, "a1")], None)])
    .script_stream("c", vec![page(&[(START + 3, "c0")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.planned_stream_count, 3);
    assert_eq!(job.finished_stream_count, 3);
    assert_eq!(job.event_count, 6);
    assert_eq!(job.listing_status, Some(StreamListingStatus::Complete));
    assert_eq!(
        held(&timeline),
        vec![
            pair("a", "a0"),
            pair("b", "b0"),
            pair("a", "a1"),
            pair("b", "b1"),
            pair("c", "c0"),
            pair("b", "b2"),
        ]
    );
    // One stream after another, in listing order (BR3.1).
    assert_eq!(
        gateway.api_log(),
        vec![
            "DescribeLogStreams",
            "DescribeLogStreams",
            "GetLogEvents:b",
            "GetLogEvents:b",
            "GetLogEvents:a",
            "GetLogEvents:c",
        ]
    );
    let tokens: Vec<_> = gateway
        .stream_calls()
        .into_iter()
        .map(|call| (call.log_group_name, call.next_token))
        .collect();
    assert_eq!(
        tokens,
        vec![
            ("/aws/lambda/orders".to_string(), None),
            ("/aws/lambda/orders".to_string(), Some("n1".to_string())),
        ]
    );
    // Sequences restart from 0 in each stream (BR3.2).
    let held = timeline.lock().unwrap();
    let b_sequences: Vec<u64> = held
        .events()
        .iter()
        .filter(|e| e.log_stream_name == "b")
        .map(|e| e.sequence)
        .collect();
    assert_eq!(b_sequences, vec![0, 1, 2]);
    assert_eq!(held.position_of("a", 1), Some(2));
}

#[tokio::test(start_paused = true)]
async fn empty_pages_and_a_group_without_streams_complete() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[("a", None, None)], None)])
        .script_stream(
            "a",
            vec![
                page(&[], Some("f/1")),
                page(&[], Some("f/2")),
                page(&[], Some("f/2")),
            ],
        );
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 0);
    assert_eq!(job.outcomes[0].page_count, 3);
    assert!(
        !sink.kinds().contains(&"Batch"),
        "an empty page adds nothing"
    );

    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[], None)]);
    let (job, _) = run(&gateway, &timeline).await;
    assert_eq!(
        job.status,
        JobStatus::Completed,
        "no stream is still Completed"
    );
    assert_eq!(job.planned_stream_count, 0);
    assert!(gateway.calls().is_empty());
}

#[tokio::test(start_paused = true)]
async fn an_old_stream_stops_the_listing_without_calling_the_next_page() {
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[
            ("new", Some(START), Some(END)),
            ("old", Some(0), Some(TOO_OLD)),
            ("no-times", None, None),
            ("older", Some(0), Some(TOO_OLD - 5)),
        ],
        Some("n1"),
    )])
    .script_stream("new", vec![page(&[(START, "n")], None)])
    .script_stream("no-times", vec![page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(gateway.stream_calls().len(), 1);
    assert_eq!(gateway.fetched_streams(), vec!["new", "no-times"]);
    assert_eq!(job.listing_status, Some(StreamListingStatus::StoppedEarly));
    assert_eq!(job.status, JobStatus::Completed);
}

#[tokio::test(start_paused = true)]
async fn a_listing_failing_on_its_first_page_fails_the_fetch() {
    let gateway = FakeGateway::with_stream_pages(vec![Err(failure(FailureKind::AccessDenied))]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.listing_status, Some(StreamListingStatus::Partial));
    assert_eq!(
        job.listing_failure.as_ref().map(|f| f.kind()),
        Some(FailureKind::AccessDenied)
    );
    assert!(gateway.calls().is_empty());
    assert_eq!(
        gateway.stream_calls().len(),
        1,
        "AccessDenied is not retried"
    );
}

#[tokio::test(start_paused = true)]
async fn a_listing_failing_later_fetches_what_was_selected_and_completes_with_failures() {
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(&[("a", Some(START), Some(END))], Some("n1")),
        Err(failure(FailureKind::NotFound)),
    ])
    .script_stream("a", vec![page(&[(START, "a0")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(job.listing_status, Some(StreamListingStatus::Partial));
    assert!(job.failed_streams.is_empty());
    assert_eq!(held(&timeline), vec![pair("a", "a0")]);
}

#[tokio::test(start_paused = true)]
async fn a_throttled_listing_is_retried_with_the_same_token() {
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(&[("a", None, None)], Some("n1")),
        Err(failure(FailureKind::Throttled)),
        stream_page(&[], None),
    ])
    .script_stream("a", vec![page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let started = Instant::now();
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(started.elapsed(), Duration::from_secs(1));
    let tokens: Vec<_> = gateway
        .stream_calls()
        .into_iter()
        .map(|call| call.next_token)
        .collect();
    assert_eq!(
        tokens,
        vec![None, Some("n1".to_string()), Some("n1".to_string())]
    );
}

/// FR4.10 acceptance criterion 1.
#[tokio::test(start_paused = true)]
async fn throttling_twice_then_succeeding_waits_one_and_two_seconds_and_completes() {
    let mut first = throttled_times(2);
    first.push(page(&[(START, "a0")], None));
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[("a", None, None), ("b", None, None)],
        None,
    )])
    .script_stream("a", first)
    .script_stream("b", vec![page(&[(START + 1, "b0")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let started = Instant::now();
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert!(job.failed_streams.is_empty());
    assert_eq!(started.elapsed(), Duration::from_secs(3));
    assert_eq!(job.outcomes[0].retry_count, 2);
    assert_eq!(job.outcomes[1].retry_count, 0);
    assert_eq!(held(&timeline), vec![pair("a", "a0"), pair("b", "b0")]);
    // The same request (same token) is sent again (BR2.1).
    let tokens: Vec<_> = gateway.calls().into_iter().map(|c| c.next_token).collect();
    assert_eq!(tokens, vec![None, None, None, None]);
}

#[tokio::test(start_paused = true)]
async fn the_waits_are_spread_by_the_jitter() {
    let mut first = throttled_times(2);
    first.push(page(&[], None));
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&[("a", None, None)], None)])
        .script_stream("a", first);
    let timeline = Mutex::new(EventTimeline::new());
    let (_handle, signal) = abort_pair();
    let started = Instant::now();
    run_with(&gateway, &timeline, 0.2, &signal).await;
    assert_eq!(started.elapsed(), Duration::from_millis(1_200 + 2_400));
}

/// FR4.10 acceptance criterion 2.
#[tokio::test(start_paused = true)]
async fn a_stream_that_uses_up_its_retries_fails_and_keeps_its_pages() {
    let mut first = vec![page(&[(START, "a0")], Some("f/1"))];
    first.extend(throttled_times(6));
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[("a", None, None), ("b", None, None)],
        None,
    )])
    .script_stream("a", first)
    .script_stream("b", vec![page(&[(START + 1, "b0")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(job.failed_streams.len(), 1);
    assert_eq!(job.failed_streams[0].log_stream_name, "a");
    assert_eq!(job.failed_streams[0].failure.kind(), FailureKind::Throttled);
    assert_eq!(job.outcomes[0].status, StreamStatus::Failed);
    assert_eq!(job.outcomes[0].retry_count, 5);
    assert_eq!(job.outcomes[0].page_count, 1);
    assert_eq!(held(&timeline), vec![pair("a", "a0"), pair("b", "b0")]);
    assert_eq!(
        gateway
            .fetched_streams()
            .iter()
            .filter(|s| *s == "a")
            .count(),
        7
    );
}

/// FR4.10 acceptance criterion 3 (review R-03, R-09).
#[tokio::test(start_paused = true)]
async fn three_exhausted_streams_in_a_row_fail_the_rest_without_calling() {
    let names = ["s1", "s2", "s3", "s4", "s5"];
    let listed: Vec<(&str, Option<i64>, Option<i64>)> =
        names.iter().map(|name| (*name, None, None)).collect();
    let mut gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)]);
    for name in &names[..3] {
        gateway = gateway.script_stream(name, throttled_times(6));
    }
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(job.failed_streams.len(), 5);
    assert_eq!(job.finished_stream_count, 5);
    assert_eq!(job.planned_stream_count, 5);
    let fetched = gateway.fetched_streams();
    assert!(!fetched.contains(&"s4".to_string()) && !fetched.contains(&"s5".to_string()));
    for skipped in &job.outcomes[3..] {
        assert_eq!(skipped.page_count, 0);
        assert_eq!(skipped.retry_count, 0);
        assert_eq!(skipped.status, StreamStatus::Failed);
        assert_eq!(
            skipped.failure, job.outcomes[2].failure,
            "the last stream's failure"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_failure_without_exhaustion_resets_the_streak() {
    let names = ["s1", "s2", "s3", "s4", "s5"];
    let listed: Vec<(&str, Option<i64>, Option<i64>)> =
        names.iter().map(|name| (*name, None, None)).collect();
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(&listed, None)])
        .script_stream("s1", throttled_times(6))
        .script_stream("s2", throttled_times(6))
        .script_stream("s3", vec![Err(failure(FailureKind::AccessDenied))])
        .script_stream("s4", throttled_times(6))
        .script_stream("s5", vec![page(&[(START, "ok")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.failed_streams.len(), 4);
    assert_eq!(held(&timeline), vec![pair("s5", "ok")]);
}

#[tokio::test(start_paused = true)]
async fn an_abort_during_a_wait_stops_at_once_and_discards_the_logs() {
    let mut first = vec![page(&[(START, "a0")], Some("f/1"))];
    first.extend(throttled_times(1));
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[("a", None, None), ("b", None, None)],
        None,
    )])
    .script_stream("a", first);
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = abort_pair();
    let fetch = run_with(&gateway, &timeline, 0.0, &signal);
    let abort_later = async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        handle.abort();
    };
    let ((job, sink), ()) = tokio::join!(fetch, abort_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert!(
        timeline.lock().unwrap().is_empty(),
        "BR5.5 discards the logs"
    );
    assert_eq!(
        sink.finished_timeline_version,
        Some(timeline.lock().unwrap().version()),
        "the version after the discard is reported (review R-02)"
    );
    assert_eq!(
        gateway.api_log().len(),
        3,
        "listing, page 1, the throttled call"
    );
    assert!(!gateway.fetched_streams().contains(&"b".to_string()));
    assert!(matches!(
        sink.delivered.last(),
        Some(Delivered::Finished(job)) if job.status == JobStatus::Aborted
    ));
}

#[tokio::test(start_paused = true)]
async fn an_abort_while_listing_fetches_no_stream() {
    let gateway = FakeGateway::with_stream_pages(vec![Err(failure(FailureKind::Network))]);
    let timeline = Mutex::new(EventTimeline::new());
    let (handle, signal) = abort_pair();
    let fetch = run_with(&gateway, &timeline, 0.0, &signal);
    let abort_later = async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.abort();
    };
    let ((job, sink), ()) = tokio::join!(fetch, abort_later);

    assert_eq!(job.status, JobStatus::Aborted);
    assert!(gateway.calls().is_empty());
    assert_eq!(sink.kinds(), vec!["Started", "Finished"]);
}

#[tokio::test(start_paused = true)]
async fn the_sink_receives_progress_in_order_with_cumulative_counts() {
    let gateway = FakeGateway::with_stream_pages(vec![
        stream_page(
            &[("a", None, None), ("skip", Some(END + 1), Some(END + 2))],
            Some("n1"),
        ),
        stream_page(&[("b", None, None)], None),
    ])
    .script_stream("a", vec![page(&[(START, "a0"), (START + 1, "a1")], None)])
    .script_stream("b", vec![Err(failure(FailureKind::AccessDenied))]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;

    assert_eq!(
        sink.kinds(),
        vec![
            "Started",
            "Listing",
            "Listing",
            "Planned",
            "Batch",
            "StreamFinished",
            "StreamFinished",
            "Finished"
        ]
    );
    assert_eq!(
        sink.delivered[1],
        Delivered::Listing {
            seen: 2,
            selected: 1
        }
    );
    assert_eq!(
        sink.delivered[2],
        Delivered::Listing {
            seen: 3,
            selected: 2
        }
    );
    assert_eq!(sink.delivered[3], Delivered::Planned { planned: 2 });
    let version = timeline.lock().unwrap().version();
    assert_eq!(
        sink.delivered[4],
        Delivered::Batch {
            added: 2,
            total: 2,
            timeline_version: version
        }
    );
    assert_eq!(
        sink.delivered[6],
        Delivered::StreamFinished {
            name: "b".to_string(),
            status: StreamStatus::Failed,
            finished: 2
        }
    );
    assert_eq!(job.status, JobStatus::CompletedWithFailures);
}
