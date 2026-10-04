//! CloudWatchLogsGateway: the only boundary through which AWS is reached.
//!
//! In U1 the boundary exposes a single operation, `GetLogEvents` (BR3.4).
//! There is deliberately no way to call any other API through it
//! (project.md Forbidden: read-only APIs only). Tests replace the AWS
//! implementation with a fake and never connect to AWS.

pub mod aws;
pub mod classify;

use std::future::Future;

use serde::Serialize;

use crate::failure::ApiFailure;

/// Name of the only API operation used in U1.
pub const GET_LOG_EVENTS: &str = "GetLogEvents";

/// Arguments of one `GetLogEvents` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLogEventsRequest {
    /// Profile used to resolve the connection target (`None`: SDK default).
    pub profile_name: Option<String>,
    /// Log group name.
    pub log_group_name: String,
    /// Log stream name.
    pub log_stream_name: String,
    /// `startTime`, epoch milliseconds (inclusive).
    pub start_time: i64,
    /// `endTime`, epoch milliseconds (exclusive).
    pub end_time: i64,
    /// `startFromHead`: read oldest-first.
    pub start_from_head: bool,
    /// Forward token from the previous response (`None` on the first call).
    pub next_token: Option<String>,
}

/// One event as returned by the API, before sequencing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayEvent {
    /// Event time, epoch milliseconds.
    pub timestamp: i64,
    /// Ingestion time, epoch milliseconds.
    pub ingestion_time: Option<i64>,
    /// Message as returned.
    pub message: String,
}

/// One page of `GetLogEvents`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetLogEventsPage {
    /// Events in the order the API returned them.
    pub events: Vec<GatewayEvent>,
    /// `nextForwardToken`, if any.
    pub next_forward_token: Option<String>,
}

/// Connection target resolved for one fetch. Holds no credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTarget {
    /// Profile name (`None`: SDK default).
    pub profile_name: Option<String>,
    /// Region from the profile or the SDK default chain.
    pub region: String,
}

/// Read-only boundary to CloudWatch Logs.
pub trait CloudWatchLogsGateway: Send + Sync {
    /// Calls `GetLogEvents` once. Before the first call the implementation
    /// resolves the connection target; when no region can be found it
    /// returns a `RegionMissing` failure without calling the API (BR1.6).
    fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> impl Future<Output = Result<GetLogEventsPage, ApiFailure>> + Send;
}
