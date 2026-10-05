//! AppSession: the state the screen displays.
//!
//! The screen keeps no state of its own; it shows [`SessionView`] and
//! forwards user operations here (ADR-001). Every change re-runs the
//! validation: the screen-only selection check (profile, region, log group)
//! and the shared validation (U2:BR2.7, U1:BR1.8). While a fetch runs, or a
//! connection change awaits confirmation, conflicting operations are refused
//! (U1:BR1.4, U2:BR2.6). Since U2 the session also holds the connection
//! selection, the log group list, its filter and the selected log group.
//! Since U3 it also holds the progress of a fetch, the failed streams and
//! whether their list is open, and the generation of the connection, which
//! drops operations made for an older connection (U3:BR6.2, BR6.3, BR6.7).

use serde::{Deserialize, Serialize};

use crate::catalog::{ConnectionCatalog, ConnectionProfile, ProfileKind, ProfileSelector};
use crate::connection::{
    ChangeOutcome, ConnectionSelection, ConnectionState, ListingCommand, PendingConnectionChange,
};
use crate::coordinator::{BatchProgress, FailedStream, FetchJob, JobStatus};
use crate::failure::ApiFailure;
use crate::log_groups::listing::ListingRequest;
use crate::log_groups::{
    EmptyState, ListingStatus, LogGroup, LogGroupListing, apply_failure_if_current,
    apply_page_if_current, empty_state, filter_groups,
};
use crate::paging::PageDecision;
use crate::request::{
    FetchInput, ValidatedFetch, ValidationError, message_keys, validate_fetch_input,
};
use crate::streams::StreamListingStatus;
use crate::streams::planner::ListingProgress;

/// Phase of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Phase {
    /// Nothing fetched yet, or the connection was changed (U2:BR2.4).
    Idle,
    /// A fetch is running.
    Fetching,
    /// The last fetch completed (Completed or CompletedWithFailures).
    Done,
    /// The last fetch failed.
    Failed,
}

/// Input field the user types (the profile and log group are selected
/// from lists since U2; there is no stream name since U3:BR6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputField {
    /// Start date and time.
    StartText,
    /// End date and time.
    EndText,
}

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// A fetch is in progress (U1:BR1.4, U2:BR2.6).
    #[error("a fetch is in progress")]
    Busy,
    /// The input does not pass validation.
    #[error("the fetch conditions are not valid")]
    Invalid,
    /// A connection change awaits confirmation (U2:BR2.5).
    #[error("a connection change awaits confirmation")]
    ConfirmationPending,
    /// Profile and region are not both selected (U2:BR3.5).
    #[error("no connection is selected")]
    NotConnected,
    /// The profile is not in the catalog.
    #[error("unknown profile")]
    UnknownProfile,
    /// The region is not one of the options.
    #[error("unknown region")]
    UnknownRegion,
    /// The log group is not in the current list.
    #[error("unknown log group")]
    UnknownLogGroup,
    /// No connection change awaits confirmation.
    #[error("no connection change is pending")]
    NothingPending,
    /// The operation was made for an older connection (BR6.7).
    #[error("the operation belongs to an older connection")]
    StaleGeneration,
    /// There is no failed stream or listing failure to show (BR6.3).
    #[error("there are no failures to show")]
    NoFailures,
}

impl SessionError {
    /// Message-catalog key of the reason.
    pub fn message_key(self) -> &'static str {
        match self {
            SessionError::Busy => "session.busy",
            SessionError::Invalid => "session.invalid",
            SessionError::ConfirmationPending => "session.confirmationPending",
            SessionError::NotConnected => "session.notConnected",
            SessionError::UnknownProfile => "session.unknownProfile",
            SessionError::UnknownRegion => "session.unknownRegion",
            SessionError::UnknownLogGroup => "session.unknownLogGroup",
            SessionError::NothingPending => "session.nothingPending",
            SessionError::StaleGeneration => "session.staleGeneration",
            SessionError::NoFailures => "session.noFailures",
        }
    }
}

/// Screen-only selection reasons (U2:BR2.7 (1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    /// No profile selected.
    ProfileRequired,
    /// No region selected.
    RegionRequired,
    /// No log group selected.
    LogGroupRequired,
}

impl SelectionError {
    /// Message-catalog key of the reason.
    pub fn message_key(self) -> &'static str {
        match self {
            SelectionError::ProfileRequired => "selection.profileRequired",
            SelectionError::RegionRequired => "selection.regionRequired",
            SelectionError::LogGroupRequired => "selection.logGroupRequired",
        }
    }
}

/// What the caller must do after a connection operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionEffect {
    /// Start this listing (a new connection or a reload).
    pub listing: Option<ListingRequest>,
    /// The shown logs were discarded: clear the EventTimeline (U2:BR2.4).
    pub logs_cleared: bool,
    /// The change waits for the user's confirmation (U2:BR2.5).
    pub awaiting_confirmation: bool,
}

/// Summary of the most recent finished fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    /// Job ID.
    pub job_id: u64,
    /// Completed, CompletedWithFailures or Failed.
    pub status: JobStatus,
    /// Number of events fetched.
    pub event_count: u64,
    /// Number of streams selected for the fetch.
    pub planned_stream_count: u32,
    /// Number of streams finished.
    pub finished_stream_count: u32,
    /// Number of failed streams.
    pub failed_stream_count: u32,
    /// Failure (kind and safe detail only), when the whole job failed.
    pub failure: Option<ApiFailure>,
}

/// Progress of the running fetch, for the status line (BR6.2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchProgress {
    /// Streams seen by the listing so far.
    pub seen_stream_count: u32,
    /// Streams selected by the listing so far.
    pub selected_stream_count: u32,
    /// Streams to fetch; `None` while the streams are being listed.
    pub planned_stream_count: Option<u32>,
    /// Streams finished, successfully or not.
    pub finished_stream_count: u32,
    /// Events added so far.
    pub event_count: u64,
}

/// The log group list as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGroupListView {
    /// Loading, Complete or Partial.
    pub status: ListingStatus,
    /// Names that pass the filter, ascending (U2:BR3.3, BR3.7).
    pub visible_groups: Vec<String>,
    /// Number of groups listed so far, before filtering.
    pub total_count: usize,
    /// Which empty-list message applies (U2:BR3.10).
    pub empty_state: Option<EmptyState>,
    /// The failure, when Partial (U2:BR3.4).
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
    /// Typed input (the times; profile and log group are empty).
    pub input: FetchInput,
    /// Reasons why Fetch cannot be pressed (message-catalog keys).
    pub validation_errors: Vec<String>,
    /// Whether Fetch can be pressed now.
    pub can_fetch: bool,
    /// ID of the running or last started job.
    pub current_job_id: Option<u64>,
    /// Number of events of the current or last fetch.
    pub event_count: u64,
    /// Most recent finished fetch.
    pub last_job: Option<JobSummary>,
    /// Profile rows, SdkDefault first (U2:BR1.1).
    pub profiles: Vec<ConnectionProfile>,
    /// Region codes (U2:BR1.4).
    pub regions: Vec<String>,
    /// Message keys of the unreadable-file notices (U2:BR1.3).
    pub catalog_notices: Vec<String>,
    /// The chosen connection.
    pub connection: ConnectionSelection,
    /// The change awaiting confirmation, if any (U2:BR2.5).
    pub pending_change: Option<PendingConnectionChange>,
    /// The log group list, when the connection is complete.
    pub log_groups: Option<LogGroupListView>,
    /// The filter text as typed (U2:BR3.7).
    pub log_group_filter: String,
    /// The selected log group (U2:BR3.8).
    pub selected_log_group_name: Option<String>,
    /// Whether profile, region and log group may be changed now (U2:BR2.6).
    pub can_change_connection: bool,
    /// Whether the list may be reloaded now (U2:BR3.5).
    pub can_reload: bool,
    /// Version of the EventTimeline: changes with every added page and
    /// every discard, so the screen re-reads its rows (U3:BR4.5, BR6.5).
    pub timeline_version: u64,
    /// Generation of the connection; the screen sends it back with the
    /// operations that depend on the connection (BR6.7).
    pub connection_generation: u64,
    /// Progress of the running fetch (BR6.2).
    pub progress: Option<FetchProgress>,
    /// Streams that failed in the last fetch (BR5.3).
    pub failed_streams: Vec<FailedStream>,
    /// How the last stream listing ended.
    pub listing_status: Option<StreamListingStatus>,
    /// The listing failure of the last fetch, when Partial (BR1.5).
    pub listing_failure: Option<ApiFailure>,
    /// Whether the list of failures is open (BR6.3).
    pub failure_list_open: bool,
}

/// State of one app session.
#[derive(Debug, Clone)]
pub struct AppSession {
    session_id: String,
    phase: Phase,
    input: FetchInput,
    validation: Result<ValidatedFetch, Vec<ValidationError>>,
    current_job_id: Option<u64>,
    event_count: u64,
    last_job: Option<JobSummary>,
    catalog: ConnectionCatalog,
    connection: ConnectionState,
    listing: Option<LogGroupListing>,
    log_group_filter: String,
    selected_log_group: Option<String>,
    timeline_version: u64,
    connection_generation: u64,
    progress: Option<FetchProgress>,
    failed_streams: Vec<FailedStream>,
    listing_status: Option<StreamListingStatus>,
    listing_failure: Option<ApiFailure>,
    failure_list_open: bool,
}

impl Default for AppSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AppSession {
    /// A session whose catalog holds only the SDK default profile.
    pub fn new() -> Self {
        let sdk_default = ConnectionProfile {
            kind: ProfileKind::SdkDefault,
            profile_name: None,
            default_region: None,
        };
        Self::with_catalog(ConnectionCatalog::new(vec![sdk_default], Vec::new()))
    }

    /// A new session in phase Idle with nothing selected (U2:BR2.1).
    pub fn with_catalog(catalog: ConnectionCatalog) -> Self {
        let mut session = Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            phase: Phase::Idle,
            input: FetchInput::default(),
            validation: Err(Vec::new()),
            current_job_id: None,
            event_count: 0,
            last_job: None,
            catalog,
            connection: ConnectionState::new(),
            listing: None,
            log_group_filter: String::new(),
            selected_log_group: None,
            timeline_version: 0,
            connection_generation: 0,
            progress: None,
            failed_streams: Vec::new(),
            listing_status: None,
            listing_failure: None,
            failure_list_open: false,
        };
        session.revalidate();
        session
    }

    /// Generation of the connection: incremented whenever a connection
    /// change is applied (BR6.7).
    pub fn connection_generation(&self) -> u64 {
        self.connection_generation
    }

    /// Session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Typed input.
    pub fn input(&self) -> &FetchInput {
        &self.input
    }

    /// Reasons from the shared validation (U1:BR1.8).
    pub fn validation_errors(&self) -> &[ValidationError] {
        match &self.validation {
            Ok(_) => &[],
            Err(errors) => errors,
        }
    }

    /// Reasons from the screen-only selection check (U2:BR2.7 (1)).
    pub fn selection_errors(&self) -> Vec<SelectionError> {
        let selection = self.connection.selection();
        let mut errors = Vec::new();
        if selection.profile.is_none() {
            errors.push(SelectionError::ProfileRequired);
        }
        if selection.region.is_none() {
            errors.push(SelectionError::RegionRequired);
        }
        if self.selected_log_group.is_none() {
            errors.push(SelectionError::LogGroupRequired);
        }
        errors
    }

    /// Whether Fetch can be pressed: everything selected and valid, no
    /// fetch running and no change awaiting confirmation.
    pub fn can_fetch(&self) -> bool {
        self.can_change_connection()
            && self.selection_errors().is_empty()
            && self.validation.is_ok()
    }

    /// Whether profile, region and log group may be changed (U2:BR2.6).
    pub fn can_change_connection(&self) -> bool {
        self.phase != Phase::Fetching && self.connection.pending().is_none()
    }

    /// The chosen connection.
    pub fn connection(&self) -> &ConnectionSelection {
        self.connection.selection()
    }

    /// The selected log group.
    pub fn selected_log_group(&self) -> Option<&str> {
        self.selected_log_group.as_deref()
    }

    /// The current log group listing.
    pub fn listing(&self) -> Option<&LogGroupListing> {
        self.listing.as_ref()
    }

    /// Changes one typed field and re-validates. Refused while a fetch is
    /// running (U1:BR1.4) and while a connection change awaits confirmation
    /// (U2:BR2.6).
    pub fn update_input(&mut self, field: InputField, value: String) -> Result<(), SessionError> {
        self.ensure_can_change()?;
        let slot = match field {
            InputField::StartText => &mut self.input.start_text,
            InputField::EndText => &mut self.input.end_text,
        };
        *slot = value;
        self.revalidate();
        Ok(())
    }

    /// Chooses a profile; its default region becomes the region (U2:BR2.2).
    /// With logs shown, the change waits for confirmation (U2:BR2.5).
    pub fn select_profile(
        &mut self,
        profile: ProfileSelector,
    ) -> Result<ConnectionEffect, SessionError> {
        self.ensure_can_change()?;
        let default_region = self
            .catalog
            .find(&profile)
            .ok_or(SessionError::UnknownProfile)?
            .default_region
            .clone();
        let has_logs = self.has_logs();
        let outcome = self
            .connection
            .request_profile(profile, default_region, has_logs);
        Ok(self.handle_outcome(outcome))
    }

    /// Chooses a region (U2:BR2.3, BR2.5).
    pub fn select_region(&mut self, region: String) -> Result<ConnectionEffect, SessionError> {
        self.ensure_can_change()?;
        if !self.catalog.has_region(&region) {
            return Err(SessionError::UnknownRegion);
        }
        let has_logs = self.has_logs();
        let outcome = self.connection.request_region(region, has_logs);
        Ok(self.handle_outcome(outcome))
    }

    /// Applies the change awaiting confirmation (U2:BR2.5, BR2.4).
    pub fn confirm_connection_change(&mut self) -> Result<ConnectionEffect, SessionError> {
        let catalog = &self.catalog;
        let command = self
            .connection
            .confirm(|profile| catalog.find(profile).and_then(|p| p.default_region.clone()))
            .ok_or(SessionError::NothingPending)?;
        Ok(self.apply_connection_change(command))
    }

    /// Drops the change awaiting confirmation; nothing else changes.
    pub fn cancel_connection_change(&mut self) -> Result<(), SessionError> {
        if self.connection.cancel() {
            Ok(())
        } else {
            Err(SessionError::NothingPending)
        }
    }

    /// Lists the log groups of the current connection again, keeping the
    /// selected log group and the shown logs (U2:BR3.5).
    pub fn reload_log_groups(&mut self, generation: u64) -> Result<ListingRequest, SessionError> {
        self.ensure_current_generation(generation)?;
        self.ensure_can_change()?;
        match self.connection.reload() {
            Some(command) => self
                .start_listing(command)
                .ok_or(SessionError::NotConnected),
            None => Err(SessionError::NotConnected),
        }
    }

    /// Changes the filter text; kept across connection changes (U2:BR3.7).
    /// Refused while a connection change awaits confirmation (U2:BR2.6);
    /// allowed during a fetch, since filtering calls no API.
    pub fn update_log_group_filter(&mut self, text: String) -> Result<(), SessionError> {
        if self.connection.pending().is_some() {
            return Err(SessionError::ConfirmationPending);
        }
        self.log_group_filter = text;
        Ok(())
    }

    /// Selects one listed log group; the shown logs stay (U2:BR3.8).
    pub fn select_log_group(&mut self, name: String, generation: u64) -> Result<(), SessionError> {
        self.ensure_current_generation(generation)?;
        self.ensure_can_change()?;
        let listed = self.listing.as_ref().is_some_and(|l| l.contains(&name));
        if !listed {
            return Err(SessionError::UnknownLogGroup);
        }
        self.selected_log_group = Some(name);
        self.revalidate();
        Ok(())
    }

    /// Whether `listing_id` is the current listing (U2:BR3.6).
    pub fn is_current_listing(&self, listing_id: &str) -> bool {
        self.listing
            .as_ref()
            .is_some_and(|listing| listing.listing_id() == listing_id)
    }

    /// Applies a listing page if it is current (U2:BR3.6); `None` when
    /// dropped.
    pub fn apply_listing_page(
        &mut self,
        listing_id: &str,
        groups: Vec<LogGroup>,
        next_token: Option<&str>,
    ) -> Option<PageDecision> {
        apply_page_if_current(self.listing.as_mut(), listing_id, groups, next_token)
    }

    /// Applies a listing failure if it is current; returns whether applied.
    pub fn apply_listing_failure(&mut self, listing_id: &str, failure: ApiFailure) -> bool {
        apply_failure_if_current(self.listing.as_mut(), listing_id, failure)
    }

    /// Starts a fetch: returns the validated conditions with the selected
    /// profile and region (U2:BR2.8) and moves to Fetching. Refused while
    /// fetching, while a change awaits confirmation, or when invalid.
    pub fn begin_fetch(&mut self, generation: u64) -> Result<ValidatedFetch, SessionError> {
        self.ensure_current_generation(generation)?;
        if self.phase == Phase::Fetching {
            return Err(SessionError::Busy);
        }
        if self.connection.pending().is_some() {
            return Err(SessionError::ConfirmationPending);
        }
        let selection = self.connection.selection().clone();
        let (Some(profile), Some(region), Some(_), Ok(validated)) = (
            selection.profile,
            selection.region,
            self.selected_log_group.as_ref(),
            self.validation.clone(),
        ) else {
            return Err(SessionError::Invalid);
        };
        self.phase = Phase::Fetching;
        // BR5.6: the previous rows, count, failures and result go away.
        self.current_job_id = None;
        self.event_count = 0;
        self.last_job = None;
        self.failed_streams.clear();
        self.listing_status = None;
        self.listing_failure = None;
        self.failure_list_open = false;
        self.progress = Some(FetchProgress::default());
        Ok(ValidatedFetch {
            request: validated.request.with_connection(profile, region),
            range: validated.range,
        })
    }

    /// Records the job the coordinator started and the version of the
    /// timeline it has just discarded (BR5.6, BR4.5).
    pub fn on_job_started(&mut self, job: &FetchJob, timeline_version: u64) {
        if self.phase != Phase::Fetching {
            return;
        }
        self.set_timeline_version(timeline_version);
        self.current_job_id = Some(job.job_id);
        self.event_count = 0;
        self.progress = Some(FetchProgress::default());
    }

    /// Records the listing progress of the running job (BR6.2).
    pub fn on_listing_progress(&mut self, job_id: u64, listing: ListingProgress) {
        if let Some(progress) = self.progress_of(job_id) {
            progress.seen_stream_count = listing.seen_stream_count;
            progress.selected_stream_count = listing.selected_stream_count;
        }
    }

    /// Records the planned stream count of the running job.
    pub fn on_planned(&mut self, job: &FetchJob) {
        if let Some(progress) = self.progress_of(job.job_id) {
            progress.planned_stream_count = Some(job.planned_stream_count);
            progress.selected_stream_count = job.planned_stream_count;
        }
    }

    /// Records one added page of the running job: the cumulative count and
    /// the timeline version.
    pub fn on_batch(&mut self, job_id: u64, batch: BatchProgress) {
        if let Some(progress) = self.progress_of(job_id) {
            progress.event_count = batch.total;
            self.event_count = batch.total;
            self.timeline_version = self.timeline_version.max(batch.timeline_version);
        }
    }

    /// Records one finished stream of the running job.
    pub fn on_stream_finished(&mut self, job: &FetchJob) {
        if let Some(progress) = self.progress_of(job.job_id) {
            progress.finished_stream_count = job.finished_stream_count;
        }
    }

    /// Records the finished job: Completed and CompletedWithFailures move to
    /// Done, Failed to Failed. Aborted (only when the window closes) returns
    /// to Idle with nothing shown. A job that is still Running, or not the
    /// current one, leaves the session as it is.
    pub fn finish_fetch(&mut self, job: &FetchJob) {
        let current = self.current_job_id.is_none_or(|id| id == job.job_id);
        if self.phase != Phase::Fetching || !current || job.status == JobStatus::Running {
            return;
        }
        self.current_job_id = Some(job.job_id);
        self.progress = None;
        if job.status == JobStatus::Aborted {
            self.phase = Phase::Idle;
            self.event_count = 0;
            return;
        }
        self.phase = if job.status == JobStatus::Failed {
            Phase::Failed
        } else {
            Phase::Done
        };
        self.event_count = job.event_count;
        self.failed_streams = job.failed_streams.clone();
        self.listing_status = job.listing_status;
        self.listing_failure = job.listing_failure.clone();
        self.last_job = Some(JobSummary {
            job_id: job.job_id,
            status: job.status,
            event_count: job.event_count,
            planned_stream_count: job.planned_stream_count,
            finished_stream_count: job.finished_stream_count,
            failed_stream_count: u32::try_from(job.failed_streams.len()).unwrap_or(u32::MAX),
            failure: job.failure().cloned(),
        });
    }

    /// Records the version of the timeline after the caller discarded it
    /// for a connection change (U3:BR4.5, U2:BR2.4).
    pub fn set_timeline_version(&mut self, version: u64) {
        self.timeline_version = self.timeline_version.max(version);
    }

    /// Opens or closes the list of failures (BR6.3). Opening needs a failed
    /// stream or a listing failure; closing always works.
    pub fn set_failure_list_open(&mut self, open: bool) -> Result<(), SessionError> {
        if open && self.failed_streams.is_empty() && self.listing_failure.is_none() {
            return Err(SessionError::NoFailures);
        }
        self.failure_list_open = open;
        Ok(())
    }

    /// Ends a fetch that stopped abnormally (for example, its task panicked
    /// before reporting a result) so the session never stays in Fetching:
    /// moves to Failed with `failure` and re-enables the inputs. Returns
    /// whether the session changed; outside Fetching it does nothing.
    pub fn abort_fetch_with_failure(&mut self, failure: ApiFailure) -> bool {
        if self.phase != Phase::Fetching {
            return false;
        }
        self.phase = Phase::Failed;
        let progress = self.progress.take().unwrap_or_default();
        self.last_job = Some(JobSummary {
            job_id: self.current_job_id.unwrap_or_default(),
            status: JobStatus::Failed,
            event_count: self.event_count,
            planned_stream_count: progress.planned_stream_count.unwrap_or_default(),
            finished_stream_count: progress.finished_stream_count,
            failed_stream_count: 0,
            failure: Some(failure),
        });
        true
    }

    fn progress_of(&mut self, job_id: u64) -> Option<&mut FetchProgress> {
        if self.phase != Phase::Fetching || self.current_job_id != Some(job_id) {
            return None;
        }
        self.progress.as_mut()
    }

    /// Snapshot for the screen.
    pub fn view(&self) -> SessionView {
        let selection_errors = self.selection_errors();
        let mut validation_errors: Vec<String> = selection_errors
            .iter()
            .map(|e| e.message_key().to_string())
            .collect();
        // The shared check also reports a missing log group; the selection
        // reason already says so, so it is not repeated.
        let log_group_unselected = selection_errors.contains(&SelectionError::LogGroupRequired);
        validation_errors.extend(message_keys(
            &self
                .validation_errors()
                .iter()
                .copied()
                .filter(|e| !(log_group_unselected && *e == ValidationError::LogGroupRequired))
                .collect::<Vec<_>>(),
        ));
        SessionView {
            session_id: self.session_id.clone(),
            phase: self.phase,
            input: self.input.clone(),
            validation_errors,
            can_fetch: self.can_fetch(),
            current_job_id: self.current_job_id,
            event_count: self.event_count,
            last_job: self.last_job.clone(),
            profiles: self.catalog.profiles().to_vec(),
            regions: self.catalog.regions().to_vec(),
            catalog_notices: self
                .catalog
                .unreadable_files()
                .iter()
                .map(|kind| kind.unreadable_message_key().to_string())
                .collect(),
            connection: self.connection.selection().clone(),
            pending_change: self.connection.pending().cloned(),
            log_groups: self.listing.as_ref().map(|listing| self.list_view(listing)),
            log_group_filter: self.log_group_filter.clone(),
            selected_log_group_name: self.selected_log_group.clone(),
            can_change_connection: self.can_change_connection(),
            can_reload: self.can_change_connection() && self.connection.selection().is_complete(),
            timeline_version: self.timeline_version,
            connection_generation: self.connection_generation,
            progress: self.progress.clone(),
            failed_streams: self.failed_streams.clone(),
            listing_status: self.listing_status,
            listing_failure: self.listing_failure.clone(),
            failure_list_open: self.failure_list_open,
        }
    }

    fn list_view(&self, listing: &LogGroupListing) -> LogGroupListView {
        LogGroupListView {
            status: listing.status(),
            visible_groups: filter_groups(listing.groups(), &self.log_group_filter)
                .into_iter()
                .map(|group| group.log_group_name.clone())
                .collect(),
            total_count: listing.groups().len(),
            empty_state: empty_state(listing, &self.log_group_filter),
            failure: listing.failure().cloned(),
        }
    }

    /// BR6.7: an operation made for an older connection is dropped.
    fn ensure_current_generation(&self, generation: u64) -> Result<(), SessionError> {
        if generation == self.connection_generation {
            Ok(())
        } else {
            Err(SessionError::StaleGeneration)
        }
    }

    fn ensure_can_change(&self) -> Result<(), SessionError> {
        if self.phase == Phase::Fetching {
            Err(SessionError::Busy)
        } else if self.connection.pending().is_some() {
            Err(SessionError::ConfirmationPending)
        } else {
            Ok(())
        }
    }

    /// Logs are shown when the current or last fetch received events.
    fn has_logs(&self) -> bool {
        self.event_count > 0
    }

    fn handle_outcome(&mut self, outcome: ChangeOutcome) -> ConnectionEffect {
        match outcome {
            ChangeOutcome::Unchanged | ChangeOutcome::Blocked => ConnectionEffect::default(),
            ChangeOutcome::AwaitingConfirmation => ConnectionEffect {
                awaiting_confirmation: true,
                ..ConnectionEffect::default()
            },
            ChangeOutcome::Applied(command) => self.apply_connection_change(command),
        }
    }

    /// U2:BR2.4: deselect the log group, discard the shown logs, count and
    /// last result, return to Idle, then list the new connection (BR2.3).
    fn apply_connection_change(&mut self, command: ListingCommand) -> ConnectionEffect {
        self.connection_generation += 1;
        self.selected_log_group = None;
        self.event_count = 0;
        self.last_job = None;
        self.current_job_id = None;
        self.failed_streams.clear();
        self.listing_status = None;
        self.listing_failure = None;
        self.failure_list_open = false;
        if matches!(self.phase, Phase::Done | Phase::Failed) {
            self.phase = Phase::Idle;
        }
        self.revalidate();
        ConnectionEffect {
            listing: self.start_listing(command),
            logs_cleared: true,
            awaiting_confirmation: false,
        }
    }

    /// Replaces the current listing; the previous one is no longer current.
    fn start_listing(&mut self, command: ListingCommand) -> Option<ListingRequest> {
        let selection = self.connection.selection().clone();
        match (command, selection.profile, selection.region) {
            (ListingCommand::Start { listing_id }, Some(profile), Some(region)) => {
                self.listing = Some(LogGroupListing::start(
                    listing_id.clone(),
                    profile.clone(),
                    region.clone(),
                ));
                Some(ListingRequest {
                    listing_id,
                    profile,
                    region,
                })
            }
            _ => {
                self.listing = None;
                None
            }
        }
    }

    /// Runs the shared validation with the selected log group (U1:BR1.8).
    fn revalidate(&mut self) {
        let input = FetchInput {
            profile_name: String::new(),
            log_group_name: self.selected_log_group.clone().unwrap_or_default(),
            ..self.input.clone()
        };
        self.validation = validate_fetch_input(&input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};
    use crate::streams::StreamListingStatus;
    use crate::streams::planner::ListingProgress;

    fn dev() -> ProfileSelector {
        ProfileSelector::Named("dev".to_string())
    }

    fn catalog() -> ConnectionCatalog {
        let profile = |kind, name: Option<&str>, region: Option<&str>| ConnectionProfile {
            kind,
            profile_name: name.map(str::to_string),
            default_region: region.map(str::to_string),
        };
        ConnectionCatalog::new(
            vec![
                profile(ProfileKind::SdkDefault, None, None),
                profile(ProfileKind::Named, Some("dev"), Some("ap-northeast-1")),
                profile(ProfileKind::Named, Some("prod"), Some("us-east-1")),
            ],
            Vec::new(),
        )
    }

    fn group(name: &str) -> LogGroup {
        LogGroup {
            log_group_name: name.to_string(),
            arn: None,
            creation_time: None,
        }
    }

    /// Selects the `dev` connection and lists two log groups.
    fn connected_session() -> (AppSession, ListingRequest) {
        let mut session = AppSession::with_catalog(catalog());
        let listing = session.select_profile(dev()).unwrap().listing.unwrap();
        session.apply_listing_page(
            &listing.listing_id,
            vec![group("/aws/lambda/orders"), group("/aws/lambda/MyFunction")],
            None,
        );
        (session, listing)
    }

    /// U1's "valid input" now also needs the connection and a log group.
    fn fill_valid(session: &mut AppSession) {
        if session.connection().profile.is_none() {
            let listing = session.select_profile(dev()).unwrap().listing.unwrap();
            session.apply_listing_page(
                &listing.listing_id,
                vec![group("/aws/lambda/orders"), group("/aws/lambda/MyFunction")],
                None,
            );
        }
        session
            .select_log_group(
                "/aws/lambda/orders".to_string(),
                session.connection_generation(),
            )
            .unwrap();
        let values = [
            (InputField::StartText, "2024-01-02 03:04:05"),
            (InputField::EndText, "2024-01-02 03:05:05"),
        ];
        for (field, value) in values {
            session.update_input(field, value.to_string()).unwrap();
        }
    }

    /// A finished one-stream job; `failure` makes the whole job fail (as a
    /// listing failure on the first page would).
    fn finished_job(
        session: &mut AppSession,
        events: u64,
        failure: Option<ApiFailure>,
    ) -> FetchJob {
        let validated = session.validation.clone().unwrap();
        let mut job = FetchJob::start(validated.range);
        session.on_job_started(&job, 1);
        job.status = if failure.is_some() {
            JobStatus::Failed
        } else {
            JobStatus::Completed
        };
        job.planned_stream_count = 1;
        job.finished_stream_count = 1;
        job.listing_status = Some(if failure.is_some() {
            StreamListingStatus::Partial
        } else {
            StreamListingStatus::Complete
        });
        job.listing_failure = failure;
        job.event_count = events;
        job
    }

    fn batch(total: u64, timeline_version: u64) -> BatchProgress {
        BatchProgress {
            added: total,
            total,
            timeline_version,
        }
    }

    #[test]
    fn starts_idle_with_reasons_and_cannot_fetch() {
        let session = AppSession::new();
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert!(!view.can_fetch);
        assert!(
            view.validation_errors
                .contains(&"selection.logGroupRequired".to_string())
        );
        assert!(!view.session_id.is_empty());
        assert_ne!(AppSession::new().session_id(), session.session_id());
    }

    #[test]
    fn invalid_input_cannot_start_a_fetch() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .update_input(InputField::EndText, "2024-01-02 03:04:05".to_string())
            .unwrap();
        assert_eq!(
            session.view().validation_errors,
            vec!["validation.rangeOrder"]
        );
        assert_eq!(
            session.begin_fetch(session.connection_generation()),
            Err(SessionError::Invalid)
        );
        assert_eq!(session.phase(), Phase::Idle);
    }

    #[test]
    fn valid_input_starts_fetching_and_locks_input() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        assert!(session.can_fetch());
        let validated = session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(validated.request.log_group_name(), "/aws/lambda/orders");
        assert_eq!(session.phase(), Phase::Fetching);
        assert!(!session.view().can_fetch);
        assert_eq!(
            session.update_input(InputField::StartText, "2024-01-01 00:00:00".to_string()),
            Err(SessionError::Busy)
        );
        assert_eq!(session.input().start_text, "2024-01-02 03:04:05");
        assert_eq!(
            session.begin_fetch(session.connection_generation()),
            Err(SessionError::Busy)
        );
    }

    #[test]
    fn progress_and_completion_move_to_done_with_count() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, 42, None);
        session.on_batch(job.job_id, batch(40, 1));
        assert_eq!(session.view().event_count, 40);
        session.finish_fetch(&job);
        let view = session.view();
        assert_eq!(view.phase, Phase::Done);
        assert_eq!(view.event_count, 42);
        assert_eq!(view.current_job_id, Some(job.job_id));
        let summary = view.last_job.unwrap();
        assert_eq!(summary.planned_stream_count, 1);
        assert_eq!(summary.failure, None);
    }

    #[test]
    fn zero_events_is_done_with_zero_count() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, 0, None);
        session.finish_fetch(&job);
        assert_eq!(session.phase(), Phase::Done);
        assert_eq!(session.view().event_count, 0);
    }

    #[test]
    fn failure_moves_to_failed_with_kind_and_safe_detail() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
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
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, 1, None);
        session.finish_fetch(&job);
        session
            .update_input(InputField::StartText, "2024-01-02 03:04:00".to_string())
            .unwrap();
        assert!(session.begin_fetch(session.connection_generation()).is_ok());
        assert_eq!(session.view().event_count, 0);
        let failure = ApiFailure::new(FailureKind::Network, &SafeDetailFields::default());
        let job = finished_job(&mut session, 0, Some(failure));
        session.finish_fetch(&job);
        assert_eq!(session.phase(), Phase::Failed);
        assert!(session.begin_fetch(session.connection_generation()).is_ok());
    }

    fn other_failure() -> ApiFailure {
        ApiFailure::new(
            FailureKind::Other,
            &SafeDetailFields {
                api_name: Some("GetLogEvents".to_string()),
                ..SafeDetailFields::default()
            },
        )
    }

    #[test]
    fn aborting_a_running_fetch_fails_it_and_reenables_input() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = FetchJob::start(session.validation.clone().unwrap().range);
        session.on_job_started(&job, 1);
        session.on_batch(job.job_id, batch(7, 1));

        assert!(session.abort_fetch_with_failure(other_failure()));

        let view = session.view();
        assert_eq!(view.phase, Phase::Failed);
        assert!(view.can_fetch);
        assert_eq!(view.event_count, 7);
        let summary = view.last_job.unwrap();
        assert_eq!(summary.job_id, job.job_id);
        assert_eq!(summary.status, JobStatus::Failed);
        let failure = summary.failure.unwrap();
        assert_eq!(failure.kind(), FailureKind::Other);
        assert_eq!(failure.safe_detail(), "kind=Other; api=GetLogEvents");
        assert!(
            session
                .update_input(InputField::StartText, "2024-01-02 03:04:00".to_string())
                .is_ok()
        );
    }

    #[test]
    fn aborting_before_the_job_started_also_fails_the_fetch() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert!(session.abort_fetch_with_failure(other_failure()));
        assert_eq!(session.phase(), Phase::Failed);
        assert_eq!(session.view().last_job.unwrap().job_id, 0);
    }

    #[test]
    fn aborting_outside_fetching_changes_nothing() {
        let mut session = AppSession::with_catalog(catalog());
        assert!(!session.abort_fetch_with_failure(other_failure()));
        assert_eq!(session.phase(), Phase::Idle);
        assert!(session.view().last_job.is_none());

        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, 5, None);
        session.finish_fetch(&job);
        let before = session.view();
        assert!(!session.abort_fetch_with_failure(other_failure()));
        assert_eq!(session.view(), before);
    }

    /// A session in Done with `events` shown logs.
    fn done_session(events: u64) -> AppSession {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, events, None);
        session.finish_fetch(&job);
        session
    }

    #[test]
    fn starts_with_nothing_selected_and_no_list() {
        let session = AppSession::with_catalog(catalog());
        let view = session.view();
        assert_eq!(view.connection, ConnectionSelection::default());
        assert!(view.log_groups.is_none());
        assert!(!view.can_reload);
        assert!(view.can_change_connection);
        assert_eq!(view.profiles.len(), 3);
        assert!(view.regions.contains(&"ap-northeast-1".to_string()));
    }

    #[test]
    fn unselected_reasons_and_shared_reasons_are_both_listed() {
        let session = AppSession::with_catalog(catalog());
        let reasons = session.view().validation_errors;
        for key in [
            "selection.profileRequired",
            "selection.regionRequired",
            "selection.logGroupRequired",
            "validation.startFormat",
        ] {
            assert!(reasons.contains(&key.to_string()), "{key}: {reasons:?}");
        }
        assert!(!reasons.contains(&"validation.logGroupRequired".to_string()));
        assert!(
            !reasons.iter().any(|key| key.contains("logStream")),
            "no stream name since U3:BR6.1"
        );
    }

    #[test]
    fn profile_without_default_region_asks_for_a_region_and_lists_nothing() {
        let mut session = AppSession::with_catalog(catalog());
        let effect = session.select_profile(ProfileSelector::SdkDefault).unwrap();
        assert!(effect.listing.is_none());
        let view = session.view();
        assert_eq!(view.connection.region, None);
        assert!(view.log_groups.is_none());
        assert!(
            view.validation_errors
                .contains(&"selection.regionRequired".to_string())
        );
        let effect = session.select_region("eu-west-1".to_string()).unwrap();
        let listing = effect.listing.unwrap();
        assert_eq!(listing.profile, ProfileSelector::SdkDefault);
        assert_eq!(listing.region, "eu-west-1");
    }

    #[test]
    fn fetch_request_carries_the_selected_profile_region_and_log_group() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        let validated = session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(validated.request.profile(), &dev());
        assert_eq!(validated.request.region(), Some("ap-northeast-1"));
        assert_eq!(validated.request.log_group_name(), "/aws/lambda/orders");
    }

    #[test]
    fn applying_a_connection_change_returns_done_to_idle_and_clears_logs() {
        let mut session = done_session(0);
        let generation = session.view().connection_generation;
        let effect = session
            .select_profile(ProfileSelector::Named("prod".to_string()))
            .unwrap();
        assert!(effect.logs_cleared);
        assert!(!effect.awaiting_confirmation);
        assert_eq!(
            effect.listing.as_ref().map(|l| l.region.as_str()),
            Some("us-east-1")
        );
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert_eq!(view.event_count, 0);
        assert_eq!(view.last_job, None);
        assert_eq!(view.selected_log_group_name, None);
        assert_eq!(view.connection_generation, generation + 1);
        assert_eq!(
            view.log_groups.map(|l| l.status),
            Some(ListingStatus::Loading)
        );
    }

    #[test]
    fn with_logs_a_change_waits_for_confirmation_and_blocks_other_operations() {
        let mut session = done_session(5);
        let effect = session.select_region("eu-west-1".to_string()).unwrap();
        assert!(effect.awaiting_confirmation);
        assert!(effect.listing.is_none());
        let view = session.view();
        assert!(view.pending_change.is_some());
        assert!(!view.can_fetch && !view.can_reload && !view.can_change_connection);
        assert_eq!(view.connection.region.as_deref(), Some("ap-northeast-1"));
        assert_eq!(
            session.select_log_group(
                "/aws/lambda/MyFunction".to_string(),
                session.connection_generation()
            ),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(
            session.reload_log_groups(session.connection_generation()),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(
            session.begin_fetch(session.connection_generation()),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(
            session.select_profile(ProfileSelector::SdkDefault),
            Err(SessionError::ConfirmationPending)
        );

        session.cancel_connection_change().unwrap();
        let view = session.view();
        assert!(view.pending_change.is_none());
        assert_eq!(view.event_count, 5, "cancel keeps the logs");
        assert_eq!(
            view.selected_log_group_name.as_deref(),
            Some("/aws/lambda/orders")
        );
        assert_eq!(
            session.cancel_connection_change(),
            Err(SessionError::NothingPending)
        );
    }

    #[test]
    fn typing_and_filtering_are_refused_while_confirmation_is_pending() {
        let mut session = done_session(5);
        session
            .update_log_group_filter("orders".to_string())
            .unwrap();
        session.select_region("eu-west-1".to_string()).unwrap();
        assert!(session.view().pending_change.is_some());
        assert_eq!(
            session.update_input(InputField::EndText, "other".to_string()),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(
            session.update_input(InputField::StartText, "2024-01-01 00:00:00".to_string()),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(
            session.update_log_group_filter("x".to_string()),
            Err(SessionError::ConfirmationPending)
        );
        let view = session.view();
        assert_eq!(view.input.end_text, "2024-01-02 03:05:05");
        assert_eq!(view.log_group_filter, "orders");
        assert!(!view.can_change_connection && !view.can_fetch && !view.can_reload);

        session.cancel_connection_change().unwrap();
        assert!(
            session
                .update_input(InputField::EndText, "other".to_string())
                .is_ok()
        );
        assert!(session.update_log_group_filter("x".to_string()).is_ok());
    }

    #[test]
    fn filtering_is_allowed_during_a_fetch_but_typing_is_not() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert!(
            session
                .update_log_group_filter("lambda".to_string())
                .is_ok()
        );
        assert_eq!(
            session.update_input(InputField::EndText, "x".to_string()),
            Err(SessionError::Busy)
        );
    }

    #[test]
    fn confirming_a_change_after_failure_returns_to_idle() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let failure = ApiFailure::new(FailureKind::Throttled, &SafeDetailFields::default());
        let job = finished_job(&mut session, 3, Some(failure));
        session.finish_fetch(&job);
        assert!(
            session
                .select_profile(ProfileSelector::SdkDefault)
                .unwrap()
                .awaiting_confirmation
        );
        let effect = session.confirm_connection_change().unwrap();
        assert!(effect.logs_cleared);
        assert!(
            effect.listing.is_none(),
            "the SDK default has no region here"
        );
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert_eq!(view.last_job, None);
        assert_eq!(view.connection.profile, Some(ProfileSelector::SdkDefault));
        assert!(view.log_groups.is_none());
        assert_eq!(
            session.confirm_connection_change(),
            Err(SessionError::NothingPending)
        );
    }

    #[test]
    fn selections_and_reload_are_refused_while_fetching() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(
            session.select_profile(ProfileSelector::SdkDefault),
            Err(SessionError::Busy)
        );
        assert_eq!(
            session.select_region("eu-west-1".to_string()),
            Err(SessionError::Busy)
        );
        assert_eq!(
            session.select_log_group(
                "/aws/lambda/MyFunction".to_string(),
                session.connection_generation()
            ),
            Err(SessionError::Busy)
        );
        assert_eq!(
            session.reload_log_groups(session.connection_generation()),
            Err(SessionError::Busy)
        );
        assert!(!session.view().can_change_connection);
    }

    #[test]
    fn reload_keeps_the_selection_and_the_logs_and_drops_old_pages() {
        let mut session = done_session(4);
        let old_listing = session.listing().unwrap().listing_id().to_string();
        let request = session
            .reload_log_groups(session.connection_generation())
            .unwrap();
        assert_ne!(request.listing_id, old_listing);
        let view = session.view();
        assert_eq!(
            view.selected_log_group_name.as_deref(),
            Some("/aws/lambda/orders")
        );
        assert_eq!(view.event_count, 4);
        assert_eq!(view.phase, Phase::Done);
        assert_eq!(
            session.apply_listing_page(&old_listing, vec![group("/late")], None),
            None
        );
        assert!(!session.is_current_listing(&old_listing));
        assert!(session.is_current_listing(&request.listing_id));
        assert!(!session.apply_listing_failure(
            &old_listing,
            ApiFailure::new(FailureKind::Network, &SafeDetailFields::default())
        ));
    }

    #[test]
    fn reselecting_a_log_group_keeps_the_logs() {
        let mut session = done_session(6);
        session
            .select_log_group(
                "/aws/lambda/MyFunction".to_string(),
                session.connection_generation(),
            )
            .unwrap();
        let view = session.view();
        assert_eq!(
            view.selected_log_group_name.as_deref(),
            Some("/aws/lambda/MyFunction")
        );
        assert_eq!(view.event_count, 6);
        assert_eq!(view.phase, Phase::Done);
        assert_eq!(
            session.select_log_group("/not/listed".to_string(), session.connection_generation()),
            Err(SessionError::UnknownLogGroup)
        );
    }

    #[test]
    fn list_view_filters_and_keeps_the_filter_across_changes() {
        let (mut session, _) = connected_session();
        session
            .update_log_group_filter("  myfunc ".to_string())
            .unwrap();
        let list = session.view().log_groups.unwrap();
        assert_eq!(list.visible_groups, ["/aws/lambda/MyFunction"]);
        assert_eq!(list.total_count, 2);
        assert_eq!(list.status, ListingStatus::Complete);
        session.update_log_group_filter("zzz".to_string()).unwrap();
        assert_eq!(
            session.view().log_groups.unwrap().empty_state,
            Some(EmptyState::NoMatches)
        );
        session.select_region("eu-west-1".to_string()).unwrap();
        assert_eq!(session.view().log_group_filter, "zzz");
    }

    #[test]
    fn unknown_profile_and_region_are_refused() {
        let mut session = AppSession::with_catalog(catalog());
        assert_eq!(
            session.select_profile(ProfileSelector::Named("ghost".to_string())),
            Err(SessionError::UnknownProfile)
        );
        assert_eq!(
            session.select_region("moon-1".to_string()),
            Err(SessionError::UnknownRegion)
        );
        assert_eq!(
            session.reload_log_groups(session.connection_generation()),
            Err(SessionError::NotConnected)
        );
        assert_eq!(
            SessionError::ConfirmationPending.message_key(),
            "session.confirmationPending"
        );
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
        let field: InputField = serde_json::from_str("\"startText\"").unwrap();
        assert_eq!(field, InputField::StartText);
        assert!(serde_json::from_str::<InputField>("\"logStreamName\"").is_err());
        assert_eq!(SessionError::Busy.message_key(), "session.busy");
    }

    // --- U3: connection generation (BR6.7, review R-02) ---

    #[test]
    fn each_applied_connection_change_increments_the_generation() {
        let mut session = AppSession::with_catalog(catalog());
        assert_eq!(session.connection_generation(), 0);
        session.select_profile(dev()).unwrap();
        assert_eq!(session.connection_generation(), 1);
        session.select_region("eu-west-1".to_string()).unwrap();
        assert_eq!(session.connection_generation(), 2);
        // Choosing the same region again changes nothing.
        session.select_region("eu-west-1".to_string()).unwrap();
        assert_eq!(session.connection_generation(), 2);
    }

    #[test]
    fn a_change_awaiting_confirmation_increments_only_when_confirmed() {
        let mut session = done_session(5);
        let before = session.connection_generation();
        session.select_region("eu-west-1".to_string()).unwrap();
        assert_eq!(session.connection_generation(), before);
        session.cancel_connection_change().unwrap();
        assert_eq!(session.connection_generation(), before);
        session.select_region("eu-west-1".to_string()).unwrap();
        session.confirm_connection_change().unwrap();
        assert_eq!(session.connection_generation(), before + 1);
    }

    #[test]
    fn operations_of_an_older_generation_are_dropped_without_changing_state() {
        let (mut session, _) = connected_session();
        let old = session.connection_generation();
        session
            .select_log_group("/aws/lambda/orders".to_string(), old)
            .unwrap();
        // The user switches region; the list of the new region arrives.
        let listing = session
            .select_region("eu-west-1".to_string())
            .unwrap()
            .listing
            .unwrap();
        session.apply_listing_page(
            &listing.listing_id,
            vec![group("/aws/lambda/orders"), group("/eu/only")],
            None,
        );
        let before = session.view();

        assert_eq!(
            session.select_log_group("/aws/lambda/orders".to_string(), old),
            Err(SessionError::StaleGeneration)
        );
        assert_eq!(
            session.reload_log_groups(old),
            Err(SessionError::StaleGeneration)
        );
        assert_eq!(session.begin_fetch(old), Err(SessionError::StaleGeneration));
        assert_eq!(session.view(), before);
        assert_eq!(before.selected_log_group_name, None);
        assert_eq!(before.connection.region.as_deref(), Some("eu-west-1"));
    }

    #[test]
    fn operations_of_the_current_generation_apply_in_the_order_received() {
        let (mut session, _) = connected_session();
        let current = session.connection_generation();
        session
            .select_log_group("/aws/lambda/orders".to_string(), current)
            .unwrap();
        session
            .select_log_group("/aws/lambda/MyFunction".to_string(), current)
            .unwrap();
        assert_eq!(
            session.view().selected_log_group_name.as_deref(),
            Some("/aws/lambda/MyFunction")
        );
        assert!(session.reload_log_groups(current).is_ok());
        assert_eq!(session.view().connection_generation, current);
        assert_eq!(
            SessionError::StaleGeneration.message_key(),
            "session.staleGeneration"
        );
    }

    // --- U3: progress, failures, timeline version (BR5.6, BR6.1-BR6.3) ---

    fn failed_stream(name: &str) -> FailedStream {
        FailedStream {
            log_stream_name: name.to_string(),
            failure: ApiFailure::new(
                FailureKind::Throttled,
                &SafeDetailFields {
                    api_name: Some("GetLogEvents".to_string()),
                    log_stream_name: Some(name.to_string()),
                    ..SafeDetailFields::default()
                },
            ),
        }
    }

    /// A session in Done after a fetch where `failed` streams failed.
    fn done_with_failures(events: u64, failed: &[&str]) -> AppSession {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let mut job = finished_job(&mut session, events, None);
        job.planned_stream_count = 3;
        job.finished_stream_count = 3;
        job.failed_streams = failed.iter().map(|name| failed_stream(name)).collect();
        job.status = JobStatus::CompletedWithFailures;
        session.finish_fetch(&job);
        session
    }

    #[test]
    fn fetch_is_possible_without_a_stream_name_but_not_without_a_log_group() {
        let (mut session, _) = connected_session();
        for (field, value) in [
            (InputField::StartText, "2024-01-02 03:04:05"),
            (InputField::EndText, "2024-01-02 03:05:05"),
        ] {
            session.update_input(field, value.to_string()).unwrap();
        }
        assert!(!session.can_fetch());
        assert_eq!(
            session.view().validation_errors,
            vec!["selection.logGroupRequired"]
        );
        session
            .select_log_group(
                "/aws/lambda/orders".to_string(),
                session.connection_generation(),
            )
            .unwrap();
        assert!(session.can_fetch());
        assert!(session.view().validation_errors.is_empty());
    }

    #[test]
    fn starting_a_fetch_clears_the_previous_count_failures_and_result() {
        let mut session = done_with_failures(9, &["s1"]);
        session.set_failure_list_open(true).unwrap();
        assert!(session.view().failure_list_open);

        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let view = session.view();
        assert_eq!(view.phase, Phase::Fetching);
        assert_eq!(view.event_count, 0);
        assert_eq!(view.last_job, None);
        assert!(view.failed_streams.is_empty());
        assert_eq!(view.listing_failure, None);
        assert!(!view.failure_list_open, "the list closes and empties");
        assert_eq!(view.progress, Some(FetchProgress::default()));
    }

    #[test]
    fn progress_shows_listing_then_fetching_counts() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let validated = session.validation.clone().unwrap();
        let mut job = FetchJob::start(validated.range);
        session.on_job_started(&job, 4);
        assert_eq!(
            session.view().timeline_version,
            4,
            "the discard at the start is shown"
        );
        session.on_listing_progress(
            job.job_id,
            ListingProgress {
                seen_stream_count: 50,
                selected_stream_count: 7,
            },
        );
        let progress = session.view().progress.unwrap();
        assert_eq!(progress.planned_stream_count, None, "still listing");
        assert_eq!(progress.selected_stream_count, 7);

        job.planned_stream_count = 8;
        session.on_planned(&job);
        session.on_batch(job.job_id, batch(120, 5));
        job.finished_stream_count = 1;
        session.on_stream_finished(&job);
        let view = session.view();
        let progress = view.progress.unwrap();
        assert_eq!(progress.planned_stream_count, Some(8));
        assert_eq!(progress.finished_stream_count, 1);
        assert_eq!(progress.event_count, 120);
        assert_eq!(view.event_count, 120);
        assert_eq!(view.timeline_version, 5);

        // Progress of another job is ignored.
        session.on_batch(job.job_id + 1, batch(999, 99));
        assert_eq!(session.view().event_count, 120);
    }

    #[test]
    fn completed_with_failures_is_done_with_the_failed_streams() {
        let session = done_with_failures(9, &["s1", "s2"]);
        let view = session.view();
        assert_eq!(view.phase, Phase::Done);
        assert_eq!(view.event_count, 9);
        assert_eq!(view.progress, None);
        let names: Vec<_> = view
            .failed_streams
            .iter()
            .map(|f| f.log_stream_name.as_str())
            .collect();
        assert_eq!(names, vec!["s1", "s2"]);
        let summary = view.last_job.unwrap();
        assert_eq!(summary.status, JobStatus::CompletedWithFailures);
        assert_eq!(summary.failed_stream_count, 2);
        assert!(session.can_fetch(), "the inputs are enabled again");
    }

    #[test]
    fn the_failure_list_opens_only_with_failures_and_always_closes() {
        let mut session = done_session(3);
        assert_eq!(
            session.set_failure_list_open(true),
            Err(SessionError::NoFailures)
        );
        assert!(session.set_failure_list_open(false).is_ok());

        let mut session = done_with_failures(3, &["s1"]);
        session.set_failure_list_open(true).unwrap();
        assert!(session.view().failure_list_open);
        session.set_failure_list_open(false).unwrap();
        assert!(!session.view().failure_list_open);
        assert_eq!(SessionError::NoFailures.message_key(), "session.noFailures");
    }

    #[test]
    fn an_aborted_job_returns_to_idle_with_nothing_shown() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let mut job = finished_job(&mut session, 4, None);
        session.on_batch(job.job_id, batch(4, 2));
        job.status = JobStatus::Aborted;
        session.finish_fetch(&job);
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert_eq!(view.event_count, 0);
        assert_eq!(view.progress, None);
    }

    #[test]
    fn a_connection_change_clears_failures_and_takes_the_new_timeline_version() {
        let mut session = done_with_failures(0, &["s1"]);
        session.set_failure_list_open(true).unwrap();
        let effect = session.select_region("eu-west-1".to_string()).unwrap();
        assert!(effect.logs_cleared);
        session.set_timeline_version(12);
        let view = session.view();
        assert!(view.failed_streams.is_empty());
        assert!(!view.failure_list_open);
        assert_eq!(view.timeline_version, 12);
        session.set_timeline_version(3);
        assert_eq!(session.view().timeline_version, 12, "never goes back");
    }
}
