//! Classification of AWS errors into [`FailureKind`] (BR4.2).
//!
//! The AWS SDK implementation reduces each SDK error to an [`ErrorSignal`]
//! (service error code, missing credentials, timeout, connection failure);
//! this pure table maps the signal to a kind.

use crate::failure::FailureKind;

/// What the SDK error amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSignal<'a> {
    /// The service answered with this error code.
    ServiceCode(&'a str),
    /// The SDK could not obtain credentials (none configured, SSO expired).
    CredentialsUnavailable,
    /// The request timed out.
    Timeout,
    /// The connection could not be established or was lost.
    ConnectionFailed,
    /// Nothing more specific is known.
    Unknown,
}

/// Maps an error signal to its failure kind.
pub fn classify(signal: ErrorSignal<'_>) -> FailureKind {
    match signal {
        ErrorSignal::CredentialsUnavailable => FailureKind::AuthRequired,
        ErrorSignal::Timeout | ErrorSignal::ConnectionFailed => FailureKind::Network,
        ErrorSignal::Unknown => FailureKind::Other,
        ErrorSignal::ServiceCode(code) => classify_service_code(code),
    }
}

/// Maps a CloudWatch Logs (or STS-style) error code to a kind.
fn classify_service_code(code: &str) -> FailureKind {
    match code {
        "ExpiredToken"
        | "ExpiredTokenException"
        | "UnrecognizedClientException"
        | "InvalidClientTokenId"
        | "InvalidSignatureException"
        | "MissingAuthenticationToken" => FailureKind::AuthRequired,
        "AccessDeniedException" | "AccessDenied" => FailureKind::AccessDenied,
        "ThrottlingException" | "Throttling" => FailureKind::Throttled,
        "ResourceNotFoundException" => FailureKind::NotFound,
        "InvalidParameterException" => FailureKind::InvalidInput,
        _ => FailureKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(code: &str) -> FailureKind {
        classify(ErrorSignal::ServiceCode(code))
    }

    #[test]
    fn expired_or_missing_credentials_need_authentication() {
        assert_eq!(code("ExpiredTokenException"), FailureKind::AuthRequired);
        assert_eq!(code("ExpiredToken"), FailureKind::AuthRequired);
        assert_eq!(
            code("UnrecognizedClientException"),
            FailureKind::AuthRequired
        );
        assert_eq!(
            classify(ErrorSignal::CredentialsUnavailable),
            FailureKind::AuthRequired
        );
    }

    #[test]
    fn access_denied_maps_to_access_denied() {
        assert_eq!(code("AccessDeniedException"), FailureKind::AccessDenied);
    }

    #[test]
    fn throttling_maps_to_throttled() {
        assert_eq!(code("ThrottlingException"), FailureKind::Throttled);
    }

    #[test]
    fn timeouts_and_connection_failures_map_to_network() {
        assert_eq!(classify(ErrorSignal::Timeout), FailureKind::Network);
        assert_eq!(
            classify(ErrorSignal::ConnectionFailed),
            FailureKind::Network
        );
    }

    #[test]
    fn missing_resources_map_to_not_found() {
        assert_eq!(code("ResourceNotFoundException"), FailureKind::NotFound);
    }

    #[test]
    fn invalid_parameters_map_to_invalid_input() {
        assert_eq!(code("InvalidParameterException"), FailureKind::InvalidInput);
    }

    #[test]
    fn everything_else_is_other() {
        assert_eq!(code("ServiceUnavailableException"), FailureKind::Other);
        assert_eq!(code(""), FailureKind::Other);
        assert_eq!(classify(ErrorSignal::Unknown), FailureKind::Other);
    }

    #[test]
    fn only_throttled_and_network_are_retryable() {
        let retryable: Vec<_> = FailureKind::ALL
            .into_iter()
            .filter(|k| k.is_retryable())
            .collect();
        assert_eq!(
            retryable,
            vec![FailureKind::Throttled, FailureKind::Network]
        );
    }
}
