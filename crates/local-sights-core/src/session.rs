//! AppSession (U1 minimal version): the state the screen displays.
//!
//! The screen keeps no state of its own; it shows [`SessionView`] and
//! forwards user operations here (ADR-001). Every input change re-runs the
//! shared validation (BR1.8). While a fetch runs, input changes and new
//! fetches are refused (BR1.4).

use serde::{Deserialize, Serialize};

use crate::coordinator::{FetchJob, JobStatus};
use crate::failure::ApiFailure;
use crate::request::{
    FetchInput, ValidatedFetch, ValidationError, message_keys, validate_fetch_input,
};

/// Phase of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Phase {
    /// Nothing fetched yet.
    Idle,
    /// A fetch is running.
    Fetching,
    /// The last fetch completed.
    Done,
    /// The last fetch failed.
    Failed,
}

/// Input field the user edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputField {
    /// Profile name.
    ProfileName,
    /// Log group name.
    LogGroupName,
    /// Log stream name.
    LogStreamName,
    /// Start date and time.
    StartText,
    /// End date and time.
    EndText,
}

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// A fetch is in progress (BR1.4).
    #[error("a fetch is in progress")]
    Busy,
    /// The input does not pass validation.
    #[error("the fetch conditions are not valid")]
    Invalid,
}

impl SessionError {
    /// Message-catalog key of the reason.
    pub fn message_key(self) -> &'static str {
        match self {
            SessionError::Busy => "session.busy",
            SessionError::Invalid => "session.invalid",
        }
    }
}

/// Summary of the most recent finished fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    /// Job ID.
    pub job_id: String,
    /// Completed or Failed.
    pub status: JobStatus,
    /// Number of events fetched.
    pub event_count: u64,
    /// Number of pages received.
    pub page_count: u32,
    /// Failure (kind and safe detail only), when the job failed.
    pub failure: Option<ApiFailure>,
}

/// Serializable snapshot of the session for the screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    /// Session ID.
    pub session_id: String,
    /// Current phase.
    pub phase: Phase,
    /// Input as typed.
    pub input: FetchInput,
    /// Reasons why Fetch cannot be pressed (message-catalog keys).
    pub validation_errors: Vec<String>,
    /// Whether Fetch can be pressed now.
    pub can_fetch: bool,
    /// ID of the running or last started job.
    pub current_job_id: Option<String>,
    /// Number of events of the current or last fetch.
    pub event_count: u64,
    /// Most recent finished fetch.
    pub last_job: Option<JobSummary>,
}

/// State of one app session.
#[derive(Debug, Clone)]
pub struct AppSession {
    session_id: String,
    phase: Phase,
    input: FetchInput,
    validation: Result<ValidatedFetch, Vec<ValidationError>>,
    current_job_id: Option<String>,
    event_count: u64,
    last_job: Option<JobSummary>,
}

impl Default for AppSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AppSession {
    /// A new session in phase Idle with empty input.
    pub fn new() -> Self {
        let input = FetchInput::default();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            phase: Phase::Idle,
            validation: validate_fetch_input(&input),
            input,
            current_job_id: None,
            event_count: 0,
            last_job: None,
        }
    }

    /// Session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Input as typed.
    pub fn input(&self) -> &FetchInput {
        &self.input
    }

    /// Reasons why Fetch cannot be pressed (empty when it can, apart from
    /// a running fetch).
    pub fn validation_errors(&self) -> &[ValidationError] {
        match &self.validation {
            Ok(_) => &[],
            Err(errors) => errors,
        }
    }

    /// Whether Fetch can be pressed: input valid and no fetch running.
    pub fn can_fetch(&self) -> bool {
        self.phase != Phase::Fetching && self.validation.is_ok()
    }

    /// Changes one input field and re-validates (BR1.8). Refused while a
    /// fetch is running (BR1.4).
    pub fn update_input(&mut self, field: InputField, value: String) -> Result<(), SessionError> {
        if self.phase == Phase::Fetching {
            return Err(SessionError::Busy);
        }
        let slot = match field {
            InputField::ProfileName => &mut self.input.profile_name,
            InputField::LogGroupName => &mut self.input.log_group_name,
            InputField::LogStreamName => &mut self.input.log_stream_name,
            InputField::StartText => &mut self.input.start_text,
            InputField::EndText => &mut self.input.end_text,
        };
        *slot = value;
        self.validation = validate_fetch_input(&self.input);
        Ok(())
    }

    /// Starts a fetch: returns the validated conditions and moves to
    /// Fetching. Refused while fetching or when the input is invalid.
    pub fn begin_fetch(&mut self) -> Result<ValidatedFetch, SessionError> {
        if self.phase == Phase::Fetching {
            return Err(SessionError::Busy);
        }
        let validated = self.validation.clone().map_err(|_| SessionError::Invalid)?;
        self.phase = Phase::Fetching;
        self.event_count = 0;
        self.current_job_id = None;
        Ok(validated)
    }

    /// Records the job the coordinator started.
    pub fn on_job_started(&mut self, job: &FetchJob) {
        self.current_job_id = Some(job.job_id.clone());
        self.event_count = 0;
    }

    /// Records the cumulative number of events received so far.
    pub fn on_progress(&mut self, total: u64) {
        self.event_count = total;
    }

    /// Records the finished job: Completed moves to Done, Failed to Failed.
    /// A job that is still Running leaves the session in Fetching.
    pub fn finish_fetch(&mut self, job: &FetchJob) {
        self.phase = match job.status {
            JobStatus::Completed => Phase::Done,
            JobStatus::Failed => Phase::Failed,
            JobStatus::Running => Phase::Fetching,
        };
        self.current_job_id = Some(job.job_id.clone());
        self.event_count = job.event_count;
        self.last_job = Some(JobSummary {
            job_id: job.job_id.clone(),
            status: job.status,
            event_count: job.event_count,
            page_count: job.page_count(),
            failure: job.failure().cloned(),
        });
    }

    /// Snapshot for the screen.
    pub fn view(&self) -> SessionView {
        SessionView {
            session_id: self.session_id.clone(),
            phase: self.phase,
            input: self.input.clone(),
            validation_errors: message_keys(self.validation_errors()),
            can_fetch: self.can_fetch(),
            current_job_id: self.current_job_id.clone(),
            event_count: self.event_count,
            last_job: self.last_job.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};
    use crate::fetcher::{StreamFetchOutcome, StreamStatus};

    fn fill_valid(session: &mut AppSession) {
        let values = [
            (InputField::ProfileName, "dev"),
            (InputField::LogGroupName, "/aws/lambda/orders"),
            (InputField::LogStreamName, "stream-a"),
            (InputField::StartText, "2024-01-02 03:04:05"),
            (InputField::EndText, "2024-01-02 03:05:05"),
        ];
        for (field, value) in values {
            session.update_input(field, value.to_string()).unwrap();
        }
    }

    fn finished_job(
        session: &mut AppSession,
        events: u64,
        failure: Option<ApiFailure>,
    ) -> FetchJob {
        let validated = session.validation.clone().unwrap();
        let mut job = FetchJob::start(validated.range);
        session.on_job_started(&job);
        let failed = failure.is_some();
        job.status = if failed {
            JobStatus::Failed
        } else {
            JobStatus::Completed
        };
        job.event_count = events;
        job.failed_stream_count = u32::from(failed);
        job.outcome = Some(StreamFetchOutcome {
            log_stream_name: "stream-a".to_string(),
            event_count: events,
            page_count: 2,
            status: if failed {
                StreamStatus::Failed
            } else {
                StreamStatus::Completed
            },
            failure,
        });
        job
    }

    #[test]
    fn starts_idle_with_reasons_and_cannot_fetch() {
        let session = AppSession::new();
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert!(!view.can_fetch);
        assert!(
            view.validation_errors
                .contains(&"validation.logGroupRequired".to_string())
        );
        assert!(!view.session_id.is_empty());
        assert_ne!(AppSession::new().session_id(), session.session_id());
    }

    #[test]
    fn invalid_input_cannot_start_a_fetch() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        session
            .update_input(InputField::EndText, "2024-01-02 03:04:05".to_string())
            .unwrap();
        assert_eq!(
            session.view().validation_errors,
            vec!["validation.rangeOrder"]
        );
        assert_eq!(session.begin_fetch(), Err(SessionError::Invalid));
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn valid_input_starts_fetching_and_locks_input() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        assert!(session.can_fetch());
        let validated = session.begin_fetch().unwrap();
        assert_eq!(validated.request.log_stream_name(), "stream-a");
        assert_eq!(session.phase(), Phase::Fetching);
        assert!(!session.view().can_fetch);
        assert_eq!(
            session.update_input(InputField::LogGroupName, "other".to_string()),
            Err(SessionError::Busy)
        );
        assert_eq!(session.input().log_group_name, "/aws/lambda/orders");
        assert_eq!(session.begin_fetch(), Err(SessionError::Busy));
    }

    #[test]
    fn progress_and_completion_move_to_done_with_count() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        session.begin_fetch().unwrap();
        let job = finished_job(&mut session, 42, None);
        session.on_progress(40);
        assert_eq!(session.view().event_count, 40);
        session.finish_fetch(&job);
        let view = session.view();
        assert_eq!(view.phase, Phase::Done);
        assert_eq!(view.event_count, 42);
        assert_eq!(view.current_job_id.as_deref(), Some(job.job_id.as_str()));
        let summary = view.last_job.unwrap();
        assert_eq!(summary.page_count, 2);
        assert_eq!(summary.failure, None);
    }

    #[test]
    fn zero_events_is_done_with_zero_count() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        session.begin_fetch().unwrap();
        let job = finished_job(&mut session, 0, None);
        session.finish_fetch(&job);
        assert_eq!(session.phase(), Phase::Done);
        assert_eq!(session.view().event_count, 0);
    }

    #[test]
    fn failure_moves_to_failed_with_kind_and_safe_detail() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        session.begin_fetch().unwrap();
        let failure = ApiFailure::new(
            FailureKind::AccessDenied,
            &SafeDetailFields {
                profile_name: Some("dev".to_string()),
                ..SafeDetailFields::default()
            },
        );
        let job = finished_job(&mut session, 3, Some(failure));
        session.finish_fetch(&job);
        let view = session.view();
        assert_eq!(view.phase, Phase::Failed);
        assert_eq!(view.event_count, 3);
        let failure = view.last_job.and_then(|job| job.failure).unwrap();
        assert_eq!(failure.kind(), FailureKind::AccessDenied);
        assert_eq!(failure.safe_detail(), "kind=AccessDenied; profile=dev");
    }

    #[test]
    fn done_and_failed_sessions_can_fetch_again() {
        let mut session = AppSession::new();
        fill_valid(&mut session);
        session.begin_fetch().unwrap();
        let job = finished_job(&mut session, 1, None);
        session.finish_fetch(&job);
        session
            .update_input(InputField::LogStreamName, "stream-b".to_string())
            .unwrap();
        assert!(session.begin_fetch().is_ok());
        assert_eq!(session.view().event_count, 0);
        let failure = ApiFailure::new(FailureKind::Network, &SafeDetailFields::default());
        let job = finished_job(&mut session, 0, Some(failure));
        session.finish_fetch(&job);
        assert_eq!(session.phase(), Phase::Failed);
        assert!(session.begin_fetch().is_ok());
    }

    #[test]
    fn view_serializes_with_camel_case_names_for_the_screen() {
        let session = AppSession::new();
        let json = serde_json::to_value(session.view()).unwrap();
        assert_eq!(json["phase"], "Idle");
        assert_eq!(json["canFetch"], false);
        assert!(json["validationErrors"].is_array());
        assert_eq!(json["input"]["logGroupName"], "");
        assert!(json["lastJob"].is_null());
        let field: InputField = serde_json::from_str("\"logStreamName\"").unwrap();
        assert_eq!(field, InputField::LogStreamName);
        assert_eq!(SessionError::Busy.message_key(), "session.busy");
    }
}
