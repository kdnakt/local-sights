//! compat_check: developer-only comparison of the app's query results with
//! the console's Logs Insights (261017-insights-query U4, SM1, SM2, NFR3).
//!
//! First export the Logs Insights results of the query set with
//! `scripts/compat/insights-export.sh` (CLI, by hand, into a folder outside
//! the repository, U4:BR1.1, BR1.2), then run this check on that folder:
//!
//! ```text
//! cargo run -p local-sights-core --example compat_check -- \
//!   --export-dir ~/insights-export [--queries FILE]
//! ```
//!
//! The connection, log group and period come from `settings.json` in the
//! export folder only, as the script ran with them (U4:BR1.4); there are no
//! arguments for them, so the two cannot be given different conditions. The
//! period goes from millisecond 0 of its start second to millisecond 999 of
//! its end second, as the app reads a period (review R-03); a query whose
//! result has events in either boundary second is marked
//! `boundary-events`, since how Logs Insights takes the end second has not
//! been checked. The period must have ended at least 30 minutes ago
//! (U4:BR1.3). `--queries` chooses the query set (by default
//! `examples/compat_queries.txt`, for CloudTrail), the same file the script
//! was given (QUERIES=).
//!
//! The range is fetched once, as the query screen fetches it (a QueryStore
//! filled by the QueryRunner through the FetchCoordinator: DescribeLogStreams
//! and GetLogEvents only, U4:BR1.1, NFR5), and every query of the set is read
//! and evaluated on it by the app's own core, then compared with its
//! `<id>.json` (U4:BR2.1-BR2.5). Standard output gets one line a query and
//! the summary (U4:BR3.2); standard error the progress and the fetch's
//! counts. Exit codes (U4:BR2.6, review R-02): 0 every compared query is the
//! same (or refused by both), 1 some query differs, 2 the comparison could
//! not be made (wrong arguments or settings, a failed or incomplete fetch,
//! an export missing or not Complete, or no query compared at all).
//!
//! Run it only on your own machine with your own credentials; `cargo test`
//! compiles it and runs its tests with sample data and a fake gateway, never
//! against AWS, and it is not part of the distributed app.

mod compat;

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use chrono_tz::Tz;
use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink};
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::gateway::CloudWatchLogsGateway;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::query::evaluator::{EvalOutput, Evaluation, QueryResult, ResultRow};
use local_sights_core::query::instant::resolve_date_texts;
use local_sights_core::query::parser::parse_query;
use local_sights_core::query::plan::{
    Command, CommandKind, DEFAULT_RESULT_LIMIT, FieldRef, QueryPlan, Span,
};
use local_sights_core::query::runner::{
    QueryOutcome, QueryRun, QueryRunSummary, QuerySink, run_query,
};
use local_sights_core::query::store::{HoldingKey, QueryStore};
use local_sights_core::retry::{RandomJitter, Retrier, RetryPolicy, StopSignal};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::time_range::TimeRange;

use compat::compare::{AppRows, AppSide, CompareSpec, compare_query};
use compat::export::{InsightsSide, parse_export_of_run};
use compat::format::app_row;
use compat::queries::{CompareMode, CompatQuery, parse_query_set};
use compat::report::{EXIT_NOT_COMPARED, exit_code, query_lines, summary_line};
use compat::settings::{RunSettings, check_end_age, check_outside, parse_settings};

const USAGE: &str = "usage: compat_check --export-dir DIR [--queries FILE]   \
(DIR is the folder insights-export.sh wrote, outside the repository)";

/// The settings file the script writes into the export folder (BR1.4).
const SETTINGS_FILE: &str = "settings.json";

/// The query set used when `--queries` is not given.
const DEFAULT_QUERIES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/compat_queries.txt");

/// The query that fills the store: the range is fetched once and every query
/// of the set is evaluated on what was fetched.
const FETCH_QUERY: &str = "limit 1";

/// R-05: folder names are compared without regard to case where the file
/// system usually ignores it (Windows, and macOS by default), so a folder
/// inside the repository written in other case is still refused.
const IGNORE_CASE: bool = cfg!(any(windows, target_os = "macos"));

/// The parsed arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckArgs {
    export_dir: PathBuf,
    queries: PathBuf,
}

/// What a run needs, checked before anything is fetched.
#[derive(Debug, Clone)]
struct Prepared {
    settings: RunSettings,
    queries: Vec<CompatQuery>,
    export_dir: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => return usage_error(&message),
    };
    let repository = repository_root(Path::new(env!("CARGO_MANIFEST_DIR")));
    let prepared = match prepare(&args, &repository, now_epoch_seconds()) {
        Ok(prepared) => prepared,
        Err(message) => return usage_error(&message),
    };
    let gateway = AwsCloudWatchLogsGateway::new();
    let mut out = BufWriter::new(io::stdout().lock());
    let code = run_check(&gateway, &prepared, &mut out).await;
    if let Err(error) = out.flush() {
        eprintln!("error: could not write to standard output: {error}");
        return ExitCode::from(EXIT_NOT_COMPARED);
    }
    ExitCode::from(code)
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{USAGE}");
    eprintln!("  - {message}");
    ExitCode::from(EXIT_NOT_COMPARED)
}

/// Parses `--export-dir DIR` (required) and `--queries FILE`.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<CheckArgs, String> {
    let mut export_dir: Option<PathBuf> = None;
    let mut queries = PathBuf::from(DEFAULT_QUERIES);
    while let Some(flag) = args.next() {
        let value = match flag.as_str() {
            "--export-dir" | "--queries" => args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?,
            "-h" | "--help" => return Err("help requested".to_string()),
            other => return Err(format!("unknown argument: {other}")),
        };
        if flag == "--export-dir" {
            export_dir = Some(PathBuf::from(value));
        } else {
            queries = PathBuf::from(value);
        }
    }
    let export_dir = export_dir.ok_or_else(|| "--export-dir is required".to_string())?;
    Ok(CheckArgs {
        export_dir,
        queries,
    })
}

/// The repository's top folder: the nearest folder above `start` that has
/// `.git` (R-05), or else the workspace folder two levels above the crate.
fn repository_root(start: &Path) -> PathBuf {
    start
        .ancestors()
        .find(|folder| folder.join(".git").exists())
        .or_else(|| start.parent().and_then(Path::parent))
        .unwrap_or(start)
        .to_path_buf()
}

/// BR1.2-BR1.4: the export folder is outside `repository` (both with their
/// symbolic links resolved), `settings.json` is read, the period ended at
/// least 30 minutes before `now_seconds`, and the query set is read.
fn prepare(args: &CheckArgs, repository: &Path, now_seconds: i64) -> Result<Prepared, String> {
    let export_dir = args.export_dir.canonicalize().map_err(|error| {
        format!(
            "the export folder {} cannot be opened: {error}",
            args.export_dir.display()
        )
    })?;
    let repository = repository
        .canonicalize()
        .map_err(|error| format!("the repository folder cannot be resolved: {error}"))?;
    check_outside(&export_dir, &repository, IGNORE_CASE).map_err(|error| error.to_string())?;
    let settings_text = std::fs::read_to_string(export_dir.join(SETTINGS_FILE))
        .map_err(|error| format!("{SETTINGS_FILE} cannot be read: {error}"))?;
    let settings = parse_settings(&settings_text).map_err(|error| error.to_string())?;
    check_end_age(settings.end_seconds, now_seconds).map_err(|error| error.to_string())?;
    let queries_text = std::fs::read_to_string(&args.queries).map_err(|error| {
        format!(
            "the query set {} cannot be read: {error}",
            args.queries.display()
        )
    })?;
    let queries = parse_query_set(&queries_text).map_err(|error| error.to_string())?;
    Ok(Prepared {
        settings,
        queries,
        export_dir,
    })
}

/// UC2 steps 3-6: fetches the period once, compares every query and writes
/// the report to `out`. Returns the exit code.
async fn run_check<G: CloudWatchLogsGateway>(
    gateway: &G,
    prepared: &Prepared,
    out: &mut dyn Write,
) -> u8 {
    let settings = &prepared.settings;
    let Some(range) = period(settings) else {
        eprintln!("error: the period of {SETTINGS_FILE} cannot be read as a range");
        return EXIT_NOT_COMPARED;
    };
    let Some(store) = fetch_period(gateway, settings, range).await else {
        return EXIT_NOT_COMPARED;
    };
    let epoch = store.epoch();
    eprintln!(
        "comparing {} queries on {} events...",
        prepared.queries.len(),
        store.view_len(epoch).unwrap_or(0)
    );
    let mut verdicts = Vec::new();
    for query in &prepared.queries {
        let Some(app) = evaluate_query(&store, epoch, &query.text, range) else {
            eprintln!("error: the evaluation of {} stopped", query.id);
            return EXIT_NOT_COMPARED;
        };
        let insights = read_export(
            &prepared.export_dir,
            &query.id,
            settings.exported_at_seconds,
        );
        let spec = app.spec(query.mode);
        let comparison = compare_query(&spec, &app.side, &insights);
        verdicts.push(comparison.verdict);
        for line in query_lines(&query.id, &comparison, app.boundary_events) {
            if writeln!(out, "{line}").is_err() {
                return EXIT_NOT_COMPARED;
            }
        }
    }
    if writeln!(out, "{}", summary_line(&verdicts)).is_err() {
        return EXIT_NOT_COMPARED;
    }
    exit_code(&verdicts)
}

/// R-03: from millisecond 0 of the start second to millisecond 999 of the
/// end second, as the app reads a period.
fn period(settings: &RunSettings) -> Option<TimeRange> {
    let start = settings.start_seconds.checked_mul(1_000)?;
    let end = settings.end_seconds.checked_mul(1_000)?.checked_add(999)?;
    TimeRange::from_instants(start, end).ok()
}

/// UC2 step 3: fetches `range` of the log group into a fresh QueryStore as
/// the query screen does. `None` (reported on standard error) unless every
/// stream was read to its end: a partial fetch is not compared.
async fn fetch_period<G: CloudWatchLogsGateway>(
    gateway: &G,
    settings: &RunSettings,
    range: TimeRange,
) -> Option<Arc<QueryStore>> {
    let plan = match parse_query(FETCH_QUERY) {
        Ok(plan) => plan,
        Err(problem) => {
            eprintln!("error: the fetch query cannot be read: {problem:?}");
            return None;
        }
    };
    let profile = settings
        .profile
        .clone()
        .map_or(ProfileSelector::SdkDefault, ProfileSelector::Named);
    let run = QueryRun {
        run_id: 1,
        plan: Arc::new(plan),
        range,
        keys: vec![HoldingKey {
            profile,
            region: settings.region.clone(),
            log_group_name: settings.log_group.clone(),
        }],
        cache: None,
        run_started_at_ms: now_epoch_seconds().saturating_mul(1_000),
        clock: now_epoch_ms,
    };
    let policy = RetryPolicy::default();
    let jitter = RandomJitter;
    let stop = StopSignal::never();
    let retrier = Retrier::new(&policy, &jitter, &stop);
    let store = Arc::new(QueryStore::new());
    let mut sink = FetchOnlySink::default();
    // Boxed: the fetch's future is deep enough to overflow the compiler's
    // layout query depth inside `main` otherwise.
    let outcome = Box::pin(run_query(gateway, &run, &store, &retrier, &mut sink)).await;
    report_fetch(&sink);
    let completed = matches!(outcome, QueryOutcome::Evaluated { .. })
        && sink
            .summary
            .as_ref()
            .is_some_and(|summary| !summary.partial && !summary.failed())
        && sink
            .job
            .as_ref()
            .is_none_or(|job| job.status == JobStatus::Completed);
    if !completed {
        eprintln!("error: the fetch did not complete ({outcome:?}); nothing is compared");
        return None;
    }
    Some(store)
}

/// The fetch's counts and failures, safe details only (U1:BR4.3).
fn report_fetch(sink: &FetchOnlySink) {
    if let Some(job) = &sink.job {
        eprintln!(
            "events: {}  streams: {}  failed streams: {}",
            job.event_count,
            job.planned_stream_count,
            job.failed_streams.len()
        );
    }
    let Some(summary) = &sink.summary else {
        return;
    };
    for group in &summary.groups {
        if let Some(failure) = &group.failure {
            eprintln!(
                "error: {}: {} ({})",
                group.log_group_name,
                failure.kind().as_str(),
                failure.safe_detail()
            );
        }
    }
    for failed in summary.failed_streams() {
        eprintln!(
            "error: stream {}: {} ({})",
            failed.log_stream_name,
            failed.failure.kind().as_str(),
            failed.failure.safe_detail()
        );
    }
}

/// What the app made of one query, with what its comparison needs.
#[derive(Debug, Clone)]
struct AppEvaluation {
    side: AppSide,
    sorted: bool,
    sort_keys: Vec<String>,
    cap: usize,
    /// R-03: the result has events in the start or the end second.
    boundary_events: bool,
}

impl AppEvaluation {
    fn rejected(message: String) -> Self {
        Self {
            side: AppSide::Rejected(message),
            sorted: false,
            sort_keys: Vec::new(),
            cap: DEFAULT_RESULT_LIMIT,
            boundary_events: false,
        }
    }

    fn spec(&self, mode: CompareMode) -> CompareSpec {
        CompareSpec {
            mode,
            sorted: self.sorted,
            sort_keys: self.sort_keys.clone(),
            cap: self.cap,
        }
    }
}

/// UC2 step 4: reads `text` as the app does (dates as UTC, like
/// fetch_check) and evaluates it on the store. A query the app cannot read
/// is the app's refusal (BR2.4). `None` when the evaluation stopped.
fn evaluate_query(
    store: &QueryStore,
    epoch: u64,
    text: &str,
    range: TimeRange,
) -> Option<AppEvaluation> {
    let plan = match parse_query(text).and_then(|plan| resolve_date_texts(plan, Tz::UTC)) {
        Ok(plan) => plan,
        Err(problem) => {
            let word = problem
                .word
                .map(|word| format!(" ({word})"))
                .unwrap_or_default();
            return Some(AppEvaluation::rejected(format!(
                "{:?} {}{word} at line {}, column {}",
                problem.kind, problem.message_key, problem.span.line, problem.span.column
            )));
        }
    };
    let (output, rows) = evaluate_on_store(store, epoch, &plan)?;
    let cut = output.truncated_at.is_some();
    let next_row = if cut && plan.has_sort() {
        next_row_after_cut(store, epoch, &plan, rows.len(), &output.columns)?
    } else {
        None
    };
    let boundary_events = rows.iter().any(|row| in_boundary_second(row, range));
    Some(AppEvaluation {
        side: AppSide::Rows(AppRows {
            columns: output
                .columns
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            rows: rows
                .iter()
                .map(|row| app_row(&output.columns, row))
                .collect(),
            cut,
            next_row,
        }),
        sorted: plan.has_sort(),
        sort_keys: visible_sort_keys(&plan),
        cap: row_cap(&plan),
        boundary_events,
    })
}

/// Evaluates `plan` on the view of `epoch` step by step and copies the
/// result rows out (as the QueryRunner does). `None` when the view is gone.
fn evaluate_on_store(
    store: &QueryStore,
    epoch: u64,
    plan: &QueryPlan,
) -> Option<(EvalOutput, Vec<ResultRow>)> {
    let total = store.view_len(epoch)?;
    let mut evaluation = Evaluation::new(plan, total);
    while !evaluation.is_done() {
        if !store.evaluate_step(plan, &mut evaluation, epoch) {
            return None;
        }
    }
    let output = evaluation.into_output()?;
    let rows = store.copy_rows(epoch, &output.rows)?;
    Some((output, rows))
}

/// R-01: the first row the cut left out, from the same plan cut one row
/// later. `Some(None)` when there is none.
fn next_row_after_cut(
    store: &QueryStore,
    epoch: u64,
    plan: &QueryPlan,
    kept: usize,
    columns: &[FieldRef],
) -> Option<Option<compat::format::Row>> {
    let mut longer = plan.clone();
    let last_limit = longer
        .commands
        .iter_mut()
        .rev()
        .find(|command| matches!(command.kind, CommandKind::Limit(count) if count > 0));
    match last_limit {
        Some(command) => {
            if let CommandKind::Limit(count) = &mut command.kind {
                *count = count.saturating_add(1);
            }
        }
        None => longer.commands.push(Command {
            kind: CommandKind::Limit(u32::try_from(DEFAULT_RESULT_LIMIT + 1).unwrap_or(u32::MAX)),
            span: Span::new(1, 1, 1),
        }),
    }
    let (output, _) = evaluate_on_store(store, epoch, &longer)?;
    let Some(position) = output.rows.get(kept) else {
        return Some(None);
    };
    let row = store.copy_rows(epoch, &[*position])?;
    Some(row.first().map(|row| app_row(columns, row)))
}

/// The fields of the last `sort` when every one of them is a column of the
/// query (so both sides show the sort values); empty otherwise.
fn visible_sort_keys(plan: &QueryPlan) -> Vec<String> {
    let Some(keys) = plan
        .commands
        .iter()
        .rev()
        .find_map(|command| match &command.kind {
            CommandKind::Sort(keys) => Some(keys),
            _ => None,
        })
    else {
        return Vec::new();
    };
    let columns = plan.columns();
    if keys.iter().all(|key| columns.contains(&key.field)) {
        keys.iter()
            .map(|key| key.field.name().to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// The row cap of `plan`: its last `limit` from 1, or 10,000 (U2:BR5.1,
/// BR5.2; `limit 0` keeps every row up to 10,000).
fn row_cap(plan: &QueryPlan) -> usize {
    plan.commands
        .iter()
        .rev()
        .find_map(|command| match command.kind {
            CommandKind::Limit(count) if count > 0 => usize::try_from(count).ok(),
            _ => None,
        })
        .unwrap_or(DEFAULT_RESULT_LIMIT)
}

/// R-03: whether `row`'s event lies in the first or the last second of
/// `range`.
fn in_boundary_second(row: &ResultRow, range: TimeRange) -> bool {
    let time = row.event.timestamp;
    let first = range.start_instant();
    let last = range.end_instant();
    (first..first + 1_000).contains(&time) || (last - 999..=last).contains(&time)
}

/// The export of query `id` (R-02): missing when there is no file, not run
/// when it cannot be read, of another run when its `exportedAtSeconds` is
/// not `run`, the value of settings.json (code review R-02).
fn read_export(export_dir: &Path, id: &str, run: Option<i64>) -> InsightsSide {
    // The id is letters, digits and '-' only (query set), so this stays in
    // the folder.
    let path = export_dir.join(format!("{id}.json"));
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_export_of_run(&text, run)
            .unwrap_or_else(|error| InsightsSide::NotRun(format!("unreadable ({error})"))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => InsightsSide::Missing,
        Err(error) => InsightsSide::NotRun(format!("unreadable ({error})")),
    }
}

/// The time now in epoch seconds (0 before 1970).
fn now_epoch_seconds() -> i64 {
    now_epoch_ms() / 1_000
}

/// The time now in epoch milliseconds (0 before 1970).
fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

/// Keeps what the fetch ended with; the fetch query's own result is not
/// used.
#[derive(Default)]
struct FetchOnlySink {
    job: Option<FetchJob>,
    summary: Option<QueryRunSummary>,
}

impl FetchSink for FetchOnlySink {
    fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {
        eprintln!("listing streams...");
    }

    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}

    fn on_planned(&mut self, job: &FetchJob) {
        eprintln!("fetching {} streams...", job.planned_stream_count);
    }

    fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}

    fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {}

    fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {}
}

impl QuerySink for FetchOnlySink {
    fn on_reading_cache(&mut self, _run_id: u64) {}

    fn on_group_started(&mut self, _run_id: u64, _index: usize, _count: usize) {}

    fn on_range_fetched(&mut self, _run_id: u64, _log_group_name: &str, job: &FetchJob) {
        self.job = Some(job.clone());
    }

    fn on_fetch_ended(&mut self, _run_id: u64, summary: &QueryRunSummary) -> bool {
        self.summary = Some(summary.clone());
        true
    }

    fn on_evaluation_progress(&mut self, _run_id: u64, _evaluated: usize, _total: usize) -> bool {
        true
    }

    fn on_evaluated(&mut self, _result: QueryResult) {}

    fn on_evaluation_stopped(&mut self, _run_id: u64) {}

    fn on_field_names(&mut self, _run_id: u64, _names: Vec<String>) {}

    fn on_evaluation_failed(&mut self, _run_id: u64) {}
}

/// The wiring with a fake gateway and exports written to a temporary
/// folder; nothing here connects to AWS (`cargo test -p local-sights-core
/// --example compat_check`).
#[cfg(test)]
mod tests {
    use super::*;
    use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
    use local_sights_core::gateway::{
        DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
        DescribeLogStreamsRequest, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
    };
    use local_sights_core::streams::LogStream;

    /// 2024/01/02 00:00:00 UTC in epoch seconds.
    const START: i64 = 1_704_153_600;
    const END: i64 = START + 3_599;
    /// The run of the script the exports of the tests come from (code
    /// review R-02).
    const EXPORTED_AT: i64 = END + 7_200;
    const STREAM: &str = "123456789012_CloudTrail_us-east-1_0";

    /// One stream of CloudTrail-like events, one page; can refuse the
    /// stream listing. Never reaches AWS.
    #[derive(Default)]
    struct FakeLogs {
        events: Vec<(i64, String)>,
        refuse_listing: bool,
    }

    impl FakeLogs {
        fn cloudtrail() -> Self {
            let event = |second: i64, name: &str, source: &str| {
                (
                    (START + second) * 1_000,
                    format!(r#"{{"eventName":"{name}","eventSource":"{source}"}}"#),
                )
            };
            Self {
                events: vec![
                    event(10, "GetObject", "s3.amazonaws.com"),
                    event(20, "Decrypt", "kms.amazonaws.com"),
                    event(30, "GetBucketAcl", "s3.amazonaws.com"),
                ],
                refuse_listing: false,
            }
        }
    }

    impl CloudWatchLogsGateway for FakeLogs {
        async fn get_log_events(
            &self,
            request: GetLogEventsRequest,
        ) -> Result<GetLogEventsPage, ApiFailure> {
            Ok(GetLogEventsPage {
                events: self
                    .events
                    .iter()
                    .filter(|(at, _)| *at >= request.start_time && *at < request.end_time)
                    .map(|(at, message)| GatewayEvent {
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
            panic!("compat_check never lists log groups");
        }

        async fn describe_log_streams(
            &self,
            _request: DescribeLogStreamsRequest,
        ) -> Result<DescribeLogStreamsPage, ApiFailure> {
            if self.refuse_listing {
                return Err(ApiFailure::new(
                    FailureKind::AccessDenied,
                    &SafeDetailFields::default(),
                ));
            }
            Ok(DescribeLogStreamsPage {
                streams: vec![LogStream {
                    log_stream_name: STREAM.to_string(),
                    first_event_timestamp: None,
                    last_event_timestamp: None,
                }],
                next_token: None,
            })
        }
    }

    const QUERIES: &str = "q1-like\tset\tlike\tfields eventName | filter eventName like /Get/\n\
        q2-latest\torder\tsort\tfields @timestamp, eventName | sort @timestamp desc | limit 2\n";

    fn settings() -> RunSettings {
        RunSettings {
            profile: Some("dev".to_string()),
            region: "us-east-1".to_string(),
            log_group: "CloudTrail".to_string(),
            start_seconds: START,
            end_seconds: END,
            exported_at_seconds: Some(EXPORTED_AT),
        }
    }

    fn write(folder: &Path, name: &str, text: &str) {
        std::fs::write(folder.join(name), text).unwrap();
    }

    /// Writes an export of the run [`EXPORTED_AT`], as the script does:
    /// `json` is the export without its stamp.
    fn write_export(folder: &Path, name: &str, json: &str) {
        let stamped = json.replacen('{', &format!(r#"{{"exportedAtSeconds":{EXPORTED_AT},"#), 1);
        write(folder, name, &stamped);
    }

    /// The exports Logs Insights would give for [`QUERIES`] on the fake's
    /// events; `latest_second` is the eventName of q2's second row.
    fn write_exports(folder: &Path, latest_second: &str) {
        write_export(
            folder,
            "q1-like.json",
            r#"{"status":"Complete","results":[
                [{"field":"eventName","value":"GetBucketAcl"},{"field":"@ptr","value":"a"}],
                [{"field":"eventName","value":"GetObject"},{"field":"@ptr","value":"b"}]]}"#,
        );
        write_export(
            folder,
            "q2-latest.json",
            &format!(
                r#"{{"status":"Complete","results":[
                [{{"field":"@timestamp","value":"2024-01-02 00:00:30.000"}},
                 {{"field":"eventName","value":"GetBucketAcl"}},{{"field":"@ptr","value":"c"}}],
                [{{"field":"@timestamp","value":"2024-01-02 00:00:20.000"}},
                 {{"field":"eventName","value":"{latest_second}"}},{{"field":"@ptr","value":"d"}}]]}}"#
            ),
        );
    }

    fn prepared(folder: &Path) -> Prepared {
        Prepared {
            settings: settings(),
            queries: parse_query_set(QUERIES).unwrap(),
            export_dir: folder.to_path_buf(),
        }
    }

    async fn run(gateway: &FakeLogs, folder: &Path) -> (u8, String) {
        let mut out = Vec::new();
        let code = run_check(gateway, &prepared(folder), &mut out).await;
        (code, String::from_utf8(out).unwrap())
    }

    #[tokio::test]
    async fn the_same_results_on_both_sides_are_compatible() {
        let folder = tempfile::tempdir().unwrap();
        write_exports(folder.path(), "Decrypt");
        let (code, out) = run(&FakeLogs::cloudtrail(), folder.path()).await;
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("q1-like  Same  insights=2  app=2"), "{out}");
        assert!(out.contains("q2-latest  Same  insights=2  app=2"), "{out}");
        assert!(out.ends_with("Skipped=0  NotCompared=0\n"), "{out}");
    }

    #[tokio::test]
    async fn a_different_row_fails_and_is_listed() {
        let folder = tempfile::tempdir().unwrap();
        write_exports(folder.path(), "Encrypt");
        let (code, out) = run(&FakeLogs::cloudtrail(), folder.path()).await;
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("q2-latest  Different"), "{out}");
        assert!(
            out.contains("only in Logs Insights: 2024-01-02 00:00:20.000 eventName=Encrypt"),
            "{out}"
        );
        assert!(
            out.contains("only in app: 2024-01-02 00:00:20.000 eventName=Decrypt"),
            "{out}"
        );
    }

    #[tokio::test]
    async fn a_failed_fetch_or_a_missing_export_cannot_compare() {
        let folder = tempfile::tempdir().unwrap();
        write_exports(folder.path(), "Decrypt");
        let refusing = FakeLogs {
            refuse_listing: true,
            ..FakeLogs::cloudtrail()
        };
        let (code, out) = run(&refusing, folder.path()).await;
        assert_eq!(code, 2);
        assert!(
            out.is_empty(),
            "nothing is compared after a failed fetch: {out}"
        );
        std::fs::remove_file(folder.path().join("q1-like.json")).unwrap();
        let (code, out) = run(&FakeLogs::cloudtrail(), folder.path()).await;
        assert_eq!(code, 2, "R-02: {out}");
        assert!(out.contains("q1-like  NotCompared"), "{out}");
    }

    /// Code review R-02: an export left in the folder by an earlier run of
    /// the script (another `exportedAtSeconds` than settings.json) is not
    /// compared, so the run cannot compare; the file is left as it is.
    #[tokio::test]
    async fn an_export_left_from_an_earlier_run_is_not_compared() {
        let folder = tempfile::tempdir().unwrap();
        write_exports(folder.path(), "Decrypt");
        let earlier = format!(
            r#"{{"exportedAtSeconds":{},"status":"Complete","results":[
                [{{"field":"eventName","value":"GetBucketAcl"}}],
                [{{"field":"eventName","value":"GetObject"}}]]}}"#,
            EXPORTED_AT - 3_600
        );
        write(folder.path(), "q1-like.json", &earlier);
        let (code, out) = run(&FakeLogs::cloudtrail(), folder.path()).await;
        assert_eq!(code, 2, "{out}");
        assert!(out.contains("q1-like  NotCompared"), "{out}");
        assert!(out.contains("another run"), "{out}");
        assert!(out.contains("q2-latest  Same"), "{out}");
        assert!(
            folder.path().join("q1-like.json").exists(),
            "the file is not removed"
        );
    }

    #[tokio::test]
    async fn a_tie_at_the_cut_and_events_in_a_boundary_second_are_marked() {
        let folder = tempfile::tempdir().unwrap();
        // With the extra kms event, sort eventSource asc | limit 3 keeps both
        // kms events and one of the two s3 events, and the other s3 event is
        // the first one cut (R-01). This export misses a kms row.
        write_export(
            folder.path(),
            "q3-tie.json",
            r#"{"status":"Complete","results":[
                [{"field":"eventSource","value":"kms.amazonaws.com"}],
                [{"field":"eventSource","value":"s3.amazonaws.com"}]]}"#,
        );
        let mut gateway = FakeLogs::cloudtrail();
        // R-03: an event in the start second of the period.
        gateway.events.insert(
            0,
            (
                START * 1_000 + 500,
                r#"{"eventName":"Encrypt","eventSource":"kms.amazonaws.com"}"#.to_string(),
            ),
        );
        let prepared = Prepared {
            queries: parse_query_set(
                "q3-tie\torder\ttie\tfields eventSource | sort eventSource asc | limit 3\n",
            )
            .unwrap(),
            ..prepared(folder.path())
        };
        let mut out = Vec::new();
        let code = run_check(&gateway, &prepared, &mut out).await;
        let out = String::from_utf8(out).unwrap();
        assert_eq!(
            code, 1,
            "one kms row on the Logs Insights side, two in the app: {out}"
        );
        // limit 3 keeps both kms events and one s3 event; the tie is s3.
        write_export(
            folder.path(),
            "q3-tie.json",
            r#"{"status":"Complete","results":[
                [{"field":"eventSource","value":"kms.amazonaws.com"}],
                [{"field":"eventSource","value":"kms.amazonaws.com"}],
                [{"field":"eventSource","value":"s3.amazonaws.com"}]]}"#,
        );
        let mut out = Vec::new();
        let code = run_check(&gateway, &prepared, &mut out).await;
        let out = String::from_utf8(out).unwrap();
        assert_eq!(code, 0, "{out}");
        assert!(
            out.contains("q3-tie  Same  insights=3  app=3  tied-at-cut=1/1  boundary-events"),
            "{out}"
        );
    }

    #[test]
    fn prepare_refuses_a_folder_inside_the_repository_and_a_recent_end() {
        let repository = repository_root(Path::new(env!("CARGO_MANIFEST_DIR")));
        let args = |folder: &Path| CheckArgs {
            export_dir: folder.to_path_buf(),
            queries: PathBuf::from(DEFAULT_QUERIES),
        };
        let settings_json = format!(
            r#"{{"profile":"dev","region":"us-east-1","logGroup":"CloudTrail",
                "startSeconds":{START},"endSeconds":{END},"exportedAtSeconds":{EXPORTED_AT}}}"#
        );
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), SETTINGS_FILE, &settings_json);
        let ready = prepare(&args(outside.path()), &repository, END + 1_800).unwrap();
        assert_eq!(ready.settings, settings());
        assert_eq!(ready.queries.len(), 20);
        let too_soon = prepare(&args(outside.path()), &repository, END + 1_799).unwrap_err();
        assert!(too_soon.contains("30 minutes"), "{too_soon}");
        // A temporary folder inside the crate: removed when the test ends.
        let inside = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        write(inside.path(), SETTINGS_FILE, &settings_json);
        let refused = prepare(&args(inside.path()), &repository, END + 1_800).unwrap_err();
        assert!(refused.contains("inside the repository"), "{refused}");
    }

    /// scripts/compat/insights-export.sh with a fake `aws` first on PATH
    /// (team.md Testing Posture: never the real CLI). The script is run by
    /// hand on a Mac, so these run on Unix runners only, not on Windows.
    #[cfg(unix)]
    mod script {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command as Process, Output};

        const SCRIPT: &str = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/compat/insights-export.sh"
        );

        /// Writes down every call; refuses a query with `stats` as the
        /// console does, with a request ID; fails start-query as expired
        /// credentials (`expired`) or a denied access (`denied`) would
        /// (code review R-01); prints a warning on standard error and still
        /// succeeds for `warn` (code review R-04); answers every other query
        /// with one Complete row.
        const FAKE_AWS: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_AWS_LOG"
case "$2" in
  start-query)
    case "$*" in
      *stats*)
        echo "An error occurred (MalformedQueryException) when calling the StartQuery operation: Query cannot be compiled (RequestId: 0a1b2c3d-1111-2222-3333-444455556666)" >&2
        exit 254 ;;
      *expired*)
        echo "An error occurred (ExpiredTokenException) when calling the StartQuery operation: The security token included in the request is expired" >&2
        exit 254 ;;
      *denied*)
        echo "An error occurred (AccessDeniedException) when calling the StartQuery operation: User: arn:aws:sts::123456789012:assumed-role/dev/me is not authorized to perform: logs:StartQuery (RequestId: 9f8e7d6c-1111-2222-3333-444455556666)" >&2
        exit 254 ;;
      *warn*)
        echo "warning: this version of the AWS CLI is old; please update" >&2 ;;
    esac
    echo "query-1" ;;
  get-query-results)
    printf '%s' '{"status":"Complete","statistics":{"recordsMatched":1.0},"results":[[{"field":"eventName","value":"Decrypt"},{"field":"@ptr","value":"CmAK"}]]}' ;;
  *)
    echo "unexpected call: $*" >&2
    exit 1 ;;
esac
"#;

        struct Fake {
            folder: tempfile::TempDir,
        }

        impl Fake {
            fn new() -> Self {
                Self::with_queries(
                    "# a note\nq1-ok\tset\tok\tfields eventName\n\
                     q2-refused\tset\trefused\tstats count(*)\n",
                )
            }

            fn with_queries(queries: &str) -> Self {
                let folder = tempfile::tempdir().unwrap();
                let bin = folder.path().join("bin");
                std::fs::create_dir(&bin).unwrap();
                std::fs::create_dir(folder.path().join("tmp")).unwrap();
                let aws = bin.join("aws");
                std::fs::write(&aws, FAKE_AWS).unwrap();
                std::fs::set_permissions(&aws, std::fs::Permissions::from_mode(0o755)).unwrap();
                write(folder.path(), "queries.txt", queries);
                Self { folder }
            }

            fn run(&self, out: &Path, end: i64) -> Output {
                let path = format!(
                    "{}:{}",
                    self.folder.path().join("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                );
                Process::new("bash")
                    .arg(SCRIPT)
                    .env("PATH", path)
                    .env("FAKE_AWS_LOG", self.folder.path().join("calls.log"))
                    .env("TMPDIR", self.folder.path().join("tmp"))
                    .env("PROFILE", "dev")
                    .env("REGION", "us-east-1")
                    .env("GROUP", "CloudTrail")
                    .env("START", START.to_string())
                    .env("END", end.to_string())
                    .env("OUT", out)
                    .env("QUERIES", self.folder.path().join("queries.txt"))
                    .output()
                    .unwrap()
            }

            fn calls(&self) -> String {
                std::fs::read_to_string(self.folder.path().join("calls.log")).unwrap_or_default()
            }
        }

        #[test]
        fn the_script_is_valid_bash() {
            let status = Process::new("bash").arg("-n").arg(SCRIPT).status().unwrap();
            assert!(status.success());
        }

        #[test]
        fn the_script_writes_the_settings_and_one_file_per_query() {
            let fake = Fake::new();
            let out = fake.folder.path().join("export");
            let output = fake.run(&out, END);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "{stdout}{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                stdout.contains("queries: 2  Complete: 1  Error: 1  other: 0"),
                "{stdout}"
            );
            let settings_text = std::fs::read_to_string(out.join(SETTINGS_FILE)).unwrap();
            let written = parse_settings(&settings_text).unwrap();
            let run = written.exported_at_seconds;
            assert!(run.is_some(), "{settings_text}");
            assert_eq!(
                written,
                RunSettings {
                    exported_at_seconds: run,
                    ..settings()
                }
            );
            // Code review R-02: every file of the run carries the stamp of
            // settings.json, so compat_check takes them for this run.
            let ok = std::fs::read_to_string(out.join("q1-ok.json")).unwrap();
            assert_eq!(
                parse_export_of_run(&ok, run).unwrap(),
                InsightsSide::Rows(vec![vec![("eventName".to_string(), "Decrypt".to_string())]])
            );
            let refused = std::fs::read_to_string(out.join("q2-refused.json")).unwrap();
            let InsightsSide::Rejected(message) = parse_export_of_run(&refused, run).unwrap()
            else {
                panic!("{refused}");
            };
            assert!(message.contains("MalformedQueryException"), "{message}");
            assert!(
                !message.contains("0a1b2c3d"),
                "the request ID is removed: {message}"
            );
            // BR1.1: only start-query and get-query-results, with the profile.
            let calls = fake.calls();
            assert!(
                calls
                    .lines()
                    .all(|call| call.starts_with("logs start-query ")
                        || call.starts_with("logs get-query-results ")),
                "{calls}"
            );
            assert!(
                calls.lines().all(|call| call.contains("--profile dev")),
                "{calls}"
            );
            assert!(
                calls.contains(&format!("--start-time {START} --end-time {END}")),
                "{calls}"
            );
        }

        #[test]
        fn the_script_stops_for_a_folder_inside_the_repository_or_a_recent_end() {
            let fake = Fake::new();
            let inside = Path::new(env!("CARGO_MANIFEST_DIR")).join("insights-export-never-made");
            let output = fake.run(&inside, END);
            assert_eq!(output.status.code(), Some(2));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("inside the repository"),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!inside.exists(), "nothing is created inside the repository");
            let out = fake.folder.path().join("export");
            let recent = fake.run(&out, now_epoch_seconds() - 60);
            assert_eq!(recent.status.code(), Some(2));
            assert!(String::from_utf8_lossy(&recent.stderr).contains("30 minutes"));
            assert!(
                fake.calls().is_empty(),
                "nothing is run with wrong settings"
            );
        }

        /// Code review R-03, R-04: every query is started with `--limit
        /// 10000`, and a warning start-query prints on standard error while
        /// it succeeds does not spoil the query ID.
        #[test]
        fn the_script_asks_for_10000_rows_and_a_warning_does_not_spoil_the_query_id() {
            let fake = Fake::with_queries(
                "q1-warn\tset\twarn\tfields warn\nq2-quiet\tset\tquiet\tfields eventName\n",
            );
            let out = fake.folder.path().join("export");
            let output = fake.run(&out, END);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "{stdout}{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                stdout.contains("queries: 2  Complete: 2  Error: 0  other: 0"),
                "{stdout}"
            );
            let calls = fake.calls();
            let starts: Vec<&str> = calls
                .lines()
                .filter(|call| call.starts_with("logs start-query "))
                .collect();
            assert_eq!(starts.len(), 2, "{calls}");
            assert!(
                starts.iter().all(|call| call.contains("--limit 10000")),
                "{calls}"
            );
            // The query ID is standard output only.
            assert!(
                calls
                    .lines()
                    .filter(|call| call.starts_with("logs get-query-results "))
                    .all(|call| call.contains("--query-id query-1 ")),
                "{calls}"
            );
            let settings_text = std::fs::read_to_string(out.join(SETTINGS_FILE)).unwrap();
            let run = parse_settings(&settings_text).unwrap().exported_at_seconds;
            let warned = std::fs::read_to_string(out.join("q1-warn.json")).unwrap();
            assert_eq!(
                parse_export_of_run(&warned, run).unwrap(),
                InsightsSide::Rows(vec![vec![("eventName".to_string(), "Decrypt".to_string())]]),
                "{warned}"
            );
            // The file that held standard error is removed at the end.
            let left: Vec<_> = std::fs::read_dir(fake.folder.path().join("tmp"))
                .unwrap()
                .collect();
            assert!(left.is_empty(), "{left:?}");
        }

        /// Code review R-01: start-query failing for expired credentials or
        /// a denied access is not Logs Insights refusing the query. It is
        /// written as Unknown, counted as not Complete, and compat_check
        /// does not compare it; only a malformed query is an Error.
        #[tokio::test]
        async fn a_start_query_failure_other_than_a_malformed_query_is_not_a_refusal() {
            let queries = "q1-expired\tset\texpired\tfields expired\n\
                           q2-denied\tset\tdenied\tfields denied\n\
                           q3-refused\tset\trefused\tstats count(*)\n";
            let fake = Fake::with_queries(queries);
            let out = fake.folder.path().join("export");
            let output = fake.run(&out, END);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert!(
                stdout.contains("queries: 3  Complete: 0  Error: 1  other: 2"),
                "{stdout}"
            );
            assert!(
                stdout.contains("[q1-expired] Unknown (start-query failed)"),
                "{stdout}"
            );
            let settings_text = std::fs::read_to_string(out.join(SETTINGS_FILE)).unwrap();
            let run = parse_settings(&settings_text).unwrap().exported_at_seconds;
            for (id, kind, request_id) in [
                ("q1-expired", "ExpiredTokenException", None),
                ("q2-denied", "AccessDeniedException", Some("9f8e7d6c")),
            ] {
                let text = std::fs::read_to_string(out.join(format!("{id}.json"))).unwrap();
                assert_eq!(
                    parse_export_of_run(&text, run).unwrap(),
                    InsightsSide::NotRun("Unknown".to_string()),
                    "{text}"
                );
                let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                let message = json["message"].as_str().unwrap_or_default();
                assert!(message.contains(kind), "{text}");
                if let Some(request_id) = request_id {
                    assert!(!message.contains(request_id), "{text}");
                }
            }
            let refused = std::fs::read_to_string(out.join("q3-refused.json")).unwrap();
            assert!(
                matches!(
                    parse_export_of_run(&refused, run).unwrap(),
                    InsightsSide::Rejected(_)
                ),
                "{refused}"
            );
            // compat_check on that export: the two not run are NotCompared,
            // so the run could not compare (exit 2, review R-02).
            let prepared = Prepared {
                settings: parse_settings(&settings_text).unwrap(),
                queries: parse_query_set(queries).unwrap(),
                export_dir: out.clone(),
            };
            let mut report = Vec::new();
            let code = run_check(&FakeLogs::cloudtrail(), &prepared, &mut report).await;
            let report = String::from_utf8(report).unwrap();
            assert_eq!(code, 2, "{report}");
            assert!(report.contains("q1-expired  NotCompared"), "{report}");
            assert!(report.contains("q2-denied  NotCompared"), "{report}");
            assert!(
                report.contains("ended Unknown") && !report.contains("another run"),
                "not compared because the query did not run: {report}"
            );
        }
    }

    #[test]
    fn the_export_folder_is_required_and_the_query_set_can_be_chosen() {
        let args = |list: &[&str]| {
            list.iter()
                .map(|arg| (*arg).to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            parse_args(args(&[]).into_iter()).unwrap_err(),
            "--export-dir is required"
        );
        let parsed =
            parse_args(args(&["--export-dir", "/x", "--queries", "/q.txt"]).into_iter()).unwrap();
        assert_eq!(parsed.export_dir, PathBuf::from("/x"));
        assert_eq!(parsed.queries, PathBuf::from("/q.txt"));
        let default = parse_args(args(&["--export-dir", "/x"]).into_iter()).unwrap();
        assert_eq!(default.queries, PathBuf::from(DEFAULT_QUERIES));
        assert!(parse_args(args(&["--log-group", "g"]).into_iter()).is_err());
    }
}
