//! Tauri wiring of local-sights (DesktopUi glue, ADR-001).
//!
//! This crate stays thin: every decision lives in `local-sights-core`.
//! Operations arrive as Tauri commands (`get_session`, `update_input`,
//! `start_fetch`, since U2 the connection and log group commands, and since
//! U3 `get_rows` and `set_failure_list_open`, and since
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
//! (U5:BR3.6). `get_rows` and `row_positions` read with the filter in
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
//! A request of more keys than it accepts is refused with an error, never
//! cut short (code generation review R-04). U3's `find_row_position` is gone
//! (review R-05).
//!
//! Closing the window (`CloseRequested`), choosing Quit in the app menu or
//! pressing Cmd+Q (a menu item of this app, handled in `on_menu_event`,
//! code generation review R-01) or ending the app with `app.exit`
//! (`ExitRequested`) while fetching or writing the cache is stopped and
//! AppSession's closeConfirmation becomes Pending (U7:BR3.1); the screen
//! answers with `cancel_close` ([Keep fetching]) or `confirm_close`
//! ([Close]: abort the fetch, set the "may exit" flag and exit, BR3.2). The
//! app is therefore built first and run with a callback that sees
//! `RunEvent::ExitRequested`. Known limitation (review R-01): the Dock
//! menu's Quit goes through `applicationWillTerminate` of tao, which can
//! neither be seen nor stopped here, so it ends the app without asking;
//! BR3.1 (2) is only partly met. Closing no longer aborts the fetch by
//! itself; only `Destroyed` still aborts, as a safety net. The settings
//! commands are async and do their file work on a blocking thread without
//! the session lock (U6 review R-01). When the last cache write ends, the
//! view is pushed again if no fetch runs, so a write that outlived its fetch
//! opens the settings dialog again on the screen (review R-03).
//!
//! Since 261011-sso-login [Sign in with SSO] runs `aws sso login` of the
//! AWS CLI (`start_sso_login`): the core decides whether it may start and
//! with which arguments, [`sso_cli`] finds and runs the CLI on a blocking
//! thread and hands back only the allow-listed URL and code, and every
//! change pushes `session-changed`. `cancel_sso_login` stops the CLI's
//! process group, `dismiss_sso_login` puts a result away and
//! `open_sso_url` opens the attempt's own URL in the default browser
//! through [`browser`]. A success drops the cached SDK clients and lists
//! the log groups again (BR4.1). Closing the window or ending the app
//! (`ExitRequested`, `Exit`, `Destroyed`) stops a running CLI; a login does
//! not ask before closing. `AWS_PROFILE` is read once at startup and given
//! to the session (BR1.3). The app itself calls no SSO API and never reads
//! or writes the SSO token cache (BR4.3).
//!
//! Since 261015-windows-support the app also runs on Windows 11 (x64).
//! Everything that depends on the OS sits behind `cfg` in [`sso_cli`]
//! (finding, starting and stopping the AWS CLI: a Unix process group or a
//! Windows job object) and [`browser`] (`/usr/bin/open` or
//! `ShellExecuteW`); the settings and cache folders come from Tauri's path
//! resolver on both. The app menu with its own Quit item exists only on
//! macOS; on Windows there is no menu bar, and closing the window
//! (the close button, Alt+F4) goes through `CloseRequested` like
//! everywhere else (FR3.4). Ending the app on Windows stops the CLI's job
//! on `Exit` as on macOS, and a crash closes the job handle, which stops
//! the job by itself (FR4.4).
//!
//! Since 261012-parallel-fetch U5 `cancel_fetch` is [Cancel]: under the
//! session lock it asks AppSession whether it can be pressed, tells the
//! running fetch's stop handle to stop for Cancel and pushes the view with
//! [Cancelling…] (BR5.1). The handle is registered when the fetch starts,
//! still under the session lock, so a cancel right after the start always
//! reaches it (NFR6.17); the lock order is session, then the handle. Closing
//! still stops the fetch for Close, which wins over a cancel (BR5.6).
//!
//! Since 261017-insights-query U1 the Log Analytics screen runs queries on
//! logs it fetches into its own `QueryStore` (U1:BR1.7, BR2.1). The commands
//! are `update_analytics_input`, `run_query`, `cancel_query` and
//! `get_query_rows`; every change arrives through `session-changed`, whose
//! view carries the query screen. `run_query` reads the query outside the
//! session lock, then starts the run in one session call that checks the
//! reasons again, takes the fetch and registers the run's stop handle
//! (functional-design review R-04, as `start_fetch`, NFR6.17); the run
//! (fetch, then evaluation chunk by chunk on blocking threads) is a task
//! of its own. `cancel_query` tells that handle to stop for Cancel (UC4,
//! R-01). Closing stops it for Close like the Log Management fetch
//! (U1:BR2.8); a connection change discards the QueryStore with the LogView
//! (U1:BR7.2). Lock order: session, then the QueryStore; the run never holds
//! both. The query fetch calls the same three read APIs only (U1:BR2.2).
//!
//! Since 261017-insights-query U2 `get_query_rows` gives each row its cells
//! for the result's columns (U2:BR7.1), `get_query_row_detail` the fields of
//! one opened row (U2:BR7.3) and `get_completion_candidates` the words and
//! field names the query field completes (U2:BR9.1, Should). An evaluation
//! that ends abnormally shows as an evaluation failure, also when the run's
//! task is lost while evaluating, never as a failed GetLogEvents
//! (U2:BR10.2).
//!
//! Since 261017-insights-query U3 `update_analytics_input` takes the log
//! groups as a list of up to 50 names (U3:BR4.1), a run fetches only what
//! the QueryStore and the disk cache do not hold, log group by log group
//! (U3:BR2.1-BR2.5), and its progress counts the log groups (U3:BR2.7).
//! `get_query_rows` and `get_query_row_detail` read the result's own copies
//! of its rows, checked by the run number, so they no longer lock the
//! QueryStore (U3:BR3.2).
//!
//! Diagnostics go to standard error only and contain only safe details:
//! no secret credential and no access key ID is ever logged or shown. No
//! log file is ever written (U7:BR4.5, NFR16).

mod browser;
mod sso_cli;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use local_sights_core::cache::LogCache;
use local_sights_core::catalog::ProfileSelector;
use local_sights_core::catalog::files::load_catalog;
use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, run_fetch_with_cache};
use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::fetch_status::JobStatus;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::filter::{FilterChange, ScanStep, ScanTicket, should_report_progress};
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::gateway::{DESCRIBE_LOG_GROUPS, GET_LOG_EVENTS};
use local_sights_core::insights_session::{
    AnalyticsField, AnalyticsValue, CompletionCandidate, QueryBegin, QueryRowDetail,
    QueryRowWindow, QueryStart,
};
use local_sights_core::log_groups::listing::{ListingRequest, ListingSink, run_listing};
use local_sights_core::log_groups::{ListingStatus, LogGroup};
use local_sights_core::log_view::{LogView, RowKey, RowPositions};
use local_sights_core::paging::PageDecision;
use local_sights_core::query::evaluator::QueryResult;
use local_sights_core::query::instant::resolve_date_texts;
use local_sights_core::query::parser::parse_query;
use local_sights_core::query::plan::{QueryPlan, QueryProblem};
use local_sights_core::query::runner::{self, QueryRun, QueryRunSummary, QuerySink};
use local_sights_core::query::store::QueryStore;
use local_sights_core::request::ValidatedFetch;
use local_sights_core::retry::{
    RandomJitter, Retrier, RetryPolicy, StopHandle, StopReason, StopSignal, stop_pair,
};
use local_sights_core::session::{
    AppSession, CacheLocation, CloseDecision, ConnectionEffect, DisplayRowWindow, FetchOwner,
    InputField, LanguagePreference, SessionError, SessionView, SettingsWork, SsoLoginStart,
};
use local_sights_core::sso_login::{SsoLoginOutcome, is_allowed_login_url};
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::time_zone::{TimeZoneChoice, TimeZoneContext};
use serde::Serialize;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime, State, WindowEvent};

use crate::sso_cli::{LoginControl, LoginTimes, STOP_GRACE, cli_candidates, locate_cli, run_login};

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
/// Id of this app's Quit menu item (Cmd+Q, code generation review R-01).
const QUIT_MENU_ID: &str = "local-sights-quit";

/// The running SSO login: its attempt number and the control that stops its
/// CLI (261011-sso-login BR3.3).
type RunningSsoLogin = Option<(u64, Arc<LoginControl>)>;

/// The stop handle of a running fetch or query run, with its number.
type RunningStop = Arc<Mutex<Option<(u64, StopHandle)>>>;

/// Least time between two progress views of a query run (U1:BR2.4): a page
/// or an evaluation chunk can come far more often than the screen needs.
const QUERY_PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Shared state of the app.
#[derive(Debug)]
struct AppState {
    session: Arc<Mutex<AppSession>>,
    /// The held events and the log filter (U5 review R-02).
    log_view: Arc<Mutex<LogView>>,
    gateway: Arc<AwsCloudWatchLogsGateway>,
    /// Stop handle of the running fetch with its fetch number from the
    /// session (BR5.5), so a finished fetch only forgets its own handle.
    abort: RunningStop,
    /// The Log Analytics screen's holdings (261017-insights-query U1:BR2.1).
    query_store: Arc<QueryStore>,
    /// Stop handle of the running query run with its run number (UC4).
    /// Locked after the session.
    query_abort: RunningStop,
    /// Set by [Close] (U7:BR3.2): the exit and window close that follow are
    /// not stopped again. Read without the session lock.
    exit_allowed: AtomicBool,
    /// The running SSO login: its attempt number and the control that
    /// stops its CLI (261011-sso-login BR3.3). Locked after the session.
    sso_login: Arc<Mutex<RunningSsoLogin>>,
}

impl AppState {
    /// Reads the shared AWS config files (U2:BR1.1-BR1.3) and the OS time
    /// zone (U4:BR1.1) once at startup. When the OS zone cannot be used,
    /// Local falls back to UTC and only the zone name is logged. Since U6 it
    /// also takes the cache location and reads the cache setting from it;
    /// without a location the cache stays disabled (U6:BR1.6). Since gate
    /// fix 16a the start and end inputs begin as yesterday 00:00:00 and
    /// today 23:59:59 in the local zone, from the system clock; when the
    /// clock cannot be read as epoch milliseconds they stay empty.
    fn load(cache_location: Option<CacheLocation>) -> Self {
        let time_zones = TimeZoneContext::detect();
        if let Some(fallback) = &time_zones.fallback {
            eprintln!("local-sights: {}", fallback.diagnostic());
        }
        // 261011-sso-login BR1.3: the SDK default profile is AWS_PROFILE
        // when set; the CLI inherits the same environment (BR2.4).
        let mut session =
            AppSession::with_catalog_and_time_zones(load_catalog(), time_zones.context)
                .with_aws_profile_env(std::env::var("AWS_PROFILE").ok());
        if let Some(now_ms) = current_epoch_ms() {
            session = session.with_default_time_range(now_ms);
        }
        if let Some(location) = cache_location {
            session = session.with_cache_location(location);
        }
        Self {
            session: Arc::new(Mutex::new(session)),
            log_view: Arc::default(),
            gateway: Arc::default(),
            abort: Arc::default(),
            query_store: Arc::default(),
            query_abort: Arc::default(),
            exit_allowed: AtomicBool::new(false),
            sso_login: Arc::default(),
        }
    }

    /// Asks the running fetch and query run, if any, to stop ([Close] or the
    /// window being destroyed, BR5.5, U7:BR3.2, 261017-insights-query
    /// U1:BR2.8). Both are a Close: the fetched logs are discarded
    /// (261012-parallel-fetch U1:BR1.16, contract C7).
    fn abort_fetch(&self) {
        for running in [&self.abort, &self.query_abort] {
            if let Some((_, handle)) = lock(running).as_ref() {
                handle.stop(StopReason::Close);
            }
        }
    }

    /// The app or its window ends: stops a running SSO login's CLI before
    /// returning, waiting at most [`STOP_GRACE`] (261011-sso-login BR3.3,
    /// NFR4). Does nothing when no login runs.
    fn stop_sso_cli(&self) {
        let running = lock(&self.sso_login).take();
        if let Some((_, control)) = running {
            control.stop_now(STOP_GRACE);
        }
    }
}

/// U6:BR1.6: the settings file in the OS app settings folder and the cache
/// folder inside the OS cache folder (on macOS `~/Library/Application
/// Support/<id>/` and `~/Library/Caches/<id>/`; on Windows
/// `%APPDATA%\<id>\` and `%LOCALAPPDATA%\<id>\`, FR3.2). `None`, with a
/// diagnostic, when the OS gives no folder; the cache then stays disabled.
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

/// U7:BR1.7: the current positions of several rows (the expanded rows, the
/// selected row and the top row) in one LogView lock, with the timeline
/// version, the result version, the discard generation and the row count.
///
/// # Errors
/// `rows.tooManyKeys` when more than `MAX_POSITION_KEYS` (10,000 open rows,
/// the selected and the top row) keys are asked; nothing is answered then,
/// so no row is taken for hidden by mistake (code generation review R-04).
#[tauri::command]
fn row_positions(
    keys: Vec<RowKey>,
    state: State<'_, AppState>,
) -> Result<RowPositions, CommandError> {
    lock(&state.log_view)
        .checked_positions_of(&keys)
        .map_err(|error| {
            // Counts only: no key, no message.
            eprintln!("local-sights: row_positions: {error}");
            CommandError {
                key: error.message_key().to_string(),
            }
        })
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

/// Code generation review R-01: Quit in the app menu or Cmd+Q goes through
/// the same decision as closing the window. While fetching or writing the
/// cache the confirmation becomes Pending (and the screen is told);
/// otherwise the app exits.
fn quit_requested(app: &AppHandle) {
    if !must_stop_closing(app) {
        app.exit(0);
    }
}

/// Handles this app's menu items: only Quit (Cmd+Q) is its own; the
/// predefined items (copy, paste, hide, ...) are handled by the OS. Only
/// macOS has the menu; elsewhere no event ever arrives here, and the
/// handler depends on nothing macOS-specific, so it is registered on all.
fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id() == QUIT_MENU_ID {
        quit_requested(app);
    }
}

/// Code generation review R-01: the macOS app menu. It is Tauri's default
/// menu with Quit (Cmd+Q) replaced by an item of this app, so that it can
/// be stopped while fetching ([`handle_menu_event`]); the default Quit
/// would end the app through `terminate:` without asking. The Edit menu
/// keeps the standard items, so copying an expanded message and pasting
/// into the inputs keep working with the keyboard.
///
/// The Dock menu's Quit cannot be routed here (known limitation, BR3.1
/// (2) only partly met).
///
/// # Errors
/// Returns the Tauri error when a menu item cannot be made.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let name = app.package_info().name.clone();
    let quit = MenuItem::with_id(
        app,
        QUIT_MENU_ID,
        format!("Quit {name}"),
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let app_submenu = Submenu::with_items(
        app,
        name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    Menu::with_items(app, &[&app_submenu, &edit, &view, &window])
}

/// Code generation review R-03: when the last cache write ends, pushes the
/// view again unless a fetch runs (a running fetch pushes its own view when
/// it ends). This is what tells the screen that a write which outlived its
/// fetch is over, so the settings dialog can be opened again. Runs on the
/// async runtime, never inside the write's own thread or a held lock.
fn watch_cache_writes(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let listener_app = app.clone();
    lock(&state.session)
        .cache_writes()
        .on_idle(Arc::new(move || {
            let app = listener_app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app.try_state::<AppState>() else {
                    return;
                };
                let view = {
                    let session = lock(&state.session);
                    // 261017-insights-query R-02: a query fetch pushes its
                    // own view when it ends, as a Log Management fetch does.
                    if session.fetch_owner() != FetchOwner::None {
                        return;
                    }
                    session.view()
                };
                emit_or_log(&app, SESSION_CHANGED, view);
            });
        }));
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

/// A fetch AppSession accepted, with the stop signal of its registered
/// handle.
struct BegunFetch {
    validated: ValidatedFetch,
    fetch_number: u64,
    cache: Option<LogCache>,
    view: SessionView,
    signal: StopSignal,
}

/// Starts a fetch in the session and registers its stop handle under the
/// fetch number while the session lock is still held (261012-parallel-fetch
/// U5:NFR6.17): a [Cancel] accepted right after the start always finds the
/// new handle, never the previous fetch's or none. The lock order is
/// session, then the handle (as in [`cancel_running_fetch`]).
fn begin_registered_fetch(
    session: &Mutex<AppSession>,
    abort: &Mutex<Option<(u64, StopHandle)>>,
    generation: u64,
) -> Result<BegunFetch, SessionError> {
    let mut session = lock(session);
    let validated = session.begin_fetch(generation)?;
    // U6:BR1.5: `None` when the cache is disabled.
    let cache = session.fetch_cache();
    let fetch_number = session.fetch_number();
    let (handle, signal) = stop_pair();
    *lock(abort) = Some((fetch_number, handle));
    Ok(BegunFetch {
        validated,
        fetch_number,
        cache,
        view: session.view(),
        signal,
    })
}

fn begin_and_spawn_fetch(
    generation: u64,
    app: &AppHandle,
    state: &AppState,
) -> Result<(), CommandError> {
    let BegunFetch {
        validated,
        fetch_number,
        cache,
        view,
        signal,
    } = begin_registered_fetch(&state.session, &state.abort, generation)?;
    let fetch_started_at_ms = now_epoch_ms();
    emit_or_log(app, SESSION_CHANGED, view);

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

/// [Cancel] (261012-parallel-fetch U5:BR5.1, security-design 2.3). Always
/// succeeds: when [Cancel] cannot be pressed nothing changes and nothing is
/// sent. Otherwise the running fetch is told to stop for Cancel and the
/// view with [Cancelling…] is pushed.
#[tauri::command]
fn cancel_fetch(app: AppHandle, state: State<'_, AppState>) {
    cancel_running_fetch(&state.session, &state.abort, |view| {
        emit_or_log(&app, SESSION_CHANGED, view);
    });
}

/// Accepts [Cancel] in the session, tells the registered handle of that
/// same fetch to stop for Cancel, and hands the new view to `send` while
/// the session lock is still held. The fetch's own end makes its view only
/// after it took the session lock, so `send` always comes before the end's
/// view and the screen is never left on a stale [Cancelling…]
/// (NFR6.16, NFR6.17). Lock order: session, then the handle; the emit does
/// not wait, so it may run under the lock. Returns whether it was accepted.
fn cancel_running_fetch(
    session: &Mutex<AppSession>,
    abort: &Mutex<Option<(u64, StopHandle)>>,
    send: impl FnOnce(SessionView),
) -> bool {
    let mut session = lock(session);
    let Some(fetch_number) = session.request_cancel() else {
        return false;
    };
    if let Some((number, handle)) = lock(abort).as_ref() {
        if *number == fetch_number {
            handle.stop(StopReason::Cancel);
        }
    }
    send(session.view());
    true
}

/// Changes one field of the Log Analytics screen (261017-insights-query
/// U1:BR1.3, BR1.4); refused while its query runs (U1:BR2.4). The log
/// groups come as a list of up to 50 names (U3:BR4.1), a text for the other
/// fields. Log groups chosen for an older connection are dropped
/// (U3:BR6.7). The new state arrives through `session-changed`.
#[tauri::command]
fn update_analytics_input(
    field: AnalyticsField,
    value: AnalyticsValue,
    generation: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    drop_if_stale(change_analytics_input(
        &app, &state, field, value, generation,
    ))
}

fn change_analytics_input(
    app: &AppHandle,
    state: &AppState,
    field: AnalyticsField,
    value: AnalyticsValue,
    generation: u64,
) -> Result<(), CommandError> {
    let view = {
        let mut session = lock(&state.session);
        session.update_analytics_input(field, value, generation)?;
        session.view()
    };
    emit_or_log(app, SESSION_CHANGED, view);
    Ok(())
}

/// [Run query] (261017-insights-query UC1). The query is read and its date
/// texts are turned into milliseconds outside the session lock; the session
/// then checks the reasons again and either holds the reading problem
/// (QueryError, nothing is fetched, U1:BR3.9) or starts the run, all in one
/// call (R-04). Refused, with the reason's key, when [Run query] cannot be
/// pressed (U1:BR1.5). The command returns nothing: the state arrives through
/// `session-changed`.
#[tauri::command]
fn run_query(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    let (snapshot, zone) = lock(&state.session).query_snapshot();
    let parsed = parse_query(&snapshot.text).and_then(|plan| resolve_date_texts(plan, zone));
    let begun = begin_registered_query(
        &state.session,
        &state.query_abort,
        snapshot.revision,
        parsed.as_ref().map_err(Clone::clone),
    )?;
    emit_or_log(&app, SESSION_CHANGED, begun.view);
    if let (Some(start), Some(signal), Ok(plan)) = (begun.start, begun.signal, parsed) {
        spawn_query(&app, &state, start, begun.cache, plan, signal);
    }
    Ok(())
}

/// A [Run query] the session took: the run to start, when the query could
/// be read, with the stop signal of its registered handle.
struct BegunQuery {
    start: Option<QueryStart>,
    cache: Option<LogCache>,
    view: SessionView,
    signal: Option<StopSignal>,
}

/// Starts the run in the session and registers its stop handle under the
/// run number while the session lock is still held, so a [Cancel] right
/// after the start always reaches it (as [`begin_registered_fetch`]).
fn begin_registered_query(
    session: &Mutex<AppSession>,
    abort: &Mutex<Option<(u64, StopHandle)>>,
    revision: u64,
    parsed: Result<&QueryPlan, QueryProblem>,
) -> Result<BegunQuery, SessionError> {
    let mut session = lock(session);
    let QueryBegin::Fetch(start) = session.begin_query(revision, parsed)? else {
        return Ok(BegunQuery {
            start: None,
            cache: None,
            view: session.view(),
            signal: None,
        });
    };
    // U6:BR1.5 as for any fetch: `None` when the cache is disabled.
    let cache = session.fetch_cache();
    let (handle, signal) = stop_pair();
    *lock(abort) = Some((start.run_id, handle));
    Ok(BegunQuery {
        start: Some(start),
        cache,
        view: session.view(),
        signal: Some(signal),
    })
}

/// Runs one query in the background: the fetch into the QueryStore, then
/// the evaluation (UC1 8-12). When the task ends, its stop handle is
/// forgotten and a run left Fetching or Evaluating is failed.
fn spawn_query(
    app: &AppHandle,
    state: &AppState,
    start: QueryStart,
    cache: Option<LogCache>,
    plan: QueryPlan,
    signal: StopSignal,
) {
    let run_id = start.run_id;
    let run = QueryRun {
        run_id,
        plan: Arc::new(plan),
        range: start.range,
        keys: start.keys,
        cache,
        run_started_at_ms: now_epoch_ms(),
        clock: now_epoch_ms,
    };
    let session = Arc::clone(&state.session);
    let store = Arc::clone(&state.query_store);
    let gateway = Arc::clone(&state.gateway);
    let mut sink = TauriQuerySink {
        app: app.clone(),
        session: Arc::clone(&session),
        run_id,
        last_progress: None,
    };
    let task = tauri::async_runtime::spawn(async move {
        let policy = RetryPolicy::default();
        let jitter = RandomJitter;
        let retrier = Retrier::new(&policy, &jitter, &signal);
        runner::run_query(gateway.as_ref(), &run, &store, &retrier, &mut sink).await;
    });
    let app = app.clone();
    let abort = Arc::clone(&state.query_abort);
    tauri::async_runtime::spawn(async move {
        let ended_normally = task.await.is_ok();
        {
            let mut running = lock(&abort);
            if running
                .as_ref()
                .is_some_and(|(number, _)| *number == run_id)
            {
                running.take();
            }
        }
        recover_if_query_still_running(&app, &session, run_id, ended_normally);
    });
}

/// As [`recover_if_still_fetching`] for a query run: a run whose task ended
/// without its end being reported is failed and the screen is told. A run
/// still fetching fails as a fetch (kind Other); one evaluating is an
/// evaluation failure without the fetch's API name (the session decides,
/// U2:BR10.2). Only run `run_id` is touched.
fn recover_if_query_still_running(
    app: &AppHandle,
    session: &Mutex<AppSession>,
    run_id: u64,
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
        if !session.abort_query_with_failure(run_id, failure) {
            return;
        }
        session.view()
    };
    let how = if ended_normally {
        "without a result"
    } else {
        "abnormally"
    };
    eprintln!("local-sights: the query task ended {how}; the query was marked as failed");
    emit_or_log(app, SESSION_CHANGED, view);
}

/// [Cancel] of the Log Analytics screen (UC4). Always succeeds: when it
/// cannot be pressed nothing changes and nothing is sent.
#[tauri::command]
fn cancel_query(app: AppHandle, state: State<'_, AppState>) {
    cancel_running_query(&state.session, &state.query_abort, |view| {
        emit_or_log(&app, SESSION_CHANGED, view);
    });
}

/// Accepts [Cancel] in the session (which marks the run, R-01), tells the
/// registered handle of that run to stop for Cancel and hands the new view
/// to `send` under the session lock (as [`cancel_running_fetch`]). Returns
/// whether it was accepted.
fn cancel_running_query(
    session: &Mutex<AppSession>,
    abort: &Mutex<Option<(u64, StopHandle)>>,
    send: impl FnOnce(SessionView),
) -> bool {
    let mut session = lock(session);
    let Some(run_id) = session.request_query_cancel() else {
        return false;
    };
    if let Some((number, handle)) = lock(abort).as_ref() {
        if *number == run_id {
            handle.stop(StopReason::Cancel);
        }
    }
    send(session.view());
    true
}

/// At most `limit` result rows from `offset` of the run the screen shows
/// (U1:BR6.4, 261017-insights-query U3:BR3.2); no rows for another run.
/// The run number alone tells the result (261020-carryover-fixes FR2.3.2).
#[tauri::command]
fn get_query_rows(
    run_id: u64,
    offset: u64,
    limit: u64,
    state: State<'_, AppState>,
) -> QueryRowWindow {
    query_rows(&state.session, run_id, (offset, limit))
}

/// Reads the rows of [`get_query_rows`] from the shown result's own copies
/// (U3:BR3.2: the QueryStore is not locked), with their times written in
/// the chosen zone.
fn query_rows(
    session: &Mutex<AppSession>,
    run_id: u64,
    (offset, limit): (u64, u64),
) -> QueryRowWindow {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let limit = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(MAX_ROWS_PER_REQUEST);
    lock(session).display_query_rows(run_id, offset, limit)
}

/// The fields of result row `index` of the run the screen shows
/// (261017-insights-query U2:BR7.3, U3:BR3.2); no field for another run.
#[tauri::command]
fn get_query_row_detail(run_id: u64, index: u64, state: State<'_, AppState>) -> QueryRowDetail {
    query_row_detail(&state.session, run_id, index)
}

/// Reads [`get_query_row_detail`] from the shown result's own copy of the
/// row.
fn query_row_detail(session: &Mutex<AppSession>, run_id: u64, index: u64) -> QueryRowDetail {
    let index = usize::try_from(index).unwrap_or(usize::MAX);
    lock(session).display_query_row_detail(run_id, index)
}

/// The completion candidates of the query field (261017-insights-query
/// U2:BR9.1, Should): the commands, words and fixed fields, and the JSON
/// field names of the query screen's last fetch.
#[tauri::command]
fn get_completion_candidates(state: State<'_, AppState>) -> Vec<CompletionCandidate> {
    lock(&state.session).query_completion_candidates()
}

/// The current time in epoch milliseconds for the default time range at
/// startup (gate fix 16a), or `None` when the clock is before 1970 or does
/// not fit; the inputs then stay empty rather than show a made-up day.
fn current_epoch_ms() -> Option<i64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(elapsed.as_millis()).ok()
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

/// Saves the cache setting and the language preference in one file
/// (U6:BR1.1, BR1.3; 261022-small-features U1:BR1.3); switching the cache
/// off removes the cached files. A failure is told in the dialog, which
/// stays open, and changes neither setting (U1:BR1.6). The file work runs
/// on a blocking thread without the session lock (U6 review R-01).
#[tauri::command]
async fn save_settings(
    enabled: bool,
    language: LanguagePreference,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    run_settings_work(&app, &state, |session| {
        session.begin_save_settings(enabled, language)
    })
    .await
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
            clear_held_logs(&mut session, &state.log_view, &state.query_store);
        }
        (effect, session.view())
    };
    emit_or_log(app, SESSION_CHANGED, view);
    if let Some(request) = effect.listing {
        start_listing(app, state, request);
    }
    Ok(())
}

/// U2:BR2.4, 261017-insights-query U1:BR7.2: a connection change discards
/// the logs of both screens. Called with the session locked; locks the
/// LogView, then the QueryStore (a run never holds the session meanwhile, and
/// none can run, U1:BR7.3).
fn clear_held_logs(session: &mut AppSession, log_view: &Mutex<LogView>, query_store: &QueryStore) {
    let mut log_view = lock(log_view);
    let version = log_view.clear();
    session.set_timeline_version(version);
    session.set_filter_state(log_view.filter_state());
    session.set_discard_generation(log_view.discard_generation());
    query_store.discard_all();
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

/// [Sign in with SSO] (261011-sso-login FR2.1): the core decides whether a
/// login may start and with which arguments (BR1.5, BR2.2, BR2.5); the CLI
/// is then found and run on a blocking thread. Returns nothing: the state
/// arrives through `session-changed`.
#[tauri::command]
fn start_sso_login(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    let control = LoginControl::new();
    let (start, view) = {
        let mut session = lock(&state.session);
        let start = session.start_sso_login()?;
        // Recorded under the session lock, so a [Cancel] that follows
        // always finds the control (lock order: session, then sso_login).
        *lock(&state.sso_login) = Some((start.attempt_id, Arc::clone(&control)));
        (start, session.view())
    };
    emit_or_log(&app, SESSION_CHANGED, view);
    spawn_sso_login(&app, &state, start, control);
    Ok(())
}

/// [Cancel] in the login dialog (FR2.4, BR3.3): the attempt becomes
/// Cancelled at once and its CLI is stopped by the runner.
#[tauri::command]
fn cancel_sso_login(app: AppHandle, state: State<'_, AppState>) {
    let view = {
        let mut session = lock(&state.session);
        if let Some(attempt_id) = session.cancel_sso_login() {
            let running = lock(&state.sso_login);
            if let Some((_, control)) = running.as_ref().filter(|(id, _)| *id == attempt_id) {
                control.request_stop();
            }
        }
        session.view()
    };
    emit_or_log(&app, SESSION_CHANGED, view);
}

/// [Close] on a login result, or closing the success notice.
#[tauri::command]
fn dismiss_sso_login(app: AppHandle, state: State<'_, AppState>) {
    let view = {
        let mut session = lock(&state.session);
        if !session.dismiss_sso_login() {
            return;
        }
        session.view()
    };
    emit_or_log(&app, SESSION_CHANGED, view);
}

/// Opens the sign-in URL of the running login in the default browser
/// (functional-design review: URL; FR4.5). Only the URL the CLI printed
/// for the current attempt, and only when it passes the allow-list again,
/// is handed to [`browser::open_url`] (no shell on either OS).
///
/// # Errors
/// `ssoLogin.openFailed` when the URL is not the attempt's or cannot be
/// opened.
#[tauri::command]
fn open_sso_url(url: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let refused = || CommandError {
        key: "ssoLogin.openFailed".to_string(),
    };
    let is_current = lock(&state.session).sso_login_url() == Some(url.as_str());
    if !is_current || !is_allowed_login_url(&url) {
        return Err(refused());
    }
    browser::open_url(&url).map_err(|error| {
        // The kind only: the message could name local paths.
        eprintln!(
            "local-sights: sso login: the browser could not be opened ({:?})",
            error.kind()
        );
        refused()
    })
}

/// Runs one login on a blocking thread. If that thread ends abnormally,
/// the attempt is ended as Failed, so the dialog never stays open.
fn spawn_sso_login(
    app: &AppHandle,
    state: &AppState,
    start: SsoLoginStart,
    control: Arc<LoginControl>,
) {
    let attempt_id = start.attempt_id;
    let task_app = app.clone();
    let task_session = Arc::clone(&state.session);
    let task = tauri::async_runtime::spawn_blocking(move || {
        run_sso_login(&task_app, &task_session, &start, &control)
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome = match task.await {
            Ok(outcome) => outcome,
            Err(_) => {
                eprintln!("local-sights: sso login: the login task ended abnormally");
                SsoLoginOutcome::Failed { exit_code: None }
            }
        };
        finish_sso_login(&app, attempt_id, outcome);
    });
}

/// Finds the CLI (BR2.1) and runs it (BR2.2-BR3.3), reporting the start
/// and every URL or code to the session as they come (BR3.1).
fn run_sso_login(
    app: &AppHandle,
    session: &Mutex<AppSession>,
    start: &SsoLoginStart,
    control: &LoginControl,
) -> SsoLoginOutcome {
    // 261015-windows-support FR4.1: the candidates are absolute paths; the
    // first that is an executable file is the one started (review R-05).
    let candidates = cli_candidates(
        std::env::var_os("ProgramFiles").as_deref(),
        std::env::var_os("PATH").as_deref(),
    );
    let Some(cli) = locate_cli(&candidates) else {
        return SsoLoginOutcome::CliNotFound;
    };
    let id = start.attempt_id;
    run_login(
        &cli,
        &start.args,
        control,
        LoginTimes::default(),
        || update_sso_login(app, session, |session| session.on_sso_cli_started(id)),
        |hints| {
            update_sso_login(app, session, |session| {
                session.on_sso_login_hints(id, hints)
            })
        },
    )
}

/// Applies one login report and pushes the view when it changed.
fn update_sso_login(
    app: &AppHandle,
    session: &Mutex<AppSession>,
    change: impl FnOnce(&mut AppSession) -> bool,
) {
    let view = {
        let mut session = lock(session);
        if !change(&mut session) {
            return;
        }
        session.view()
    };
    emit_or_log(app, SESSION_CHANGED, view);
}

/// Ends the attempt (BR4.1, BR4.2): after a success the cached SDK clients
/// are dropped first, so the listing that follows reads the new token, and
/// the log groups are listed again; the fetch is not started. A report the
/// session ignores (the attempt was cancelled) changes nothing.
fn finish_sso_login(app: &AppHandle, attempt_id: u64, outcome: SsoLoginOutcome) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    {
        let mut running = lock(&state.sso_login);
        if running.as_ref().is_some_and(|(id, _)| *id == attempt_id) {
            running.take();
        }
    }
    if outcome == SsoLoginOutcome::Succeeded {
        state.gateway.clear_clients();
    }
    let (finish, view) = {
        let mut session = lock(&state.session);
        // The CLI is gone now. After a [Cancel] the attempt has already
        // ended, so the only change is that a new login may start (FR2.6).
        let freed = session.on_sso_cli_ended(attempt_id);
        let Some(finish) = session.finish_sso_login(attempt_id, outcome) else {
            if freed {
                let view = session.view();
                drop(session);
                emit_or_log(app, SESSION_CHANGED, view);
            }
            return;
        };
        (finish, session.view())
    };
    // The outcome only: no CLI output, URL or code is logged (NFR3).
    eprintln!("local-sights: sso login {attempt_id} ended: {outcome:?}");
    emit_or_log(app, SESSION_CHANGED, view);
    if let Some(request) = finish.listing {
        start_listing(app, &state, request);
    }
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

/// Applies one fetch change to the session under its lock, copies the
/// filter summary and makes the view, which the caller sends after the lock
/// is released. The view of the fetch's end is therefore made only after a
/// [Cancel] that took the lock first has sent its own (U5:NFR6.16).
fn change_and_view(
    session: &Mutex<AppSession>,
    log_view: &Mutex<LogView>,
    change: impl FnOnce(&mut AppSession),
) -> SessionView {
    let mut session = lock(session);
    change(&mut session);
    sync_filter(&mut session, log_view);
    session.view()
}

impl TauriSink {
    fn update(&self, change: impl FnOnce(&mut AppSession)) {
        let view = change_and_view(&self.session, &self.log_view, change);
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

/// Forwards one query run's progress to AppSession with its run number and
/// pushes `session-changed` (261017-insights-query U1:BR2.4): at the start,
/// the plan, each finished stream, the cache write and every end at once;
/// listing pages, added pages and evaluated chunks at most every
/// [`QUERY_PROGRESS_INTERVAL`]. The coordinator and the runner fill the
/// QueryStore themselves; nothing here touches the Log Management screen's
/// LogView (U1:BR1.7).
struct TauriQuerySink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
    run_id: u64,
    last_progress: Option<Instant>,
}

impl TauriQuerySink {
    /// Applies `change` and pushes the view; returns what `change` returned.
    fn update<T>(&mut self, change: impl FnOnce(&mut AppSession) -> T) -> T {
        let (answer, view) = {
            let mut session = lock(&self.session);
            let answer = change(&mut session);
            (answer, session.view())
        };
        self.last_progress = Some(Instant::now());
        emit_or_log(&self.app, SESSION_CHANGED, view);
        answer
    }

    /// Applies `change`; pushes the view only when the last one is older
    /// than [`QUERY_PROGRESS_INTERVAL`].
    fn update_progress<T>(&mut self, change: impl FnOnce(&mut AppSession) -> T) -> T {
        let due = self
            .last_progress
            .is_none_or(|at| at.elapsed() >= QUERY_PROGRESS_INTERVAL);
        let (answer, view) = {
            let mut session = lock(&self.session);
            let answer = change(&mut session);
            (answer, due.then(|| session.view()))
        };
        if let Some(view) = view {
            self.last_progress = Some(Instant::now());
            emit_or_log(&self.app, SESSION_CHANGED, view);
        }
        answer
    }
}

impl FetchSink for TauriQuerySink {
    fn on_started(&mut self, job: &FetchJob, _timeline_version: u64) {
        let run_id = self.run_id;
        self.update(|session| session.on_query_job_started(run_id, job));
    }

    fn on_listing_progress(&mut self, job_id: u64, progress: ListingProgress) {
        let run_id = self.run_id;
        self.update_progress(|session| session.on_query_listing_progress(run_id, job_id, progress));
    }

    fn on_planned(&mut self, job: &FetchJob) {
        let run_id = self.run_id;
        self.update(|session| session.on_query_planned(run_id, job));
    }

    fn on_batch(&mut self, job_id: u64, progress: BatchProgress) {
        let run_id = self.run_id;
        self.update_progress(|session| session.on_query_batch(run_id, job_id, progress));
    }

    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        if let Some(failure) = &outcome.failure {
            // Safe detail only: built from allow-listed fields and redacted.
            eprintln!(
                "local-sights: query fetch {} stream failed: {}",
                job.job_id,
                failure.safe_detail()
            );
        }
        let run_id = self.run_id;
        self.update(|session| session.on_query_stream_finished(run_id, job));
    }

    fn on_saving(&mut self, job_id: u64) {
        let run_id = self.run_id;
        self.update(|session| session.on_query_saving(run_id, job_id));
    }

    /// Never reached: the runner holds the fetch's end back and reports it
    /// with [`QuerySink::on_fetch_ended`].
    fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {}
}

impl QuerySink for TauriQuerySink {
    fn on_reading_cache(&mut self, run_id: u64) {
        self.update(|session| session.on_query_reading_cache(run_id));
    }

    fn on_group_started(&mut self, run_id: u64, index: usize, count: usize) {
        self.update(|session| session.on_query_group_started(run_id, index, count));
    }

    fn on_range_fetched(&mut self, run_id: u64, _log_group_name: &str, job: &FetchJob) {
        if job.status == JobStatus::Failed {
            // Safe detail only: built from allow-listed fields and redacted.
            let detail = job.failure().map_or("", |failure| failure.safe_detail());
            eprintln!("local-sights: query fetch {} failed: {detail}", job.job_id);
        }
        self.update(|session| session.on_query_range_fetched(run_id, job));
    }

    fn on_fetch_ended(&mut self, run_id: u64, summary: &QueryRunSummary) -> bool {
        self.update(|session| session.finish_query_fetch(run_id, summary))
    }

    fn on_evaluation_progress(&mut self, run_id: u64, evaluated: usize, total: usize) -> bool {
        self.update_progress(|session| {
            session.on_query_evaluation_progress(run_id, evaluated, total)
        })
    }

    fn on_evaluated(&mut self, result: QueryResult) {
        self.update(|session| session.finish_query_evaluation(result));
    }

    fn on_evaluation_stopped(&mut self, run_id: u64) {
        self.update(|session| session.stop_query_evaluation(run_id));
    }

    /// The names change no view; they are read by `get_completion_candidates`.
    fn on_field_names(&mut self, run_id: u64, names: Vec<String>) {
        lock(&self.session).set_query_field_names(run_id, names);
    }

    fn on_evaluation_failed(&mut self, run_id: u64) {
        self.update(|session| session.fail_query_evaluation(run_id));
    }
}

/// Starts the desktop app.
///
/// The app is built first and then run with a callback, so that ending the
/// app (`RunEvent::ExitRequested`: `app.exit`, the last window closing) can
/// be stopped like closing the window (U7:BR3.1, review R-02). On macOS the
/// app menu's Quit (Cmd+Q) is this app's own item, routed through the same
/// decision (code generation review R-01); the Dock menu's Quit cannot be
/// stopped (known limitation).
///
/// # Errors
/// Returns the Tauri error when the app cannot be built.
pub fn run() -> Result<(), tauri::Error> {
    let builder = tauri::Builder::default();
    // Review R-01: Quit (Cmd+Q) of the macOS app menu is this app's own item.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(app_menu);
    let app = builder
        .setup(|app| {
            // U6:BR1.6: the folders come from the OS, so the state is made
            // here, where the app's paths are known.
            let location = cache_location(app.handle());
            app.manage(AppState::load(location));
            // Gate fix 14c: the session starts on the SDK default profile;
            // when the SDK has a default region, its log group listing has
            // already started and is run here like any other listing.
            let state = app.state::<AppState>();
            let startup_listing = lock(&state.session).startup_listing();
            if let Some(request) = startup_listing {
                start_listing(app.handle(), &state, request);
            }
            // Review R-03: a write that outlived its fetch tells the screen.
            watch_cache_writes(app.handle());
            Ok(())
        })
        .on_menu_event(handle_menu_event)
        .on_window_event(|window, event| match event {
            // U7:BR3.1 (1): stopped while fetching or writing the cache; the
            // fetch is no longer aborted here (BR3.2 does that on [Close]).
            WindowEvent::CloseRequested { api, .. } => {
                if must_stop_closing(window.app_handle()) {
                    api.prevent_close();
                }
            }
            // U7:BR3.1 (3): the window is gone; stop a fetch just in case,
            // and a running SSO login's CLI (261011-sso-login BR3.3).
            WindowEvent::Destroyed => {
                if let Some(state) = window.try_state::<AppState>() {
                    state.abort_fetch();
                    state.stop_sso_cli();
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
            row_positions,
            set_failure_list_open,
            select_time_zone,
            set_log_filter,
            open_settings,
            cancel_settings,
            save_settings,
            clear_cache,
            cancel_close,
            confirm_close,
            start_sso_login,
            cancel_sso_login,
            dismiss_sso_login,
            open_sso_url,
            cancel_fetch,
            update_analytics_input,
            run_query,
            cancel_query,
            get_query_rows,
            get_query_row_detail,
            get_completion_candidates
        ])
        .build(tauri::generate_context!())?;
    app.run(|app, event| match event {
        // U7:BR3.1 (2): `app.exit` and the last window closing, stopped like
        // the window (Cmd+Q comes through `handle_menu_event`, review R-01).
        // When the exit goes ahead, a running SSO login's CLI is stopped
        // first (261011-sso-login BR3.3); a login never stops the exit.
        RunEvent::ExitRequested { api, .. } => {
            if must_stop_closing(app) {
                api.prevent_exit();
            } else if let Some(state) = app.try_state::<AppState>() {
                state.stop_sso_cli();
            }
        }
        // The app ends (also the Dock menu's Quit): the last chance.
        RunEvent::Exit => {
            if let Some(state) = app.try_state::<AppState>() {
                state.stop_sso_cli();
            }
        }
        _ => {}
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    //! 261012-parallel-fetch U5: the stop handle registered with the start
    //! (NFR6.17), [Cancel] and the order of the views (NFR6.16), and the
    //! three lists that grant a command (infrastructure-design review R-01).

    use super::*;
    use local_sights_core::catalog::{ConnectionCatalog, ConnectionProfile, ProfileKind};

    /// A session on `dev` with `/aws/lambda/orders` selected and a valid
    /// range: [Fetch] can be pressed.
    fn ready_session() -> Mutex<AppSession> {
        let catalog = ConnectionCatalog::new(
            vec![
                ConnectionProfile {
                    kind: ProfileKind::SdkDefault,
                    profile_name: None,
                    default_region: None,
                },
                ConnectionProfile {
                    kind: ProfileKind::Named,
                    profile_name: Some("dev".to_string()),
                    default_region: Some("ap-northeast-1".to_string()),
                },
            ],
            Vec::new(),
        );
        let mut session = AppSession::with_catalog(catalog);
        let listing = session
            .select_profile(ProfileSelector::Named("dev".to_string()))
            .unwrap()
            .listing
            .unwrap();
        session.apply_listing_page(
            &listing.listing_id,
            vec![LogGroup {
                log_group_name: "/aws/lambda/orders".to_string(),
                arn: None,
                creation_time: None,
            }],
            None,
        );
        let generation = session.connection_generation();
        session
            .select_log_group("/aws/lambda/orders".to_string(), generation)
            .unwrap();
        session
            .update_input(InputField::StartText, "2024/01/02 03:04:05".to_string())
            .unwrap();
        session
            .update_input(InputField::EndText, "2024/01/02 03:05:05".to_string())
            .unwrap();
        Mutex::new(session)
    }

    fn generation(session: &Mutex<AppSession>) -> u64 {
        lock(session).connection_generation()
    }

    #[test]
    fn the_start_registers_the_new_handle_so_an_immediate_cancel_reaches_it() {
        let session = ready_session();
        let (old_handle, old_signal) = stop_pair();
        let abort = Mutex::new(Some((0, old_handle)));

        let begun = begin_registered_fetch(&session, &abort, generation(&session)).unwrap();
        assert_eq!(begun.view.phase, Phase::Fetching);
        assert!(begun.view.fetch_cancel_available);
        assert_eq!(
            lock(&abort).as_ref().map(|(number, _)| *number),
            Some(begun.fetch_number),
            "registered before the session lock was released"
        );

        let mut sent = Vec::new();
        assert!(cancel_running_fetch(&session, &abort, |view| sent.push(view)));
        assert_eq!(begun.signal.reason(), Some(StopReason::Cancel));
        assert_eq!(old_signal.reason(), None, "the previous fetch is not told");
        assert_eq!(sent.len(), 1);
        assert!(sent[0].cancelling);
        assert!(!sent[0].fetch_cancel_available);
    }

    #[test]
    fn a_refused_start_registers_nothing() {
        let session = ready_session();
        let abort = Mutex::new(None);
        let stale = generation(&session) + 1;
        assert!(begin_registered_fetch(&session, &abort, stale).is_err());
        assert!(lock(&abort).is_none());
    }

    #[test]
    fn a_cancel_without_a_running_fetch_does_nothing_and_sends_nothing() {
        let session = ready_session();
        let abort = Mutex::new(None);
        let mut sent = Vec::new();
        assert!(!cancel_running_fetch(&session, &abort, |view| sent.push(view)));
        assert!(sent.is_empty());
        assert!(!lock(&session).view().cancelling);
    }

    #[test]
    fn a_second_cancel_does_nothing() {
        let session = ready_session();
        let abort = Mutex::new(None);
        let begun = begin_registered_fetch(&session, &abort, generation(&session)).unwrap();
        let mut sent = Vec::new();
        assert!(cancel_running_fetch(&session, &abort, |view| sent.push(view)));
        assert!(!cancel_running_fetch(&session, &abort, |view| sent.push(view)));
        assert_eq!(sent.len(), 1, "nothing sent for the second press");
        assert_eq!(begun.signal.reason(), Some(StopReason::Cancel));
    }

    #[test]
    fn a_handle_of_another_fetch_is_not_told_to_stop() {
        let session = ready_session();
        let abort = Mutex::new(None);
        let begun = begin_registered_fetch(&session, &abort, generation(&session)).unwrap();
        let (other_handle, other_signal) = stop_pair();
        *lock(&abort) = Some((begun.fetch_number + 1, other_handle));
        assert!(cancel_running_fetch(&session, &abort, |_| {}));
        assert_eq!(other_signal.reason(), None);
        assert_eq!(begun.signal.reason(), None);
    }

    #[test]
    fn the_end_of_a_cancelled_fetch_is_the_last_view_sent() {
        // NFR6.16: the fetch ends right after the stop; its view comes last
        // and turns [Cancelling…] off.
        let session = ready_session();
        let log_view = Mutex::new(LogView::new());
        let abort = Mutex::new(None);
        let begun = begin_registered_fetch(&session, &abort, generation(&session)).unwrap();
        let mut job = FetchJob::start(begun.validated.range);
        let mut sent = vec![change_and_view(&session, &log_view, |s| {
            s.on_job_started(&job, 1);
        })];
        cancel_running_fetch(&session, &abort, |view| sent.push(view));
        job.status = JobStatus::Cancelled;
        job.unfinished_stream_count = 4;
        sent.push(change_and_view(&session, &log_view, |s| {
            s.finish_fetch(&job, 1);
        }));

        let cancelling: Vec<bool> = sent.iter().map(|view| view.cancelling).collect();
        assert_eq!(cancelling, [false, true, false]);
        let last = sent.last().unwrap();
        assert_eq!(last.phase, Phase::Done);
        assert!(!last.fetch_cancel_available);
        assert!(last.cancel_notice.is_some());
    }

    /// The quoted names of the first `.commands(&[ … ])` block.
    fn manifest_commands(build_rs: &str) -> Vec<String> {
        let start = build_rs.find(".commands(&[").unwrap();
        let block = &build_rs[start..];
        let block = &block[..block.find("])").unwrap()];
        block
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    }

    /// The names inside the first invoke-handler macro of `run`.
    fn handler_commands(lib_rs: &str) -> Vec<String> {
        let marker = concat!("generate_handler", "![");
        let start = lib_rs.find(marker).unwrap() + marker.len();
        let block = &lib_rs[start..];
        let block = &block[..block.find(']').unwrap()];
        block
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// The commands the window may invoke: `allow-<kebab-name>` entries.
    fn allowed_commands(capability: &str) -> Vec<String> {
        let json: serde_json::Value = serde_json::from_str(capability).unwrap();
        json["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|permission| permission.as_str()?.strip_prefix("allow-"))
            .map(|name| name.replace('-', "_"))
            .collect()
    }

    fn sorted(mut names: Vec<String>) -> Vec<String> {
        names.sort();
        names
    }

    /// 261020-carryover-fixes FR4.3: whether the three lists of granted
    /// commands are all non-empty and name exactly the same commands, in any
    /// order; `Err` says which differs. How many commands there are is not
    /// fixed, so adding one in all three places needs no change here.
    fn same_commands(
        manifest: &[String],
        handler: &[String],
        allowed: &[String],
    ) -> Result<(), String> {
        let lists = [
            ("build.rs", manifest),
            ("generate_handler", handler),
            ("the capability", allowed),
        ];
        if let Some((name, _)) = lists.iter().find(|(_, list)| list.is_empty()) {
            return Err(format!("{name} lists no command"));
        }
        let expected = sorted(manifest.to_vec());
        for (name, list) in &lists[1..] {
            let actual = sorted(list.to_vec());
            if actual != expected {
                return Err(format!(
                    "{name} lists {actual:?}, build.rs lists {expected:?}"
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn the_manifest_the_capability_and_the_handler_list_the_same_commands() {
        // Infrastructure-design review R-01 (NFR8.15): a command is granted
        // in three places, and they must name exactly the same commands.
        let manifest = manifest_commands(include_str!("../build.rs"));
        let handler = handler_commands(include_str!("lib.rs"));
        let allowed = allowed_commands(include_str!("../capabilities/default.json"));
        assert_eq!(same_commands(&manifest, &handler, &allowed), Ok(()));
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn same_commands_fails_when_one_place_has_a_command_more() {
        let two = names(&["cancel_fetch", "start_fetch"]);
        let three = names(&["cancel_fetch", "new_command", "start_fetch"]);
        assert!(same_commands(&two, &two, &three).is_err(), "the capability");
        assert!(same_commands(&two, &three, &two).is_err(), "the handler");
        assert!(same_commands(&three, &two, &two).is_err(), "build.rs");
        let reordered = names(&["start_fetch", "cancel_fetch"]);
        assert_eq!(same_commands(&two, &reordered, &two), Ok(()), "any order");
    }

    #[test]
    fn same_commands_fails_when_one_place_lacks_a_command() {
        let two = names(&["cancel_fetch", "start_fetch"]);
        let one = names(&["start_fetch"]);
        assert!(same_commands(&two, &one, &two).is_err());
        assert!(same_commands(&two, &two, &one).is_err());
        assert!(same_commands(&one, &two, &two).is_err());
    }

    #[test]
    fn same_commands_fails_when_a_list_is_empty() {
        let none: Vec<String> = Vec::new();
        let one = names(&["start_fetch"]);
        assert!(same_commands(&none, &none, &none).is_err(), "all empty");
        assert!(same_commands(&one, &one, &none).is_err());
        assert_eq!(same_commands(&one, &one, &one), Ok(()));
    }

    // ---- 261017-insights-query U1: the Log Analytics commands -----------

    use local_sights_core::coordinator::TimelineStore;
    use local_sights_core::event::LogEvent;
    use local_sights_core::insights_session::AnalyticsPhase;
    use local_sights_core::session::Phase;

    #[test]
    fn generate_handler_build_rs_and_the_capability_list_the_query_commands() {
        // Infrastructure-design review R-01: each new command in all three.
        let manifest = manifest_commands(include_str!("../build.rs"));
        let handler = handler_commands(include_str!("lib.rs"));
        let allowed = allowed_commands(include_str!("../capabilities/default.json"));
        for command in [
            "update_analytics_input",
            "run_query",
            "cancel_query",
            "get_query_rows",
            "get_query_row_detail",
            "get_completion_candidates",
        ] {
            let command = command.to_string();
            assert!(manifest.contains(&command), "{command} in build.rs");
            assert!(handler.contains(&command), "{command} in generate_handler");
            assert!(allowed.contains(&command), "{command} in the capability");
        }
    }

    /// [`ready_session`] with the query screen ready to run on
    /// `/aws/lambda/orders`.
    fn query_ready_session() -> Mutex<AppSession> {
        let session = ready_session();
        {
            let mut locked = lock(&session);
            let generation = locked.connection_generation();
            for (field, value) in [
                (AnalyticsField::LogGroup, "/aws/lambda/orders"),
                (AnalyticsField::StartText, "2024/01/02 03:04:05"),
                (AnalyticsField::EndText, "2024/01/02 03:05:05"),
            ] {
                locked
                    .update_analytics_input(field, value.to_string(), generation)
                    .unwrap();
            }
        }
        session
    }

    fn query_begun(
        session: &Mutex<AppSession>,
        abort: &Mutex<Option<(u64, StopHandle)>>,
        text: &str,
    ) -> BegunQuery {
        let (snapshot, zone) = lock(session).query_snapshot();
        assert_eq!(
            snapshot.text,
            local_sights_core::insights_session::DEFAULT_QUERY
        );
        let parsed = parse_query(text).and_then(|plan| resolve_date_texts(plan, zone));
        begin_registered_query(
            session,
            abort,
            snapshot.revision,
            parsed.as_ref().map_err(Clone::clone),
        )
        .unwrap()
    }

    #[test]
    fn query_start_registers_the_handle_so_an_immediate_cancel_reaches_it() {
        // NFR6.17 for the query run; UC4, R-01.
        let session = query_ready_session();
        let abort = Mutex::new(None);
        let begun = query_begun(&session, &abort, "filter @message like /ERROR/");
        let start = begun.start.expect("the run starts");
        assert_eq!(begun.view.insights.phase, AnalyticsPhase::Fetching);
        assert_eq!(begun.view.fetch_owner, FetchOwner::LogAnalytics);
        assert!(begun.view.insights.cancel_available);
        assert_eq!(
            lock(&abort).as_ref().map(|(number, _)| *number),
            Some(start.run_id)
        );
        let mut sent = Vec::new();
        assert!(cancel_running_query(&session, &abort, |view| sent.push(view)));
        assert_eq!(
            begun.signal.as_ref().and_then(StopSignal::reason),
            Some(StopReason::Cancel)
        );
        assert_eq!(sent.len(), 1);
        assert!(sent[0].insights.cancelling);
        assert!(!sent[0].insights.cancel_available);
        assert!(!cancel_running_query(&session, &abort, |view| sent.push(view)));
        assert_eq!(sent.len(), 1, "a second press sends nothing");
    }

    #[test]
    fn query_with_a_reading_problem_starts_nothing_and_registers_no_handle() {
        // U1:BR3.9: QueryError, no fetch, no handle; the text stays.
        let session = query_ready_session();
        let abort = Mutex::new(None);
        let begun = query_begun(
            &session,
            &abort,
            "filter @message like /ERROR/ | stats count(*)",
        );
        assert!(begun.start.is_none() && begun.signal.is_none());
        assert!(lock(&abort).is_none());
        let insights = &begun.view.insights;
        assert_eq!(insights.phase, AnalyticsPhase::QueryError);
        assert_eq!(
            insights.problem.as_ref().map(|problem| problem.message_key),
            Some("query.unsupported.command")
        );
        assert_eq!(begun.view.fetch_owner, FetchOwner::None);
        let refused = {
            let mut locked = lock(&session);
            let generation = locked.connection_generation();
            locked.begin_fetch(generation).map(|_| ())
        };
        assert_eq!(refused, Ok(()), "Log Management may fetch: nothing runs");
    }

    #[test]
    fn query_cancel_without_a_run_does_nothing_and_sends_nothing() {
        let session = query_ready_session();
        let abort = Mutex::new(None);
        let mut sent = Vec::new();
        assert!(!cancel_running_query(&session, &abort, |view| sent.push(view)));
        assert!(sent.is_empty());
        assert_eq!(lock(&session).phase(), Phase::Idle);
    }

    fn held(timestamp: i64, stream: &str, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: format!("ERROR {stream} {sequence}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    /// The end of a query run's fetch of `/aws/lambda/orders` (U3:BR2.5).
    fn completed_fetch() -> QueryRunSummary {
        use local_sights_core::query::runner::{GroupFetchOutcome, GroupFetchStatus};
        QueryRunSummary {
            groups: vec![GroupFetchOutcome {
                log_group_name: "/aws/lambda/orders".to_string(),
                status: GroupFetchStatus::Completed,
                failed_streams: Vec::new(),
                failure: None,
            }],
            partial: false,
            from_aws: true,
            stopped: false,
            closed: false,
        }
    }

    /// Copies of `events` as result rows of `/aws/lambda/orders`.
    fn copied(events: Vec<LogEvent>) -> Vec<ResultRow> {
        events
            .into_iter()
            .map(|event| ResultRow {
                event,
                log_group_name: "/aws/lambda/orders".to_string(),
            })
            .collect()
    }

    #[test]
    fn query_rows_are_read_from_the_result_only_for_the_shown_run() {
        // U1:BR6.3, U3:BR3.2: the copies, checked by the run number; the
        // QueryStore is not needed.
        let session = query_ready_session();
        let abort = Mutex::new(None);
        let start = query_begun(&session, &abort, "filter @message like 'ERROR'")
            .start
            .unwrap();
        {
            let mut locked = lock(&session);
            assert!(locked.finish_query_fetch(start.run_id, &completed_fetch()));
            locked.finish_query_evaluation(QueryResult::new(
                start.run_id,
                2,
                copied(vec![held(1_704_164_647_000, "b", 0)]),
            ));
        }
        let window = query_rows(&session, start.run_id, (0, 50));
        assert_eq!(window.total_count, 1);
        assert_eq!(window.rows.len(), 1);
        let row = &window.rows[0];
        assert_eq!(row.event.log_stream_name, "b");
        assert_eq!(row.log_group_name, "/aws/lambda/orders");
        assert_eq!(row.display_time, "2024/01/02 03:04:07.000", "UTC zone");
        let json = serde_json::to_value(&window).unwrap();
        assert_eq!(json["rows"][0]["logGroupName"], "/aws/lambda/orders");
        assert_eq!(json["rows"][0]["displayTime"], "2024/01/02 03:04:07.000");
        assert_eq!(json["rows"][0]["message"], "ERROR b 0");
        assert_eq!(json["runId"], start.run_id);
        assert!(
            json.get("holdingsEpoch").is_none(),
            "no holdings epoch is sent (261020-carryover-fixes FR2.3)"
        );
        assert!(
            query_rows(&session, start.run_id + 1, (0, 50))
                .rows
                .is_empty(),
            "another run"
        );
    }

    // ---- 261017-insights-query U2 ---------------------------------------

    use local_sights_core::query::evaluator::{EvalOutput, ResultRow};
    use local_sights_core::query::plan::{FieldRef, JsonPath};

    /// A query run of `session` that fetched `events` and is evaluating;
    /// returns the run number and the events.
    fn query_evaluating(
        session: &Mutex<AppSession>,
        events: Vec<LogEvent>,
    ) -> (u64, Vec<LogEvent>) {
        let abort = Mutex::new(None);
        let start = query_begun(session, &abort, "filter @message like 'ERROR'")
            .start
            .unwrap();
        assert!(lock(session).finish_query_fetch(start.run_id, &completed_fetch()));
        (start.run_id, events)
    }

    fn json_event(timestamp: i64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: "s".to_string(),
            sequence: 0,
        }
    }

    /// Ends the evaluation of `run_id` with the columns @timestamp and
    /// `level` over the rows at `rows` of `events`, copied (U3:BR3.2).
    fn finish_with_level_column(
        session: &Mutex<AppSession>,
        (run_id, events): (u64, Vec<LogEvent>),
        rows: Vec<usize>,
    ) {
        let copies = copied(rows.iter().map(|row| events[*row].clone()).collect());
        lock(session).finish_query_evaluation(QueryResult::from_output(
            run_id,
            EvalOutput {
                matched_count: rows.len(),
                rows,
                columns: vec![
                    FieldRef::Timestamp,
                    FieldRef::Json(JsonPath::new(vec!["level".to_string()], "level")),
                ],
                truncated_at: None,
                evaluated_count: 2,
            },
            copies,
        ));
    }

    #[test]
    fn query_rows_carry_one_cell_per_result_column() {
        // U2:BR7.1, BR7.4: the cells are read with the rows.
        let session = query_ready_session();
        let evaluating = query_evaluating(
            &session,
            vec![
                json_event(1_704_164_646_000, r#"{"level":"error"}"#),
                json_event(1_704_164_647_000, "plain"),
            ],
        );
        let run_id = evaluating.0;
        finish_with_level_column(&session, evaluating, vec![1, 0]);
        let window = query_rows(&session, run_id, (0, 50));
        let json = serde_json::to_value(&window).unwrap();
        assert_eq!(json["rows"][0]["values"][0], "2024/01/02 03:04:07.000");
        assert!(
            json["rows"][0]["values"][1].is_null(),
            "no level: an empty cell"
        );
        assert_eq!(json["rows"][1]["values"][1], "error");
        let summary = serde_json::to_value(&lock(&session).view().insights.result).unwrap();
        assert_eq!(
            summary["columns"],
            serde_json::json!(["@timestamp", "level"])
        );
        assert_eq!(summary["matchedCount"], 2);
    }

    #[test]
    fn an_opened_row_lists_its_fields_only_for_the_shown_result() {
        // U2:BR7.3.
        let session = query_ready_session();
        let evaluating = query_evaluating(
            &session,
            vec![json_event(
                1_704_164_646_000,
                r#"{"level":"error","http":{"status":500}}"#,
            )],
        );
        let run_id = evaluating.0;
        finish_with_level_column(&session, evaluating, vec![0]);
        let detail = query_row_detail(&session, run_id, 0);
        let json = serde_json::to_value(&detail).unwrap();
        let names: Vec<&str> = json["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "@timestamp",
                "@log",
                "@logStream",
                "@message",
                "http.status",
                "level"
            ]
        );
        assert_eq!(json["fields"][4]["value"], "500");
        assert_eq!(json["runId"], run_id);
        for (run, index) in [(run_id + 1, 0), (run_id, 1)] {
            assert!(
                query_row_detail(&session, run, index).fields.is_empty(),
                "{run} {index}"
            );
        }
    }

    #[test]
    fn a_query_task_lost_while_evaluating_is_an_evaluation_failure() {
        // U2:BR10.2 (U1 code review R-03): the recovery's failure names
        // GetLogEvents, but an evaluating run is not said to have failed to
        // fetch.
        let session = query_ready_session();
        let (run_id, _) = query_evaluating(&session, vec![json_event(1, "ERROR")]);
        let failure = ApiFailure::new(
            FailureKind::Other,
            &SafeDetailFields {
                api_name: Some(GET_LOG_EVENTS.to_string()),
                ..SafeDetailFields::default()
            },
        );
        assert!(lock(&session).abort_query_with_failure(run_id, failure));
        let insights = lock(&session).view().insights;
        assert_eq!(insights.phase, AnalyticsPhase::EvaluationFailed);
        assert_eq!(insights.failure, None);
        let json = serde_json::to_value(&insights).unwrap();
        assert_eq!(json["phase"], "EvaluationFailed");
    }

    #[test]
    fn completion_candidates_add_the_field_names_of_the_last_fetch() {
        // U2:BR1.7, BR9.1.
        let session = query_ready_session();
        let before = lock(&session).query_completion_candidates();
        assert!(before.iter().any(|candidate| candidate.text == "filter"));
        let (run_id, _) = query_evaluating(&session, vec![json_event(1, "ERROR")]);
        lock(&session).set_query_field_names(run_id, vec!["http.status".to_string()]);
        let after = serde_json::to_value(lock(&session).query_completion_candidates()).unwrap();
        let last = after.as_array().unwrap().last().unwrap();
        assert_eq!(last["text"], "http.status");
        assert_eq!(last["kind"], "JsonField");
    }

    #[test]
    fn query_holdings_go_with_the_log_management_logs_on_a_connection_change() {
        // U1:BR7.2: one discard for both screens.
        let session = query_ready_session();
        let log_view = Mutex::new(LogView::new());
        log_view.add(vec![held(1, "lm", 0)]);
        let store = QueryStore::new();
        let key = local_sights_core::query::store::HoldingKey {
            profile: ProfileSelector::Named("dev".to_string()),
            region: "ap-northeast-1".to_string(),
            log_group_name: "/aws/lambda/orders".to_string(),
        };
        store.begin_run(vec![key.clone()]);
        let range = local_sights_core::cache::plan::CoveredRange {
            start_ms: 0,
            end_ms: 10,
        };
        store.settle_range(&key, range, vec![held(2, "q", 0)], true, 1_000_000_000);
        let epoch = store.epoch();
        clear_held_logs(&mut lock(&session), &log_view, &store);
        assert!(store.is_empty());
        assert!(store.covered_ranges(&key).is_empty(), "U3:BR1.6");
        assert_eq!(store.epoch(), epoch + 1);
        assert_eq!(lock(&log_view).rows(0, 10).all_count, 0);
    }

    // ---- 261017-insights-query U3:BR4.1 ---------------------------------

    #[test]
    fn the_log_groups_arrive_as_a_list_and_the_other_fields_as_texts() {
        // The shapes `update_analytics_input` takes from the screen.
        let names: AnalyticsValue =
            serde_json::from_str(r#"["/aws/lambda/orders","/aws/lambda/MyFunction"]"#).unwrap();
        assert_eq!(
            names,
            AnalyticsValue::Names(vec![
                "/aws/lambda/orders".to_string(),
                "/aws/lambda/MyFunction".to_string()
            ])
        );
        let text: AnalyticsValue = serde_json::from_str(r#""filter @log = 'x'""#).unwrap();
        assert_eq!(text, AnalyticsValue::Text("filter @log = 'x'".to_string()));
        // The test session lists `/aws/lambda/orders` only.
        let session = query_ready_session();
        {
            let mut locked = lock(&session);
            let generation = locked.connection_generation();
            assert_eq!(
                locked.update_analytics_input(AnalyticsField::LogGroup, names, generation),
                Err(SessionError::UnknownLogGroup),
                "a name not listed is refused"
            );
            let listed: AnalyticsValue = serde_json::from_str(r#"["/aws/lambda/orders"]"#).unwrap();
            locked
                .update_analytics_input(AnalyticsField::LogGroup, listed, generation)
                .unwrap();
        }
        let json = serde_json::to_value(lock(&session).view().insights).unwrap();
        assert_eq!(
            json["selectedLogGroupNames"],
            serde_json::json!(["/aws/lambda/orders"])
        );
        assert_eq!(json["maxLogGroups"], 50);
        assert_eq!(json["progress"], serde_json::Value::Null);
        assert_eq!(json["failedGroups"], serde_json::json!([]));
    }
}
