//! A fake [`CloudWatchLogsGateway`] for tests. It never connects to AWS: it
//! returns scripted responses in order and records every request it gets.
//! Since U3 it also answers `DescribeLogStreams` and keeps a separate script
//! of `GetLogEvents` responses per stream name.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::gateway::{
    CloudWatchLogsGateway, DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
    DescribeLogStreamsRequest, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
};
use local_sights_core::log_groups::LogGroup;
use local_sights_core::streams::LogStream;

type EventsScript = VecDeque<Result<GetLogEventsPage, ApiFailure>>;

/// Scripted gateway.
#[derive(Debug, Default)]
pub struct FakeGateway {
    responses: Mutex<VecDeque<Result<GetLogEventsPage, ApiFailure>>>,
    calls: Mutex<Vec<GetLogEventsRequest>>,
    group_responses: Mutex<VecDeque<Result<DescribeLogGroupsPage, ApiFailure>>>,
    group_calls: Mutex<Vec<DescribeLogGroupsRequest>>,
    connect_failure: Option<ApiFailure>,
    stream_responses: Mutex<VecDeque<Result<DescribeLogStreamsPage, ApiFailure>>>,
    stream_calls: Mutex<Vec<DescribeLogStreamsRequest>>,
    per_stream: Mutex<HashMap<String, EventsScript>>,
    api_log: Mutex<Vec<String>>,
}

impl FakeGateway {
    /// A gateway that answers with `responses`, one per call.
    pub fn new(responses: Vec<Result<GetLogEventsPage, ApiFailure>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            ..Self::default()
        }
    }

    /// A gateway that answers `DescribeLogGroups` with `responses`, one per
    /// call (U2).
    pub fn with_log_group_pages(responses: Vec<Result<DescribeLogGroupsPage, ApiFailure>>) -> Self {
        Self {
            group_responses: Mutex::new(responses.into()),
            ..Self::default()
        }
    }

    /// `DescribeLogGroups` requests that reached the (fake) API, in order.
    pub fn group_calls(&self) -> Vec<DescribeLogGroupsRequest> {
        self.group_calls.lock().unwrap().clone()
    }

    /// A gateway whose connection target cannot be resolved: every call
    /// fails with `failure` before any API call is made (like RegionMissing).
    pub fn failing_to_connect(failure: ApiFailure) -> Self {
        Self {
            connect_failure: Some(failure),
            ..Self::default()
        }
    }

    /// Requests that reached the (fake) API, in order.
    pub fn calls(&self) -> Vec<GetLogEventsRequest> {
        self.calls.lock().unwrap().clone()
    }

    /// A gateway that answers `DescribeLogStreams` with `pages`, one per
    /// call (U3).
    pub fn with_stream_pages(pages: Vec<Result<DescribeLogStreamsPage, ApiFailure>>) -> Self {
        Self {
            stream_responses: Mutex::new(pages.into()),
            ..Self::default()
        }
    }

    /// A gateway whose log group has the single stream `name`, read with
    /// the U1-style `responses` (U1 tests).
    pub fn with_single_stream(
        name: &str,
        responses: Vec<Result<GetLogEventsPage, ApiFailure>>,
    ) -> Self {
        Self::with_stream_pages(vec![stream_page(&[(name, None, None)], None)])
            .script_stream(name, responses)
    }

    /// Scripts the `GetLogEvents` responses of one stream.
    pub fn script_stream(
        self,
        name: &str,
        responses: Vec<Result<GetLogEventsPage, ApiFailure>>,
    ) -> Self {
        self.per_stream
            .lock()
            .unwrap()
            .insert(name.to_string(), responses.into());
        self
    }

    /// `DescribeLogStreams` requests that reached the (fake) API, in order.
    pub fn stream_calls(&self) -> Vec<DescribeLogStreamsRequest> {
        self.stream_calls.lock().unwrap().clone()
    }

    /// Every call that reached the (fake) API, in order:
    /// `DescribeLogStreams`, or `GetLogEvents:<stream>`.
    pub fn api_log(&self) -> Vec<String> {
        self.api_log.lock().unwrap().clone()
    }

    /// Streams that `GetLogEvents` was called for, once per call.
    pub fn fetched_streams(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .map(|call| call.log_stream_name)
            .collect()
    }
}

impl CloudWatchLogsGateway for FakeGateway {
    async fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> Result<GetLogEventsPage, ApiFailure> {
        if let Some(failure) = &self.connect_failure {
            return Err(failure.clone());
        }
        self.api_log
            .lock()
            .unwrap()
            .push(format!("GetLogEvents:{}", request.log_stream_name));
        let stream = request.log_stream_name.clone();
        self.calls.lock().unwrap().push(request);
        if let Some(script) = self.per_stream.lock().unwrap().get_mut(&stream) {
            return script.pop_front().unwrap_or_else(|| {
                panic!("FakeGateway received more GetLogEvents calls for {stream} than scripted")
            });
        }
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeGateway received more calls than scripted responses")
    }

    async fn describe_log_groups(
        &self,
        request: DescribeLogGroupsRequest,
    ) -> Result<DescribeLogGroupsPage, ApiFailure> {
        if let Some(failure) = &self.connect_failure {
            return Err(failure.clone());
        }
        self.group_calls.lock().unwrap().push(request);
        self.group_responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeGateway received more DescribeLogGroups calls than scripted")
    }

    async fn describe_log_streams(
        &self,
        request: DescribeLogStreamsRequest,
    ) -> Result<DescribeLogStreamsPage, ApiFailure> {
        if let Some(failure) = &self.connect_failure {
            return Err(failure.clone());
        }
        self.api_log
            .lock()
            .unwrap()
            .push("DescribeLogStreams".to_string());
        self.stream_calls.lock().unwrap().push(request);
        self.stream_responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeGateway received more DescribeLogStreams calls than scripted")
    }
}

/// A `DescribeLogGroups` page with the given names and next token.
pub fn group_page(
    names: &[&str],
    token: Option<&str>,
) -> Result<DescribeLogGroupsPage, ApiFailure> {
    Ok(DescribeLogGroupsPage {
        groups: names
            .iter()
            .map(|name| LogGroup {
                log_group_name: (*name).to_string(),
                arn: None,
                creation_time: None,
            })
            .collect(),
        next_token: token.map(str::to_string),
    })
}

/// A page with events `(timestamp, message)` and an optional forward token.
pub fn page(events: &[(i64, &str)], token: Option<&str>) -> Result<GetLogEventsPage, ApiFailure> {
    Ok(GetLogEventsPage {
        events: events
            .iter()
            .map(|(timestamp, message)| GatewayEvent {
                timestamp: *timestamp,
                ingestion_time: Some(timestamp + 5),
                message: (*message).to_string(),
            })
            .collect(),
        next_forward_token: token.map(str::to_string),
    })
}

/// A failure of the given kind, as the AWS gateway would build it.
pub fn failure(kind: FailureKind) -> ApiFailure {
    ApiFailure::new(
        kind,
        &SafeDetailFields {
            api_name: Some("GetLogEvents".to_string()),
            ..SafeDetailFields::default()
        },
    )
}

/// A `DescribeLogStreams` page of `(name, firstEventTimestamp,
/// lastEventTimestamp)` and an optional next token.
pub fn stream_page(
    streams: &[(&str, Option<i64>, Option<i64>)],
    token: Option<&str>,
) -> Result<DescribeLogStreamsPage, ApiFailure> {
    Ok(DescribeLogStreamsPage {
        streams: streams
            .iter()
            .map(|(name, first, last)| LogStream {
                log_stream_name: (*name).to_string(),
                first_event_timestamp: *first,
                last_event_timestamp: *last,
            })
            .collect(),
        next_token: token.map(str::to_string),
    })
}
