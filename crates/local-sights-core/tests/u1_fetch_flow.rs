//! U1 fetch flow (EventFetcher + FetchCoordinator) against a fake gateway,
//! updated for U3: the fetch covers the whole log group, so every test
//! lists one stream first. The U1 rules that U3 did not replace (paging,
//! the range on every call, the connection, discarding the previous logs)
//! are still checked here; the U3 rules are in `u3_fetch_flow.rs`.
//! Nothing here connects to AWS.

mod support;

use std::sync::Mutex;

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{FetchJob, JobStatus, run_fetch};
use local_sights_core::failure::FailureKind;
use local_sights_core::fetcher::StreamStatus;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{FixedJitter, Retrier, RetryPolicy, abort_pair};
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page};
use support::recording_sink::{Delivered, RecordingSink};

/// 2024-01-02 03:04:05 UTC in epoch milliseconds.
const START: i64 = 1_704_164_645_000;

fn validated(profile: &str) -> ValidatedFetch {
    validate_fetch_input(&FetchInput {
        profile_name: profile.to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024-01-02 03:04:05".to_string(),
        end_text: "2024-01-02 03:05:05".to_string(),
    })
    .expect("test input is valid")
}

async fn run_with(
    gateway: &FakeGateway,
    fetch: &ValidatedFetch,
    timeline: &Mutex<EventTimeline>,
) -> (FetchJob, RecordingSink) {
    let policy = RetryPolicy::default();
    let (_handle, signal) = abort_pair();
    let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
    let mut sink = RecordingSink::default();
    let job = run_fetch(gateway, fetch, timeline, &retrier, &mut sink).await;
    (job, sink)
}

async fn run(gateway: &FakeGateway, timeline: &Mutex<EventTimeline>) -> (FetchJob, RecordingSink) {
    run_with(gateway, &validated("dev"), timeline).await
}

fn messages(timeline: &Mutex<EventTimeline>) -> Vec<String> {
    timeline
        .lock()
        .unwrap()
        .events()
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

#[tokio::test]
async fn reads_every_page_including_empty_ones_until_the_same_token_returns() {
    let gateway = FakeGateway::with_single_stream(
        "stream-a",
        vec![
            page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
            page(&[], Some("f/2")),
            page(&[(START + 2, "c\nmultiline")], Some("f/3")),
            page(&[], Some("f/3")),
        ],
    );
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 3);
    assert!(job.failed_streams.is_empty());
    assert_eq!(job.outcomes[0].page_count, 4);
    assert_eq!(messages(&timeline), vec!["a", "b", "c\nmultiline"]);
    let held = timeline.lock().unwrap();
    let sequences: Vec<u64> = held.events().iter().map(|e| e.sequence).collect();
    assert_eq!(sequences, vec![0, 1, 2]);
    assert!(
        held.events()
            .iter()
            .all(|e| e.log_stream_name == "stream-a")
    );
}

#[tokio::test]
async fn stops_when_the_first_response_has_no_token() {
    let gateway = FakeGateway::with_single_stream("stream-a", vec![page(&[(START, "only")], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    let (job, _) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.outcomes[0].page_count, 1);
    assert_eq!(gateway.calls().len(), 1);
    assert_eq!(messages(&timeline), vec!["only"]);
}

#[tokio::test]
async fn sends_the_range_and_oldest_first_on_every_call_and_follows_tokens() {
    let gateway = FakeGateway::with_single_stream(
        "stream-a",
        vec![
            page(&[(START, "a")], Some("f/1")),
            page(&[(START + 1, "b")], Some("f/2")),
            page(&[], Some("f/2")),
        ],
    );
    let timeline = Mutex::new(EventTimeline::new());
    run(&gateway, &timeline).await;

    let calls = gateway.calls();
    assert_eq!(calls.len(), 3);
    for call in &calls {
        assert_eq!(call.start_time, START);
        assert_eq!(call.end_time, START + 60_999 + 1);
        assert!(call.start_from_head);
        assert_eq!(call.profile_name.as_deref(), Some("dev"));
        assert_eq!(call.log_group_name, "/aws/lambda/orders");
        assert_eq!(call.log_stream_name, "stream-a");
    }
    let tokens: Vec<Option<&str>> = calls.iter().map(|c| c.next_token.as_deref()).collect();
    assert_eq!(tokens, vec![None, Some("f/1"), Some("f/2")]);
}

/// U1:BR4.1, as extended by U3:BR3.3 and BR5.2: a stream that fails keeps
/// its fetched events and the job completes with failures.
#[tokio::test]
async fn an_error_mid_way_keeps_fetched_events_and_marks_the_stream_failed() {
    let gateway = FakeGateway::with_single_stream(
        "stream-a",
        vec![
            page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
            Err(failure(FailureKind::AccessDenied)),
        ],
    );
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;

    assert_eq!(job.status, JobStatus::CompletedWithFailures);
    assert_eq!(job.failed_streams.len(), 1);
    assert_eq!(job.event_count, 2);
    let outcome = &job.outcomes[0];
    assert_eq!(outcome.page_count, 1);
    assert_eq!(outcome.status, StreamStatus::Failed);
    assert_eq!(
        job.failed_streams[0].failure.kind(),
        FailureKind::AccessDenied
    );
    assert_eq!(messages(&timeline), vec!["a", "b"]);
    assert!(matches!(
        sink.delivered.last(),
        Some(Delivered::Finished(_))
    ));
}

#[tokio::test]
async fn region_missing_fails_without_calling_any_api() {
    let gateway = FakeGateway::failing_to_connect(failure(FailureKind::RegionMissing));
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;

    assert!(gateway.calls().is_empty());
    assert!(gateway.api_log().is_empty());
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.event_count, 0);
    assert_eq!(
        job.failure().map(|f| f.kind()),
        Some(FailureKind::RegionMissing)
    );
    assert!(timeline.lock().unwrap().is_empty());
    assert_eq!(sink.kinds(), vec!["Started", "Planned", "Finished"]);
}

#[tokio::test]
async fn sink_receives_started_then_cumulative_batches_then_finished() {
    let gateway = FakeGateway::with_single_stream(
        "stream-a",
        vec![
            page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
            page(&[(START + 2, "c")], Some("f/1")),
        ],
    );
    let timeline = Mutex::new(EventTimeline::new());
    let (job, sink) = run(&gateway, &timeline).await;

    let first_version = match sink.delivered[3] {
        Delivered::Batch {
            timeline_version, ..
        } => timeline_version,
        ref other => panic!("expected a batch, got {other:?}"),
    };
    assert_eq!(
        sink.delivered,
        vec![
            Delivered::Started {
                job_id: job.job_id,
                timeline_version: first_version - 1
            },
            Delivered::Listing {
                seen: 1,
                selected: 1
            },
            Delivered::Planned { planned: 1 },
            Delivered::Batch {
                added: 2,
                total: 2,
                timeline_version: first_version
            },
            Delivered::Batch {
                added: 1,
                total: 3,
                timeline_version: first_version + 1
            },
            Delivered::StreamFinished {
                name: "stream-a".to_string(),
                status: StreamStatus::Completed,
                finished: 1
            },
            Delivered::Finished(Box::new(job.clone())),
        ]
    );
}

#[tokio::test]
async fn fetching_again_discards_the_previous_logs() {
    let timeline = Mutex::new(EventTimeline::new());
    let first = FakeGateway::with_single_stream("stream-a", vec![page(&[(START, "old")], None)]);
    let (first_job, _) = run(&first, &timeline).await;
    assert_eq!(messages(&timeline), vec!["old"]);

    let second =
        FakeGateway::with_single_stream("stream-a", vec![page(&[(START + 9, "new")], None)]);
    let (second_job, _) = run(&second, &timeline).await;
    assert_eq!(messages(&timeline), vec!["new"]);
    assert_eq!(timeline.lock().unwrap().events()[0].sequence, 0);
    assert!(second_job.job_id > first_job.job_id);
}

#[tokio::test]
async fn blank_profile_is_passed_as_sdk_default() {
    let gateway = FakeGateway::with_single_stream("stream-a", vec![page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    run_with(&gateway, &validated("  "), &timeline).await;
    assert_eq!(gateway.calls()[0].profile_name, None);
    assert_eq!(gateway.stream_calls()[0].profile_name, None);
}

/// U2:BR2.8 — a fetch from the screen connects with the selected profile
/// and the selected region on every call, the listing included.
#[tokio::test]
async fn screen_connection_passes_the_selected_profile_and_region() {
    let gateway = FakeGateway::with_single_stream(
        "stream-a",
        vec![page(&[(START, "a")], Some("f/1")), page(&[], Some("f/1"))],
    );
    let mut fetch = validated("");
    fetch.request = fetch.request.with_connection(
        ProfileSelector::Named("prod".to_string()),
        "eu-west-1".to_string(),
    );
    let timeline = Mutex::new(EventTimeline::new());
    run_with(&gateway, &fetch, &timeline).await;

    let calls = gateway.calls();
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert_eq!(call.profile_name.as_deref(), Some("prod"));
        assert_eq!(call.region.as_deref(), Some("eu-west-1"));
    }
    let listing = &gateway.stream_calls()[0];
    assert_eq!(listing.profile_name.as_deref(), Some("prod"));
    assert_eq!(listing.region.as_deref(), Some("eu-west-1"));
}

/// U2:BR2.8 with the SDK default: no profile is passed, the region is.
#[tokio::test]
async fn screen_connection_with_sdk_default_passes_no_profile() {
    let gateway = FakeGateway::with_single_stream("stream-a", vec![page(&[], None)]);
    let mut fetch = validated("dev");
    fetch.request = fetch
        .request
        .with_connection(ProfileSelector::SdkDefault, "us-east-1".to_string());
    let timeline = Mutex::new(EventTimeline::new());
    run_with(&gateway, &fetch, &timeline).await;

    assert_eq!(gateway.calls()[0].profile_name, None);
    assert_eq!(gateway.calls()[0].region.as_deref(), Some("us-east-1"));
}

/// The fetch_check path (no screen connection) leaves the region unset so
/// the profile's default region is used (U2:BR2.8, R-11).
#[tokio::test]
async fn fetch_check_path_passes_no_region() {
    let gateway = FakeGateway::with_single_stream("stream-a", vec![page(&[], None)]);
    let timeline = Mutex::new(EventTimeline::new());
    run(&gateway, &timeline).await;
    assert_eq!(gateway.calls()[0].region, None);
    assert_eq!(gateway.stream_calls()[0].region, None);
}
