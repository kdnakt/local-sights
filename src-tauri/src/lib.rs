//! Tauri wiring of local-sights (DesktopUi glue, ADR-001).
//!
//! This crate stays thin: every decision lives in `local-sights-core`.
//! Operations arrive as Tauri commands (`get_session`, `update_input`,
//! `start_fetch`); state changes are pushed to the screen with the
//! `session-changed` event and each fetched page with the `log-batch` event.
//! Diagnostics go to standard error only and contain only safe details:
//! no secret credential and no access key ID is ever logged or shown.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use local_sights_core::coordinator::{FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::session::{AppSession, InputField, SessionError, SessionView};
use local_sights_core::timeline::EventTimeline;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// Event carrying the latest [`SessionView`].
pub const SESSION_CHANGED: &str = "session-changed";
/// Event carrying one page of fetched log events.
pub const LOG_BATCH: &str = "log-batch";

/// Shared state of the app.
#[derive(Debug, Default)]
struct AppState {
    session: Arc<Mutex<AppSession>>,
    timeline: Arc<tokio::sync::Mutex<EventTimeline>>,
    gateway: Arc<AwsCloudWatchLogsGateway>,
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

/// Changes one input field; refused while fetching (BR1.4).
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

/// Starts fetching with the validated conditions. Progress arrives through
/// the `session-changed` and `log-batch` events.
#[tauri::command]
fn start_fetch(app: AppHandle, state: State<'_, AppState>) -> Result<SessionView, CommandError> {
    let (validated, view) = {
        let mut session = lock_session(&state.session);
        let validated = session.begin_fetch()?;
        (validated, session.view())
    };
    emit_or_log(&app, SESSION_CHANGED, view.clone());

    let session = Arc::clone(&state.session);
    let timeline = Arc::clone(&state.timeline);
    let gateway = Arc::clone(&state.gateway);
    tauri::async_runtime::spawn(async move {
        let mut timeline = timeline.lock().await;
        let mut sink = TauriSink { app, session };
        run_fetch(gateway.as_ref(), &validated, &mut timeline, &mut sink).await;
    });
    Ok(view)
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
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_session,
            update_input,
            start_fetch
        ])
        .run(tauri::generate_context!())
}
