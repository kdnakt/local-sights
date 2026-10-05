//! FetchCoordinator: one fetch of a whole log group (U3:BR5.1-BR5.5).
//!
//! The flow is "list and select the streams → fetch them one by one → add
//! each page to the timeline as it arrives". Progress goes to the
//! [`FetchSink`] the caller passes in; the coordinator knows nothing of its
//! caller (ADR-007). The timeline is reached through [`TimelineStore`],
//! which is locked only for each page, so the screen can read rows while
//! the fetch runs (NFR3, BR4.3).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::fetcher::{StreamFetchEnd, StreamFetchOutcome, fetch_stream};
use crate::gateway::CloudWatchLogsGateway;
use crate::request::ValidatedFetch;
use crate::retry::{ExhaustionStreak, Retrier};
use crate::streams::planner::{ListingProgress, PlanOutcome, plan_streams};
use crate::streams::{StreamListingStatus, StreamPlan};
use crate::time_range::TimeRange;
use crate::timeline::EventTimeline;

/// Source of job IDs: one more for every fetch in this run of the app.
static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

/// Status of a fetch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    /// In progress.
    Running,
    /// Every selected stream was read (also when none was selected).
    Completed,
    /// The listing failed on its first page and nothing was selected
    /// (including RegionMissing).
    Failed,
    /// Finished, but some streams failed or the listing was partial.
    CompletedWithFailures,
    /// Stopped by the caller (BR5.5).
    Aborted,
}

/// BR5.2: the status of a finished fetch.
pub fn decide_job_status(
    aborted: bool,
    plan: Option<&StreamPlan>,
    failed_stream_count: usize,
) -> JobStatus {
    if aborted {
        return JobStatus::Aborted;
    }
    let Some(plan) = plan else {
        return JobStatus::Failed;
    };
    let partial = plan.listing_status == StreamListingStatus::Partial;
    if partial && plan.listed_page_count == 0 && plan.selected_streams.is_empty() {
        JobStatus::Failed
    } else if partial || failed_stream_count > 0 {
        JobStatus::CompletedWithFailures
    } else {
        JobStatus::Completed
    }
}

/// One stream that could not be fetched, with a safe reason (BR5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedStream {
    /// Stream name.
    pub log_stream_name: String,
    /// Kind and safe detail only (U1:BR4.3).
    pub failure: ApiFailure,
}

/// State and result of one fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchJob {
    /// Unique, increasing ID of the job.
    pub job_id: u64,
    /// Running, Completed, CompletedWithFailures, Failed or Aborted.
    pub status: JobStatus,
    /// The range being fetched.
    pub range: TimeRange,
    /// Number of streams to fetch.
    pub planned_stream_count: u32,
    /// Number of streams finished, successfully or not.
    pub finished_stream_count: u32,
    /// Streams that failed, in fetch order.
    pub failed_streams: Vec<FailedStream>,
    /// How the listing ended; copy of the StreamPlan's (review R-09).
    pub listing_status: Option<StreamListingStatus>,
    /// The listing failure, when Partial (BR1.5).
    pub listing_failure: Option<ApiFailure>,
    /// Number of events added so far.
    pub event_count: u64,
    /// Outcome of every finished stream, in fetch order.
    pub outcomes: Vec<StreamFetchOutcome>,
}

impl FetchJob {
    /// A new running job with the next ID.
    pub fn start(range: TimeRange) -> Self {
        Self {
            job_id: NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed),
            status: JobStatus::Running,
            range,
            planned_stream_count: 0,
            finished_stream_count: 0,
            failed_streams: Vec::new(),
            listing_status: None,
            listing_failure: None,
            event_count: 0,
            outcomes: Vec::new(),
        }
    }

    /// Counts one finished stream, never beyond the planned count.
    pub fn count_finished_stream(&mut self) {
        if self.finished_stream_count < self.planned_stream_count {
            self.finished_stream_count += 1;
        }
    }

    /// Number of failed streams.
    pub fn failed_stream_count(&self) -> usize {
        self.failed_streams.len()
    }

    /// The failure that made the whole job fail: the listing failure.
    pub fn failure(&self) -> Option<&ApiFailure> {
        self.listing_failure.as_ref()
    }

    fn record_plan(&mut self, plan: &StreamPlan) {
        self.planned_stream_count = u32::try_from(plan.selected_streams.len()).unwrap_or(u32::MAX);
        self.listing_status = Some(plan.listing_status);
        self.listing_failure = plan.listing_failure.clone();
    }

    fn record_stream(&mut self, outcome: StreamFetchOutcome) {
        if let Some(failure) = &outcome.failure {
            self.failed_streams.push(FailedStream {
                log_stream_name: outcome.log_stream_name.clone(),
                failure: failure.clone(),
            });
        }
        self.count_finished_stream();
        self.outcomes.push(outcome);
    }
}

/// Progress of one added page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchProgress {
    /// Events added by this page.
    pub added: u64,
    /// Events added by this fetch so far.
    pub total: u64,
    /// Timeline version after the page was added.
    pub timeline_version: u64,
}

/// Receives the progress of a fetch, in this order (BR5.4): started →
/// listing progress (per listing page) → planned → batches and finished
/// streams → finished.
pub trait FetchSink {
    /// The fetch started; the timeline has just been discarded and is now
    /// at `timeline_version` (BR4.5, BR5.6).
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64);
    /// One listing page was read.
    fn on_listing_progress(&mut self, job_id: u64, progress: ListingProgress);
    /// The listing ended; `job` carries the planned stream count.
    fn on_planned(&mut self, job: &FetchJob);
    /// One page of events was added to the timeline.
    fn on_batch(&mut self, job_id: u64, progress: BatchProgress);
    /// One stream finished (successfully or not); `job` has the counts.
    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome);
    /// The fetch finished with its final status; `timeline_version` is the
    /// version of the timeline after it (after the discard when Aborted,
    /// BR4.5, review R-02).
    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64);
}

/// Where the fetch keeps its events. Each call is short: the store is not
/// held between pages.
pub trait TimelineStore: Send + Sync {
    /// Discards every event (BR4.5) and returns the new version.
    fn discard(&self) -> u64;
    /// Adds one page in order (BR4.2) and returns the new version.
    fn add(&self, events: Vec<LogEvent>) -> u64;
}

impl TimelineStore for Mutex<EventTimeline> {
    fn discard(&self) -> u64 {
        // A poisoned timeline is discarded anyway, which repairs it.
        self.lock().unwrap_or_else(PoisonError::into_inner).clear()
    }

    fn add(&self, events: Vec<LogEvent>) -> u64 {
        self.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .append(events)
    }
}

/// Runs one fetch of the whole log group (BR5.1).
///
/// Discards the timeline before the first call (BR5.6), lists and selects
/// the streams (BR1.1-BR1.5), fetches them one by one (BR3.1) adding each
/// page as it arrives (BR4.2), collects the failed streams (BR5.3) and
/// decides the status (BR5.2). When `max_consecutive_exhausted` streams in
/// a row used up their retries, the remaining streams are failed without
/// calling the API, with the last stream's failure (BR2.1, review R-09).
/// When the abort is requested, nothing more is called, the job is
/// Aborted and the timeline is discarded (BR5.5).
pub async fn run_fetch<G, T, S>(
    gateway: &G,
    fetch: &ValidatedFetch,
    timeline: &T,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> FetchJob
where
    G: CloudWatchLogsGateway,
    T: TimelineStore,
    S: FetchSink + Send,
{
    let mut job = FetchJob::start(fetch.range);
    let job_id = job.job_id;
    let mut latest_version = timeline.discard();
    sink.on_started(&job, latest_version);

    let plan = match plan_streams(gateway, &fetch.request, &fetch.range, retrier, |progress| {
        sink.on_listing_progress(job_id, progress);
    })
    .await
    {
        PlanOutcome::Planned(plan) => plan,
        PlanOutcome::Aborted => return abort(job, timeline, sink),
    };
    job.record_plan(&plan);
    sink.on_planned(&job);

    let mut streak = ExhaustionStreak::new(retrier.policy().max_consecutive_exhausted);
    let mut last_failure: Option<ApiFailure> = None;
    for stream in &plan.selected_streams {
        if retrier.is_aborted() {
            return abort(job, timeline, sink);
        }
        let name = stream.log_stream_name.as_str();
        let outcome = match (streak.limit_reached(), last_failure.clone()) {
            (true, Some(failure)) => StreamFetchOutcome::skipped(name, failure),
            _ => {
                let mut total = job.event_count;
                let fetched = fetch_stream(
                    gateway,
                    &fetch.request,
                    &fetch.range,
                    name,
                    retrier,
                    |events| {
                        let added = events.len() as u64;
                        let timeline_version = timeline.add(events);
                        latest_version = timeline_version;
                        total += added;
                        sink.on_batch(
                            job_id,
                            BatchProgress {
                                added,
                                total,
                                timeline_version,
                            },
                        );
                    },
                )
                .await;
                job.event_count = total;
                let Some(ending) = fetched.end.streak_ending() else {
                    return abort(job, timeline, sink);
                };
                streak.record(ending);
                if let StreamFetchEnd::Failed { .. } = fetched.end {
                    last_failure = fetched.outcome.failure.clone();
                }
                fetched.outcome
            }
        };
        job.record_stream(outcome);
        if let Some(outcome) = job.outcomes.last() {
            sink.on_stream_finished(&job, outcome);
        }
    }

    job.status = decide_job_status(false, Some(&plan), job.failed_stream_count());
    sink.on_finished(&job, latest_version);
    job
}

/// BR5.5: stop, discard the timeline and report the Aborted job.
fn abort<T, S>(mut job: FetchJob, timeline: &T, sink: &mut S) -> FetchJob
where
    T: TimelineStore,
    S: FetchSink,
{
    let timeline_version = timeline.discard();
    job.status = JobStatus::Aborted;
    job.event_count = 0;
    sink.on_finished(&job, timeline_version);
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};
    use crate::streams::{LogStream, StreamListingStatus, StreamPlan};

    fn plan(status: StreamListingStatus, selected: usize, pages: u32) -> StreamPlan {
        let failure = (status == StreamListingStatus::Partial)
            .then(|| ApiFailure::new(FailureKind::Throttled, &SafeDetailFields::default()));
        StreamPlan {
            selected_streams: (0..selected)
                .map(|i| LogStream {
                    log_stream_name: format!("s{i}"),
                    first_event_timestamp: None,
                    last_event_timestamp: None,
                })
                .collect(),
            listing_status: status,
            listing_failure: failure,
            listed_page_count: pages,
            seen_stream_count: u32::try_from(selected).unwrap(),
        }
    }

    #[test]
    fn an_abort_wins_over_everything() {
        let complete = plan(StreamListingStatus::Complete, 2, 1);
        assert_eq!(
            decide_job_status(true, Some(&complete), 1),
            JobStatus::Aborted
        );
        assert_eq!(decide_job_status(true, None, 0), JobStatus::Aborted);
    }

    #[test]
    fn a_listing_failing_on_its_first_page_with_nothing_selected_fails() {
        let partial = plan(StreamListingStatus::Partial, 0, 0);
        assert_eq!(
            decide_job_status(false, Some(&partial), 0),
            JobStatus::Failed
        );
    }

    #[test]
    fn failed_streams_or_a_partial_listing_complete_with_failures() {
        let complete = plan(StreamListingStatus::Complete, 3, 1);
        assert_eq!(
            decide_job_status(false, Some(&complete), 1),
            JobStatus::CompletedWithFailures
        );
        let partial_later = plan(StreamListingStatus::Partial, 0, 1);
        assert_eq!(
            decide_job_status(false, Some(&partial_later), 0),
            JobStatus::CompletedWithFailures,
            "a failure on the second page is not a first-page failure"
        );
        let partial_with_streams = plan(StreamListingStatus::Partial, 2, 0);
        assert_eq!(
            decide_job_status(false, Some(&partial_with_streams), 0),
            JobStatus::CompletedWithFailures
        );
    }

    #[test]
    fn everything_else_completes_even_with_no_stream() {
        for status in [
            StreamListingStatus::Complete,
            StreamListingStatus::StoppedEarly,
        ] {
            assert_eq!(
                decide_job_status(false, Some(&plan(status, 4, 2)), 0),
                JobStatus::Completed
            );
            assert_eq!(
                decide_job_status(false, Some(&plan(status, 0, 1)), 0),
                JobStatus::Completed
            );
        }
    }

    #[test]
    fn finished_streams_never_exceed_the_planned_streams() {
        let range = crate::time_range::TimeRange::from_seconds(0, 60).unwrap();
        let mut job = FetchJob::start(range);
        job.planned_stream_count = 2;
        for _ in 0..3 {
            job.count_finished_stream();
        }
        assert_eq!(job.finished_stream_count, 2);
    }
}
