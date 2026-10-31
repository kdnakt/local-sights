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
//!
//! Since 261025-insights-stats U3 a run shows partial results while it
//! fetches (U3:BR1.1-BR2.1). The main flow keeps fetching as before and is
//! the only one holding the sink: it waits on the fetch and on a channel of
//! partial results with `select!`. A partial evaluation runs on a blocking
//! thread over the store's partial view (settled events plus copies of the
//! events under fetch, U3:BR1.3), started at once after the first page, then
//! at least [`PARTIAL_EVALUATION_INTERVAL`] after the last one ended and
//! only when more was fetched; only one that reached its end is sent, and a
//! change of the holdings meanwhile drops it (the view's version, U3:BR1.4).
//! When the fetch ends the running partial evaluation is told to stop and
//! waited for before the final view is built. Each range's holder is
//! registered in the store and settled there under one store lock
//! (U3:BR1.2). [Cancel] while fetching no longer drops the run: what was
//! fetched, staging included, is evaluated once into the cancelled result,
//! an evaluation only the second [Cancel] (the stop signal's abandon) stops
//! (U3:BR3.1, BR3.2).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::{Notify, mpsc};

use crate::cache::plan::CoveredRange;
use crate::cache::{CacheOverlap, LogCache};
use crate::coordinator::{
    BatchProgress, DeferredFinishSink, FetchJob, FetchSink, run_fetch_with_cache,
};
use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};
use crate::fetch_stats::FetchSummary;
use crate::fetch_status::JobStatus;
use crate::fetcher::StreamFetchOutcome;
use crate::gateway::CloudWatchLogsGateway;
use crate::insights_session::PartialCoverage;
use crate::query::evaluator::{EvalOutput, Evaluation, QueryResult};
use crate::query::plan::QueryPlan;
use crate::query::store::{HoldingKey, QueryStore};
use crate::request::{FetchRequest, ValidatedFetch};
use crate::retry::{Retrier, StopReason};
use crate::streams::planner::ListingProgress;
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
    /// The fetch of `run_id` ended with `summary`. Returns whether and how
    /// the run is evaluated: into the final result, after [Cancel] into the
    /// cancelled one (261025-insights-stats U3:BR3.1), or not at all (the
    /// run is no longer the current one, U1 review R-01, or the second
    /// [Cancel] came, U3:BR3.2).
    fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> FetchEnd;
    /// A partial result of `run_id` made while fetching, with what it holds
    /// (U3:BR2.1).
    fn on_partial_result(&mut self, run_id: u64, result: QueryResult, coverage: PartialCoverage);
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

/// 261025-insights-stats U3:BR1.1: the least time from the end of one
/// partial evaluation to the start of the next.
pub const PARTIAL_EVALUATION_INTERVAL: Duration = Duration::from_secs(1);

/// What the start of the next partial evaluation depends on (U3:BR1.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PartialGate {
    /// A range fetch from AWS started (FR4.9: none without missing parts).
    pub fetch_started: bool,
    /// The first page of events came.
    pub first_page: bool,
    /// A partial evaluation runs.
    pub running: bool,
    /// When the last partial evaluation ended.
    pub last_end: Option<Instant>,
    /// Events fetched so far.
    pub fetched: u64,
    /// Events fetched when the last partial evaluation started; `None`
    /// before the first.
    pub fetched_at_last: Option<u64>,
}

/// U3:BR1.1: whether a partial evaluation starts at `now`: after the first
/// page, none running, the first at once, the next ones `interval` after
/// the last ended and only when more events were fetched since it started.
pub fn should_start_partial(gate: &PartialGate, now: Instant, interval: Duration) -> bool {
    if !gate.fetch_started || !gate.first_page || gate.running {
        return false;
    }
    let Some(fetched_at_last) = gate.fetched_at_last else {
        return true;
    };
    gate.fetched > fetched_at_last && gate.last_end.is_none_or(|ended| now >= ended + interval)
}

/// When to look at the gate again when only time stands in the way of the
/// next partial evaluation; `None` when a new page or the end of the
/// running one is what it waits for.
pub fn next_partial_check(gate: &PartialGate, interval: Duration) -> Option<Instant> {
    let waits_for_time = gate.fetch_started
        && gate.first_page
        && !gate.running
        && gate
            .fetched_at_last
            .is_some_and(|fetched_at_last| gate.fetched > fetched_at_last);
    if !waits_for_time {
        return None;
    }
    gate.last_end.map(|ended| ended + interval)
}

/// What the sink says when the fetch ended (U3:BR3.1, BR3.2,
/// [`QuerySink::on_fetch_ended`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchEnd {
    /// Evaluate into the final result.
    Evaluate,
    /// [Cancel] stopped the fetch: evaluate what was fetched into the
    /// cancelled result, stopped only by the second [Cancel] (abandon).
    EvaluateCancelled,
    /// Do not evaluate (another run, the second [Cancel], a failure).
    Skip,
}

/// What a run does after its fetch (U3:BR3.1, BR3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterFetch {
    /// Evaluate into the final result.
    Evaluate,
    /// Evaluate into the cancelled result.
    EvaluateCancelled,
    /// End without evaluating: Aborted (closing), Failed or Cancelled.
    NotEvaluated(JobStatus),
}

/// U3:BR3.1, BR3.2 (261017-insights-query U3:BR2.5): closing ends the run
/// as if it never started, a failed log group evaluates nothing, then the
/// sink's word holds; a fetch that [Cancel] stopped is never evaluated into
/// a complete result.
pub fn after_fetch(summary: &QueryRunSummary, end: FetchEnd, closing: bool) -> AfterFetch {
    if summary.closed || closing {
        return AfterFetch::NotEvaluated(JobStatus::Aborted);
    }
    if summary.failed() {
        return AfterFetch::NotEvaluated(JobStatus::Failed);
    }
    match end {
        FetchEnd::Skip => AfterFetch::NotEvaluated(JobStatus::Cancelled),
        FetchEnd::EvaluateCancelled => AfterFetch::EvaluateCancelled,
        FetchEnd::Evaluate if summary.stopped => AfterFetch::EvaluateCancelled,
        FetchEnd::Evaluate => AfterFetch::Evaluate,
    }
}

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
    /// The least time between partial evaluations (261025-insights-stats
    /// U3:BR1.1, [`PARTIAL_EVALUATION_INTERVAL`] in the app); `None` shows
    /// no partial result (the `fetch_check` comparison, NFR1).
    pub partial_interval: Option<Duration>,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOutcome {
    /// Evaluated to its end; `rows` events matched.
    Evaluated {
        /// Number of result rows.
        rows: usize,
    },
    /// [Cancel] stopped the fetch and what was fetched was evaluated to its
    /// end into the cancelled result (261025-insights-stats U3:BR3.1).
    EvaluatedAfterCancel {
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
    let summary = fetch_with_partials(gateway, run, store, epoch, retrier, sink).await;
    let end = sink.on_fetch_ended(run.run_id, &summary);
    let signal = retrier.stop_signal();
    let after = after_fetch(&summary, end, signal.reason() == Some(StopReason::Close));
    if let AfterFetch::NotEvaluated(status @ (JobStatus::Aborted | JobStatus::Failed)) = after {
        return QueryOutcome::NotEvaluated(status);
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
    match after {
        AfterFetch::NotEvaluated(status) => QueryOutcome::NotEvaluated(status),
        AfterFetch::Evaluate => {
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
        AfterFetch::EvaluateCancelled => {
            // U3:BR3.1, BR3.2: the first [Cancel] does not stop it; the
            // second (abandon) and a closing app do.
            let abandoned = || signal.is_abandoned() || signal.reason() == Some(StopReason::Close);
            let held = Held {
                run,
                epoch,
                total,
                store,
            };
            match evaluate_until(held, &abandoned, sink, QueryStore::evaluate_step).await {
                QueryOutcome::Evaluated { rows } => QueryOutcome::EvaluatedAfterCancel { rows },
                other => other,
            }
        }
    }
}

/// The key columns of the aggregate rows `plan` makes (U3:BR4.2).
fn key_count(plan: &QueryPlan) -> usize {
    plan.stats().map_or(0, |stats| stats.group_keys.len())
}

/// How far the run's fetch went, written by the main flow as the fetch
/// tells it and read to decide when a partial evaluation starts (U3:BR1.1).
#[derive(Debug, Default)]
struct FetchCounters {
    fetch_started: AtomicBool,
    first_page: AtomicBool,
    fetched: AtomicU64,
    group_number: AtomicU64,
    group_count: AtomicU64,
}

impl FetchCounters {
    /// Brings `gate` up to what the fetch told so far.
    fn read_into(&self, gate: &mut PartialGate) {
        gate.fetch_started = self.fetch_started.load(Ordering::Acquire);
        gate.first_page = self.first_page.load(Ordering::Acquire);
        gate.fetched = self.fetched.load(Ordering::Acquire);
    }

    /// U3:BR2.6: the log group under fetch and the chosen count, for a
    /// partial result holding `length` events.
    fn coverage(&self, length: usize) -> PartialCoverage {
        let number =
            |value: &AtomicU64| u32::try_from(value.load(Ordering::Acquire)).unwrap_or(u32::MAX);
        PartialCoverage {
            group_number: number(&self.group_number),
            group_count: number(&self.group_count),
            included_event_count: u64::try_from(length).unwrap_or(u64::MAX),
        }
    }
}

/// The sink as the fetch uses it while partial results come
/// (U3:BR1.1): every notice goes to the one sink behind a lock that the
/// main flow alone takes, between its awaits; the pages are also counted.
struct SharedSink<'a, 's, S> {
    inner: &'a Mutex<&'s mut S>,
    counters: &'a FetchCounters,
    page_came: &'a Notify,
}

impl<S> SharedSink<'_, '_, S> {
    fn with<T>(&self, notice: impl FnOnce(&mut S) -> T) -> T {
        let mut sink = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        notice(&mut sink)
    }
}

impl<S: QuerySink> FetchSink for SharedSink<'_, '_, S> {
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64) {
        self.with(|sink| sink.on_started(job, timeline_version));
    }
    fn on_listing_progress(&mut self, job_id: u64, progress: ListingProgress) {
        self.with(|sink| sink.on_listing_progress(job_id, progress));
    }
    fn on_planned(&mut self, job: &FetchJob) {
        self.with(|sink| sink.on_planned(job));
    }
    fn on_batch(&mut self, job_id: u64, progress: BatchProgress) {
        self.counters
            .fetched
            .fetch_add(progress.added, Ordering::AcqRel);
        self.counters.first_page.store(true, Ordering::Release);
        self.page_came.notify_one();
        self.with(|sink| sink.on_batch(job_id, progress));
    }
    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        self.with(|sink| sink.on_stream_finished(job, outcome));
    }
    fn on_saving(&mut self, job_id: u64) {
        self.with(|sink| sink.on_saving(job_id));
    }
    fn on_summary(&mut self, summary: &FetchSummary) {
        self.with(|sink| sink.on_summary(summary));
    }
    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64) {
        self.with(|sink| sink.on_finished(job, timeline_version));
    }
}

impl<S: QuerySink> QuerySink for SharedSink<'_, '_, S> {
    fn on_reading_cache(&mut self, run_id: u64) {
        self.with(|sink| sink.on_reading_cache(run_id));
    }
    fn on_group_started(&mut self, run_id: u64, index: usize, count: usize) {
        let counters = self.counters;
        counters.fetch_started.store(true, Ordering::Release);
        let wide = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
        counters
            .group_number
            .store(wide(index.saturating_add(1)), Ordering::Release);
        counters.group_count.store(wide(count), Ordering::Release);
        self.with(|sink| sink.on_group_started(run_id, index, count));
    }
    fn on_range_fetched(&mut self, run_id: u64, log_group_name: &str, job: &FetchJob) {
        self.with(|sink| sink.on_range_fetched(run_id, log_group_name, job));
    }
    fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> FetchEnd {
        self.with(|sink| sink.on_fetch_ended(run_id, summary))
    }
    fn on_partial_result(&mut self, run_id: u64, result: QueryResult, coverage: PartialCoverage) {
        self.with(|sink| sink.on_partial_result(run_id, result, coverage));
    }
    fn on_field_names(&mut self, run_id: u64, names: Vec<String>) {
        self.with(|sink| sink.on_field_names(run_id, names));
    }
    fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool {
        self.with(|sink| sink.on_evaluation_progress(run_id, evaluated, total))
    }
    fn on_evaluated(&mut self, result: QueryResult) {
        self.with(|sink| sink.on_evaluated(result));
    }
    fn on_evaluation_stopped(&mut self, run_id: u64) {
        self.with(|sink| sink.on_evaluation_stopped(run_id));
    }
    fn on_evaluation_failed(&mut self, run_id: u64) {
        self.with(|sink| sink.on_evaluation_failed(run_id));
    }
}

/// What a partial evaluation sends back: its result with what it holds, or
/// `None` when it was stopped, the view changed under it or it failed.
type PartialMessage = Option<(QueryResult, PartialCoverage)>;

/// U3:BR1.1, BR1.4, BR2.1: fetches what is missing (as
/// [`fetch_missing`]) while partial results are made and handed to the
/// sink. The main flow alone holds the sink; it waits on the fetch, the
/// partial results, a new page and the interval with `select!`. When the
/// fetch ends a running partial evaluation is told to stop and waited for;
/// what it found is dropped.
async fn fetch_with_partials<G, S>(
    gateway: &G,
    run: &QueryRun,
    store: &Arc<QueryStore>,
    epoch: u64,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> QueryRunSummary
where
    G: CloudWatchLogsGateway,
    S: QuerySink,
{
    let Some(interval) = run.partial_interval else {
        return fetch_missing(gateway, run, store, retrier, sink).await;
    };
    let counters = FetchCounters::default();
    let page_came = Notify::new();
    let shared = Mutex::new(sink);
    let mut fetch_sink = SharedSink {
        inner: &shared,
        counters: &counters,
        page_came: &page_came,
    };
    let (sender, mut receiver) = mpsc::unbounded_channel::<PartialMessage>();
    let mut gate = PartialGate::default();
    let mut stop: Option<Arc<AtomicBool>> = None;
    let fetch = fetch_missing(gateway, run, store, retrier, &mut fetch_sink);
    tokio::pin!(fetch);
    let summary = loop {
        counters.read_into(&mut gate);
        if should_start_partial(&gate, Instant::now(), interval) {
            gate.running = true;
            gate.fetched_at_last = Some(gate.fetched);
            stop = Some(spawn_partial(run, store, epoch, &counters, sender.clone()));
        }
        let wake = next_partial_check(&gate, interval);
        tokio::select! {
            biased;
            summary = &mut fetch => break summary,
            Some(message) = receiver.recv() => {
                gate.running = false;
                gate.last_end = Some(Instant::now());
                if let Some((result, coverage)) = message {
                    let mut sink = shared.lock().unwrap_or_else(PoisonError::into_inner);
                    sink.on_partial_result(run.run_id, result, coverage);
                }
            }
            () = page_came.notified() => {}
            () = sleep_until(wake) => {}
        }
    };
    if gate.running {
        if let Some(stop) = &stop {
            stop.store(true, Ordering::Release);
        }
        // BR1.4: the final view is built only once it ended.
        let _dropped = receiver.recv().await;
    }
    summary
}

/// Completes at `wake`; never without one.
async fn sleep_until(wake: Option<Instant>) {
    match wake {
        Some(wake) => tokio::time::sleep_until(tokio::time::Instant::from_std(wake)).await,
        None => std::future::pending::<()>().await,
    }
}

/// Starts one partial evaluation of the run on a blocking thread; it sends
/// one message when it ends, whatever happens to it. Returns its stop flag.
fn spawn_partial(
    run: &QueryRun,
    store: &Arc<QueryStore>,
    epoch: u64,
    counters: &FetchCounters,
    sender: mpsc::UnboundedSender<PartialMessage>,
) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let task = PartialTask {
        store: Arc::clone(store),
        plan: Arc::clone(&run.plan),
        epoch,
        range: run.range,
        run_id: run.run_id,
        stop: Arc::clone(&stop),
    };
    let under_fetch = counters.coverage(0);
    tokio::task::spawn_blocking(move || {
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task.evaluate()))
            .unwrap_or_else(|_| {
                eprintln!("local-sights: query: a partial evaluation ended abnormally");
                None
            });
        let message = found.map(|(result, length)| {
            let coverage = PartialCoverage {
                included_event_count: u64::try_from(length).unwrap_or(u64::MAX),
                ..under_fetch
            };
            (result, coverage)
        });
        // The main flow may have ended: nothing waits for it then.
        let _unheard = sender.send(message);
    });
    stop
}

/// One partial evaluation (U3:BR1.3, BR1.4, BR2.1).
struct PartialTask {
    store: Arc<QueryStore>,
    plan: Arc<QueryPlan>,
    epoch: u64,
    range: TimeRange,
    run_id: u64,
    stop: Arc<AtomicBool>,
}

impl PartialTask {
    /// Builds the partial view and evaluates it to its end with the final
    /// result's evaluator, one store lock per step; returns the result with
    /// the view's length. `None` when told to stop or when the view changed
    /// (a range settled, a new run: BR1.4).
    fn evaluate(&self) -> Option<(QueryResult, usize)> {
        let (store, plan) = (&self.store, self.plan.as_ref());
        let view = store.build_partial_view(
            self.epoch,
            self.range.start_instant(),
            self.range.end_instant(),
        )?;
        let mut evaluation = Evaluation::new(plan, view.length);
        while !evaluation.is_done() {
            if self.stop.load(Ordering::Acquire) {
                return None;
            }
            if !store.evaluate_view_step(plan, &mut evaluation, view.stamp) {
                return None;
            }
        }
        let result = match evaluation.into_output()? {
            EvalOutput::Events(output) => {
                let rows = store.copy_view_rows(view.stamp, &output.rows)?;
                QueryResult::from_output(self.run_id, output, rows)
            }
            EvalOutput::Aggregates(output) => {
                QueryResult::from_aggregates(self.run_id, output).with_key_count(key_count(plan))
            }
        };
        Some((result, view.length))
    }
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
    // 261025-insights-stats U3:BR1.2: the holder is registered, so partial
    // views see its events; the fetch itself only locks the holder.
    let holder = Arc::new(Mutex::new(EventTimeline::new()));
    let inflight = store.register_inflight(key, range, Arc::clone(&holder));
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
    let completed = job.status == JobStatus::Completed;
    let (store, started) = (Arc::clone(store), run.run_started_at_ms);
    // U3:BR1.2: taken out, settled and unregistered under one store lock.
    let settled = on_blocking(move || store.settle_inflight(inflight, completed, started)).await;
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
/// result rows (BR3.2); [Cancel] stops it between steps.
async fn evaluate_held<S: QuerySink>(
    run: &QueryRun,
    epoch: u64,
    total: usize,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
    step: EvaluateStep,
) -> QueryOutcome {
    let held = Held {
        run,
        epoch,
        total,
        store,
    };
    evaluate_until(held, &|| retrier.is_stopped(), sink, step).await
}

/// The view a final or cancelled evaluation goes over.
struct Held<'a> {
    run: &'a QueryRun,
    epoch: u64,
    total: usize,
    store: &'a Arc<QueryStore>,
}

/// As [`evaluate_held`], stopped between steps when `stopped` says so
/// (261025-insights-stats U3:BR3.1: after [Cancel] only the abandon stops
/// it).
async fn evaluate_until<S: QuerySink>(
    held: Held<'_>,
    stopped: &(dyn Fn() -> bool + Sync),
    sink: &mut S,
    step: EvaluateStep,
) -> QueryOutcome {
    let Held {
        run,
        epoch,
        total,
        store,
    } = held;
    let mut evaluation = Evaluation::new(&run.plan, total);
    while !evaluation.is_done() {
        if stopped() {
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
            let result = QueryResult::from_aggregates(run.run_id, output)
                .with_key_count(key_count(&run.plan));
            sink.on_evaluated(result);
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
        // 261025-insights-stats U3.
        partials: Vec<(QueryResult, PartialCoverage)>,
        /// Lets the gated fake serve one more page per partial result.
        pages: Option<Arc<tokio::sync::Semaphore>>,
        cancel_after_failed_stream: bool,
        abandon_after_cancel: bool,
        abandon_at_first_evaluation_step: bool,
        abandoned: bool,
    }

    impl Recorder {
        fn cancel(&mut self) {
            self.cancelled = true;
            if let Some(handle) = &self.handle {
                handle.stop(StopReason::Cancel);
            }
            if self.abandon_after_cancel {
                self.abandon();
            }
        }

        /// The second [Cancel] (U3:BR3.2).
        fn abandon(&mut self) {
            self.abandoned = true;
            if let Some(handle) = &self.handle {
                handle.abandon();
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
        fn on_stream_finished(&mut self, _job: &FetchJob, outcome: &StreamFetchOutcome) {
            if self.cancel_after_first_stream && !self.cancelled {
                self.cancel();
            }
            if self.cancel_after_failed_stream && outcome.failure.is_some() && !self.cancelled {
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
        /// As the session answers (U3:BR3.1, BR3.2).
        fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> FetchEnd {
            self.heard.push(format!("fetch ended {run_id}"));
            self.summary = Some(summary.clone());
            if self.abandoned {
                FetchEnd::Skip
            } else if self.cancelled {
                FetchEnd::EvaluateCancelled
            } else {
                FetchEnd::Evaluate
            }
        }
        fn on_partial_result(
            &mut self,
            run_id: u64,
            result: QueryResult,
            coverage: PartialCoverage,
        ) {
            self.heard.push(format!("partial {run_id}"));
            self.partials.push((result, coverage));
            if let Some(pages) = &self.pages {
                pages.add_permits(1);
            }
        }
        fn on_evaluation_progress(&mut self, _run_id: u64, evaluated: usize, total: usize) -> bool {
            self.progress.push((evaluated, total));
            if self.cancel_after_first_chunk {
                self.cancel();
            }
            if self.abandon_at_first_evaluation_step && !self.abandoned {
                self.abandon();
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
            // As in the app (261025-insights-stats U3:BR1.1).
            partial_interval: Some(PARTIAL_EVALUATION_INTERVAL),
        }
    }

    /// Runs `run` on `gateway`; `prepare` sets the recorder up with the
    /// run's stop handle.
    async fn query_with<G: CloudWatchLogsGateway>(
        gateway: &G,
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
        // BR2.4, U1:BR2.7; since 261025-insights-stats U3:BR3.1 (FR5.1,
        // BR5.1) what was fetched is evaluated into the cancelled result.
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
        let QueryOutcome::EvaluatedAfterCancel { rows: count } = outcome else {
            panic!("not evaluated after the cancel: {outcome:?}");
        };
        assert_eq!(recorder.results.len(), 1, "evaluated once");
        assert_eq!(recorder.results[0].row_count(), count);
        assert!(count <= 2, "only ORDERS was fetched");
        assert!(
            rows(&recorder.results[0])
                .iter()
                .all(|(_, group, _)| group == ORDERS)
        );
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

    // ---- 261025-insights-stats U3:BR1.1, BR3.1, BR3.2 (test first) -----

    use std::time::{Duration, Instant};

    const SECOND: Duration = Duration::from_secs(1);

    /// A fetch from AWS whose first page came, `fetched` events so far.
    fn gate(fetched: u64) -> PartialGate {
        PartialGate {
            fetch_started: true,
            first_page: true,
            running: false,
            last_end: None,
            fetched,
            fetched_at_last: None,
        }
    }

    #[test]
    fn no_partial_evaluation_starts_before_the_first_page() {
        let now = Instant::now();
        let waiting = PartialGate {
            first_page: false,
            ..gate(0)
        };
        assert!(!should_start_partial(&waiting, now, SECOND));
        assert_eq!(next_partial_check(&waiting, SECOND), None);
    }

    #[test]
    fn the_first_partial_evaluation_starts_at_once_after_the_first_page() {
        // RA review R-06: no second of waiting first.
        let now = Instant::now();
        assert!(should_start_partial(&gate(1), now, SECOND));
        assert!(
            should_start_partial(&gate(0), now, SECOND),
            "even an empty page"
        );
    }

    #[test]
    fn no_partial_evaluation_starts_while_one_runs() {
        let now = Instant::now();
        let running = PartialGate {
            running: true,
            ..gate(5)
        };
        assert!(!should_start_partial(&running, now, SECOND));
        assert_eq!(next_partial_check(&running, SECOND), None);
    }

    #[test]
    fn the_next_partial_evaluation_waits_a_second_after_the_last_one_ended() {
        let ended = Instant::now();
        let after = PartialGate {
            last_end: Some(ended),
            fetched_at_last: Some(5),
            ..gate(9)
        };
        assert!(!should_start_partial(&after, ended, SECOND));
        assert!(!should_start_partial(
            &after,
            ended + Duration::from_millis(999),
            SECOND
        ));
        assert!(
            should_start_partial(&after, ended + SECOND, SECOND),
            "the boundary"
        );
        assert_eq!(next_partial_check(&after, SECOND), Some(ended + SECOND));
        // An evaluation longer than a second: the next one starts as soon
        // as it ends.
        assert!(should_start_partial(&after, ended + 3 * SECOND, SECOND));
    }

    #[test]
    fn no_partial_evaluation_starts_until_more_events_were_fetched() {
        let ended = Instant::now();
        let same = PartialGate {
            last_end: Some(ended),
            fetched_at_last: Some(9),
            ..gate(9)
        };
        assert!(!should_start_partial(&same, ended + 5 * SECOND, SECOND));
        assert_eq!(
            next_partial_check(&same, SECOND),
            None,
            "a new page wakes it"
        );
    }

    #[test]
    fn no_partial_evaluation_starts_without_a_fetch_from_aws() {
        // FR4.9: nothing missing, nothing fetched.
        let idle = PartialGate {
            fetch_started: false,
            ..gate(3)
        };
        assert!(!should_start_partial(&idle, Instant::now(), SECOND));
        assert_eq!(next_partial_check(&idle, SECOND), None);
        assert_eq!(PARTIAL_EVALUATION_INTERVAL, SECOND);
    }

    fn summary_of(groups: &[GroupFetchStatus], stopped: bool, closed: bool) -> QueryRunSummary {
        QueryRunSummary {
            groups: groups
                .iter()
                .map(|status| GroupFetchOutcome {
                    status: *status,
                    ..GroupFetchOutcome::new(ORDERS)
                })
                .collect(),
            partial: groups.contains(&GroupFetchStatus::CompletedWithFailures),
            from_aws: true,
            stopped,
            closed,
        }
    }

    #[test]
    fn the_end_of_the_fetch_decides_whether_and_how_to_evaluate() {
        // BR3.1, BR3.2: a [Cancel] evaluates as a cancelled result.
        let done = summary_of(&[GroupFetchStatus::Completed], false, false);
        let stopped = summary_of(&[GroupFetchStatus::Cancelled], true, false);
        let some_failed = summary_of(&[GroupFetchStatus::CompletedWithFailures], true, false);
        assert_eq!(
            after_fetch(&done, FetchEnd::Evaluate, false),
            AfterFetch::Evaluate
        );
        assert_eq!(
            after_fetch(&stopped, FetchEnd::EvaluateCancelled, false),
            AfterFetch::EvaluateCancelled
        );
        assert_eq!(
            after_fetch(&stopped, FetchEnd::Evaluate, false),
            AfterFetch::EvaluateCancelled,
            "a stopped fetch is never a complete result"
        );
        assert_eq!(
            after_fetch(&some_failed, FetchEnd::EvaluateCancelled, false),
            AfterFetch::EvaluateCancelled,
            "FR5.4: cancelled and partly failed"
        );
        assert_eq!(
            after_fetch(&stopped, FetchEnd::Skip, false),
            AfterFetch::NotEvaluated(JobStatus::Cancelled),
            "the second [Cancel] came first"
        );
    }

    // ---- 261025-insights-stats U3: the runner's tasks and channel ------

    /// The fake whose `GetLogEvents` pages are served one at a time: each
    /// waits for a permit, which the recorder gives at every partial result
    /// (at most a few seconds, so a test never hangs; no time is checked).
    struct GatedLogs {
        logs: FakeLogs,
        pages: Arc<tokio::sync::Semaphore>,
    }

    impl GatedLogs {
        fn new(logs: FakeLogs) -> Self {
            Self {
                logs,
                pages: Arc::new(tokio::sync::Semaphore::new(1)),
            }
        }
    }

    impl CloudWatchLogsGateway for GatedLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            let permit = tokio::time::timeout(Duration::from_secs(5), self.pages.acquire()).await;
            if let Ok(Ok(permit)) = permit {
                permit.forget();
            }
            self.logs.get_log_events(request).await
        }

        async fn describe_log_groups(
            &self,
            request: DescribeLogGroupsRequest,
        ) -> Result<DescribeLogGroupsPage, ApiFailure> {
            self.logs.describe_log_groups(request).await
        }

        async fn describe_log_streams(
            &self,
            request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            self.logs.describe_log_streams(request).await
        }
    }

    /// `streams` streams of `group`, each with `per_stream` events a second
    /// apart from 10:00:00, the same times on every stream.
    fn spread(group: &str, streams: usize, per_stream: i64) -> Vec<(String, String, i64, String)> {
        (0..streams)
            .flat_map(|stream| {
                (0..per_stream).map(move |second| {
                    (
                        group.to_string(),
                        format!("s{stream}"),
                        at(10, 0, 0) + second * 1_000,
                        format!(r#"{{"stream":{stream},"n":{second}}}"#),
                    )
                })
            })
            .collect()
    }

    /// Runs `query` over `groups` on gated pages of `events` with partial
    /// results as soon as the last one ended.
    async fn gated_run(
        events: Vec<(String, String, i64, String)>,
        query: &str,
        groups: &[&str],
    ) -> (QueryOutcome, Recorder) {
        let gateway = GatedLogs::new(FakeLogs {
            events,
            ..FakeLogs::default()
        });
        let store = Arc::new(QueryStore::new());
        let mut period = run(query, groups, at(10, 0, 0), at(10, 1, 0));
        period.partial_interval = Some(Duration::ZERO);
        let pages = Arc::clone(&gateway.pages);
        query_with(&gateway, &store, &period, |recorder| {
            recorder.pages = Some(pages);
        })
        .await
    }

    /// `(time, @log, stream, sequence)` of the event rows of `result`.
    fn identities(result: &QueryResult) -> Vec<(i64, String, String, u64)> {
        result
            .window(result.run_id, 0, usize::MAX)
            .iter()
            .filter_map(|row| {
                let event = row.event()?;
                Some((
                    event.timestamp,
                    row.log_group_name()?.to_string(),
                    event.log_stream_name.clone(),
                    event.sequence,
                ))
            })
            .collect()
    }

    #[tokio::test]
    async fn partial_results_come_while_fetching_and_the_final_result_last() {
        // FR4.1, FR4.3, BR1.1, BR2.1: the first at once after the first
        // page, then as pages come.
        let (outcome, recorder) = gated_run(
            spread(ORDERS, 4, 3),
            "filter @message like 'stream'",
            &[ORDERS],
        )
        .await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 12 });
        assert!(!recorder.partials.is_empty(), "{:?}", recorder.heard);
        let ended = recorder
            .heard
            .iter()
            .position(|heard| heard == "fetch ended 7")
            .unwrap();
        assert!(
            recorder.heard[..ended]
                .iter()
                .filter(|heard| heard.starts_with("partial"))
                .count()
                == recorder.partials.len(),
            "every partial result comes before the fetch ends"
        );
        assert_eq!(recorder.heard.last().map(String::as_str), Some("evaluated"));
        let counts: Vec<u64> = recorder
            .partials
            .iter()
            .map(|(_, coverage)| coverage.included_event_count)
            .collect();
        assert!(
            counts.windows(2).all(|pair| pair[0] <= pair[1]),
            "{counts:?}"
        );
        assert!(counts.iter().all(|count| *count <= 12));
        for (result, coverage) in &recorder.partials {
            assert_eq!((coverage.group_number, coverage.group_count), (1, 1));
            assert_eq!(result.run_id, 7);
            assert_eq!(
                result.row_count(),
                usize::try_from(coverage.included_event_count).unwrap()
            );
        }
    }

    #[tokio::test]
    async fn a_partial_result_keeps_the_final_order_and_holds_no_event_twice() {
        // BR2.2 (FR4.2 as read), FR4.8: its rows are in the final result,
        // in the same order, each once.
        let (_, recorder) = gated_run(
            spread(ORDERS, 3, 4),
            "fields @timestamp, @logStream | sort @logStream desc, @timestamp asc",
            &[ORDERS],
        )
        .await;
        let last = identities(&recorder.results[0]);
        assert_eq!(last.len(), 12);
        assert!(!recorder.partials.is_empty());
        for (result, _) in &recorder.partials {
            let early = identities(result);
            let mut unique = early.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), early.len(), "no event twice: {early:?}");
            let mut cursor = last.iter();
            assert!(
                early.iter().all(|row| cursor.any(|other| other == row)),
                "in the final order: {early:?}"
            );
        }
    }

    #[tokio::test]
    async fn partial_results_across_log_groups_count_every_event_once() {
        // FR4.8, BR2.6: a stats count while one log group is settled and
        // the next under fetch equals the events in the view.
        let mut events = spread(ORDERS, 2, 3);
        events.extend(spread(PAYMENTS, 2, 3));
        let (outcome, recorder) =
            gated_run(events, "stats count(*) as n", &[ORDERS, PAYMENTS]).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 1 });
        assert_eq!(
            recorder.results[0].row_at(7, 0).and_then(ResultRow::cells),
            Some(&[CellValue::Integer(12)][..])
        );
        assert!(!recorder.partials.is_empty());
        for (result, coverage) in &recorder.partials {
            let counted = result
                .row_at(7, 0)
                .and_then(ResultRow::cells)
                .map(<[CellValue]>::to_vec);
            assert_eq!(
                counted,
                Some(vec![CellValue::Integer(
                    i64::try_from(coverage.included_event_count).unwrap()
                )]),
                "{coverage:?}"
            );
            assert_eq!(coverage.group_count, 2);
            assert!((1..=2).contains(&coverage.group_number));
        }
    }

    #[tokio::test]
    async fn stats_and_the_commands_after_it_make_partial_results_too() {
        // BR2.1 with U2's after-stats commands and cut.
        let (outcome, recorder) = gated_run(
            spread(ORDERS, 3, 2),
            "stats count(*) as n by @logStream | sort @logStream desc | limit 2",
            &[ORDERS],
        )
        .await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
        assert!(!recorder.partials.is_empty());
        for (result, _) in recorder.partials.iter().chain(std::iter::empty()) {
            assert_eq!(result.shape(), crate::query::plan::ResultShape::Aggregates);
            assert_eq!(result.key_count(), 1);
            assert!(result.row_count() <= 2);
        }
        assert_eq!(recorder.results[0].key_count(), 1, "the final one too");
    }

    #[tokio::test]
    async fn a_period_fetched_already_makes_no_partial_result() {
        // FR4.9, BR1.1: no AWS call, no partial evaluation.
        let gateway = orders_morning();
        let store = Arc::new(QueryStore::new());
        let mut period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(11, 0, 0),
        );
        period.partial_interval = Some(Duration::ZERO);
        query_with(&gateway, &store, &period, |_| {}).await;
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 8 });
        assert!(recorder.partials.is_empty());
        assert!(!recorder.summary.unwrap().from_aws);
    }

    #[tokio::test]
    async fn without_partial_results_a_run_goes_as_before() {
        // NFR1: the comparison of fetch_check; BR5.1.
        let gateway = GatedLogs::new(FakeLogs {
            events: spread(ORDERS, 3, 2),
            ..FakeLogs::default()
        });
        gateway.pages.add_permits(100);
        let store = Arc::new(QueryStore::new());
        let mut period = run(
            "filter @message like 'n'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        period.partial_interval = None;
        let (outcome, recorder) = query_with(&gateway, &store, &period, |_| {}).await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 6 });
        assert!(recorder.partials.is_empty());
        assert_eq!(store.inflight_count(), 0, "every holder settled");
    }

    #[tokio::test]
    async fn a_cancel_with_failed_streams_gives_the_cancelled_result_and_the_failures() {
        // FR5.4, BR3.1, BR3.4.
        let mut logs = FakeLogs::with(&[
            (ORDERS, "a", at(10, 0, 10), "ERROR a"),
            (PAYMENTS, "s1", at(10, 0, 20), "ERROR p"),
        ]);
        logs.failing_streams
            .push((ORDERS.to_string(), "b".to_string()));
        let store = Arc::new(QueryStore::new());
        let both = run(
            "filter @message like 'ERROR'",
            &[ORDERS, PAYMENTS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&logs, &store, &both, |recorder| {
            recorder.cancel_after_failed_stream = true;
        })
        .await;
        assert!(
            matches!(outcome, QueryOutcome::EvaluatedAfterCancel { rows } if rows <= 1),
            "{outcome:?}"
        );
        let summary = recorder.summary.unwrap();
        assert!(summary.stopped);
        assert_eq!(
            summary
                .failed_streams()
                .iter()
                .map(|failed| failed.log_stream_name.as_str())
                .collect::<Vec<_>>(),
            ["b"]
        );
        assert!(store.covered_ranges(&key(ORDERS)).is_empty(), "FR5.3");
        assert_eq!(recorder.results.len(), 1);
    }

    /// 25,000 events of [`ORDERS`] on one stream, a millisecond apart.
    fn heavy() -> FakeLogs {
        FakeLogs {
            events: (0..25_000i64)
                .map(|i| {
                    (
                        ORDERS.to_string(),
                        "s".to_string(),
                        at(10, 0, 0) + i,
                        format!("ERROR {}", i % 97),
                    )
                })
                .collect(),
            ..FakeLogs::default()
        }
    }

    #[tokio::test]
    async fn a_second_cancel_stops_the_cancelled_evaluation_even_of_a_heavy_stats() {
        // BR3.2 (DD R-10 for U3): stopped at the next chunk, nothing told.
        let gateway = heavy();
        let store = Arc::new(QueryStore::new());
        let period = run(
            "stats count(*) as n by @message | sort n desc",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |recorder| {
            recorder.cancel_after_first_stream = true;
            recorder.abandon_at_first_evaluation_step = true;
        })
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluationStopped);
        assert_eq!(recorder.progress, vec![(10_000, 25_000)]);
        assert!(recorder.results.is_empty());
        assert_eq!(recorder.heard.last().map(String::as_str), Some("stopped 7"));
    }

    #[tokio::test]
    async fn a_second_cancel_before_the_fetch_ended_starts_no_evaluation() {
        // BR3.2: nothing is evaluated after the fetch.
        let gateway = heavy();
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |recorder| {
            recorder.cancel_after_first_stream = true;
            recorder.abandon_after_cancel = true;
        })
        .await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Cancelled));
        assert!(recorder.progress.is_empty() && recorder.results.is_empty());
    }

    #[tokio::test]
    async fn a_cancelled_evaluation_is_not_stopped_by_the_first_cancel() {
        // BR3.1: the stop signal says Cancel all along.
        let gateway = heavy();
        let store = Arc::new(QueryStore::new());
        let period = run(
            "filter @message like 'ERROR'",
            &[ORDERS],
            at(10, 0, 0),
            at(10, 1, 0),
        );
        let (outcome, recorder) = query_with(&gateway, &store, &period, |recorder| {
            recorder.cancel_after_first_stream = true;
        })
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluatedAfterCancel { rows: 10_000 });
        assert_eq!(recorder.progress.len(), 3, "every chunk");
        assert_eq!(recorder.results[0].matched_count(), 25_000);
        assert!(recorder.summary.unwrap().stopped);
    }

    #[test]
    fn a_partial_evaluation_drops_its_result_when_told_to_stop_or_when_the_view_changed() {
        // BR1.4.
        let store = Arc::new(QueryStore::new());
        let group = key(ORDERS);
        let epoch = store.begin_run(vec![group.clone()]);
        let range = CoveredRange {
            start_ms: at(10, 0, 0),
            end_ms: at(10, 1, 0),
        };
        let mut timeline = EventTimeline::new();
        timeline.append(vec![LogEvent {
            timestamp: at(10, 0, 1),
            ingestion_time: None,
            message: "ERROR".to_string(),
            log_stream_name: "s".to_string(),
            sequence: 0,
        }]);
        let id = store.register_inflight(&group, range, Arc::new(Mutex::new(timeline)));
        let task = PartialTask {
            store: Arc::clone(&store),
            plan: Arc::new(parse_query("filter @message like 'ERROR'").unwrap()),
            epoch,
            range: TimeRange::from_instants(range.start_ms, range.end_ms).unwrap(),
            run_id: 7,
            stop: Arc::new(AtomicBool::new(false)),
        };
        let (result, length) = task.evaluate().unwrap();
        assert_eq!(
            (result.row_count(), length),
            (1, 1),
            "the event under fetch"
        );
        task.stop.store(true, Ordering::Release);
        assert!(task.evaluate().is_none(), "told to stop");
        task.stop.store(false, Ordering::Release);
        store.settle_inflight(id, true, a_day_later());
        store.begin_run(vec![group]);
        assert!(task.evaluate().is_none(), "another run");
    }

    #[test]
    fn a_failed_or_closing_fetch_is_not_evaluated_whatever_the_sink_says() {
        let failed = summary_of(
            &[GroupFetchStatus::Failed, GroupFetchStatus::Cancelled],
            true,
            false,
        );
        let closed = summary_of(&[GroupFetchStatus::Cancelled], true, true);
        let done = summary_of(&[GroupFetchStatus::Completed], false, false);
        assert_eq!(
            after_fetch(&failed, FetchEnd::EvaluateCancelled, false),
            AfterFetch::NotEvaluated(JobStatus::Failed)
        );
        assert_eq!(
            after_fetch(&closed, FetchEnd::Evaluate, false),
            AfterFetch::NotEvaluated(JobStatus::Aborted)
        );
        assert_eq!(
            after_fetch(&done, FetchEnd::Evaluate, true),
            AfterFetch::NotEvaluated(JobStatus::Aborted),
            "closing was asked meanwhile"
        );
    }
}
