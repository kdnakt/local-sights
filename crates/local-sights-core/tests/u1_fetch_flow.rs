//! U1 fetch flow (EventFetcher + FetchCoordinator) against a fake gateway.
//! Nothing here connects to AWS.

mod support;

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::failure::FailureKind;
use local_sights_core::fetcher::StreamStatus;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::timeline::EventTimeline;
use support::fake_gateway::{FakeGateway, failure, page};

/// 2024-01-02 03:04:05 UTC in epoch milliseconds.
const START: i64 = 1_704_164_645_000;

#[derive(Debug, Clone, PartialEq)]
enum Delivered {
    Started {
        job_id: String,
    },
    Batch {
        job_id: String,
        sequences: Vec<u64>,
        total: u64,
    },
    Finished(Box<FetchJob>),
}

#[derive(Debug, Default)]
struct RecordingSink {
    delivered: Vec<Delivered>,
}

impl FetchSink for RecordingSink {
    fn on_started(&mut self, job: &FetchJob) {
        assert_eq!(job.status, JobStatus::Running);
        self.delivered.push(Delivered::Started {
            job_id: job.job_id.clone(),
        });
    }

    fn on_batch(&mut self, job_id: &str, events: &[LogEvent], total: u64) {
        self.delivered.push(Delivered::Batch {
            job_id: job_id.to_string(),
            sequences: events.iter().map(|event| event.sequence).collect(),
            total,
        });
    }

    fn on_finished(&mut self, job: &FetchJob) {
        self.delivered
            .push(Delivered::Finished(Box::new(job.clone())));
    }
}

fn validated(profile: &str) -> ValidatedFetch {
    validate_fetch_input(&FetchInput {
        profile_name: profile.to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        log_stream_name: "stream-a".to_string(),
        start_text: "2024-01-02 03:04:05".to_string(),
        end_text: "2024-01-02 03:05:05".to_string(),
    })
    .expect("test input is valid")
}

async fn run(gateway: &FakeGateway, timeline: &mut EventTimeline) -> (FetchJob, RecordingSink) {
    let mut sink = RecordingSink::default();
    let job = run_fetch(gateway, &validated("dev"), timeline, &mut sink).await;
    (job, sink)
}

fn messages(timeline: &EventTimeline) -> Vec<String> {
    timeline
        .events()
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

#[tokio::test]
async fn reads_every_page_including_empty_ones_until_the_same_token_returns() {
    let gateway = FakeGateway::new(vec![
        page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
        page(&[], Some("f/2")),
        page(&[(START + 2, "c\nmultiline")], Some("f/3")),
        page(&[], Some("f/3")),
    ]);
    let mut timeline = EventTimeline::new();
    let (job, _) = run(&gateway, &mut timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.event_count, 3);
    assert_eq!(job.failed_stream_count, 0);
    assert_eq!(job.page_count(), 4);
    assert_eq!(messages(&timeline), vec!["a", "b", "c\nmultiline"]);
    let sequences: Vec<u64> = timeline.events().iter().map(|e| e.sequence).collect();
    assert_eq!(sequences, vec![0, 1, 2]);
    assert!(
        timeline
            .events()
            .iter()
            .all(|e| e.log_stream_name == "stream-a")
    );
}

#[tokio::test]
async fn stops_when_the_first_response_has_no_token() {
    let gateway = FakeGateway::new(vec![page(&[(START, "only")], None)]);
    let mut timeline = EventTimeline::new();
    let (job, _) = run(&gateway, &mut timeline).await;

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.page_count(), 1);
    assert_eq!(gateway.calls().len(), 1);
    assert_eq!(messages(&timeline), vec!["only"]);
}

#[tokio::test]
async fn sends_the_range_and_oldest_first_on_every_call_and_follows_tokens() {
    let gateway = FakeGateway::new(vec![
        page(&[(START, "a")], Some("f/1")),
        page(&[(START + 1, "b")], Some("f/2")),
        page(&[], Some("f/2")),
    ]);
    let mut timeline = EventTimeline::new();
    run(&gateway, &mut timeline).await;

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

#[tokio::test]
async fn an_error_mid_way_keeps_fetched_events_and_fails_the_job() {
    let gateway = FakeGateway::new(vec![
        page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
        Err(failure(FailureKind::Throttled)),
    ]);
    let mut timeline = EventTimeline::new();
    let (job, sink) = run(&gateway, &mut timeline).await;

    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.failed_stream_count, 1);
    assert_eq!(job.event_count, 2);
    assert_eq!(job.page_count(), 1);
    let outcome = job.outcome.as_ref().expect("finished job has an outcome");
    assert_eq!(outcome.status, StreamStatus::Failed);
    assert_eq!(
        job.failure().map(|f| f.kind()),
        Some(FailureKind::Throttled)
    );
    assert_eq!(messages(&timeline), vec!["a", "b"]);
    assert!(matches!(
        sink.delivered.last(),
        Some(Delivered::Finished(_))
    ));
}

#[tokio::test]
async fn region_missing_fails_without_calling_get_log_events() {
    let gateway = FakeGateway::failing_to_connect(failure(FailureKind::RegionMissing));
    let mut timeline = EventTimeline::new();
    let (job, sink) = run(&gateway, &mut timeline).await;

    assert!(gateway.calls().is_empty());
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.page_count(), 0);
    assert_eq!(job.event_count, 0);
    assert_eq!(
        job.failure().map(|f| f.kind()),
        Some(FailureKind::RegionMissing)
    );
    assert!(timeline.is_empty());
    assert_eq!(sink.delivered.len(), 2, "started then finished, no batch");
}

#[tokio::test]
async fn sink_receives_started_then_cumulative_batches_then_finished() {
    let gateway = FakeGateway::new(vec![
        page(&[(START, "a"), (START + 1, "b")], Some("f/1")),
        page(&[(START + 2, "c")], Some("f/1")),
    ]);
    let mut timeline = EventTimeline::new();
    let (job, sink) = run(&gateway, &mut timeline).await;

    let id = job.job_id.clone();
    assert_eq!(
        sink.delivered,
        vec![
            Delivered::Started { job_id: id.clone() },
            Delivered::Batch {
                job_id: id.clone(),
                sequences: vec![0, 1],
                total: 2
            },
            Delivered::Batch {
                job_id: id.clone(),
                sequences: vec![2],
                total: 3
            },
            Delivered::Finished(Box::new(job.clone())),
        ]
    );
}

#[tokio::test]
async fn fetching_again_discards_the_previous_logs() {
    let mut timeline = EventTimeline::new();
    let first = FakeGateway::new(vec![page(&[(START, "old")], None)]);
    let (first_job, _) = run(&first, &mut timeline).await;
    assert_eq!(messages(&timeline), vec!["old"]);

    let second = FakeGateway::new(vec![page(&[(START + 9, "new")], None)]);
    let (second_job, _) = run(&second, &mut timeline).await;
    assert_eq!(messages(&timeline), vec!["new"]);
    assert_eq!(timeline.events()[0].sequence, 0);
    assert_ne!(first_job.job_id, second_job.job_id);
}

#[tokio::test]
async fn blank_profile_is_passed_as_sdk_default() {
    let gateway = FakeGateway::new(vec![page(&[], None)]);
    let mut timeline = EventTimeline::new();
    let mut sink = RecordingSink::default();
    run_fetch(&gateway, &validated("  "), &mut timeline, &mut sink).await;
    assert_eq!(gateway.calls()[0].profile_name, None);
}

/// U2:BR2.8 — a fetch from the screen connects with the selected profile
/// and the selected region on every call.
#[tokio::test]
async fn screen_connection_passes_the_selected_profile_and_region() {
    let gateway = FakeGateway::new(vec![
        page(&[(START, "a")], Some("f/1")),
        page(&[], Some("f/1")),
    ]);
    let mut fetch = validated("");
    fetch.request = fetch.request.with_connection(
        ProfileSelector::Named("prod".to_string()),
        "eu-west-1".to_string(),
    );
    let mut timeline = EventTimeline::new();
    let mut sink = RecordingSink::default();
    run_fetch(&gateway, &fetch, &mut timeline, &mut sink).await;

    let calls = gateway.calls();
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert_eq!(call.profile_name.as_deref(), Some("prod"));
        assert_eq!(call.region.as_deref(), Some("eu-west-1"));
    }
}

/// U2:BR2.8 with the SDK default: no profile is passed, the region is.
#[tokio::test]
async fn screen_connection_with_sdk_default_passes_no_profile() {
    let gateway = FakeGateway::new(vec![page(&[], None)]);
    let mut fetch = validated("dev");
    fetch.request = fetch
        .request
        .with_connection(ProfileSelector::SdkDefault, "us-east-1".to_string());
    let mut timeline = EventTimeline::new();
    let mut sink = RecordingSink::default();
    run_fetch(&gateway, &fetch, &mut timeline, &mut sink).await;

    assert_eq!(gateway.calls()[0].profile_name, None);
    assert_eq!(gateway.calls()[0].region.as_deref(), Some("us-east-1"));
}

/// The fetch_check path (no screen connection) leaves the region unset so
/// the profile's default region is used (U2:BR2.8, R-11).
#[tokio::test]
async fn fetch_check_path_passes_no_region() {
    let gateway = FakeGateway::new(vec![page(&[], None)]);
    let mut timeline = EventTimeline::new();
    let mut sink = RecordingSink::default();
    run_fetch(&gateway, &validated("dev"), &mut timeline, &mut sink).await;
    assert_eq!(gateway.calls()[0].region, None);
}
