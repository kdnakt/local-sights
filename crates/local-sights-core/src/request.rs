//! Shared validation of fetch conditions (FetchRequest).
//!
//! Both the desktop app (through AppSession) and the `fetch_check` example
//! call [`validate_fetch_input`], so neither depends on the other (BR1.8).
//! Validation failures are reported as message-catalog keys. Since U3 the
//! fetch covers the whole log group, so there is no stream name (U3:BR6.1).
//!
//! Since U4 the screen's start and end are [`DateTimeInput`]s read in the
//! chosen time zone; AppSession validates them with
//! [`validate_session_input`], which builds the range and checks the order
//! on their instants (U4:BR1.6). The `fetch_check` example still passes UTC
//! text through [`validate_fetch_input`] (U4:BR3.5). Both share one check.

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::catalog::ProfileSelector;
use crate::date_input::{DateTimeInput, DateTimeInputError};
use crate::time_range::TimeRange;

/// Maximum length (characters) of a log group name.
pub const MAX_NAME_LEN: usize = 512;

/// Raw, unvalidated input as typed by the user or passed on the command line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchInput {
    /// AWS profile name; blank means "SDK default" (U1:BR1.5). Used by the
    /// `fetch_check` example; the screen selects a profile instead (U2).
    pub profile_name: String,
    /// Log group name (required).
    pub log_group_name: String,
    /// Start date and time, `yyyy-mm-dd hh:mm:ss` in UTC (U4:BR3.5).
    pub start_text: String,
    /// End date and time, `yyyy-mm-dd hh:mm:ss` in UTC (U4:BR3.5).
    pub end_text: String,
}

/// One reason why the input cannot be fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationError {
    /// BR1.1: the log group name is blank.
    LogGroupRequired,
    /// The log group name is longer than [`MAX_NAME_LEN`].
    LogGroupTooLong,
    /// BR1.2: the start is not a valid `yyyy-mm-dd hh:mm:ss`.
    StartFormat,
    /// U4:BR2.2: the start is skipped by a daylight saving change in the
    /// local time zone.
    StartNonexistentLocalTime,
    /// BR1.2: the end is not a valid `yyyy-mm-dd hh:mm:ss`.
    EndFormat,
    /// U4:BR2.2: the end is skipped by a daylight saving change in the
    /// local time zone.
    EndNonexistentLocalTime,
    /// BR1.3: the start is not before the end.
    RangeOrder,
}

/// Validated, trimmed conditions of one fetch (the whole log group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchRequest {
    profile: ProfileSelector,
    region: Option<String>,
    log_group_name: String,
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
    /// Every variant, in the order of the conditions (U4:BR2.1).
    pub const ALL: [ValidationError; 7] = [
        ValidationError::LogGroupRequired,
        ValidationError::LogGroupTooLong,
        ValidationError::StartFormat,
        ValidationError::StartNonexistentLocalTime,
        ValidationError::EndFormat,
        ValidationError::EndNonexistentLocalTime,
        ValidationError::RangeOrder,
    ];

    /// Message-catalog key of this reason.
    pub fn message_key(self) -> &'static str {
        match self {
            ValidationError::LogGroupRequired => "validation.logGroupRequired",
            ValidationError::LogGroupTooLong => "validation.logGroupTooLong",
            ValidationError::StartFormat => "validation.startFormat",
            ValidationError::StartNonexistentLocalTime => "validation.startNonexistentLocalTime",
            ValidationError::EndFormat => "validation.endFormat",
            ValidationError::EndNonexistentLocalTime => "validation.endNonexistentLocalTime",
            ValidationError::RangeOrder => "validation.rangeOrder",
        }
    }
}

impl FetchRequest {
    /// Builds a request directly; prefer [`validate_fetch_input`].
    pub fn new(profile: ProfileSelector, region: Option<String>, log_group_name: String) -> Self {
        Self {
            profile,
            region,
            log_group_name,
        }
    }

    /// The same request with the connection chosen on the screen: the
    /// selected profile and region are always passed (U2:BR2.8).
    pub fn with_connection(mut self, profile: ProfileSelector, region: String) -> Self {
        self.profile = profile;
        self.region = Some(region);
        self
    }

    /// The profile (SdkDefault: let the SDK decide, U1:BR1.5).
    pub fn profile(&self) -> &ProfileSelector {
        &self.profile
    }

    /// Profile name, or `None` to use the SDK default (U1:BR1.5).
    pub fn profile_name(&self) -> Option<&str> {
        self.profile.profile_name()
    }

    /// Region to connect to; `None` means the profile's default region
    /// (only the `fetch_check` example omits it, U2:BR2.8).
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Log group name.
    pub fn log_group_name(&self) -> &str {
        &self.log_group_name
    }
}

/// Validates the raw input of the `fetch_check` example (BR1.1-BR1.3; no
/// stream name since U3:BR6.1) and derives the time range (BR2.1, BR2.2).
/// The times are UTC (U4:BR3.5). On failure, returns every reason found.
pub fn validate_fetch_input(input: &FetchInput) -> Result<ValidatedFetch, Vec<ValidationError>> {
    validate_inputs(
        &input.profile_name,
        &input.log_group_name,
        &DateTimeInput::parse(input.start_text.as_str(), Tz::UTC),
        &DateTimeInput::parse(input.end_text.as_str(), Tz::UTC),
    )
}

/// Validates the screen's conditions (U4:BR2.1): the log group name and the
/// start and end inputs, already read in the chosen time zone. The order
/// is checked and the range built from the instants (U4:BR1.6); the
/// time zone plays no further part. An empty input is a format error. When
/// either input has an error, the order is not checked. On failure,
/// returns every reason found, in the order of the conditions. The profile
/// is left to the caller ([`FetchRequest::with_connection`], U2:BR2.8).
pub fn validate_session_input(
    log_group_name: &str,
    start: &DateTimeInput,
    end: &DateTimeInput,
) -> Result<ValidatedFetch, Vec<ValidationError>> {
    validate_inputs("", log_group_name, start, end)
}

/// The shared check of both entry points (U1:BR1.8).
fn validate_inputs(
    profile_name: &str,
    log_group_name: &str,
    start: &DateTimeInput,
    end: &DateTimeInput,
) -> Result<ValidatedFetch, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let log_group = check_name(
        log_group_name,
        ValidationError::LogGroupRequired,
        ValidationError::LogGroupTooLong,
        &mut errors,
    );
    let start = check_instant(
        start,
        ValidationError::StartFormat,
        ValidationError::StartNonexistentLocalTime,
        &mut errors,
    );
    let end = check_instant(
        end,
        ValidationError::EndFormat,
        ValidationError::EndNonexistentLocalTime,
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
    match (log_group, range) {
        (Some(log_group), Some(range)) if errors.is_empty() => Ok(ValidatedFetch {
            request: FetchRequest::new(
                ProfileSelector::from_optional_name(Some(profile_name)),
                None,
                log_group,
            ),
            range,
        }),
        _ => Err(errors),
    }
}

/// The instant of an input, or records why it has none: a nonexistent
/// local time has its own reason (U4:BR2.2); an empty or malformed text is
/// a format error.
fn check_instant(
    input: &DateTimeInput,
    format: ValidationError,
    nonexistent: ValidationError,
    errors: &mut Vec<ValidationError>,
) -> Option<i64> {
    if let Some(instant) = input.instant() {
        return Some(instant);
    }
    errors.push(match input.error() {
        Some(DateTimeInputError::NonexistentLocalTime) => nonexistent,
        Some(DateTimeInputError::Format) | None => format,
    });
    None
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
        assert_eq!(validated.range.start_instant(), 1_704_164_645_000);
        assert_eq!(validated.range.end_instant(), 1_704_164_705_999);
    }

    #[test]
    fn blank_group_is_required_and_no_stream_name_is_needed() {
        let mut input = valid_input();
        assert!(validate_fetch_input(&input).is_ok(), "U3:BR6.1");
        input.log_group_name = "   ".to_string();
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![ValidationError::LogGroupRequired])
        );
    }

    #[test]
    fn names_are_trimmed() {
        let mut input = valid_input();
        input.log_group_name = "  g  ".to_string();
        let validated = validate_fetch_input(&input).unwrap();
        assert_eq!(validated.request.log_group_name(), "g");
    }

    #[test]
    fn overlong_names_are_rejected() {
        let mut input = valid_input();
        input.log_group_name = "g".repeat(MAX_NAME_LEN + 1);
        assert_eq!(
            validate_fetch_input(&input),
            Err(vec![ValidationError::LogGroupTooLong])
        );
        input.log_group_name = "g".repeat(MAX_NAME_LEN);
        assert!(validate_fetch_input(&input).is_ok());
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
    fn profile_argument_maps_to_sdk_default_or_named_without_region() {
        let mut input = valid_input();
        input.profile_name = String::new();
        let request = validate_fetch_input(&input).unwrap().request;
        assert_eq!(request.profile(), &ProfileSelector::SdkDefault);
        assert_eq!(request.region(), None);
        input.profile_name = "dev".to_string();
        let request = validate_fetch_input(&input).unwrap().request;
        assert_eq!(
            request.profile(),
            &ProfileSelector::Named("dev".to_string())
        );
        assert_eq!(request.region(), None);
    }

    #[test]
    fn screen_connection_replaces_profile_and_sets_region() {
        let request = validate_fetch_input(&valid_input())
            .unwrap()
            .request
            .with_connection(ProfileSelector::SdkDefault, "eu-west-1".to_string());
        assert_eq!(request.profile_name(), None);
        assert_eq!(request.region(), Some("eu-west-1"));
        assert_eq!(request.log_group_name(), "/aws/lambda/orders");
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

    // ---- U4: the screen's inputs hold instants (BR1.6, BR2.1, BR2.2) ----

    use crate::date_input::DateTimeInput;
    use chrono_tz::Tz;

    const NEW_YORK: Tz = chrono_tz::America::New_York;
    const TOKYO: Tz = chrono_tz::Asia::Tokyo;
    const GROUP: &str = "/aws/lambda/orders";

    fn utc(text: &str) -> i64 {
        crate::time_range::parse_utc_seconds(text).unwrap()
    }

    #[test]
    fn the_range_comes_from_the_instants_whatever_the_zone() {
        // As text the start sorts after the end; as instants it is earlier.
        let start = DateTimeInput::parse("2024-03-01 10:00:00", TOKYO);
        let end = DateTimeInput::parse("2024-03-01 01:00:01", Tz::UTC);
        let validated = validate_session_input(GROUP, &start, &end).unwrap();
        assert_eq!(validated.request.log_group_name(), GROUP);
        assert_eq!(validated.range.start_instant(), utc("2024-03-01 01:00:00"));
        assert_eq!(
            validated.range.end_instant(),
            utc("2024-03-01 01:00:01") + 999
        );
    }

    #[test]
    fn the_order_is_checked_on_instants() {
        // The same instant written in two zones is not "before".
        let start = DateTimeInput::parse("2024-03-01 01:00:00", Tz::UTC);
        let end = DateTimeInput::parse("2024-03-01 10:00:00", TOKYO);
        assert_eq!(
            validate_session_input(GROUP, &start, &end),
            Err(vec![ValidationError::RangeOrder])
        );
    }

    #[test]
    fn nonexistent_local_times_are_reasons_of_their_own() {
        let skipped_start = DateTimeInput::parse("2024-03-10 02:30:00", NEW_YORK);
        let skipped_end = DateTimeInput::parse("2024-03-10 02:45:00", NEW_YORK);
        assert_eq!(
            validate_session_input(GROUP, &skipped_start, &skipped_end),
            Err(vec![
                ValidationError::StartNonexistentLocalTime,
                ValidationError::EndNonexistentLocalTime
            ])
        );
        let malformed = DateTimeInput::parse("2024-13-01 00:00:00", NEW_YORK);
        assert_eq!(
            validate_session_input(GROUP, &malformed, &skipped_end),
            Err(vec![
                ValidationError::StartFormat,
                ValidationError::EndNonexistentLocalTime
            ])
        );
    }

    #[test]
    fn an_input_error_hides_the_order_reason_and_empty_inputs_are_format_errors() {
        let later = DateTimeInput::parse("2024-03-01 10:00:00", Tz::UTC);
        let malformed = DateTimeInput::parse("tomorrow", Tz::UTC);
        assert_eq!(
            validate_session_input(GROUP, &later, &malformed),
            Err(vec![ValidationError::EndFormat])
        );
        assert_eq!(
            validate_session_input("", &DateTimeInput::default(), &DateTimeInput::default()),
            Err(vec![
                ValidationError::LogGroupRequired,
                ValidationError::StartFormat,
                ValidationError::EndFormat
            ])
        );
    }

    #[test]
    fn nonexistent_reasons_have_their_own_keys_in_condition_order() {
        assert_eq!(
            ValidationError::StartNonexistentLocalTime.message_key(),
            "validation.startNonexistentLocalTime"
        );
        assert_eq!(
            ValidationError::EndNonexistentLocalTime.message_key(),
            "validation.endNonexistentLocalTime"
        );
        assert_eq!(
            ValidationError::ALL,
            [
                ValidationError::LogGroupRequired,
                ValidationError::LogGroupTooLong,
                ValidationError::StartFormat,
                ValidationError::StartNonexistentLocalTime,
                ValidationError::EndFormat,
                ValidationError::EndNonexistentLocalTime,
                ValidationError::RangeOrder,
            ]
        );
    }
}
