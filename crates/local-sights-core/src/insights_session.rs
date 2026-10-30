//! InsightsSession: the Log Analytics screen's input and the state of its
//! last query run (261017-insights-query U1, entities.md AnalyticsInput,
//! InsightsRun).
//!
//! The input (one log group, the time range, the query text) is held apart
//! from the Log Management screen's and stays when the screens are switched
//! (U1:BR1.2). Only the time zone is shared; AppSession passes it in. A run
//! goes through the phases of functional-spec 4: Idle → Fetching →
//! Evaluating → Done, or QueryError (nothing fetched), FetchFailed,
//! Cancelled or, since 261017-insights-query U2, EvaluationFailed (U2:BR10.2).
//! Reading the query is one call and leaves no phase of its own. A [Cancel]
//! that comes after the last evaluation step still ends the run Cancelled
//! without a result (U2:BR10.1).
//!
//! Since U2 the result carries its columns, the matched count and the cut
//! (U2:BR3.2-BR3.4, BR5.3); the rows give their cells for those columns and
//! one row opened gives every field (U2:BR7.1, BR7.3). The JSON field names
//! of the last fetch are kept for completion until the connection changes
//! (U2:BR1.7, BR9.1, Should).
//!
//! Every notice of a fetch or an evaluation carries the run number; a notice
//! of another run is dropped, so only the last run started is shown
//! (U1:BR6.2). Whether [Run query] can be pressed is decided here from the
//! input and what AppSession says about the rest of the app, in the order
//! of U1:BR1.5. The input has a revision, incremented by every change, so a
//! run whose query was read while the input changed is refused (R-04).
//!
//! Since 261017-insights-query U3 up to [`MAX_LOG_GROUPS`] log groups are
//! chosen, in the order chosen (U3:BR4.1, BR4.2). A run's progress counts
//! the log groups and the streams of the current one, and says when the
//! disk cache is read (U3:BR2.7, review R-04); its failed streams carry
//! their log group (U3:BR2.6). The previous result stays shown while the
//! next run fetches and evaluates, and goes when that run's fetch or
//! evaluation fails (U3:BR3.4); its rows are copies, read here without the
//! QueryStore (U3:BR3.2). The completion names go when a run starts and are
//! gathered again from what it fetched (U3:BR5.2).
//!
//! Since 261022-small-features U2 the shown result can be exported
//! (U2:BR3.1, BR3.2): [`InsightsSession::can_export`] says when, and
//! [`InsightsSession::export_source`] copies the result's columns and rows
//! for the export, whose text is made outside the session lock
//! (functional-design review R-01). The run remembers the log groups chosen
//! when it started, so the default file name is made from the first of them
//! (review R-06).

use std::time::Instant;

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::catalog::ProfileSelector;
use crate::coordinator::{BatchProgress, FetchJob};
use crate::date_input::DateTimeInput;
use crate::event::LogEvent;
use crate::export::exporter::{ExportAvailability, ExportBlocker, ExportSnapshotSource};
use crate::failure::ApiFailure;
use crate::log_groups::ListingStatus;
use crate::query::catalog::{BIN, QueryCatalog, TO_MILLIS, VocabularyKind};
use crate::query::evaluator::{QueryResult, ResultRow, result_row_cells, row_detail};
use crate::query::plan::{QueryPlan, QueryProblem, ResultShape};
use crate::query::runner::{FetchEnd, GroupFailedStream, GroupFetchStatus, QueryRunSummary};
use crate::query::stats::key_text;
use crate::query::store::HoldingKey;
use crate::request::{ValidationError, message_keys, validate_session_input};
use crate::session::{ElapsedView, SessionError};
use crate::streams::planner::ListingProgress;
use crate::time_range::{TimeRange, format_display_in_zone};

/// Most log groups one query runs on: the console's limit (ADR-006,
/// 261017-insights-query U3:BR4.1).
pub const MAX_LOG_GROUPS: usize = 50;

/// Longest query text, in Unicode scalar values: the console's limit
/// (StartQuery's queryString, functional-design Q4).
pub const MAX_QUERY_CHARS: usize = 10_000;

/// The query the screen opens with (entities.md AnalyticsInput.queryText):
/// the console's default columns, newest first, with U1's filter of errors.
pub const DEFAULT_QUERY: &str = "fields @timestamp, @message, @logStream, @log\n\
     | filter @message like /ERROR/\n\
     | sort @timestamp desc\n\
     | limit 10000";

/// The fixed words of completion, all from the QueryCatalog
/// (261025-insights-stats U2:BR4.1, BR4.3): the commands this version runs,
/// the keywords, the fixed fields with `toMillis(@timestamp)` (its text as
/// U2 of 261017-insights-query offered it), and the aggregate functions and
/// `bin`.
fn catalog_candidates() -> Vec<CompletionCandidate> {
    let candidate = |text: &str, kind| CompletionCandidate {
        text: text.to_string(),
        kind,
    };
    let names = |kind, supported| QueryCatalog::names(kind, supported).into_iter();
    let mut candidates: Vec<CompletionCandidate> = names(VocabularyKind::Command, true)
        .map(|word| candidate(word, CompletionKind::Command))
        .collect();
    candidates.extend(
        names(VocabularyKind::Keyword, true).map(|word| candidate(word, CompletionKind::Keyword)),
    );
    candidates.extend(
        names(VocabularyKind::FixedField, true)
            .map(|word| candidate(word, CompletionKind::FixedField)),
    );
    candidates.push(candidate(
        &format!("{TO_MILLIS}(@timestamp)"),
        CompletionKind::FixedField,
    ));
    candidates.extend(
        names(VocabularyKind::AggregateFunction, true)
            .chain(std::iter::once(BIN))
            .map(|word| candidate(word, CompletionKind::Function)),
    );
    candidates
}

/// Phase of the last run (functional-spec 4, entities.md InsightsRun.phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum AnalyticsPhase {
    /// Nothing run yet, or the connection was changed.
    #[default]
    Idle,
    /// Fetching the whole range into the query holdings.
    Fetching,
    /// Evaluating the held events.
    Evaluating,
    /// The result is shown.
    Done,
    /// The query could not be read; nothing was fetched (U1:BR3.9).
    QueryError,
    /// The whole fetch failed (U1:BR2.6).
    FetchFailed,
    /// [Cancel] stopped the run (U1:BR2.7).
    Cancelled,
    /// The evaluation ended abnormally; nothing is shown (U2:BR10.2).
    EvaluationFailed,
    /// [Cancel] was pressed while fetching: the fetch stops, then what was
    /// fetched is evaluated into the cancelled result; running, as Fetching
    /// and Evaluating are (261025-insights-stats U3:BR3.1, BR3.2, BR3.7).
    Cancelling,
}

/// Which result a version is (entities.md ResultVersion.stage,
/// 261025-insights-stats U3:BR4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultStage {
    /// Made while fetching from what was fetched so far (FR4.1).
    Partial,
    /// Made from everything, after the fetch.
    Final,
    /// Made after [Cancel] from what was fetched until then (FR5.1).
    CancelledPartial,
}

/// What a partial result holds (entities.md PartialCoverage, U3:BR2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialCoverage {
    /// The log group under fetch, from 1.
    pub group_number: u32,
    /// The chosen log groups.
    pub group_count: u32,
    /// Events of the partial view evaluated.
    pub included_event_count: u64,
}

/// One version of the result of a run (entities.md ResultVersion): partial
/// results and the final one move it on by one (U3:BR2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultVersion {
    /// The run.
    pub run_id: u64,
    /// From 1 in each run.
    pub version: u64,
    /// Partial, final or cancelled.
    pub stage: ResultStage,
    /// What a partial result holds.
    pub coverage: Option<PartialCoverage>,
}

/// What a [Cancel] the session took asks of the run (U3:BR3.1, BR3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelRequest {
    /// Stop the fetch or the evaluation of run `0` (its stop handle, for
    /// Cancel).
    Stop(u64),
    /// The second [Cancel] while cancelling: abandon the evaluation of run
    /// `0` after the fetch (its stop handle's abandon).
    Abandon(u64),
}

/// Joins the parts of a row key (U3:BR4.2).
const ROW_KEY_SEPARATOR: char = '\u{1f}';

/// U3:BR4.2 (Q4 A): what tells a row apart across the versions of a result.
/// An event row: its log group, stream and order in the stream; an
/// aggregate row: the text of its first `key_count` cells (its keys, as
/// told apart by U1:BR2.1, a missing value empty), so the one row without
/// `by` has the empty key. The parts are joined with U+001F.
pub fn row_key(row: &ResultRow, key_count: usize) -> String {
    match row {
        ResultRow::Event {
            event,
            log_group_name,
        } => format!(
            "{log_group_name}{ROW_KEY_SEPARATOR}{}{ROW_KEY_SEPARATOR}{}",
            event.log_stream_name, event.sequence
        ),
        ResultRow::Aggregate { cells } => {
            let parts: Vec<String> = cells
                .iter()
                .take(key_count)
                .map(|cell| key_text(cell).unwrap_or_default())
                .collect();
            parts.join(&ROW_KEY_SEPARATOR.to_string())
        }
    }
}

/// What a completion candidate is (entities.md CompletionCandidate.kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CompletionKind {
    /// `fields`, `filter`, `sort`, `limit`.
    Command,
    /// `and`, `or`, `not`, `like`, `in`, `asc`, `desc`.
    Keyword,
    /// One of the fixed fields, or `toMillis(@timestamp)`.
    FixedField,
    /// A JSON field of the fetched logs.
    JsonField,
    /// An aggregate function of `stats`, or `bin`
    /// (261025-insights-stats U2:BR4.3).
    Function,
}

/// One completion candidate (entities.md CompletionCandidate, U2:BR9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionCandidate {
    /// The text put in the query.
    pub text: String,
    /// What it is.
    pub kind: CompletionKind,
}

/// A field of the Log Analytics screen's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalyticsField {
    /// The log groups (U3:BR4.1): a list of names in the order chosen, or
    /// one name; an empty text or list deselects every one.
    LogGroup,
    /// The start date and time.
    StartText,
    /// The end date and time.
    EndText,
    /// The query text.
    QueryText,
}

/// The value of one field: a text, or the names of the log groups
/// (261017-insights-query U3:BR4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnalyticsValue {
    /// The text of a field; for [`AnalyticsField::LogGroup`] one name, or
    /// nothing when empty.
    Text(String),
    /// The chosen log groups, in the order chosen.
    Names(Vec<String>),
}

impl From<String> for AnalyticsValue {
    fn from(text: String) -> Self {
        AnalyticsValue::Text(text)
    }
}

impl From<Vec<String>> for AnalyticsValue {
    fn from(names: Vec<String>) -> Self {
        AnalyticsValue::Names(names)
    }
}

/// Why [Run query] cannot be pressed, in the order U1:BR1.5 checks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunBlocker {
    /// (1) No connection is selected.
    NotConnected,
    /// (2) No log group is selected.
    LogGroupRequired,
    /// (3) The time range is not valid.
    RangeInvalid,
    /// (4) The query is blank.
    QueryEmpty,
    /// (5) The query is longer than [`MAX_QUERY_CHARS`].
    QueryTooLong,
    /// (6) A query runs on this screen.
    QueryRunning,
    /// (7) The Log Management screen is fetching (U1:BR2.3).
    LogManagementFetching,
    /// (8) A connection change awaits confirmation, the settings dialog is
    /// open, an SSO login runs, or closing awaits an answer.
    OtherOperation,
}

impl RunBlocker {
    /// Message-catalog key of the reason.
    pub fn message_key(self) -> &'static str {
        match self {
            RunBlocker::NotConnected => "insights.blocked.notConnected",
            RunBlocker::LogGroupRequired => "insights.blocked.logGroupRequired",
            RunBlocker::RangeInvalid => "insights.blocked.rangeInvalid",
            RunBlocker::QueryEmpty => "insights.blocked.queryEmpty",
            RunBlocker::QueryTooLong => "insights.blocked.queryTooLong",
            RunBlocker::QueryRunning => "insights.blocked.queryRunning",
            RunBlocker::LogManagementFetching => "insights.blocked.logManagementFetching",
            RunBlocker::OtherOperation => "insights.blocked.otherOperation",
        }
    }

    /// The error a refused run answers with.
    pub fn session_error(self) -> SessionError {
        match self {
            RunBlocker::NotConnected => SessionError::NotConnected,
            RunBlocker::QueryRunning | RunBlocker::LogManagementFetching => SessionError::Busy,
            RunBlocker::OtherOperation => SessionError::ConfirmationPending,
            RunBlocker::LogGroupRequired
            | RunBlocker::RangeInvalid
            | RunBlocker::QueryEmpty
            | RunBlocker::QueryTooLong => SessionError::Invalid,
        }
    }
}

/// What AppSession knows about the rest of the app for U1:BR1.5.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunContext {
    /// The profile and region, when both are selected.
    pub connection: Option<(ProfileSelector, String)>,
    /// (7) The Log Management screen is fetching.
    pub log_management_fetching: bool,
    /// (8) Another operation awaits an answer or runs.
    pub other_operation: bool,
    /// Closing awaits an answer (no [Cancel] then, as on Log Management).
    pub close_pending: bool,
    /// An export is under way (261022-small-features U2:BR3.2, BR4.8).
    pub exporting: bool,
}

/// The query text as it was when [Run query] was pressed, read outside the
/// session lock (functional-design review R-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySnapshot {
    /// The input revision the text belongs to.
    pub revision: u64,
    /// The query text.
    pub text: String,
}

/// What a run that was accepted needs (functional-spec UC1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryStart {
    /// The new run.
    pub run_id: u64,
    /// The period, both ends included.
    pub range: TimeRange,
    /// The chosen log groups of the connection, in the order chosen
    /// (U3:BR4.1).
    pub keys: Vec<HoldingKey>,
}

/// How [Run query] was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryBegin {
    /// The query could not be read: the phase is QueryError and nothing is
    /// fetched (U1:BR3.9).
    Problem,
    /// The fetch starts.
    Fetch(QueryStart),
}

/// Progress of the running query as the screen shows it (U1:BR2.4,
/// U3:BR2.7).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightsProgress {
    /// The disk cache is being read (U3:BR2.7).
    pub reading_cache: bool,
    /// The log group being fetched from AWS, from 1; `None` before the
    /// first (U3:BR2.7).
    pub group_number: Option<u32>,
    /// The chosen log groups.
    pub group_count: u32,
    /// Log groups whose fetch failed so far (review R-04).
    pub failed_group_count: u32,
    /// Streams selected by the listing of the current fetch so far.
    pub selected_stream_count: u32,
    /// Streams to fetch in the current fetch; `None` while they are listed.
    pub planned_stream_count: Option<u32>,
    /// Streams of the current fetch finished.
    pub finished_stream_count: u32,
    /// Events fetched from AWS by the run so far.
    pub event_count: u64,
    /// Events evaluated so far (Evaluating).
    pub evaluated_count: u64,
    /// Events to evaluate (Evaluating).
    pub total_count: u64,
}

/// The result of the last run as the screen shows it; the rows are read with
/// these numbers (U1:BR6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultSummary {
    /// The run that made it; the rows and the details are asked for by it
    /// alone (261020-carryover-fixes FR2.3.1).
    pub run_id: u64,
    /// Event rows or aggregate rows: the table draws the rows by it
    /// (261025-insights-stats BR4.1, ADR-006).
    pub shape: ResultShape,
    /// Result rows (U2:BR7.2).
    pub row_count: u64,
    /// Events evaluated.
    pub evaluated_count: u64,
    /// The column headings, the fields' names as written (U2:BR3.2-BR3.4).
    pub columns: Vec<String>,
    /// Rows that matched before the cut (U2:BR5.3, review R-08).
    pub matched_count: u64,
    /// The rows shown, when a `limit` or the default cut dropped rows that
    /// matched (U2:BR5.3).
    pub truncated_at: Option<u64>,
    /// The version within the run; the rows are read with it
    /// (261025-insights-stats U3:BR2.4, BR4.1).
    pub version: u64,
    /// Partial, final or cancelled (U3:BR4.1).
    pub stage: ResultStage,
    /// What a partial result holds (U3:BR2.6).
    pub coverage: Option<PartialCoverage>,
}

/// One log group whose fetch failed, with why (U3:BR2.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupFailure {
    /// The log group.
    pub log_group_name: String,
    /// Kind and safe detail only.
    pub failure: ApiFailure,
}

/// The Log Analytics screen as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightsView {
    /// The chosen log groups, in the order chosen (U3:BR4.1).
    pub selected_log_group_names: Vec<String>,
    /// The most log groups that can be chosen (U3:BR4.1).
    pub max_log_groups: u32,
    /// Every log group of the current connection's list, ascending, to
    /// choose from: Log Management's list before its own filter (U1:BR1.3).
    /// Filled by AppSession.
    pub log_group_options: Vec<String>,
    /// How that list stands; `None` without a complete connection.
    pub log_group_status: Option<ListingStatus>,
    /// The start input in the chosen zone.
    pub start_input: DateTimeInput,
    /// The end input in the chosen zone.
    pub end_input: DateTimeInput,
    /// The query text as written.
    pub query_text: String,
    /// The time range's reasons (`validation.*` keys) for the fields.
    pub range_errors: Vec<String>,
    /// The first reason why [Run query] cannot be pressed (U1:BR1.5).
    pub run_blocker: Option<String>,
    /// Whether [Run query] can be pressed now.
    pub can_run: bool,
    /// The number of the current or last run; 0 before the first.
    pub run_id: u64,
    /// The phase of that run.
    pub phase: AnalyticsPhase,
    /// Progress while Fetching or Evaluating.
    pub progress: Option<InsightsProgress>,
    /// The reading problem when QueryError (U1:BR3.9).
    pub problem: Option<QueryProblem>,
    /// The failure of the whole fetch when FetchFailed (U1:BR2.6): the
    /// first failed log group's.
    pub failure: Option<ApiFailure>,
    /// Every log group whose fetch failed, when FetchFailed (U3:BR2.6).
    pub failed_groups: Vec<GroupFailure>,
    /// Whether some streams failed: the result is partial (U1:BR2.5).
    pub partial: bool,
    /// The streams that failed, with their log group (U3:BR2.6; names and
    /// safe reasons only).
    pub failed_streams: Vec<GroupFailedStream>,
    /// The listing failure, when the listing ended Partial.
    pub listing_failure: Option<ApiFailure>,
    /// The result, while one is shown.
    pub result: Option<ResultSummary>,
    /// The elapsed time of the run (`fetch_number` is the run number).
    pub elapsed: Option<ElapsedView>,
    /// [Cancel] was pressed and the run has not ended yet.
    pub cancelling: bool,
    /// Whether [Cancel] can be pressed now (as on Log Management: not while
    /// the cache is written, not while closing awaits an answer, R-02).
    pub cancel_available: bool,
    /// Whether the shown result can be exported now (261022-small-features
    /// U2:BR3.2).
    pub can_export: bool,
}

/// One result row as the screen shows it (U1:BR6.3, U2:BR7.1). An
/// aggregate row has its cells only (261025-insights-stats BR4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRow {
    /// The event as held; an event row only.
    #[serde(flatten)]
    pub event: Option<LogEvent>,
    /// `@timestamp` in the chosen zone, as on Log Management; an event row
    /// only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_time: Option<String>,
    /// `@log`; an event row only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_group_name: Option<String>,
    /// One cell per result column, `None` without a value (U2:BR7.1); an
    /// aggregate row's as the table writes them, times in the chosen zone
    /// (BR2.8, BR4.2).
    pub values: Vec<Option<String>>,
    /// What tells the row apart across versions ([`row_key`], U3:BR4.2).
    pub row_key: String,
}

/// One field of an opened row (U2:BR7.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailField {
    /// The field's name, a JSON field as the screen writes it (R-09).
    pub name: String,
    /// Its value as a cell shows it, `None` without one.
    pub value: Option<String>,
}

/// The fields of one opened result row (U2:BR7.3, functional-spec 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRowDetail {
    /// The run asked for.
    pub run_id: u64,
    /// The version asked for (U3:BR2.4).
    pub version: u64,
    /// The row's place in the result.
    pub index: u64,
    /// The fixed fields, then the JSON fields by name; empty when the run
    /// is not the shown one.
    pub fields: Vec<DetailField>,
}

/// The result rows of one viewport request (U1:BR6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRowWindow {
    /// The run asked for.
    pub run_id: u64,
    /// The version asked for (U3:BR2.4).
    pub version: u64,
    /// Position of the first row.
    pub offset: u64,
    /// The rows; empty when the run is not the shown one.
    pub rows: Vec<QueryRow>,
    /// Number of matching events of the shown result.
    pub total_count: u64,
}

/// One run (entities.md InsightsRun).
#[derive(Debug, Clone, Default)]
struct InsightsRun {
    run_id: u64,
    phase: AnalyticsPhase,
    cancel_requested: bool,
    /// The second [Cancel] while Cancelling (U3:BR3.2).
    abandon_requested: bool,
    /// While Cancelling: the fetch ended and the cancelled evaluation is
    /// under way (U3:BR3.1).
    fetch_ended: bool,
    /// The version of `result` (U3:BR2.4): of an earlier run while the
    /// previous result is shown (BR2.5).
    result_version: Option<ResultVersion>,
    job_id: Option<u64>,
    progress: Option<InsightsProgress>,
    /// Events fetched by the run's earlier range fetches (U3:BR2.7).
    events_before: u64,
    partial: bool,
    failed_streams: Vec<GroupFailedStream>,
    failed_groups: Vec<GroupFailure>,
    listing_failure: Option<ApiFailure>,
    failure: Option<ApiFailure>,
    problem: Option<QueryProblem>,
    result: Option<QueryResult>,
    cache_saving: bool,
    started_at: Option<Instant>,
    final_ms: Option<u64>,
    /// The log groups chosen when the run started, in the order chosen: the
    /// default export file name is made from the first (261022-small-features
    /// U2:BR4.5, functional-design review R-06).
    log_group_names: Vec<String>,
}

/// The Log Analytics screen's input and last run.
#[derive(Debug, Clone)]
pub struct InsightsSession {
    selected_log_groups: Vec<String>,
    start_input: DateTimeInput,
    end_input: DateTimeInput,
    query_text: String,
    revision: u64,
    run: InsightsRun,
    /// The JSON field names of the last fetch (U2:BR1.7).
    field_names: Vec<String>,
}

impl Default for InsightsSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Milliseconds from `start` to `now`; 0 when the clock went back.
fn millis_between(start: Instant, now: Instant) -> u64 {
    u64::try_from(now.saturating_duration_since(start).as_millis()).unwrap_or(u64::MAX)
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// U3:BR4.1: `names` as a choice of log groups: each listed, each once in
/// the order first chosen, at most [`MAX_LOG_GROUPS`].
fn chosen_log_groups(
    names: Vec<String>,
    listed: &impl Fn(&str) -> bool,
) -> Result<Vec<String>, SessionError> {
    let mut chosen: Vec<String> = Vec::with_capacity(names.len().min(MAX_LOG_GROUPS));
    for name in names {
        if !listed(&name) {
            return Err(SessionError::UnknownLogGroup);
        }
        if !chosen.contains(&name) {
            if chosen.len() == MAX_LOG_GROUPS {
                return Err(SessionError::Invalid);
            }
            chosen.push(name);
        }
    }
    Ok(chosen)
}

/// The screen's summary of `result` of `version` (U1:BR6.4, U2:BR5.3,
/// BR7.2, 261025-insights-stats U3:BR4.1).
fn summary_of((result, version): (&QueryResult, &ResultVersion)) -> ResultSummary {
    ResultSummary {
        run_id: result.run_id,
        shape: result.shape(),
        row_count: count(result.row_count()),
        evaluated_count: count(result.evaluated_count),
        columns: result.columns().names(),
        matched_count: count(result.matched_count()),
        truncated_at: result.truncated_at().map(count),
        version: version.version,
        stage: version.stage,
        coverage: version.coverage,
    }
}

impl InsightsSession {
    /// Nothing selected, empty times and the example query.
    pub fn new() -> Self {
        Self {
            selected_log_groups: Vec::new(),
            start_input: DateTimeInput::default(),
            end_input: DateTimeInput::default(),
            query_text: DEFAULT_QUERY.to_string(),
            revision: 0,
            run: InsightsRun::default(),
            field_names: Vec::new(),
        }
    }

    /// The same session with the time range of a first launch (the same
    /// default as the Log Management screen's, gate fix 16a).
    pub fn with_range(mut self, start: DateTimeInput, end: DateTimeInput) -> Self {
        self.start_input = start;
        self.end_input = end;
        self
    }

    /// The phase of the current or last run.
    pub fn phase(&self) -> AnalyticsPhase {
        self.run.phase
    }

    /// The number of the current or last run.
    pub fn run_id(&self) -> u64 {
        self.run.run_id
    }

    /// The query text.
    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    /// The chosen log groups, in the order chosen (U3:BR4.1).
    pub fn selected_log_groups(&self) -> &[String] {
        &self.selected_log_groups
    }

    /// Whether the run is fetching (U1:BR2.3: it owns the one fetch).
    pub fn is_fetching(&self) -> bool {
        self.run.phase == AnalyticsPhase::Fetching
    }

    /// Whether the run is evaluating.
    pub fn is_evaluating(&self) -> bool {
        self.run.phase == AnalyticsPhase::Evaluating
    }

    /// Whether [Cancel] was pressed while fetching and the run has not
    /// ended (261025-insights-stats U3:BR3.1).
    pub fn is_cancelling(&self) -> bool {
        self.run.phase == AnalyticsPhase::Cancelling
    }

    /// Fetching, evaluating or cancelling (U1:BR1.5 (6), 261025-insights-stats
    /// U3:BR3.7).
    pub fn is_running(&self) -> bool {
        self.is_fetching() || self.is_evaluating() || self.is_cancelling()
    }

    /// Whether the run's fetch may still be under way: Fetching, or
    /// Cancelling before the fetch ended (U3:BR3.7). The one fetch of the
    /// app is the query's meanwhile (U1:BR2.3).
    pub fn holds_fetch(&self) -> bool {
        self.is_fetching() || (self.is_cancelling() && !self.run.fetch_ended)
    }

    /// Whether the run evaluates: Evaluating, or Cancelling after the fetch
    /// ended (the cancelled evaluation, U3:BR3.1).
    fn evaluates(&self) -> bool {
        self.is_evaluating() || (self.is_cancelling() && self.run.fetch_ended)
    }

    /// The version of the shown result (U3:BR2.4).
    pub fn result_version(&self) -> Option<ResultVersion> {
        self.run.result_version
    }

    /// Whether `(run_id, version)` is the shown result's (U3:BR2.4).
    fn is_shown(&self, run_id: u64, version: u64) -> bool {
        self.run
            .result_version
            .is_some_and(|shown| shown.run_id == run_id && shown.version == version)
    }

    /// U3:BR2.4, BR2.5: `result` of the current run becomes the shown one at
    /// the next version (1 for the run's first), replacing the previous
    /// run's whatever its shape.
    fn show(&mut self, result: QueryResult, stage: ResultStage, coverage: Option<PartialCoverage>) {
        let run = &mut self.run;
        let version = match run.result_version {
            Some(shown) if shown.run_id == result.run_id => shown.version + 1,
            _ => 1,
        };
        run.result_version = Some(ResultVersion {
            run_id: result.run_id,
            version,
            stage,
            coverage,
        });
        run.result = Some(result);
    }

    /// U3:BR2.1, BR2.4: a partial result of the current run while its fetch
    /// may go on is shown at the next version; one of another run, or after
    /// the fetch ended, is dropped. Returns whether it was taken.
    pub fn on_partial_result(&mut self, result: QueryResult, coverage: PartialCoverage) -> bool {
        if result.run_id != self.run.run_id || !self.holds_fetch() {
            return false;
        }
        self.show(result, ResultStage::Partial, Some(coverage));
        true
    }

    /// U3:BR3.2, BR3.3: the run ends Cancelled without a new result: a
    /// partial result of the run stays, marked cancelled, at the same
    /// version; otherwise what was shown stays as it was (the previous
    /// result, if any).
    fn end_cancelled(&mut self, now: Instant) {
        let run = &mut self.run;
        if let Some(shown) = run.result_version.as_mut() {
            if shown.run_id == run.run_id && shown.stage == ResultStage::Partial {
                shown.stage = ResultStage::CancelledPartial;
                shown.coverage = None;
            }
        }
        run.phase = AnalyticsPhase::Cancelled;
        run.progress = None;
        run.cancel_requested = false;
        run.abandon_requested = false;
        run.cache_saving = false;
        self.settle(now);
    }

    /// The shown result, if any.
    pub fn result(&self) -> Option<&QueryResult> {
        self.run.result.as_ref()
    }

    /// Whether a result with at least one row is shown (U1:BR7.1); since
    /// U3 the previous result stays shown during a run and after a
    /// [Cancel] (U3:BR3.4).
    pub fn has_result_rows(&self) -> bool {
        self.run.result.as_ref().is_some_and(|r| r.row_count() > 0)
    }

    /// Changes one field (U1:BR1.3, BR1.4). Refused while running
    /// (U1:BR2.4). `listed` says whether a log group is in the current list.
    /// Returns whether the input changed.
    ///
    /// The log groups (U3:BR4.1) come as a list in the order chosen (or as
    /// one name, or empty); every one must be listed, a name chosen twice
    /// counts once, and more than [`MAX_LOG_GROUPS`] is refused (the 51st is
    /// not taken).
    pub fn update(
        &mut self,
        field: AnalyticsField,
        value: impl Into<AnalyticsValue>,
        zone: Tz,
        listed: impl Fn(&str) -> bool,
    ) -> Result<bool, SessionError> {
        if self.is_running() {
            return Err(SessionError::Busy);
        }
        let value = value.into();
        let changed = match (field, value) {
            (AnalyticsField::LogGroup, value) => {
                let names = match value {
                    AnalyticsValue::Text(name) if name.is_empty() => Vec::new(),
                    AnalyticsValue::Text(name) => vec![name],
                    AnalyticsValue::Names(names) => names,
                };
                let chosen = chosen_log_groups(names, &listed)?;
                let changed = chosen != self.selected_log_groups;
                self.selected_log_groups = chosen;
                changed
            }
            (_, AnalyticsValue::Names(_)) => return Err(SessionError::Invalid),
            (AnalyticsField::StartText, AnalyticsValue::Text(text)) => {
                self.start_input.edit(text, zone)
            }
            (AnalyticsField::EndText, AnalyticsValue::Text(text)) => {
                self.end_input.edit(text, zone)
            }
            (AnalyticsField::QueryText, AnalyticsValue::Text(text)) => {
                let changed = text != self.query_text;
                self.query_text = text;
                changed
            }
        };
        if changed {
            self.revision += 1;
        }
        Ok(changed)
    }

    /// The time zone was switched (U4:BR1.4): the inputs follow it, and a
    /// query read in the old zone may not start (R-04).
    pub fn switch_zone(&mut self, zone: Tz) {
        self.start_input.switch_zone(zone);
        self.end_input.switch_zone(zone);
        self.revision += 1;
    }

    /// The time range's reasons, without the log group (U1:BR1.4).
    fn range_errors(&self) -> Vec<ValidationError> {
        match validate_session_input("-", &self.start_input, &self.end_input) {
            Ok(_) => Vec::new(),
            Err(errors) => errors
                .into_iter()
                .filter(|error| {
                    !matches!(
                        error,
                        ValidationError::LogGroupRequired | ValidationError::LogGroupTooLong
                    )
                })
                .collect(),
        }
    }

    /// The first reason why [Run query] cannot be pressed (U1:BR1.5).
    pub fn blocker(&self, context: &RunContext) -> Option<RunBlocker> {
        if context.connection.is_none() {
            Some(RunBlocker::NotConnected)
        } else if self.selected_log_groups.is_empty() {
            // U3:BR4.2: from 1 to 50; more than 50 is never taken (BR4.1).
            Some(RunBlocker::LogGroupRequired)
        } else if !self.range_errors().is_empty() {
            Some(RunBlocker::RangeInvalid)
        } else if self.query_text.trim().is_empty() {
            Some(RunBlocker::QueryEmpty)
        } else if self.query_text.chars().count() > MAX_QUERY_CHARS {
            Some(RunBlocker::QueryTooLong)
        } else if self.is_running() {
            Some(RunBlocker::QueryRunning)
        } else if context.log_management_fetching {
            Some(RunBlocker::LogManagementFetching)
        } else if context.other_operation {
            Some(RunBlocker::OtherOperation)
        } else {
            None
        }
    }

    /// The query text and its revision, for reading outside the lock.
    pub fn snapshot(&self) -> QuerySnapshot {
        QuerySnapshot {
            revision: self.revision,
            text: self.query_text.clone(),
        }
    }

    /// [Run query] with the query read from `revision` (UC1 2, R-04): in
    /// one call, the reasons are checked again, the revision must still be
    /// the input's, a new run is numbered and either holds the reading
    /// problem (QueryError, the previous result stays, U1:BR3.9) or starts
    /// fetching. Since U3 the previous result stays while the run fetches
    /// and evaluates (U3:BR3.4, replacing U1:BR2.9) and the completion names
    /// go (U3:BR5.2).
    pub fn begin(
        &mut self,
        revision: u64,
        parsed: Result<&QueryPlan, QueryProblem>,
        context: &RunContext,
        now: Instant,
    ) -> Result<QueryBegin, SessionError> {
        if let Some(blocker) = self.blocker(context) {
            return Err(blocker.session_error());
        }
        if revision != self.revision {
            return Err(SessionError::StaleInput);
        }
        let Some((profile, region)) = context.connection.clone() else {
            return Err(SessionError::Invalid);
        };
        let mut range = None;
        for group in &self.selected_log_groups {
            let validated = validate_session_input(group, &self.start_input, &self.end_input)
                .map_err(|_| SessionError::Invalid)?;
            range = Some(validated.range);
        }
        let Some(range) = range else {
            return Err(SessionError::Invalid);
        };
        let run_id = self.run.run_id + 1;
        if let Err(problem) = parsed {
            // U1:BR3.9: the previous result stays; the alert of a previous
            // failed fetch is not this run's and goes.
            self.run.run_id = run_id;
            self.run.phase = AnalyticsPhase::QueryError;
            self.run.problem = Some(problem);
            self.run.failure = None;
            self.run.cancel_requested = false;
            return Ok(QueryBegin::Problem);
        }
        let group_count = u32::try_from(self.selected_log_groups.len()).unwrap_or(u32::MAX);
        self.run = InsightsRun {
            run_id,
            phase: AnalyticsPhase::Fetching,
            progress: Some(InsightsProgress {
                group_count,
                ..InsightsProgress::default()
            }),
            result: self.run.result.take(),
            result_version: self.run.result_version.take(),
            started_at: Some(now),
            log_group_names: self.selected_log_groups.clone(),
            ..InsightsRun::default()
        };
        self.field_names.clear();
        let keys = self
            .selected_log_groups
            .iter()
            .map(|group| HoldingKey {
                profile: profile.clone(),
                region: region.clone(),
                log_group_name: group.clone(),
            })
            .collect();
        Ok(QueryBegin::Fetch(QueryStart {
            run_id,
            range,
            keys,
        }))
    }

    /// The fetching run `run_id` (also while its fetch stops after
    /// [Cancel], U3:BR3.7), or `None` for another run (U1:BR6.2).
    fn fetching(&mut self, run_id: u64) -> Option<&mut InsightsRun> {
        (self.run.run_id == run_id && self.holds_fetch()).then_some(&mut self.run)
    }

    fn progress_of(&mut self, run_id: u64, job_id: u64) -> Option<&mut InsightsProgress> {
        let run = self.fetching(run_id)?;
        if run.job_id != Some(job_id) {
            return None;
        }
        run.progress.as_mut()
    }

    /// The fetch of one range of `run_id` started `job`: its stream counts
    /// start again, the log group counts stay (U3:BR2.7).
    pub fn on_job_started(&mut self, run_id: u64, job: &FetchJob) {
        if let Some(run) = self.fetching(run_id) {
            run.job_id = Some(job.job_id);
            let previous = run.progress.take().unwrap_or_default();
            run.progress = Some(InsightsProgress {
                reading_cache: false,
                group_number: previous.group_number,
                group_count: previous.group_count,
                failed_group_count: previous.failed_group_count,
                event_count: run.events_before,
                ..InsightsProgress::default()
            });
        }
    }

    /// U3:BR2.7: the disk cache is read for `run_id`.
    pub fn on_reading_cache(&mut self, run_id: u64) {
        if let Some(progress) = self.fetching(run_id).and_then(|run| run.progress.as_mut()) {
            progress.reading_cache = true;
        }
    }

    /// U3:BR2.7: chosen log group `index` (from 0) of `count` starts its
    /// fetch from AWS.
    pub fn on_group_started(&mut self, run_id: u64, index: usize, count: usize) {
        if let Some(run) = self.fetching(run_id) {
            run.job_id = None;
            let progress = run.progress.get_or_insert_with(InsightsProgress::default);
            progress.reading_cache = false;
            progress.group_number = Some(u32::try_from(index + 1).unwrap_or(u32::MAX));
            progress.group_count = u32::try_from(count).unwrap_or(u32::MAX);
            progress.selected_stream_count = 0;
            progress.planned_stream_count = None;
            progress.finished_stream_count = 0;
        }
    }

    /// U3:BR2.7, R-04: one range of `run_id` was fetched with `job`; its
    /// events count on, and a failed fetch counts its log group as failed.
    pub fn on_range_fetched(&mut self, run_id: u64, job: &FetchJob) {
        if let Some(run) = self.fetching(run_id) {
            run.events_before += job.event_count;
            run.cache_saving = false;
            if let Some(progress) = run.progress.as_mut() {
                progress.event_count = run.events_before;
                if job.status == crate::fetch_status::JobStatus::Failed {
                    progress.failed_group_count += 1;
                }
            }
        }
    }

    /// One listing page was read.
    pub fn on_listing_progress(&mut self, run_id: u64, job_id: u64, listing: ListingProgress) {
        if let Some(progress) = self.progress_of(run_id, job_id) {
            progress.selected_stream_count = listing.selected_stream_count;
        }
    }

    /// The listing ended.
    pub fn on_planned(&mut self, run_id: u64, job: &FetchJob) {
        if let Some(progress) = self.progress_of(run_id, job.job_id) {
            progress.planned_stream_count = Some(job.planned_stream_count);
            progress.selected_stream_count = job.planned_stream_count;
        }
    }

    /// One page was added to the current range's holder.
    pub fn on_batch(&mut self, run_id: u64, job_id: u64, batch: BatchProgress) {
        let before = self.run.events_before;
        if let Some(progress) = self.progress_of(run_id, job_id) {
            progress.event_count = before + batch.total;
        }
    }

    /// One stream finished.
    pub fn on_stream_finished(&mut self, run_id: u64, job: &FetchJob) {
        if let Some(progress) = self.progress_of(run_id, job.job_id) {
            progress.finished_stream_count = job.finished_stream_count;
        }
    }

    /// The fetched logs are being written to the disk cache: [Cancel] is
    /// not offered meanwhile (R-02, as Log Management's cache_saving).
    pub fn on_saving(&mut self, run_id: u64, job_id: u64) {
        if self.progress_of(run_id, job_id).is_some() {
            self.run.cache_saving = true;
        }
    }

    fn settle(&mut self, now: Instant) {
        if let (Some(started), None) = (self.run.started_at, self.run.final_ms) {
            self.run.final_ms = Some(millis_between(started, now));
        }
    }

    /// The fetch of `run_id` ended with `summary` (UC1 8, U3:BR2.5).
    /// Returns whether to evaluate:
    ///
    /// - the app closes: Idle, as if the run never started;
    /// - a log group failed: FetchFailed, with every failed log group and
    ///   the first one's failure; the previous result goes (U3:BR3.4);
    /// - [Cancel] (or the fetch was stopped): Cancelling, and what was
    ///   fetched is evaluated into the cancelled result
    ///   ([`FetchEnd::EvaluateCancelled`], 261025-insights-stats U3:BR3.1,
    ///   replacing U1:BR2.7's "nothing is evaluated"); after a second
    ///   [Cancel] nothing is evaluated and the run ends Cancelled (BR3.2);
    ///   the previous result stays until a new one comes;
    /// - otherwise Evaluating, partial when some log group completed with
    ///   failures (U1:BR2.5). Nothing fetched from AWS or the cache is no
    ///   different (U3:BR2.3).
    pub fn finish_fetch(
        &mut self,
        run_id: u64,
        summary: &QueryRunSummary,
        now: Instant,
    ) -> FetchEnd {
        if self.fetching(run_id).is_none() {
            return FetchEnd::Skip;
        }
        if summary.closed {
            self.run = InsightsRun {
                run_id: self.run.run_id,
                ..InsightsRun::default()
            };
            return FetchEnd::Skip;
        }
        let failed = summary.failed();
        let phase = if failed {
            AnalyticsPhase::FetchFailed
        } else if summary.stopped || self.run.cancel_requested {
            AnalyticsPhase::Cancelling
        } else {
            AnalyticsPhase::Evaluating
        };
        let run = &mut self.run;
        run.phase = phase;
        run.cache_saving = false;
        run.failed_streams = summary.failed_streams();
        run.listing_failure = summary
            .groups
            .iter()
            .filter(|group| group.status == GroupFetchStatus::CompletedWithFailures)
            .find_map(|group| group.failure.clone());
        if phase != AnalyticsPhase::FetchFailed {
            run.partial = summary.partial;
            let events = run.events_before;
            run.progress = Some(InsightsProgress {
                event_count: events,
                ..run.progress.clone().unwrap_or_default()
            });
        }
        if phase == AnalyticsPhase::Evaluating {
            return FetchEnd::Evaluate;
        }
        if phase == AnalyticsPhase::Cancelling {
            run.cancel_requested = true;
            run.fetch_ended = true;
            if run.abandon_requested {
                self.end_cancelled(now);
                return FetchEnd::Skip;
            }
            return FetchEnd::EvaluateCancelled;
        }
        if phase == AnalyticsPhase::FetchFailed {
            run.failed_groups = summary
                .groups
                .iter()
                .filter(|group| group.status == GroupFetchStatus::Failed)
                .map(|group| GroupFailure {
                    log_group_name: group.log_group_name.clone(),
                    failure: group.failure.clone().unwrap_or_else(|| {
                        ApiFailure::new(
                            crate::failure::FailureKind::Other,
                            &crate::failure::SafeDetailFields::default(),
                        )
                    }),
                })
                .collect();
            run.failure = run.failed_groups.first().map(|group| group.failure.clone());
            run.result = None;
            run.result_version = None;
        }
        run.progress = None;
        run.cancel_requested = false;
        self.settle(now);
        FetchEnd::Skip
    }

    /// `evaluated` of `total` events of `run_id` were evaluated; returns
    /// whether to go on: not for another run, not after [Cancel] while
    /// Evaluating, and after the second [Cancel] only while Cancelling
    /// (261025-insights-stats U3:BR3.1, BR3.2).
    pub fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool {
        if self.run.run_id != run_id || !self.evaluates() {
            return false;
        }
        let stopped = if self.is_cancelling() {
            self.run.abandon_requested
        } else {
            self.run.cancel_requested
        };
        if stopped {
            return false;
        }
        if let Some(progress) = self.run.progress.as_mut() {
            progress.evaluated_count = u64::try_from(evaluated).unwrap_or(u64::MAX);
            progress.total_count = u64::try_from(total).unwrap_or(u64::MAX);
        }
        true
    }

    /// The evaluation of the current run finished (UC1 12): the final
    /// result, or after [Cancel] while fetching the cancelled one
    /// (261025-insights-stats U3:BR3.1), each at the next version. When
    /// [Cancel] was pressed while Evaluating, or the second [Cancel] while
    /// Cancelling, even after the last step or with nothing held to
    /// evaluate, the result is dropped and the run is Cancelled (U2:BR10.1,
    /// U1 code review R-02), a partial result of the run staying marked
    /// cancelled (U3:BR3.2, BR3.3).
    pub fn finish_evaluation(&mut self, result: QueryResult, now: Instant) {
        if self.run.run_id != result.run_id || !self.evaluates() {
            return;
        }
        let cancelled = self.is_cancelling();
        let dropped = if cancelled {
            self.run.abandon_requested
        } else {
            self.run.cancel_requested
        };
        if dropped {
            self.end_cancelled(now);
            return;
        }
        let (phase, stage) = if cancelled {
            (AnalyticsPhase::Cancelled, ResultStage::CancelledPartial)
        } else {
            (AnalyticsPhase::Done, ResultStage::Final)
        };
        self.show(result, stage, None);
        self.run.phase = phase;
        self.run.progress = None;
        self.run.cancel_requested = false;
        self.settle(now);
    }

    /// The evaluation of `run_id` ended abnormally (U2:BR10.2): no result
    /// is shown, the previous one going too (U3:BR3.4), and the fetch is
    /// not said to have failed.
    pub fn fail_evaluation(&mut self, run_id: u64, now: Instant) {
        if self.run.run_id != run_id || !self.evaluates() {
            return;
        }
        self.run.phase = AnalyticsPhase::EvaluationFailed;
        self.run.result = None;
        self.run.result_version = None;
        self.run.failure = None;
        self.run.progress = None;
        self.run.cancel_requested = false;
        self.run.abandon_requested = false;
        self.settle(now);
    }

    /// U2:BR1.7: the JSON field names of the logs `run_id` fetched, kept
    /// for completion; names of another run are dropped.
    pub fn set_field_names(&mut self, run_id: u64, names: Vec<String>) {
        if self.run.run_id == run_id {
            self.field_names = names;
        }
    }

    /// The JSON field names kept for completion.
    pub fn field_names(&self) -> &[String] {
        &self.field_names
    }

    /// U2:BR9.1: the commands, the words, the fixed fields and the JSON
    /// field names of the last fetch (none before one).
    pub fn completion_candidates(&self) -> Vec<CompletionCandidate> {
        let mut candidates = catalog_candidates();
        candidates.extend(self.field_names.iter().map(|name| CompletionCandidate {
            text: name.clone(),
            kind: CompletionKind::JsonField,
        }));
        candidates
    }

    /// The evaluation of `run_id` stopped before its end (U1:BR2.7, BR5.8):
    /// the run is Cancelled, a partial result of it staying marked
    /// cancelled (261025-insights-stats U3:BR3.2, BR3.3).
    pub fn stop_evaluation(&mut self, run_id: u64, now: Instant) {
        if self.run.run_id != run_id || !self.evaluates() {
            return;
        }
        self.end_cancelled(now);
    }

    /// Whether [Cancel] can be pressed (R-02: as Log Management's): once
    /// while Fetching or Evaluating, and once more while Cancelling
    /// (261025-insights-stats U3:BR3.2, FR review R-04).
    pub fn cancel_available(&self, close_pending: bool) -> bool {
        let pressed = if self.is_cancelling() {
            self.run.abandon_requested
        } else {
            self.run.cancel_requested
        };
        self.is_running()
            && !pressed
            && !(self.is_fetching() && self.run.cache_saving)
            && !close_pending
    }

    /// [Cancel] (UC4): while Fetching the run goes to Cancelling and its
    /// fetch is to stop (U3:BR3.1); while Evaluating it is to stop
    /// evaluating (U3:BR3.3, R-01); while Cancelling the evaluation after
    /// the fetch is abandoned (U3:BR3.2). Returns what the caller tells the
    /// run's stop handle; `None` when it cannot be pressed.
    pub fn request_cancel(&mut self, close_pending: bool) -> Option<CancelRequest> {
        if !self.cancel_available(close_pending) {
            return None;
        }
        let run_id = self.run.run_id;
        if self.is_cancelling() {
            self.run.abandon_requested = true;
            return Some(CancelRequest::Abandon(run_id));
        }
        self.run.cancel_requested = true;
        if self.is_fetching() {
            self.run.phase = AnalyticsPhase::Cancelling;
        }
        Some(CancelRequest::Stop(run_id))
    }

    /// Ends a run whose task stopped without reporting (it panicked): a
    /// fetching run fails with `failure`, an evaluating one is an
    /// evaluation failure without it (U2:BR10.2: the fetch did not fail).
    /// Returns whether the session changed.
    pub fn abort_with_failure(&mut self, run_id: u64, failure: ApiFailure, now: Instant) -> bool {
        if self.run.run_id != run_id || !self.is_running() {
            return false;
        }
        if self.evaluates() {
            self.fail_evaluation(run_id, now);
            self.run.cache_saving = false;
            return true;
        }
        self.run.phase = AnalyticsPhase::FetchFailed;
        self.run.failure = Some(failure);
        self.run.failed_groups.clear();
        self.run.result = None;
        self.run.result_version = None;
        self.run.progress = None;
        self.run.cache_saving = false;
        self.run.cancel_requested = false;
        self.run.abandon_requested = false;
        self.settle(now);
        true
    }

    /// U1:BR7.2, U3:BR4.1: the connection changed. The log group choice
    /// (all of it), the result, the error and the warning go and the phase
    /// is Idle; the query text and the time range stay. Never called while
    /// running (U1:BR7.3).
    pub fn on_connection_changed(&mut self) {
        self.selected_log_groups.clear();
        self.run = InsightsRun {
            run_id: self.run.run_id,
            ..InsightsRun::default()
        };
        self.field_names.clear();
        self.revision += 1;
    }

    /// At most `limit` rows from `offset` of the shown result, when
    /// `run_id` is its run (U1:BR6.3, U3:BR3.2: read from the result's own
    /// copies, so the QueryStore is not needed and the run number alone
    /// tells the result, 261020-carryover-fixes FR2.3.2), with their cells
    /// for the result's columns (U2:BR7.1). Each message is read as JSON
    /// once, now, and nothing is kept (U2:BR7.4).
    ///
    /// Since 261025-insights-stats U3 the rows are read for one version of
    /// the result: none for another (U3:BR2.4), and each row carries its
    /// [`row_key`] (U3:BR4.2).
    pub fn display_rows(
        &self,
        run_id: u64,
        version: u64,
        (offset, limit): (usize, usize),
        zone: Tz,
    ) -> QueryRowWindow {
        let total_count = self.result().map_or(0, |result| {
            u64::try_from(result.row_count()).unwrap_or(u64::MAX)
        });
        let format_time = |time| format_display_in_zone(time, zone);
        let rows = self
            .shown_result(run_id, version)
            .map_or_else(Vec::new, |result| {
                result
                    .window(run_id, offset, limit)
                    .iter()
                    .map(|row| QueryRow {
                        display_time: row.event().map(|event| format_time(event.timestamp)),
                        values: result_row_cells(result.columns(), row, &format_time),
                        log_group_name: row.log_group_name().map(str::to_string),
                        event: row.event().cloned(),
                        row_key: row_key(row, result.key_count()),
                    })
                    .collect()
            });
        QueryRowWindow {
            run_id,
            version,
            offset: u64::try_from(offset).unwrap_or(u64::MAX),
            rows,
            total_count,
        }
    }

    /// The shown result when `(run_id, version)` is its (U3:BR2.4).
    fn shown_result(&self, run_id: u64, version: u64) -> Option<&QueryResult> {
        self.result().filter(|_| self.is_shown(run_id, version))
    }

    /// U3:BR2.8: the place of the row whose [`row_key`] is `key` in version
    /// `version` of the result of `run_id`; `None` when that is not the
    /// shown version or no row has the key.
    pub fn find_row(&self, run_id: u64, version: u64, key: &str) -> Option<u64> {
        let result = self.shown_result(run_id, version)?;
        let rows = result.window(run_id, 0, result.row_count());
        rows.iter()
            .position(|row| row_key(row, result.key_count()) == key)
            .map(count)
    }

    /// The fields of result row `index` of the shown result, when `run_id`
    /// is its run (U2:BR7.3, U3:BR3.2); no field otherwise. An aggregate
    /// row lists its columns with their cells, as the table writes them
    /// (261025-insights-stats BR4.3). Since U3 of that intent, for the shown
    /// version only (U3:BR2.4).
    pub fn display_row_detail(
        &self,
        run_id: u64,
        version: u64,
        index: usize,
        zone: Tz,
    ) -> QueryRowDetail {
        let format_time = |time| format_display_in_zone(time, zone);
        let fields = self
            .shown_result(run_id, version)
            .and_then(|result| {
                let row = result.row_at(run_id, index)?;
                let pairs = match row.event_fields() {
                    Some(fields) => row_detail(&fields, &format_time),
                    None => result
                        .columns()
                        .names()
                        .into_iter()
                        .zip(result_row_cells(result.columns(), row, &format_time))
                        .collect(),
                };
                Some(
                    pairs
                        .into_iter()
                        .map(|(name, value)| DetailField { name, value })
                        .collect(),
                )
            })
            .unwrap_or_default();
        QueryRowDetail {
            run_id,
            version,
            index: u64::try_from(index).unwrap_or(u64::MAX),
            fields,
        }
    }

    /// The screen's view of the Log Analytics screen.
    pub fn view(&self, context: &RunContext, now: Instant) -> InsightsView {
        let blocker = self.blocker(context);
        let run = &self.run;
        InsightsView {
            selected_log_group_names: self.selected_log_groups.clone(),
            max_log_groups: u32::try_from(MAX_LOG_GROUPS).unwrap_or(u32::MAX),
            log_group_options: Vec::new(),
            log_group_status: None,
            start_input: self.start_input.clone(),
            end_input: self.end_input.clone(),
            query_text: self.query_text.clone(),
            range_errors: message_keys(&self.range_errors()),
            run_blocker: blocker.map(|blocker| blocker.message_key().to_string()),
            can_run: blocker.is_none(),
            run_id: run.run_id,
            phase: run.phase,
            progress: run.progress.clone(),
            problem: (run.phase == AnalyticsPhase::QueryError)
                .then(|| run.problem.clone())
                .flatten(),
            failure: run.failure.clone(),
            failed_groups: run.failed_groups.clone(),
            partial: run.partial,
            failed_streams: run.failed_streams.clone(),
            listing_failure: run.listing_failure.clone(),
            result: run
                .result
                .as_ref()
                .zip(run.result_version.as_ref())
                .map(summary_of),
            elapsed: run.started_at.map(|started| ElapsedView {
                running: run.final_ms.is_none(),
                elapsed_ms: run.final_ms.unwrap_or_else(|| millis_between(started, now)),
                fetch_number: run.run_id,
            }),
            cancelling: self.is_running() && run.cancel_requested,
            cancel_available: self.cancel_available(context.close_pending),
            can_export: self.can_export(context.exporting).is_available(),
        }
    }

    /// 261022-small-features U2:BR3.2: whether the shown result can be
    /// exported, with the first reason why not: no result, a result with no
    /// row, a run fetching or evaluating (also while the previous result is
    /// shown), or an export under way (`exporting`, the app's one job).
    pub fn can_export(&self, exporting: bool) -> ExportAvailability {
        match self.result() {
            None => ExportAvailability::blocked(ExportBlocker::NoResult),
            Some(result) if result.row_count() == 0 => {
                ExportAvailability::blocked(ExportBlocker::NoRows)
            }
            Some(_) if self.is_running() => ExportAvailability::blocked(ExportBlocker::Running),
            Some(_) if exporting => ExportAvailability::blocked(ExportBlocker::Exporting),
            Some(_) => ExportAvailability::AVAILABLE,
        }
    }

    /// U2:BR3.1: the shown result's columns and every row, copied for an
    /// export; their text is made outside the lock (review R-01). `None`
    /// without a result.
    pub fn export_source(&self, zone: Tz) -> Option<ExportSnapshotSource> {
        let result = self.result()?;
        Some(ExportSnapshotSource::LogAnalytics {
            columns: result.columns().clone(),
            rows: result.window(result.run_id, 0, result.row_count()).to_vec(),
            zone,
        })
    }

    /// The log group the default export file name is made from (U2:BR4.5,
    /// review R-06): the first chosen when the run that made the result
    /// started; the first of the current choice when the run holds none.
    pub fn export_log_group_name(&self) -> Option<&str> {
        self.run
            .log_group_names
            .first()
            .or_else(|| self.selected_log_groups.first())
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};
    use crate::fetch_status::JobStatus;
    use crate::query::evaluator::ResultRow;
    use crate::query::parser::parse_query;
    use crate::query::runner::GroupFetchOutcome;

    const GROUP: &str = "/aws/lambda/orders";
    const UTC: Tz = Tz::UTC;

    fn connected() -> RunContext {
        RunContext {
            connection: Some((
                ProfileSelector::Named("dev".to_string()),
                "ap-northeast-1".to_string(),
            )),
            ..RunContext::default()
        }
    }

    fn listed(name: &str) -> bool {
        name == GROUP
    }

    /// Group, valid range and the default query: [Run query] can be pressed.
    fn ready() -> InsightsSession {
        let mut session = InsightsSession::new();
        for (field, value) in [
            (AnalyticsField::LogGroup, GROUP),
            (AnalyticsField::StartText, "2024/01/02 03:04:05"),
            (AnalyticsField::EndText, "2024/01/02 03:05:05"),
        ] {
            session
                .update(field, value.to_string(), UTC, listed)
                .unwrap();
        }
        session
    }

    fn plan() -> QueryPlan {
        parse_query(DEFAULT_QUERY).unwrap()
    }

    fn start(session: &mut InsightsSession, now: Instant) -> QueryStart {
        let revision = session.snapshot().revision;
        match session.begin(revision, Ok(&plan()), &connected(), now) {
            Ok(QueryBegin::Fetch(start)) => start,
            other => panic!("not started: {other:?}"),
        }
    }

    fn job(status: JobStatus, events: u64) -> FetchJob {
        let mut job = FetchJob::start(TimeRange::from_seconds(0, 60_000).unwrap());
        job.status = status;
        job.event_count = events;
        job
    }

    /// The end of a run's fetch of [`GROUP`] as the runner tells it, from
    /// the status its one range fetch ended with; the events counted in
    /// the run so far are the session's.
    fn ended(status: JobStatus, _events: u64) -> QueryRunSummary {
        let group = match status {
            JobStatus::Completed | JobStatus::Running => GroupFetchStatus::Completed,
            JobStatus::CompletedWithFailures => GroupFetchStatus::CompletedWithFailures,
            JobStatus::Failed => GroupFetchStatus::Failed,
            JobStatus::Cancelled | JobStatus::Aborted => GroupFetchStatus::Cancelled,
        };
        QueryRunSummary {
            groups: vec![GroupFetchOutcome {
                log_group_name: GROUP.to_string(),
                status: group,
                failed_streams: Vec::new(),
                failure: None,
            }],
            partial: status == JobStatus::CompletedWithFailures,
            from_aws: true,
            stopped: matches!(status, JobStatus::Cancelled | JobStatus::Aborted),
            closed: status == JobStatus::Aborted,
        }
    }

    /// Result rows at `positions` (their times), copied from [`GROUP`].
    fn copies(positions: Vec<usize>) -> Vec<ResultRow> {
        positions
            .into_iter()
            .map(|position| ResultRow::Event {
                event: LogEvent {
                    timestamp: i64::try_from(position).unwrap(),
                    ingestion_time: None,
                    message: format!("row {position}"),
                    log_stream_name: "s".to_string(),
                    sequence: 0,
                },
                log_group_name: GROUP.to_string(),
            })
            .collect()
    }

    /// Runs the session to Done with rows at `rows`.
    fn done(session: &mut InsightsSession, now: Instant, rows: Vec<usize>) -> u64 {
        let run_id = start(session, now).run_id;
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 10), now),
            FetchEnd::Evaluate
        );
        session.finish_evaluation(QueryResult::new(run_id, 10, copies(rows)), now);
        assert_eq!(session.phase(), AnalyticsPhase::Done);
        run_id
    }

    #[test]
    fn the_reasons_run_query_cannot_be_pressed_come_in_order() {
        // U1:BR1.5 (1)-(8): each reason shows only once the earlier ones are
        // gone.
        let mut session = InsightsSession::new();
        let mut context = RunContext {
            log_management_fetching: true,
            other_operation: true,
            ..RunContext::default()
        };
        let reason = |session: &InsightsSession, context: &RunContext| session.blocker(context);
        assert_eq!(reason(&session, &context), Some(RunBlocker::NotConnected));
        context.connection = connected().connection;
        assert_eq!(
            reason(&session, &context),
            Some(RunBlocker::LogGroupRequired)
        );
        session
            .update(AnalyticsField::LogGroup, GROUP.to_string(), UTC, listed)
            .unwrap();
        assert_eq!(reason(&session, &context), Some(RunBlocker::RangeInvalid));
        for (field, value) in [
            (AnalyticsField::StartText, "2024/01/02 03:04:05"),
            (AnalyticsField::EndText, "2024/01/02 03:05:05"),
            (AnalyticsField::QueryText, " \n\t "),
        ] {
            session
                .update(field, value.to_string(), UTC, listed)
                .unwrap();
        }
        assert_eq!(reason(&session, &context), Some(RunBlocker::QueryEmpty));
        session
            .update(
                AnalyticsField::QueryText,
                DEFAULT_QUERY.to_string(),
                UTC,
                listed,
            )
            .unwrap();
        assert_eq!(
            reason(&session, &context),
            Some(RunBlocker::LogManagementFetching)
        );
        context.log_management_fetching = false;
        assert_eq!(reason(&session, &context), Some(RunBlocker::OtherOperation));
        context.other_operation = false;
        assert_eq!(reason(&session, &context), None);
        let view = session.view(&context, Instant::now());
        assert!(view.can_run);
        assert_eq!(view.run_blocker, None);
        start(&mut session, Instant::now());
        assert_eq!(reason(&session, &context), Some(RunBlocker::QueryRunning));
        assert_eq!(
            RunBlocker::QueryRunning.message_key(),
            "insights.blocked.queryRunning"
        );
    }

    #[test]
    fn a_query_longer_than_ten_thousand_characters_cannot_run() {
        // Functional-design Q4; characters, not bytes.
        let mut session = ready();
        // `filter @message = ''` is 20 characters.
        let exactly = format!("filter @message = '{}'", "あ".repeat(MAX_QUERY_CHARS - 20));
        assert_eq!(exactly.chars().count(), MAX_QUERY_CHARS);
        session
            .update(AnalyticsField::QueryText, exactly.clone(), UTC, listed)
            .unwrap();
        assert_eq!(session.blocker(&connected()), None);
        session
            .update(
                AnalyticsField::QueryText,
                format!("{exactly} "),
                UTC,
                listed,
            )
            .unwrap();
        assert_eq!(
            session.blocker(&connected()),
            Some(RunBlocker::QueryTooLong)
        );
        let view = session.view(&connected(), Instant::now());
        assert_eq!(
            view.run_blocker.as_deref(),
            Some("insights.blocked.queryTooLong")
        );
        assert!(!view.can_run);
    }

    #[test]
    fn a_run_starts_once_and_only_from_the_input_it_was_read_from() {
        // R-04: the second press is refused, and a query read before the
        // input changed does not start.
        let mut session = ready();
        let now = Instant::now();
        let first = start(&mut session, now);
        assert_eq!(first.run_id, 1);
        assert_eq!(first.keys.len(), 1);
        assert_eq!(first.keys[0].log_group_name, GROUP);
        assert_eq!(first.keys[0].region, "ap-northeast-1");
        assert_eq!(
            first.range.end_instant() - first.range.start_instant(),
            60_999
        );
        let revision = session.snapshot().revision;
        assert_eq!(
            session.begin(revision, Ok(&plan()), &connected(), now),
            Err(SessionError::Busy)
        );
        let mut other = ready();
        let read_from = other.snapshot().revision;
        other
            .update(
                AnalyticsField::QueryText,
                "filter @log = 'x'".to_string(),
                UTC,
                listed,
            )
            .unwrap();
        assert_eq!(
            other.begin(read_from, Ok(&plan()), &connected(), now),
            Err(SessionError::StaleInput)
        );
        assert_eq!(other.phase(), AnalyticsPhase::Idle);
    }

    #[test]
    fn a_reading_problem_keeps_the_text_and_the_previous_result_and_fetches_nothing() {
        // U1:BR3.9, then U3:BR3.4 (which replaces U1:BR2.9) when a
        // readable query runs: the previous result stays while it fetches.
        let mut session = ready();
        let now = Instant::now();
        let first_run = done(&mut session, now, vec![0, 3]);
        let revision = session.snapshot().revision;
        // `stats` is read since 261025-insights-stats U1; `parse` is not.
        let problem = parse_query("parse @message 'a*b' as x").unwrap_err();
        assert_eq!(
            session.begin(revision, Err(problem.clone()), &connected(), now),
            Ok(QueryBegin::Problem)
        );
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::QueryError);
        assert_eq!(view.problem, Some(problem));
        assert_eq!(view.query_text, DEFAULT_QUERY);
        assert_eq!(
            view.result.map(|r| (r.run_id, r.row_count)),
            Some((first_run, 2))
        );
        let next = start(&mut session, now);
        assert_eq!(
            next.run_id,
            first_run + 2,
            "the QueryError run had its number"
        );
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::Fetching);
        assert_eq!(
            view.result.map(|r| (r.run_id, r.row_count)),
            Some((first_run, 2)),
            "U3:BR3.4"
        );
        assert_eq!(view.problem, None);
    }

    #[test]
    fn the_end_of_the_fetch_decides_what_follows() {
        // UC1 10: evaluate, partial, failed, cancelled, closed; R-01.
        let now = Instant::now();
        let failure = ApiFailure::new(FailureKind::AccessDenied, &SafeDetailFields::default());
        // 261025-insights-stats U3:BR3.1 (FR5.1, BR5.1): a cancelled fetch
        // is now evaluated into the cancelled result.
        let cases = [
            (
                JobStatus::Completed,
                FetchEnd::Evaluate,
                AnalyticsPhase::Evaluating,
                false,
            ),
            (
                JobStatus::CompletedWithFailures,
                FetchEnd::Evaluate,
                AnalyticsPhase::Evaluating,
                true,
            ),
            (
                JobStatus::Failed,
                FetchEnd::Skip,
                AnalyticsPhase::FetchFailed,
                false,
            ),
            (
                JobStatus::Cancelled,
                FetchEnd::EvaluateCancelled,
                AnalyticsPhase::Cancelling,
                false,
            ),
            (
                JobStatus::Aborted,
                FetchEnd::Skip,
                AnalyticsPhase::Idle,
                false,
            ),
        ];
        for (status, evaluates, phase, partial) in cases {
            let mut session = ready();
            let run_id = start(&mut session, now).run_id;
            let mut summary = ended(status, 5);
            if status == JobStatus::Failed {
                summary.groups[0].failure = Some(failure.clone());
            }
            assert_eq!(
                session.finish_fetch(run_id, &summary, now),
                evaluates,
                "{status:?}"
            );
            let view = session.view(&connected(), now);
            assert_eq!(view.phase, phase, "{status:?}");
            assert_eq!(view.partial, partial, "{status:?}");
            if status == JobStatus::Failed {
                assert_eq!(view.failure, Some(failure.clone()));
                assert_eq!(
                    view.failed_groups,
                    vec![GroupFailure {
                        log_group_name: GROUP.to_string(),
                        failure: failure.clone()
                    }],
                    "U3:BR2.6"
                );
            }
        }
        // R-01: [Cancel] after the last stream, the fetch still Completed.
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        assert!(session.view(&connected(), now).cancelling);
        // 261025-insights-stats U3:BR3.1 (BR5.1): evaluated as cancelled.
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 5), now),
            FetchEnd::EvaluateCancelled
        );
        assert_eq!(session.phase(), AnalyticsPhase::Cancelling);
        assert!(session.view(&connected(), now).cancelling);
        session.finish_evaluation(QueryResult::new(run_id, 5, copies(vec![1])), now);
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
        assert!(!session.view(&connected(), now).cancelling);
    }

    #[test]
    fn notices_of_another_run_are_dropped() {
        // U1:BR6.2.
        let mut session = ready();
        let now = Instant::now();
        let old = done(&mut session, now, vec![1]);
        let current = start(&mut session, now).run_id;
        let mut started = job(JobStatus::Running, 0);
        session.on_job_started(current, &started);
        started.planned_stream_count = 4;
        session.on_planned(old, &started);
        assert_eq!(
            session.finish_fetch(old, &ended(JobStatus::Completed, 1), now),
            FetchEnd::Skip
        );
        assert_eq!(session.phase(), AnalyticsPhase::Fetching);
        session.on_planned(current, &started);
        session.on_batch(
            current,
            started.job_id,
            BatchProgress {
                added: 3,
                total: 3,
                timeline_version: 1,
            },
        );
        let progress = session.view(&connected(), now).progress.unwrap();
        assert_eq!(progress.planned_stream_count, Some(4));
        assert_eq!(progress.event_count, 3);
        assert_eq!(
            session.finish_fetch(current, &ended(JobStatus::Completed, 3), now),
            FetchEnd::Evaluate
        );
        assert!(!session.on_evaluation_progress(old, 1, 3));
        session.finish_evaluation(QueryResult::new(old, 3, copies(vec![0])), now);
        assert_eq!(
            session.phase(),
            AnalyticsPhase::Evaluating,
            "an old result is dropped"
        );
        assert!(session.on_evaluation_progress(current, 3, 3));
        session.finish_evaluation(QueryResult::new(current, 3, Vec::new()), now);
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::Done);
        assert_eq!(
            view.result.map(|r| (r.run_id, r.row_count)),
            Some((current, 0))
        );
        assert!(!session.has_result_rows(), "no rows: U1:BR7.1 asks nothing");
    }

    #[test]
    fn the_input_cannot_change_while_running_and_a_group_must_be_listed() {
        // U1:BR1.3, BR2.4.
        let mut session = ready();
        assert_eq!(
            session.update(
                AnalyticsField::LogGroup,
                "/not/listed".to_string(),
                UTC,
                listed
            ),
            Err(SessionError::UnknownLogGroup)
        );
        assert_eq!(session.selected_log_groups(), [GROUP.to_string()]);
        let now = Instant::now();
        start(&mut session, now);
        for field in [
            AnalyticsField::LogGroup,
            AnalyticsField::StartText,
            AnalyticsField::EndText,
            AnalyticsField::QueryText,
        ] {
            assert_eq!(
                session.update(field, String::new(), UTC, listed),
                Err(SessionError::Busy),
                "{field:?}"
            );
        }
        let mut idle = ready();
        assert_eq!(
            idle.update(AnalyticsField::LogGroup, String::new(), UTC, listed),
            Ok(true),
            "an empty value deselects"
        );
        assert!(idle.selected_log_groups().is_empty());
        assert_eq!(
            idle.update(
                AnalyticsField::QueryText,
                DEFAULT_QUERY.to_string(),
                UTC,
                listed
            ),
            Ok(false),
            "the same text changes nothing"
        );
    }

    #[test]
    fn cancel_is_offered_while_running_but_not_while_the_cache_is_written() {
        // R-02 (as Log Management's cache_saving), U1:BR2.4, BR5.8.
        let mut session = ready();
        let now = Instant::now();
        let run_id = start(&mut session, now).run_id;
        let fetch = job(JobStatus::Running, 0);
        session.on_job_started(run_id, &fetch);
        assert!(session.cancel_available(false));
        assert!(!session.cancel_available(true), "closing awaits an answer");
        session.on_saving(run_id, fetch.job_id);
        assert!(!session.cancel_available(false));
        assert_eq!(session.request_cancel(false), None);
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 20_000), now),
            FetchEnd::Evaluate
        );
        assert!(
            session.cancel_available(false),
            "the evaluation can be cancelled"
        );
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        assert_eq!(session.request_cancel(false), None, "pressed once");
        assert!(!session.on_evaluation_progress(run_id, 10_000, 20_000));
        let later = now + Duration::from_secs(3);
        session.stop_evaluation(run_id, later);
        let view = session.view(&connected(), later);
        assert_eq!(view.phase, AnalyticsPhase::Cancelled);
        assert_eq!(view.result, None);
        assert_eq!(
            view.elapsed
                .map(|e| (e.running, e.elapsed_ms, e.fetch_number)),
            Some((false, 3_000, run_id))
        );
    }

    #[test]
    fn a_connection_change_keeps_the_query_and_range_but_drops_the_rest() {
        // U1:BR7.2.
        let mut session = ready();
        let now = Instant::now();
        session
            .update(
                AnalyticsField::QueryText,
                "filter @log = 'a'".to_string(),
                UTC,
                listed,
            )
            .unwrap();
        done(&mut session, now, vec![2]);
        assert!(session.has_result_rows());
        session.on_connection_changed();
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::Idle);
        assert!(view.selected_log_group_names.is_empty());
        assert_eq!(view.result, None);
        assert_eq!(view.elapsed, None);
        assert_eq!(view.query_text, "filter @log = 'a'");
        assert_eq!(view.start_input.text(), "2024/01/02 03:04:05");
        assert_eq!(view.end_input.text(), "2024/01/02 03:05:05");
        assert_eq!(
            view.run_blocker.as_deref(),
            Some("insights.blocked.logGroupRequired")
        );
    }

    // ---- 261017-insights-query U2 ---------------------------------------

    /// A run that fetched `events` events and now evaluates.
    fn evaluating(session: &mut InsightsSession, now: Instant, events: u64) -> u64 {
        let run_id = start(session, now).run_id;
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, events), now),
            FetchEnd::Evaluate
        );
        assert!(session.is_evaluating());
        run_id
    }

    #[test]
    fn a_cancel_after_the_last_evaluation_step_drops_the_result() {
        // U2:BR10.1 (U1 code review R-02): the evaluation had nothing left
        // to stop, but [Cancel] was pressed.
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 3);
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        session.finish_evaluation(QueryResult::new(run_id, 3, copies(vec![0, 1])), now);
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::Cancelled);
        assert_eq!(view.result, None);
        assert!(!view.cancelling);
        assert!(!session.has_result_rows());
    }

    #[test]
    fn a_cancel_with_nothing_held_still_ends_cancelled() {
        // U2:BR10.1: no event, so no evaluation step at all.
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 0);
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        session.finish_evaluation(QueryResult::new(run_id, 0, Vec::new()), now);
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
    }

    #[test]
    fn an_evaluation_failure_shows_no_result_and_no_fetch_failure() {
        // U2:BR10.2, U3:BR3.4: the previous result, kept while the run
        // fetched and evaluated, goes now; a connection change goes back to
        // Idle.
        let mut session = ready();
        let now = Instant::now();
        done(&mut session, now, vec![0]);
        let run_id = evaluating(&mut session, now, 5);
        let later = now + Duration::from_secs(2);
        session.fail_evaluation(run_id + 1, later);
        assert!(session.is_evaluating(), "another run's failure is dropped");
        session.fail_evaluation(run_id, later);
        let view = session.view(&connected(), later);
        assert_eq!(view.phase, AnalyticsPhase::EvaluationFailed);
        assert_eq!(view.result, None);
        assert_eq!(view.failure, None, "the fetch did not fail");
        assert_eq!(view.progress, None);
        assert_eq!(
            view.elapsed.map(|e| (e.running, e.elapsed_ms)),
            Some((false, 2_000))
        );
        assert!(view.can_run, "it can run again");
        session.on_connection_changed();
        assert_eq!(session.phase(), AnalyticsPhase::Idle);
    }

    #[test]
    fn a_task_lost_while_evaluating_is_an_evaluation_failure() {
        // U2:BR10.2 (U1 code review R-03): the recovery does not call it a
        // failed GetLogEvents.
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 5);
        let failure = ApiFailure::new(FailureKind::Other, &SafeDetailFields::default());
        assert!(session.abort_with_failure(run_id, failure, now));
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::EvaluationFailed);
        assert_eq!(view.failure, None);
    }

    #[test]
    fn the_summary_carries_the_columns_the_matched_count_and_the_cut() {
        // U2:BR3.2, BR5.3, BR7.2.
        use crate::query::evaluator::EventsOutput;
        use crate::query::plan::{FieldRef, JsonPath};
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 12_000);
        session.finish_evaluation(
            QueryResult::from_output(
                run_id,
                EventsOutput {
                    rows: (0..10_000).collect(),
                    columns: vec![
                        FieldRef::Timestamp,
                        FieldRef::Json(JsonPath::new(vec!["user.id".to_string()], "`user.id`")),
                    ],
                    matched_count: 12_000,
                    truncated_at: Some(10_000),
                    evaluated_count: 12_000,
                },
                copies((0..10_000).collect()),
            ),
            now,
        );
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(summary.columns, vec!["@timestamp", "`user.id`"]);
        assert_eq!(
            (
                summary.row_count,
                summary.matched_count,
                summary.truncated_at
            ),
            (10_000, 12_000, Some(10_000))
        );
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["matchedCount"], 12_000);
        assert_eq!(json["truncatedAt"], 10_000);
    }

    #[test]
    fn rows_give_their_cells_and_an_opened_row_its_fields() {
        // U2:BR7.1, BR7.3.
        use crate::query::evaluator::EventsOutput;
        use crate::query::plan::{FieldRef, JsonPath};
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 1);
        let level = FieldRef::Json(JsonPath::new(vec!["level".to_string()], "level"));
        let gone = FieldRef::Json(JsonPath::new(vec!["gone".to_string()], "gone"));
        let copied = ResultRow::Event {
            event: LogEvent {
                timestamp: 0,
                ingestion_time: None,
                message: r#"{"level":"error","n":1}"#.to_string(),
                log_stream_name: "s".to_string(),
                sequence: 0,
            },
            log_group_name: GROUP.to_string(),
        };
        session.finish_evaluation(
            QueryResult::from_output(
                run_id,
                EventsOutput {
                    rows: vec![0],
                    columns: vec![FieldRef::Timestamp, level, gone],
                    matched_count: 1,
                    truncated_at: None,
                    evaluated_count: 1,
                },
                vec![copied],
            ),
            now,
        );
        let window = session.display_rows(run_id, 1, (0, 50), UTC);
        assert_eq!(
            window.rows[0].values,
            vec![
                Some("1970/01/01 00:00:00.000".to_string()),
                Some("error".to_string()),
                None
            ]
        );
        assert_eq!(window.rows[0].log_group_name.as_deref(), Some(GROUP));
        assert_eq!((window.run_id, window.total_count), (run_id, 1));
        assert!(
            session
                .display_rows(run_id + 1, 1, (0, 50), UTC)
                .rows
                .is_empty(),
            "another run"
        );
        let detail = session.display_row_detail(run_id, 1, 0, UTC);
        let names: Vec<&str> = detail.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["@timestamp", "@log", "@logStream", "@message", "level", "n"]
        );
        assert_eq!(detail.fields[1].value.as_deref(), Some(GROUP));
        for (run, index) in [(run_id + 1, 0), (run_id, 1)] {
            assert!(
                session
                    .display_row_detail(run, 1, index, UTC)
                    .fields
                    .is_empty(),
                "{run} {index}: nothing listed"
            );
        }
    }

    #[test]
    fn field_names_are_kept_for_the_current_run_and_go_with_the_connection() {
        // U2:BR1.7, BR9.1.
        let mut session = ready();
        let now = Instant::now();
        let before: Vec<CompletionKind> = session
            .completion_candidates()
            .iter()
            .map(|candidate| candidate.kind)
            .collect();
        assert!(
            !before.contains(&CompletionKind::JsonField),
            "nothing fetched yet"
        );
        let texts = |session: &InsightsSession| -> Vec<String> {
            session
                .completion_candidates()
                .into_iter()
                .map(|candidate| candidate.text)
                .collect()
        };
        assert!(texts(&session).contains(&"fields".to_string()));
        assert!(texts(&session).contains(&"@logStream".to_string()));
        let run_id = evaluating(&mut session, now, 2);
        session.set_field_names(run_id + 1, vec!["other".to_string()]);
        assert!(session.field_names().is_empty(), "another run's names");
        session.set_field_names(run_id, vec!["http.status".to_string()]);
        let last = session.completion_candidates().pop().unwrap();
        assert_eq!(
            (last.text.as_str(), last.kind),
            ("http.status", CompletionKind::JsonField)
        );
        session.finish_evaluation(QueryResult::new(run_id, 2, Vec::new()), now);
        session.on_connection_changed();
        assert!(session.field_names().is_empty());
    }

    // ---- 261017-insights-query U3:BR3.2, BR3.4 -------------------------

    #[test]
    fn the_previous_result_is_read_while_the_next_run_fetches_and_evaluates() {
        // BR3.4: replaced only when the new result comes; BR3.2: the rows
        // are the old result's own copies.
        let mut session = ready();
        let now = Instant::now();
        let old = done(&mut session, now, vec![5, 3]);
        let times = |session: &InsightsSession, run: u64| -> Vec<i64> {
            session
                .display_rows(run, 1, (0, 10), UTC)
                .rows
                .iter()
                .filter_map(|row| row.event.as_ref().map(|event| event.timestamp))
                .collect()
        };
        let next = start(&mut session, now).run_id;
        assert_eq!(times(&session, old), vec![5, 3], "while fetching");
        assert!(session.has_result_rows());
        assert_eq!(
            session.finish_fetch(next, &ended(JobStatus::Completed, 2), now),
            FetchEnd::Evaluate
        );
        assert_eq!(times(&session, old), vec![5, 3], "while evaluating");
        assert_eq!(
            session.view(&connected(), now).result.map(|r| r.run_id),
            Some(old)
        );
        session.finish_evaluation(QueryResult::new(next, 2, copies(vec![9])), now);
        assert!(times(&session, old).is_empty(), "replaced");
        assert_eq!(times(&session, next), vec![9]);
    }

    #[test]
    fn a_failed_fetch_or_evaluation_drops_the_previous_result_and_a_cancel_keeps_it() {
        // BR3.4 (interaction-spec): a result of other conditions could be
        // mistaken for this run's.
        let now = Instant::now();
        for (failure, kept) in [("fetch", false), ("evaluation", false), ("cancel", true)] {
            let mut session = ready();
            let old = done(&mut session, now, vec![1]);
            let next = start(&mut session, now).run_id;
            match failure {
                "fetch" => {
                    let mut summary = ended(JobStatus::Failed, 0);
                    summary.groups[0].failure = Some(ApiFailure::new(
                        FailureKind::AccessDenied,
                        &SafeDetailFields::default(),
                    ));
                    assert_eq!(session.finish_fetch(next, &summary, now), FetchEnd::Skip);
                }
                "evaluation" => {
                    assert_eq!(
                        session.finish_fetch(next, &ended(JobStatus::Completed, 1), now),
                        FetchEnd::Evaluate
                    );
                    session.fail_evaluation(next, now);
                }
                _ => {
                    assert_eq!(
                        session.request_cancel(false),
                        Some(CancelRequest::Stop(next))
                    );
                    // 261025-insights-stats U3:BR3.1 (BR5.1): evaluated as
                    // cancelled; the previous result stays until it comes.
                    assert_eq!(
                        session.finish_fetch(next, &ended(JobStatus::Cancelled, 0), now),
                        FetchEnd::EvaluateCancelled
                    );
                }
            }
            let result = session.view(&connected(), now).result.map(|r| r.run_id);
            assert_eq!(result, kept.then_some(old), "{failure}");
        }
    }

    // ---- 261017-insights-query U3:BR2.7, BR4.1, BR4.2, BR5.2 ------------

    fn names(count: usize) -> Vec<String> {
        (1..=count).map(|n| format!("/g/{n:02}")).collect()
    }

    fn anything_listed(_: &str) -> bool {
        true
    }

    #[test]
    fn up_to_fifty_log_groups_are_chosen_in_order_and_the_fifty_first_is_refused() {
        // BR4.1.
        let mut session = ready();
        let fifty = names(MAX_LOG_GROUPS);
        assert_eq!(
            session.update(
                AnalyticsField::LogGroup,
                fifty.clone(),
                UTC,
                anything_listed
            ),
            Ok(true)
        );
        assert_eq!(session.selected_log_groups(), fifty.as_slice());
        assert_eq!(
            session.update(AnalyticsField::LogGroup, names(51), UTC, anything_listed),
            Err(SessionError::Invalid),
            "the 51st is not taken"
        );
        assert_eq!(session.selected_log_groups(), fifty.as_slice(), "unchanged");
        let twice = vec!["/b".to_string(), "/a".to_string(), "/b".to_string()];
        session
            .update(AnalyticsField::LogGroup, twice, UTC, anything_listed)
            .unwrap();
        assert_eq!(
            session.selected_log_groups(),
            ["/b".to_string(), "/a".to_string()],
            "once each, in the order first chosen"
        );
        assert_eq!(
            session.update(
                AnalyticsField::LogGroup,
                vec![GROUP.to_string(), "/not/listed".to_string()],
                UTC,
                listed
            ),
            Err(SessionError::UnknownLogGroup)
        );
        assert_eq!(
            session.update(
                AnalyticsField::QueryText,
                vec!["x".to_string()],
                UTC,
                listed
            ),
            Err(SessionError::Invalid),
            "a list is only for the log groups"
        );
        let view = session.view(&connected(), Instant::now());
        assert_eq!(view.max_log_groups, 50);
        assert_eq!(view.selected_log_group_names, ["/b", "/a"]);
    }

    #[test]
    fn run_query_needs_one_to_fifty_log_groups_and_runs_on_every_one() {
        // BR4.2.
        let mut session = ready();
        session
            .update(
                AnalyticsField::LogGroup,
                Vec::<String>::new(),
                UTC,
                anything_listed,
            )
            .unwrap();
        assert_eq!(
            session.blocker(&connected()),
            Some(RunBlocker::LogGroupRequired)
        );
        session
            .update(
                AnalyticsField::LogGroup,
                names(MAX_LOG_GROUPS),
                UTC,
                anything_listed,
            )
            .unwrap();
        assert_eq!(session.blocker(&connected()), None);
        let started = start(&mut session, Instant::now());
        assert_eq!(started.keys.len(), 50);
        assert_eq!(started.keys[0].log_group_name, "/g/01");
        assert_eq!(started.keys[49].log_group_name, "/g/50");
        let progress = session.view(&connected(), Instant::now()).progress.unwrap();
        assert_eq!((progress.group_count, progress.group_number), (50, None));
    }

    #[test]
    fn a_run_clears_the_completion_names_and_gathers_its_own() {
        // BR5.2 (U2 code review R-02).
        let mut session = ready();
        let now = Instant::now();
        let first = evaluating(&mut session, now, 2);
        session.set_field_names(first, vec!["old.name".to_string()]);
        session.finish_evaluation(QueryResult::new(first, 2, Vec::new()), now);
        assert_eq!(session.field_names(), ["old.name"]);
        let next = start(&mut session, now).run_id;
        assert!(session.field_names().is_empty(), "gone when the run starts");
        let mut summary = ended(JobStatus::Failed, 0);
        summary.groups[0].failure = Some(ApiFailure::new(
            FailureKind::AccessDenied,
            &SafeDetailFields::default(),
        ));
        session.finish_fetch(next, &summary, now);
        assert!(
            session.field_names().is_empty(),
            "a failed fetch gathers none"
        );
    }

    #[test]
    fn the_progress_counts_log_groups_streams_events_and_failed_log_groups() {
        // BR2.7, R-04.
        let mut session = ready();
        session
            .update(AnalyticsField::LogGroup, names(3), UTC, anything_listed)
            .unwrap();
        let now = Instant::now();
        let run_id = start(&mut session, now).run_id;
        session.on_reading_cache(run_id);
        assert!(
            session
                .view(&connected(), now)
                .progress
                .unwrap()
                .reading_cache
        );
        session.on_group_started(run_id, 0, 3);
        let first = job(JobStatus::Running, 0);
        session.on_job_started(run_id, &first);
        session.on_batch(
            run_id,
            first.job_id,
            BatchProgress {
                added: 4,
                total: 4,
                timeline_version: 1,
            },
        );
        session.on_range_fetched(run_id, &job(JobStatus::Failed, 4));
        session.on_group_started(run_id, 1, 3);
        let second = job(JobStatus::Running, 0);
        session.on_job_started(run_id, &second);
        session.on_batch(
            run_id,
            second.job_id,
            BatchProgress {
                added: 2,
                total: 2,
                timeline_version: 1,
            },
        );
        let progress = session.view(&connected(), now).progress.unwrap();
        assert!(!progress.reading_cache);
        assert_eq!(
            (
                progress.group_number,
                progress.group_count,
                progress.failed_group_count,
                progress.event_count
            ),
            (Some(2), 3, 1, 6)
        );
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["groupNumber"], 2);
        assert_eq!(json["failedGroupCount"], 1);
    }

    #[test]
    fn a_connection_change_empties_the_whole_choice() {
        // BR4.1.
        let mut session = ready();
        session
            .update(AnalyticsField::LogGroup, names(4), UTC, anything_listed)
            .unwrap();
        session.on_connection_changed();
        assert!(session.selected_log_groups().is_empty());
        assert_eq!(
            session.blocker(&connected()),
            Some(RunBlocker::LogGroupRequired)
        );
    }

    #[test]
    fn the_default_query_is_read_with_its_four_commands() {
        // The console's default columns, sort and limit with U1's filter.
        let plan = parse_query(DEFAULT_QUERY).unwrap();
        assert_eq!(plan.commands.len(), 4);
        assert!(plan.has_fields() && plan.has_sort() && plan.has_row_limit());
    }

    // ---- 261022-small-features U2: the export (BR3.1, BR3.2, R-06) ----

    #[test]
    fn export_is_unavailable_without_a_result_with_zero_rows_while_running_and_while_exporting() {
        let now = Instant::now();
        let mut session = ready();
        assert_eq!(
            session.can_export(false).blocker,
            Some(ExportBlocker::NoResult)
        );
        assert!(session.export_source(UTC).is_none());
        assert!(!session.view(&connected(), now).can_export);

        done(&mut session, now, Vec::new());
        assert_eq!(
            session.can_export(false).blocker,
            Some(ExportBlocker::NoRows)
        );

        done(&mut session, now, vec![3, 1]);
        assert!(session.can_export(false).is_available());
        assert!(session.view(&connected(), now).can_export);
        assert_eq!(
            session.can_export(true).blocker,
            Some(ExportBlocker::Exporting)
        );
        let exporting = RunContext {
            exporting: true,
            ..connected()
        };
        assert!(!session.view(&exporting, now).can_export);

        // The previous result stays shown while the next run goes, but it
        // cannot be exported then (BR3.2).
        let run_id = start(&mut session, now).run_id;
        assert!(session.result().is_some());
        assert_eq!(
            session.can_export(false).blocker,
            Some(ExportBlocker::Running)
        );
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 10), now),
            FetchEnd::Evaluate
        );
        assert_eq!(
            session.can_export(false).blocker,
            Some(ExportBlocker::Running)
        );
    }

    #[test]
    fn the_query_export_snapshot_has_the_result_columns_full_messages_and_empty_cells() {
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 10), now),
            FetchEnd::Evaluate
        );
        let plan = parse_query("fields @timestamp, level, @message | limit 10").unwrap();
        let output = crate::query::evaluator::EventsOutput {
            columns: plan.columns(),
            rows: vec![0, 1],
            evaluated_count: 2,
            matched_count: 2,
            truncated_at: None,
        };
        let rows = vec![
            ResultRow::Event {
                event: LogEvent {
                    timestamp: 1_704_164_645_123,
                    ingestion_time: None,
                    message: "{\"level\":\"ERROR\",\"msg\":\"a\\nb\"}".to_string(),
                    log_stream_name: "s".to_string(),
                    sequence: 0,
                },
                log_group_name: GROUP.to_string(),
            },
            ResultRow::Event {
                event: LogEvent {
                    timestamp: 1_704_164_646_000,
                    ingestion_time: None,
                    message: "not json\nsecond line".to_string(),
                    log_stream_name: "s".to_string(),
                    sequence: 1,
                },
                log_group_name: GROUP.to_string(),
            },
        ];
        session.finish_evaluation(QueryResult::from_output(run_id, output, rows), now);
        let source = session.export_source(UTC).unwrap();
        assert_eq!(source.row_count(), 2);
        let snapshot = source.into_snapshot();
        assert_eq!(snapshot.columns, ["@timestamp", "level", "@message"]);
        assert_eq!(
            snapshot.rows,
            vec![
                vec![
                    "2024/01/02 03:04:05.123".to_string(),
                    "ERROR".to_string(),
                    "{\"level\":\"ERROR\",\"msg\":\"a\\nb\"}".to_string(),
                ],
                vec![
                    "2024/01/02 03:04:06.000".to_string(),
                    String::new(),
                    "not json\nsecond line".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn the_export_default_name_uses_the_first_log_group_of_the_run() {
        let now = Instant::now();
        let mut session = ready();
        assert_eq!(session.export_log_group_name(), Some(GROUP));
        done(&mut session, now, vec![1]);
        assert_eq!(session.export_log_group_name(), Some(GROUP));
        // Changing the choice after the run keeps the run's own log group.
        session
            .update(AnalyticsField::LogGroup, String::new(), UTC, listed)
            .unwrap();
        assert_eq!(session.selected_log_groups(), &[] as &[String]);
        assert_eq!(session.export_log_group_name(), Some(GROUP));
        // No run yet and nothing chosen: no name (the file name falls back).
        let fresh = InsightsSession::new();
        assert_eq!(fresh.export_log_group_name(), None);
    }

    // ---- 261025-insights-stats U1:BR4.1-BR4.4 ----------------------------

    /// 2025-08-24 16:00:00 UTC, 2025-08-25 01:00:00 in Tokyo.
    const FOUR_PM: i64 = 1_756_051_200_000;

    /// Finishes a run with the aggregates of `text` over JSON `messages`, one
    /// a minute from [`FOUR_PM`]; returns the run number.
    fn aggregated(
        session: &mut InsightsSession,
        now: Instant,
        text: &str,
        messages: &[&str],
    ) -> u64 {
        use crate::query::evaluator::{EvalItem, EvalOutput, evaluate};
        let events: Vec<LogEvent> = messages
            .iter()
            .enumerate()
            .map(|(index, message)| LogEvent {
                timestamp: FOUR_PM + i64::try_from(index).unwrap() * 60_000,
                ingestion_time: None,
                message: (*message).to_string(),
                log_stream_name: "s".to_string(),
                sequence: u64::try_from(index).unwrap(),
            })
            .collect();
        let items: Vec<EvalItem<'_>> = events
            .iter()
            .map(|event| EvalItem {
                event,
                log_group_name: GROUP,
            })
            .collect();
        let plan = parse_query(text).unwrap();
        let Some(EvalOutput::Aggregates(output)) =
            evaluate(&plan, items.as_slice(), || false, |_, _| {})
        else {
            panic!("{text:?} gave no aggregates");
        };
        let run_id = evaluating(session, now, u64::try_from(events.len()).unwrap());
        // As the runner gives it (261025-insights-stats U3:BR4.2).
        let keys = plan.stats().map_or(0, |stats| stats.group_keys.len());
        session.finish_evaluation(
            QueryResult::from_aggregates(run_id, output).with_key_count(keys),
            now,
        );
        run_id
    }

    const LEVELS: [&str; 4] = [
        r#"{"level":"error","latency":100}"#,
        r#"{"level":"info","latency":200}"#,
        r#"{"level":"error","latency":"101"}"#,
        r#"{"level":"info"}"#,
    ];

    #[test]
    fn the_summary_of_an_aggregates_result_has_its_shape_and_names() {
        // BR4.1, BR3.4.
        let now = Instant::now();
        let mut session = ready();
        aggregated(
            &mut session,
            now,
            "stats count(*) as n, avg(latency) by level",
            &LEVELS,
        );
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(summary.shape, ResultShape::Aggregates);
        assert_eq!(summary.columns, vec!["level", "n", "avg(latency)"]);
        assert_eq!(
            (
                summary.row_count,
                summary.matched_count,
                summary.evaluated_count,
                summary.truncated_at
            ),
            (2, 2, 4, None)
        );
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["shape"], "aggregates");
        let mut events = ready();
        done(&mut events, now, vec![1]);
        let json = serde_json::to_value(events.view(&connected(), now).result.unwrap()).unwrap();
        assert_eq!(json["shape"], "events", "an events result says so too");
    }

    #[test]
    fn an_aggregate_row_has_only_its_cells_as_the_table_writes_them() {
        // BR4.1, BR2.8: values only, no event, no time, no log group.
        let now = Instant::now();
        let mut session = ready();
        let run_id = aggregated(
            &mut session,
            now,
            "stats count(*) as n, avg(latency), max(gone) by level",
            &LEVELS,
        );
        let window = session.display_rows(run_id, 1, (0, 50), UTC);
        assert_eq!(window.total_count, 2);
        assert_eq!(
            window
                .rows
                .iter()
                .map(|row| row.values.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![
                    Some("error".to_string()),
                    Some("2".to_string()),
                    Some("100.5".to_string()),
                    None
                ],
                vec![
                    Some("info".to_string()),
                    Some("2".to_string()),
                    Some("200".to_string()),
                    None
                ],
            ]
        );
        assert_eq!(window.rows[0].event, None);
        let json = serde_json::to_value(&window.rows[0]).unwrap();
        // 261025-insights-stats U3:BR4.2: every row also carries its row
        // key, an aggregate row's made of its key cells.
        assert_eq!(
            json,
            serde_json::json!({ "values": ["error", "2", "100.5", null], "rowKey": "error" })
        );
        let mut events = ready();
        let events_run = done(&mut events, now, vec![7]);
        let row =
            serde_json::to_value(&events.display_rows(events_run, 1, (0, 1), UTC).rows[0]).unwrap();
        assert_eq!(row["displayTime"], "1970/01/01 00:00:00.007");
        assert_eq!(row["logGroupName"], GROUP);
        assert_eq!(row["timestamp"], 7, "an event row keeps its event");
    }

    #[test]
    fn the_time_cells_follow_the_chosen_zone_and_nothing_else_changes() {
        // BR4.2, FR2.3.
        let now = Instant::now();
        let mut session = ready();
        let run_id = aggregated(
            &mut session,
            now,
            "stats count(*), min(@timestamp) by bin(2m)",
            &LEVELS,
        );
        let utc = session.display_rows(run_id, 1, (0, 50), UTC);
        let tokyo = session.display_rows(run_id, 1, (0, 50), chrono_tz::Asia::Tokyo);
        let cells = |window: &QueryRowWindow| -> Vec<Vec<Option<String>>> {
            window.rows.iter().map(|row| row.values.clone()).collect()
        };
        assert_eq!(
            cells(&utc),
            vec![
                vec![
                    Some("2025/08/24 16:00:00.000".to_string()),
                    Some("2".to_string()),
                    Some("2025/08/24 16:00:00.000".to_string()),
                ],
                vec![
                    Some("2025/08/24 16:02:00.000".to_string()),
                    Some("2".to_string()),
                    Some("2025/08/24 16:02:00.000".to_string()),
                ],
            ]
        );
        assert_eq!(
            cells(&tokyo)[1],
            vec![
                Some("2025/08/25 01:02:00.000".to_string()),
                Some("2".to_string()),
                Some("2025/08/25 01:02:00.000".to_string()),
            ]
        );
        assert_eq!(utc.total_count, tokyo.total_count);
    }

    #[test]
    fn an_opened_aggregate_row_lists_its_columns_with_their_cells() {
        // BR4.3.
        let now = Instant::now();
        let mut session = ready();
        let run_id = aggregated(
            &mut session,
            now,
            "stats count(*) as n, max(gone) by level",
            &LEVELS,
        );
        let detail = session.display_row_detail(run_id, 1, 1, UTC);
        assert_eq!(
            detail
                .fields
                .iter()
                .map(|field| (field.name.as_str(), field.value.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("level", Some("info")),
                ("n", Some("2")),
                ("max(gone)", None)
            ]
        );
        assert!(
            session
                .display_row_detail(run_id + 1, 1, 0, UTC)
                .fields
                .is_empty()
        );
        assert!(
            session
                .display_row_detail(run_id, 1, 9, UTC)
                .fields
                .is_empty()
        );
    }

    #[test]
    fn an_aggregates_result_is_exported_with_the_table_text() {
        // BR4.4.
        let now = Instant::now();
        let mut session = ready();
        aggregated(
            &mut session,
            now,
            "stats count(*), max(gone) by bin(5m)",
            &LEVELS,
        );
        assert!(session.can_export(false).is_available());
        let snapshot = session
            .export_source(chrono_tz::Asia::Tokyo)
            .unwrap()
            .into_snapshot();
        assert_eq!(snapshot.columns, vec!["bin(5m)", "count(*)", "max(gone)"]);
        assert_eq!(
            snapshot.rows,
            vec![vec![
                "2025/08/25 01:00:00.000".to_string(),
                "4".to_string(),
                String::new()
            ]]
        );
    }

    #[test]
    fn no_aggregate_row_is_a_result_of_zero_rows() {
        // BR2.6, FR3.2: with `by` and nothing left, the result has no row
        // and nothing to export; without `by` it has one.
        let now = Instant::now();
        let mut session = ready();
        let run_id = aggregated(
            &mut session,
            now,
            "filter level = 'none' | stats count(*) by level",
            &LEVELS,
        );
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(
            (summary.shape, summary.row_count),
            (ResultShape::Aggregates, 0)
        );
        assert_eq!(summary.columns, vec!["level", "count(*)"]);
        assert!(
            session
                .display_rows(run_id, 1, (0, 50), UTC)
                .rows
                .is_empty()
        );
        assert_eq!(
            session.can_export(false).blocker,
            Some(ExportBlocker::NoRows)
        );
        let one = aggregated(
            &mut session,
            now,
            "filter level = 'none' | stats count(*)",
            &LEVELS,
        );
        assert_eq!(
            session.display_rows(one, 1, (0, 50), UTC).rows[0].values,
            vec![Some("0".to_string())]
        );
    }

    fn kinds_of(session: &InsightsSession) -> Vec<(String, CompletionKind)> {
        session
            .completion_candidates()
            .into_iter()
            .map(|candidate| (candidate.text, candidate.kind))
            .collect()
    }

    fn kind_in(candidates: &[(String, CompletionKind)], text: &str) -> Option<CompletionKind> {
        candidates
            .iter()
            .find(|(candidate, _)| candidate == text)
            .map(|(_, kind)| *kind)
    }

    #[test]
    fn stats_by_as_and_every_function_are_completed() {
        // U1 offered count..bin as provisional keywords; U2:BR4.3 offers
        // the twelve functions and bin as functions, from the catalog
        // (changed as BR5.1 allows).
        let candidates = kinds_of(&ready());
        assert_eq!(kind_in(&candidates, "stats"), Some(CompletionKind::Command));
        for word in ["by", "as"] {
            assert_eq!(
                kind_in(&candidates, word),
                Some(CompletionKind::Keyword),
                "{word}"
            );
        }
        for word in [
            "count",
            "count_distinct",
            "sum",
            "avg",
            "min",
            "max",
            "pct",
            "stddev",
            "earliest",
            "latest",
            "sortsFirst",
            "sortsLast",
            "bin",
        ] {
            assert_eq!(
                kind_in(&candidates, word),
                Some(CompletionKind::Function),
                "{word}"
            );
        }
        assert_eq!(
            serde_json::to_value(CompletionKind::Function).unwrap(),
            serde_json::json!("Function")
        );
    }

    #[test]
    fn the_earlier_candidates_keep_their_texts_and_kinds() {
        // U2:BR4.1: the words of 261017-insights-query U2:BR9.1 stay as they
        // were; the console's commands this version does not run are not
        // offered.
        let candidates = kinds_of(&ready());
        for word in ["fields", "filter", "sort", "limit"] {
            assert_eq!(
                kind_in(&candidates, word),
                Some(CompletionKind::Command),
                "{word}"
            );
        }
        for word in ["and", "or", "not", "like", "in", "asc", "desc"] {
            assert_eq!(
                kind_in(&candidates, word),
                Some(CompletionKind::Keyword),
                "{word}"
            );
        }
        for word in [
            "@timestamp",
            "@message",
            "@logStream",
            "@log",
            "toMillis(@timestamp)",
        ] {
            assert_eq!(
                kind_in(&candidates, word),
                Some(CompletionKind::FixedField),
                "{word}"
            );
        }
        for word in ["parse", "dedup", "filterIndex", "toMillis"] {
            assert_eq!(kind_in(&candidates, word), None, "{word}");
        }
        let mut texts: Vec<&str> = candidates.iter().map(|(text, _)| text.as_str()).collect();
        texts.sort_unstable();
        let before = texts.len();
        texts.dedup();
        assert_eq!(texts.len(), before, "every candidate once");
        assert_eq!(before, 5 + 9 + 5 + 13);
    }

    #[test]
    fn the_json_field_names_follow_the_catalog_words() {
        // U2:BR4.3 with 261017-insights-query U2:BR9.1: the names of the
        // last fetch come after the fixed words.
        let mut session = ready();
        session.field_names = vec!["http.status".to_string(), "level".to_string()];
        let candidates = kinds_of(&session);
        let fixed = catalog_candidates().len();
        assert_eq!(
            candidates[fixed..].to_vec(),
            vec![
                ("http.status".to_string(), CompletionKind::JsonField),
                ("level".to_string(), CompletionKind::JsonField),
            ]
        );
    }

    #[test]
    fn the_cut_of_an_aggregates_result_reaches_the_summary() {
        // U2:BR3.2: matched before the cut and the rows shown.
        let now = Instant::now();
        let mut session = ready();
        let messages: Vec<String> = (0..10_003).map(|n| format!(r#"{{"n":{n}}}"#)).collect();
        let messages: Vec<&str> = messages.iter().map(String::as_str).collect();
        aggregated(&mut session, now, "stats count(*) by n", &messages);
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(
            (
                summary.row_count,
                summary.matched_count,
                summary.truncated_at
            ),
            (10_000, 10_003, Some(10_000))
        );
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["truncatedAt"], 10_000);
        assert_eq!(json["matchedCount"], 10_003);
        aggregated(
            &mut session,
            now,
            "stats count(*) by n | limit 3",
            &messages,
        );
        let limited = session.view(&connected(), now).result.unwrap();
        assert_eq!((limited.row_count, limited.truncated_at), (3, Some(3)));
    }

    // ---- 261025-insights-stats U3 (test first) --------------------------

    use crate::query::evaluator::AggregatesOutput;
    use crate::query::runner::FetchEnd;
    use crate::query::stats::CellValue;

    /// Group 1 of 1, `count` events in the partial view.
    fn coverage(count: u64) -> PartialCoverage {
        PartialCoverage {
            group_number: 1,
            group_count: 1,
            included_event_count: count,
        }
    }

    /// An event row of [`GROUP`] at `time` on stream `stream`, `sequence`.
    fn event_row(time: i64, stream: &str, sequence: u64) -> ResultRow {
        ResultRow::Event {
            event: LogEvent {
                timestamp: time,
                ingestion_time: None,
                message: format!("row {time}"),
                log_stream_name: stream.to_string(),
                sequence,
            },
            log_group_name: GROUP.to_string(),
        }
    }

    /// An aggregates result of `run_id`: one key column `level`, then `n`.
    fn aggregates(run_id: u64, rows: &[(&str, i64)]) -> QueryResult {
        QueryResult::from_aggregates(
            run_id,
            AggregatesOutput {
                columns: vec!["level".to_string(), "n".to_string()],
                rows: rows
                    .iter()
                    .map(|(level, n)| {
                        vec![
                            CellValue::Text((*level).to_string()),
                            CellValue::Integer(*n),
                        ]
                    })
                    .collect(),
                evaluated_count: 3,
                matched_count: rows.len(),
                truncated_at: None,
            },
        )
        .with_key_count(1)
    }

    /// `(run_id, version, stage)` of the shown result.
    fn shown(session: &InsightsSession) -> Option<(u64, u64, ResultStage)> {
        session
            .view(&connected(), Instant::now())
            .result
            .map(|summary| (summary.run_id, summary.version, summary.stage))
    }

    #[test]
    fn partial_results_of_the_current_run_move_the_version_on_and_are_shown() {
        // BR2.4, BR2.6, BR4.1.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert!(
            session.on_partial_result(QueryResult::new(run_id, 2, copies(vec![1])), coverage(2))
        );
        assert_eq!(shown(&session), Some((run_id, 1, ResultStage::Partial)));
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(summary.coverage, Some(coverage(2)));
        assert!(
            session.on_partial_result(QueryResult::new(run_id, 5, copies(vec![2, 1])), coverage(5))
        );
        assert_eq!(shown(&session), Some((run_id, 2, ResultStage::Partial)));
        assert_eq!(session.phase(), AnalyticsPhase::Fetching, "still fetching");
        let json = serde_json::to_value(session.view(&connected(), now).result).unwrap();
        assert_eq!(json["stage"], "partial");
        assert_eq!(json["version"], 2);
        assert_eq!(json["coverage"]["groupNumber"], 1);
        assert_eq!(json["coverage"]["includedEventCount"], 5);
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 5), now),
            FetchEnd::Evaluate
        );
        session.finish_evaluation(QueryResult::new(run_id, 5, copies(vec![3, 2, 1])), now);
        assert_eq!(shown(&session), Some((run_id, 3, ResultStage::Final)));
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!(
            summary.coverage, None,
            "the mark goes with the final result"
        );
    }

    #[test]
    fn results_of_another_run_or_after_the_run_are_dropped() {
        // BR2.4: an old run's, and a partial one after the final.
        let now = Instant::now();
        let mut session = ready();
        let old = done(&mut session, now, vec![1]);
        let current = start(&mut session, now).run_id;
        assert!(!session.on_partial_result(QueryResult::new(old, 1, copies(vec![9])), coverage(1)));
        assert_eq!(
            shown(&session),
            Some((old, 1, ResultStage::Final)),
            "the previous result"
        );
        assert_eq!(
            session.finish_fetch(current, &ended(JobStatus::Completed, 1), now),
            FetchEnd::Evaluate
        );
        assert!(
            !session.on_partial_result(QueryResult::new(current, 1, copies(vec![9])), coverage(1)),
            "not while evaluating the final result"
        );
        session.finish_evaluation(QueryResult::new(current, 1, copies(vec![4])), now);
        assert!(
            !session.on_partial_result(QueryResult::new(current, 1, copies(vec![9])), coverage(1))
        );
        assert_eq!(shown(&session), Some((current, 1, ResultStage::Final)));
    }

    #[test]
    fn the_previous_result_stays_until_the_first_version_of_the_run_whatever_its_shape() {
        // BR2.5, FR4.7: replaced at the first partial result, even by
        // another shape.
        let now = Instant::now();
        let mut session = ready();
        let old = done(&mut session, now, vec![1, 2]);
        let current = start(&mut session, now).run_id;
        assert_eq!(shown(&session), Some((old, 1, ResultStage::Final)));
        assert!(session.on_partial_result(aggregates(current, &[("error", 1)]), coverage(1)));
        let summary = session.view(&connected(), now).result.unwrap();
        assert_eq!((summary.run_id, summary.version), (current, 1));
        assert_eq!(summary.shape, ResultShape::Aggregates);
        assert!(
            session.display_rows(old, 1, (0, 10), UTC).rows.is_empty(),
            "replaced"
        );
    }

    #[test]
    fn a_final_result_without_partial_ones_is_the_first_version() {
        // BR2.5, UC2: nothing fetched from AWS, no partial result.
        let now = Instant::now();
        let mut session = ready();
        let old = done(&mut session, now, vec![1]);
        let current = done(&mut session, now, vec![2]);
        assert_ne!(old, current);
        assert_eq!(shown(&session), Some((current, 1, ResultStage::Final)));
        assert_eq!(
            session
                .result_version()
                .map(|version| (version.run_id, version.version)),
            Some((current, 1))
        );
    }

    #[test]
    fn cancelling_counts_as_running_for_every_check() {
        // BR3.7.
        let now = Instant::now();
        let mut session = ready();
        done(&mut session, now, vec![1]);
        let run_id = start(&mut session, now).run_id;
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        assert_eq!(session.phase(), AnalyticsPhase::Cancelling);
        assert!(session.is_running() && session.is_cancelling());
        assert!(session.holds_fetch(), "the fetch is still stopping");
        let view = session.view(&connected(), now);
        assert!(!view.can_run && view.cancelling);
        assert_eq!(
            view.run_blocker.as_deref(),
            Some("insights.blocked.queryRunning")
        );
        assert_eq!(
            session.update(
                AnalyticsField::QueryText,
                "limit 1".to_string(),
                UTC,
                listed
            ),
            Err(SessionError::Busy)
        );
        assert!(!session.can_export(false).is_available(), "Running");
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Cancelled, 0), now),
            FetchEnd::EvaluateCancelled
        );
        assert!(
            session.is_running() && !session.holds_fetch(),
            "evaluating what was fetched"
        );
    }

    #[test]
    fn cancel_is_pressed_once_then_once_more_while_cancelling_and_never_a_third_time() {
        // BR3.2 (FR review R-04).
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert!(session.cancel_available(false));
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        assert!(
            session.cancel_available(false),
            "a second time while cancelling"
        );
        assert!(
            !session.cancel_available(true),
            "not while closing awaits an answer"
        );
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Abandon(run_id))
        );
        assert!(!session.cancel_available(false));
        assert_eq!(session.request_cancel(false), None, "no third time");
    }

    #[test]
    fn a_cancel_while_fetching_ends_with_a_cancelled_result_of_what_was_fetched() {
        // BR3.1, BR3.4, FR5.1, FR5.4: evaluated once; the failed streams
        // stay shown with it.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        let mut summary = ended(JobStatus::CompletedWithFailures, 4);
        summary.stopped = true;
        summary.groups[0].failed_streams.push(GroupFailedStream {
            log_group_name: GROUP.to_string(),
            log_stream_name: "b".to_string(),
            failure: ApiFailure::new(FailureKind::AccessDenied, &SafeDetailFields::default()),
        });
        assert_eq!(
            session.finish_fetch(run_id, &summary, now),
            FetchEnd::EvaluateCancelled
        );
        assert_eq!(session.phase(), AnalyticsPhase::Cancelling);
        assert!(
            session.on_evaluation_progress(run_id, 10_000, 20_000),
            "the first [Cancel] does not stop it"
        );
        session.finish_evaluation(QueryResult::new(run_id, 4, copies(vec![3, 1])), now);
        let view = session.view(&connected(), now);
        assert_eq!(view.phase, AnalyticsPhase::Cancelled);
        assert_eq!(
            shown(&session),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );
        assert_eq!(view.failed_streams.len(), 1, "both shown");
        assert!(view.partial);
        assert!(!view.cancel_available && !view.cancelling);
        assert!(session.can_export(false).is_available(), "BR3.5");
    }

    #[test]
    fn a_second_cancel_before_the_fetch_ends_keeps_the_partial_result_as_cancelled() {
        // BR3.2: not evaluated; the version does not move on.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert!(
            session.on_partial_result(QueryResult::new(run_id, 2, copies(vec![1])), coverage(2))
        );
        session.request_cancel(false);
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Abandon(run_id))
        );
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Cancelled, 2), now),
            FetchEnd::Skip
        );
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
        assert_eq!(
            shown(&session),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );
        assert_eq!(
            session
                .view(&connected(), now)
                .result
                .and_then(|summary| summary.coverage),
            None
        );
    }

    #[test]
    fn a_second_cancel_without_a_partial_result_leaves_the_previous_one_as_it_was() {
        // BR3.2 ELSE: the current Cancelled.
        let now = Instant::now();
        let mut session = ready();
        let old = done(&mut session, now, vec![1]);
        let run_id = start(&mut session, now).run_id;
        session.request_cancel(false);
        session.request_cancel(false);
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Cancelled, 0), now),
            FetchEnd::Skip
        );
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
        assert_eq!(
            shown(&session),
            Some((old, 1, ResultStage::Final)),
            "the previous result"
        );
    }

    #[test]
    fn a_second_cancel_stops_the_cancelled_evaluation_and_keeps_the_partial_result() {
        // BR3.2: stopped between chunks.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert!(
            session.on_partial_result(QueryResult::new(run_id, 2, copies(vec![1])), coverage(2))
        );
        session.request_cancel(false);
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Cancelled, 2), now),
            FetchEnd::EvaluateCancelled
        );
        assert!(
            !session
                .on_partial_result(QueryResult::new(run_id, 2, copies(vec![1, 2])), coverage(2)),
            "no partial result after the fetch"
        );
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Abandon(run_id))
        );
        assert!(!session.on_evaluation_progress(run_id, 10_000, 20_000));
        session.stop_evaluation(run_id, now);
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
        assert_eq!(
            shown(&session),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );
        // A result that comes after the stop changes nothing.
        session.finish_evaluation(QueryResult::new(run_id, 2, copies(vec![5])), now);
        assert_eq!(
            shown(&session),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );
    }

    #[test]
    fn a_cancel_while_evaluating_the_final_result_keeps_the_partial_result_as_cancelled() {
        // BR3.3: stopped as before; the partial result stays marked.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert!(
            session.on_partial_result(QueryResult::new(run_id, 2, copies(vec![1])), coverage(2))
        );
        assert_eq!(
            session.finish_fetch(run_id, &ended(JobStatus::Completed, 3), now),
            FetchEnd::Evaluate
        );
        assert_eq!(
            session.request_cancel(false),
            Some(CancelRequest::Stop(run_id))
        );
        assert!(!session.on_evaluation_progress(run_id, 1, 3));
        session.stop_evaluation(run_id, now);
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
        assert_eq!(
            shown(&session),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );

        // After the last step: the final result is dropped, the partial one
        // kept.
        let mut late = ready();
        let run_id = start(&mut late, now).run_id;
        assert!(late.on_partial_result(QueryResult::new(run_id, 2, copies(vec![1])), coverage(2)));
        late.finish_fetch(run_id, &ended(JobStatus::Completed, 3), now);
        late.request_cancel(false);
        late.finish_evaluation(QueryResult::new(run_id, 3, copies(vec![2, 1])), now);
        assert_eq!(
            shown(&late),
            Some((run_id, 1, ResultStage::CancelledPartial))
        );
    }

    #[test]
    fn row_keys_tell_rows_apart_across_versions() {
        // BR4.2 (Q4 A).
        let event = event_row(5, "s1", 7);
        assert_eq!(row_key(&event, 0), format!("{GROUP}\u{1f}s1\u{1f}7"));
        let aggregate = ResultRow::Aggregate {
            cells: vec![
                CellValue::Text("error".to_string()),
                CellValue::Timestamp(60_000),
                CellValue::Integer(3),
            ],
        };
        assert_eq!(
            row_key(&aggregate, 2),
            "error\u{1f}60000",
            "the key cells only"
        );
        assert_eq!(row_key(&aggregate, 0), "", "no `by`: one row, an empty key");
        let missing = ResultRow::Aggregate {
            cells: vec![CellValue::Missing, CellValue::Integer(3)],
        };
        assert_eq!(row_key(&missing, 1), "");
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        session.on_partial_result(
            aggregates(run_id, &[("error", 1), ("info", 2)]),
            coverage(3),
        );
        let keys: Vec<String> = session
            .display_rows(run_id, 1, (0, 10), UTC)
            .rows
            .into_iter()
            .map(|row| row.row_key)
            .collect();
        assert_eq!(keys, vec!["error", "info"]);
    }

    #[test]
    fn rows_details_and_row_positions_are_read_for_the_shown_version_only() {
        // BR2.4, BR2.8, BR4.1.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        session.on_partial_result(
            QueryResult::new(run_id, 2, vec![event_row(9, "a", 1), event_row(5, "a", 0)]),
            coverage(2),
        );
        session.on_partial_result(
            QueryResult::new(
                run_id,
                3,
                vec![
                    event_row(9, "a", 1),
                    event_row(7, "b", 0),
                    event_row(5, "a", 0),
                ],
            ),
            coverage(3),
        );
        assert!(
            session
                .display_rows(run_id, 1, (0, 10), UTC)
                .rows
                .is_empty(),
            "an old version"
        );
        let window = session.display_rows(run_id, 2, (0, 10), UTC);
        assert_eq!(window.rows.len(), 3);
        assert_eq!(window.version, 2);
        assert!(
            session
                .display_row_detail(run_id, 1, 0, UTC)
                .fields
                .is_empty()
        );
        assert!(
            !session
                .display_row_detail(run_id, 2, 0, UTC)
                .fields
                .is_empty()
        );
        let key = format!("{GROUP}\u{1f}a\u{1f}0");
        assert_eq!(
            session.find_row(run_id, 2, &key),
            Some(2),
            "moved down by the new row"
        );
        assert_eq!(session.find_row(run_id, 1, &key), None, "an old version");
        assert_eq!(session.find_row(run_id, 2, "no such row"), None);
        assert_eq!(session.find_row(run_id + 1, 2, &key), None);
    }

    #[test]
    fn fetch_notices_still_count_while_the_fetch_stops() {
        // BR3.7: the progress goes on while Cancelling.
        let now = Instant::now();
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        let fetch = job(JobStatus::Running, 0);
        session.on_job_started(run_id, &fetch);
        session.request_cancel(false);
        session.on_batch(
            run_id,
            fetch.job_id,
            BatchProgress {
                added: 4,
                total: 4,
                timeline_version: 1,
            },
        );
        assert_eq!(
            session
                .view(&connected(), now)
                .progress
                .map(|p| p.event_count),
            Some(4)
        );
    }
}
