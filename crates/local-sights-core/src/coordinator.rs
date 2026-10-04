//! FetchCoordinator (U1 minimal version): one fetch of one stream.
//!
//! Progress and logs go to the [`FetchSink`] the caller passes in, and the
//! logs are kept by the [`EventTimeline`] (BR4.5, ADR-007). The desktop app
//! and the `fetch_check` example each provide their own sink.

use serde::Serialize;

use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::fetcher::{StreamFetchOutcome, StreamStatus, fetch_stream};
use crate::gateway::CloudWatchLogsGateway;
use crate::request::ValidatedFetch;
use crate::time_range::TimeRange;
use crate::timeline::EventTimeline;

/// Status of a fetch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    /// In progress.
    Running,
    /// Every page was read.
    Completed,
    /// Stopped by an error (including RegionMissing).
    Failed,
}

/// State and result of one fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchJob {
    /// Unique ID of the job.
    pub job_id: String,
    /// Running, Completed or Failed.
    pub status: JobStatus,
    /// Number of events received so far.
    pub event_count: u64,
    /// 0 or 1 in U1.
    pub failed_stream_count: u32,
    /// The range being fetched.
    pub range: TimeRange,
    /// The stream outcome, once finished.
    pub outcome: Option<StreamFetchOutcome>,
}

impl FetchJob {
    /// A new running job with a fresh ID.
    pub fn start(range: TimeRange) -> Self {
        Self {
            job_id: uuid::Uuid::new_v4().to_string(),
            status: JobStatus::Running,
            event_count: 0,
            failed_stream_count: 0,
            range,
            outcome: None,
        }
    }

    /// The failure, when the job failed.
    pub fn failure(&self) -> Option<&ApiFailure> {
        self.outcome
            .as_ref()
            .and_then(|outcome| outcome.failure.as_ref())
    }

    /// Number of pages received.
    pub fn page_count(&self) -> u32 {
        self.outcome
            .as_ref()
            .map_or(0, |outcome| outcome.page_count)
    }

    fn finish(&mut self, outcome: StreamFetchOutcome) {
        let failed = outcome.status == StreamStatus::Failed;
        self.status = if failed {
            JobStatus::Failed
        } else {
            JobStatus::Completed
        };
        self.failed_stream_count = u32::from(failed);
        self.event_count = outcome.event_count;
        self.outcome = Some(outcome);
    }
}

/// Receives the progress of a fetch.
pub trait FetchSink {
    /// The fetch started; the timeline has just been cleared.
    fn on_started(&mut self, job: &FetchJob);
    /// One page of events was added to the timeline; `total` is the
    /// cumulative number of events of this fetch.
    fn on_batch(&mut self, job_id: &str, events: &[LogEvent], total: u64);
    /// The fetch finished (Completed or Failed).
    fn on_finished(&mut self, job: &FetchJob);
}

/// Runs one fetch: clears the timeline, reads the stream page by page,
/// appends each page to the timeline and reports it to the sink, then
/// reports the finished job. On failure, the events already fetched stay in
/// the timeline (BR4.1).
pub async fn run_fetch<G, S>(
    gateway: &G,
    fetch: &ValidatedFetch,
    timeline: &mut EventTimeline,
    sink: &mut S,
) -> FetchJob
where
    G: CloudWatchLogsGateway,
    S: FetchSink + Send,
{
    let mut job = FetchJob::start(fetch.range);
    timeline.clear();
    sink.on_started(&job);
    let mut total: u64 = 0;
    let job_id = job.job_id.clone();
    let outcome = fetch_stream(gateway, fetch, |events| {
        total += events.len() as u64;
        timeline.append(events.clone());
        sink.on_batch(&job_id, &events, total);
    })
    .await;
    job.finish(outcome);
    sink.on_finished(&job);
    job
}
