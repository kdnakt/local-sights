//! Shared validation of fetch conditions (FetchRequest).
//!
//! Both the desktop app (through AppSession) and the `fetch_check` example
//! call [`validate_fetch_input`], so neither depends on the other (BR1.8).
//! Validation failures are reported as message-catalog keys.

use serde::{Deserialize, Serialize};

use crate::time_range::{TimeRange, parse_utc_seconds};

/// Maximum length (characters) of a log group or log stream name.
pub const MAX_NAME_LEN: usize = 512;

/// Raw, unvalidated input as typed by the user or passed on the command line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchInput {
    /// AWS profile name; blank means "SDK default" (BR1.5).
    pub profile_name: String,
    /// Log group name (required).
    pub log_group_name: String,
    /// Log stream name (required).
    pub log_stream_name: String,
    /// Start date and time, `yyyy-mm-dd hh:mm:ss` in UTC.
    pub start_text: String,
    /// End date and time, `yyyy-mm-dd hh:mm:ss` in UTC.
    pub end_text: String,
}

/// One reason why the input cannot be fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationError {
    /// BR1.1: the log group name is blank.
    LogGroupRequired,
    /// The log group name is longer than [`MAX_NAME_LEN`].
    LogGroupTooLong,
    /// BR1.1: the log stream name is blank.
    LogStreamRequired,
    /// The log stream name is longer than [`MAX_NAME_LEN`].
    LogStreamTooLong,
    /// BR1.2: the start is not a valid `yyyy-mm-dd hh:mm:ss`.
    StartFormat,
    /// BR1.2: the end is not a valid `yyyy-mm-dd hh:mm:ss`.
    EndFormat,
    /// BR1.3: the start is not before the end.
    RangeOrder,
}

/// Validated, trimmed conditions of one fetch (one stream in U1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchRequest {
    profile_name: Option<String>,
    log_group_name: String,
    log_stream_name: String,
}

/// A validated request together with its derived time range (BR1.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedFetch {
    /// The validated request.
    pub request: FetchRequest,
    /// The derived millisecond range.
    pub range: TimeRange,
}

impl ValidationError {
    /// Every variant, in display order.
    pub const ALL: [ValidationError; 7] = [
        ValidationError::LogGroupRequired,
        ValidationError::LogGroupTooLong,
        ValidationError::LogStreamRequired,
        ValidationError::LogStreamTooLong,
        ValidationError::StartFormat,
        ValidationError::EndFormat,
        ValidationError::RangeOrder,
    ];

    /// Message-catalog key of this reason.
    pub fn message_key(self) -> &'static str {
        match self {
            ValidationError::LogGroupRequired => "validation.logGroupRequired",
            ValidationError::LogGroupTooLong => "validation.logGroupTooLong",
            ValidationError::LogStreamRequired => "validation.logStreamRequired",
            ValidationError::LogStreamTooLong => "validation.logStreamTooLong",
            ValidationError::StartFormat => "validation.startFormat",
            ValidationError::EndFormat => "validation.endFormat",
            ValidationError::RangeOrder => "validation.rangeOrder",
        }
    }
}

impl FetchRequest {
    /// Builds a request directly; prefer [`validate_fetch_input`].
    pub fn new(
        profile_name: Option<String>,
        log_group_name: String,
        log_stream_name: String,
    ) -> Self {
        Self {
            profile_name,
            log_group_name,
            log_stream_name,
        }
    }

    /// Profile name, or `None` to use the SDK default (BR1.5).
    pub fn profile_name(&self) -> Option<&str> {
        self.profile_name.as_deref()
    }

    /// Log group name.
    pub fn log_group_name(&self) -> &str {
        &self.log_group_name
    }

    /// Log stream name.
    pub fn log_stream_name(&self) -> &str {
        &self.log_stream_name
    }
}

/// Validates the raw input (BR1.1-BR1.3) and derives the time range
/// (BR2.1, BR2.2). On failure, returns every reason found.
pub fn validate_fetch_input(input: &FetchInput) -> Result<ValidatedFetch, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let log_group = check_name(
        &input.log_group_name,
        ValidationError::LogGroupRequired,
        ValidationError::LogGroupTooLong,
        &mut errors,
    );
    let log_stream = check_name(
        &input.log_stream_name,
        ValidationError::LogStreamRequired,
        ValidationError::LogStreamTooLong,
        &mut errors,
    );
    let start = record_error(
        parse_utc_seconds(&input.start_text),
        ValidationError::StartFormat,
        &mut errors,
    );
    let end = record_error(
        parse_utc_seconds(&input.end_text),
        ValidationError::EndFormat,
        &mut errors,
    );
    let range = match (start, end) {
        (Some(start), Some(end)) => record_error(
            TimeRange::from_seconds(start, end),
            ValidationError::RangeOrder,
            &mut errors,
        ),
        _ => None,
    };
    match (log_group, log_stream, range) {
        (Some(log_group), Some(log_stream), Some(range)) if errors.is_empty() => {
            Ok(ValidatedFetch {
                request: FetchRequest::new(
                    normalize_profile(&input.profile_name),
                    log_group,
                    log_stream,
                ),
                range,
            })
        }
        _ => Err(errors),
    }
}

/// Keeps the value of `result`, or records `error` when it failed.
fn record_error<T, E>(
    result: Result<T, E>,
    error: ValidationError,
    errors: &mut Vec<ValidationError>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(_) => {
            errors.push(error);
            None
        }
    }
}

/// Trims the profile name; blank means "use the SDK default" (BR1.5).
fn normalize_profile(raw: &str) -> Option<String> {
    let profile = raw.trim();
    (!profile.is_empty()).then(|| profile.to_string())
}

/// Trims a required name and checks it is present and not too long.
fn check_name(
    raw: &str,
    missing: ValidationError,
    too_long: ValidationError,
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let name = raw.trim();
    if name.is_empty() {
        errors.push(missing);
        None
    } else if name.chars().count() > MAX_NAME_LEN {
        errors.push(too_long);
        None
    } else {
        Some(name.to_string())
    }
}

/// Converts reasons to their message-catalog keys.
pub fn message_keys(errors: &[ValidationError]) -> Vec<String> {
    errors
        .iter()
        .map(|error| error.message_key().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> FetchInput {
        FetchInput {
            profile_name: "dev".to_string(),
            log_group_name: "/aws/lambda/orders".to_string(),
            log_stream_name: "2024/01/02/[$LATEST]abc".to_string(),
            start_text: "2024-01-02 03:04:05".to_string(),
            end_text: "2024-01-02 03:05:05".to_string(),
        }
    }

    #[test]
    fn valid_input_yields_request_and_range() {
        let validated = validate_fetch_input(&valid_input());
        let validated = validated.unwrap();
        assert_eq!(validated.request.profile_name(), Some("dev"));
        assert_eq!(validated.request.log_group_name(), "/aws/lambda/orders");
        assert_eq!(
            validated.request.log_stream_name(),
            "2024/01/02/[$LATEST]abc"
        );
        assert_eq!(validated.range.start_instant(), 1_704_164_645_000);
        assert_eq!(validated.range.end_instant(), 1_704_164_705_999);
    }

    #[test]
    fn blank_group_and_stream_are_required() {
        let mut input = valid_input();
        input.log_group_name = "   ".to_string();
        input.log_stream_name = String::new();
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![
                ValidationError::LogGroupRequired,
                ValidationError::LogStreamRequired
            ])
        );
    }

    #[test]
    fn names_are_trimmed() {
        let mut input = valid_input();
        input.log_group_name = "  g  ".to_string();
        input.log_stream_name = "\ts\n".to_string();
        let validated = validate_fetch_input(&input).unwrap();
        assert_eq!(validated.request.log_group_name(), "g");
        assert_eq!(validated.request.log_stream_name(), "s");
    }

    #[test]
    fn overlong_names_are_rejected() {
        let mut input = valid_input();
        input.log_group_name = "g".repeat(MAX_NAME_LEN + 1);
        input.log_stream_name = "s".repeat(MAX_NAME_LEN);
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![ValidationError::LogGroupTooLong])
        );
    }

    #[test]
    fn profile_is_trimmed_and_blank_means_sdk_default() {
        let mut input = valid_input();
        input.profile_name = "  prod  ".to_string();
        assert_eq!(
            validate_fetch_input(&input).unwrap().request.profile_name(),
            Some("prod")
        );
        input.profile_name = "   ".to_string();
        assert_eq!(
            validate_fetch_input(&input).unwrap().request.profile_name(),
            None
        );
    }

    #[test]
    fn malformed_or_nonexistent_dates_are_format_errors() {
        let mut input = valid_input();
        input.start_text = "2024-02-30 00:00:00".to_string();
        input.end_text = "tomorrow".to_string();
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![
                ValidationError::StartFormat,
                ValidationError::EndFormat
            ])
        );
    }

    #[test]
    fn start_must_be_before_end() {
        let mut input = valid_input();
        input.end_text = input.start_text.clone();
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![ValidationError::RangeOrder])
        );
        input.end_text = "2024-01-02 03:04:04".to_string();
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![ValidationError::RangeOrder])
        );
    }

    #[test]
    fn empty_input_reports_every_reason_as_message_keys() {
        let errors = validate_fetch_input(&FetchInput::default()).unwrap_err();
        assert_eq!(
            message_keys(&errors),
            vec![
                "validation.logGroupRequired",
                "validation.logStreamRequired",
                "validation.startFormat",
                "validation.endFormat",
            ]
        );
    }

    #[test]
    fn every_reason_has_a_distinct_validation_key() {
        let keys: std::collections::HashSet<_> = ValidationError::ALL
            .iter()
            .map(|e| e.message_key())
            .collect();
        assert_eq!(keys.len(), ValidationError::ALL.len());
        assert!(keys.iter().all(|k| k.starts_with("validation.")));
    }
}
