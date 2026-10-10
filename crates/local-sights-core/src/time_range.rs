//! TimeRangeModel: parsing, formatting and the fetch range.
//!
//! U1 parsed `yyyy-mm-dd hh:mm:ss` as UTC only ([`parse_utc_seconds`],
//! still used by the `fetch_check` example, U4:BR3.5). Since U4 every time
//! zone conversion lives here (U4:BR3.4): the screen's inputs are wall-clock
//! times in the chosen zone ([`parse_in_zone`], U4:BR1.2), inputs are
//! rewritten from their instants ([`format_input_in_zone`], U4:BR1.4) and log
//! rows are formatted for display ([`format_display_in_zone`], U4:BR3.1). The
//! screen converts nothing; it shows the strings made here.

use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use serde::Serialize;

/// Length of `yyyy-mm-dd hh:mm:ss`.
const INPUT_LEN: usize = 19;
/// Milliseconds added to the end second so the whole second is included.
const LAST_MS_OF_SECOND: i64 = 999;
/// Years that `yyyy` can write (U4:BR1.4, BR3.1).
const WRITABLE_YEARS: std::ops::RangeInclusive<i32> = 0..=9999;
/// Input format `yyyy-mm-dd hh:mm:ss`.
const INPUT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
/// Display format `yyyy-mm-dd hh:mm:ss.mmm`.
const DISPLAY_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

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

/// Why a wall-clock text has no instant in the chosen time zone (U4:BR1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DateTimeParseError {
    /// Not a valid `yyyy-mm-dd hh:mm:ss` date and time.
    #[error("expected a valid date and time in the form yyyy-mm-dd hh:mm:ss")]
    Format,
    /// A valid date and time that the zone skips at the start of daylight
    /// saving time, so no instant shows it (U4:BR2.2).
    #[error("the local time does not exist because of a daylight saving change")]
    NonexistentLocalTime,
}

/// The start is not before the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the start must be before the end")]
pub struct RangeOrderError;

/// Parses `yyyy-mm-dd hh:mm:ss` as UTC and returns the epoch milliseconds of
/// millisecond 0 of that second (U1:BR1.2; the `fetch_check` example keeps
/// using it, U4:BR3.5).
pub fn parse_utc_seconds(text: &str) -> Result<i64, DateTimeFormatError> {
    parse_wall_clock(text).map(|naive| naive.and_utc().timestamp_millis())
}

/// Parses `yyyy-mm-dd hh:mm:ss` as the wall-clock time of `zone` and returns
/// the epoch milliseconds of millisecond 0 of that second (U4:BR1.2).
///
/// Surrounding whitespace is ignored. A time shown twice (the end of
/// daylight saving time) takes the earlier instant; a time the zone skips
/// (the start of daylight saving time) is [`DateTimeParseError::NonexistentLocalTime`],
/// which is a different reason from a malformed text.
pub fn parse_in_zone(text: &str, zone: Tz) -> Result<i64, DateTimeParseError> {
    let naive = parse_wall_clock(text).map_err(|_| DateTimeParseError::Format)?;
    match zone.from_local_datetime(&naive) {
        LocalResult::Single(instant) => Ok(instant.timestamp_millis()),
        LocalResult::Ambiguous(earlier, later) => {
            Ok(earlier.timestamp_millis().min(later.timestamp_millis()))
        }
        LocalResult::None => Err(DateTimeParseError::NonexistentLocalTime),
    }
}

/// Writes an instant as the `yyyy-mm-dd hh:mm:ss` wall-clock time of `zone`
/// for an input field (U4:BR1.4), dropping the milliseconds. Returns `None`
/// when the instant falls outside the years 0000-9999 in that zone.
pub fn format_input_in_zone(instant: i64, zone: Tz) -> Option<String> {
    format_in_zone(instant, zone, INPUT_FORMAT)
}

/// Writes a log timestamp as the `yyyy-mm-dd hh:mm:ss.mmm` wall-clock time
/// of `zone`, using the offset in force at that instant (U4:BR3.1).
/// Timestamps outside the years 0000-9999 in that zone are returned as the
/// raw number, as in U1, so that nothing is silently dropped.
pub fn format_display_in_zone(timestamp: i64, zone: Tz) -> String {
    format_in_zone(timestamp, zone, DISPLAY_FORMAT).unwrap_or_else(|| timestamp.to_string())
}

/// Formats an instant in `zone` with `format`, if its year has four digits.
fn format_in_zone(instant: i64, zone: Tz, format: &str) -> Option<String> {
    let local = DateTime::from_timestamp_millis(instant)?.with_timezone(&zone);
    WRITABLE_YEARS
        .contains(&local.year())
        .then(|| local.format(format).to_string())
}

/// Parses the fixed `yyyy-mm-dd hh:mm:ss` shape into a calendar date and
/// time, checking that both exist (U1:BR1.2).
fn parse_wall_clock(text: &str) -> Result<NaiveDateTime, DateTimeFormatError> {
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
        (Some(date), Some(time)) => Ok(NaiveDateTime::new(date, time)),
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
/// raw number so that nothing is silently dropped. The screen uses
/// [`format_display_in_zone`] since U4; the `fetch_check` example keeps
/// printing UTC with this function (U4:BR3.5).
pub fn format_utc_millis(timestamp: i64) -> String {
    match DateTime::from_timestamp_millis(timestamp) {
        Some(instant) => instant.format(DISPLAY_FORMAT).to_string(),
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

    // ---- U4: wall-clock time in a chosen time zone (BR1.2, BR3.1) ----

    const NEW_YORK: Tz = chrono_tz::America::New_York;
    const TOKYO: Tz = chrono_tz::Asia::Tokyo;

    fn utc(text: &str) -> i64 {
        parse_utc_seconds(text).unwrap()
    }

    #[test]
    fn utc_zone_parses_exactly_like_u1() {
        for text in [
            "2024-01-02 03:04:05",
            "1970-01-01 00:00:00",
            " 2024-02-29 12:00:00 ",
        ] {
            assert_eq!(
                parse_in_zone(text, Tz::UTC),
                Ok(parse_utc_seconds(text).unwrap()),
                "{text:?}"
            );
        }
    }

    #[test]
    fn tokyo_wall_clock_is_nine_hours_ahead_of_utc() {
        // Acceptance criterion: 2024-03-01 10:00:00 (UTC+9) is 01:00:00 UTC.
        assert_eq!(
            parse_in_zone("2024-03-01 10:00:00", TOKYO),
            Ok(utc("2024-03-01 01:00:00"))
        );
    }

    #[test]
    fn times_skipped_by_the_dst_start_are_nonexistent_not_format_errors() {
        // New York jumps from 02:00 to 03:00 on 2024-03-10.
        for text in [
            "2024-03-10 02:00:00",
            "2024-03-10 02:30:00",
            "2024-03-10 02:59:59",
        ] {
            assert_eq!(
                parse_in_zone(text, NEW_YORK),
                Err(DateTimeParseError::NonexistentLocalTime),
                "{text:?}"
            );
        }
        // The seconds around the gap exist (EST before, EDT after).
        assert_eq!(
            parse_in_zone("2024-03-10 01:59:59", NEW_YORK),
            Ok(utc("2024-03-10 06:59:59"))
        );
        assert_eq!(
            parse_in_zone("2024-03-10 03:00:00", NEW_YORK),
            Ok(utc("2024-03-10 07:00:00"))
        );
    }

    #[test]
    fn times_repeated_by_the_dst_end_take_the_earlier_instant() {
        // 01:00-02:00 occurs twice on 2024-11-03; the first (EDT, UTC-4)
        // is the earlier instant.
        assert_eq!(
            parse_in_zone("2024-11-03 01:30:00", NEW_YORK),
            Ok(utc("2024-11-03 05:30:00"))
        );
    }

    #[test]
    fn malformed_text_is_a_format_error_in_every_zone() {
        for zone in [Tz::UTC, NEW_YORK, TOKYO] {
            for text in [
                "",
                "2024-13-01 00:00:00",
                "2024-01-02T03:04:05",
                "2024-02-30 00:00:00",
            ] {
                assert_eq!(
                    parse_in_zone(text, zone),
                    Err(DateTimeParseError::Format),
                    "{text:?} in {zone}"
                );
            }
        }
    }

    #[test]
    fn input_text_uses_the_offset_at_each_instant() {
        // One second apart across the DST start: the offset changes.
        assert_eq!(
            format_input_in_zone(utc("2024-03-10 06:59:59"), NEW_YORK),
            Some("2024-03-10 01:59:59".to_string())
        );
        assert_eq!(
            format_input_in_zone(utc("2024-03-10 07:00:00"), NEW_YORK),
            Some("2024-03-10 03:00:00".to_string())
        );
        assert_eq!(
            format_input_in_zone(utc("2024-03-01 01:00:00"), TOKYO),
            Some("2024-03-01 10:00:00".to_string())
        );
        assert_eq!(
            format_input_in_zone(JAN_2_030405, Tz::UTC),
            Some("2024-01-02 03:04:05".to_string())
        );
    }

    #[test]
    fn display_text_has_milliseconds_and_follows_dst() {
        // Winter (EST, UTC-5) and summer (EDT, UTC-4).
        assert_eq!(
            format_display_in_zone(JAN_2_030405 + 7, NEW_YORK),
            "2024-01-01 22:04:05.007"
        );
        assert_eq!(
            format_display_in_zone(utc("2024-07-01 12:00:00") + 999, NEW_YORK),
            "2024-07-01 08:00:00.999"
        );
        assert_eq!(
            format_display_in_zone(JAN_2_030405 + 7, Tz::UTC),
            format_utc_millis(JAN_2_030405 + 7)
        );
    }

    #[test]
    fn instants_outside_four_digit_years_cannot_be_written() {
        // 9999-12-31 23:00:00 UTC is year 10000 in Tokyo (R-05).
        let late = utc("9999-12-31 23:00:00");
        assert_eq!(format_input_in_zone(late, TOKYO), None);
        assert_eq!(format_display_in_zone(late, TOKYO), late.to_string());
        assert_eq!(
            format_input_in_zone(late, Tz::UTC),
            Some("9999-12-31 23:00:00".to_string())
        );
        // 0000-01-01 00:00:00 UTC is year -1 in New York.
        let early = utc("0000-01-01 00:00:00");
        assert_eq!(format_input_in_zone(early, NEW_YORK), None);
        assert_eq!(format_display_in_zone(early, NEW_YORK), early.to_string());
        assert_eq!(
            format_display_in_zone(i64::MAX, Tz::UTC),
            i64::MAX.to_string()
        );
    }
}
