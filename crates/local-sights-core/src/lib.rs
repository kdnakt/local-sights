//! GUI-independent core of local-sights, a local CloudWatch Logs viewer.
//!
//! The desktop app (`src-tauri`) and the developer-only `fetch_check`
//! example both depend on this crate; the crate itself never depends on a
//! GUI framework (ADR-001). AWS is reached only through the
//! [`gateway::CloudWatchLogsGateway`] boundary, which exposes nothing but the
//! read-only `GetLogEvents` operation in U1 (BR3.4).

#![warn(missing_docs)]

pub mod coordinator;
pub mod event;
pub mod failure;
pub mod fetcher;
pub mod gateway;
pub mod paging;
pub mod request;
pub mod session;
pub mod time_range;
pub mod timeline;
