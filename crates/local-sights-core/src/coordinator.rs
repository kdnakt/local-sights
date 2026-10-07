//! FetchCoordinator: one fetch of a whole log group (U3:BR5.1-BR5.5).
//!
//! The flow is "list and select the streams → fetch them one by one → add
//! each page to the timeline as it arrives". Progress goes to the
//! [`FetchSink`] the caller passes in; the coordinator knows nothing of its
//! caller (ADR-007). The timeline is reached through [`TimelineStore`],
//! which is locked only for each page, so the screen can read rows while
//! the fetch runs (NFR3, BR4.3).
//!
//! Since U6, [`run_fetch_with_cache`] puts the disk cache in front of
//! [`run_fetch`], which itself is unchanged (U6 review R-12). When the cache
//! is enabled and one cached range holds the requested range, the events
//! come from the cache and no AWS API is called (U6:BR2.3); otherwise the
//! whole range is fetched from AWS (U6:BR2.4) and, when every stream
//! succeeded, the recordable part is written before the job finishes
//! (U6:BR3.1, BR3.2, BR3.5). The order of the notices is U6:BR3.7.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use crate::cache::plan::{
    CacheKey, CacheOutcome, CachedEvent, CoveredRange, WriteCheck, decide_write, resequence,
};
use crate::cache::{CacheLookup, LogCache};
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

/// Held events copied per lock while preparing a cache write (U6:BR3.5):
/// small enough that a viewport read waits for at most one chunk.
pub const CACHE_COPY_CHUNK_EVENTS: usize = 4_096;

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
    /// Whether the events came from the disk cache without calling AWS
    /// (U6:BR2.3).
    pub served_from_cache: bool,
    /// What the disk cache did for this fetch (U6:BR5.3).
    pub cache_outcome: CacheOutcome,
    /// Whether the cache could not be read and the logs were fetched again
    /// from AWS (U6:BR4.1).
    pub read_failed: bool,
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
            served_from_cache: false,
            cache_outcome: CacheOutcome::NotUsed,
            read_failed: false,
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
    /// The fetched events are about to be written to the disk cache; the
    /// job finishes after the write (U6:BR3.5, BR3.7). Does nothing unless
    /// the sink shows it.
    fn on_saving(&mut self, _job_id: u64) {}
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

/// Read access to the held events for a cache write (U6:BR3.5).
pub trait HeldEvents {
    /// Copies at most `max` held events whose timestamps lie in `range`,
    /// starting at `cursor` (`None`: the first such event), and returns them
    /// with the cursor of the next chunk (`None` when nothing is left). Each
    /// call takes the lock once and releases it, so the store is never held
    /// across chunks.
    fn copy_events_in_range(
        &self,
        range: CoveredRange,
        cursor: Option<usize>,
        max: usize,
    ) -> (Vec<LogEvent>, Option<usize>);
}

/// One chunk of [`HeldEvents::copy_events_in_range`] from ordered `events`.
pub(crate) fn copy_chunk_in_range(
    events: &[LogEvent],
    range: CoveredRange,
    cursor: Option<usize>,
    max: usize,
) -> (Vec<LogEvent>, Option<usize>) {
    let limit = events.partition_point(|event| event.timestamp <= range.end_ms);
    let start = cursor
        .unwrap_or_else(|| events.partition_point(|event| event.timestamp < range.start_ms))
        .min(limit);
    let end = start.saturating_add(max.max(1)).min(limit);
    let next = (end < limit).then_some(end);
    (events[start..end].to_vec(), next)
}

impl HeldEvents for Mutex<EventTimeline> {
    fn copy_events_in_range(
        &self,
        range: CoveredRange,
        cursor: Option<usize>,
        max: usize,
    ) -> (Vec<LogEvent>, Option<usize>) {
        let timeline = self.lock().unwrap_or_else(PoisonError::into_inner);
        copy_chunk_in_range(timeline.events(), range, cursor, max)
    }
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

/// Wraps a sink and holds back its `on_finished`, passing everything else on
/// (U6 review R-12): [`run_fetch_with_cache`] decides and does the cache
/// write first, then finishes the job on the wrapped sink.
#[derive(Debug)]
pub struct DeferredFinishSink<'a, S> {
    inner: &'a mut S,
    finished: Option<(FetchJob, u64)>,
}

impl<'a, S> DeferredFinishSink<'a, S> {
    /// Wraps `inner`.
    pub fn new(inner: &'a mut S) -> Self {
        Self {
            inner,
            finished: None,
        }
    }

    /// The held-back job and timeline version, once `on_finished` came.
    pub fn take_finished(&mut self) -> Option<(FetchJob, u64)> {
        self.finished.take()
    }
}

impl<S: FetchSink> FetchSink for DeferredFinishSink<'_, S> {
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
    }

    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        self.inner.on_stream_finished(job, outcome);
    }

    fn on_saving(&mut self, job_id: u64) {
        self.inner.on_saving(job_id);
    }

    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64) {
        self.finished = Some((job.clone(), timeline_version));
    }
}

/// What the cache answered for a requested range.
enum CacheAnswer {
    /// The events of the range, numbered again (U6:BR4.3).
    Hit(Vec<LogEvent>),
    /// No cache, or no single cached range holds the request.
    NoCache,
    /// The cache was unreadable or broken (U6:BR4.1).
    ReadFailed,
}

/// Runs one fetch with the disk cache in front of [`run_fetch`] (U6).
///
/// `cache` is `None` when the cache is disabled: the fetch is exactly
/// [`run_fetch`], nothing is read or written and the outcome is NotUsed
/// (U6:BR1.5). Otherwise the cache of the request's key is looked up on a
/// blocking thread (U6:BR4.2):
///
/// - when one cached range holds the whole range and every event read
///   passes the checks, the timeline is discarded and then gets all of
///   them in one addition; no AWS API is called (U6:BR2.3, BR3.7);
/// - otherwise the whole range is fetched from AWS through [`run_fetch`]
///   (U6:BR2.4), with an unreadable or broken cache marked `read_failed`
///   (U6:BR4.1). The end of that fetch is held back; when the fetch
///   completed with no failed stream and a recordable range (U6:BR3.1,
///   BR3.2), `on_saving` is sent, the recordable events are copied from the
///   timeline one chunk per lock and written on a blocking thread
///   (U6:BR3.5). Then the job finishes with its cache outcome.
///
/// `fetch_started_at_ms` is when the fetch started (epoch milliseconds), the
/// base of the five-minute margin (U6:BR3.1).
pub async fn run_fetch_with_cache<G, T, S>(
    gateway: &G,
    fetch: &ValidatedFetch,
    timeline: &T,
    retrier: &Retrier<'_>,
    cache: Option<&LogCache>,
    fetch_started_at_ms: i64,
    sink: &mut S,
) -> FetchJob
where
    G: CloudWatchLogsGateway,
    T: TimelineStore + HeldEvents,
    S: FetchSink + Send,
{
    let (Some(cache), Some(key)) = (cache, CacheKey::from_request(&fetch.request)) else {
        return run_fetch(gateway, fetch, timeline, retrier, sink).await;
    };
    let start_ms = fetch.range.start_instant();
    let end_ms = fetch.range.end_instant();
    let read_failed = match look_up(cache, &key, start_ms, end_ms).await {
        CacheAnswer::Hit(events) => {
            return serve_from_cache(fetch, timeline, retrier, events, sink);
        }
        CacheAnswer::NoCache => false,
        CacheAnswer::ReadFailed => true,
    };

    let mut deferred = DeferredFinishSink::new(sink);
    let returned = run_fetch(gateway, fetch, timeline, retrier, &mut deferred).await;
    let Some((mut job, timeline_version)) = deferred.take_finished() else {
        // run_fetch always finishes its job; without it there is nothing to
        // report, and the caller's recovery ends the fetch.
        return returned;
    };
    job.read_failed = read_failed;
    let write = decide_write(&WriteCheck {
        cache_enabled: true,
        served_from_cache: false,
        status: job.status,
        failed_stream_count: job.failed_stream_count(),
        range_start_ms: start_ms,
        range_end_ms: end_ms,
        fetch_started_at_ms,
    });
    job.cache_outcome = match write {
        None => CacheOutcome::NotSaved,
        Some(record) => {
            sink.on_saving(job.job_id);
            let events = copy_held_events(timeline, record);
            save(cache, key, record, events).await
        }
    };
    sink.on_finished(&job, timeline_version);
    job
}

/// U6:BR2.3, BR3.7: discard → started → one addition → one batch (when
/// there are events) → finished, as Completed, served from the cache.
fn serve_from_cache<T, S>(
    fetch: &ValidatedFetch,
    timeline: &T,
    retrier: &Retrier<'_>,
    events: Vec<LogEvent>,
    sink: &mut S,
) -> FetchJob
where
    T: TimelineStore,
    S: FetchSink,
{
    let mut job = FetchJob::start(fetch.range);
    let mut timeline_version = timeline.discard();
    if retrier.is_aborted() {
        sink.on_started(&job, timeline_version);
        return abort(job, timeline, sink);
    }
    job.served_from_cache = true;
    job.cache_outcome = CacheOutcome::Hit;
    sink.on_started(&job, timeline_version);
    let count = events.len() as u64;
    if count > 0 {
        timeline_version = timeline.add(events);
        sink.on_batch(
            job.job_id,
            BatchProgress {
                added: count,
                total: count,
                timeline_version,
            },
        );
    }
    job.event_count = count;
    job.status = JobStatus::Completed;
    sink.on_finished(&job, timeline_version);
    job
}

/// Looks the range up on a blocking thread (U6:BR4.2) and numbers the
/// events of a hit again (U6:BR4.3).
async fn look_up(cache: &LogCache, key: &CacheKey, start_ms: i64, end_ms: i64) -> CacheAnswer {
    let cache = cache.clone();
    let key = key.clone();
    let task = tokio::task::spawn_blocking(move || match cache.lookup(&key, start_ms, end_ms) {
        CacheLookup::Hit(events) => CacheAnswer::Hit(resequence(events)),
        CacheLookup::Missing | CacheLookup::NotCovered => CacheAnswer::NoCache,
        CacheLookup::Unreadable | CacheLookup::Corrupt(_) => CacheAnswer::ReadFailed,
    });
    match task.await {
        Ok(answer) => answer,
        Err(_) => {
            eprintln!("local-sights: cache: the cache lookup ended abnormally; fetching from AWS");
            CacheAnswer::ReadFailed
        }
    }
}

/// Copies the held events of `range`, one chunk per lock (U6:BR3.5).
fn copy_held_events<T: HeldEvents>(timeline: &T, range: CoveredRange) -> Vec<CachedEvent> {
    let mut events = Vec::new();
    let mut cursor = None;
    loop {
        let (chunk, next) = timeline.copy_events_in_range(range, cursor, CACHE_COPY_CHUNK_EVENTS);
        events.extend(chunk.into_iter().map(CachedEvent::from));
        match next {
            Some(next) => cursor = Some(next),
            None => return events,
        }
    }
}

/// Writes on a blocking thread (U6:BR3.5): Saved, or SaveFailed with a
/// diagnostic (no key, no message).
async fn save(
    cache: &LogCache,
    key: CacheKey,
    range: CoveredRange,
    events: Vec<CachedEvent>,
) -> CacheOutcome {
    let cache = cache.clone();
    let task = tokio::task::spawn_blocking(move || cache.write(&key, range, events));
    match task.await {
        Ok(Ok(_)) => CacheOutcome::Saved,
        Ok(Err(error)) => {
            eprintln!("local-sights: cache: the fetched logs could not be saved: {error}");
            CacheOutcome::SaveFailed
        }
        Err(_) => {
            eprintln!("local-sights: cache: the cache write ended abnormally");
            CacheOutcome::SaveFailed
        }
    }
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

    // ---- U6: the disk cache in front of the fetch ----------------------

    use std::sync::Mutex as StdMutex;

    use crate::cache::plan::CacheKey;
    use crate::cache::{HeaderRead, LogCache};
    use crate::catalog::ProfileSelector;
    use crate::gateway::{
        DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
        DescribeLogStreamsRequest, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
    };
    use crate::request::{FetchInput, validate_fetch_input};
    use crate::retry::{FixedJitter, RetryPolicy, abort_pair};

    /// 2024-01-02 03:04:05 UTC, the start of the fetched range.
    const START: i64 = 1_704_164_645_000;
    /// A fetch started a day later: the whole range is recordable.
    const A_DAY_LATER: i64 = START + 86_400_000;

    /// An in-crate fake of the gateway: every stream answers one page with
    /// its events inside the requested time, and a listed failing stream
    /// is denied. Counts every call; never connects to AWS.
    #[derive(Default)]
    struct FakeLogs {
        events: Vec<(&'static str, i64, &'static str)>,
        failing_stream: Option<&'static str>,
        calls: StdMutex<Vec<String>>,
    }

    impl FakeLogs {
        fn new(events: Vec<(&'static str, i64, &'static str)>) -> Self {
            Self {
                events,
                ..Self::default()
            }
        }

        fn calls(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl CloudWatchLogsGateway for FakeLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("GetLogEvents:{}", request.log_stream_name));
            if self.failing_stream == Some(request.log_stream_name.as_str()) {
                return Err(ApiFailure::new(
                    FailureKind::AccessDenied,
                    &SafeDetailFields::default(),
                ));
            }
            let events = self
                .events
                .iter()
                .filter(|(stream, timestamp, _)| {
                    *stream == request.log_stream_name
                        && *timestamp >= request.start_time
                        && *timestamp < request.end_time
                })
                .map(|(_, timestamp, message)| GatewayEvent {
                    timestamp: *timestamp,
                    ingestion_time: None,
                    message: (*message).to_string(),
                })
                .collect();
            Ok(GetLogEventsPage {
                events,
                next_forward_token: None,
            })
        }

        async fn describe_log_groups(
            &self,
            _request: DescribeLogGroupsRequest,
        ) -> Result<DescribeLogGroupsPage, ApiFailure> {
            panic!("a fetch never lists log groups");
        }

        async fn describe_log_streams(
            &self,
            _request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            self.calls
                .lock()
                .unwrap()
                .push("DescribeLogStreams".to_string());
            let mut names: Vec<&str> = self.events.iter().map(|(stream, _, _)| *stream).collect();
            names.extend(self.failing_stream);
            names.sort_unstable();
            names.dedup();
            Ok(DescribeLogStreamsPage {
                streams: names
                    .into_iter()
                    .map(|name| LogStream {
                        log_stream_name: name.to_string(),
                        first_event_timestamp: None,
                        last_event_timestamp: None,
                    })
                    .collect(),
                next_token: None,
            })
        }
    }

    /// Records the kind of every delivery, in order.
    #[derive(Default)]
    struct Recorder {
        kinds: Vec<&'static str>,
        batches: Vec<BatchProgress>,
        finished: Option<FetchJob>,
    }

    impl FetchSink for Recorder {
        fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {
            self.kinds.push("Started");
        }
        fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {
            self.kinds.push("Listing");
        }
        fn on_planned(&mut self, _job: &FetchJob) {
            self.kinds.push("Planned");
        }
        fn on_batch(&mut self, _job_id: u64, progress: BatchProgress) {
            self.kinds.push("Batch");
            self.batches.push(progress);
        }
        fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {
            self.kinds.push("StreamFinished");
        }
        fn on_saving(&mut self, _job_id: u64) {
            self.kinds.push("Saving");
        }
        fn on_finished(&mut self, job: &FetchJob, _timeline_version: u64) {
            self.kinds.push("Finished");
            self.finished = Some(job.clone());
        }
    }

    /// The U3 range 03:04:05 - 03:05:05.999 for `dev` in ap-northeast-1.
    fn validated() -> ValidatedFetch {
        let validated = validate_fetch_input(&FetchInput {
            profile_name: "dev".to_string(),
            log_group_name: "/aws/lambda/orders".to_string(),
            start_text: "2024-01-02 03:04:05".to_string(),
            end_text: "2024-01-02 03:05:05".to_string(),
        })
        .unwrap();
        ValidatedFetch {
            request: validated.request.with_connection(
                ProfileSelector::Named("dev".to_string()),
                "ap-northeast-1".to_string(),
            ),
            range: validated.range,
        }
    }

    fn cache_key() -> CacheKey {
        CacheKey::from_request(&validated().request).unwrap()
    }

    fn two_streams() -> FakeLogs {
        FakeLogs::new(vec![
            ("a", START + 1_000, "a1"),
            ("b", START + 2_000, "b1"),
            ("a", START + 3_000, "a2"),
        ])
    }

    async fn fetch_with(
        gateway: &FakeLogs,
        timeline: &Mutex<EventTimeline>,
        cache: Option<&LogCache>,
        started_at: i64,
        aborted: bool,
    ) -> (FetchJob, Recorder) {
        let policy = RetryPolicy::default();
        let jitter = FixedJitter(0.0);
        let (handle, signal) = abort_pair();
        if aborted {
            handle.abort();
        }
        let retrier = Retrier::new(&policy, &jitter, &signal);
        let mut sink = Recorder::default();
        let job = run_fetch_with_cache(
            gateway,
            &validated(),
            timeline,
            &retrier,
            cache,
            started_at,
            &mut sink,
        )
        .await;
        (job, sink)
    }

    async fn fetch(
        gateway: &FakeLogs,
        timeline: &Mutex<EventTimeline>,
        cache: Option<&LogCache>,
    ) -> (FetchJob, Recorder) {
        fetch_with(gateway, timeline, cache, A_DAY_LATER, false).await
    }

    fn messages(timeline: &Mutex<EventTimeline>) -> Vec<(String, String, u64)> {
        timeline
            .lock()
            .unwrap()
            .events()
            .iter()
            .map(|e| (e.log_stream_name.clone(), e.message.clone(), e.sequence))
            .collect()
    }

    fn triple(stream: &str, message: &str, sequence: u64) -> (String, String, u64) {
        (stream.to_string(), message.to_string(), sequence)
    }

    fn whole_range() -> CoveredRange {
        let range = validated().range;
        CoveredRange {
            start_ms: range.start_instant(),
            end_ms: range.end_instant(),
        }
    }

    fn stored(timestamp: i64, stream: &str, sequence: u64) -> CachedEvent {
        CachedEvent {
            timestamp,
            ingestion_time: None,
            message: format!("cached {stream}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    #[tokio::test]
    async fn a_disabled_cache_reads_and_writes_nothing_and_fetches_as_u3() {
        let dir = tempfile::tempdir().unwrap();
        let gateway = two_streams();
        let timeline = Mutex::new(EventTimeline::new());
        let (job, sink) = fetch(&gateway, &timeline, None).await;
        assert_eq!(job.cache_outcome, CacheOutcome::NotUsed);
        assert!(!job.served_from_cache && !job.read_failed);
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(gateway.calls(), 3, "listing and two streams");
        assert!(!sink.kinds.contains(&"Saving"));
        assert_eq!(sink.kinds.last(), Some(&"Finished"));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn a_hit_calls_no_api_and_reports_started_batch_finished() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        let held = vec![
            stored(START + 10, "b", 41),
            stored(START + 20, "a", 7),
            stored(START + 30, "b", 42),
        ];
        let wider = CoveredRange {
            start_ms: START - 60_000,
            end_ms: START + 120_000,
        };
        cache.write(&cache_key(), wider, held).unwrap();
        let gateway = FakeLogs::default();
        let timeline = Mutex::new(EventTimeline::new());
        timeline.add(vec![LogEvent {
            timestamp: 1,
            ingestion_time: None,
            message: "previous fetch".to_string(),
            log_stream_name: "old".to_string(),
            sequence: 0,
        }]);

        let (job, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert_eq!(gateway.calls(), 0, "no AWS API is called");
        assert_eq!(sink.kinds, vec!["Started", "Batch", "Finished"]);
        assert_eq!(sink.batches[0].added, 3);
        assert_eq!(sink.batches[0].total, 3);
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.served_from_cache);
        assert_eq!(job.cache_outcome, CacheOutcome::Hit);
        assert_eq!(job.event_count, 3);
        assert_eq!(job.failed_stream_count(), 0);
        assert_eq!(
            messages(&timeline),
            vec![
                triple("b", "cached b", 0),
                triple("a", "cached a", 0),
                triple("b", "cached b", 1),
            ],
            "the previous rows are gone and the sequences start from 0"
        );
    }

    #[tokio::test]
    async fn an_empty_cached_range_is_still_a_hit() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        cache
            .write(&cache_key(), whole_range(), Vec::new())
            .unwrap();
        let gateway = FakeLogs::default();
        let timeline = Mutex::new(EventTimeline::new());
        let (job, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert_eq!(gateway.calls(), 0);
        assert_eq!(sink.kinds, vec!["Started", "Finished"]);
        assert_eq!(job.cache_outcome, CacheOutcome::Hit);
        assert_eq!(job.event_count, 0);
    }

    #[tokio::test]
    async fn a_miss_fetches_everything_saves_before_finishing_and_then_hits() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().join("cache"));
        let gateway = two_streams();
        let timeline = Mutex::new(EventTimeline::new());
        let (job, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert_eq!(job.cache_outcome, CacheOutcome::Saved);
        assert!(!job.served_from_cache && !job.read_failed);
        assert_eq!(&sink.kinds[sink.kinds.len() - 2..], ["Saving", "Finished"]);
        assert_eq!(sink.kinds.first(), Some(&"Started"));
        assert!(sink.kinds.contains(&"Planned"));
        let calls = gateway.calls();
        assert_eq!(calls, 3);

        let (again, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert_eq!(
            gateway.calls(),
            calls,
            "the same fetch is served from the cache"
        );
        assert_eq!(again.cache_outcome, CacheOutcome::Hit);
        assert_eq!(sink.kinds, vec!["Started", "Batch", "Finished"]);
        assert_eq!(
            messages(&timeline),
            vec![
                triple("a", "a1", 0),
                triple("b", "b1", 0),
                triple("a", "a2", 1)
            ]
        );
    }

    #[tokio::test]
    async fn a_failed_stream_or_an_abort_writes_nothing_and_keeps_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        let elsewhere = CoveredRange {
            start_ms: 0,
            end_ms: 999,
        };
        cache.write(&cache_key(), elsewhere, Vec::new()).unwrap();
        let before = std::fs::read(cache.entry_path(&cache_key())).unwrap();

        let mut gateway = two_streams();
        gateway.failing_stream = Some("c");
        let timeline = Mutex::new(EventTimeline::new());
        let (job, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert_eq!(job.status, JobStatus::CompletedWithFailures);
        assert_eq!(job.cache_outcome, CacheOutcome::NotSaved);
        assert!(!sink.kinds.contains(&"Saving"));
        assert_eq!(timeline.lock().unwrap().len(), 3, "the fetched rows stay");

        let (aborted, sink) =
            fetch_with(&two_streams(), &timeline, Some(&cache), A_DAY_LATER, true).await;
        assert_eq!(aborted.status, JobStatus::Aborted);
        assert_eq!(aborted.cache_outcome, CacheOutcome::NotSaved);
        assert_eq!(sink.kinds.last(), Some(&"Finished"));
        assert_eq!(
            std::fs::read(cache.entry_path(&cache_key())).unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn nothing_is_written_when_no_part_is_five_minutes_old() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        let gateway = two_streams();
        let timeline = Mutex::new(EventTimeline::new());
        let just_after = START + 60_000;
        let (job, sink) = fetch_with(&gateway, &timeline, Some(&cache), just_after, false).await;
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.cache_outcome, CacheOutcome::NotSaved);
        assert!(!sink.kinds.contains(&"Saving"));
        assert_eq!(cache.read_header(&cache_key()), HeaderRead::Missing);
    }

    #[tokio::test]
    async fn a_broken_cache_adds_nothing_is_removed_and_the_logs_are_fetched_again() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        cache
            .write(&cache_key(), whole_range(), vec![stored(START + 5, "a", 0)])
            .unwrap();
        let path = cache.entry_path(&cache_key());
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, &text[..text.len() - 3]).unwrap();

        let gateway = two_streams();
        let timeline = Mutex::new(EventTimeline::new());
        let just_after = START + 60_000;
        let (job, _sink) = fetch_with(&gateway, &timeline, Some(&cache), just_after, false).await;
        assert!(job.read_failed);
        assert!(!job.served_from_cache);
        assert_eq!(job.cache_outcome, CacheOutcome::NotSaved);
        assert_eq!(gateway.calls(), 3, "fetched from AWS");
        assert!(!path.exists(), "the broken file was removed");
        assert_eq!(
            messages(&timeline),
            vec![
                triple("a", "a1", 0),
                triple("b", "b1", 0),
                triple("a", "a2", 1)
            ],
            "nothing from the broken cache"
        );
    }

    #[tokio::test]
    async fn an_unreadable_cache_is_kept_and_a_failed_write_keeps_the_rows() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        // A folder where the cache file should be: it cannot be read, and
        // the new file cannot replace it.
        std::fs::create_dir(cache.entry_path(&cache_key())).unwrap();
        let gateway = two_streams();
        let timeline = Mutex::new(EventTimeline::new());
        let (job, sink) = fetch(&gateway, &timeline, Some(&cache)).await;
        assert!(job.read_failed);
        assert_eq!(job.cache_outcome, CacheOutcome::SaveFailed);
        assert_eq!(
            job.status,
            JobStatus::Completed,
            "the fetch itself succeeded"
        );
        assert_eq!(&sink.kinds[sink.kinds.len() - 2..], ["Saving", "Finished"]);
        assert!(cache.entry_path(&cache_key()).is_dir(), "kept");
        assert_eq!(timeline.lock().unwrap().len(), 3, "the fetched rows stay");
    }

    #[test]
    fn held_events_of_a_range_are_copied_in_chunks() {
        let timeline = Mutex::new(EventTimeline::new());
        timeline.add(
            (0..10)
                .map(|i| LogEvent {
                    timestamp: i * 10,
                    ingestion_time: None,
                    message: String::new(),
                    log_stream_name: "s".to_string(),
                    sequence: i as u64,
                })
                .collect(),
        );
        let range = CoveredRange {
            start_ms: 15,
            end_ms: 70,
        };
        let (first, next) = timeline.copy_events_in_range(range, None, 4);
        assert_eq!(
            first.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            [20, 30, 40, 50]
        );
        let (second, next) = timeline.copy_events_in_range(range, next, 4);
        assert_eq!(
            second.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            [60, 70]
        );
        assert_eq!(next, None);
        let nothing = CoveredRange {
            start_ms: 200,
            end_ms: 300,
        };
        assert_eq!(
            timeline.copy_events_in_range(nothing, None, 4),
            (Vec::new(), None)
        );
    }
}
