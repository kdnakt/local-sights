//! AWS SDK implementation of [`CloudWatchLogsGateway`].
//!
//! Authentication is left entirely to the AWS SDK default chain (SSO,
//! access keys, AssumeRole, environment variables; U1:BR1.7). This module
//! never reads, stores or logs credentials. SDK errors are reduced to an
//! [`ErrorSignal`], classified, and returned with a safe detail only; the raw
//! SDK message is never surfaced (U1:BR4.3). Only `GetLogEvents`,
//! `DescribeLogGroups` and `DescribeLogStreams` are called (U2:BR3.9,
//! U3:BR1.4).

use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::{Mutex, PoisonError};

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::provider::error::{CredentialsError, TokenError};
use aws_sdk_cloudwatchlogs::Client;
use aws_sdk_cloudwatchlogs::config::http::HttpResponse;
use aws_sdk_cloudwatchlogs::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_cloudwatchlogs::operation::RequestId;
use aws_sdk_cloudwatchlogs::operation::describe_log_groups::{
    DescribeLogGroupsError, DescribeLogGroupsOutput,
};
use aws_sdk_cloudwatchlogs::operation::describe_log_streams::{
    DescribeLogStreamsError, DescribeLogStreamsOutput,
};
use aws_sdk_cloudwatchlogs::operation::get_log_events::{GetLogEventsError, GetLogEventsOutput};
use aws_sdk_cloudwatchlogs::types::OrderBy;

use super::classify::{ErrorSignal, classify};
use super::{
    CloudWatchLogsGateway, ConnectionTarget, DESCRIBE_LOG_GROUPS, DESCRIBE_LOG_STREAMS,
    DescribeLogGroupsPage, DescribeLogGroupsRequest, DescribeLogStreamsPage,
    DescribeLogStreamsRequest, GET_LOG_EVENTS, GatewayEvent, GetLogEventsPage, GetLogEventsRequest,
};
use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};
use crate::log_groups::LogGroup;
use crate::streams::LogStream;

/// The SDK error type of `GetLogEvents`.
pub type GetLogEventsSdkError = SdkError<GetLogEventsError, HttpResponse>;
/// The SDK error type of `DescribeLogGroups`.
pub type DescribeLogGroupsSdkError = SdkError<DescribeLogGroupsError, HttpResponse>;
/// The SDK error type of `DescribeLogStreams`.
pub type DescribeLogStreamsSdkError = SdkError<DescribeLogStreamsError, HttpResponse>;

/// Cache key: (profile name, explicit region).
type ClientKey = (Option<String>, Option<String>);

/// CloudWatch Logs gateway backed by the AWS SDK. Clients are built lazily
/// and reused per (profile, region) (U2:BR2.8).
#[derive(Debug, Default)]
pub struct AwsCloudWatchLogsGateway {
    clients: Mutex<HashMap<ClientKey, Client>>,
}

impl AwsCloudWatchLogsGateway {
    /// A gateway that resolves its connection lazily, before the first call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a client for the profile and region, resolving the target
    /// if needed.
    async fn client_for(
        &self,
        profile_name: Option<&str>,
        region: Option<&str>,
    ) -> Result<Client, ApiFailure> {
        let key: ClientKey = (profile_name.map(str::to_string), region.map(str::to_string));
        if let Some(client) = self.lock_clients().get(&key) {
            return Ok(client.clone());
        }
        let (_target, config) = resolve_connection(profile_name, region).await?;
        let client = Client::new(&config);
        self.lock_clients().insert(key, client.clone());
        Ok(client)
    }

    /// A poisoned cache still holds valid clients, so it is recovered.
    fn lock_clients(&self) -> std::sync::MutexGuard<'_, HashMap<ClientKey, Client>> {
        self.clients.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CloudWatchLogsGateway for AwsCloudWatchLogsGateway {
    async fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> Result<GetLogEventsPage, ApiFailure> {
        let client = self
            .client_for(request.profile_name.as_deref(), request.region.as_deref())
            .await?;
        let output = client
            .get_log_events()
            .log_group_name(&request.log_group_name)
            .log_stream_name(&request.log_stream_name)
            .start_time(request.start_time)
            .end_time(request.end_time)
            .start_from_head(request.start_from_head)
            .set_next_token(request.next_token.clone())
            .send()
            .await
            .map_err(|error| failure_from_sdk_error(&error, &request))?;
        Ok(page_from_output(&output))
    }

    async fn describe_log_groups(
        &self,
        request: DescribeLogGroupsRequest,
    ) -> Result<DescribeLogGroupsPage, ApiFailure> {
        let client = self
            .client_for(request.profile_name.as_deref(), request.region.as_deref())
            .await?;
        // Only the next token is passed (U2:BR3.9).
        let output = client
            .describe_log_groups()
            .set_next_token(request.next_token.clone())
            .send()
            .await
            .map_err(|error| failure_from_describe_error(&error, &request))?;
        Ok(groups_page_from_output(&output))
    }

    async fn describe_log_streams(
        &self,
        request: DescribeLogStreamsRequest,
    ) -> Result<DescribeLogStreamsPage, ApiFailure> {
        let client = self
            .client_for(request.profile_name.as_deref(), request.region.as_deref())
            .await?;
        // Newest last event first; no name prefix (U3:BR1.1, BR1.4).
        let output = client
            .describe_log_streams()
            .log_group_name(&request.log_group_name)
            .order_by(OrderBy::LastEventTime)
            .descending(true)
            .set_next_token(request.next_token.clone())
            .send()
            .await
            .map_err(|error| failure_from_describe_streams_error(&error, &request))?;
        Ok(streams_page_from_output(&output))
    }
}

/// Resolves the connection target for a profile (`None`: SDK default,
/// U1:BR1.5). An explicit `region` is used as is (U2:BR2.8); otherwise the
/// region comes from the profile or the SDK default chain, and without one
/// `RegionMissing` is returned and no API is called (U1:BR1.6).
/// Credentials are resolved lazily by the SDK and never read here.
pub async fn resolve_connection_target(
    profile_name: Option<&str>,
    region: Option<&str>,
) -> Result<ConnectionTarget, ApiFailure> {
    resolve_connection(profile_name, region)
        .await
        .map(|(target, _config)| target)
}

async fn resolve_connection(
    profile_name: Option<&str>,
    region: Option<&str>,
) -> Result<(ConnectionTarget, SdkConfig), ApiFailure> {
    let mut loader = aws_config::defaults(BehaviorVersion::latest());
    if let Some(profile) = profile_name {
        loader = loader.profile_name(profile);
    }
    if let Some(region) = region {
        loader = loader.region(Region::new(region.to_string()));
    }
    let config = loader.load().await;
    let Some(region) = config.region() else {
        let fields = SafeDetailFields {
            profile_name: profile_name.map(str::to_string),
            ..SafeDetailFields::default()
        };
        return Err(ApiFailure::new(FailureKind::RegionMissing, &fields));
    };
    let target = ConnectionTarget {
        profile_name: profile_name.map(str::to_string),
        region: region.to_string(),
    };
    Ok((target, config))
}

fn page_from_output(output: &GetLogEventsOutput) -> GetLogEventsPage {
    let events = output
        .events()
        .iter()
        .map(|event| GatewayEvent {
            // The API always sets the timestamp; 0 keeps a malformed event
            // visible instead of dropping it.
            timestamp: event.timestamp().unwrap_or_default(),
            ingestion_time: event.ingestion_time(),
            message: event.message().unwrap_or_default().to_string(),
        })
        .collect();
    GetLogEventsPage {
        events,
        next_forward_token: output.next_forward_token().map(str::to_string),
    }
}

fn groups_page_from_output(output: &DescribeLogGroupsOutput) -> DescribeLogGroupsPage {
    let groups = output
        .log_groups()
        .iter()
        // A log group always has a name; one without could not be selected.
        .filter_map(|group| {
            group.log_group_name().map(|name| LogGroup {
                log_group_name: name.to_string(),
                arn: group.arn().map(str::to_string),
                creation_time: group.creation_time(),
            })
        })
        .collect();
    DescribeLogGroupsPage {
        groups,
        next_token: output.next_token().map(str::to_string),
    }
}

fn streams_page_from_output(output: &DescribeLogStreamsOutput) -> DescribeLogStreamsPage {
    let streams = output
        .log_streams()
        .iter()
        // A stream always has a name; one without could not be fetched.
        .filter_map(|stream| {
            stream.log_stream_name().map(|name| LogStream {
                log_stream_name: name.to_string(),
                first_event_timestamp: stream.first_event_timestamp(),
                last_event_timestamp: stream.last_event_timestamp(),
            })
        })
        .collect();
    DescribeLogStreamsPage {
        streams,
        next_token: output.next_token().map(str::to_string),
    }
}

/// Converts a `GetLogEvents` SDK error into an [`ApiFailure`] that carries
/// only the kind and allow-listed fields; the SDK's own message is dropped.
pub fn failure_from_sdk_error(
    error: &GetLogEventsSdkError,
    request: &GetLogEventsRequest,
) -> ApiFailure {
    let modeled = match error {
        SdkError::ServiceError(context) => match context.err() {
            GetLogEventsError::InvalidParameterException(_) => Some("InvalidParameterException"),
            GetLogEventsError::ResourceNotFoundException(_) => Some("ResourceNotFoundException"),
            GetLogEventsError::ServiceUnavailableException(_) => {
                Some("ServiceUnavailableException")
            }
            _ => None,
        },
        _ => None,
    };
    let fields = SafeDetailFields {
        api_name: Some(GET_LOG_EVENTS.to_string()),
        request_id: error.request_id().map(str::to_string),
        profile_name: request.profile_name.clone(),
        log_group_name: Some(request.log_group_name.clone()),
        log_stream_name: Some(request.log_stream_name.clone()),
        ..SafeDetailFields::default()
    };
    ApiFailure::new(classify_sdk_error(error, modeled), &fields)
}

/// Converts a `DescribeLogGroups` SDK error into an [`ApiFailure`] with the
/// same classification and safe detail rules as `GetLogEvents`.
pub fn failure_from_describe_error(
    error: &DescribeLogGroupsSdkError,
    request: &DescribeLogGroupsRequest,
) -> ApiFailure {
    let modeled = match error {
        SdkError::ServiceError(context) => match context.err() {
            DescribeLogGroupsError::InvalidParameterException(_) => {
                Some("InvalidParameterException")
            }
            DescribeLogGroupsError::ServiceUnavailableException(_) => {
                Some("ServiceUnavailableException")
            }
            _ => None,
        },
        _ => None,
    };
    let fields = SafeDetailFields {
        api_name: Some(DESCRIBE_LOG_GROUPS.to_string()),
        request_id: error.request_id().map(str::to_string),
        profile_name: request.profile_name.clone(),
        ..SafeDetailFields::default()
    };
    ApiFailure::new(classify_sdk_error(error, modeled), &fields)
}

/// Converts a `DescribeLogStreams` SDK error into an [`ApiFailure`] with the
/// same classification and safe detail rules as the other operations.
pub fn failure_from_describe_streams_error(
    error: &DescribeLogStreamsSdkError,
    request: &DescribeLogStreamsRequest,
) -> ApiFailure {
    let modeled = match error {
        SdkError::ServiceError(context) => match context.err() {
            DescribeLogStreamsError::InvalidParameterException(_) => {
                Some("InvalidParameterException")
            }
            DescribeLogStreamsError::ResourceNotFoundException(_) => {
                Some("ResourceNotFoundException")
            }
            DescribeLogStreamsError::ServiceUnavailableException(_) => {
                Some("ServiceUnavailableException")
            }
            _ => None,
        },
        _ => None,
    };
    let fields = SafeDetailFields {
        api_name: Some(DESCRIBE_LOG_STREAMS.to_string()),
        request_id: error.request_id().map(str::to_string),
        profile_name: request.profile_name.clone(),
        log_group_name: Some(request.log_group_name.clone()),
        ..SafeDetailFields::default()
    };
    ApiFailure::new(classify_sdk_error(error, modeled), &fields)
}

/// Classifies any operation's SDK error: the service code from the error
/// metadata (or the modeled variant), credentials failures in the source
/// chain, timeouts and I/O failures.
fn classify_sdk_error<E>(error: &SdkError<E, HttpResponse>, modeled: Option<&str>) -> FailureKind
where
    E: ProvideErrorMetadata + StdError + 'static,
{
    let code = match error {
        SdkError::ServiceError(_) => error.code().or(modeled),
        _ => None,
    };
    if let Some(code) = code {
        return classify(ErrorSignal::ServiceCode(code));
    }
    let signal = if has_credentials_error(error) {
        ErrorSignal::CredentialsUnavailable
    } else {
        match error {
            SdkError::TimeoutError(_) => ErrorSignal::Timeout,
            SdkError::DispatchFailure(failure) if failure.is_timeout() => ErrorSignal::Timeout,
            SdkError::DispatchFailure(failure) if failure.is_io() => ErrorSignal::ConnectionFailed,
            _ => ErrorSignal::Unknown,
        }
    };
    classify(signal)
}

/// Whether a credentials or token provider failure is anywhere in the chain.
fn has_credentials_error(error: &(dyn StdError + 'static)) -> bool {
    let mut current: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(source) = current {
        if source.is::<CredentialsError>() || source.is::<TokenError>() {
            return true;
        }
        current = source.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use aws_sdk_cloudwatchlogs::error::ErrorMetadata;
    use aws_sdk_cloudwatchlogs::types::error::ResourceNotFoundException;
    use aws_smithy_runtime_api::client::result::ConnectorError;
    use aws_smithy_types::body::SdkBody;

    use super::*;

    fn request() -> GetLogEventsRequest {
        GetLogEventsRequest {
            profile_name: Some("dev".to_string()),
            region: Some("ap-northeast-1".to_string()),
            log_group_name: "/aws/lambda/orders".to_string(),
            log_stream_name: "stream-a".to_string(),
            start_time: 0,
            end_time: 1_000,
            start_from_head: true,
            next_token: None,
        }
    }

    fn http_response(status: u16) -> HttpResponse {
        let status = aws_smithy_runtime_api::http::StatusCode::try_from(status).unwrap();
        HttpResponse::new(status, SdkBody::empty())
    }

    fn dummy_access_key_id() -> String {
        format!("{}{}", "AKIA", "EXAMPLE0DUMMY000")
    }

    fn unhandled_service_error(code: &str, raw_message: &str) -> GetLogEventsSdkError {
        let meta = ErrorMetadata::builder()
            .code(code)
            .message(raw_message)
            .build();
        SdkError::service_error(GetLogEventsError::generic(meta), http_response(400))
    }

    fn write_temp(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file
    }

    /// All environment changes happen inside this single test, one after the
    /// other, so no parallel test observes them. Only temporary files are
    /// read, instance metadata is disabled, and no AWS API is called.
    #[tokio::test]
    // The guard only serializes environment access with other tests; this
    // test's own single-threaded runtime never contends for it.
    #[allow(clippy::await_holding_lock)]
    async fn connection_target_resolution_uses_profile_or_sdk_default_region() {
        let _env = crate::test_support::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let config = write_temp(
            "[default]\nregion = us-west-2\n\n\
             [profile with-region]\nregion = ap-northeast-1\n\n\
             [profile without-region]\noutput = json\n",
        );
        let credentials = write_temp("");
        // SAFETY: every test in this crate that reads or writes the process
        // environment holds ENV_LOCK, so these writes cannot race.
        unsafe {
            std::env::set_var("AWS_CONFIG_FILE", config.path());
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", credentials.path());
            std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
            for name in ["AWS_REGION", "AWS_DEFAULT_REGION", "AWS_PROFILE"] {
                std::env::remove_var(name);
            }
        }

        let target = resolve_connection_target(Some("with-region"), None)
            .await
            .unwrap();
        assert_eq!(target.region, "ap-northeast-1");
        assert_eq!(target.profile_name.as_deref(), Some("with-region"));

        let failure = resolve_connection_target(Some("without-region"), None)
            .await
            .unwrap_err();
        assert_eq!(failure.kind(), FailureKind::RegionMissing);
        assert_eq!(
            failure.safe_detail(),
            "kind=RegionMissing; profile=without-region"
        );

        let target = resolve_connection_target(None, None).await.unwrap();
        assert_eq!(target.region, "us-west-2");
        assert_eq!(target.profile_name, None);

        // An explicit region is used even when the profile has none (U2:BR2.8).
        let target = resolve_connection_target(Some("without-region"), Some("eu-west-1"))
            .await
            .unwrap();
        assert_eq!(target.region, "eu-west-1");
        let target = resolve_connection_target(Some("with-region"), Some("us-east-2"))
            .await
            .unwrap();
        assert_eq!(target.region, "us-east-2");

        // The gateway itself resolves the target before the first call and
        // returns RegionMissing without calling GetLogEvents (BR1.6).
        let gateway = AwsCloudWatchLogsGateway::new();
        let mut call = request();
        call.profile_name = Some("without-region".to_string());
        call.region = None;
        let failure = gateway.get_log_events(call).await.unwrap_err();
        assert_eq!(failure.kind(), FailureKind::RegionMissing);
    }

    #[test]
    fn service_error_is_classified_and_raw_message_is_dropped() {
        let raw = format!("Rate exceeded for {} secret", dummy_access_key_id());
        let error = unhandled_service_error("ThrottlingException", &raw);
        let failure = failure_from_sdk_error(&error, &request());
        assert_eq!(failure.kind(), FailureKind::Throttled);
        assert!(failure.retryable());
        assert_eq!(
            failure.safe_detail(),
            "kind=Throttled; api=GetLogEvents; profile=dev; \
             logGroup=/aws/lambda/orders; logStream=stream-a"
        );
        assert!(!failure.safe_detail().contains("Rate exceeded"));
        assert!(!failure.safe_detail().contains("AKIA"));
    }

    #[test]
    fn access_denied_and_expired_token_codes_are_classified() {
        let denied = unhandled_service_error("AccessDeniedException", "denied");
        assert_eq!(
            failure_from_sdk_error(&denied, &request()).kind(),
            FailureKind::AccessDenied
        );
        let expired = unhandled_service_error("ExpiredTokenException", "expired");
        assert_eq!(
            failure_from_sdk_error(&expired, &request()).kind(),
            FailureKind::AuthRequired
        );
    }

    #[test]
    fn modeled_variant_without_metadata_code_is_classified() {
        let error = SdkError::service_error(
            GetLogEventsError::ResourceNotFoundException(
                ResourceNotFoundException::builder()
                    .message("The specified log group does not exist.")
                    .build(),
            ),
            http_response(400),
        );
        let failure = failure_from_sdk_error(&error, &request());
        assert_eq!(failure.kind(), FailureKind::NotFound);
        assert!(!failure.safe_detail().contains("does not exist"));
    }

    #[test]
    fn timeouts_and_io_failures_are_network_errors() {
        let timeout: GetLogEventsSdkError = SdkError::timeout_error("operation timed out");
        assert_eq!(
            failure_from_sdk_error(&timeout, &request()).kind(),
            FailureKind::Network
        );
        let io: GetLogEventsSdkError = SdkError::dispatch_failure(ConnectorError::io(
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused").into(),
        ));
        let failure = failure_from_sdk_error(&io, &request());
        assert_eq!(failure.kind(), FailureKind::Network);
        assert!(failure.retryable());
    }

    #[test]
    fn credentials_failures_require_authentication() {
        let error: GetLogEventsSdkError = SdkError::dispatch_failure(ConnectorError::other(
            CredentialsError::not_loaded("no providers in chain provided credentials").into(),
            None,
        ));
        let failure = failure_from_sdk_error(&error, &request());
        assert_eq!(failure.kind(), FailureKind::AuthRequired);
        assert!(!failure.retryable());
    }

    #[test]
    fn page_keeps_api_order_and_forward_token() {
        use aws_sdk_cloudwatchlogs::types::OutputLogEvent;
        let output = GetLogEventsOutput::builder()
            .events(
                OutputLogEvent::builder()
                    .timestamp(2)
                    .ingestion_time(3)
                    .message("b\nc")
                    .build(),
            )
            .events(OutputLogEvent::builder().timestamp(1).message("a").build())
            .next_forward_token("f/1")
            .build();
        let page = page_from_output(&output);
        assert_eq!(page.next_forward_token.as_deref(), Some("f/1"));
        assert_eq!(
            page.events,
            vec![
                GatewayEvent {
                    timestamp: 2,
                    ingestion_time: Some(3),
                    message: "b\nc".to_string()
                },
                GatewayEvent {
                    timestamp: 1,
                    ingestion_time: None,
                    message: "a".to_string()
                },
            ]
        );
    }

    fn describe_request() -> DescribeLogGroupsRequest {
        DescribeLogGroupsRequest {
            profile_name: Some("dev".to_string()),
            region: Some("ap-northeast-1".to_string()),
            next_token: None,
        }
    }

    fn unhandled_describe_error(code: &str, raw_message: &str) -> DescribeLogGroupsSdkError {
        let meta = ErrorMetadata::builder()
            .code(code)
            .message(raw_message)
            .build();
        SdkError::service_error(DescribeLogGroupsError::generic(meta), http_response(400))
    }

    #[test]
    fn describe_errors_use_the_same_classification_and_drop_raw_messages() {
        let raw = format!("User {} is not authorized", dummy_access_key_id());
        let denied = unhandled_describe_error("AccessDeniedException", &raw);
        let failure = failure_from_describe_error(&denied, &describe_request());
        assert_eq!(failure.kind(), FailureKind::AccessDenied);
        assert_eq!(
            failure.safe_detail(),
            "kind=AccessDenied; api=DescribeLogGroups; profile=dev"
        );
        assert!(!failure.safe_detail().contains("not authorized"));

        let throttled = unhandled_describe_error("ThrottlingException", "slow down");
        let failure = failure_from_describe_error(&throttled, &describe_request());
        assert_eq!(failure.kind(), FailureKind::Throttled);
        assert!(failure.retryable());

        let expired = unhandled_describe_error("ExpiredTokenException", "expired");
        assert_eq!(
            failure_from_describe_error(&expired, &describe_request()).kind(),
            FailureKind::AuthRequired
        );
    }

    #[test]
    fn describe_credentials_and_network_failures_are_classified() {
        let no_credentials: DescribeLogGroupsSdkError =
            SdkError::dispatch_failure(ConnectorError::other(
                CredentialsError::not_loaded("no providers in chain provided credentials").into(),
                None,
            ));
        assert_eq!(
            failure_from_describe_error(&no_credentials, &describe_request()).kind(),
            FailureKind::AuthRequired
        );
        let timeout: DescribeLogGroupsSdkError = SdkError::timeout_error("timed out");
        assert_eq!(
            failure_from_describe_error(&timeout, &describe_request()).kind(),
            FailureKind::Network
        );
    }

    #[test]
    fn groups_page_keeps_names_arns_and_next_token() {
        use aws_sdk_cloudwatchlogs::types::LogGroup as SdkLogGroup;
        let output = DescribeLogGroupsOutput::builder()
            .log_groups(
                SdkLogGroup::builder()
                    .log_group_name("/aws/lambda/MyFunction")
                    .arn("arn:aws:logs:ap-northeast-1:123456789012:log-group:x")
                    .creation_time(5)
                    .build(),
            )
            .log_groups(SdkLogGroup::builder().creation_time(6).build())
            .next_token("n1")
            .build();
        let page = groups_page_from_output(&output);
        assert_eq!(page.next_token.as_deref(), Some("n1"));
        assert_eq!(
            page.groups.len(),
            1,
            "a group without a name cannot be listed"
        );
        assert_eq!(page.groups[0].log_group_name, "/aws/lambda/MyFunction");
        assert_eq!(page.groups[0].creation_time, Some(5));
        assert!(page.groups[0].arn.is_some());
    }

    // --- U3: DescribeLogStreams ---

    fn streams_request() -> DescribeLogStreamsRequest {
        DescribeLogStreamsRequest {
            profile_name: Some("dev".to_string()),
            region: Some("ap-northeast-1".to_string()),
            log_group_name: "/aws/lambda/orders".to_string(),
            next_token: None,
        }
    }

    fn unhandled_streams_error(code: &str, raw_message: &str) -> DescribeLogStreamsSdkError {
        let meta = ErrorMetadata::builder()
            .code(code)
            .message(raw_message)
            .build();
        SdkError::service_error(DescribeLogStreamsError::generic(meta), http_response(400))
    }

    #[test]
    fn describe_streams_errors_are_classified_and_raw_messages_dropped() {
        let raw = format!("Rate exceeded for {}", dummy_access_key_id());
        let throttled = unhandled_streams_error("ThrottlingException", &raw);
        let failure = failure_from_describe_streams_error(&throttled, &streams_request());
        assert_eq!(failure.kind(), FailureKind::Throttled);
        assert!(failure.retryable());
        assert_eq!(
            failure.safe_detail(),
            "kind=Throttled; api=DescribeLogStreams; profile=dev; logGroup=/aws/lambda/orders"
        );
        assert!(!failure.safe_detail().contains("Rate exceeded"));
        assert!(!failure.safe_detail().contains("AKIA"));

        let denied = unhandled_streams_error("AccessDeniedException", "denied");
        assert_eq!(
            failure_from_describe_streams_error(&denied, &streams_request()).kind(),
            FailureKind::AccessDenied
        );
        let missing = SdkError::service_error(
            DescribeLogStreamsError::ResourceNotFoundException(
                ResourceNotFoundException::builder()
                    .message("The specified log group does not exist.")
                    .build(),
            ),
            http_response(400),
        );
        let failure = failure_from_describe_streams_error(&missing, &streams_request());
        assert_eq!(failure.kind(), FailureKind::NotFound);
        assert!(!failure.safe_detail().contains("does not exist"));
    }

    #[test]
    fn describe_streams_network_and_credentials_failures_are_classified() {
        let timeout: DescribeLogStreamsSdkError = SdkError::timeout_error("timed out");
        let failure = failure_from_describe_streams_error(&timeout, &streams_request());
        assert_eq!(failure.kind(), FailureKind::Network);
        assert!(failure.retryable());
        let io: DescribeLogStreamsSdkError = SdkError::dispatch_failure(ConnectorError::io(
            std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset").into(),
        ));
        assert_eq!(
            failure_from_describe_streams_error(&io, &streams_request()).kind(),
            FailureKind::Network
        );
        let no_credentials: DescribeLogStreamsSdkError =
            SdkError::dispatch_failure(ConnectorError::other(
                CredentialsError::not_loaded("no providers in chain provided credentials").into(),
                None,
            ));
        let failure = failure_from_describe_streams_error(&no_credentials, &streams_request());
        assert_eq!(failure.kind(), FailureKind::AuthRequired);
        assert!(!failure.retryable());
    }

    #[test]
    fn streams_page_keeps_order_missing_times_and_next_token() {
        use aws_sdk_cloudwatchlogs::types::LogStream as SdkLogStream;
        let output = DescribeLogStreamsOutput::builder()
            .log_streams(
                SdkLogStream::builder()
                    .log_stream_name("new")
                    .first_event_timestamp(10)
                    .last_event_timestamp(20)
                    .build(),
            )
            .log_streams(SdkLogStream::builder().log_stream_name("empty").build())
            .log_streams(SdkLogStream::builder().last_event_timestamp(5).build())
            .next_token("n1")
            .build();
        let page = streams_page_from_output(&output);
        assert_eq!(page.next_token.as_deref(), Some("n1"));
        assert_eq!(
            page.streams,
            vec![
                LogStream {
                    log_stream_name: "new".to_string(),
                    first_event_timestamp: Some(10),
                    last_event_timestamp: Some(20),
                },
                LogStream {
                    log_stream_name: "empty".to_string(),
                    first_event_timestamp: None,
                    last_event_timestamp: None,
                },
            ],
            "a stream without a name cannot be fetched"
        );
    }
}
