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
//! Since U4 it holds the chosen time zone and the start and end inputs as
//! [`DateTimeInput`]s read in that zone; switching the zone is accepted at
//! any time and rewrites the inputs without touching the phase (U4:BR1.4).
//! It also composes the rows the screen shows with their times formatted
//! in the chosen zone (U4:BR3.4, review R-08).
//! Since U5 it holds the log filter text as typed and the latest filter
//! summary copied from the LogView: it trims the text and says whether the
//! filter must change (U5:BR1.1, BR1.4), accepts it during a fetch and
//! keeps it across fetches, and refuses it while a connection change
//! awaits confirmation. The summary goes into [`SessionView`],
//! [`FetchProgressUpdate`] and [`FilterProgressUpdate`] (U5:BR3.3, BR3.6).
//! Since U6 it holds the disk cache setting and the settings dialog: the
//! app passes the settings file and the cache folder with
//! [`AppSession::with_cache_location`] (a session made without them never
//! touches the disk, U6:BR1.6); the setting is read at startup (U6:BR1.1,
//! BR1.2), saved from the dialog, and switching it off removes every cached
//! file (U6:BR1.3, review R-10). While the dialog is open, everything that
//! a pending connection change refuses is refused too (U6:BR5.1). The
//! session also shows "saving to the cache" while a fetch writes and the
//! cache notices of the last fetch (U6:BR3.7, BR5.3, review R-11).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cache::LogCache;
use crate::cache::plan::{CacheNotice, cache_notices};
use crate::cache::settings::{load_settings, save_settings};
use crate::catalog::{ConnectionCatalog, ConnectionProfile, ProfileKind, ProfileSelector};
use crate::connection::{
    ChangeOutcome, ConnectionSelection, ConnectionState, ListingCommand, PendingConnectionChange,
};
use crate::coordinator::{BatchProgress, FailedStream, FetchJob, JobStatus};
use crate::date_input::DateTimeInput;
use crate::event::LogEvent;
use crate::failure::ApiFailure;
use crate::filter::{FilterState, FilterSummary, normalize_filter_text};
use crate::log_groups::listing::ListingRequest;
use crate::log_groups::{
    EmptyState, ListingStatus, LogGroup, LogGroupListing, apply_failure_if_current,
    apply_page_if_current, empty_state, filter_groups,
};
use crate::paging::PageDecision;
use crate::request::{ValidatedFetch, ValidationError, message_keys, validate_session_input};
use crate::streams::StreamListingStatus;
use crate::streams::planner::ListingProgress;
use crate::time_range::format_display_in_zone;
use crate::time_zone::{TimeZoneChoice, TimeZoneContext};
use crate::timeline::RowWindow;

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
    /// The settings dialog is not open (U6:BR5.1).
    #[error("the settings dialog is not open")]
    SettingsClosed,
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
            SessionError::SettingsClosed => "session.settingsClosed",
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

/// Where the settings file and the cache folder are, decided by the app at
/// startup from the OS folders (U6:BR1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLocation {
    /// The settings file (in the OS app settings folder).
    pub settings_path: PathBuf,
    /// The cache folder (in the OS cache folder).
    pub cache_directory: PathBuf,
}

/// Whether the settings dialog is open (entities.md SessionState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum SettingsDialog {
    /// Closed.
    #[default]
    Closed,
    /// Open: other operations are refused (U6:BR5.1).
    Open,
}

/// What the settings dialog tells after an operation (U6:BR1.3, BR1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SettingsNotice {
    /// The cached files were removed.
    Cleared,
    /// The cached files could not all be removed.
    ClearFailed,
    /// The setting could not be written; it is unchanged.
    SaveFailed,
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

/// The light progress message sent to the screen for every page while a
/// fetch runs, instead of the whole [`SessionView`] (review R-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchProgressUpdate {
    /// The running job.
    pub job_id: u64,
    /// Its progress.
    pub progress: FetchProgress,
    /// Events added so far.
    pub event_count: u64,
    /// Version of the timeline.
    pub timeline_version: u64,
    /// The log filter summary, `None` without a filter (U5:BR3.6).
    pub filter_summary: Option<FilterSummary>,
    /// Result version of the log filter, for ordering (U5:BR3.6).
    pub filter_result_version: u64,
    /// Whether the fetched logs are being written to the cache (U6:BR3.7).
    pub cache_saving: bool,
}

/// The light message sent when the log filter result changes without a
/// fetch page, for example while the scan runs or when it becomes Ready
/// (U5:BR3.6). Same weight as [`FetchProgressUpdate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterProgressUpdate {
    /// The log filter summary, `None` without a filter.
    pub filter_summary: Option<FilterSummary>,
    /// Result version of the log filter; the screen ignores a message older
    /// than the one it shows.
    pub filter_result_version: u64,
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
    /// The chosen time zone of the inputs and the log list (U4:BR1.1).
    pub time_zone: TimeZoneChoice,
    /// The start input: its text in the chosen zone, and its error.
    pub start_input: DateTimeInput,
    /// The end input: its text in the chosen zone, and its error.
    pub end_input: DateTimeInput,
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
    /// The log filter text as typed (U5:BR3.4).
    pub log_filter: String,
    /// The log filter summary, `None` without a filter (U5:BR3.3, BR3.6).
    pub filter_summary: Option<FilterSummary>,
    /// Result version of the log filter, for ordering (U5:BR3.6).
    pub filter_result_version: u64,
    /// Whether the disk cache is enabled (U6:BR1.1).
    pub cache_enabled: bool,
    /// The cache folder shown in the settings dialog (FR7.3); `None` when
    /// the app gave no location.
    pub cache_directory: Option<String>,
    /// Whether the settings dialog is open (U6:BR5.1).
    pub settings_dialog: SettingsDialog,
    /// What the dialog tells after its last operation.
    pub settings_notice: Option<SettingsNotice>,
    /// Whether the settings dialog may be opened now (U6:BR5.1).
    pub can_open_settings: bool,
    /// Whether the fetched logs are being written to the cache (U6:BR3.7).
    pub cache_saving: bool,
    /// Cache notices of the last fetch (U6:BR5.3).
    pub cache_notices: Vec<CacheNotice>,
}

/// One log row as the screen shows it: the event and its time written in
/// the chosen time zone by TimeRangeModel (U4:BR3.1, BR3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayRow {
    /// The event as held by the EventTimeline.
    #[serde(flatten)]
    pub event: LogEvent,
    /// `yyyy-mm-dd hh:mm:ss.mmm` in the chosen zone, or the raw number when
    /// that cannot be written; the screen shows it as is.
    pub display_time: String,
}

/// The rows of one viewport request with their display times (U3's
/// RowWindow plus `rows[].displayTime`, entities.md RowWindow).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayRowWindow {
    /// Position of the first row.
    pub offset: u64,
    /// The rows, in timeline order.
    pub rows: Vec<DisplayRow>,
    /// Number of rows the offset counts in (the matching events while a
    /// log filter is in force, U5:BR3.1).
    pub total_count: u64,
    /// Version of the timeline when the rows were taken.
    pub timeline_version: u64,
    /// Whether the rows are those of the filter result (U5:BR3.1).
    pub filtered: bool,
    /// Number of held events (U5 FR6.3).
    pub all_count: u64,
    /// The filter result version, when filtered (U5:BR3.6).
    pub result_version: Option<u64>,
}

/// State of one app session.
#[derive(Debug, Clone)]
pub struct AppSession {
    session_id: String,
    phase: Phase,
    time_zones: TimeZoneContext,
    time_zone: TimeZoneChoice,
    start_input: DateTimeInput,
    end_input: DateTimeInput,
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
    /// Number of the latest fetch started by [`AppSession::begin_fetch`].
    fetch_number: u64,
    /// The log filter text as typed (U5).
    log_filter: String,
    /// The trimmed text last handed to the filter (U5:BR1.1, BR1.4).
    applied_log_filter: String,
    /// Latest filter summary copied from the LogView (U5:BR3.6).
    filter: FilterState,
    /// The settings file and cache folder; `None`: no disk (U6:BR1.6).
    cache_location: Option<CacheLocation>,
    /// Whether the disk cache is enabled (U6:BR1.1).
    cache_enabled: bool,
    settings_dialog: SettingsDialog,
    settings_notice: Option<SettingsNotice>,
    /// From `on_saving` to the end of the fetch (U6:BR3.7).
    cache_saving: bool,
    cache_notices: Vec<CacheNotice>,
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

    /// A new session in phase Idle with nothing selected (U2:BR2.1) whose
    /// local time zone is UTC. The app passes the OS time zone with
    /// [`AppSession::with_catalog_and_time_zones`].
    pub fn with_catalog(catalog: ConnectionCatalog) -> Self {
        Self::with_catalog_and_time_zones(catalog, TimeZoneContext::utc())
    }

    /// A new session in phase Idle with nothing selected (U2:BR2.1), the
    /// time zone Local (U4:BR1.1) and `time_zones` saying which zone that is
    /// (U4:BR3.4).
    pub fn with_catalog_and_time_zones(
        catalog: ConnectionCatalog,
        time_zones: TimeZoneContext,
    ) -> Self {
        let mut session = Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            phase: Phase::Idle,
            time_zones,
            time_zone: TimeZoneChoice::Local,
            start_input: DateTimeInput::default(),
            end_input: DateTimeInput::default(),
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
            fetch_number: 0,
            log_filter: String::new(),
            applied_log_filter: String::new(),
            filter: FilterState::default(),
            cache_location: None,
            cache_enabled: false,
            settings_dialog: SettingsDialog::Closed,
            settings_notice: None,
            cache_saving: false,
            cache_notices: Vec::new(),
        };
        session.revalidate();
        session
    }

    /// The same session with the settings file and the cache folder the app
    /// decided at startup (U6:BR1.6). Reads the setting: enabled only when
    /// the file says so (U6:BR1.1, BR1.2).
    pub fn with_cache_location(mut self, location: CacheLocation) -> Self {
        self.cache_enabled = load_settings(&location.settings_path).cache_enabled();
        self.cache_location = Some(location);
        self
    }

    /// Whether the disk cache is enabled (U6:BR1.1).
    pub fn cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    /// The settings file and cache folder, when the app gave them.
    pub fn cache_location(&self) -> Option<&CacheLocation> {
        self.cache_location.as_ref()
    }

    /// The cache a fetch starting now uses: `None` when disabled, so the
    /// fetch neither reads nor writes the disk (U6:BR1.5).
    pub fn fetch_cache(&self) -> Option<LogCache> {
        let location = self.cache_location.as_ref()?;
        self.cache_enabled
            .then(|| LogCache::new(location.cache_directory.clone()))
    }

    /// Whether the settings dialog is open.
    pub fn settings_dialog(&self) -> SettingsDialog {
        self.settings_dialog
    }

    /// Whether the settings dialog may be opened: not while fetching (also
    /// while writing the cache) and not while a connection change awaits
    /// confirmation (U6:BR5.1).
    pub fn can_open_settings(&self) -> bool {
        self.phase != Phase::Fetching && self.connection.pending().is_none()
    }

    /// Opens the settings dialog (U6:BR5.1); opening it again keeps it open.
    pub fn open_settings(&mut self) -> Result<(), SessionError> {
        if self.phase == Phase::Fetching {
            return Err(SessionError::Busy);
        }
        if self.connection.pending().is_some() {
            return Err(SessionError::ConfirmationPending);
        }
        if self.settings_dialog == SettingsDialog::Closed {
            self.settings_dialog = SettingsDialog::Open;
            self.settings_notice = None;
        }
        Ok(())
    }

    /// Closes the dialog without saving ([Cancel] and Escape, U6:BR5.2);
    /// files already removed by [Clear cache] stay removed.
    pub fn cancel_settings(&mut self) {
        self.settings_dialog = SettingsDialog::Closed;
        self.settings_notice = None;
    }

    /// [Save] (U6:BR1.1, BR1.3, review R-10): writes the setting first. When
    /// it cannot be written, the dialog stays open with the notice and the
    /// setting is unchanged. When written, the setting is taken; switching
    /// from enabled to disabled then removes every cached file, and the
    /// dialog closes only when that worked too (otherwise it stays open,
    /// disabled, with the notice).
    pub fn save_settings(&mut self, enabled: bool) -> Result<(), SessionError> {
        if self.settings_dialog != SettingsDialog::Open {
            return Err(SessionError::SettingsClosed);
        }
        let Some(location) = self.cache_location.clone() else {
            self.settings_notice = Some(SettingsNotice::SaveFailed);
            return Ok(());
        };
        if let Err(error) = save_settings(&location.settings_path, enabled) {
            eprintln!("local-sights: settings: the setting could not be saved: {error}");
            self.settings_notice = Some(SettingsNotice::SaveFailed);
            return Ok(());
        }
        let was_enabled = self.cache_enabled;
        self.cache_enabled = enabled;
        if was_enabled && !enabled && !self.clear_cache_files(&location) {
            self.settings_notice = Some(SettingsNotice::ClearFailed);
            return Ok(());
        }
        self.cancel_settings();
        Ok(())
    }

    /// [Clear cache] (U6:BR1.4): removes every cached file at once and tells
    /// the result in the dialog; the setting is unchanged and [Cancel] does
    /// not bring the files back.
    pub fn clear_cache(&mut self) -> Result<(), SessionError> {
        if self.settings_dialog != SettingsDialog::Open {
            return Err(SessionError::SettingsClosed);
        }
        let cleared = match self.cache_location.clone() {
            Some(location) => self.clear_cache_files(&location),
            None => true,
        };
        self.settings_notice = Some(if cleared {
            SettingsNotice::Cleared
        } else {
            SettingsNotice::ClearFailed
        });
        Ok(())
    }

    fn clear_cache_files(&self, location: &CacheLocation) -> bool {
        match LogCache::new(location.cache_directory.clone()).clear_all() {
            Ok(_) => true,
            Err(error) => {
                eprintln!("local-sights: cache: the cache could not be cleared: {error}");
                false
            }
        }
    }

    /// Records that the running job writes to the cache (U6:BR3.7); only
    /// for the current job.
    pub fn on_saving(&mut self, job_id: u64) {
        if self.progress_of(job_id).is_some() {
            self.cache_saving = true;
        }
    }

    /// Whether the running fetch writes to the cache.
    pub fn cache_saving(&self) -> bool {
        self.cache_saving
    }

    /// Cache notices of the last fetch (U6:BR5.3).
    pub fn cache_notices(&self) -> &[CacheNotice] {
        &self.cache_notices
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

    /// The start input (U4:BR1.3).
    pub fn start_input(&self) -> &DateTimeInput {
        &self.start_input
    }

    /// The end input (U4:BR1.3).
    pub fn end_input(&self) -> &DateTimeInput {
        &self.end_input
    }

    /// The chosen time zone (U4:BR1.1).
    pub fn time_zone(&self) -> TimeZoneChoice {
        self.time_zone
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
        self.phase != Phase::Fetching
            && self.connection.pending().is_none()
            && self.settings_dialog == SettingsDialog::Closed
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

    /// Changes one typed field: a changed text is read in the chosen time
    /// zone and the conditions are checked again; the same text again keeps
    /// the instant (U4:BR1.5). Refused while a fetch is running (U1:BR1.4)
    /// and while a connection change awaits confirmation (U2:BR2.6).
    pub fn update_input(&mut self, field: InputField, value: String) -> Result<(), SessionError> {
        self.ensure_can_change()?;
        let zone = self.time_zones.zone(self.time_zone);
        let input = match field {
            InputField::StartText => &mut self.start_input,
            InputField::EndText => &mut self.end_input,
        };
        if input.edit(value, zone) {
            self.revalidate();
        }
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
        if self.connection.pending().is_some() || self.settings_dialog == SettingsDialog::Open {
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
        if self.connection.pending().is_some() || self.settings_dialog == SettingsDialog::Open {
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
        self.fetch_number += 1;
        // BR5.6: the previous rows, count, failures and result go away.
        self.current_job_id = None;
        self.event_count = 0;
        self.last_job = None;
        self.failed_streams.clear();
        self.listing_status = None;
        self.listing_failure = None;
        self.failure_list_open = false;
        self.progress = Some(FetchProgress::default());
        // U6:BR5.3: the cache notices go with the next Fetch.
        self.cache_saving = false;
        self.cache_notices.clear();
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

    /// Number of the latest fetch started by [`AppSession::begin_fetch`]; the
    /// caller keeps it to recognise its own fetch later (review R-03).
    pub fn fetch_number(&self) -> u64 {
        self.fetch_number
    }

    /// The progress of the running job, for the light per-page message
    /// (review R-05); `None` outside a fetch or before the job started.
    pub fn progress_update(&self) -> Option<FetchProgressUpdate> {
        if self.phase != Phase::Fetching {
            return None;
        }
        Some(FetchProgressUpdate {
            job_id: self.current_job_id?,
            progress: self.progress.clone()?,
            event_count: self.event_count,
            timeline_version: self.timeline_version,
            filter_summary: self.filter.summary.clone(),
            filter_result_version: self.filter.result_version,
            cache_saving: self.cache_saving,
        })
    }

    /// Takes the log filter text as typed (U5:BR1.1, BR1.4, BR3.4).
    ///
    /// Accepted while fetching, since filtering is not a fetch condition and
    /// calls no API; refused while a connection change awaits confirmation
    /// (U2:BR2.6). The text is kept across fetches and connection changes.
    /// Returns the trimmed text the LogView must apply, or `None` when the
    /// trimmed text is the one already applied (nothing to redo).
    pub fn update_log_filter(&mut self, text: String) -> Result<Option<String>, SessionError> {
        if self.connection.pending().is_some() || self.settings_dialog == SettingsDialog::Open {
            return Err(SessionError::ConfirmationPending);
        }
        let trimmed = normalize_filter_text(&text);
        self.log_filter = text;
        if trimmed == self.applied_log_filter {
            return Ok(None);
        }
        self.applied_log_filter = trimmed.clone();
        Ok(Some(trimmed))
    }

    /// The log filter text as typed.
    pub fn log_filter(&self) -> &str {
        &self.log_filter
    }

    /// Records the filter summary the caller read from the LogView, under
    /// the session lock and then the LogView lock (U5:BR2.5, BR3.6).
    pub fn set_filter_state(&mut self, state: FilterState) {
        self.filter = state;
    }

    /// The last recorded filter summary.
    pub fn filter_state(&self) -> &FilterState {
        &self.filter
    }

    /// The light message for a filter change without a fetch page (U5:BR3.6).
    pub fn filter_update(&self) -> FilterProgressUpdate {
        FilterProgressUpdate {
            filter_summary: self.filter.summary.clone(),
            filter_result_version: self.filter.result_version,
        }
    }

    /// Records the finished job and the timeline version after it:
    /// Completed and CompletedWithFailures move to Done, Failed to Failed.
    /// Aborted (only when the window closes) returns to Idle with nothing
    /// shown and takes the version of the discard (BR4.5, review R-02). A
    /// job that is still Running, or not the current one, leaves the
    /// session as it is.
    pub fn finish_fetch(&mut self, job: &FetchJob, timeline_version: u64) {
        let current = self.current_job_id.is_none_or(|id| id == job.job_id);
        if self.phase != Phase::Fetching || !current || job.status == JobStatus::Running {
            return;
        }
        self.set_timeline_version(timeline_version);
        self.current_job_id = Some(job.job_id);
        self.progress = None;
        // U6:BR3.7, BR5.3: the write is over; keep the notices of this job.
        self.cache_saving = false;
        self.cache_notices = if job.status == JobStatus::Aborted {
            Vec::new()
        } else {
            cache_notices(job.served_from_cache, job.read_failed, job.cache_outcome)
        };
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
    /// whether the session changed. It does nothing outside Fetching, and
    /// nothing when `fetch_number` is not the running fetch (a newer fetch
    /// has started meanwhile, review R-03).
    pub fn abort_fetch_with_failure(&mut self, fetch_number: u64, failure: ApiFailure) -> bool {
        if self.phase != Phase::Fetching || fetch_number != self.fetch_number {
            return false;
        }
        self.phase = Phase::Failed;
        // U6 review R-11: as at a normal finish, the write is over.
        self.cache_saving = false;
        self.cache_notices.clear();
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

    /// Switches the time zone of the inputs and the log list (U4:BR1.4).
    ///
    /// Accepted at any time, also while fetching and while a connection
    /// change awaits confirmation, because it changes no fetch condition;
    /// the phase stays as it is. Each input keeps its instant and gets its
    /// text rewritten, or keeps its text and is read again in the new zone
    /// (see [`DateTimeInput::switch_zone`]); then the conditions are checked
    /// again (U4:BR2.1). No API is called and the timeline is untouched: the
    /// screen re-reads its rows when the view's `time_zone` changes (R-08).
    pub fn select_time_zone(&mut self, choice: TimeZoneChoice) {
        if choice == self.time_zone {
            return;
        }
        self.time_zone = choice;
        let zone = self.time_zones.zone(choice);
        self.start_input.switch_zone(zone);
        self.end_input.switch_zone(zone);
        self.revalidate();
    }

    /// Adds to rows read from the EventTimeline their times written in the
    /// chosen zone (U4:BR3.1, BR3.4). AppSession composes them so that the
    /// EventTimeline needs no time zone (review R-08).
    pub fn display_rows(&self, window: RowWindow) -> DisplayRowWindow {
        let zone = self.time_zones.zone(self.time_zone);
        DisplayRowWindow {
            offset: window.offset,
            rows: window
                .rows
                .into_iter()
                .map(|event| DisplayRow {
                    display_time: format_display_in_zone(event.timestamp, zone),
                    event,
                })
                .collect(),
            total_count: window.total_count,
            timeline_version: window.timeline_version,
            filtered: window.filtered,
            all_count: window.all_count,
            result_version: window.result_version,
        }
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
            time_zone: self.time_zone,
            start_input: self.start_input.clone(),
            end_input: self.end_input.clone(),
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
            log_filter: self.log_filter.clone(),
            filter_summary: self.filter.summary.clone(),
            filter_result_version: self.filter.result_version,
            cache_enabled: self.cache_enabled,
            cache_directory: self
                .cache_location
                .as_ref()
                .map(|location| location.cache_directory.display().to_string()),
            settings_dialog: self.settings_dialog,
            settings_notice: self.settings_notice,
            can_open_settings: self.can_open_settings(),
            cache_saving: self.cache_saving,
            cache_notices: self.cache_notices.clone(),
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
        } else if self.connection.pending().is_some()
            || self.settings_dialog == SettingsDialog::Open
        {
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
        self.cache_notices.clear();
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

    /// Runs the shared validation with the selected log group and the
    /// instants of the inputs (U1:BR1.8, U4:BR1.6, BR2.1).
    fn revalidate(&mut self) {
        self.validation = validate_session_input(
            self.selected_log_group.as_deref().unwrap_or_default(),
            &self.start_input,
            &self.end_input,
        );
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
        assert_eq!(session.start_input().text(), "2024-01-02 03:04:05");
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
        session.finish_fetch(&job, 0);
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
        session.finish_fetch(&job, 0);
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
        session.finish_fetch(&job, 0);
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
        session.finish_fetch(&job, 0);
        session
            .update_input(InputField::StartText, "2024-01-02 03:04:00".to_string())
            .unwrap();
        assert!(session.begin_fetch(session.connection_generation()).is_ok());
        assert_eq!(session.view().event_count, 0);
        let failure = ApiFailure::new(FailureKind::Network, &SafeDetailFields::default());
        let job = finished_job(&mut session, 0, Some(failure));
        session.finish_fetch(&job, 0);
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

        assert!(session.abort_fetch_with_failure(session.fetch_number(), other_failure()));

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
        assert!(session.abort_fetch_with_failure(session.fetch_number(), other_failure()));
        assert_eq!(session.phase(), Phase::Failed);
        assert_eq!(session.view().last_job.unwrap().job_id, 0);
    }

    #[test]
    fn aborting_outside_fetching_changes_nothing() {
        let mut session = AppSession::with_catalog(catalog());
        assert!(!session.abort_fetch_with_failure(session.fetch_number(), other_failure()));
        assert_eq!(session.phase(), Phase::Idle);
        assert!(session.view().last_job.is_none());

        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let job = finished_job(&mut session, 5, None);
        session.finish_fetch(&job, 0);
        let before = session.view();
        assert!(!session.abort_fetch_with_failure(session.fetch_number(), other_failure()));
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
        session.finish_fetch(&job, 0);
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
        assert_eq!(view.end_input.text(), "2024-01-02 03:05:05");
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
        session.finish_fetch(&job, 0);
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
        assert_eq!(json["timeZone"], "Local");
        assert_eq!(json["startInput"]["text"], "");
        assert!(json["endInput"]["error"].is_null());
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
        session.finish_fetch(&job, 0);
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
        session.finish_fetch(&job, 0);
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

    // --- U3 review-1 fixes (R-02, R-03, R-05) ---

    #[test]
    fn an_aborted_finish_takes_the_version_of_the_discard() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let mut job = finished_job(&mut session, 4, None);
        session.on_batch(job.job_id, batch(4, 2));
        job.status = JobStatus::Aborted;
        session.finish_fetch(&job, 3);
        let view = session.view();
        assert_eq!(view.phase, Phase::Idle);
        assert_eq!(view.timeline_version, 3, "BR4.5: the discard is shown");
    }

    #[test]
    fn recovery_only_fails_the_fetch_it_belongs_to() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let first = session.fetch_number();
        let job = finished_job(&mut session, 1, None);
        session.finish_fetch(&job, 1);
        // A second fetch starts before the first fetch's recovery runs.
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(session.fetch_number(), first + 1);
        assert!(!session.abort_fetch_with_failure(first, other_failure()));
        assert_eq!(session.phase(), Phase::Fetching);
        assert!(session.abort_fetch_with_failure(first + 1, other_failure()));
        assert_eq!(session.phase(), Phase::Failed);
    }

    #[test]
    fn the_progress_update_carries_only_the_running_job_progress() {
        let mut session = AppSession::with_catalog(catalog());
        assert_eq!(session.progress_update(), None);
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(session.progress_update(), None, "no job yet");
        let validated = session.validation.clone().unwrap();
        let job = FetchJob::start(validated.range);
        session.on_job_started(&job, 2);
        session.on_batch(job.job_id, batch(30, 3));
        let update = session.progress_update().unwrap();
        assert_eq!(update.job_id, job.job_id);
        assert_eq!(update.event_count, 30);
        assert_eq!(update.timeline_version, 3);
        assert_eq!(update.progress.event_count, 30);
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["timelineVersion"], 3);
        assert!(json.get("profiles").is_none(), "no full session view");
    }

    // ---- U4: the [Fetch] conditions and their reasons (BR2.1, R-04) ----

    use crate::time_zone::TimeZoneContext;

    /// A session whose local time zone is New York (BR3.4: tests never read
    /// the OS setting).
    fn new_york_session() -> AppSession {
        AppSession::with_catalog_and_time_zones(
            catalog(),
            TimeZoneContext::new(chrono_tz::America::New_York),
        )
    }

    #[test]
    fn every_reason_is_listed_in_the_order_of_the_conditions() {
        let mut session = new_york_session();
        session
            .update_input(InputField::StartText, "2024-03-10 02:30:00".to_string())
            .unwrap();
        session
            .update_input(InputField::EndText, "bad".to_string())
            .unwrap();
        let view = session.view();
        assert_eq!(
            view.validation_errors,
            vec![
                "selection.profileRequired",
                "selection.regionRequired",
                "selection.logGroupRequired",
                "validation.startNonexistentLocalTime",
                "validation.endFormat",
            ]
        );
        assert!(!view.can_fetch);
    }

    #[test]
    fn the_order_reason_appears_only_when_both_inputs_hold_instants() {
        let mut session = new_york_session();
        fill_valid(&mut session);
        session
            .update_input(InputField::StartText, "2024-01-02 03:05:05".to_string())
            .unwrap();
        session
            .update_input(InputField::EndText, "2024-01-02 03:04:05".to_string())
            .unwrap();
        assert_eq!(
            session.view().validation_errors,
            vec!["validation.rangeOrder"]
        );
        session
            .update_input(InputField::EndText, "2024-03-10 02:30:00".to_string())
            .unwrap();
        assert_eq!(
            session.view().validation_errors,
            vec!["validation.endNonexistentLocalTime"]
        );
    }

    #[test]
    fn valid_new_york_input_can_fetch_with_the_instants_as_range() {
        let mut session = new_york_session();
        fill_valid(&mut session);
        // 2024-01-02 03:04:05 EST is 08:04:05 UTC.
        assert!(session.can_fetch());
        let validated = session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(
            validated.range.start_instant(),
            1_704_164_645_000 + 5 * 3_600_000
        );
    }

    #[test]
    fn fetch_is_disabled_while_fetching_without_a_new_reason() {
        let mut session = new_york_session();
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let view = session.view();
        assert!(!view.can_fetch);
        assert!(view.validation_errors.is_empty(), "the status line says it");
    }

    #[test]
    fn fetch_is_disabled_while_a_change_awaits_confirmation_without_a_new_reason() {
        let mut session = done_session(5);
        session.select_region("eu-west-1".to_string()).unwrap();
        let view = session.view();
        assert!(view.pending_change.is_some());
        assert!(!view.can_fetch);
        assert!(view.validation_errors.is_empty(), "the dialog says it");
    }

    // ---- U4: the time zone switch and the display rows (BR1.4, R-08) ----

    fn tokyo_session() -> AppSession {
        AppSession::with_catalog_and_time_zones(
            catalog(),
            TimeZoneContext::new(chrono_tz::Asia::Tokyo),
        )
    }

    fn held_row(timestamp: i64, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: format!("m{sequence}"),
            log_stream_name: "stream-a".to_string(),
            sequence,
        }
    }

    #[test]
    fn the_session_starts_with_local_time() {
        let session = tokyo_session();
        assert_eq!(session.time_zone(), TimeZoneChoice::Local);
        assert_eq!(session.view().time_zone, TimeZoneChoice::Local);
        assert_eq!(AppSession::new().time_zone(), TimeZoneChoice::Local);
    }

    #[test]
    fn switching_rewrites_the_inputs_and_keeps_the_fetch_range() {
        let mut session = tokyo_session();
        fill_valid(&mut session);
        session
            .update_input(InputField::StartText, "2024-03-01 10:00:00".to_string())
            .unwrap();
        session
            .update_input(InputField::EndText, "2024-03-01 11:00:00".to_string())
            .unwrap();
        let range = session.validation.clone().unwrap().range;
        session.select_time_zone(TimeZoneChoice::Utc);
        let view = session.view();
        assert_eq!(view.time_zone, TimeZoneChoice::Utc);
        assert_eq!(view.start_input.text(), "2024-03-01 01:00:00");
        assert_eq!(view.end_input.text(), "2024-03-01 02:00:00");
        assert!(view.can_fetch);
        let validated = session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(validated.range, range, "the zone does not change the range");
    }

    #[test]
    fn switching_to_utc_clears_the_nonexistent_local_time_reason() {
        let mut session = new_york_session();
        fill_valid(&mut session);
        session
            .update_input(InputField::StartText, "2024-03-10 02:30:00".to_string())
            .unwrap();
        session
            .update_input(InputField::EndText, "2024-03-10 04:00:00".to_string())
            .unwrap();
        assert_eq!(
            session.view().validation_errors,
            vec!["validation.startNonexistentLocalTime"]
        );
        session.select_time_zone(TimeZoneChoice::Utc);
        let view = session.view();
        assert_eq!(view.start_input.text(), "2024-03-10 02:30:00");
        assert!(view.validation_errors.is_empty());
        assert!(view.can_fetch);
    }

    #[test]
    fn switching_works_while_fetching_and_keeps_the_phase_but_typing_does_not() {
        let mut session = tokyo_session();
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        session.select_time_zone(TimeZoneChoice::Utc);
        assert_eq!(session.phase(), Phase::Fetching);
        assert_eq!(session.time_zone(), TimeZoneChoice::Utc);
        // 2024-01-02 03:04:05 in Tokyo is 2024-01-01 18:04:05 UTC.
        assert_eq!(session.start_input().text(), "2024-01-01 18:04:05");
        assert_eq!(
            session.update_input(InputField::StartText, "2024-01-01 00:00:00".to_string()),
            Err(SessionError::Busy)
        );
        assert_eq!(session.start_input().text(), "2024-01-01 18:04:05");
    }

    #[test]
    fn switching_works_while_a_change_awaits_confirmation() {
        let mut session = done_session(5);
        session.select_region("eu-west-1".to_string()).unwrap();
        assert!(session.view().pending_change.is_some());
        session.select_time_zone(TimeZoneChoice::Utc);
        let view = session.view();
        assert_eq!(view.time_zone, TimeZoneChoice::Utc);
        assert_eq!(view.phase, Phase::Done);
        assert!(view.pending_change.is_some(), "the dialog stays open");
    }

    #[test]
    fn display_rows_carry_times_in_the_chosen_zone() {
        let mut session = new_york_session();
        // 2024-01-02 03:04:05.007 UTC: 22:04:05.007 the day before in New York.
        let window = RowWindow {
            offset: 4,
            rows: vec![held_row(1_704_164_645_007, 0), held_row(i64::MAX, 1)],
            total_count: 9,
            timeline_version: 3,
            filtered: false,
            all_count: 9,
            result_version: None,
        };
        let local = session.display_rows(window.clone());
        assert_eq!(local.offset, 4);
        assert_eq!(local.total_count, 9);
        assert_eq!(local.timeline_version, 3);
        assert_eq!(local.rows[0].display_time, "2024-01-01 22:04:05.007");
        assert_eq!(local.rows[1].display_time, i64::MAX.to_string());
        assert_eq!(local.rows[0].event, window.rows[0]);

        session.select_time_zone(TimeZoneChoice::Utc);
        let utc = session.display_rows(window);
        assert_eq!(utc.rows[0].display_time, "2024-01-02 03:04:05.007");
        assert_eq!(utc.timeline_version, 3, "switching keeps the version");
        let json = serde_json::to_value(&utc).unwrap();
        assert_eq!(json["rows"][0]["displayTime"], "2024-01-02 03:04:05.007");
        assert_eq!(json["rows"][0]["timestamp"], 1_704_164_645_007_i64);
        assert_eq!(json["rows"][0]["logStreamName"], "stream-a");
        assert_eq!(json["totalCount"], 9);
    }

    #[test]
    fn selecting_the_current_zone_changes_nothing() {
        let mut session = new_york_session();
        // The later 01:30 of 2024-11-03, typed in UTC, shown in New York.
        session.select_time_zone(TimeZoneChoice::Utc);
        session
            .update_input(InputField::StartText, "2024-11-03 06:30:00".to_string())
            .unwrap();
        session.select_time_zone(TimeZoneChoice::Local);
        assert_eq!(session.start_input().text(), "2024-11-03 01:30:00");
        session.select_time_zone(TimeZoneChoice::Local);
        // The screen re-sends the unchanged text: the instant is kept (R-06).
        session
            .update_input(InputField::StartText, "2024-11-03 01:30:00".to_string())
            .unwrap();
        session.select_time_zone(TimeZoneChoice::Utc);
        assert_eq!(session.start_input().text(), "2024-11-03 06:30:00");
    }

    // ---- U5: the log filter (BR1.1, BR1.4, BR3.3, BR3.6) ----

    use crate::filter::{FilterStatus, FilterSummary};

    fn summary(matched: u64, all: u64, status: FilterStatus, version: u64) -> FilterState {
        FilterState {
            result_version: version,
            summary: Some(FilterSummary {
                filter_id: 1,
                matched_count: matched,
                all_count: all,
                status,
                result_version: version,
            }),
        }
    }

    #[test]
    fn the_log_filter_is_trimmed_and_only_a_changed_text_is_handed_on() {
        let mut session = AppSession::with_catalog(catalog());
        assert_eq!(
            session.update_log_filter("  error ".to_string()),
            Ok(Some("error".to_string()))
        );
        assert_eq!(session.log_filter(), "  error ", "kept as typed");
        assert_eq!(session.view().log_filter, "  error ");
        assert_eq!(session.update_log_filter("error".to_string()), Ok(None));
        assert_eq!(session.log_filter(), "error");
        assert_eq!(
            session.update_log_filter("   ".to_string()),
            Ok(Some(String::new())),
            "an empty text clears the filter"
        );
        assert_eq!(session.update_log_filter(String::new()), Ok(None));
        assert_eq!(
            session.phase(),
            Phase::Idle,
            "filtering never moves the phase"
        );
    }

    #[test]
    fn the_log_filter_is_accepted_while_fetching_and_kept_across_fetches() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session.update_log_filter("ERROR".to_string()).unwrap();
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(session.log_filter(), "ERROR", "a new fetch keeps the text");
        assert_eq!(
            session.update_log_filter("timeout".to_string()),
            Ok(Some("timeout".to_string())),
            "accepted while fetching"
        );
        let job = finished_job(&mut session, 3, None);
        session.finish_fetch(&job, 2);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert_eq!(session.view().log_filter, "timeout");
    }

    #[test]
    fn the_log_filter_is_refused_while_a_connection_change_awaits_confirmation() {
        let mut session = done_session(5);
        session.update_log_filter("error".to_string()).unwrap();
        session.select_region("eu-west-1".to_string()).unwrap();
        assert_eq!(
            session.update_log_filter("other".to_string()),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(session.log_filter(), "error", "unchanged");
        session.confirm_connection_change().unwrap();
        assert_eq!(
            session.log_filter(),
            "error",
            "a connection change keeps it"
        );
        assert_eq!(session.update_log_filter("error".to_string()), Ok(None));
    }

    #[test]
    fn the_view_and_both_light_messages_carry_the_filter_summary() {
        let mut session = AppSession::with_catalog(catalog());
        let view = session.view();
        assert_eq!(view.filter_summary, None);
        assert_eq!(view.filter_result_version, 0);

        session.set_filter_state(summary(2, 10, FilterStatus::Filtering, 7));
        let view = session.view();
        assert_eq!(
            view.filter_summary.as_ref().map(|s| s.matched_count),
            Some(2)
        );
        assert_eq!(view.filter_result_version, 7);
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["filterSummary"]["matchedCount"], 2);
        assert_eq!(json["filterSummary"]["allCount"], 10);
        assert_eq!(json["filterSummary"]["status"], "Filtering");
        assert_eq!(json["filterSummary"]["resultVersion"], 7);
        assert_eq!(json["filterResultVersion"], 7);
        assert_eq!(json["logFilter"], "");

        let update = session.filter_update();
        assert_eq!(update.filter_result_version, 7);
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["filterSummary"]["filterId"], 1);
        assert!(json.get("profiles").is_none(), "no full session view");

        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let validated = session.validation.clone().unwrap();
        let job = FetchJob::start(validated.range);
        session.on_job_started(&job, 2);
        session.on_batch(job.job_id, batch(10, 3));
        session.set_filter_state(summary(4, 10, FilterStatus::Ready, 9));
        let progress = session.progress_update().unwrap();
        assert_eq!(progress.filter_result_version, 9);
        assert_eq!(
            progress.filter_summary.map(|s| (s.matched_count, s.status)),
            Some((4, FilterStatus::Ready))
        );
        assert_eq!(session.filter_state().result_version, 9);
    }

    #[test]
    fn display_rows_keep_the_filtered_window_fields() {
        let session = AppSession::with_catalog(catalog());
        let window = RowWindow {
            offset: 1,
            rows: vec![held_row(1_704_164_645_007, 0)],
            total_count: 2,
            timeline_version: 5,
            filtered: true,
            all_count: 40,
            result_version: Some(8),
        };
        let shown = session.display_rows(window);
        assert!(shown.filtered);
        assert_eq!((shown.total_count, shown.all_count), (2, 40));
        assert_eq!(shown.result_version, Some(8));
        assert_eq!(shown.rows[0].display_time, "2024-01-02 03:04:05.007");
        let json = serde_json::to_value(&shown).unwrap();
        assert_eq!(json["filtered"], true);
        assert_eq!(json["allCount"], 40);
        assert_eq!(json["resultVersion"], 8);
    }

    // ---- U6: disk cache settings and notices ---------------------------

    use crate::cache::plan::{CacheKey, CacheOutcome, CoveredRange};
    use crate::cache::settings::SettingsLoad;

    fn location(dir: &tempfile::TempDir) -> CacheLocation {
        CacheLocation {
            settings_path: dir.path().join("config").join("settings.json"),
            cache_directory: dir.path().join("cache"),
        }
    }

    fn enabled_session(dir: &tempfile::TempDir) -> AppSession {
        let location = location(dir);
        crate::cache::settings::save_settings(&location.settings_path, true).unwrap();
        AppSession::with_catalog(catalog()).with_cache_location(location)
    }

    fn some_cache_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
        let cache = LogCache::new(location(dir).cache_directory);
        let key = CacheKey::new(dev(), "ap-northeast-1".into(), "/g".into());
        cache
            .write(
                &key,
                CoveredRange {
                    start_ms: 0,
                    end_ms: 9,
                },
                Vec::new(),
            )
            .unwrap();
        cache.entry_path(&key)
    }

    #[test]
    fn a_session_without_a_location_is_disabled_and_has_no_disk() {
        let mut session = AppSession::with_catalog(catalog());
        assert!(!session.cache_enabled());
        assert_eq!(session.fetch_cache(), None);
        assert_eq!(session.cache_location(), None);
        let view = session.view();
        assert!(!view.cache_enabled);
        assert_eq!(view.cache_directory, None);
        assert_eq!(view.settings_dialog, SettingsDialog::Closed);
        assert!(view.can_open_settings);
        session.open_settings().unwrap();
        session.save_settings(true).unwrap();
        assert!(!session.cache_enabled(), "nowhere to save it");
        assert_eq!(
            session.view().settings_notice,
            Some(SettingsNotice::SaveFailed)
        );
        assert_eq!(session.settings_dialog(), SettingsDialog::Open);
    }

    #[test]
    fn a_saved_enabled_setting_starts_enabled_with_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let session = enabled_session(&dir);
        assert!(session.cache_enabled());
        assert_eq!(
            session
                .fetch_cache()
                .map(|cache| cache.directory().to_path_buf()),
            Some(location(&dir).cache_directory)
        );
        let view = session.view();
        assert!(view.cache_enabled);
        assert_eq!(
            view.cache_directory,
            Some(location(&dir).cache_directory.display().to_string())
        );
        let fresh = tempfile::tempdir().unwrap();
        let first_launch =
            AppSession::with_catalog(catalog()).with_cache_location(location(&fresh));
        assert!(!first_launch.cache_enabled(), "no settings file: disabled");
        assert_eq!(first_launch.fetch_cache(), None);
    }

    #[test]
    fn the_dialog_cannot_open_while_fetching_or_while_a_change_awaits_confirmation() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert!(!session.view().can_open_settings);
        assert_eq!(session.open_settings(), Err(SessionError::Busy));

        let mut session = done_session(5);
        session
            .select_profile(ProfileSelector::Named("prod".to_string()))
            .unwrap();
        assert!(!session.view().can_open_settings);
        assert_eq!(
            session.open_settings(),
            Err(SessionError::ConfirmationPending)
        );
        assert_eq!(session.settings_dialog(), SettingsDialog::Closed);
    }

    #[test]
    fn while_the_dialog_is_open_fetching_and_changes_are_refused() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        let generation = session.connection_generation();
        session.open_settings().unwrap();
        let view = session.view();
        assert!(!view.can_fetch && !view.can_change_connection && !view.can_reload);
        let refused = SessionError::ConfirmationPending;
        assert_eq!(session.begin_fetch(generation), Err(refused));
        assert_eq!(
            session.update_input(InputField::StartText, "2024-01-02 03:00:00".into()),
            Err(refused)
        );
        assert_eq!(session.select_profile(dev()), Err(refused));
        assert_eq!(session.select_region("us-east-1".into()), Err(refused));
        assert_eq!(session.reload_log_groups(generation), Err(refused));
        assert_eq!(
            session.select_log_group("/aws/lambda/MyFunction".into(), generation),
            Err(refused)
        );
        assert_eq!(session.update_log_group_filter("x".into()), Err(refused));
        assert_eq!(session.update_log_filter("error".into()), Err(refused));
        assert_eq!(session.log_filter(), "", "not taken");
        session.cancel_settings();
        assert!(session.can_fetch());
        assert_eq!(
            session.update_log_filter("error".into()),
            Ok(Some("error".into()))
        );
    }

    #[test]
    fn saving_enabled_writes_the_setting_and_closes_the_dialog() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = AppSession::with_catalog(catalog()).with_cache_location(location(&dir));
        session.open_settings().unwrap();
        session.save_settings(true).unwrap();
        assert!(session.cache_enabled());
        assert_eq!(session.settings_dialog(), SettingsDialog::Closed);
        assert_eq!(session.view().settings_notice, None);
        assert_eq!(
            crate::cache::settings::load_settings(&location(&dir).settings_path),
            SettingsLoad::Loaded {
                cache_enabled: true
            }
        );
        assert_eq!(
            session.save_settings(false),
            Err(SessionError::SettingsClosed)
        );
    }

    #[test]
    fn saving_disabled_removes_the_cached_files_and_closes_the_dialog() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = enabled_session(&dir);
        let cached = some_cache_file(&dir);
        session.open_settings().unwrap();
        session.save_settings(false).unwrap();
        assert!(!session.cache_enabled());
        assert_eq!(session.fetch_cache(), None);
        assert_eq!(session.settings_dialog(), SettingsDialog::Closed);
        assert!(!cached.exists());
        assert!(
            !crate::cache::settings::load_settings(&location(&dir).settings_path).cache_enabled()
        );
    }

    #[test]
    fn a_failed_removal_keeps_the_dialog_open_and_the_cache_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = enabled_session(&dir);
        // A regular file where the cache folder should be cannot be listed.
        std::fs::write(location(&dir).cache_directory, b"not a folder").unwrap();
        session.open_settings().unwrap();
        session.save_settings(false).unwrap();
        assert!(!session.cache_enabled(), "the setting was written");
        assert_eq!(session.settings_dialog(), SettingsDialog::Open);
        assert_eq!(
            session.view().settings_notice,
            Some(SettingsNotice::ClearFailed)
        );
        session.cancel_settings();
        assert_eq!(session.settings_dialog(), SettingsDialog::Closed);
        assert_eq!(session.view().settings_notice, None);
    }

    #[test]
    fn a_setting_that_cannot_be_written_keeps_the_dialog_and_the_value() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = enabled_session(&dir);
        let config = location(&dir).settings_path.parent().unwrap().to_path_buf();
        std::fs::remove_dir_all(&config).unwrap();
        std::fs::write(&config, b"not a folder").unwrap();
        session.open_settings().unwrap();
        session.save_settings(false).unwrap();
        assert!(session.cache_enabled(), "unchanged");
        assert_eq!(session.settings_dialog(), SettingsDialog::Open);
        assert_eq!(
            session.view().settings_notice,
            Some(SettingsNotice::SaveFailed)
        );
    }

    #[test]
    fn clear_cache_removes_the_files_at_once_and_keeps_the_setting() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = enabled_session(&dir);
        let cached = some_cache_file(&dir);
        assert_eq!(session.clear_cache(), Err(SessionError::SettingsClosed));
        assert!(cached.exists());
        session.open_settings().unwrap();
        session.clear_cache().unwrap();
        assert!(!cached.exists());
        assert!(session.cache_enabled());
        assert_eq!(session.settings_dialog(), SettingsDialog::Open);
        assert_eq!(
            session.view().settings_notice,
            Some(SettingsNotice::Cleared)
        );
        session.cancel_settings();
        assert!(!cached.exists(), "cancel does not bring them back");
    }

    #[test]
    fn saving_shows_only_for_the_running_job_and_the_finish_makes_the_notices() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let mut job = finished_job(&mut session, 3, None);
        session.on_saving(job.job_id + 1000);
        assert!(!session.cache_saving(), "an older job is ignored");
        session.on_saving(job.job_id);
        assert!(session.cache_saving());
        assert!(session.progress_update().unwrap().cache_saving);
        assert!(session.view().cache_saving);
        job.read_failed = true;
        job.cache_outcome = CacheOutcome::SaveFailed;
        session.finish_fetch(&job, 2);
        assert!(!session.cache_saving());
        assert_eq!(
            session.view().cache_notices,
            vec![CacheNotice::ReadFailed, CacheNotice::SaveFailed]
        );
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        assert!(
            session.cache_notices().is_empty(),
            "gone with the next Fetch"
        );
    }

    #[test]
    fn a_hit_is_noticed_and_an_abnormal_end_also_stops_saving() {
        let mut session = AppSession::with_catalog(catalog());
        fill_valid(&mut session);
        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let mut job = finished_job(&mut session, 2, None);
        job.served_from_cache = true;
        job.cache_outcome = CacheOutcome::Hit;
        session.finish_fetch(&job, 2);
        assert_eq!(session.cache_notices(), [CacheNotice::Hit]);

        session
            .begin_fetch(session.connection_generation())
            .unwrap();
        let running = finished_job(&mut session, 0, None);
        session.on_saving(running.job_id);
        assert!(session.cache_saving());
        let failure = ApiFailure::new(FailureKind::Other, &SafeDetailFields::default());
        assert!(session.abort_fetch_with_failure(session.fetch_number(), failure));
        assert!(!session.cache_saving());
        assert!(session.cache_notices().is_empty());
        assert_eq!(session.phase(), Phase::Failed);
    }
}
