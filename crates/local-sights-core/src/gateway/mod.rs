//! CloudWatchLogsGateway: the only boundary through which AWS is reached.
//!
//! The boundary exposes only read-only operations: `GetLogEvents` (U1:BR3.4),
//! since U2 `DescribeLogGroups` (U2:BR3.9) and since U3 `DescribeLogStreams`
//! (U3:BR1.4). There is deliberately no
//! way to call any other API through it (project.md Forbidden: read-only
//! APIs only). Tests replace the AWS
//! implementation with a fake and never connect to AWS.

pub mod aws;
pub mod classify;

use std::future::Future;

use serde::Serialize;

use crate::failure::ApiFailure;
use crate::log_groups::LogGroup;
use crate::streams::LogStream;

/// API name of `GetLogEvents`.
pub const GET_LOG_EVENTS: &str = "GetLogEvents";
/// API name of `DescribeLogGroups`.
pub const DESCRIBE_LOG_GROUPS: &str = "DescribeLogGroups";
/// API name of `DescribeLogStreams`.
pub const DESCRIBE_LOG_STREAMS: &str = "DescribeLogStreams";

/// Arguments of one `GetLogEvents` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLogEventsRequest {
    /// Profile used to resolve the connection target (`None`: SDK default).
    pub profile_name: Option<String>,
    /// Region to connect to (`None`: the profile's default region).
    pub region: Option<String>,
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

/// Arguments of one `DescribeLogGroups` call. Only the next token is passed
/// to the API; filtering happens on the fetched list (U2:BR3.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribeLogGroupsRequest {
    /// Profile (`None`: SDK default).
    pub profile_name: Option<String>,
    /// Region to connect to (`None`: the profile's default region).
    pub region: Option<String>,
    /// `nextToken` from the previous response (`None` on the first call).
    pub next_token: Option<String>,
}

/// One page of `DescribeLogGroups`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DescribeLogGroupsPage {
    /// Log groups in the order the API returned them.
    pub groups: Vec<LogGroup>,
    /// `nextToken`, if any.
    pub next_token: Option<String>,
}

/// Arguments of one `DescribeLogStreams` call. The implementation always
/// asks for the streams by last event time, newest first; nothing else (no
/// name prefix) is passed (U3:BR1.1, BR1.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribeLogStreamsRequest {
    /// Profile (`None`: SDK default).
    pub profile_name: Option<String>,
    /// Region to connect to (`None`: the profile's default region).
    pub region: Option<String>,
    /// Log group name.
    pub log_group_name: String,
    /// `nextToken` from the previous response (`None` on the first call).
    pub next_token: Option<String>,
}

/// One page of `DescribeLogStreams`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DescribeLogStreamsPage {
    /// Streams in the order the API returned them.
    pub streams: Vec<LogStream>,
    /// `nextToken`, if any.
    pub next_token: Option<String>,
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
    /// resolves the connection target: the given region, or else the
    /// profile's default region; when no region can be found it returns a
    /// `RegionMissing` failure without calling the API (U1:BR1.6).
    fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> impl Future<Output = Result<GetLogEventsPage, ApiFailure>> + Send;

    /// Calls `DescribeLogGroups` once, passing only the next token.
    fn describe_log_groups(
        &self,
        request: DescribeLogGroupsRequest,
    ) -> impl Future<Output = Result<DescribeLogGroupsPage, ApiFailure>> + Send;

    /// Calls `DescribeLogStreams` once for the log group, ordered by last
    /// event time, newest first, following the next token (U3:BR1.1).
    fn describe_log_streams(
        &self,
        request: DescribeLogStreamsRequest,
    ) -> impl Future<Output = Result<DescribeLogStreamsPage, ApiFailure>> + Send;
}
