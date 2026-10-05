//! Tauri wiring of local-sights (DesktopUi glue, ADR-001).
//!
//! This crate stays thin: every decision lives in `local-sights-core`.
//! Operations arrive as Tauri commands (`get_session`, `update_input`,
//! `start_fetch`, since U2 the connection and log group commands, and since
//! U3 `get_rows`, `find_row_position` and `set_failure_list_open`); state
//! changes are pushed to the screen with the `session-changed` event; while
//! a fetch runs, each listing page and each added page sends only the light
//! `fetch-progress` event (progress, event count, timeline version). Commands that
//! change state return nothing: their result arrives as an event, so a late
//! response never overwrites newer state.
//!
//! Since U3 the screen never holds the fetched events: it reads the rows of
//! its viewport with `get_rows` (U3:BR4.3, BR6.4). The EventTimeline sits
//! behind a `std::sync::Mutex` that the fetch locks for one page at a time,
//! so rows can be read while a fetch runs (NFR3). Lock order is always
//! session, then timeline; the fetch never holds both.
//!
//! Operations that depend on the connection carry the connection generation
//! the screen last saw; an operation for an older connection is dropped
//! without an answer (U3:BR6.7).
//!
//! Diagnostics go to standard error only and contain only safe details:
//! no secret credential and no access key ID is ever logged or shown.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::catalog::files::load_catalog;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::gateway::{DESCRIBE_LOG_GROUPS, GET_LOG_EVENTS};
use local_sights_core::log_groups::listing::{ListingRequest, ListingSink, run_listing};
use local_sights_core::log_groups::{ListingStatus, LogGroup};
use local_sights_core::paging::PageDecision;
use local_sights_core::retry::{AbortHandle, RandomJitter, Retrier, RetryPolicy, abort_pair};
use local_sights_core::session::{
    AppSession, ConnectionEffect, InputField, SessionError, SessionView,
};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::timeline::{EventTimeline, RowWindow};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

/// Event carrying the latest [`SessionView`].
pub const SESSION_CHANGED: &str = "session-changed";
/// Light event carrying only the progress of the running fetch, sent for
/// every listing page and every added page instead of the whole view
/// (review R-05).
pub const FETCH_PROGRESS: &str = "fetch-progress";

/// Most rows one `get_rows` call returns; a viewport needs far fewer.
const MAX_ROWS_PER_REQUEST: usize = 1_000;

/// Shared state of the app.
#[derive(Debug)]
struct AppState {
    session: Arc<Mutex<AppSession>>,
    timeline: Arc<Mutex<EventTimeline>>,
    gateway: Arc<AwsCloudWatchLogsGateway>,
    /// Abort handle of the running fetch with its fetch number from the
    /// session (BR5.5), so a finished fetch only forgets its own handle.
    abort: Arc<Mutex<Option<(u64, AbortHandle)>>>,
}

impl AppState {
    /// Reads the shared AWS config files once at startup (U2:BR1.1-BR1.3).
    fn load() -> Self {
        Self {
            session: Arc::new(Mutex::new(AppSession::with_catalog(load_catalog()))),
            timeline: Arc::default(),
            gateway: Arc::default(),
            abort: Arc::default(),
        }
    }

    /// Asks the running fetch, if any, to stop (window closing, BR5.5).
    fn abort_fetch(&self) {
        if let Some((_, handle)) = lock(&self.abort).as_ref() {
            handle.abort();
        }
    }
}

/// Error returned by a command: a message-catalog key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandError {
    key: String,
}

impl From<SessionError> for CommandError {
    fn from(error: SessionError) -> Self {
        Self {
            key: error.message_key().to_string(),
        }
    }
}

/// Answer of `find_row_position`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RowPosition {
    /// Current position of the event, or `None` when it is not held.
    position: Option<u64>,
    /// Version of the timeline the position belongs to.
    timeline_version: u64,
}

/// Locks a mutex. A poisoned lock still holds consistent data (every
/// update is a single step), so it is recovered, not fatal.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn emit_or_log<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Err(error) = app.emit(event, payload) {
        eprintln!("local-sights: failed to emit {event}: {error}");
    }
}

/// Maps a stale operation (an older connection generation, BR6.7) to a
/// silent success: it is dropped and nothing is answered.
fn drop_if_stale(result: Result<(), CommandError>) -> Result<(), CommandError> {
    match result {
        Err(error) if error.key == SessionError::StaleGeneration.message_key() => Ok(()),
        other => other,
    }
}

/// Returns the current session state.
#[tauri::command]
fn get_session(state: State<'_, AppState>) -> SessionView {
    lock(&state.session).view()
}

/// Changes a time; refused while fetching (U1:BR1.4).
#[tauri::command]
fn update_input(
    field: InputField,
    value: String,
    state: State<'_, AppState>,
) -> Result<SessionView, CommandError> {
    let mut session = lock(&state.session);
    session.update_input(field, value)?;
    Ok(session.view())
}

/// Returns at most `limit` rows of the timeline from `offset` (U3:BR4.3).
#[tauri::command]
fn get_rows(offset: u64, limit: u64, state: State<'_, AppState>) -> RowWindow {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let limit = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(MAX_ROWS_PER_REQUEST);
    lock(&state.timeline).rows(offset, limit)
}

/// Returns the current position of one event (U3:BR4.4).
#[tauri::command]
fn find_row_position(
    log_stream_name: String,
    sequence: u64,
    state: State<'_, AppState>,
) -> RowPosition {
    let timeline = lock(&state.timeline);
    RowPosition {
        position: timeline
            .position_of(&log_stream_name, sequence)
            .map(|position| position as u64),
        timeline_version: timeline.version(),
    }
}

/// Opens or closes the list of failed streams (U3:BR6.3).
#[tauri::command]
fn set_failure_list_open(
    open: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let view = {
        let mut session = lock(&state.session);
        session.set_failure_list_open(open)?;
        session.view()
    };
    emit_or_log(&app, SESSION_CHANGED, view);
    Ok(())
}

/// Starts fetching the selected log group. The command returns nothing:
/// the new state (Fetching), the progress and the result all arrive through
/// `session-changed`, in order. An operation for an older connection is
/// dropped (BR6.7).
#[tauri::command]
fn start_fetch(
    generation: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    drop_if_stale(begin_and_spawn_fetch(generation, &app, &state))
}

fn begin_and_spawn_fetch(
    generation: u64,
    app: &AppHandle,
    state: &AppState,
) -> Result<(), CommandError> {
    let (validated, fetch_number, view) = {
        let mut session = lock(&state.session);
        let validated = session.begin_fetch(generation)?;
        (validated, session.fetch_number(), session.view())
    };
    emit_or_log(app, SESSION_CHANGED, view);

    let (handle, signal) = abort_pair();
    *lock(&state.abort) = Some((fetch_number, handle));
    let session = Arc::clone(&state.session);
    let timeline = Arc::clone(&state.timeline);
    let gateway = Arc::clone(&state.gateway);
    let mut sink = TauriSink {
        app: app.clone(),
        session: Arc::clone(&session),
    };
    let task = tauri::async_runtime::spawn(async move {
        let policy = RetryPolicy::default();
        let jitter = RandomJitter;
        let retrier = Retrier::new(&policy, &jitter, &signal);
        run_fetch(
            gateway.as_ref(),
            &validated,
            timeline.as_ref(),
            &retrier,
            &mut sink,
        )
        .await;
    });
    let app = app.clone();
    let abort = Arc::clone(&state.abort);
    tauri::async_runtime::spawn(async move {
        let ended_normally = task.await.is_ok();
        {
            let mut running = lock(&abort);
            if running
                .as_ref()
                .is_some_and(|(number, _)| *number == fetch_number)
            {
                running.take();
            }
        }
        recover_if_still_fetching(&app, &session, fetch_number, ended_normally);
    });
    Ok(())
}

/// Runs after the fetch task ends. If it ended (normally or not) without
/// reporting a result, the session would stay in Fetching forever; move it
/// to Failed (kind Other) and tell the screen. Only the fetch numbered
/// `fetch_number` is touched: a newer fetch that started meanwhile is left
/// alone (review R-03). The panic message is not logged: only a safe
/// detail is.
fn recover_if_still_fetching(
    app: &AppHandle,
    session: &Mutex<AppSession>,
    fetch_number: u64,
    ended_normally: bool,
) {
    let failure = ApiFailure::new(
        FailureKind::Other,
        &SafeDetailFields {
            api_name: Some(GET_LOG_EVENTS.to_string()),
            ..SafeDetailFields::default()
        },
    );
    let view = {
        let mut session = lock(session);
        if !session.abort_fetch_with_failure(fetch_number, failure) {
            return;
        }
        session.view()
    };
    let how = if ended_normally {
        "without a result"
    } else {
        "abnormally"
    };
    eprintln!("local-sights: the fetch task ended {how}; the fetch was marked as failed");
    emit_or_log(app, SESSION_CHANGED, view);
}

/// Runs a session operation that may change the connection, then pushes the
/// new state and starts a log group listing when needed (U2:BR2.3, BR3.5).
///
/// When the operation discards the shown logs (U2:BR2.4), the EventTimeline
/// is cleared while the session lock is still held, and its new version is
/// recorded in the session (U3:BR4.5), before `session-changed` is emitted.
/// A new fetch can only start under the same session lock, so a late clear
/// can never wipe the logs of a newer fetch. During a fetch the operations
/// that discard logs are refused.
fn change_connection(
    app: &AppHandle,
    state: &AppState,
    operation: impl FnOnce(&mut AppSession) -> Result<ConnectionEffect, SessionError>,
) -> Result<(), CommandError> {
    let (effect, view) = {
        let mut session = lock(&state.session);
        let effect = operation(&mut session)?;
        if effect.logs_cleared {
            let version = lock(&state.timeline).clear();
            session.set_timeline_version(version);
        }
        (effect, session.view())
    };
    emit_or_log(app, SESSION_CHANGED, view);
    if let Some(request) = effect.listing {
        start_listing(app, state, request);
    }
    Ok(())
}

/// Lists log groups in the background; each page updates the session and
/// pushes `session-changed`. If the task ends without finishing a listing
/// that is still current, the listing is marked Partial (kind Other).
fn start_listing(app: &AppHandle, state: &AppState, request: ListingRequest) {
    let session = Arc::clone(&state.session);
    let gateway = Arc::clone(&state.gateway);
    let mut sink = TauriListingSink {
        app: app.clone(),
        session: Arc::clone(&session),
    };
    let task_request = request.clone();
    let task = tauri::async_runtime::spawn(async move {
        run_listing(gateway.as_ref(), &task_request, &mut sink).await;
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let ended_normally = task.await.is_ok();
        let view = {
            let mut session = lock(&session);
            let still_loading = session.is_current_listing(&request.listing_id)
                && session.listing().map(|l| l.status()) == Some(ListingStatus::Loading);
            if !still_loading {
                return;
            }
            let failure = ApiFailure::new(
                FailureKind::Other,
                &SafeDetailFields {
                    api_name: Some(DESCRIBE_LOG_GROUPS.to_string()),
                    ..SafeDetailFields::default()
                },
            );
            session.apply_listing_failure(&request.listing_id, failure);
            session.view()
        };
        let how = if ended_normally {
            "without a result"
        } else {
            "abnormally"
        };
        eprintln!("local-sights: the log group listing ended {how}; it was marked as partial");
        emit_or_log(&app, SESSION_CHANGED, view);
    });
}

/// Chooses a profile (U2:BR2.2, BR2.5).
#[tauri::command]
fn select_profile(
    profile: ProfileSelector,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| session.select_profile(profile))
}

/// Chooses a region (U2:BR2.3, BR2.5).
#[tauri::command]
fn select_region(
    region: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| session.select_region(region))
}

/// Applies the connection change awaiting confirmation (U2:BR2.5).
#[tauri::command]
fn confirm_connection_change(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, AppSession::confirm_connection_change)
}

/// Drops the connection change awaiting confirmation (U2:BR2.5).
#[tauri::command]
fn cancel_connection_change(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        session.cancel_connection_change()?;
        Ok(ConnectionEffect::default())
    })
}

/// Lists the log groups of the current connection again (U2:BR3.5); an
/// operation for an older connection is dropped (U3:BR6.7).
#[tauri::command]
fn reload_log_groups(
    generation: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    drop_if_stale(change_connection(&app, &state, |session| {
        let listing = session.reload_log_groups(generation)?;
        Ok(ConnectionEffect {
            listing: Some(listing),
            ..ConnectionEffect::default()
        })
    }))
}

/// Changes the log group filter (U2:BR3.7).
#[tauri::command]
fn update_log_group_filter(
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        session.update_log_group_filter(text)?;
        Ok(ConnectionEffect::default())
    })
}

/// Selects one log group (U2:BR3.8); an operation for an older connection
/// is dropped (U3:BR6.7).
#[tauri::command]
fn select_log_group(
    name: String,
    generation: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    drop_if_stale(change_connection(&app, &state, |session| {
        session.select_log_group(name, generation)?;
        Ok(ConnectionEffect::default())
    }))
}

/// Applies listing pages to the session and pushes the new state.
struct TauriListingSink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
}

impl ListingSink for TauriListingSink {
    fn is_current(&self, listing_id: &str) -> bool {
        lock(&self.session).is_current_listing(listing_id)
    }

    fn on_page(
        &mut self,
        listing_id: &str,
        groups: Vec<LogGroup>,
        next_token: Option<&str>,
    ) -> Option<PageDecision> {
        let (decision, view) = {
            let mut session = lock(&self.session);
            let decision = session.apply_listing_page(listing_id, groups, next_token)?;
            (decision, session.view())
        };
        emit_or_log(&self.app, SESSION_CHANGED, view);
        Some(decision)
    }

    fn on_failure(&mut self, listing_id: &str, failure: ApiFailure) -> bool {
        // Safe detail only: built from allow-listed fields and redacted.
        let detail = failure.safe_detail().to_string();
        let view = {
            let mut session = lock(&self.session);
            if !session.apply_listing_failure(listing_id, failure) {
                return false;
            }
            session.view()
        };
        eprintln!("local-sights: log group listing failed: {detail}");
        emit_or_log(&self.app, SESSION_CHANGED, view);
        true
    }
}

/// Forwards fetch progress to AppSession and pushes the new state: the whole
/// view at the start, the plan, each finished stream and the end, and only
/// the light `fetch-progress` message for each listing page and each added
/// page (review R-05). The timeline is never locked here; the coordinator
/// adds each page itself.
struct TauriSink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
}

impl TauriSink {
    fn update(&self, change: impl FnOnce(&mut AppSession)) {
        let view = {
            let mut session = lock(&self.session);
            change(&mut session);
            session.view()
        };
        emit_or_log(&self.app, SESSION_CHANGED, view);
    }

    fn update_progress(&self, change: impl FnOnce(&mut AppSession)) {
        let update = {
            let mut session = lock(&self.session);
            change(&mut session);
            session.progress_update()
        };
        if let Some(update) = update {
            emit_or_log(&self.app, FETCH_PROGRESS, update);
        }
    }
}

impl FetchSink for TauriSink {
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64) {
        self.update(|session| session.on_job_started(job, timeline_version));
    }

    fn on_listing_progress(&mut self, job_id: u64, progress: ListingProgress) {
        self.update_progress(|session| session.on_listing_progress(job_id, progress));
    }

    fn on_planned(&mut self, job: &FetchJob) {
        self.update(|session| session.on_planned(job));
    }

    fn on_batch(&mut self, job_id: u64, progress: BatchProgress) {
        self.update_progress(|session| session.on_batch(job_id, progress));
    }

    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        if let Some(failure) = &outcome.failure {
            // Safe detail only: built from allow-listed fields and redacted.
            eprintln!(
                "local-sights: fetch {} stream failed: {}",
                job.job_id,
                failure.safe_detail()
            );
        }
        self.update(|session| session.on_stream_finished(job));
    }

    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64) {
        if job.status == JobStatus::Failed {
            // Safe detail only: built from allow-listed fields and redacted.
            let detail = job.failure().map_or("", |failure| failure.safe_detail());
            eprintln!("local-sights: fetch {} failed: {detail}", job.job_id);
        }
        self.update(|session| session.finish_fetch(job, timeline_version));
    }
}

/// Starts the desktop app.
///
/// # Errors
/// Returns the Tauri error when the app cannot be built or run.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .manage(AppState::load())
        .on_window_event(|window, event| {
            // Closing the window stops a running fetch (BR5.5); the
            // confirmation dialog comes with U7.
            if matches!(
                event,
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
            ) {
                window.state::<AppState>().abort_fetch();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_session,
            update_input,
            start_fetch,
            select_profile,
            select_region,
            confirm_connection_change,
            cancel_connection_change,
            reload_log_groups,
            update_log_group_filter,
            select_log_group,
            get_rows,
            find_row_position,
            set_failure_list_open
        ])
        .run(tauri::generate_context!())
}
