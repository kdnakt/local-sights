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

use std::time::Instant;

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::catalog::ProfileSelector;
use crate::coordinator::{BatchProgress, FailedStream, FetchJob};
use crate::date_input::DateTimeInput;
use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::fetch_status::JobStatus;
use crate::log_groups::ListingStatus;
use crate::query::evaluator::{QueryResult, row_cells, row_detail};
use crate::query::fields::EventFields;
use crate::query::plan::{QueryPlan, QueryProblem};
use crate::query::store::{HeldRows, HoldingKey};
use crate::request::{ValidatedFetch, ValidationError, message_keys, validate_session_input};
use crate::session::{ElapsedView, SessionError};
use crate::streams::planner::ListingProgress;
use crate::time_range::format_display_in_zone;

/// Longest query text, in Unicode scalar values: the console's limit
/// (StartQuery's queryString, functional-design Q4).
pub const MAX_QUERY_CHARS: usize = 10_000;

/// The query the screen opens with (entities.md AnalyticsInput.queryText):
/// the console's default columns, newest first, with U1's filter of errors.
pub const DEFAULT_QUERY: &str = "fields @timestamp, @message, @logStream, @log\n\
     | filter @message like /ERROR/\n\
     | sort @timestamp desc\n\
     | limit 10000";

/// The commands a query can use, for completion (U2:BR2.1, BR9.1).
const COMMAND_WORDS: [&str; 4] = ["fields", "filter", "sort", "limit"];

/// The other words, for completion (U2:BR9.1).
const KEYWORD_WORDS: [&str; 7] = ["and", "or", "not", "like", "in", "asc", "desc"];

/// The fixed fields and the one function, for completion (U2:BR9.1).
const FIXED_FIELD_WORDS: [&str; 5] = [
    "@timestamp",
    "@message",
    "@logStream",
    "@log",
    "toMillis(@timestamp)",
];

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
    /// The log group; an empty value deselects it.
    LogGroup,
    /// The start date and time.
    StartText,
    /// The end date and time.
    EndText,
    /// The query text.
    QueryText,
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

/// What a run that was accepted needs (functional-spec UC1 6-9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryStart {
    /// The new run.
    pub run_id: u64,
    /// The range and the connection to fetch.
    pub fetch: ValidatedFetch,
    /// What the holdings are for.
    pub key: HoldingKey,
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

/// Progress of the running query as the screen shows it (U1:BR2.4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightsProgress {
    /// Streams selected by the listing so far.
    pub selected_stream_count: u32,
    /// Streams to fetch; `None` while the streams are being listed.
    pub planned_stream_count: Option<u32>,
    /// Streams finished.
    pub finished_stream_count: u32,
    /// Events fetched so far.
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
    /// The run that made it.
    pub run_id: u64,
    /// The holdings epoch its rows point into.
    pub holdings_epoch: u64,
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
}

/// The Log Analytics screen as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightsView {
    /// The selected log group (U1:BR1.3).
    pub selected_log_group_name: Option<String>,
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
    /// The failure of the whole fetch when FetchFailed (U1:BR2.6).
    pub failure: Option<ApiFailure>,
    /// Whether some streams failed: the result is partial (U1:BR2.5).
    pub partial: bool,
    /// The streams that failed (name and safe reason only).
    pub failed_streams: Vec<FailedStream>,
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
}

/// One result row as the screen shows it (U1:BR6.3, U2:BR7.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRow {
    /// The event as held.
    #[serde(flatten)]
    pub event: LogEvent,
    /// `@timestamp` in the chosen zone, as on Log Management.
    pub display_time: String,
    /// `@log`.
    pub log_group_name: String,
    /// One cell per result column, `None` without a value (U2:BR7.1).
    pub values: Vec<Option<String>>,
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
    /// The holdings epoch asked for.
    pub holdings_epoch: u64,
    /// The row's place in the result.
    pub index: u64,
    /// The fixed fields, then the JSON fields by name; empty when the run
    /// or the epoch is not the shown one.
    pub fields: Vec<DetailField>,
}

/// The result rows of one viewport request (U1:BR6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRowWindow {
    /// The run asked for.
    pub run_id: u64,
    /// The holdings epoch asked for.
    pub holdings_epoch: u64,
    /// Position of the first row.
    pub offset: u64,
    /// The rows; empty when the run or the epoch is not the shown one.
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
    job_id: Option<u64>,
    progress: Option<InsightsProgress>,
    partial: bool,
    failed_streams: Vec<FailedStream>,
    listing_failure: Option<ApiFailure>,
    failure: Option<ApiFailure>,
    problem: Option<QueryProblem>,
    result: Option<QueryResult>,
    cache_saving: bool,
    started_at: Option<Instant>,
    final_ms: Option<u64>,
}

/// The Log Analytics screen's input and last run.
#[derive(Debug, Clone)]
pub struct InsightsSession {
    selected_log_group: Option<String>,
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

/// The screen's summary of `result` (U1:BR6.4, U2:BR5.3, BR7.2).
fn summary_of(result: &QueryResult) -> ResultSummary {
    ResultSummary {
        run_id: result.run_id,
        holdings_epoch: result.holdings_epoch,
        row_count: count(result.row_count()),
        evaluated_count: count(result.evaluated_count),
        columns: result
            .columns()
            .iter()
            .map(|column| column.name().to_string())
            .collect(),
        matched_count: count(result.matched_count()),
        truncated_at: result.truncated_at().map(count),
    }
}

impl InsightsSession {
    /// Nothing selected, empty times and the example query.
    pub fn new() -> Self {
        Self {
            selected_log_group: None,
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

    /// The selected log group.
    pub fn selected_log_group(&self) -> Option<&str> {
        self.selected_log_group.as_deref()
    }

    /// Whether the run is fetching (U1:BR2.3: it owns the one fetch).
    pub fn is_fetching(&self) -> bool {
        self.run.phase == AnalyticsPhase::Fetching
    }

    /// Whether the run is evaluating.
    pub fn is_evaluating(&self) -> bool {
        self.run.phase == AnalyticsPhase::Evaluating
    }

    /// Fetching or evaluating (U1:BR1.5 (6)).
    pub fn is_running(&self) -> bool {
        self.is_fetching() || self.is_evaluating()
    }

    /// The shown result, if any.
    pub fn result(&self) -> Option<&QueryResult> {
        self.run.result.as_ref()
    }

    /// Whether a result with at least one row is shown (U1:BR7.1).
    pub fn has_result_rows(&self) -> bool {
        self.run.phase == AnalyticsPhase::Done
            && self.run.result.as_ref().is_some_and(|r| r.row_count() > 0)
    }

    /// Changes one field (U1:BR1.3, BR1.4). Refused while running
    /// (U1:BR2.4). `listed` says whether a log group is in the current list.
    /// Returns whether the input changed.
    pub fn update(
        &mut self,
        field: AnalyticsField,
        value: String,
        zone: Tz,
        listed: impl Fn(&str) -> bool,
    ) -> Result<bool, SessionError> {
        if self.is_running() {
            return Err(SessionError::Busy);
        }
        let changed = match field {
            AnalyticsField::LogGroup => {
                let selected = (!value.is_empty()).then_some(value);
                if let Some(name) = &selected {
                    if !listed(name) {
                        return Err(SessionError::UnknownLogGroup);
                    }
                }
                let changed = selected != self.selected_log_group;
                self.selected_log_group = selected;
                changed
            }
            AnalyticsField::StartText => self.start_input.edit(value, zone),
            AnalyticsField::EndText => self.end_input.edit(value, zone),
            AnalyticsField::QueryText => {
                let changed = value != self.query_text;
                self.query_text = value;
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
        } else if self.selected_log_group.is_none() {
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

    /// [Run query] with the query read from `revision` (UC1 2-7, R-04): in
    /// one call, the reasons are checked again, the revision must still be
    /// the input's, a new run is numbered and either holds the reading
    /// problem (QueryError, the previous result stays, U1:BR3.9) or starts
    /// fetching with the previous result gone (U1:BR2.9).
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
        let (Some((profile, region)), Some(group)) =
            (context.connection.clone(), self.selected_log_group.clone())
        else {
            return Err(SessionError::Invalid);
        };
        let validated = validate_session_input(&group, &self.start_input, &self.end_input)
            .map_err(|_| SessionError::Invalid)?;
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
        self.run = InsightsRun {
            run_id,
            phase: AnalyticsPhase::Fetching,
            progress: Some(InsightsProgress::default()),
            started_at: Some(now),
            ..InsightsRun::default()
        };
        Ok(QueryBegin::Fetch(QueryStart {
            run_id,
            fetch: ValidatedFetch {
                request: validated
                    .request
                    .with_connection(profile.clone(), region.clone()),
                range: validated.range,
            },
            key: HoldingKey {
                profile,
                region,
                log_group_name: group,
            },
        }))
    }

    /// The fetching run `run_id`, or `None` for another run (U1:BR6.2).
    fn fetching(&mut self, run_id: u64) -> Option<&mut InsightsRun> {
        (self.run.run_id == run_id && self.run.phase == AnalyticsPhase::Fetching)
            .then_some(&mut self.run)
    }

    fn progress_of(&mut self, run_id: u64, job_id: u64) -> Option<&mut InsightsProgress> {
        let run = self.fetching(run_id)?;
        if run.job_id != Some(job_id) {
            return None;
        }
        run.progress.as_mut()
    }

    /// The fetch of `run_id` started `job`.
    pub fn on_job_started(&mut self, run_id: u64, job: &FetchJob) {
        if let Some(run) = self.fetching(run_id) {
            run.job_id = Some(job.job_id);
            run.progress = Some(InsightsProgress::default());
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

    /// One page was added to the query holdings.
    pub fn on_batch(&mut self, run_id: u64, job_id: u64, batch: BatchProgress) {
        if let Some(progress) = self.progress_of(run_id, job_id) {
            progress.event_count = batch.total;
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

    /// The fetch of `run_id` ended with `job` (UC1 10). Returns whether to
    /// evaluate: Completed and CompletedWithFailures do, unless [Cancel] was
    /// pressed (R-01); a failed fetch is FetchFailed (U1:BR2.6), a cancelled
    /// one Cancelled (U1:BR2.7), an aborted one (the app closes) Idle.
    pub fn finish_fetch(&mut self, run_id: u64, job: &FetchJob, now: Instant) -> bool {
        if self.fetching(run_id).is_none() || job.status == JobStatus::Running {
            return false;
        }
        let fetched = matches!(
            job.status,
            JobStatus::Completed | JobStatus::CompletedWithFailures
        );
        let phase = match job.status {
            JobStatus::Aborted => AnalyticsPhase::Idle,
            JobStatus::Failed => AnalyticsPhase::FetchFailed,
            _ if !fetched || self.run.cancel_requested => AnalyticsPhase::Cancelled,
            _ => AnalyticsPhase::Evaluating,
        };
        if phase == AnalyticsPhase::Idle {
            self.run = InsightsRun {
                run_id: self.run.run_id,
                ..InsightsRun::default()
            };
            return false;
        }
        let run = &mut self.run;
        run.phase = phase;
        run.cache_saving = false;
        run.failed_streams = job.failed_streams.clone();
        run.listing_failure = job.listing_failure.clone();
        if phase == AnalyticsPhase::Evaluating {
            run.partial = job.status == JobStatus::CompletedWithFailures;
            run.progress = Some(InsightsProgress {
                event_count: job.event_count,
                total_count: job.event_count,
                ..run.progress.clone().unwrap_or_default()
            });
            return true;
        }
        if phase == AnalyticsPhase::FetchFailed {
            run.failure = job.failure().cloned();
        }
        run.progress = None;
        run.cancel_requested = false;
        self.settle(now);
        false
    }

    /// `evaluated` of `total` events of `run_id` were evaluated; returns
    /// whether to go on (not after [Cancel], not for another run).
    pub fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool {
        if self.run.run_id != run_id || !self.is_evaluating() || self.run.cancel_requested {
            return false;
        }
        if let Some(progress) = self.run.progress.as_mut() {
            progress.evaluated_count = u64::try_from(evaluated).unwrap_or(u64::MAX);
            progress.total_count = u64::try_from(total).unwrap_or(u64::MAX);
        }
        true
    }

    /// The evaluation of the current run finished (UC1 12). When [Cancel]
    /// was pressed meanwhile, even after the last step or with nothing held
    /// to evaluate, the result is dropped and the run is Cancelled
    /// (U2:BR10.1, U1 code review R-02).
    pub fn finish_evaluation(&mut self, result: QueryResult, now: Instant) {
        if self.run.run_id != result.run_id || !self.is_evaluating() {
            return;
        }
        if self.run.cancel_requested {
            self.stop_evaluation(result.run_id, now);
            return;
        }
        self.run.phase = AnalyticsPhase::Done;
        self.run.result = Some(result);
        self.run.progress = None;
        self.run.cancel_requested = false;
        self.settle(now);
    }

    /// The evaluation of `run_id` ended abnormally (U2:BR10.2): no result
    /// is shown (the previous one went when the fetch started, U1:BR2.9;
    /// review R-08), and the fetch is not said to have failed.
    pub fn fail_evaluation(&mut self, run_id: u64, now: Instant) {
        if self.run.run_id != run_id || !self.is_evaluating() {
            return;
        }
        self.run.phase = AnalyticsPhase::EvaluationFailed;
        self.run.result = None;
        self.run.failure = None;
        self.run.progress = None;
        self.run.cancel_requested = false;
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
        let words = |words: &[&str], kind| {
            words
                .iter()
                .map(move |word| CompletionCandidate {
                    text: (*word).to_string(),
                    kind,
                })
                .collect::<Vec<_>>()
        };
        let mut candidates = words(&COMMAND_WORDS, CompletionKind::Command);
        candidates.extend(words(&KEYWORD_WORDS, CompletionKind::Keyword));
        candidates.extend(words(&FIXED_FIELD_WORDS, CompletionKind::FixedField));
        candidates.extend(self.field_names.iter().map(|name| CompletionCandidate {
            text: name.clone(),
            kind: CompletionKind::JsonField,
        }));
        candidates
    }

    /// The evaluation of `run_id` stopped before its end (U1:BR2.7, BR5.8).
    pub fn stop_evaluation(&mut self, run_id: u64, now: Instant) {
        if self.run.run_id != run_id || !self.is_evaluating() {
            return;
        }
        self.run.phase = AnalyticsPhase::Cancelled;
        self.run.progress = None;
        self.run.cancel_requested = false;
        self.settle(now);
    }

    /// Whether [Cancel] can be pressed (R-02: as Log Management's).
    pub fn cancel_available(&self, close_pending: bool) -> bool {
        self.is_running()
            && !self.run.cancel_requested
            && !(self.is_fetching() && self.run.cache_saving)
            && !close_pending
    }

    /// [Cancel] (UC4): marks the run, which then neither evaluates nor
    /// goes on evaluating (R-01), and returns its number, whose stop
    /// handle the caller tells to stop. `None` when it cannot be pressed.
    pub fn request_cancel(&mut self, close_pending: bool) -> Option<u64> {
        if !self.cancel_available(close_pending) {
            return None;
        }
        self.run.cancel_requested = true;
        Some(self.run.run_id)
    }

    /// Ends a run whose task stopped without reporting (it panicked): a
    /// fetching run fails with `failure`, an evaluating one is an
    /// evaluation failure without it (U2:BR10.2: the fetch did not fail).
    /// Returns whether the session changed.
    pub fn abort_with_failure(&mut self, run_id: u64, failure: ApiFailure, now: Instant) -> bool {
        if self.run.run_id != run_id || !self.is_running() {
            return false;
        }
        if self.is_evaluating() {
            self.fail_evaluation(run_id, now);
            self.run.cache_saving = false;
            return true;
        }
        self.run.phase = AnalyticsPhase::FetchFailed;
        self.run.failure = Some(failure);
        self.run.progress = None;
        self.run.cache_saving = false;
        self.run.cancel_requested = false;
        self.settle(now);
        true
    }

    /// U1:BR7.2: the connection changed. The log group selection, the
    /// result, the error and the warning go and the phase is Idle; the
    /// query text and the time range stay. Never called while running
    /// (U1:BR7.3).
    pub fn on_connection_changed(&mut self) {
        self.selected_log_group = None;
        self.run = InsightsRun {
            run_id: self.run.run_id,
            ..InsightsRun::default()
        };
        self.field_names.clear();
        self.revision += 1;
    }

    /// The rows of one viewport request, from the rows `held` read for
    /// `positions_offset` (U1:BR6.3, BR6.4), with their cells for the
    /// result's columns (U2:BR7.1). Each message is read as JSON once, now,
    /// and nothing is kept (U2:BR7.4).
    pub fn display_rows(
        &self,
        run_id: u64,
        holdings_epoch: u64,
        offset: usize,
        held: Option<HeldRows>,
        zone: Tz,
    ) -> QueryRowWindow {
        let total_count = self.result().map_or(0, |result| {
            u64::try_from(result.row_count()).unwrap_or(u64::MAX)
        });
        let columns = self.result().map_or(&[][..], QueryResult::columns);
        let format_time = |time| format_display_in_zone(time, zone);
        let rows = held.map_or_else(Vec::new, |held| {
            held.events
                .into_iter()
                .map(|event| {
                    let values = row_cells(
                        columns,
                        &EventFields::new(&event, &held.log_group_name),
                        &format_time,
                    );
                    QueryRow {
                        display_time: format_time(event.timestamp),
                        log_group_name: held.log_group_name.clone(),
                        values,
                        event,
                    }
                })
                .collect()
        });
        QueryRowWindow {
            run_id,
            holdings_epoch,
            offset: u64::try_from(offset).unwrap_or(u64::MAX),
            rows,
            total_count,
        }
    }

    /// The holdings position of result row `index` of the shown result, when
    /// `run_id` and `holdings_epoch` are its own (U2:BR7.3, BR7.4).
    pub fn row_position(&self, run_id: u64, holdings_epoch: u64, index: usize) -> Option<usize> {
        self.result()?.row_at(run_id, holdings_epoch, index)
    }

    /// The fields of result row `index`, from the one event `held` read for
    /// it (U2:BR7.3); no field when it was not read.
    pub fn display_row_detail(
        &self,
        run_id: u64,
        holdings_epoch: u64,
        index: usize,
        held: Option<HeldRows>,
        zone: Tz,
    ) -> QueryRowDetail {
        let format_time = |time| format_display_in_zone(time, zone);
        let fields = held
            .and_then(|held| {
                let event = held.events.into_iter().next()?;
                Some(
                    row_detail(
                        &EventFields::new(&event, &held.log_group_name),
                        &format_time,
                    )
                    .into_iter()
                    .map(|(name, value)| DetailField { name, value })
                    .collect(),
                )
            })
            .unwrap_or_default();
        QueryRowDetail {
            run_id,
            holdings_epoch,
            index: u64::try_from(index).unwrap_or(u64::MAX),
            fields,
        }
    }

    /// The screen's view of the Log Analytics screen.
    pub fn view(&self, context: &RunContext, now: Instant) -> InsightsView {
        let blocker = self.blocker(context);
        let run = &self.run;
        InsightsView {
            selected_log_group_name: self.selected_log_group.clone(),
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
            partial: run.partial,
            failed_streams: run.failed_streams.clone(),
            listing_failure: run.listing_failure.clone(),
            result: run.result.as_ref().map(summary_of),
            elapsed: run.started_at.map(|started| ElapsedView {
                running: run.final_ms.is_none(),
                elapsed_ms: run.final_ms.unwrap_or_else(|| millis_between(started, now)),
                fetch_number: run.run_id,
            }),
            cancelling: self.is_running() && run.cancel_requested,
            cancel_available: self.cancel_available(context.close_pending),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};
    use crate::query::parser::parse_query;
    use crate::time_range::TimeRange;

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

    /// Runs the session to Done with `rows` matching positions.
    fn done(session: &mut InsightsSession, now: Instant, rows: Vec<usize>) -> u64 {
        let run_id = start(session, now).run_id;
        assert!(session.finish_fetch(run_id, &job(JobStatus::Completed, 10), now));
        session.finish_evaluation(QueryResult::new(run_id, 1, 10, rows), now);
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
        assert_eq!(first.key.log_group_name, GROUP);
        assert_eq!(first.fetch.request.region(), Some("ap-northeast-1"));
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
        // U1:BR3.9, then U1:BR2.9 when a readable query runs.
        let mut session = ready();
        let now = Instant::now();
        let first_run = done(&mut session, now, vec![0, 3]);
        let revision = session.snapshot().revision;
        let problem = parse_query("stats count(*)").unwrap_err();
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
        assert_eq!(view.result, None, "U1:BR2.9");
        assert_eq!(view.problem, None);
    }

    #[test]
    fn the_end_of_the_fetch_decides_what_follows() {
        // UC1 10: evaluate, partial, failed, cancelled, closed; R-01.
        let now = Instant::now();
        let failure = ApiFailure::new(FailureKind::AccessDenied, &SafeDetailFields::default());
        let cases = [
            (
                JobStatus::Completed,
                true,
                AnalyticsPhase::Evaluating,
                false,
            ),
            (
                JobStatus::CompletedWithFailures,
                true,
                AnalyticsPhase::Evaluating,
                true,
            ),
            (JobStatus::Failed, false, AnalyticsPhase::FetchFailed, false),
            (
                JobStatus::Cancelled,
                false,
                AnalyticsPhase::Cancelled,
                false,
            ),
            (JobStatus::Aborted, false, AnalyticsPhase::Idle, false),
        ];
        for (status, evaluates, phase, partial) in cases {
            let mut session = ready();
            let run_id = start(&mut session, now).run_id;
            let mut ended = job(status, 5);
            if status == JobStatus::Failed {
                ended.listing_failure = Some(failure.clone());
            }
            assert_eq!(
                session.finish_fetch(run_id, &ended, now),
                evaluates,
                "{status:?}"
            );
            let view = session.view(&connected(), now);
            assert_eq!(view.phase, phase, "{status:?}");
            assert_eq!(view.partial, partial, "{status:?}");
            if status == JobStatus::Failed {
                assert_eq!(view.failure, Some(failure.clone()));
            }
        }
        // R-01: [Cancel] after the last stream, the fetch still Completed.
        let mut session = ready();
        let run_id = start(&mut session, now).run_id;
        assert_eq!(session.request_cancel(false), Some(run_id));
        assert!(session.view(&connected(), now).cancelling);
        assert!(!session.finish_fetch(run_id, &job(JobStatus::Completed, 5), now));
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
        assert!(!session.finish_fetch(old, &job(JobStatus::Completed, 1), now));
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
        assert!(session.finish_fetch(current, &job(JobStatus::Completed, 3), now));
        assert!(!session.on_evaluation_progress(old, 1, 3));
        session.finish_evaluation(QueryResult::new(old, 1, 3, vec![0]), now);
        assert_eq!(
            session.phase(),
            AnalyticsPhase::Evaluating,
            "an old result is dropped"
        );
        assert!(session.on_evaluation_progress(current, 3, 3));
        session.finish_evaluation(QueryResult::new(current, 2, 3, vec![]), now);
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
        assert_eq!(session.selected_log_group(), Some(GROUP));
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
        assert_eq!(idle.selected_log_group(), None);
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
        assert!(session.finish_fetch(run_id, &job(JobStatus::Completed, 20_000), now));
        assert!(
            session.cancel_available(false),
            "the evaluation can be cancelled"
        );
        assert_eq!(session.request_cancel(false), Some(run_id));
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
        assert_eq!(view.selected_log_group_name, None);
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
        assert!(session.finish_fetch(run_id, &job(JobStatus::Completed, events), now));
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
        assert_eq!(session.request_cancel(false), Some(run_id));
        session.finish_evaluation(QueryResult::new(run_id, 1, 3, vec![0, 1]), now);
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
        assert_eq!(session.request_cancel(false), Some(run_id));
        session.finish_evaluation(QueryResult::new(run_id, 1, 0, Vec::new()), now);
        assert_eq!(session.phase(), AnalyticsPhase::Cancelled);
    }

    #[test]
    fn an_evaluation_failure_shows_no_result_and_no_fetch_failure() {
        // U2:BR10.2, review R-08: the previous result went when the fetch
        // started; a connection change goes back to Idle.
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
        use crate::query::evaluator::EvalOutput;
        use crate::query::plan::{FieldRef, JsonPath};
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 12_000);
        session.finish_evaluation(
            QueryResult::from_output(
                run_id,
                4,
                EvalOutput {
                    rows: (0..10_000).collect(),
                    columns: vec![
                        FieldRef::Timestamp,
                        FieldRef::Json(JsonPath::new(vec!["user.id".to_string()], "`user.id`")),
                    ],
                    matched_count: 12_000,
                    truncated_at: Some(10_000),
                    evaluated_count: 12_000,
                },
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
        use crate::query::evaluator::EvalOutput;
        use crate::query::plan::{FieldRef, JsonPath};
        let mut session = ready();
        let now = Instant::now();
        let run_id = evaluating(&mut session, now, 1);
        let level = FieldRef::Json(JsonPath::new(vec!["level".to_string()], "level"));
        let gone = FieldRef::Json(JsonPath::new(vec!["gone".to_string()], "gone"));
        session.finish_evaluation(
            QueryResult::from_output(
                run_id,
                4,
                EvalOutput {
                    rows: vec![0],
                    columns: vec![FieldRef::Timestamp, level, gone],
                    matched_count: 1,
                    truncated_at: None,
                    evaluated_count: 1,
                },
            ),
            now,
        );
        let held = || HeldRows {
            log_group_name: GROUP.to_string(),
            events: vec![LogEvent {
                timestamp: 0,
                ingestion_time: None,
                message: r#"{"level":"error","n":1}"#.to_string(),
                log_stream_name: "s".to_string(),
                sequence: 0,
            }],
        };
        let window = session.display_rows(run_id, 4, 0, Some(held()), UTC);
        assert_eq!(
            window.rows[0].values,
            vec![
                Some("1970/01/01 00:00:00.000".to_string()),
                Some("error".to_string()),
                None
            ]
        );
        assert_eq!(session.row_position(run_id, 4, 0), Some(0));
        assert_eq!(session.row_position(run_id, 5, 0), None, "another epoch");
        let detail = session.display_row_detail(run_id, 4, 0, Some(held()), UTC);
        let names: Vec<&str> = detail.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["@timestamp", "@log", "@logStream", "@message", "level", "n"]
        );
        assert_eq!(detail.fields[1].value.as_deref(), Some(GROUP));
        assert!(
            session
                .display_row_detail(run_id, 4, 0, None, UTC)
                .fields
                .is_empty(),
            "nothing read, nothing listed"
        );
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
        session.finish_evaluation(QueryResult::new(run_id, 1, 2, vec![]), now);
        session.on_connection_changed();
        assert!(session.field_names().is_empty());
    }

    #[test]
    fn the_default_query_is_read_with_its_four_commands() {
        // The console's default columns, sort and limit with U1's filter.
        let plan = parse_query(DEFAULT_QUERY).unwrap();
        assert_eq!(plan.commands.len(), 4);
        assert!(plan.has_fields() && plan.has_sort() && plan.has_row_limit());
    }
}
