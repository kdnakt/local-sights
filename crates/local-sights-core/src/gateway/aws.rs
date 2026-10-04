//! AWS SDK implementation of [`CloudWatchLogsGateway`].
//!
//! Authentication is left entirely to the AWS SDK default chain (SSO,
//! access keys, AssumeRole, environment variables; BR1.7). This module never
//! reads, stores or logs credentials. SDK errors are reduced to an
//! [`ErrorSignal`], classified, and returned with a safe detail only; the raw
//! SDK message is never surfaced (BR4.3).

use std::error::Error as StdError;
use std::sync::{Mutex, PoisonError};

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::provider::error::{CredentialsError, TokenError};
use aws_sdk_cloudwatchlogs::Client;
use aws_sdk_cloudwatchlogs::config::http::HttpResponse;
use aws_sdk_cloudwatchlogs::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_cloudwatchlogs::operation::RequestId;
use aws_sdk_cloudwatchlogs::operation::get_log_events::{GetLogEventsError, GetLogEventsOutput};

use super::classify::{ErrorSignal, classify};
use super::{
    CloudWatchLogsGateway, ConnectionTarget, GET_LOG_EVENTS, GatewayEvent, GetLogEventsPage,
    GetLogEventsRequest,
};
use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};

/// The SDK error type of `GetLogEvents`.
pub type GetLogEventsSdkError = SdkError<GetLogEventsError, HttpResponse>;

/// A client built for one profile, reused across pages and fetches.
#[derive(Debug, Clone)]
struct CachedClient {
    profile_name: Option<String>,
    client: Client,
}

/// CloudWatch Logs gateway backed by the AWS SDK.
#[derive(Debug, Default)]
pub struct AwsCloudWatchLogsGateway {
    cached: Mutex<Option<CachedClient>>,
}

impl AwsCloudWatchLogsGateway {
    /// A gateway that resolves its connection lazily, before the first call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a client for the profile, resolving the target if needed.
    async fn client_for(&self, profile_name: Option<&str>) -> Result<Client, ApiFailure> {
        if let Some(client) = self.cached_client(profile_name) {
            return Ok(client);
        }
        let (_target, config) = resolve_connection(profile_name).await?;
        let client = Client::new(&config);
        let mut cached = self.cached.lock().unwrap_or_else(PoisonError::into_inner);
        *cached = Some(CachedClient {
            profile_name: profile_name.map(str::to_string),
            client: client.clone(),
        });
        Ok(client)
    }

    fn cached_client(&self, profile_name: Option<&str>) -> Option<Client> {
        let cached = self.cached.lock().unwrap_or_else(PoisonError::into_inner);
        cached
            .as_ref()
            .filter(|entry| entry.profile_name.as_deref() == profile_name)
            .map(|entry| entry.client.clone())
    }
}

impl CloudWatchLogsGateway for AwsCloudWatchLogsGateway {
    async fn get_log_events(
        &self,
        request: GetLogEventsRequest,
    ) -> Result<GetLogEventsPage, ApiFailure> {
        let client = self.client_for(request.profile_name.as_deref()).await?;
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
}

/// Resolves the connection target for a profile (`None`: SDK default,
/// BR1.5). The region comes from the profile or the SDK default chain;
/// without one, `RegionMissing` is returned and no API is called (BR1.6).
/// Credentials are resolved lazily by the SDK and never read here (BR1.7).
pub async fn resolve_connection_target(
    profile_name: Option<&str>,
) -> Result<ConnectionTarget, ApiFailure> {
    resolve_connection(profile_name)
        .await
        .map(|(target, _config)| target)
}

async fn resolve_connection(
    profile_name: Option<&str>,
) -> Result<(ConnectionTarget, SdkConfig), ApiFailure> {
    let mut loader = aws_config::defaults(BehaviorVersion::latest());
    if let Some(profile) = profile_name {
        loader = loader.profile_name(profile);
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

/// Converts an SDK error into an [`ApiFailure`] that carries only the kind
/// and allow-listed fields; the SDK's own message is discarded.
pub fn failure_from_sdk_error(
    error: &GetLogEventsSdkError,
    request: &GetLogEventsRequest,
) -> ApiFailure {
    let code = service_code(error);
    let kind = classify(error_signal(error, code.as_deref()));
    let fields = SafeDetailFields {
        api_name: Some(GET_LOG_EVENTS.to_string()),
        request_id: error.request_id().map(str::to_string),
        profile_name: request.profile_name.clone(),
        log_group_name: Some(request.log_group_name.clone()),
        log_stream_name: Some(request.log_stream_name.clone()),
        ..SafeDetailFields::default()
    };
    ApiFailure::new(kind, &fields)
}

/// The service error code, from the error metadata or the modeled variant.
fn service_code(error: &GetLogEventsSdkError) -> Option<String> {
    let SdkError::ServiceError(context) = error else {
        return None;
    };
    let modeled = match context.err() {
        GetLogEventsError::InvalidParameterException(_) => Some("InvalidParameterException"),
        GetLogEventsError::ResourceNotFoundException(_) => Some("ResourceNotFoundException"),
        GetLogEventsError::ServiceUnavailableException(_) => Some("ServiceUnavailableException"),
        _ => None,
    };
    error.code().or(modeled).map(str::to_string)
}

fn error_signal<'a>(error: &GetLogEventsSdkError, code: Option<&'a str>) -> ErrorSignal<'a> {
    if let Some(code) = code {
        return ErrorSignal::ServiceCode(code);
    }
    if has_credentials_error(error) {
        return ErrorSignal::CredentialsUnavailable;
    }
    match error {
        SdkError::TimeoutError(_) => ErrorSignal::Timeout,
        SdkError::DispatchFailure(failure) if failure.is_timeout() => ErrorSignal::Timeout,
        SdkError::DispatchFailure(failure) if failure.is_io() => ErrorSignal::ConnectionFailed,
        _ => ErrorSignal::Unknown,
    }
}

/// Whether a credentials or token provider failure is anywhere in the chain.
fn has_credentials_error(error: &GetLogEventsSdkError) -> bool {
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
    async fn connection_target_resolution_uses_profile_or_sdk_default_region() {
        let config = write_temp(
            "[default]\nregion = us-west-2\n\n\
             [profile with-region]\nregion = ap-northeast-1\n\n\
             [profile without-region]\noutput = json\n",
        );
        let credentials = write_temp("");
        // SAFETY: no other test in this crate reads or writes the process
        // environment, so these writes cannot race with a concurrent reader.
        unsafe {
            std::env::set_var("AWS_CONFIG_FILE", config.path());
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", credentials.path());
            std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
            for name in ["AWS_REGION", "AWS_DEFAULT_REGION", "AWS_PROFILE"] {
                std::env::remove_var(name);
            }
        }

        let target = resolve_connection_target(Some("with-region"))
            .await
            .unwrap();
        assert_eq!(target.region, "ap-northeast-1");
        assert_eq!(target.profile_name.as_deref(), Some("with-region"));

        let failure = resolve_connection_target(Some("without-region"))
            .await
            .unwrap_err();
        assert_eq!(failure.kind(), FailureKind::RegionMissing);
        assert_eq!(
            failure.safe_detail(),
            "kind=RegionMissing; profile=without-region"
        );

        let target = resolve_connection_target(None).await.unwrap();
        assert_eq!(target.region, "us-west-2");
        assert_eq!(target.profile_name, None);

        // The gateway itself resolves the target before the first call and
        // returns RegionMissing without calling GetLogEvents (BR1.6).
        let gateway = AwsCloudWatchLogsGateway::new();
        let mut call = request();
        call.profile_name = Some("without-region".to_string());
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
}
