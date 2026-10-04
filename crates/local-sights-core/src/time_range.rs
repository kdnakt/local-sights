//! TimeRangeModel (U1: UTC only).
//!
//! Parses `yyyy-mm-dd hh:mm:ss` as UTC (BR1.2), derives the millisecond range
//! used for fetching (BR2.1, BR2.2) and formats timestamps for display
//! (BR5.3). Local time zones arrive in U4.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use serde::Serialize;

/// Length of `yyyy-mm-dd hh:mm:ss`.
const INPUT_LEN: usize = 19;
/// Milliseconds added to the end second so the whole second is included.
const LAST_MS_OF_SECOND: i64 = 999;

/// Millisecond range of one fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    start_instant: i64,
    end_instant: i64,
}

/// The text is not a valid `yyyy-mm-dd hh:mm:ss` date and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected a valid date and time in the form yyyy-mm-dd hh:mm:ss")]
pub struct DateTimeFormatError;

/// The start is not before the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the start must be before the end")]
pub struct RangeOrderError;

/// Parses `yyyy-mm-dd hh:mm:ss` as UTC and returns the epoch milliseconds of
/// millisecond 0 of that second.
pub fn parse_utc_seconds(text: &str) -> Result<i64, DateTimeFormatError> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() != INPUT_LEN || !has_input_shape(bytes) {
        return Err(DateTimeFormatError);
    }
    let field = |range: std::ops::Range<usize>| -> Result<u32, DateTimeFormatError> {
        text.get(range)
            .and_then(|digits| digits.parse::<u32>().ok())
            .ok_or(DateTimeFormatError)
    };
    let year = i32::try_from(field(0..4)?).map_err(|_| DateTimeFormatError)?;
    let date = NaiveDate::from_ymd_opt(year, field(5..7)?, field(8..10)?);
    // `from_hms_opt` rejects hour 24, minute 60 and second 60 (no leap seconds).
    let time = NaiveTime::from_hms_opt(field(11..13)?, field(14..16)?, field(17..19)?);
    match (date, time) {
        (Some(date), Some(time)) => Ok(NaiveDateTime::new(date, time).and_utc().timestamp_millis()),
        _ => Err(DateTimeFormatError),
    }
}

/// Checks the fixed `dddd-dd-dd dd:dd:dd` shape (digits and separators only).
fn has_input_shape(bytes: &[u8]) -> bool {
    bytes.iter().enumerate().all(|(index, byte)| match index {
        4 | 7 => *byte == b'-',
        10 => *byte == b' ',
        13 | 16 => *byte == b':',
        _ => byte.is_ascii_digit(),
    })
}

/// Formats epoch milliseconds as UTC `yyyy-mm-dd hh:mm:ss.mmm`.
///
/// Timestamps outside the representable calendar range are returned as the
/// raw number so that nothing is silently dropped.
pub fn format_utc_millis(timestamp: i64) -> String {
    match DateTime::from_timestamp_millis(timestamp) {
        Some(instant) => instant.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        None => timestamp.to_string(),
    }
}

impl TimeRange {
    /// Builds the range from the start and end seconds (epoch milliseconds).
    ///
    /// The start is millisecond 0 of its second (BR2.1) and the end includes
    /// its whole second, up to millisecond 999 (BR2.2). The seconds are
    /// compared as given: the start must be strictly earlier (BR1.3).
    pub fn from_seconds(start_second: i64, end_second: i64) -> Result<Self, RangeOrderError> {
        if start_second >= end_second {
            return Err(RangeOrderError);
        }
        Ok(Self {
            start_instant: start_second,
            end_instant: end_second.saturating_add(LAST_MS_OF_SECOND),
        })
    }

    /// First millisecond of the range.
    pub fn start_instant(&self) -> i64 {
        self.start_instant
    }

    /// Last millisecond of the range (inclusive).
    pub fn end_instant(&self) -> i64 {
        self.end_instant
    }

    /// `endTime` for GetLogEvents, which excludes that exact millisecond.
    pub fn api_end_time(&self) -> i64 {
        self.end_instant.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2024-01-02 03:04:05 UTC
    const JAN_2_030405: i64 = 1_704_164_645_000;

    #[test]
    fn parses_utc_text_at_millisecond_zero() {
        assert_eq!(parse_utc_seconds("2024-01-02 03:04:05"), Ok(JAN_2_030405));
        assert_eq!(parse_utc_seconds("1970-01-01 00:00:00"), Ok(0));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            parse_utc_seconds("  2024-01-02 03:04:05 \t"),
            Ok(JAN_2_030405)
        );
    }

    #[test]
    fn rejects_wrong_shapes() {
        for text in [
            "",
            "2024-01-02",
            "2024-01-02T03:04:05",
            "2024-1-02 03:04:05",
            "2024-01-02 3:04:05",
            "2024-01-02 03:04:05.000",
            "2024/01/02 03:04:05",
            "abcd-ef-gh ij:kl:mn",
        ] {
            assert_eq!(
                parse_utc_seconds(text),
                Err(DateTimeFormatError),
                "{text:?}"
            );
        }
    }

    #[test]
    fn rejects_dates_and_times_that_do_not_exist() {
        for text in [
            "2024-13-01 00:00:00",
            "2024-02-30 00:00:00",
            "2023-02-29 00:00:00",
            "2024-01-02 24:00:00",
            "2024-01-02 23:60:00",
            "2024-01-02 23:59:60",
        ] {
            assert_eq!(
                parse_utc_seconds(text),
                Err(DateTimeFormatError),
                "{text:?}"
            );
        }
    }

    #[test]
    fn accepts_february_29_in_leap_years() {
        // 2024-02-29 12:00:00 UTC
        assert_eq!(
            parse_utc_seconds("2024-02-29 12:00:00"),
            Ok(1_709_208_000_000)
        );
    }

    #[test]
    fn range_starts_at_ms_zero_and_ends_at_ms_999() {
        let range = TimeRange::from_seconds(JAN_2_030405, JAN_2_030405 + 60_000);
        let range = range.unwrap();
        assert_eq!(range.start_instant(), JAN_2_030405);
        assert_eq!(range.end_instant(), JAN_2_030405 + 60_999);
        assert_eq!(range.api_end_time(), JAN_2_030405 + 61_000);
    }

    #[test]
    fn range_rejects_start_not_before_end() {
        assert_eq!(
            TimeRange::from_seconds(JAN_2_030405, JAN_2_030405),
            Err(RangeOrderError)
        );
        assert_eq!(
            TimeRange::from_seconds(JAN_2_030405 + 1_000, JAN_2_030405),
            Err(RangeOrderError)
        );
        assert!(TimeRange::from_seconds(JAN_2_030405, JAN_2_030405 + 1_000).is_ok());
    }

    #[test]
    fn formats_utc_with_milliseconds() {
        assert_eq!(
            format_utc_millis(JAN_2_030405 + 7),
            "2024-01-02 03:04:05.007"
        );
        assert_eq!(format_utc_millis(0), "1970-01-01 00:00:00.000");
        assert_eq!(
            format_utc_millis(1_709_208_000_999),
            "2024-02-29 12:00:00.999"
        );
    }

    #[test]
    fn formats_out_of_range_timestamps_as_raw_numbers() {
        assert_eq!(format_utc_millis(i64::MAX), i64::MAX.to_string());
    }
}
