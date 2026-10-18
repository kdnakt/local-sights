//! QueryRunner: one run of a read query, fetch then evaluate
//! (261017-insights-query U1, functional-spec UC1, UC3-UC5).
//!
//! The fetch is the existing flow, [`run_fetch_with_cache`], with the
//! [`QueryStore`] as its timeline, so the stream selection, the eight streams
//! read side by side, the call pacing, the retries, the give-up rule and the
//! disk cache stay as they are (U1:BR2.2). Only the end of the fetch is held
//! back: the runner decides from it whether to evaluate (UC1 10):
//!
//! - Completed or CompletedWithFailures: evaluate, unless [Cancel] was
//!   pressed meanwhile; the sink's answer to [`QuerySink::on_fetch_ended`]
//!   says so, so a [Cancel] that arrived after the last stream, when the
//!   fetch still ends as without a stop (U5 FR5.9), does not evaluate either
//!   (functional-design review R-01, U1:BR2.7);
//! - Failed, Cancelled or Aborted: nothing is evaluated (U1:BR2.6, BR2.7).
//!
//! When the fetch ended with logs, the JSON field names of the first
//! held events are gathered for completion (U2:BR1.7, Should).
//!
//! The evaluation goes step by step, each step one chunk of
//! [`EVALUATION_CHUNK`](crate::query::evaluator::EVALUATION_CHUNK) rows (and
//! what follows it without a chunk) on a blocking thread under one lock of
//! the store, and between steps the stop signal, the sink's answer to the
//! progress and the store's epoch are checked (U1:BR5.8, U2:BR3.1). A step
//! that ends abnormally (its thread panics) is an evaluation failure, told
//! as such, never as a failed fetch (U2:BR10.2, U1 code review R-03). The Log
//! Management screen's timeline is never touched (U1:BR1.7).

use std::sync::Arc;

use crate::cache::LogCache;
use crate::coordinator::{DeferredFinishSink, FetchJob, FetchSink, run_fetch_with_cache};
use crate::fetch_status::JobStatus;
use crate::gateway::CloudWatchLogsGateway;
use crate::query::evaluator::{Evaluation, QueryResult};
use crate::query::plan::QueryPlan;
use crate::query::store::{HoldingKey, QueryStore};
use crate::request::ValidatedFetch;
use crate::retry::Retrier;

/// Receives one run's progress: the fetch's notices as any fetch sink, then
/// the end of the fetch, the field names, the evaluation's progress and its
/// end.
pub trait QuerySink: FetchSink + Send {
    /// The fetch of `run_id` ended with `job` (its `on_finished` is not
    /// sent). Returns whether the run may still be evaluated: `false` once
    /// [Cancel] was pressed or the run is no longer the current one (R-01).
    fn on_fetch_ended(&mut self, run_id: u64, job: &FetchJob) -> bool;
    /// The JSON field names of the logs `run_id` fetched (U2:BR1.7).
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

/// What one run needs (UC1 6-9).
#[derive(Debug, Clone)]
pub struct QueryRun {
    /// The run, as InsightsSession numbered it.
    pub run_id: u64,
    /// The read plan with its date texts already in milliseconds.
    pub plan: Arc<QueryPlan>,
    /// The range and the connection to fetch.
    pub fetch: ValidatedFetch,
    /// What the holdings are for.
    pub key: HoldingKey,
    /// The disk cache, `None` when disabled (U6:BR1.5).
    pub cache: Option<LogCache>,
    /// When the fetch started, epoch milliseconds (U6:BR3.1).
    pub fetch_started_at_ms: i64,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOutcome {
    /// Evaluated to its end; `rows` events matched.
    Evaluated {
        /// Number of matching events.
        rows: usize,
    },
    /// Not evaluated: the fetch ended with this status, or [Cancel] came.
    NotEvaluated(JobStatus),
    /// The evaluation was stopped between two chunks.
    EvaluationStopped,
    /// An evaluation step ended abnormally (its thread panicked); the sink
    /// heard [`QuerySink::on_evaluation_failed`] (U2:BR10.2).
    EvaluationFailed,
}

/// Runs `run`: discards the store and names its key (U1:BR2.1), fetches the
/// whole range into it (U1:BR2.2) and evaluates when the fetch allows it.
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
    store.begin_run(run.key.clone());
    let mut deferred = DeferredFinishSink::new(&mut *sink);
    let returned = run_fetch_with_cache(
        gateway,
        &run.fetch,
        store,
        retrier,
        run.cache.as_ref(),
        run.fetch_started_at_ms,
        &mut deferred,
    )
    .await;
    let job = deferred
        .take_finished()
        .map_or(returned, |(job, _version)| job);
    let fetched = matches!(
        job.status,
        JobStatus::Completed | JobStatus::CompletedWithFailures
    );
    let allowed = sink.on_fetch_ended(run.run_id, &job);
    if fetched {
        gather_field_names(run.run_id, store, sink).await;
    }
    if !fetched || !allowed {
        return QueryOutcome::NotEvaluated(job.status);
    }
    evaluate_held(run, store, retrier, sink, QueryStore::evaluate_step).await
}

/// U2:BR1.7: the JSON field names of the held logs, read on a blocking
/// thread. A read that ends abnormally only leaves completion without the
/// names (it is a Should, FR9.1).
async fn gather_field_names<S: QuerySink>(run_id: u64, store: &Arc<QueryStore>, sink: &mut S) {
    let epoch = store.epoch();
    let task_store = Arc::clone(store);
    match tokio::task::spawn_blocking(move || task_store.json_field_names(epoch)).await {
        Ok(Some(names)) => sink.on_field_names(run_id, names),
        Ok(None) => {}
        Err(_) => eprintln!("local-sights: query: reading the field names ended abnormally"),
    }
}

/// UC1 11-12: evaluates the holdings step by step with `step`.
async fn evaluate_held<S: QuerySink>(
    run: &QueryRun,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
    step: EvaluateStep,
) -> QueryOutcome {
    let epoch = store.epoch();
    let mut evaluation = Evaluation::new(&run.plan, store.len());
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
    let Some(output) = evaluation.into_output() else {
        return evaluation_failed(run.run_id, sink);
    };
    let count = output.rows.len();
    sink.on_evaluated(QueryResult::from_output(run.run_id, epoch, output));
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
    use std::sync::Mutex;

    use super::*;
    use crate::catalog::ProfileSelector;
    use crate::coordinator::{BatchProgress, TimelineStore};
    use crate::event::LogEvent;
    use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};
    use crate::fetcher::StreamFetchOutcome;
    use crate::gateway::{
        DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
        DescribeLogStreamsRequest, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
    };
    use crate::log_view::LogView;
    use crate::query::parser::parse_query;
    use crate::request::{FetchInput, validate_fetch_input};
    use crate::retry::{FixedJitter, RetryPolicy, StopHandle, StopReason, stop_pair};
    use crate::streams::LogStream;
    use crate::streams::planner::ListingProgress;

    /// 2024/01/02 03:04:05 UTC, the start of the fetched range.
    const START: i64 = 1_704_164_645_000;
    const GROUP: &str = "/aws/lambda/orders";

    /// Answers one page per stream with its events in the asked time, and
    /// denies the failing stream or the whole listing. Never reaches AWS.
    #[derive(Default)]
    struct FakeLogs {
        events: Vec<(String, i64, String)>,
        failing_stream: Option<&'static str>,
        failing_listing: bool,
    }

    impl FakeLogs {
        fn with(events: &[(&str, i64, &str)]) -> Self {
            Self {
                events: events
                    .iter()
                    .map(|(stream, at, message)| {
                        ((*stream).to_string(), *at, (*message).to_string())
                    })
                    .collect(),
                ..Self::default()
            }
        }
    }

    fn denied() -> ApiFailure {
        ApiFailure::new(FailureKind::AccessDenied, &SafeDetailFields::default())
    }

    impl CloudWatchLogsGateway for FakeLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            if self.failing_stream == Some(request.log_stream_name.as_str()) {
                return Err(denied());
            }
            Ok(GetLogEventsPage {
                events: self
                    .events
                    .iter()
                    .filter(|(stream, at, _)| {
                        *stream == request.log_stream_name
                            && *at >= request.start_time
                            && *at < request.end_time
                    })
                    .map(|(_, at, message)| GatewayEvent {
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
            _request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            if self.failing_listing {
                return Err(denied());
            }
            let mut names: Vec<&str> = self.events.iter().map(|(s, _, _)| s.as_str()).collect();
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

    /// Keeps what it hears, in order; can press [Cancel] at a chosen moment
    /// through the run's stop handle, as the session and the app would.
    #[derive(Default)]
    struct Recorder {
        heard: Vec<String>,
        handle: Option<StopHandle>,
        cancel_after_last_stream: bool,
        cancel_after_first_chunk: bool,
        cancelled: bool,
        ended: Option<JobStatus>,
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
        fn on_planned(&mut self, _job: &FetchJob) {
            self.heard.push("planned".to_string());
        }
        fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}
        fn on_stream_finished(&mut self, job: &FetchJob, _outcome: &StreamFetchOutcome) {
            let last = job.finished_stream_count == job.planned_stream_count;
            if last && self.cancel_after_last_stream {
                self.cancel();
            }
        }
        fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {
            self.heard.push("finished".to_string());
        }
    }

    impl QuerySink for Recorder {
        fn on_fetch_ended(&mut self, run_id: u64, job: &FetchJob) -> bool {
            self.heard.push(format!("fetch ended {run_id}"));
            self.ended = Some(job.status);
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

    fn run(query: &str) -> QueryRun {
        let validated = validate_fetch_input(&FetchInput {
            profile_name: "dev".to_string(),
            log_group_name: GROUP.to_string(),
            start_text: "2024/01/02 03:04:05".to_string(),
            end_text: "2024/01/02 03:05:05".to_string(),
        })
        .unwrap();
        let profile = ProfileSelector::Named("dev".to_string());
        QueryRun {
            run_id: 7,
            plan: Arc::new(parse_query(query).unwrap()),
            fetch: ValidatedFetch {
                request: validated
                    .request
                    .with_connection(profile.clone(), "ap-northeast-1".to_string()),
                range: validated.range,
            },
            key: HoldingKey {
                profile,
                region: "ap-northeast-1".to_string(),
                log_group_name: GROUP.to_string(),
            },
            cache: None,
            fetch_started_at_ms: START + 86_400_000,
        }
    }

    /// Runs `query` on `gateway`; `prepare` sets the recorder up with the
    /// run's stop handle, and `cancel_first` presses [Cancel] before the run.
    async fn query_with(
        gateway: &FakeLogs,
        store: &Arc<QueryStore>,
        query: &str,
        prepare: impl FnOnce(&mut Recorder),
        cancel_first: bool,
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
        if cancel_first {
            recorder.cancel();
        }
        let outcome = run_query(gateway, &run(query), store, &retrier, &mut recorder).await;
        (outcome, recorder)
    }

    fn three_events() -> FakeLogs {
        FakeLogs::with(&[
            ("a", START + 1_000, "ERROR one"),
            ("b", START + 2_000, "INFO two"),
            ("a", START + 3_000, "ERROR three"),
        ])
    }

    fn messages(store: &QueryStore, result: &QueryResult) -> Vec<String> {
        let positions = result.window(result.run_id, result.holdings_epoch, 0, usize::MAX);
        store
            .rows_at(result.holdings_epoch, positions)
            .unwrap()
            .events
            .into_iter()
            .map(|event| event.message)
            .collect()
    }

    #[tokio::test]
    async fn a_completed_fetch_is_evaluated_and_its_matches_are_kept() {
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) = query_with(
            &three_events(),
            &store,
            "filter @message like /ERROR/",
            |_| {},
            false,
        )
        .await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
        assert_eq!(
            recorder.heard,
            vec![
                "started",
                "planned",
                "fetch ended 7",
                "names 7",
                "evaluated"
            ],
            "the fetch's own end is held back"
        );
        assert_eq!(recorder.ended, Some(JobStatus::Completed));
        let result = &recorder.results[0];
        assert_eq!((result.run_id, result.evaluated_count), (7, 3));
        assert_eq!(result.holdings_epoch, store.epoch());
        assert_eq!(
            messages(&store, result),
            vec!["ERROR three", "ERROR one"],
            "newest first (U2:BR4.4)"
        );
        assert_eq!(
            store.key().map(|key| key.log_group_name),
            Some(GROUP.to_string())
        );
    }

    #[tokio::test]
    async fn a_failed_stream_still_evaluates_what_was_fetched() {
        // U1:BR2.5: CompletedWithFailures is evaluated; the session marks it
        // partial from the job it hears.
        let mut gateway = three_events();
        gateway.failing_stream = Some("c");
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) =
            query_with(&gateway, &store, "filter @logStream = 'a'", |_| {}, false).await;
        assert_eq!(recorder.ended, Some(JobStatus::CompletedWithFailures));
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 2 });
    }

    #[tokio::test]
    async fn a_failed_listing_is_not_evaluated() {
        // U1:BR2.6, UC3.
        let mut gateway = three_events();
        gateway.failing_listing = true;
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) =
            query_with(&gateway, &store, "filter @message like 'E'", |_| {}, false).await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Failed));
        assert!(recorder.results.is_empty());
        assert!(recorder.progress.is_empty());
    }

    #[tokio::test]
    async fn a_cancel_during_the_fetch_is_not_evaluated() {
        // U1:BR2.7, UC4 2: cancelled before the listing ended.
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) =
            query_with(&three_events(), &store, "filter @log = 'x'", |_| {}, true).await;
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Cancelled));
        assert!(recorder.results.is_empty());
    }

    #[tokio::test]
    async fn a_cancel_after_the_last_stream_is_not_evaluated_even_when_completed() {
        // Functional-design review R-01: the fetch ends Completed (nothing
        // was left unread, U5 FR5.9), but [Cancel] was pressed.
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) = query_with(
            &three_events(),
            &store,
            "filter @message like 'E'",
            |recorder| recorder.cancel_after_last_stream = true,
            false,
        )
        .await;
        assert_eq!(recorder.ended, Some(JobStatus::Completed));
        assert_eq!(outcome, QueryOutcome::NotEvaluated(JobStatus::Completed));
        assert!(recorder.results.is_empty());
        assert!(recorder.progress.is_empty());
    }

    #[tokio::test]
    async fn a_cancel_during_the_evaluation_stops_at_the_next_chunk_and_drops_the_rows() {
        // U1:BR2.7, BR5.8 with 25,000 events: the first chunk is evaluated,
        // then the stop is seen before the second.
        let events: Vec<(String, i64, String)> = (0..25_000i64)
            .map(|i| ("s".to_string(), START + i, "ERROR".to_string()))
            .collect();
        let gateway = FakeLogs {
            events,
            ..FakeLogs::default()
        };
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) = query_with(
            &gateway,
            &store,
            "filter @message like 'ERROR'",
            |recorder| recorder.cancel_after_first_chunk = true,
            false,
        )
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluationStopped);
        assert_eq!(recorder.progress, vec![(10_000, 25_000)]);
        assert!(recorder.results.is_empty());
        assert_eq!(recorder.heard.last().map(String::as_str), Some("stopped 7"));
        assert_eq!(
            store.len(),
            25_000,
            "the fetched events stay until the next run"
        );
    }

    #[tokio::test]
    async fn the_log_management_timeline_gets_nothing() {
        // U1:BR1.7: the query fetch goes to the QueryStore only.
        let log_view = Mutex::new(LogView::new());
        log_view.add(vec![LogEvent {
            timestamp: 1,
            ingestion_time: None,
            message: "shown in Log Management".to_string(),
            log_stream_name: "lm".to_string(),
            sequence: 0,
        }]);
        let before = log_view.lock().unwrap().rows(0, 10);
        let store = Arc::new(QueryStore::new());
        let (outcome, _) = query_with(
            &three_events(),
            &store,
            "filter @message like 'E'",
            |_| {},
            false,
        )
        .await;
        assert!(matches!(outcome, QueryOutcome::Evaluated { .. }));
        assert_eq!(store.len(), 3);
        assert_eq!(log_view.lock().unwrap().rows(0, 10), before);
    }

    // ---- 261017-insights-query U2 ---------------------------------------

    /// `count` JSON events on one stream, a second apart, with `n` from 0
    /// to 6 and `level` on every other one.
    fn json_logs(count: i64) -> FakeLogs {
        let events: Vec<(String, i64, String)> = (0..count)
            .map(|i| {
                let level = if i % 2 == 0 {
                    r#","level":"error""#
                } else {
                    ""
                };
                (
                    "s".to_string(),
                    START + i,
                    format!(r#"{{"n":{}{level}}}"#, i % 7),
                )
            })
            .collect();
        FakeLogs {
            events,
            ..FakeLogs::default()
        }
    }

    #[tokio::test]
    async fn a_query_with_json_fields_sort_and_limit_runs_to_its_end() {
        // UC1 with U2:BR3.1-BR5.2: the columns, the order and the cut.
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) = query_with(
            &json_logs(30),
            &store,
            "fields @timestamp, n, level | filter n >= 5 | sort n desc, @timestamp | limit 3",
            |_| {},
            false,
        )
        .await;
        assert_eq!(outcome, QueryOutcome::Evaluated { rows: 3 });
        let result = &recorder.results[0];
        let names: Vec<&str> = result.columns().iter().map(|field| field.name()).collect();
        assert_eq!(names, vec!["@timestamp", "n", "level"]);
        assert_eq!(result.matched_count(), 8, "n is 5 or 6 for 8 of 30");
        assert_eq!(result.truncated_at(), Some(3));
        let stamps: Vec<i64> = store
            .rows_at(
                result.holdings_epoch,
                result.window(7, result.holdings_epoch, 0, 9),
            )
            .unwrap()
            .events
            .iter()
            .map(|event| event.timestamp - START)
            .collect();
        assert_eq!(stamps, vec![6, 13, 20], "n = 6, oldest first among equals");
    }

    #[tokio::test]
    async fn a_cancel_while_the_sort_values_are_taken_stops_before_the_next_chunk() {
        // R-02: the values of a JSON sort are taken chunk by chunk, and
        // [Cancel] is seen between two of them.
        let store = Arc::new(QueryStore::new());
        let (outcome, recorder) = query_with(
            &json_logs(25_000),
            &store,
            "sort n desc",
            |recorder| recorder.cancel_after_first_chunk = true,
            false,
        )
        .await;
        assert_eq!(outcome, QueryOutcome::EvaluationStopped);
        assert_eq!(recorder.progress, vec![(10_000, 25_000)]);
        assert!(recorder.results.is_empty());
    }

    /// A step whose thread panics, as an abnormal end of the evaluation.
    fn panicking_step(_: &QueryStore, _: &QueryPlan, _: &mut Evaluation, _: u64) -> bool {
        panic!("an evaluation step that ends abnormally (test)");
    }

    #[tokio::test]
    async fn an_evaluation_that_ends_abnormally_is_told_as_an_evaluation_failure() {
        // U2:BR10.2 (U1 code review R-03): not a failed fetch.
        let store = Arc::new(QueryStore::new());
        let run = run("filter @message like 'E'");
        store.begin_run(run.key.clone());
        store.add(vec![LogEvent {
            timestamp: START,
            ingestion_time: None,
            message: "ERROR".to_string(),
            log_stream_name: "s".to_string(),
            sequence: 0,
        }]);
        let policy = RetryPolicy::default();
        let jitter = FixedJitter(0.0);
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &jitter, &signal);
        let mut recorder = Recorder::default();
        let outcome = evaluate_held(&run, &store, &retrier, &mut recorder, panicking_step).await;
        assert_eq!(outcome, QueryOutcome::EvaluationFailed);
        assert_eq!(recorder.heard, vec!["evaluation failed 7"]);
        assert!(recorder.results.is_empty());
        assert_eq!(store.len(), 1, "the fetched logs stay");
    }

    #[tokio::test]
    async fn the_field_names_of_the_fetched_logs_are_told_after_the_fetch() {
        // U2:BR1.7: only when the fetch ended with logs.
        let store = Arc::new(QueryStore::new());
        let (_, recorder) = query_with(&json_logs(4), &store, "limit 1", |_| {}, false).await;
        assert_eq!(
            recorder.names,
            vec![(7, vec!["level".to_string(), "n".to_string()])]
        );
        let mut failing = json_logs(4);
        failing.failing_listing = true;
        let (_, recorder) = query_with(&failing, &store, "limit 1", |_| {}, false).await;
        assert!(recorder.names.is_empty(), "a failed fetch has no names");
    }
}
