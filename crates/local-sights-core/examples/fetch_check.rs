//! fetch_check: developer-only check of local-sights against real AWS.
//!
//! Uses the same shared validation (U1:BR1.8) and FetchCoordinator as the
//! desktop app, with the AWS SDK gateway, and fetches the whole log group:
//! every stream that matters for the range (U3:BR6.9). Run it on your own
//! machine with your own AWS credentials (team.md Walking Skeleton). It is
//! never run by `cargo test` or CI (U1:BR7.2) and is not part of the
//! distributed app.
//!
//! ```text
//! cargo run -p local-sights-core --example fetch_check -- \
//!   [--profile NAME] --region CODE --log-group NAME \
//!   --start "yyyy/mm/dd hh:mm:ss" --end "yyyy/mm/dd hh:mm:ss"
//! ```
//!
//! Connection (U2, review R-11): without `--profile` the fetch uses the
//! "default settings" profile (`ProfileSelector::SdkDefault`); with
//! `--profile NAME` it uses that named profile (`ProfileSelector::Named`).
//!
//! Region (261012-parallel-fetch U1, functional-spec 5, NFR design review
//! R-03): `--region` is required. It becomes the fetch's region, which sets
//! both the region the SDK connects to (U2:BR2.8) and the per-region ceiling
//! of `GetLogEvents` calls (U1:BR1.1, BR1.2): without it the ceiling would
//! fall back to 5 calls a second and the check would understate the speed.
//! Because the argument is also the SDK's region, `AWS_REGION` and the
//! profile's region do not apply to this version, so the two cannot
//! disagree.
//!
//! Standard output: one event per line, in time order (U3:BR4.1),
//! `UTC time<TAB>stream name<TAB>message on one line`.
//! Standard error: the one-line fetch summary written by the coordinator
//! (`local-sights: fetch summary streams=… calls=… throttled=… elapsed_ms=…
//! max_in_flight=… min_rate=… status=…`, U1:NFR2.2, NFR2.3, U2:BR2.7), then
//! the event, stream and failure counts, then every failed stream (or the
//! listing failure) with its kind and safe detail.
//! Exit codes: 0 success, 1 something failed, 2 invalid arguments.
//!
//! A query (261017-insights-query U1, team.md Walking Skeleton 2): with
//! `--query TEXT` the range is fetched through the query screen's own flow
//! instead, into a QueryStore by the QueryRunner (the same FetchCoordinator,
//! U1:BR2.2), and the query is read and evaluated on it as the app does.
//! Date texts compared with `@timestamp` are read as UTC, like `--start` and
//! `--end`. Standard output is then `matched N of M events, showing S`
//! (with ` (cut at C)` when the result was cut at the row limit, U2:BR6.3),
//! a `columns:` line with the result's column names (U2:BR7.1, from
//! `fields` or the default columns), and the cells of the first
//! [`FIRST_ROWS`] result rows, separated by tabs, in the result's order
//! (`sort`, `limit` and JSON fields apply as in the app); standard error has
//! the fetch summary and counts as without a query. A query that cannot be
//! read is an argument error (exit 2) naming its line, column and message
//! key; nothing is fetched then (U1:BR3.9).
//!
//! ```text
//! cargo run -p local-sights-core --example fetch_check -- --profile NAME \
//!   --region us-east-1 --log-group NAME --start "2025/08/24 16:13:26" \
//!   --end "2025/08/24 17:13:26" --query 'filter @timestamp > 1756053806000'
//! ```
//!
//! Comparing with the version on `main` by hand (U1:NFR2.1, review R-03):
//! use the same profile, log group and time range for both, and run each
//! three times under `time`, comparing the median real time and the event
//! count. Build first (`cargo build --release -p local-sights-core --example
//! fetch_check`) so the build is not timed. `time` comes first and the
//! region goes through `env`, because zsh's `time` is a reserved word and
//! cannot follow a `NAME=value` prefix.
//!
//! ```text
//! # main (no --region argument there; the region comes from the environment)
//! time env AWS_REGION=ap-northeast-1 cargo run --release -p local-sights-core \
//!   --example fetch_check -- --profile NAME --log-group NAME --start ... --end ...
//! # this version: the same region in the environment and in --region
//! time env AWS_REGION=ap-northeast-1 cargo run --release -p local-sights-core \
//!   --example fetch_check -- --profile NAME --region ap-northeast-1 \
//!   --log-group NAME --start ... --end ...
//! ```

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

use chrono_tz::Tz;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::query::evaluator::{QueryResult, row_cells};
use local_sights_core::query::fields::EventFields;
use local_sights_core::query::instant::resolve_date_texts;
use local_sights_core::query::parser::parse_query;
use local_sights_core::query::plan::{FieldRef, QueryPlan};
use local_sights_core::query::runner::{QueryRun, QuerySink, run_query};
use local_sights_core::query::store::{HeldRows, HoldingKey, QueryStore};
use local_sights_core::request::{
    FetchInput, ValidatedFetch, ValidationError, validate_fetch_input,
};
use local_sights_core::retry::{RandomJitter, Retrier, RetryPolicy, StopSignal};
use local_sights_core::streams::StreamListingStatus;
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::time_range::format_utc_millis;
use local_sights_core::timeline::EventTimeline;

const USAGE: &str = "usage: fetch_check [--profile NAME] --region CODE --log-group NAME \
--start \"yyyy/mm/dd hh:mm:ss\" --end \"yyyy/mm/dd hh:mm:ss\" [--query TEXT]   (times in UTC)";

const EXIT_FETCH_FAILED: u8 = 1;
const EXIT_USAGE: u8 = 2;

/// Matching events printed with `--query`.
const FIRST_ROWS: usize = 10;

/// The parsed arguments: the raw fetch input, the region code and the
/// query, when one was given.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckArgs {
    input: FetchInput,
    region: String,
    query: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => return usage_error(&[message]),
    };
    // The shared validation maps a missing or blank --profile to SdkDefault
    // and a name to Named (R-11); the region is then set from --region.
    let validated = match validate_fetch_input(&args.input) {
        Ok(validated) => {
            let profile = validated.request.profile().clone();
            ValidatedFetch {
                request: validated.request.with_connection(profile, args.region),
                range: validated.range,
            }
        }
        Err(errors) => {
            let reasons: Vec<String> = errors.iter().map(|e| describe(*e).to_string()).collect();
            return usage_error(&reasons);
        }
    };

    let gateway = AwsCloudWatchLogsGateway::new();
    let policy = RetryPolicy::default();
    let jitter = RandomJitter;
    let stop = StopSignal::never();
    let retrier = Retrier::new(&policy, &jitter, &stop);
    if let Some(text) = &args.query {
        // 261017-insights-query U1:BR3.9: read before anything is fetched.
        let plan = match read_query(text) {
            Ok(plan) => plan,
            Err(reason) => return usage_error(&[reason]),
        };
        return check_query(&gateway, validated, plan, &retrier).await;
    }
    let timeline = Mutex::new(EventTimeline::new());
    let mut sink = ProgressSink;
    let job = run_fetch(&gateway, &validated, &timeline, &retrier, &mut sink).await;

    let timeline = timeline.lock().unwrap_or_else(PoisonError::into_inner);
    if let Err(error) = write_events(timeline.events()) {
        eprintln!("error: could not write to standard output: {error}");
        return ExitCode::from(EXIT_FETCH_FAILED);
    }
    report(&job)
}

/// Reads the query and turns its date texts into milliseconds in UTC, or
/// says where it cannot be read (only the query's place and message key).
fn read_query(text: &str) -> Result<QueryPlan, String> {
    parse_query(text)
        .and_then(|plan| resolve_date_texts(plan, Tz::UTC))
        .map_err(|problem| {
            format!(
                "--query cannot be read at line {}, column {}: {:?} {}{}",
                problem.span.line,
                problem.span.column,
                problem.kind,
                problem.message_key,
                problem
                    .word
                    .map(|word| format!(" ({word})"))
                    .unwrap_or_default()
            )
        })
}

/// 261017-insights-query U1: fetches the range into a QueryStore with the
/// QueryRunner, evaluates the query and prints the counts, the columns and
/// the first result rows (U2).
async fn check_query(
    gateway: &AwsCloudWatchLogsGateway,
    validated: ValidatedFetch,
    plan: QueryPlan,
    retrier: &Retrier<'_>,
) -> ExitCode {
    let key = HoldingKey {
        profile: validated.request.profile().clone(),
        region: validated.request.region().unwrap_or_default().to_string(),
        log_group_name: validated.request.log_group_name().to_string(),
    };
    let run = QueryRun {
        run_id: 1,
        plan: Arc::new(plan),
        fetch: validated,
        key,
        cache: None,
        fetch_started_at_ms: 0,
    };
    let store = Arc::new(QueryStore::new());
    let mut sink = QueryCheckSink::default();
    run_query(gateway, &run, &store, retrier, &mut sink).await;
    let Some(job) = sink.ended else {
        eprintln!("error: the fetch ended without a result");
        return ExitCode::from(EXIT_FETCH_FAILED);
    };
    let code = report(&job);
    let Some(result) = sink.result else {
        eprintln!("error: the query was not evaluated");
        return ExitCode::from(EXIT_FETCH_FAILED);
    };
    let positions = result.window(result.run_id, result.holdings_epoch, 0, FIRST_ROWS);
    let first = store.rows_at(result.holdings_epoch, positions);
    if let Err(error) = write_query_result(&result, first.as_ref()) {
        eprintln!("error: could not write to standard output: {error}");
        return ExitCode::from(EXIT_FETCH_FAILED);
    }
    code
}

/// 261017-insights-query U2:BR6.1, BR6.3, BR7.1: the counts (matched,
/// shown and, when the result was cut, where), the columns, then the cells
/// of the first rows, one row a line, separated by tabs. A cell without a
/// value is empty; newlines in a value become spaces.
fn write_query_result(result: &QueryResult, first: Option<&HeldRows>) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    write!(
        out,
        "matched {} of {} events, showing {}",
        result.matched_count(),
        result.evaluated_count,
        result.row_count()
    )?;
    if let Some(cut) = result.truncated_at() {
        write!(out, " (cut at {cut})")?;
    }
    writeln!(out)?;
    let names: Vec<&str> = result.columns().iter().map(FieldRef::name).collect();
    writeln!(out, "columns: {}", names.join("\t"))?;
    if let Some(rows) = first {
        for event in &rows.events {
            let fields = EventFields::new(event, &rows.log_group_name);
            let cells = row_cells(result.columns(), &fields, &format_utc_millis);
            let line: Vec<String> = cells
                .into_iter()
                .map(|cell| one_line(&cell.unwrap_or_default()))
                .collect();
            writeln!(out, "{}", line.join("\t"))?;
        }
    }
    out.flush()
}

/// The value on one line: every line break becomes a space.
fn one_line(value: &str) -> String {
    value.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

/// Prints the counts and the failures to standard error and chooses the
/// exit code. Only safe details are printed (U1:BR4.3).
fn report(job: &FetchJob) -> ExitCode {
    eprintln!(
        "events: {}  streams: {}  failed streams: {}",
        job.event_count,
        job.planned_stream_count,
        job.failed_streams.len()
    );
    if job.listing_status == Some(StreamListingStatus::Partial) {
        match &job.listing_failure {
            Some(failure) => eprintln!(
                "error: stream listing stopped part way: {} ({})",
                failure.kind().as_str(),
                failure.safe_detail()
            ),
            None => eprintln!("error: stream listing stopped part way"),
        }
    }
    for failed in &job.failed_streams {
        eprintln!(
            "error: stream {}: {} ({})",
            failed.log_stream_name,
            failed.failure.kind().as_str(),
            failed.failure.safe_detail()
        );
    }
    match job.status {
        JobStatus::Completed => ExitCode::SUCCESS,
        // 261012-parallel-fetch U5:BR5.14: this check never sends [Cancel]
        // (its stop signal never fires), so a Cancelled job is not expected;
        // it would still not be a complete fetch.
        JobStatus::Cancelled => {
            eprintln!(
                "error: the fetch was cancelled; {} streams were not read to their end",
                job.unfinished_stream_count
            );
            ExitCode::from(EXIT_FETCH_FAILED)
        }
        JobStatus::Running
        | JobStatus::Failed
        | JobStatus::CompletedWithFailures
        | JobStatus::Aborted => ExitCode::from(EXIT_FETCH_FAILED),
    }
}

fn usage_error(reasons: &[String]) -> ExitCode {
    eprintln!("{USAGE}");
    for reason in reasons {
        eprintln!("  - {reason}");
    }
    ExitCode::from(EXIT_USAGE)
}

/// Parses `--name value` pairs into the raw input and the region. The
/// region is required and must look like a region code (lower-case letters,
/// digits and hyphens, e.g. `ap-northeast-1`).
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<CheckArgs, String> {
    let mut input = FetchInput::default();
    let mut region = String::new();
    let mut query: Option<String> = None;
    while let Some(flag) = args.next() {
        if flag == "--query" {
            let text = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            query = Some(text);
            continue;
        }
        let slot = match flag.as_str() {
            "--profile" => &mut input.profile_name,
            "--region" => &mut region,
            "--log-group" => &mut input.log_group_name,
            "--start" => &mut input.start_text,
            "--end" => &mut input.end_text,
            "-h" | "--help" => return Err("help requested".to_string()),
            other => return Err(format!("unknown argument: {other}")),
        };
        *slot = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
    }
    let region = region.trim().to_string();
    if region.is_empty() {
        return Err("--region is required (e.g. --region ap-northeast-1)".to_string());
    }
    let looks_like_a_region = region
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !looks_like_a_region {
        return Err("--region must be a region code such as ap-northeast-1".to_string());
    }
    Ok(CheckArgs {
        input,
        region,
        query,
    })
}

/// English description of a validation reason for the terminal.
fn describe(error: ValidationError) -> &'static str {
    match error {
        ValidationError::LogGroupRequired => "--log-group is required",
        ValidationError::LogGroupTooLong => "--log-group must be 512 characters or fewer",
        ValidationError::StartFormat => "--start must be a valid yyyy/mm/dd hh:mm:ss (UTC)",
        ValidationError::EndFormat => "--end must be a valid yyyy/mm/dd hh:mm:ss (UTC)",
        // The arguments are always UTC (U4:BR3.5), which skips no local
        // time; these reasons only arise for the screen's local zone.
        ValidationError::StartNonexistentLocalTime => "--start does not exist in local time",
        ValidationError::EndNonexistentLocalTime => "--end does not exist in local time",
        ValidationError::RangeOrder => "--start must be before --end",
    }
}

/// Writes every event, in timeline order, one per line.
fn write_events(events: &[LogEvent]) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for event in events {
        let message = one_line(&event.message);
        writeln!(
            out,
            "{}\t{}\t{message}",
            format_utc_millis(event.timestamp),
            event.log_stream_name
        )?;
    }
    out.flush()
}

/// Shows that the check is working, on standard error; the events are
/// written in time order once every stream has been read.
struct ProgressSink;

impl FetchSink for ProgressSink {
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

/// The progress of a `--query` check, and what it ended with.
#[derive(Default)]
struct QueryCheckSink {
    ended: Option<FetchJob>,
    result: Option<QueryResult>,
}

impl FetchSink for QueryCheckSink {
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64) {
        ProgressSink.on_started(job, timeline_version);
    }

    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}

    fn on_planned(&mut self, job: &FetchJob) {
        ProgressSink.on_planned(job);
    }

    fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}

    fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {}

    fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {}
}

impl QuerySink for QueryCheckSink {
    fn on_fetch_ended(&mut self, _run_id: u64, job: &FetchJob) -> bool {
        self.ended = Some(job.clone());
        eprintln!("evaluating {} events...", job.event_count);
        true
    }

    fn on_evaluation_progress(&mut self, _run_id: u64, _evaluated: usize, _total: usize) -> bool {
        true
    }

    fn on_evaluated(&mut self, result: QueryResult) {
        self.result = Some(result);
    }

    fn on_evaluation_stopped(&mut self, _run_id: u64) {}

    fn on_field_names(&mut self, _run_id: u64, _names: Vec<String>) {}

    fn on_evaluation_failed(&mut self, _run_id: u64) {}
}

/// Argument parsing only; nothing here connects to AWS (`cargo test -p
/// local-sights-core --example fetch_check`).
#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    const BASE: [&str; 6] = [
        "--log-group",
        "/aws/lambda/orders",
        "--start",
        "2024/01/02 03:04:05",
        "--end",
        "2024/01/02 03:05:05",
    ];

    #[test]
    fn without_region_the_arguments_are_a_usage_error() {
        let error = parse_args(args(&BASE)).unwrap_err();
        assert!(error.contains("--region is required"), "{error}");
        let blank: Vec<&str> = BASE.iter().copied().chain(["--region", "  "]).collect();
        assert!(
            parse_args(args(&blank))
                .unwrap_err()
                .contains("--region is required")
        );
    }

    #[test]
    fn the_region_is_taken_with_the_other_arguments() {
        let with_region: Vec<&str> = ["--profile", "dev", "--region", " ap-northeast-1 "]
            .into_iter()
            .chain(BASE)
            .collect();
        let parsed = parse_args(args(&with_region)).unwrap();
        assert_eq!(parsed.region, "ap-northeast-1");
        assert_eq!(parsed.input.profile_name, "dev");
        assert_eq!(parsed.input.log_group_name, "/aws/lambda/orders");
        assert_eq!(parsed.input.start_text, "2024/01/02 03:04:05");
        assert_eq!(parsed.input.end_text, "2024/01/02 03:05:05");
    }

    #[test]
    fn a_region_that_is_not_a_region_code_or_has_no_value_is_refused() {
        let odd: Vec<&str> = BASE
            .iter()
            .copied()
            .chain(["--region", "Tokyo 1"])
            .collect();
        let error = parse_args(args(&odd)).unwrap_err();
        assert!(error.contains("region code"), "{error}");
        let missing: Vec<&str> = BASE.iter().copied().chain(["--region"]).collect();
        assert_eq!(
            parse_args(args(&missing)).unwrap_err(),
            "missing value for --region"
        );
    }

    // ---- 261017-insights-query U1: --query -----------------------------

    #[test]
    fn a_query_is_taken_as_written_and_is_optional() {
        let plain: Vec<&str> = BASE
            .iter()
            .copied()
            .chain(["--region", "us-east-1"])
            .collect();
        assert_eq!(parse_args(args(&plain)).unwrap().query, None);
        let with_query: Vec<&str> = plain
            .iter()
            .copied()
            .chain(["--query", "filter @timestamp > 1756053806000"])
            .collect();
        assert_eq!(
            parse_args(args(&with_query)).unwrap().query.as_deref(),
            Some("filter @timestamp > 1756053806000")
        );
        let missing: Vec<&str> = plain.iter().copied().chain(["--query"]).collect();
        assert_eq!(
            parse_args(args(&missing)).unwrap_err(),
            "missing value for --query"
        );
    }

    #[test]
    fn a_query_that_cannot_be_read_names_its_place_and_reads_dates_as_utc() {
        let error = read_query("filter @message like /a/\n| stats count(*)").unwrap_err();
        assert!(error.contains("line 2, column 3"), "{error}");
        assert!(
            error.contains("query.unsupported.command (stats)"),
            "{error}"
        );
        assert!(read_query("filter @timestamp > '2025-08-24 16:43:26'").is_ok());
        assert!(read_query("filter @timestamp > 'yesterday'").is_err());
    }

    // ---- 261017-insights-query U2: fields, sort, limit, JSON -----------

    #[test]
    fn a_query_with_fields_sort_limit_and_json_fields_is_read() {
        let plan = read_query(
            "fields @timestamp, level, http.status\n| filter level = 'ERROR'\n\
             | sort http.status desc\n| limit 5",
        )
        .unwrap();
        let columns = plan.columns();
        let names: Vec<&str> = columns.iter().map(FieldRef::name).collect();
        assert_eq!(names, ["@timestamp", "level", "http.status"]);
        assert!(plan.has_sort());
        assert!(plan.has_row_limit());
    }

    #[test]
    fn a_value_is_written_on_one_line() {
        assert_eq!(one_line("a\r\nb\nc\rd"), "a b c d");
        assert_eq!(one_line(""), "");
    }
}
