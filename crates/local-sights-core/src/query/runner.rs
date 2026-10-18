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
//! The evaluation goes [`EVALUATION_CHUNK`](crate::query::evaluator::EVALUATION_CHUNK)
//! events at a time, each chunk on a blocking thread under one lock of the
//! store, and between chunks the stop signal, the sink's answer to the
//! progress and the store's epoch are checked (U1:BR5.8). The Log Management
//! screen's timeline is never touched (U1:BR1.7).

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
/// the end of the fetch, the evaluation's progress and its end.
pub trait QuerySink: FetchSink + Send {
    /// The fetch of `run_id` ended with `job` (its `on_finished` is not
    /// sent). Returns whether the run may still be evaluated: `false` once
    /// [Cancel] was pressed or the run is no longer the current one (R-01).
    fn on_fetch_ended(&mut self, run_id: u64, job: &FetchJob) -> bool;
    /// `evaluated` of `total` events were evaluated. Returns whether to go
    /// on (U1:BR2.4, BR5.8).
    fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool;
    /// The evaluation finished with `result`.
    fn on_evaluated(&mut self, result: QueryResult);
    /// The evaluation stopped before its end; what it found is dropped
    /// (U1:BR2.7, BR5.8).
    fn on_evaluation_stopped(&mut self, run_id: u64);
}

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
    /// An evaluation chunk ended abnormally (its thread panicked); the sink
    /// heard nothing and the caller's recovery ends the run.
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
    if !fetched || !allowed {
        return QueryOutcome::NotEvaluated(job.status);
    }
    evaluate_held(run, store, retrier, sink).await
}

/// UC1 11-12: evaluates the holdings chunk by chunk.
async fn evaluate_held<S: QuerySink>(
    run: &QueryRun,
    store: &Arc<QueryStore>,
    retrier: &Retrier<'_>,
    sink: &mut S,
) -> QueryOutcome {
    let epoch = store.epoch();
    let mut evaluation = Evaluation::new(store.len());
    while evaluation.next_chunk().is_some() {
        if retrier.is_stopped() {
            sink.on_evaluation_stopped(run.run_id);
            return QueryOutcome::EvaluationStopped;
        }
        let task_store = Arc::clone(store);
        let plan = Arc::clone(&run.plan);
        let task = tokio::task::spawn_blocking(move || {
            let current = task_store.evaluate_chunk(&plan, &mut evaluation, epoch);
            (evaluation, current)
        });
        let current;
        (evaluation, current) = match task.await {
            Ok(done) => done,
            Err(_) => {
                eprintln!("local-sights: query: an evaluation chunk ended abnormally");
                return QueryOutcome::EvaluationFailed;
            }
        };
        let go_on = current
            && sink.on_evaluation_progress(run.run_id, evaluation.evaluated(), evaluation.total());
        if !go_on {
            sink.on_evaluation_stopped(run.run_id);
            return QueryOutcome::EvaluationStopped;
        }
    }
    let evaluated = evaluation.evaluated();
    let rows = evaluation.into_rows();
    let count = rows.len();
    sink.on_evaluated(QueryResult::new(run.run_id, epoch, evaluated, rows));
    QueryOutcome::Evaluated { rows: count }
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
            vec!["started", "planned", "fetch ended 7", "evaluated"],
            "the fetch's own end is held back"
        );
        assert_eq!(recorder.ended, Some(JobStatus::Completed));
        let result = &recorder.results[0];
        assert_eq!((result.run_id, result.evaluated_count), (7, 3));
        assert_eq!(result.holdings_epoch, store.epoch());
        assert_eq!(messages(&store, result), vec!["ERROR one", "ERROR three"]);
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
}
