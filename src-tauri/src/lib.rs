//! Tauri wiring of local-sights (DesktopUi glue, ADR-001).
//!
//! This crate stays thin: every decision lives in `local-sights-core`.
//! Operations arrive as Tauri commands (`get_session`, `update_input`,
//! `start_fetch` and, since U2, the connection and log group commands);
//! state changes, including the log group list, are pushed to the screen
//! with the `session-changed` event and each fetched page of log events
//! with the `log-batch` event. Commands that change state return nothing:
//! their result arrives as an event, so a late response never overwrites
//! newer state.
//! Diagnostics go to standard error only and contain only safe details:
//! no secret credential and no access key ID is ever logged or shown.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::catalog::files::load_catalog;
use local_sights_core::coordinator::{FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::gateway::{DESCRIBE_LOG_GROUPS, GET_LOG_EVENTS};
use local_sights_core::log_groups::listing::{ListingRequest, ListingSink, run_listing};
use local_sights_core::log_groups::{ListingStatus, LogGroup};
use local_sights_core::paging::PageDecision;
use local_sights_core::session::{
    AppSession, ConnectionEffect, InputField, Phase, SessionError, SessionView,
};
use local_sights_core::timeline::EventTimeline;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// Event carrying the latest [`SessionView`].
pub const SESSION_CHANGED: &str = "session-changed";
/// Event carrying one page of fetched log events.
pub const LOG_BATCH: &str = "log-batch";

/// Shared state of the app.
#[derive(Debug)]
struct AppState {
    session: Arc<Mutex<AppSession>>,
    timeline: Arc<tokio::sync::Mutex<EventTimeline>>,
    gateway: Arc<AwsCloudWatchLogsGateway>,
}

impl AppState {
    /// Reads the shared AWS config files once at startup (U2:BR1.1-BR1.3).
    fn load() -> Self {
        Self {
            session: Arc::new(Mutex::new(AppSession::with_catalog(load_catalog()))),
            timeline: Arc::default(),
            gateway: Arc::default(),
        }
    }
}

/// Payload of [`LOG_BATCH`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogBatchPayload<'a> {
    job_id: &'a str,
    events: &'a [LogEvent],
    total: u64,
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

/// Locks the session. A poisoned lock still holds a consistent session
/// (every update is a single assignment), so it is recovered, not fatal.
fn lock_session(session: &Mutex<AppSession>) -> MutexGuard<'_, AppSession> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}

fn emit_or_log<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Err(error) = app.emit(event, payload) {
        eprintln!("local-sights: failed to emit {event}: {error}");
    }
}

/// Returns the current session state.
#[tauri::command]
fn get_session(state: State<'_, AppState>) -> SessionView {
    lock_session(&state.session).view()
}

/// Changes the stream name or a time; refused while fetching (U1:BR1.4).
#[tauri::command]
fn update_input(
    field: InputField,
    value: String,
    state: State<'_, AppState>,
) -> Result<SessionView, CommandError> {
    let mut session = lock_session(&state.session);
    session.update_input(field, value)?;
    Ok(session.view())
}

/// Starts fetching with the validated conditions. The command returns
/// nothing: the new state (Fetching), the job, each page and the result all
/// arrive through the `session-changed` and `log-batch` events, in order, so
/// a late command response can never overwrite newer state.
#[tauri::command]
fn start_fetch(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    let (validated, view) = {
        let mut session = lock_session(&state.session);
        let validated = session.begin_fetch()?;
        (validated, session.view())
    };
    emit_or_log(&app, SESSION_CHANGED, view);

    let session = Arc::clone(&state.session);
    let timeline = Arc::clone(&state.timeline);
    let gateway = Arc::clone(&state.gateway);
    let task_app = app.clone();
    let task_session = Arc::clone(&session);
    let task = tauri::async_runtime::spawn(async move {
        let mut timeline = timeline.lock().await;
        let mut sink = TauriSink {
            app: task_app,
            session: task_session,
        };
        run_fetch(gateway.as_ref(), &validated, &mut timeline, &mut sink).await;
    });
    tauri::async_runtime::spawn(async move {
        let ended_normally = task.await.is_ok();
        recover_if_still_fetching(&app, &session, ended_normally);
    });
    Ok(())
}

/// Runs after the fetch task ends. If it ended (normally or not) without
/// reporting a result, the session would stay in Fetching forever; move it
/// to Failed (kind Other) and tell the screen. The panic message is not
/// logged: only a safe detail is.
fn recover_if_still_fetching(app: &AppHandle, session: &Mutex<AppSession>, ended_normally: bool) {
    let failure = ApiFailure::new(
        FailureKind::Other,
        &SafeDetailFields {
            api_name: Some(GET_LOG_EVENTS.to_string()),
            ..SafeDetailFields::default()
        },
    );
    let view = {
        let mut session = lock_session(session);
        if !session.abort_fetch_with_failure(failure) {
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
/// is cleared inside this command, while its lock is held across the session
/// change, before `session-changed` is emitted. A new fetch can only start
/// once that is done, so a late clear can never wipe the logs of a newer
/// fetch. During a fetch the operations that discard logs are refused, so
/// the lock (held by the running fetch) is not awaited then.
async fn change_connection(
    app: &AppHandle,
    state: &AppState,
    operation: impl FnOnce(&mut AppSession) -> Result<ConnectionEffect, SessionError> + Send,
) -> Result<(), CommandError> {
    let fetching = lock_session(&state.session).phase() == Phase::Fetching;
    let mut timeline = if fetching {
        None
    } else {
        Some(state.timeline.lock().await)
    };
    let (effect, view) = {
        let mut session = lock_session(&state.session);
        let effect = operation(&mut session)?;
        (effect, session.view())
    };
    if effect.logs_cleared {
        match timeline.as_mut() {
            Some(timeline) => timeline.clear(),
            None => state.timeline.lock().await.clear(),
        }
    }
    drop(timeline);
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
            let mut session = lock_session(&session);
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
async fn select_profile(
    profile: ProfileSelector,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| session.select_profile(profile)).await
}

/// Chooses a region (U2:BR2.3, BR2.5).
#[tauri::command]
async fn select_region(
    region: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| session.select_region(region)).await
}

/// Applies the connection change awaiting confirmation (U2:BR2.5).
#[tauri::command]
async fn confirm_connection_change(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, AppSession::confirm_connection_change).await
}

/// Drops the connection change awaiting confirmation (U2:BR2.5).
#[tauri::command]
async fn cancel_connection_change(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        session.cancel_connection_change()?;
        Ok(ConnectionEffect::default())
    })
    .await
}

/// Lists the log groups of the current connection again (U2:BR3.5).
#[tauri::command]
async fn reload_log_groups(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        let listing = session.reload_log_groups()?;
        Ok(ConnectionEffect {
            listing: Some(listing),
            ..ConnectionEffect::default()
        })
    })
    .await
}

/// Changes the log group filter (U2:BR3.7).
#[tauri::command]
async fn update_log_group_filter(
    text: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        session.update_log_group_filter(text)?;
        Ok(ConnectionEffect::default())
    })
    .await
}

/// Selects one log group (U2:BR3.8).
#[tauri::command]
async fn select_log_group(
    name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    change_connection(&app, &state, |session| {
        session.select_log_group(name)?;
        Ok(ConnectionEffect::default())
    })
    .await
}

/// Applies listing pages to the session and pushes the new state.
struct TauriListingSink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
}

impl ListingSink for TauriListingSink {
    fn is_current(&self, listing_id: &str) -> bool {
        lock_session(&self.session).is_current_listing(listing_id)
    }

    fn on_page(
        &mut self,
        listing_id: &str,
        groups: Vec<LogGroup>,
        next_token: Option<&str>,
    ) -> Option<PageDecision> {
        let (decision, view) = {
            let mut session = lock_session(&self.session);
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
            let mut session = lock_session(&self.session);
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

/// Forwards fetch progress to AppSession and to the screen.
struct TauriSink {
    app: AppHandle,
    session: Arc<Mutex<AppSession>>,
}

impl TauriSink {
    fn emit_session(&self) {
        let view = lock_session(&self.session).view();
        emit_or_log(&self.app, SESSION_CHANGED, view);
    }
}

impl FetchSink for TauriSink {
    fn on_started(&mut self, job: &FetchJob) {
        lock_session(&self.session).on_job_started(job);
        self.emit_session();
    }

    fn on_batch(&mut self, job_id: &str, events: &[LogEvent], total: u64) {
        lock_session(&self.session).on_progress(total);
        emit_or_log(
            &self.app,
            LOG_BATCH,
            LogBatchPayload {
                job_id,
                events,
                total,
            },
        );
    }

    fn on_finished(&mut self, job: &FetchJob) {
        if job.status == JobStatus::Failed {
            // Safe detail only: built from allow-listed fields and redacted.
            let detail = job.failure().map_or("", |failure| failure.safe_detail());
            eprintln!("local-sights: fetch {} failed: {detail}", job.job_id);
        }
        lock_session(&self.session).finish_fetch(job);
        self.emit_session();
    }
}

/// Starts the desktop app.
///
/// # Errors
/// Returns the Tauri error when the app cannot be built or run.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .manage(AppState::load())
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
            select_log_group
        ])
        .run(tauri::generate_context!())
}
