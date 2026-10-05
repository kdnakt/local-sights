//! DateTimeInput: one start or end input field (U4, TimeRangeModel).
//!
//! An input holds the text the screen shows and, when that text could be
//! read in the chosen time zone, the instant it stands for (U4:BR1.3). The
//! fetch range, the order check and a time zone switch all use the instant,
//! never the text.
//!
//! The instant is re-derived from the text only when the user changes the
//! text ([`DateTimeInput::edit`], U4:BR1.5) and, for inputs without a
//! writable instant, when the time zone is switched
//! ([`DateTimeInput::switch_zone`], U4:BR1.4).
//!
//! Two documented exceptions to "the instant is what the text means in the
//! current zone" (review R-09):
//!
//! - After a switch, the text is rewritten from the instant and is *not*
//!   parsed again. For the later of two repeated times at the end of
//!   daylight saving time, the rewritten text would parse to the earlier
//!   instant; the input keeps the later one until the user edits the text.
//! - An instant that the new zone cannot write with a four-digit year is
//!   not kept: the text stays and is parsed again in the new zone, so the
//!   input may then hold a different instant or an error.

use chrono_tz::Tz;
use serde::Serialize;

use crate::time_range::{DateTimeParseError, format_input_in_zone, parse_in_zone};

/// Why a non-empty input has no instant (U4:BR1.2, BR2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DateTimeInputError {
    /// Not a valid `yyyy-mm-dd hh:mm:ss`.
    Format,
    /// Skipped by the start of daylight saving time in the local zone; only
    /// possible while the local zone is chosen.
    NonexistentLocalTime,
}

/// One start or end input: its text, and its instant or its error.
///
/// Invariant: an input never holds both an instant and an error, and an
/// empty (blank) text holds neither.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeInput {
    text: String,
    /// Epoch milliseconds, millisecond 0 of the second (U1:BR2.1).
    instant: Option<i64>,
    error: Option<DateTimeInputError>,
}

impl From<DateTimeParseError> for DateTimeInputError {
    fn from(error: DateTimeParseError) -> Self {
        match error {
            DateTimeParseError::Format => DateTimeInputError::Format,
            DateTimeParseError::NonexistentLocalTime => DateTimeInputError::NonexistentLocalTime,
        }
    }
}

impl DateTimeInput {
    /// An input holding `text` read as the wall-clock time of `zone`
    /// (U4:BR1.2). The text is kept exactly as given.
    pub fn parse(text: impl Into<String>, zone: Tz) -> Self {
        let text = text.into();
        if text.trim().is_empty() {
            return Self {
                text,
                instant: None,
                error: None,
            };
        }
        match parse_in_zone(&text, zone) {
            Ok(instant) => Self {
                text,
                instant: Some(instant),
                error: None,
            },
            Err(error) => Self {
                text,
                instant: None,
                error: Some(error.into()),
            },
        }
    }

    /// The user typed `text` (U4:BR1.5). A changed text is parsed in `zone`
    /// and replaces the instant and error; the same text again (a re-sent
    /// value, a redraw) changes nothing and keeps the instant. Returns
    /// whether the input changed.
    pub fn edit(&mut self, text: String, zone: Tz) -> bool {
        if text == self.text {
            return false;
        }
        *self = Self::parse(text, zone);
        true
    }

    /// The time zone was switched to `zone` (U4:BR1.4).
    ///
    /// An input with an instant that `zone` can write keeps the instant and
    /// gets the rewritten text, which is not parsed again. Any other
    /// non-empty input (a format error, a nonexistent local time, or an
    /// instant beyond the years 0000-9999 in `zone`) keeps its text and is
    /// parsed again in `zone`. An empty input stays empty.
    pub fn switch_zone(&mut self, zone: Tz) {
        if let Some(text) = self
            .instant
            .and_then(|instant| format_input_in_zone(instant, zone))
        {
            self.text = text;
            return;
        }
        *self = Self::parse(std::mem::take(&mut self.text), zone);
    }

    /// The text as shown on the screen.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The instant (epoch milliseconds), when the text could be read.
    pub fn instant(&self) -> Option<i64> {
        self.instant
    }

    /// Why a non-empty text could not be read.
    pub fn error(&self) -> Option<DateTimeInputError> {
        self.error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_range::parse_utc_seconds;

    const NEW_YORK: Tz = chrono_tz::America::New_York;
    const TOKYO: Tz = chrono_tz::Asia::Tokyo;

    fn utc(text: &str) -> i64 {
        parse_utc_seconds(text).unwrap()
    }

    #[test]
    fn empty_input_has_neither_instant_nor_error() {
        for input in [
            DateTimeInput::default(),
            DateTimeInput::parse("", NEW_YORK),
            DateTimeInput::parse("   ", Tz::UTC),
        ] {
            assert_eq!(input.instant(), None);
            assert_eq!(input.error(), None);
        }
        // The text is kept exactly as typed.
        assert_eq!(DateTimeInput::parse("   ", Tz::UTC).text(), "   ");
    }

    #[test]
    fn valid_text_holds_the_instant_in_the_chosen_zone() {
        let input = DateTimeInput::parse("2024-03-01 10:00:00", TOKYO);
        assert_eq!(input.text(), "2024-03-01 10:00:00");
        assert_eq!(input.instant(), Some(utc("2024-03-01 01:00:00")));
        assert_eq!(input.error(), None);
    }

    #[test]
    fn invalid_text_holds_an_error_and_no_instant() {
        let malformed = DateTimeInput::parse("2024-13-01 00:00:00", Tz::UTC);
        assert_eq!(malformed.error(), Some(DateTimeInputError::Format));
        assert_eq!(malformed.instant(), None);
        let skipped = DateTimeInput::parse("2024-03-10 02:30:00", NEW_YORK);
        assert_eq!(
            skipped.error(),
            Some(DateTimeInputError::NonexistentLocalTime)
        );
        assert_eq!(skipped.instant(), None);
    }

    #[test]
    fn switching_zones_rewrites_the_text_and_keeps_the_instant() {
        // Acceptance criterion: Tokyo 10:00:00 <-> UTC 01:00:00.
        let mut input = DateTimeInput::parse("2024-03-01 10:00:00", TOKYO);
        let instant = input.instant();
        input.switch_zone(Tz::UTC);
        assert_eq!(input.text(), "2024-03-01 01:00:00");
        assert_eq!(input.instant(), instant);
        input.switch_zone(TOKYO);
        assert_eq!(input.text(), "2024-03-01 10:00:00");
        assert_eq!(input.instant(), instant);
    }

    #[test]
    fn the_later_of_two_repeated_times_survives_a_round_trip() {
        // 06:30 UTC is the second 01:30 in New York (EST); the rewritten
        // text would parse to the first one, so it is not re-parsed (R-09).
        let mut input = DateTimeInput::parse("2024-11-03 06:30:00", Tz::UTC);
        input.switch_zone(NEW_YORK);
        assert_eq!(input.text(), "2024-11-03 01:30:00");
        assert_eq!(input.instant(), Some(utc("2024-11-03 06:30:00")));
        input.switch_zone(Tz::UTC);
        assert_eq!(input.text(), "2024-11-03 06:30:00");
        assert_eq!(input.instant(), Some(utc("2024-11-03 06:30:00")));
    }

    #[test]
    fn a_format_error_keeps_its_text_and_error_across_switches() {
        let mut input = DateTimeInput::parse("2024-13-01 00:00:00", NEW_YORK);
        input.switch_zone(Tz::UTC);
        assert_eq!(input.text(), "2024-13-01 00:00:00");
        assert_eq!(input.error(), Some(DateTimeInputError::Format));
        assert_eq!(input.instant(), None);
    }

    #[test]
    fn a_nonexistent_local_time_becomes_valid_in_utc() {
        // R-02: the text stays and is parsed again in the new zone.
        let mut input = DateTimeInput::parse("2024-03-10 02:30:00", NEW_YORK);
        input.switch_zone(Tz::UTC);
        assert_eq!(input.text(), "2024-03-10 02:30:00");
        assert_eq!(input.instant(), Some(utc("2024-03-10 02:30:00")));
        assert_eq!(input.error(), None);
    }

    #[test]
    fn an_instant_beyond_year_9999_keeps_its_text_and_is_parsed_again() {
        // R-05: 9999-12-31 23:00:00 UTC is year 10000 in Tokyo.
        let mut input = DateTimeInput::parse("9999-12-31 23:00:00", Tz::UTC);
        input.switch_zone(TOKYO);
        assert_eq!(input.text(), "9999-12-31 23:00:00");
        assert_eq!(input.instant(), Some(utc("9999-12-31 14:00:00")));
        assert_eq!(input.error(), None);
    }

    #[test]
    fn the_same_text_is_not_parsed_again() {
        // R-06: a re-sent, unchanged text keeps the later repeated instant.
        let mut input = DateTimeInput::parse("2024-11-03 06:30:00", Tz::UTC);
        input.switch_zone(NEW_YORK);
        assert!(!input.edit("2024-11-03 01:30:00".to_string(), NEW_YORK));
        assert_eq!(input.instant(), Some(utc("2024-11-03 06:30:00")));
        // A changed text is parsed: the repeated time takes the earlier one.
        assert!(input.edit("2024-11-03 01:30:00 ".to_string(), NEW_YORK));
        assert_eq!(input.instant(), Some(utc("2024-11-03 05:30:00")));
        assert!(input.edit(String::new(), NEW_YORK));
        assert_eq!((input.instant(), input.error()), (None, None));
    }
}
