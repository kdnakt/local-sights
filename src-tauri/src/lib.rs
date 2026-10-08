//! Tauri wiring of local-sights (DesktopUi glue, ADR-001).
//!
//! This crate stays thin: every decision lives in `local-sights-core`.
//! Operations arrive as Tauri commands (`get_session`, `update_input`,
//! `start_fetch`, since U2 the connection and log group commands, and since
//! U3 `get_rows`, `find_row_position` and `set_failure_list_open`, and since
//! U4 `select_time_zone`, and since U5 `set_log_filter`); state
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
//! Since U4 the local time zone is read from the OS once at startup and
//! given to AppSession (U4:BR1.1, BR3.4). `get_rows` returns each row with
//! its time already written in the chosen zone by the core (review R-08);
//! the screen converts nothing.
//!
//! Since U5 the held events sit in a `LogView` (EventTimeline and the log
//! filter behind one `Mutex`, U5 review R-02): the fetch reaches it as its
//! `TimelineStore`, so adding a page also judges it against the filter and
//! a discard also empties the result (U5:BR2.1, BR2.2, BR2.5). The command
//! `set_log_filter` changes the filter text (U5:BR1.1, BR1.4) and starts a
//! scan of the held events on a blocking thread, which locks the LogView
//! for one chunk at a time and stops as soon as its ticket is stale
//! (U5:BR1.5, BR2.4). The scan pushes the light `filter-progress` event;
//! `session-changed` and `fetch-progress` carry the filter summary too
//! (U5:BR3.6). `get_rows` and `find_row_position` read with the filter in
//! mind (U5:BR3.1, BR3.2). Lock order is session, then LogView; the filter
//! summary is copied into the session while both are held. Filtering calls
//! no AWS API (U5 FR6.2).
//!
//! Since U6 the app decides at startup where the settings file (the OS app
//! settings folder) and the disk cache (a folder in the OS cache folder)
//! live and gives both to AppSession (U6:BR1.6). A fetch runs through
//! `run_fetch_with_cache` with the cache AppSession hands out, `None` when
//! the cache is disabled (U6:BR1.5); `on_saving` sends the light
//! `fetch-progress` event so the status line can say "saving to the cache"
//! (U6:BR3.7). The settings dialog commands are `open_settings`,
//! `cancel_settings`, `save_settings` and `clear_cache`; each pushes
//! `session-changed` (U6:BR5.1). The filter scan asks the core whether to
//! report its progress (`filter::should_report_progress`, U5 review R-03).
//!
//! Since U7 the screen asks the positions of its expanded, selected and
//! top rows in one call, `row_positions`, which locks the LogView once and
//! also answers the discard generation (U7:BR1.6, BR1.7, review R-11); the
//! generation is copied into the session whenever the filter summary is.
//! Closing the window (`CloseRequested`) or ending the app (`ExitRequested`,
//! Cmd+Q and the Dock menu) while fetching or writing the cache is stopped
//! and AppSession's closeConfirmation becomes Pending (U7:BR3.1); the
//! screen answers with `cancel_close` ([Keep fetching]) or `confirm_close`
//! ([Close]: abort the fetch, set the "may exit" flag and exit, BR3.2). The
//! app is therefore built first and run with a callback that sees
//! `RunEvent::ExitRequested`. Closing no longer aborts the fetch by itself;
//! only `Destroyed` still aborts, as a safety net. The settings commands are
//! async and do their file work on a blocking thread without the session
//! lock (U6 review R-01).
//!
//! Diagnostics go to standard error only and contain only safe details:
//! no secret credential and no access key ID is ever logged or shown. No
//! log file is ever written (U7:BR4.5, NFR16).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::catalog::files::load_catalog;
use local_sights_core::coordinator::{
    BatchProgress, FetchJob, FetchSink, JobStatus, run_fetch_with_cache,
};
use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::filter::{FilterChange, ScanStep, ScanTicket, should_report_progress};
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::gateway::{DESCRIBE_LOG_GROUPS, GET_LOG_EVENTS};
use local_sights_core::log_groups::listing::{ListingRequest, ListingSink, run_listing};
use local_sights_core::log_groups::{ListingStatus, LogGroup};
use local_sights_core::log_view::{LogView, RowKey, RowPositions};
use local_sights_core::paging::PageDecision;
use local_sights_core::retry::{AbortHandle, RandomJitter, Retrier, RetryPolicy, abort_pair};
use local_sights_core::session::{
    AppSession, CacheLocation, CloseDecision, ConnectionEffect, DisplayRowWindow, InputField,
    SessionError, SessionView, SettingsWork,
};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::time_zone::{TimeZoneChoice, TimeZoneContext};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

/// Event carrying the latest [`SessionView`].
pub const SESSION_CHANGED: &str = "session-changed";
/// Light event carrying only the progress of the running fetch, sent for
/// every listing page and every added page instead of the whole view
/// (review R-05).
pub const FETCH_PROGRESS: &str = "fetch-progress";
/// Light event carrying only the log filter summary, sent while the scan
/// runs and when it becomes Ready (U5:BR3.6).
pub const FILTER_PROGRESS: &str = "filter-progress";

/// Held events the filter scan judges per lock (U5:BR1.5): small enough
/// that a viewport read waits for at most one chunk.
const SCAN_CHUNK_ROWS: usize = 4_096;

/// Name of the settings file in the OS app settings folder (U6:BR1.1).
const SETTINGS_FILE_NAME: &str = "settings.json";
/// Name of the disk cache folder inside the OS cache folder of the app
/// (U6:BR1.6); a folder of its own, so nothing else shares it.
const CACHE_FOLDER_NAME: &str = "log-cache";

/// Most rows one `get_rows` call returns; a viewport needs far fewer.
const MAX_ROWS_PER_REQUEST: usize = 1_000;
/// Most keys one `row_positions` call answers: the expanded rows, the
/// selected row and the top row (U7:BR1.7).
const MAX_KEYS_PER_REQUEST: usize = 10_000;

/// Shared state of the app.
#[derive(Debug)]
struct AppState {
    session: Arc<Mutex<AppSession>>,
    /// The held events and the log filter (U5 review R-02).
    log_view: Arc<Mutex<LogView>>,
    gateway: Arc<AwsCloudWatchLogsGateway>,
    /// Abort handle of the running fetch with its fetch number from the
    /// session (BR5.5), so a finished fetch only forgets its own handle.
    abort: Arc<Mutex<Option<(u64, AbortHandle)>>>,
    /// Set by [Close] (U7:BR3.2): the exit and window close that follow are
    /// not stopped again. Read without the session lock.
    exit_allowed: AtomicBool,
}

impl AppState {
    /// Reads the shared AWS config files (U2:BR1.1-BR1.3) and the OS time
    /// zone (U4:BR1.1) once at startup. When the OS zone cannot be used,
    /// Local falls back to UTC and only the zone name is logged. Since U6 it
    /// also takes the cache location and reads the cache setting from it;
    /// without a location the cache stays disabled (U6:BR1.6).
    fn load(cache_location: Option<CacheLocation>) -> Self {
        let time_zones = TimeZoneContext::detect();
        if let Some(fallback) = &time_zones.fallback {
            eprintln!("local-sights: {}", fallback.diagnostic());
        }
        let mut session =
            AppSession::with_catalog_and_time_zones(load_catalog(), time_zones.context);
        if let Some(location) = cache_location {
            session = session.with_cache_location(location);
        }
        Self {
            session: Arc::new(Mutex::new(session)),
            log_view: Arc::default(),
            gateway: Arc::default(),
            abort: Arc::default(),
            exit_allowed: AtomicBool::new(false),
        }
    }

    /// Asks the running fetch, if any, to stop ([Close] or the window being
    /// destroyed, BR5.5, U7:BR3.2).
    fn abort_fetch(&self) {
        if let Some((_, handle)) = lock(&self.abort).as_ref() {
            handle.abort();
        }
    }
}

/// U6:BR1.6: the settings file in the OS app settings folder and the cache
/// folder inside the OS cache folder (on macOS `~/Library/Application
/// Support/<id>/` and `~/Library/Caches/<id>/`). `None`, with a diagnostic,
/// when the OS gives no folder; the cache then stays disabled.
fn cache_location(app: &AppHandle) -> Option<CacheLocation> {
    let paths = app.path();
    match (paths.app_config_dir(), paths.app_cache_dir()) {
        (Ok(config), Ok(cache)) => Some(CacheLocation {
            settings_path: config.join(SETTINGS_FILE_NAME),
            cache_directory: cache.join(CACHE_FOLDER_NAME),
        }),
        _ => {
            eprintln!(
                "local-sights: the OS folders could not be found; the disk cache stays disabled"
            );
            None
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

/// Copies the log filter summary (U5:BR3.6) and the discard generation
/// (U7:BR1.6, review R-11) into the session. Called with the session
/// locked; locks the LogView second (the lock order).
fn sync_filter(session: &mut AppSession, log_view: &Mutex<LogView>) {
    let view = lock(log_view);
    session.set_filter_state(view.filter_state());
    session.set_discard_generation(view.discard_generation());
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

/// Returns at most `limit` rows of the timeline from `offset` (U3:BR4.3),
/// each with its time written in the chosen zone by AppSession (U4:BR3.4,
/// review R-08). With a log filter in force the offset counts in the
/// filter result and the window says so (U5:BR3.1). Locks the session,
/// then the LogView (the lock order).
#[tauri::command]
fn get_rows(offset: u64, limit: u64, state: State<'_, AppState>) -> DisplayRowWindow {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let limit = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(MAX_ROWS_PER_REQUEST);
    let session = lock(&state.session);
    let window = lock(&state.log_view).rows(offset, limit);
    session.display_rows(window)
}

/// Switches the time zone of the inputs and the log list (U4:BR1.4).
/// Accepted while fetching and while a change awaits confirmation; calls no
/// API. The new state arrives through `session-changed`, and the screen
/// re-reads its rows when the time zone in it changes.
#[tauri::command]
fn select_time_zone(time_zone: TimeZoneChoice, app: AppHandle, state: State<'_, AppState>) {
    let view = {
        let mut session = lock(&state.session);
        session.select_time_zone(time_zone);
        session.view()
    };
    emit_or_log(&app, SESSION_CHANGED, view);
}

/// Returns the current position of one event (U3:BR4.4); with a log filter
/// in force, its position in the filter result, or none when it is not in
/// it (U5:BR3.2, review R-08).
#[tauri::command]
fn find_row_position(
    log_stream_name: String,
    sequence: u64,
    state: State<'_, AppState>,
) -> RowPosition {
    let log_view = lock(&state.log_view);
    RowPosition {
        position: log_view
            .position_of(&log_stream_name, sequence)
            .map(|position| position as u64),
        timeline_version: log_view.version(),
    }
}

/// U7:BR1.7: the current positions of several rows (the expanded rows, the
/// selected row and the top row) in one LogView lock, with the timeline
/// version, the result version, the discard generation and the row count.
/// At most `MAX_KEYS_PER_REQUEST` keys are answered.
#[tauri::command]
fn row_positions(keys: Vec<RowKey>, state: State<'_, AppState>) -> RowPositions {
    let keys = &keys[..keys.len().min(MAX_KEYS_PER_REQUEST)];
    lock(&state.log_view).positions_of(keys)
}

/// [Keep fetching] or Escape on the close confirmation (U7:BR3.2): the
/// fetch goes on and the dialog closes.
#[tauri::command]
fn cancel_close(app: AppHandle, state: State<'_, AppState>) {
    let view = {
        let mut session = lock(&state.session);
        session.cancel_close();
        session.view()
    };
    emit_or_log(&app, SESSION_CHANGED, view);
}

/// [Close] on the close confirmation (U7:BR3.2, BR3.4): aborts a fetch
/// still running (U3:BR5.5; nothing is written to the cache, U6:BR3.2),
/// marks that the app may exit and exits without asking again. Does
/// nothing when no confirmation is pending.
#[tauri::command]
fn confirm_close(app: AppHandle, state: State<'_, AppState>) {
    let Some(confirmed) = lock(&state.session).confirm_close() else {
        return;
    };
    state.exit_allowed.store(true, Ordering::SeqCst);
    if confirmed.abort_fetch {
        state.abort_fetch();
    }
    app.exit(0);
}

/// U7:BR3.1: whether a request to close the window or end the app must be
/// stopped. Never once [Close] was chosen; otherwise AppSession decides,
/// and when its confirmation has just become Pending the screen is told.
fn must_stop_closing(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    if state.exit_allowed.load(Ordering::SeqCst) {
        return false;
    }
    let (decision, view) = {
        let mut session = lock(&state.session);
        let decision = session.request_close();
        (decision, session.view())
    };
    match decision {
        CloseDecision::Allow => false,
        CloseDecision::Prevent { changed } => {
            if changed {
                emit_or_log(app, SESSION_CHANGED, view);
            }
            true
        }
    }
}

/// Changes the log filter text (U5:BR1.1, BR1.4). Accepted while fetching;
/// refused while a connection change awaits confirmation. When the trimmed
/// text changed, the LogView gets the new condition under the session lock
/// and, for a non-empty text, a scan of the held events starts (U5:BR1.5);
/// a running scan of the previous text stops by itself (U5:BR2.4). The new
/// state arrives through `session-changed`. Calls no AWS API.
#[tauri::command]
fn set_log_filter(
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let (scan, view) = {
        let mut session = lock(&state.session);
        let change = session.update_log_filter(text)?;
        let mut log_view = lock(&state.log_view);
        let scan = match change.map(|text| log_view.set_filter(&text)) {
            Some(FilterChange::Started(ticket)) => Some(ticket),
            _ => None,
        };
        session.set_filter_state(log_view.filter_state());
        (scan, session.view())
    };
    emit_or_log(&app, SESSION_CHANGED, view);
    if let Some(ticket) = scan {
        spawn_filter_scan(&app, &state, ticket);
    }
    Ok(())
}

/// Scans the held events for the filter of `ticket` on a blocking thread
/// (U5:BR1.5), so the screen and the fetch keep working.
fn spawn_filter_scan(app: &AppHandle, state: &AppState, ticket: ScanTicket) {
    let session = Arc::clone(&state.session);
    let log_view = Arc::clone(&state.log_view);
    let task_app = app.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        run_filter_scan(&task_app, &session, &log_view, ticket);
    });
    tauri::async_runtime::spawn(async move {
        if task.await.is_err() {
            // Nothing secret can be in a scan; only the fact is logged.
            eprintln!("local-sights: the log filter scan ended abnormally");
        }
    });
}

/// Judges the held events one chunk per lock until the end, or until the
/// ticket is stale because the text changed or the timeline was discarded
/// (U5:BR1.5, BR2.4). Reports the rows found so far as the core's
/// `should_report_progress` decides: at most every 100 ms and always when
/// Ready (U5:BR3.6, U5 review R-03).
fn run_filter_scan(
    app: &AppHandle,
    session: &Mutex<AppSession>,
    log_view: &Mutex<LogView>,
    ticket: ScanTicket,
) {
    let mut last_report: Option<Instant> = None;
    loop {
        let step = lock(log_view).scan_chunk(ticket, SCAN_CHUNK_ROWS);
        match step {
            ScanStep::Stale => return,
            ScanStep::Finished => {
                if should_report_progress(last_report.map(|at| at.elapsed()), true) {
                    report_filter(app, session, log_view);
                }
                return;
            }
            ScanStep::Progressed => {
                if should_report_progress(last_report.map(|at| at.elapsed()), false) {
                    report_filter(app, session, log_view);
                    last_report = Some(Instant::now());
                }
                std::thread::yield_now();
            }
        }
    }
}

/// Copies the filter summary into the session and pushes `filter-progress`.
fn report_filter(app: &AppHandle, session: &Mutex<AppSession>, log_view: &Mutex<LogView>) {
    let update = {
        let mut session = lock(session);
        sync_filter(&mut session, log_view);
        session.filter_update()
    };
    emit_or_log(app, FILTER_PROGRESS, update);
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
    let (validated, fetch_number, cache, view) = {
        let mut session = lock(&state.session);
        let validated = session.begin_fetch(generation)?;
        // U6:BR1.5: `None` when the cache is disabled.
        let cache = session.fetch_cache();
        (validated, session.fetch_number(), cache, session.view())
    };
    let fetch_started_at_ms = now_epoch_ms();
    emit_or_log(app, SESSION_CHANGED, view);

    let (handle, signal) = abort_pair();
    *lock(&state.abort) = Some((fetch_number, handle));
    let session = Arc::clone(&state.session);
    let log_view = Arc::clone(&state.log_view);
    let gateway = Arc::clone(&state.gateway);
    let mut sink = TauriSink {
        app: app.clone(),
        session: Arc::clone(&session),
        log_view: Arc::clone(&log_view),
    };
    let task = tauri::async_runtime::spawn(async move {
        let policy = RetryPolicy::default();
        let jitter = RandomJitter;
        let retrier = Retrier::new(&policy, &jitter, &signal);
        run_fetch_with_cache(
            gateway.as_ref(),
            &validated,
            &log_view,
            &retrier,
            cache.as_ref(),
            fetch_started_at_ms,
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

/// The current time in epoch milliseconds, the base of the five-minute
/// margin of the cache (U6:BR3.1). A clock before 1970 counts as 0, which
/// only makes less recordable.
fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

/// Opens the settings dialog (U6:BR5.1); refused while fetching, while a
/// cache write runs (U6 review R-02) or while a connection change awaits
/// confirmation.
#[tauri::command]
fn open_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    change_settings(&app, &state, AppSession::open_settings)
}

/// Closes the settings dialog without saving ([Cancel], Escape, U6:BR5.2).
#[tauri::command]
fn cancel_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    change_settings(&app, &state, |session| {
        session.cancel_settings();
        Ok(())
    })
}

/// Saves the cache setting; switching it off removes the cached files
/// (U6:BR1.1, BR1.3). A failure is told in the dialog, which stays open.
/// The file work runs on a blocking thread without the session lock (U6
/// review R-01).
#[tauri::command]
async fn save_settings(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    run_settings_work(&app, &state, |session| session.begin_save_settings(enabled)).await
}

/// Removes every cached file at once ([Clear cache], U6:BR1.4), on a
/// blocking thread without the session lock (U6 review R-01).
#[tauri::command]
async fn clear_cache(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    run_settings_work(&app, &state, AppSession::begin_clear_cache).await
}

/// Runs one settings dialog operation that needs no file work and pushes
/// the new state. The dialog cannot be open while a fetch runs.
fn change_settings(
    app: &AppHandle,
    state: &AppState,
    operation: impl FnOnce(&mut AppSession) -> Result<(), SessionError>,
) -> Result<(), CommandError> {
    let view = {
        let mut session = lock(&state.session);
        operation(&mut session)?;
        session.view()
    };
    emit_or_log(app, SESSION_CHANGED, view);
    Ok(())
}

/// U6 review R-01: decides under the session lock, does the file work on a
/// blocking thread with no lock held, then applies the result under the
/// lock and pushes the new state.
async fn run_settings_work(
    app: &AppHandle,
    state: &AppState,
    begin: impl FnOnce(&mut AppSession) -> Result<Option<SettingsWork>, SessionError>,
) -> Result<(), CommandError> {
    let (work, view) = {
        let mut session = lock(&state.session);
        let work = begin(&mut session)?;
        (work, session.view())
    };
    let Some(work) = work else {
        emit_or_log(app, SESSION_CHANGED, view);
        return Ok(());
    };
    let failed = work.failed();
    let result = match tauri::async_runtime::spawn_blocking(move || work.run()).await {
        Ok(result) => result,
        Err(_) => {
            eprintln!("local-sights: settings: the file work ended abnormally");
            failed
        }
    };
    let view = {
        let mut session = lock(&state.session);
        session.finish_settings_work(result);
        session.view()
    };
    emit_or_log(app, SESSION_CHANGED, view);
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
/// When the operation discards the shown logs (U2:BR2.4), the LogView is
/// cleared while the session lock is still held (which also empties the
/// filter result and keeps its text, U5:BR2.2), and its new version and
/// filter summary are recorded in the session (U3:BR4.5), before
/// `session-changed` is emitted.
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
            let mut log_view = lock(&state.log_view);
            let version = log_view.clear();
            session.set_timeline_version(version);
            session.set_filter_state(log_view.filter_state());
            session.set_discard_generation(log_view.discard_generation());
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
/// page (review R-05). The coordinator adds each page to the LogView
/// itself; here the LogView is only read, after the session lock, to copy
/// the filter summary that the page or the discard changed (U5:BR3.6).
struct TauriSink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
    log_view: Arc<Mutex<LogView>>,
}

impl TauriSink {
    fn update(&self, change: impl FnOnce(&mut AppSession)) {
        let view = {
            let mut session = lock(&self.session);
            change(&mut session);
            sync_filter(&mut session, &self.log_view);
            session.view()
        };
        emit_or_log(&self.app, SESSION_CHANGED, view);
    }

    fn update_progress(&self, change: impl FnOnce(&mut AppSession)) {
        let update = {
            let mut session = lock(&self.session);
            change(&mut session);
            sync_filter(&mut session, &self.log_view);
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

    fn on_saving(&mut self, job_id: u64) {
        self.update_progress(|session| session.on_saving(job_id));
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
/// The app is built first and then run with a callback, so that ending the
/// app (`RunEvent::ExitRequested`: Cmd+Q, the Dock menu, the last window
/// closing) can be stopped like closing the window (U7:BR3.1, review R-02).
///
/// # Errors
/// Returns the Tauri error when the app cannot be built.
pub fn run() -> Result<(), tauri::Error> {
    let app = tauri::Builder::default()
        .setup(|app| {
            // U6:BR1.6: the folders come from the OS, so the state is made
            // here, where the app's paths are known.
            let location = cache_location(app.handle());
            app.manage(AppState::load(location));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // U7:BR3.1 (1): stopped while fetching or writing the cache; the
            // fetch is no longer aborted here (BR3.2 does that on [Close]).
            WindowEvent::CloseRequested { api, .. } => {
                if must_stop_closing(window.app_handle()) {
                    api.prevent_close();
                }
            }
            // U7:BR3.1 (3): the window is gone; stop a fetch just in case.
            WindowEvent::Destroyed => {
                if let Some(state) = window.try_state::<AppState>() {
                    state.abort_fetch();
                }
            }
            _ => {}
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
            row_positions,
            set_failure_list_open,
            select_time_zone,
            set_log_filter,
            open_settings,
            cancel_settings,
            save_settings,
            clear_cache,
            cancel_close,
            confirm_close
        ])
        .build(tauri::generate_context!())?;
    app.run(|app, event| {
        // U7:BR3.1 (2): Cmd+Q and the Dock menu, stopped like the window.
        if let RunEvent::ExitRequested { api, .. } = event {
            if must_stop_closing(app) {
                api.prevent_exit();
            }
        }
    });
    Ok(())
}
