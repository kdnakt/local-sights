//! GUI-independent core of local-sights, a local CloudWatch Logs viewer.
//!
//! The desktop app (`src-tauri`) and the developer-only `fetch_check`
//! example both depend on this crate; the crate itself never depends on a
//! GUI framework (ADR-001). AWS is reached only through the
//! [`gateway::CloudWatchLogsGateway`] boundary, which exposes nothing but the
//! three read-only operations `DescribeLogGroups`, `DescribeLogStreams` and
//! `GetLogEvents` (U3:BR1.4, project.md Forbidden).

#![warn(missing_docs)]

pub mod catalog;
pub mod connection;
pub mod coordinator;
pub mod event;
pub mod failure;
pub mod fetcher;
pub mod gateway;
pub mod log_groups;
pub mod paging;
pub mod request;
pub mod retry;
pub mod session;
pub mod streams;
pub mod time_range;
pub mod timeline;

/// Test-only helpers shared across modules.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    /// Serializes the unit tests that change process environment variables,
    /// so they never run concurrently with each other.
    pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());
}
