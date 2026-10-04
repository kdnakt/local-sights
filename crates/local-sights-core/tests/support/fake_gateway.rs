//! A fake [`CloudWatchLogsGateway`] for tests. It never connects to AWS: it
//! returns scripted responses in order and records every request it gets.

use std::collections::VecDeque;
use std::sync::Mutex;

use local_sights_core::failure::{ApiFailure, FailureKind, SafeDetailFields};
use local_sights_core::gateway::{
    CloudWatchLogsGateway, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
};

/// Scripted gateway.
#[derive(Debug, Default)]
pub struct FakeGateway {
    responses: Mutex<VecDeque<Result<GetLogEventsPage, ApiFailure>>>,
    calls: Mutex<Vec<GetLogEventsRequest>>,
    connect_failure: Option<ApiFailure>,
}

impl FakeGateway {
    /// A gateway that answers with `responses`, one per call.
    pub fn new(responses: Vec<Result<GetLogEventsPage, ApiFailure>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            ..Self::default()
        }
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
}

impl CloudWatchLogsGateway for FakeGateway {
    async fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> Result<GetLogEventsPage, ApiFailure> {
        if let Some(failure) = &self.connect_failure {
            return Err(failure.clone());
        }
        self.calls.lock().unwrap().push(request);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeGateway received more calls than scripted responses")
    }
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
