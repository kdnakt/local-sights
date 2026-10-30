//! QueryRunner: one run of a read query over up to 50 log groups, fetching
//! only what is missing, then evaluating (261017-insights-query U3, UC1-UC3,
//! BR2.1-BR2.7, BR3.2, BR3.3, BR5.2).
//!
//! A run goes:
//!
//! 1. the QueryStore discards every log group's staging events (BR1.3);
//! 2. for every chosen log group, the part of the period it has not
//!    fetched (BR1.1);
//! 3. with the disk cache on, what of those parts the cache holds is read
//!    and committed (BR2.1, review R-06);
//! 4. the rest is fetched from AWS, log group by log group in the order
//!    chosen and range by range in time order, each range through the
//!    existing flow [`run_fetch_with_cache`] (stream selection, eight streams
//!    side by side, call pacing, retries, the give-up rule, the cache write)
//!    into a holder of its own, handed to the store when it ends (BR2.2,
//!    review R-01). Nothing is fetched for a log group with nothing missing
//!    (BR2.3);
//! 5. the end is decided from every log group's outcome (BR2.5): [Cancel]
//!    or a closing app evaluates nothing; a log group whose fetch failed
//!    evaluates nothing either (FetchFailed); a log group where some streams
//!    failed makes the result partial.
//!
//! A log group that fails does not stop the others, so every failed log
//! group can be shown, except when the credentials failed (expired SSO, no
//! credentials): the rest would fail the same, so they are not started
//! (review R-04). [Cancel] stops the fetch under way and starts nothing
//! more (BR2.4). The streams that failed carry their log group (BR2.6).
//!
//! The evaluation goes step by step over the run's view of the store, each
//! step one chunk of
//! [`EVALUATION_CHUNK`](crate::query::evaluator::EVALUATION_CHUNK) rows on a
//! blocking thread under one lock, with the stop signal, the sink's answer
//! and the epoch checked between steps (U1:BR5.8, U2:BR3.1). When it ends
//! the result rows are copied out of the store (BR3.2). A step that ends
//! abnormally is an evaluation failure (U2:BR10.2). The JSON field names of
//! the view are gathered for completion after the fetch (BR3.3). The Log
//! Management screen's timeline is never touched (U1:BR1.7).

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;

use crate::cache::plan::CoveredRange;
use crate::cache::{CacheOverlap, LogCache};
use crate::coordinator::{DeferredFinishSink, FetchJob, FetchSink, run_fetch_with_cache};
use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};
use crate::fetch_status::JobStatus;
use crate::gateway::CloudWatchLogsGateway;
use crate::query::evaluator::{EvalOutput, Evaluation, QueryResult};
use crate::query::plan::QueryPlan;
use crate::query::store::{HoldingKey, QueryStore};
use crate::request::{FetchRequest, ValidatedFetch};
use crate::retry::{Retrier, StopReason};
use crate::time_range::TimeRange;
use crate::timeline::EventTimeline;

/// How one chosen log group's fetch ended in a run (entities.md
/// GroupFetchOutcome.status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GroupFetchStatus {
    /// Everything was fetched already: neither AWS nor the cache was used
    /// (BR2.3).
    NotNeeded,
    /// The missing parts came from the disk cache only (BR2.1).
    FromCache,
    /// Every missing part was fetched from AWS.
    Completed,
    /// Some streams failed or the listing ended early; the rest is used.
    CompletedWithFailures,
    /// A fetch failed as a whole, or the credentials failed (R-04).
    Failed,
    /// [Cancel] or the closing app stopped it, or it was never started
    /// after one.
    Cancelled,
}

/// A stream that could not be fetched, with its log group (BR2.6): two log
/// groups may have streams of one name (FR2.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupFailedStream {
    /// The log group.
    pub log_group_name: String,
    /// The stream.
    pub log_stream_name: String,
    /// Kind and safe detail only.
    pub failure: ApiFailure,
}

/// One chosen log group's fetch in a run (entities.md GroupFetchOutcome).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupFetchOutcome {
    /// The log group.
    pub log_group_name: String,
    /// How it ended.
    pub status: GroupFetchStatus,
    /// The streams that failed.
    pub failed_streams: Vec<GroupFailedStream>,
    /// Why the fetch failed as a whole (Failed), or the listing failure of
    /// a listing that ended early.
    pub failure: Option<ApiFailure>,
}

impl GroupFetchOutcome {
    fn new(log_group_name: &str) -> Self {
        Self {
            log_group_name: log_group_name.to_string(),
            status: GroupFetchStatus::NotNeeded,
            failed_streams: Vec::new(),
            failure: None,
        }
    }
}

/// The fetch of one run (entities.md QueryRunSummary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRunSummary {
    /// Every chosen log group, in the order chosen.
    pub groups: Vec<GroupFetchOutcome>,
    /// Some log group completed with failures: the result is partial.
    pub partial: bool,
    /// Whether AWS was called (FR2.3).
    pub from_aws: bool,
    /// [Cancel] (or the closing app) stopped the run.
    pub stopped: bool,
    /// The app closes: the run ends as if it never started (U1:BR2.8).
    pub closed: bool,
}

impl QueryRunSummary {
    /// Whether a log group failed (BR2.5: nothing is evaluated).
    pub fn failed(&self) -> bool {
        self.groups
            .iter()
            .any(|group| group.status == GroupFetchStatus::Failed)
    }

    /// The streams that failed, log group by log group (BR2.6).
    pub fn failed_streams(&self) -> Vec<GroupFailedStream> {
        self.groups
            .iter()
            .flat_map(|group| group.failed_streams.iter().cloned())
            .collect()
    }
}

/// Receives one run's progress: every range fetch's notices as any fetch
/// sink, the run's own steps, then the evaluation's progress and its end.
pub trait QuerySink: FetchSink + Send {
    /// The disk cache is read for the chosen log groups (BR2.7).
    fn on_reading_cache(&mut self, run_id: u64);
    /// Chosen log group `index` (from 0) of `count` starts its fetch from
    /// AWS (BR2.7).
    fn on_group_started(&mut self, run_id: u64, index: usize, count: usize);
    /// One range of `log_group_name` was fetched with `job` (its
    /// `on_finished` is not sent).
    fn on_range_fetched(&mut self, run_id: u64, log_group_name: &str, job: &FetchJob);
    /// The fetch of `run_id` ended with `summary`. Returns whether the run
    /// may still be evaluated: `false` once [Cancel] was pressed or the run
    /// is no longer the current one (U1 review R-01).
    fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> bool;
    /// The JSON field names of the logs of `run_id` (BR3.3).
    fn on_field_names(&mut self, run_id: u64, names: Vec<String>);
    /// `evaluated` of `total` events were evaluated. Returns whether to go
    /// on (U1:BR2.4, BR5.8).
    fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool;
    /// The evaluation finished with `result`.
    fn on_evaluated(&mut self, result: QueryResult);
    /// The evaluation stopped before its end; what it found is dropped
    /// (U1:BR2.7, BR5.8).
    fn on_evaluation_stopped(&mut self, run_id: u64);
    /// The evaluation ended abnormally; nothing was found (U2:BR10.2).
    fn on_evaluation_failed(&mut self, run_id: u64);
}

/// One step of an evaluation over the store: [`QueryStore::evaluate_step`],
/// or in tests one that fails.
type EvaluateStep = fn(&QueryStore, &QueryPlan, &mut Evaluation, u64) -> bool;

/// What one run needs (UC1).
#[derive(Debug, Clone)]
pub struct QueryRun {
    /// The run, as InsightsSession numbered it.
    pub run_id: u64,
    /// The read plan with its date texts already in milliseconds.
    pub plan: Arc<QueryPlan>,
    /// The period, both ends included.
    pub range: TimeRange,
    /// The chosen log groups of the connection, in the order chosen.
    pub keys: Vec<HoldingKey>,
    /// The disk cache, `None` when disabled (U6:BR1.5).
    pub cache: Option<LogCache>,
    /// When the run started, epoch milliseconds: the base of the
    /// five-minute boundary of every log group (BR1.2, review R-07).
    pub run_started_at_ms: i64,
    /// The time now, epoch milliseconds: each range fetch's start, the base
    /// of its cache write as before (U6:BR3.1).
    pub clock: fn() -> i64,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOutcome {
    /// Evaluated to its end; `rows` events matched.
    Evaluated {
        /// Number of result rows.
        rows: usize,
    },
    /// Not evaluated: Failed (a log group failed), Cancelled ([Cancel]) or
    /// Aborted (the app closes).
    NotEvaluated(JobStatus),
    /// The evaluation was stopped between two chunks.
    EvaluationStopped,
    /// An evaluation step ended abnormally (its thread panicked); the sink
    /// heard [`QuerySink::on_evaluation_failed`] (U2:BR10.2).
    EvaluationFailed,
}

/// Runs `run` (UC1): fetches what is missing and evaluates when the fetch
/// allows it.
pub async fn run_query<G, S>(
    gateway: &G,
    run: &QueryRun,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> QueryOutcome
where
    G: CloudWatchLogsGateway,
    S: QuerySink,
{
    let epoch = store.begin_run(run.keys.clone());
    let summary = fetch_missing(gateway, run, store, retrier, sink).await;
    let allowed = sink.on_fetch_ended(run.run_id, &summary);
    if summary.closed || retrier.stop_signal().reason() == Some(StopReason::Close) {
        return QueryOutcome::NotEvaluated(JobStatus::Aborted);
    }
    if summary.failed() {
        return QueryOutcome::NotEvaluated(JobStatus::Failed);
    }
    let range = run.range;
    let view = {
        let store = Arc::clone(store);
        on_blocking(move || store.build_view(epoch, range.start_instant(), range.end_instant()))
            .await
            .flatten()
    };
    let Some(total) = view else {
        return QueryOutcome::NotEvaluated(JobStatus::Cancelled);
    };
    gather_field_names(run.run_id, epoch, store, sink).await;
    if summary.stopped || !allowed {
        return QueryOutcome::NotEvaluated(JobStatus::Cancelled);
    }
    evaluate_held(
        run,
        epoch,
        total,
        store,
        retrier,
        sink,
        QueryStore::evaluate_step,
    )
    .await
}

/// Runs `work` on a blocking thread; `None`, with a diagnostic, when that
/// thread ended abnormally.
async fn on_blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    match tokio::task::spawn_blocking(work).await {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("local-sights: query: work on the held logs ended abnormally");
            None
        }
    }
}

/// UC1 3-8: the missing parts of every chosen log group, from the cache and
/// then from AWS.
async fn fetch_missing<G, S>(
    gateway: &G,
    run: &QueryRun,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> QueryRunSummary
where
    G: CloudWatchLogsGateway,
    S: QuerySink,
{
    let (start_ms, end_ms) = (run.range.start_instant(), run.range.end_instant());
    let mut groups: Vec<GroupFetchOutcome> = run
        .keys
        .iter()
        .map(|key| GroupFetchOutcome::new(&key.log_group_name))
        .collect();
    let mut missing: Vec<Vec<CoveredRange>> = run
        .keys
        .iter()
        .map(|key| store.missing_ranges(key, start_ms, end_ms))
        .collect();
    if let Some(cache) = &run.cache {
        if missing.iter().any(|parts| !parts.is_empty()) && !retrier.is_stopped() {
            sink.on_reading_cache(run.run_id);
            read_cache(cache, run, store, retrier, &mut missing, &mut groups).await;
        }
    }
    let mut from_aws = false;
    let mut halted: Option<ApiFailure> = None;
    let count = run.keys.len();
    for (index, key) in run.keys.iter().enumerate() {
        if missing[index].is_empty() {
            continue;
        }
        if let Some(failure) = &halted {
            // R-04: not started; it would fail the same.
            groups[index].status = GroupFetchStatus::Failed;
            groups[index].failure = Some(failure.clone());
            continue;
        }
        if retrier.is_stopped() {
            // BR2.4: nothing more starts after [Cancel].
            groups[index].status = GroupFetchStatus::Cancelled;
            continue;
        }
        sink.on_group_started(run.run_id, index, count);
        from_aws = true;
        let ranges = std::mem::take(&mut missing[index]);
        let outcome = &mut groups[index];
        outcome.status = GroupFetchStatus::Completed;
        for range in ranges {
            let job = fetch_range(gateway, run, key, range, store, retrier, sink).await;
            sink.on_range_fetched(run.run_id, &key.log_group_name, &job);
            record_range_end(outcome, &job);
            if matches!(
                outcome.status,
                GroupFetchStatus::Failed | GroupFetchStatus::Cancelled
            ) {
                break;
            }
        }
        // R-04: every other log group would fail the same.
        halted = credential_failure(outcome);
    }
    let partial = groups
        .iter()
        .any(|group| group.status == GroupFetchStatus::CompletedWithFailures);
    QueryRunSummary {
        groups,
        partial,
        from_aws,
        stopped: retrier.is_stopped(),
        closed: retrier.stop_signal().reason() == Some(StopReason::Close),
    }
}

/// BR2.1: for every log group with missing parts, the parts the cache
/// holds are read and committed, then the parts are worked out again. A
/// cache that cannot be read or is broken leaves its parts to AWS.
async fn read_cache(
    cache: &LogCache,
    run: &QueryRun,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    missing: &mut [Vec<CoveredRange>],
    groups: &mut [GroupFetchOutcome],
) {
    let (start_ms, end_ms) = (run.range.start_instant(), run.range.end_instant());
    for (index, key) in run.keys.iter().enumerate() {
        if missing[index].is_empty() {
            continue;
        }
        if retrier.is_stopped() {
            return;
        }
        let (cache, store_ref, key_ref) = (cache.clone(), Arc::clone(store), key.clone());
        let wanted = missing[index].clone();
        let read = on_blocking(
            move || match cache.read_overlaps(&key_ref.cache_key(), &wanted) {
                CacheOverlap::Found { ranges, events } if !ranges.is_empty() => {
                    store_ref.add_cached(&key_ref, &ranges, events);
                    true
                }
                CacheOverlap::Found { .. } | CacheOverlap::Missing => false,
                CacheOverlap::Unreadable | CacheOverlap::Corrupt(_) => false,
            },
        )
        .await;
        if read == Some(true) {
            missing[index] = store.missing_ranges(key, start_ms, end_ms);
            if missing[index].is_empty() {
                groups[index].status = GroupFetchStatus::FromCache;
            }
        }
    }
}

/// BR2.2, review R-01: fetches `range` of `key` through the existing flow
/// into a holder of its own, then hands the events to the store, which
/// commits what a completed fetch may commit (BR1.2).
async fn fetch_range<G, S>(
    gateway: &G,
    run: &QueryRun,
    key: &HoldingKey,
    range: CoveredRange,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> FetchJob
where
    G: CloudWatchLogsGateway,
    S: QuerySink,
{
    let Ok(period) = TimeRange::from_instants(range.start_ms, range.end_ms) else {
        // Never: a missing part is never reversed.
        let mut job = FetchJob::start(run.range);
        job.status = JobStatus::Completed;
        return job;
    };
    let fetch = ValidatedFetch {
        request: FetchRequest::new(
            key.profile.clone(),
            Some(key.region.clone()),
            key.log_group_name.clone(),
        ),
        range: period,
    };
    let holder = Arc::new(Mutex::new(EventTimeline::new()));
    let mut deferred = DeferredFinishSink::new(&mut *sink);
    let returned = run_fetch_with_cache(
        gateway,
        &fetch,
        &holder,
        retrier,
        run.cache.as_ref(),
        (run.clock)(),
        &mut deferred,
    )
    .await;
    let mut job = deferred
        .take_finished()
        .map_or(returned, |(job, _version)| job);
    let events = holder
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_events();
    let completed = job.status == JobStatus::Completed;
    let (store, key, started) = (Arc::clone(store), key.clone(), run.run_started_at_ms);
    let settled =
        on_blocking(move || store.settle_range(&key, range, events, completed, started)).await;
    if settled.is_none() {
        // The fetched events were lost: the range counts as failed.
        job.status = JobStatus::Failed;
        job.listing_failure = Some(ApiFailure::new(
            FailureKind::Other,
            &SafeDetailFields::default(),
        ));
    }
    job
}

/// Folds the end of one range fetch into its log group's outcome (BR2.5,
/// BR2.6): the worst status wins, the failed streams are named with the
/// log group.
fn record_range_end(outcome: &mut GroupFetchOutcome, job: &FetchJob) {
    for failed in &job.failed_streams {
        outcome.failed_streams.push(GroupFailedStream {
            log_group_name: outcome.log_group_name.clone(),
            log_stream_name: failed.log_stream_name.clone(),
            failure: failed.failure.clone(),
        });
    }
    let status = match job.status {
        JobStatus::Completed | JobStatus::Running => GroupFetchStatus::Completed,
        JobStatus::CompletedWithFailures => GroupFetchStatus::CompletedWithFailures,
        JobStatus::Failed => GroupFetchStatus::Failed,
        JobStatus::Cancelled | JobStatus::Aborted => GroupFetchStatus::Cancelled,
    };
    if job.listing_failure.is_some() && outcome.failure.is_none() {
        outcome.failure.clone_from(&job.listing_failure);
    }
    if rank(status) > rank(outcome.status) {
        outcome.status = status;
    }
    if credential_failure(outcome).is_some() {
        outcome.status = GroupFetchStatus::Failed;
        if outcome.failure.is_none() {
            outcome.failure = credential_failure(outcome);
        }
    }
}

/// How bad a status is, for [`record_range_end`].
fn rank(status: GroupFetchStatus) -> u8 {
    match status {
        GroupFetchStatus::NotNeeded | GroupFetchStatus::FromCache => 0,
        GroupFetchStatus::Completed => 1,
        GroupFetchStatus::CompletedWithFailures => 2,
        GroupFetchStatus::Cancelled => 3,
        GroupFetchStatus::Failed => 4,
    }
}

/// R-04: the credentials' failure of `outcome`, if its listing or one of its
/// streams failed for missing or expired credentials.
fn credential_failure(outcome: &GroupFetchOutcome) -> Option<ApiFailure> {
    let is_credential = |failure: &ApiFailure| failure.kind() == FailureKind::AuthRequired;
    outcome
        .failure
        .iter()
        .find(|failure| is_credential(failure))
        .or_else(|| {
            outcome
                .failed_streams
                .iter()
                .map(|stream| &stream.failure)
                .find(|failure| is_credential(failure))
        })
        .cloned()
}

/// BR3.3: the JSON field names of the view, read on a blocking thread. A
/// read that ends abnormally only leaves completion without the names (it
/// is a Should, FR9.1).
async fn gather_field_names<S: QuerySink>(
    run_id: u64,
    epoch: u64,
    store: &Arc<QueryStore>,
    sink: &mut S,
) {
    let task_store = Arc::clone(store);
    if let Some(Some(names)) = on_blocking(move || task_store.json_field_names(epoch)).await {
        sink.on_field_names(run_id, names);
    }
}

/// UC1 9-10: evaluates the view step by step with `step`, then copies the
/// result rows (BR3.2).
async fn evaluate_held<S: QuerySink>(
    run: &QueryRun,
    epoch: u64,
    total: usize,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
    step: EvaluateStep,
) -> QueryOutcome {
    let mut evaluation = Evaluation::new(&run.plan, total);
    while !evaluation.is_done() {
        if retrier.is_stopped() {
            sink.on_evaluation_stopped(run.run_id);
            return QueryOutcome::EvaluationStopped;
        }
        let task_store = Arc::clone(store);
        let plan = Arc::clone(&run.plan);
        let task = tokio::task::spawn_blocking(move || {
            let current = step(&task_store, &plan, &mut evaluation, epoch);
            (evaluation, current)
        });
        let current;
        (evaluation, current) = match task.await {
            Ok(done) => done,
            Err(_) => return evaluation_failed(run.run_id, sink),
        };
        let go_on = current
            && sink.on_evaluation_progress(run.run_id, evaluation.evaluated(), evaluation.total());
        if !go_on {
            sink.on_evaluation_stopped(run.run_id);
            return QueryOutcome::EvaluationStopped;
        }
    }
    let output = match evaluation.into_output() {
        Some(EvalOutput::Events(output)) => output,
        // 261025-insights-stats BR3.5: the aggregate rows are copies
        // already; nothing is copied out of the holdings.
        Some(EvalOutput::Aggregates(output)) => {
            let count = output.rows.len();
            sink.on_evaluated(QueryResult::from_aggregates(run.run_id, output));
            return QueryOutcome::Evaluated { rows: count };
        }
        None => return evaluation_failed(run.run_id, sink),
    };
    let positions = output.rows.clone();
    let task_store = Arc::clone(store);
    let Some(copied) = on_blocking(move || task_store.copy_rows(epoch, &positions)).await else {
        return evaluation_failed(run.run_id, sink);
    };
    let Some(rows) = copied else {
        sink.on_evaluation_stopped(run.run_id);
        return QueryOutcome::EvaluationStopped;
    };
    let count = rows.len();
    sink.on_evaluated(QueryResult::from_output(run.run_id, output, rows));
    QueryOutcome::Evaluated { rows: count }
}

/// U2:BR10.2: tells the sink that the evaluation of `run_id` failed.
fn evaluation_failed<S: QuerySink>(run_id: u64, sink: &mut S) -> QueryOutcome {
    eprintln!("local-sights: query: an evaluation step ended abnormally");
    sink.on_evaluation_failed(run_id);
    QueryOutcome::EvaluationFailed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::plan::CachedEvent;
    use crate::catalog::ProfileSelector;
    use crate::coordinator::{BatchProgress, TimelineStore};
    use crate::event::LogEvent;
    use crate::fetcher::StreamFetchOutcome;
    use crate::gateway::{
        DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
        DescribeLogStreamsRequest, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
    };
    use crate::log_view::LogView;
    use crate::query::evaluator::ResultRow;
    use crate::query::parser::parse_query;
    use crate::query::stats::CellValue;
    use crate::retry::{FixedJitter, RetryPolicy, StopHandle, stop_pair};
    use crate::streams::LogStream;
    use crate::streams::planner::ListingProgress;

    /// 2024/01/02 at `hh:mm:ss` UTC, in epoch milliseconds.
    fn at(hour: i64, minute: i64, second: i64) -> i64 {
        1_704_153_600_000 + ((hour * 60 + minute) * 60 + second) * 1_000
    }

    /// A day after every event of the tests: nothing fetched is too new.
    fn a_day_later() -> i64 {
        at(10, 0, 0) + 86_400_000
    }

    const ORDERS: &str = "/aws/lambda/orders";
    const PAYMENTS: &str = "/aws/lambda/payments";

    /// Serves the events of several log groups, one page per stream, fails
    /// the streams and listings it is told to, and writes down every call.
    /// Never reaches AWS.
    #[derive(Default)]
    struct FakeLogs {
        /// (log group, stream, time, message).
        events: Vec<(String, String, i64, String)>,
        failing_streams: Vec<(String, String)>,
        failing_listings: Vec<(String, FailureKind)>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeLogs {
        fn with(events: &[(&str, &str, i64, &str)]) -> Self {
            Self {
                events: events
                    .iter()
                    .map(|(group, stream, at, message)| {
                        (
                            (*group).to_string(),
                            (*stream).to_string(),
                            *at,
                            (*message).to_string(),
                        )
                    })
                    .collect(),
                ..Self::default()
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// The `GetLogEvents` calls as (log group, start, end exclusive).
        fn event_calls(&self) -> Vec<(String, i64, i64)> {
            self.calls()
                .iter()
                .filter_map(|call| {
                    let mut words = call.split(' ');
                    (words.next()? == "events").then_some(())?;
                    let group = words.next()?.to_string();
                    let _stream = words.next()?;
                    let start = words.next()?.parse().ok()?;
                    let end = words.next()?.parse().ok()?;
                    Some((group, start, end))
                })
                .collect()
        }

        fn note(&self, call: String) {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(call);
        }
    }

    fn failure(kind: FailureKind) -> ApiFailure {
        ApiFailure::new(kind, &SafeDetailFields::default())
    }

    impl CloudWatchLogsGateway for FakeLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            self.note(format!(
                "events {} {} {} {}",
                request.log_group_name,
                request.log_stream_name,
                request.start_time,
                request.end_time
            ));
            let failing = self.failing_streams.iter().any(|(group, stream)| {
                *group == request.log_group_name && *stream == request.log_stream_name
            });
            if failing {
                return Err(failure(FailureKind::AccessDenied));
            }
            Ok(GetLogEventsPage {
                events: self
                    .events
                    .iter()
                    .filter(|(group, stream, at, _)| {
                        *group == request.log_group_name
                            && *stream == request.log_stream_name
                            && *at >= request.start_time
                            && *at < request.end_time
                    })
                    .map(|(_, _, at, message)| GatewayEvent {
                        timestamp: *at,
                        ingestion_time: None,
                        message: message.clone(),
                    })
                    .collect(),
                next_forward_token: None,
            })
        }

        async fn describe_log_groups(
            &self,
            _request: DescribeLogGroupsRequest,
        ) -> Result<DescribeLogGroupsPage, ApiFailure> {
            panic!("a query never lists log groups");
        }

        async fn describe_log_streams(
            &self,
            request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            self.note(format!("streams {}", request.log_group_name));
            if let Some((_, kind)) = self
                .failing_listings
                .iter()
                .find(|(group, _)| *group == request.log_group_name)
            {
                return Err(failure(*kind));
            }
            let mut names: Vec<&str> = self
                .events
                .iter()
                .filter(|(group, ..)| *group == request.log_group_name)
                .map(|(_, stream, ..)| stream.as_str())
                .chain(
                    self.failing_streams
                        .iter()
                        .filter(|(group, _)| *group == request.log_group_name)
                        .map(|(_, stream)| stream.as_str()),
                )
                .collect();
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

    /// Keeps what it hears, in order; can press [Cancel] at a chosen moment
    /// through the run's stop handle, as the session and the app would.
    #[derive(Default)]
    struct Recorder {
        heard: Vec<String>,
        handle: Option<StopHandle>,
        cancel_after_first_stream: bool,
        cancel_after_first_chunk: bool,
        cancelled: bool,
        summary: Option<QueryRunSummary>,
        results: Vec<QueryResult>,
        progress: Vec<(usize, usize)>,
        names: Vec<(u64, Vec<String>)>,
    }

    impl Recorder {
        fn cancel(&mut self) {
            self.cancelled = true;
            if let Some(handle) = &self.handle {
                handle.stop(StopReason::Cancel);
            }
        }
    }

    impl FetchSink for Recorder {
        fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {
            self.heard.push("started".to_string());
        }
        fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}
        fn on_planned(&mut self, _job: &FetchJob) {}
        fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}
        fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {
            if self.cancel_after_first_stream && !self.cancelled {
                self.cancel();
            }
        }
        fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {
            self.heard.push("finished".to_string());
        }
    }

    impl QuerySink for Recorder {
        fn on_reading_cache(&mut self, run_id: u64) {
            self.heard.push(format!("cache {run_id}"));
        }
        fn on_group_started(&mut self, _run_id: u64, index: usize, count: usize) {
            self.heard.push(format!("group {}/{count}", index + 1));
        }
        fn on_range_fetched(&mut self, _run_id: u64, log_group_name: &str, job: &FetchJob) {
            self.heard
                .push(format!("range {log_group_name} {:?}", job.status));
        }
        fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> bool {
            self.heard.push(format!("fetch ended {run_id}"));
            self.summary = Some(summary.clone());
            !self.cancelled
        }
        fn on_evaluation_progress(&mut self, _run_id: u64, evaluated: usize, total: usize) -> bool {
            self.progress.push((evaluated, total));
            if self.cancel_after_first_chunk {
                self.cancel();
            }
            true
        }
        fn on_evaluated(&mut self, result: QueryResult) {
            self.heard.push("evaluated".to_string());
            self.results.push(result);
        }
        fn on_evaluation_stopped(&mut self, run_id: u64) {
            self.heard.push(format!("stopped {run_id}"));
        }
        fn on_field_names(&mut self, run_id: u64, names: Vec<String>) {
            self.heard.push(format!("names {run_id}"));
            self.names.push((run_id, names));
        }
        fn on_evaluation_failed(&mut self, run_id: u64) {
            self.heard.push(format!("evaluation failed {run_id}"));
        }
    }

    fn key(group: &str) -> HoldingKey {
        HoldingKey {
            profile: ProfileSelector::Named("dev".to_string()),
            region: "ap-northeast-1".to_string(),
            log_group_name: group.to_string(),
        }
    }

    /// A run of `query` over `groups` from `start` to `end` (seconds, the
    /// end second included up to its 999th millisecond).
    fn run(query: &str, groups: &[&str], start: i64, end: i64) -> QueryRun {
        QueryRun {
            run_id: 7,
            plan: Arc::new(parse_query(query).unwrap()),
            range: TimeRange::from_seconds(start, end).unwrap(),
            keys: groups.iter().map(|group| key(group)).collect(),
            cache: None,
            run_started_at_ms: a_day_later(),
            clock: a_day_later,
        }
    }

    /// Runs `run` on `gateway`; `prepare` sets the recorder up with the
    /// run's stop handle.
    async fn query_with(
        gateway: &FakeLogs,
        store: &Arc<QueryStore>,
        run: &QueryRun,
        prepare: impl FnOnce(&mut Recorder),
    ) -> (QueryOutcome, Recorder) {
        let policy = RetryPolicy::default();
        let jitter = FixedJitter(0.0);
        let (handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &jitter, &signal);
        let mut recorder = Recorder {
            handle: Some(handle),
            ..Recorder::default()
        };
        prepare(&mut recorder);
        let outcome = run_query(gateway, run, store, &retrier, &mut recorder).await;
        (outcome, recorder)
    }

    /// The `(time, @log, message)` of every row of `result`.
    fn rows(result: &QueryResult) -> Vec<(i64, String, String)> {
        result
            .window(result.run_id, 0, usize::MAX)
            .iter()
            .filter_map(|row| {
                let event = row.event()?;
                Some((
                    event.timestamp,
                    row.log_group_name()?.to_string(),
                    event.message.clone(),
                ))
            })
            .collect()
    }

    fn group_statuses(recorder: &Recorder) -> Vec<GroupFetchStatus> {
        recorder
            .summary
            .as_ref()
            .map(|summary| summary.groups.iter().map(|group| group.status).collect())
            .unwrap_or_default()
    }

    /// Events of `ORDERS` from 10:00 to 12:00, one every ten minutes on
    /// stream `s1`, with one at 10:59:59 and one at 11:00:01.
    fn orders_morning() -> FakeLogs {
        let mut events: Vec<(&str, &str, i64, &str)> = (0..=12)
            .map(|step| (ORDERS, "s1", at(10, 0, 0) + step * 600_000, "ERROR tick"))
            .collect();
        events.push((ORDERS, "s1", at(10, 59, 59), "ERROR edge before"));
        events.push((ORDERS, "s1", at(11, 0, 1), "ERROR edge after"));
        FakeLogs::with(&events)
    }

    #[tokio::test]
    async fn fr2_ac1_only_the_missing_part_is_fetched_then_the_whole_period_is_evaluated() {
        // FR2 acceptance 1: 10:00-11:00 fetched, then 10:00-12:00.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let first = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        let (outcome, _) = query_with(&gateway, &store, &first, |_| {}).await;
        assert_eq!(
            outcome,
            QueryOutcome::Evaluated { rows: 8 },
            "10:00 to 11:00 inclusive"
        );
        let before = gateway.event_calls().len();

        let second = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(12, 0, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &second, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 15 });
        let calls = gateway.event_calls();
        assert_eq!(
            calls[before..],
            [(ORDERS.to_string(), at(11, 0, 1), at(12, 0, 1))],
            "one call, from the millisecond after 11:00:00.999"
        );
        let summary = recorder.summary.unwrap();
        assert!(summary.from_aws);
        assert_eq!(summary.groups[0].status, GroupFetchStatus::Completed);
    }

    #[tokio::test]
    async fn fr2_ac2_a_period_fetched_already_calls_no_aws_api() {
        // FR2 acceptance 2, BR2.3.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let first = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        query_with(&gateway, &store, &first, |_| {}).await;
        let calls = gateway.calls().len();
        let inner = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 15, 0),
            at(10, 45, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &inner, |_| {}).await;
        assert_eq!(
            outcome,
            QueryOutcome::Evaluated { rows: 3 },
            "10:20, 10:30, 10:40"
        );
        assert_eq!(
            gateway.calls().len(),
            calls,
            "neither stream listing nor events"
        );
        let summary = recorder.summary.unwrap();
        assert!(!summary.from_aws);
        assert_eq!(summary.groups[0].status, GroupFetchStatus::NotNeeded);
        assert!(
            !recorder
                .heard
                .iter()
                .any(|heard| heard.starts_with("group")),
            "no log group started a fetch"
        );
    }

    #[tokio::test]
    async fn fr2_ac3_an_event_on_both_sides_of_a_border_is_in_the_result_once() {
        // FR2 acceptance 3, FR2.7.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let first = run(
            "filter @message like 'edge'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        query_with(&gateway, &store, &first, |_| {}).await;
        let second = run(
            "filter @message like 'edge'",
            &[ORDERS],
            at(10, 59, 0),
            at(11, 1, 0),
        );
        let (_, recorder) = query_with(&gateway, &store, &second, |_| {}).await;
        assert_eq!(
            rows(&recorder.results[0]),
            vec![
                (
                    at(11, 0, 1),
                    ORDERS.to_string(),
                    "ERROR edge after".to_string()
                ),
                (
                    at(10, 59, 59),
                    ORDERS.to_string(),
                    "ERROR edge before".to_string()
                ),
            ],
            "each once, newest first"
        );
    }

    #[tokio::test]
    async fn fr2_ac4_streams_of_one_name_in_two_log_groups_are_told_apart_by_log() {
        // FR2 acceptance 4, FR2.8, FR1.3's acceptance with two log groups.
        let gateway = FakeLogs::with(&[
            (ORDERS, "s1", at(10, 0, 1), "from orders"),
            (PAYMENTS, "s1", at(10, 0, 1), "from payments"),
            (PAYMENTS, "s1", at(10, 0, 2), "later in payments"),
        ]);
        let store = Arc::new(QueryStore::new());
        let both = run(
            "fields @log, @logStream",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &both, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 3 });
        assert_eq!(
            rows(&recorder.results[0]),
            vec![
                (
                    at(10, 0, 2),
                    PAYMENTS.to_string(),
                    "later in payments".to_string()
                ),
                (at(10, 0, 1), ORDERS.to_string(), "from orders".to_string()),
                (
                    at(10, 0, 1),
                    PAYMENTS.to_string(),
                    "from payments".to_string()
                ),
            ],
            "equal times by log group (U2:BR4.4)"
        );
        assert_eq!(
            recorder
                .heard
                .iter()
                .filter(|heard| heard.starts_with("group"))
                .collect::<Vec<_>>(),
            ["group 1/2", "group 2/2"]
        );
        let only = run(
            "filter @log = '/aws/lambda/payments'",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, _) = query_with(&gateway, &store, &only, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
    }

    #[tokio::test]
    async fn fr2_ac5_a_failed_stream_gives_a_partial_result_and_its_part_is_fetched_again() {
        // FR2 acceptance 5, FR2.5, BR1.2, BR1.3, BR2.6.
        let mut gateway = FakeLogs::with(&[
            (ORDERS, "a", at(10, 0, 10), "ERROR a"),
            (ORDERS, "a", at(10, 0, 20), "ERROR a again"),
        ]);
        gateway
            .failing_streams
            .push((ORDERS.to_string(), "b".to_string()));
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(
            outcome,
            QueryOutcome::Evaluated { rows: 2 },
            "what was fetched"
        );
        let summary = recorder.summary.unwrap();
        assert!(summary.partial);
        assert_eq!(
            summary.groups[0].status,
            GroupFetchStatus::CompletedWithFailures
        );
        assert_eq!(
            summary
                .failed_streams()
                .iter()
                .map(|failed| (
                    failed.log_group_name.as_str(),
                    failed.log_stream_name.as_str()
                ))
                .collect::<Vec<_>>(),
            [(ORDERS, "b")]
        );
        assert!(
            store.covered_ranges(&key(ORDERS)).is_empty(),
            "not taken as fetched"
        );
        let before = gateway.event_calls().len();
        let (outcome, _) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(
            outcome,
            QueryOutcome::Evaluated { rows: 2 },
            "no event twice"
        );
        assert!(
            gateway.event_calls().len() > before,
            "the part was fetched again"
        );
    }

    #[tokio::test]
    async fn fr2_ac6_with_the_cache_off_the_query_fetches_and_log_management_keeps_its_logs() {
        // FR2 acceptance 6, FR2.1, U1:BR1.7.
        let log_view = Mutex::new(LogView::new());
        log_view.add(vec![LogEvent {
            timestamp: at(10, 30, 0),
            ingestion_time: None,
            message: "shown in Log Management".to_string(),
            log_stream_name: "lm".to_string(),
            sequence: 0,
        }]);
        let before = log_view.lock().unwrap().rows(0, 10);
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        let (outcome, _) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 8 });
        assert!(!gateway.event_calls().is_empty(), "fetched from AWS");
        assert_eq!(log_view.lock().unwrap().rows(0, 10), before);
    }

    fn cached(timestamp: i64, stream: &str, sequence: u64) -> CachedEvent {
        CachedEvent {
            timestamp,
            ingestion_time: None,
            message: format!("ERROR cached {timestamp}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    #[tokio::test]
    async fn with_the_cache_on_its_overlapping_part_is_read_and_not_fetched() {
        // BR2.1: the cache holds 10:00:00-10:30:00.999, the run is
        // 10:00-11:00: only 10:30:01 on is fetched from AWS.
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().join("cache"));
        let cached_range = CoveredRange {
            start_ms: at(10, 0, 0),
            end_ms: at(10, 30, 0) + 999,
        };
        cache
            .write(
                &key(ORDERS).cache_key(),
                cached_range,
                vec![
                    cached(at(10, 5, 0), "s1", 0),
                    cached(at(10, 25, 0), "s1", 1),
                ],
            )
            .unwrap();
        let gateway = FakeLogs::with(&[(ORDERS, "s1", at(10, 45, 0), "ERROR fetched")]);
        let store = Arc::new(QueryStore::new());
        let mut period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        period.cache = Some(cache.clone());
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 3 });
        assert!(recorder.heard.contains(&"cache 7".to_string()));
        assert_eq!(
            gateway.event_calls(),
            [(ORDERS.to_string(), at(10, 30, 1), at(11, 0, 1))]
        );

        // Everything in the cache: no AWS call at all.
        let gateway = FakeLogs::default();
        let store = Arc::new(QueryStore::new());
        let mut inner = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 20, 0),
        );
        inner.cache = Some(cache);
        let (outcome, recorder) = query_with(&gateway, &store, &inner, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 1 });
        assert!(gateway.calls().is_empty());
        let summary = recorder.summary.unwrap();
        assert_eq!(summary.groups[0].status, GroupFetchStatus::FromCache);
        assert!(!summary.from_aws);
    }

    #[tokio::test]
    async fn a_log_group_left_out_of_a_run_is_fetched_whole_when_chosen_again() {
        // 261020-carryover-fixes FR2.2.1-FR2.2.3: run both, then ORDERS
        // only (PAYMENTS' holdings go), then both over a longer period:
        // PAYMENTS is fetched again from the start, ORDERS only the part
        // after what it holds.
        let gateway = FakeLogs::with(&[
            (ORDERS, "s1", at(10, 10, 0), "ERROR o1"),
            (ORDERS, "s1", at(11, 10, 0), "ERROR o2"),
            (PAYMENTS, "s1", at(10, 20, 0), "ERROR p1"),
            (PAYMENTS, "s1", at(11, 20, 0), "ERROR p2"),
        ]);
        let store = Arc::new(QueryStore::new());
        let query = "filter @message like 'ERROR'";
        let both = run(query, &[ORDERS, PAYMENTS], at(10, 0, 0), at(11, 0, 0));
        let (outcome, _) = query_with(&gateway, &store, &both, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
        let orders_only = run(query, &[ORDERS], at(10, 0, 0), at(11, 0, 0));
        let (outcome, _) = query_with(&gateway, &store, &orders_only, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 1 });
        assert_eq!(store.counts(&key(PAYMENTS)), (0, 0), "discarded");
        let before = gateway.event_calls().len();

        let longer = run(query, &[ORDERS, PAYMENTS], at(10, 0, 0), at(11, 30, 0));
        let (outcome, _) = query_with(&gateway, &store, &longer, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 4 });
        let mut calls = gateway.event_calls()[before..].to_vec();
        calls.sort();
        assert_eq!(
            calls,
            [
                (ORDERS.to_string(), at(11, 0, 1), at(11, 30, 1)),
                (PAYMENTS.to_string(), at(10, 0, 0), at(11, 30, 1)),
            ]
        );
    }

    #[tokio::test]
    async fn a_failed_log_group_is_not_evaluated_but_the_others_are_fetched() {
        // BR2.5, R-04: every failed log group can be shown.
        let mut gateway = FakeLogs::with(&[(PAYMENTS, "s1", at(10, 0, 1), "ERROR p")]);
        gateway
            .failing_listings
            .push((ORDERS.to_string(), FailureKind::AccessDenied));
        let store = Arc::new(QueryStore::new());
        let both = run(
            "filter @message like 'ERROR'",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &both, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Failed));
        assert!(recorder.results.is_empty() && recorder.progress.is_empty());
        assert_eq!(
            group_statuses(&recorder),
            [GroupFetchStatus::Failed, GroupFetchStatus::Completed]
        );
        let summary = recorder.summary.unwrap();
        assert_eq!(
            summary.groups[0].failure.as_ref().map(ApiFailure::kind),
            Some(FailureKind::AccessDenied)
        );
        assert_eq!(
            store.counts(&key(PAYMENTS)),
            (1, 0),
            "kept for the next run"
        );
        assert!(
            recorder.names.is_empty(),
            "no names after a failed fetch (BR5.2)"
        );
    }

    #[tokio::test]
    async fn failed_credentials_start_no_other_log_group() {
        // R-04: an expired SSO login fails every log group the same.
        let mut gateway = FakeLogs::with(&[(PAYMENTS, "s1", at(10, 0, 1), "ERROR p")]);
        gateway
            .failing_listings
            .push((ORDERS.to_string(), FailureKind::AuthRequired));
        let store = Arc::new(QueryStore::new());
        let both = run(
            "filter @message like 'ERROR'",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &both, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Failed));
        assert!(
            !gateway.calls().iter().any(|call| call.contains(PAYMENTS)),
            "payments was never asked"
        );
        let summary = recorder.summary.unwrap();
        assert_eq!(
            summary
                .groups
                .iter()
                .map(|group| (group.status, group.failure.as_ref().map(ApiFailure::kind)))
                .collect::<Vec<_>>(),
            [
                (GroupFetchStatus::Failed, Some(FailureKind::AuthRequired)),
                (GroupFetchStatus::Failed, Some(FailureKind::AuthRequired)),
            ]
        );
    }

    #[tokio::test]
    async fn cancel_stops_the_fetch_and_starts_no_other_log_group() {
        // BR2.4, U1:BR2.7.
        let gateway = FakeLogs::with(&[
            (ORDERS, "a", at(10, 0, 1), "ERROR a"),
            (ORDERS, "b", at(10, 0, 2), "ERROR b"),
            (PAYMENTS, "s1", at(10, 0, 3), "ERROR p"),
        ]);
        let store = Arc::new(QueryStore::new());
        let both = run(
            "filter @message like 'ERROR'",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &both, |recorder| {
            recorder.cancel_after_first_stream = true;
        })
        .await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Cancelled));
        assert!(recorder.results.is_empty());
        assert!(!gateway.calls().iter().any(|call| call.contains(PAYMENTS)));
        assert_eq!(
            group_statuses(&recorder),
            [GroupFetchStatus::Cancelled, GroupFetchStatus::Cancelled]
        );
        assert!(recorder.summary.unwrap().stopped);
        assert!(
            store.covered_ranges(&key(ORDERS)).is_empty(),
            "a cancelled range is not taken"
        );
    }

    #[tokio::test]
    async fn the_newest_five_minutes_are_staging_and_fetched_again_by_the_next_run() {
        // BR1.2, R-07: the run started at 10:10, so 10:05 on is too new.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let mut period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 10, 0),
        );
        period.run_started_at_ms = at(10, 10, 0);
        period.clock = || 0;
        let (outcome, _) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(
            outcome,
            QueryOutcome::Evaluated { rows: 2 },
            "10:00 and 10:10"
        );
        assert_eq!(
            store.covered_ranges(&key(ORDERS)),
            vec![CoveredRange {
                start_ms: at(10, 0, 0),
                end_ms: at(10, 5, 0)
            }]
        );
        assert_eq!(store.counts(&key(ORDERS)), (1, 1));
        let before = gateway.event_calls().len();
        let (outcome, _) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
        assert_eq!(
            gateway.event_calls()[before..],
            [(ORDERS.to_string(), at(10, 5, 0) + 1, at(10, 10, 0) + 1_000)]
        );
    }

    #[tokio::test]
    async fn a_result_keeps_its_rows_after_the_next_run_discards_the_staging_events() {
        // BR3.2, BR3.4: the rows are copies.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let mut period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 10, 0),
        );
        period.run_started_at_ms = at(10, 10, 0);
        let (_, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        let result = &recorder.results[0];
        store.begin_run(vec![key(ORDERS)]);
        store.discard_all();
        assert_eq!(rows(result).len(), 2, "still readable");
        assert_eq!(
            result
                .row_at(result.run_id, 0)
                .and_then(ResultRow::event)
                .map(|event| event.timestamp),
            Some(at(10, 10, 0))
        );
    }

    #[tokio::test]
    async fn the_copies_are_the_rows_left_after_the_limit_or_the_default_cut() {
        // BR3.2: at most what `limit` (0 to 100,000; 0 leaves no row,
        // Loop-back 3 p08) or the default 10,000 leaves is copied.
        let events: Vec<(String, String, i64, String)> = (0..12_000i64)
            .map(|i| {
                (
                    ORDERS.to_string(),
                    "s".to_string(),
                    at(10, 0, 0) + i,
                    "m".to_string(),
                )
            })
            .collect();
        let gateway = FakeLogs {
            events,
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let cut = run(
            "filter @message = 'm'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (_, recorder) = query_with(&gateway, &store, &cut, |_| {}).await;
        let result = &recorder.results[0];
        assert_eq!(
            (result.row_count(), result.matched_count()),
            (10_000, 12_000)
        );
        assert_eq!(rows(result).len(), 10_000);
        let limited = run(
            "filter @message = 'm' | limit 5",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (_, recorder) = query_with(&gateway, &store, &limited, |_| {}).await;
        assert_eq!(rows(&recorder.results[0]).len(), 5);
        assert_eq!(
            rows(&recorder.results[0])[0].0,
            at(10, 0, 0) + 11_999,
            "the newest first"
        );
    }

    #[tokio::test]
    async fn a_cancel_during_the_evaluation_stops_at_the_next_chunk_and_drops_the_rows() {
        // U1:BR2.7, BR5.8 with 25,000 events.
        let events: Vec<(String, String, i64, String)> = (0..25_000i64)
            .map(|i| {
                (
                    ORDERS.to_string(),
                    "s".to_string(),
                    at(10, 0, 0) + i,
                    "ERROR".to_string(),
                )
            })
            .collect();
        let gateway = FakeLogs {
            events,
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |recorder| {
            recorder.cancel_after_first_chunk = true;
        })
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluationStopped);
        assert_eq!(recorder.progress, vec![(10_000, 25_000)]);
        assert!(recorder.results.is_empty());
        assert_eq!(recorder.heard.last().map(String::as_str), Some("stopped 7"));
        assert_eq!(store.len(), 25_000, "the fetched events stay");
    }

    /// A step whose thread panics, as an abnormal end of the evaluation.
    fn panicking_step(_: &QueryStore, _: &QueryPlan, _: &mut Evaluation, _: u64) -> bool {
        panic!("an evaluation step that ends abnormally (test)");
    }

    #[tokio::test]
    async fn an_evaluation_that_ends_abnormally_is_told_as_an_evaluation_failure() {
        // U2:BR10.2 (U1 code review R-03): not a failed fetch.
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'E'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let epoch = store.begin_run(period.keys.clone());
        let range = CoveredRange {
            start_ms: at(10, 0, 0),
            end_ms: at(10, 1, 0),
        };
        store.settle_range(
            &key(ORDERS),
            range,
            vec![LogEvent {
                timestamp: at(10, 0, 1),
                ingestion_time: None,
                message: "ERROR".to_string(),
                log_stream_name: "s".to_string(),
                sequence: 0,
            }],
            true,
            a_day_later(),
        );
        store.build_view(epoch, range.start_ms, range.end_ms);
        let policy = RetryPolicy::default();
        let jitter = FixedJitter(0.0);
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &jitter, &signal);
        let mut recorder = Recorder::default();
        let outcome = evaluate_held(
            &period,
            epoch,
            1,
            &store,
            &retrier,
            &mut recorder,
            panicking_step,
        )
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluationFailed);
        assert_eq!(recorder.heard, vec!["evaluation failed 7"]);
        assert_eq!(store.len(), 1, "the fetched logs stay");
    }

    #[tokio::test]
    async fn the_field_names_of_the_view_are_told_after_the_fetch() {
        // BR3.3, BR5.2 (U2:BR1.7).
        let gateway = FakeLogs::with(&[
            (ORDERS, "s", at(10, 0, 1), r#"{"level":"error","n":1}"#),
            (PAYMENTS, "s", at(10, 0, 2), r#"{"http":{"status":500}}"#),
        ]);
        let store = Arc::new(QueryStore::new());
        let both = run("limit 1", &[ORDERS, PAYMENTS], at(10, 0, 0), at(10, 1, 0));
        let (_, recorder) = query_with(&gateway, &store, &both, |_| {}).await;
        assert_eq!(
            recorder.names,
            vec![(
                7,
                vec![
                    "http.status".to_string(),
                    "level".to_string(),
                    "n".to_string()
                ]
            )]
        );
        let at_the_end: Vec<&str> = recorder.heard[recorder.heard.len() - 3..]
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(at_the_end, ["fetch ended 7", "names 7", "evaluated"]);
    }

    #[tokio::test]
    async fn a_query_with_json_fields_sort_and_limit_runs_to_its_end() {
        // UC1 with U2:BR3.1-BR5.2: the columns, the order and the cut.
        let events: Vec<(String, String, i64, String)> = (0..30i64)
            .map(|i| {
                let level = if i % 2 == 0 {
                    r#","level":"error""#
                } else {
                    ""
                };
                (
                    ORDERS.to_string(),
                    "s".to_string(),
                    at(10, 0, 0) + i,
                    format!(r#"{{"n":{}{level}}}"#, i % 7),
                )
            })
            .collect();
        let gateway = FakeLogs {
            events,
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let query =
            "fields @timestamp, n, level | filter n >= 5 | sort n desc, @timestamp | limit 3";
        let period = run(query, &[ORDERS], at(10, 0, 0), at(10, 1, 0));
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 3 });
        let result = &recorder.results[0];
        let names = result.columns().names();
        assert_eq!(names, vec!["@timestamp", "n", "level"]);
        assert_eq!(result.matched_count(), 8, "n is 5 or 6 for 8 of 30");
        assert_eq!(result.truncated_at(), Some(3));
        let offsets: Vec<i64> = rows(result)
            .iter()
            .map(|(time, ..)| time - at(10, 0, 0))
            .collect();
        assert_eq!(offsets, vec![6, 13, 20], "n = 6, oldest first among equals");
    }

    // ---- 261025-insights-stats U1:BR3.1, BR3.5, BR5.2 --------------------

    /// 30 events of [`ORDERS`] a second apart from 10:00:00, every third an
    /// error.
    fn leveled_events() -> Vec<(String, String, i64, String)> {
        (0..30i64)
            .map(|i| {
                let level = if i % 3 == 0 { "error" } else { "info" };
                (
                    ORDERS.to_string(),
                    "s".to_string(),
                    at(10, 0, 0) + i * 1_000,
                    format!(r#"{{"level":"{level}","n":{i}}}"#),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn a_stats_query_runs_to_an_aggregates_result_of_its_own_rows() {
        // BR3.5: the runner's one branch; the rows are made by the
        // aggregator, not copied from the holdings, and stay readable after
        // the holdings are discarded.
        let gateway = FakeLogs {
            events: leveled_events(),
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let query = "filter level = 'error' | stats count(*) as n, max(n) by level";
        let period = run(query, &[ORDERS], at(10, 0, 0), at(10, 1, 0));
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 1 });
        let result = &recorder.results[0];
        assert_eq!(result.shape(), crate::query::plan::ResultShape::Aggregates);
        assert_eq!(result.columns().names(), vec!["level", "n", "max(n)"]);
        assert_eq!((result.matched_count(), result.truncated_at()), (1, None));
        assert_eq!(result.evaluated_count, 30);
        store.discard_all();
        assert_eq!(
            result.row_at(result.run_id, 0).and_then(ResultRow::cells),
            Some(
                &[
                    CellValue::Text("error".to_string()),
                    CellValue::Integer(10),
                    CellValue::Integer(27),
                ][..]
            )
        );
        assert!(rows(result).is_empty(), "no event rows");
    }

    #[tokio::test]
    async fn a_stats_query_over_bins_fetches_as_any_query_does() {
        // BR5.2: the fetch is the same GetLogEvents flow; the grouping is
        // done here. Bins of 10 seconds from the epoch.
        let gateway = FakeLogs {
            events: leveled_events(),
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let period = run(
            "stats count(*) by bin(10s)",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 3 });
        let result = &recorder.results[0];
        let firsts: Vec<CellValue> = (0..3)
            .filter_map(|index| result.row_at(result.run_id, index))
            .filter_map(|row| row.cells().and_then(|cells| cells.first()).cloned())
            .collect();
        assert_eq!(
            firsts,
            vec![
                CellValue::Timestamp(at(10, 0, 0)),
                CellValue::Timestamp(at(10, 0, 10)),
                CellValue::Timestamp(at(10, 0, 20)),
            ]
        );
        assert_eq!(recorder.heard.last().map(String::as_str), Some("evaluated"));
    }
}
