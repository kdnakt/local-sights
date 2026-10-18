//! Date texts compared with `@timestamp` (261017-insights-query U1:BR5.9,
//! FR5.6, functional-design Q2a, Q5, Q6).
//!
//! Two forms only, an extension the console does not have:
//!
//! - `YYYY-MM-DD HH:MM:SS` and `YYYY-MM-DD HH:MM:SS.sss`, read in the time
//!   zone chosen when the query runs;
//! - `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS.sssZ`, read as UTC (the
//!   `Z` in capitals only).
//!
//! Without milliseconds the text is millisecond 0 of its second; the
//! milliseconds have exactly three digits. Unlike the time range fields, no
//! 999 ms is added (U1:BR5.9). The form is read here anew (functional-design
//! review R-03: the range fields use slashes and no milliseconds); only a
//! wall-clock time shown twice or skipped by a daylight saving change is
//! decided as the range fields decide it (the earlier instant, or not a
//! time at all).

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono_tz::Tz;

use crate::query::parser::keys;
use crate::query::plan::{
    Command, CommandKind, Condition, ConditionKind, Literal, QueryPlan, QueryProblem,
};
use crate::time_range::{DateTimeParseError, instant_in_zone};

/// Why a date text has no instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTextError {
    /// In neither accepted form, or a date or time that does not exist
    /// (month 13, hour 25).
    Format,
    /// A wall-clock time the chosen zone skips at a daylight saving change.
    Nonexistent,
}

/// A date text read, before the zone is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateText {
    /// No `Z`: a wall-clock time of the chosen zone.
    WallClock(NaiveDateTime),
    /// With `Z`: a UTC time.
    Utc(NaiveDateTime),
}

/// Whether `text` is in one of the accepted forms with a real date and
/// time; the zone is not needed for that.
pub fn is_date_text(text: &str) -> bool {
    read_date_text(text).is_some()
}

/// The epoch milliseconds of `text`, read in `zone` when it has no `Z`
/// (U1:BR5.9).
pub fn parse_date_text(text: &str, zone: Tz) -> Result<i64, DateTextError> {
    match read_date_text(text).ok_or(DateTextError::Format)? {
        DateText::Utc(naive) => Ok(naive.and_utc().timestamp_millis()),
        DateText::WallClock(naive) => instant_in_zone(&naive, zone).map_err(|error| match error {
            DateTimeParseError::Format => DateTextError::Format,
            DateTimeParseError::NonexistentLocalTime => DateTextError::Nonexistent,
        }),
    }
}

/// Reads the shape `YYYY-MM-DD?HH:MM:SS[.sss][Z]`, where `?` is a blank
/// without `Z` and `T` with it, and checks that the date and time exist.
fn read_date_text(text: &str) -> Option<DateText> {
    let bytes = text.as_bytes();
    let utc = bytes.last() == Some(&b'Z');
    let body = if utc {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    };
    let millis = match body.len() {
        19 => 0,
        23 if body[19] == b'.' => number(&body[20..23])?,
        _ => return None,
    };
    let separator = if utc { b'T' } else { b' ' };
    let shape = [
        (4, b'-'),
        (7, b'-'),
        (10, separator),
        (13, b':'),
        (16, b':'),
    ];
    if shape.iter().any(|(index, byte)| body[*index] != *byte) {
        return None;
    }
    let year = i32::try_from(number(&body[0..4])?).ok()?;
    let date = NaiveDate::from_ymd_opt(year, number(&body[5..7])?, number(&body[8..10])?)?;
    // `from_hms_milli_opt` rejects hour 24, minute 60 and second 60.
    let time = NaiveTime::from_hms_milli_opt(
        number(&body[11..13])?,
        number(&body[14..16])?,
        number(&body[17..19])?,
        millis,
    )?;
    let naive = NaiveDateTime::new(date, time);
    Some(if utc {
        DateText::Utc(naive)
    } else {
        DateText::WallClock(naive)
    })
}

/// The value of ASCII digits only (no sign, no blank, no other digits).
fn number(digits: &[u8]) -> Option<u32> {
    digits.iter().try_fold(0u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + u32::from(byte - b'0'))
    })
}

/// Turns every date text of `plan` into milliseconds in `zone`, the zone
/// chosen when the query runs (U1:BR5.9), or returns the first text that has
/// no instant.
pub fn resolve_date_texts(plan: QueryPlan, zone: Tz) -> Result<QueryPlan, QueryProblem> {
    let commands = plan
        .commands
        .into_iter()
        .map(|command| {
            Ok(Command {
                kind: match command.kind {
                    CommandKind::Filter(condition) => {
                        CommandKind::Filter(resolve_condition(condition, zone)?)
                    }
                    other => other,
                },
                span: command.span,
            })
        })
        .collect::<Result<_, QueryProblem>>()?;
    Ok(QueryPlan { commands })
}

fn resolve_condition(condition: Condition, zone: Tz) -> Result<Condition, QueryProblem> {
    // 261017-insights-query U3:BR5.1: a list is walked, not recursed into
    // pair by pair.
    let all = |parts: Vec<Condition>| -> Result<Vec<Condition>, QueryProblem> {
        parts
            .into_iter()
            .map(|part| resolve_condition(part, zone))
            .collect()
    };
    let kind = match condition.kind {
        ConditionKind::And(parts) => ConditionKind::And(all(parts)?),
        ConditionKind::Or(parts) => ConditionKind::Or(all(parts)?),
        ConditionKind::Not(inner) => ConditionKind::Not(Box::new(resolve_condition(*inner, zone)?)),
        ConditionKind::Compare { field, op, value } => ConditionKind::Compare {
            field,
            op,
            value: resolve_literal(value, zone)?,
        },
        ConditionKind::In {
            field,
            values,
            negated,
        } => ConditionKind::In {
            field,
            values: values
                .into_iter()
                .map(|value| resolve_literal(value, zone))
                .collect::<Result<_, _>>()?,
            negated,
        },
        other @ (ConditionKind::Like { .. } | ConditionKind::Match { .. }) => other,
    };
    Ok(Condition::new(kind, condition.span))
}

fn resolve_literal(literal: Literal, zone: Tz) -> Result<Literal, QueryProblem> {
    let Literal::DateText { text, span } = literal else {
        return Ok(literal);
    };
    parse_date_text(&text, zone)
        .map(Literal::Instant)
        .map_err(|error| {
            let key = match error {
                DateTextError::Format => keys::INVALID_DATE_TIME,
                DateTextError::Nonexistent => keys::NONEXISTENT_DATE_TIME,
            };
            QueryProblem::syntax(key, span)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::parse_query;
    use crate::query::plan::{CompareOp, ConditionKind, FieldRef, ProblemKind, Span};

    const TOKYO: Tz = chrono_tz::Asia::Tokyo;
    const NEW_YORK: Tz = chrono_tz::America::New_York;
    /// 2025-08-24T16:43:26Z, the edge of the Q2 check.
    const EDGE: i64 = 1_756_053_806_000;

    #[test]
    fn both_forms_are_read_with_or_without_milliseconds() {
        assert_eq!(parse_date_text("2025-08-24 16:43:26", Tz::UTC), Ok(EDGE));
        assert_eq!(
            parse_date_text("2025-08-24 16:43:26.250", Tz::UTC),
            Ok(EDGE + 250)
        );
        assert_eq!(parse_date_text("2025-08-24T16:43:26Z", TOKYO), Ok(EDGE));
        assert_eq!(
            parse_date_text("2025-08-24T16:43:26.007Z", TOKYO),
            Ok(EDGE + 7)
        );
    }

    #[test]
    fn without_milliseconds_the_text_is_millisecond_zero_and_no_999_is_added() {
        // Q5: the example of Q2a works as written; unlike the end of a
        // time range, the whole second is not added.
        assert_eq!(
            parse_date_text("2025-08-24 16:43:26", Tz::UTC),
            parse_date_text("2025-08-24 16:43:26.000", Tz::UTC)
        );
        assert_eq!(
            parse_date_text("2025-08-24T16:43:26Z", Tz::UTC),
            Ok(EDGE),
            "not EDGE + 999"
        );
    }

    #[test]
    fn a_text_without_z_is_read_in_the_chosen_zone() {
        // Acceptance criterion of FR5.6.
        assert_eq!(parse_date_text("2025-08-25 01:43:26", TOKYO), Ok(EDGE));
        assert_eq!(
            parse_date_text("2025-08-24 12:43:26", NEW_YORK),
            Ok(EDGE),
            "EDT, UTC-4"
        );
    }

    #[test]
    fn a_time_shown_twice_is_the_earlier_and_a_skipped_time_has_no_instant() {
        // New York: 01:00-02:00 twice on 2024-11-03, 02:00-03:00 skipped on
        // 2024-03-10 (as the range fields, time_range::parse_in_zone).
        assert_eq!(
            parse_date_text("2024-11-03 01:30:00", NEW_YORK),
            parse_date_text("2024-11-03T05:30:00Z", Tz::UTC),
            "EDT, the earlier of the two"
        );
        assert_eq!(
            parse_date_text("2024-03-10 02:30:00", NEW_YORK),
            Err(DateTextError::Nonexistent)
        );
        assert_eq!(
            parse_date_text("2024-03-10T02:30:00Z", NEW_YORK),
            Ok(1_710_037_800_000),
            "a UTC text is never skipped"
        );
        assert!(is_date_text("2024-03-10 02:30:00"), "its form is right");
    }

    #[test]
    fn dates_and_times_that_do_not_exist_are_form_errors() {
        for text in [
            "2025-13-01 00:00:00",
            "2025-02-29 00:00:00",
            "2025-08-24 25:00:00",
            "2025-08-24 23:60:00",
            "2025-08-24 23:59:60",
            "2025-08-32T00:00:00Z",
        ] {
            assert_eq!(
                parse_date_text(text, Tz::UTC),
                Err(DateTextError::Format),
                "{text:?}"
            );
            assert!(!is_date_text(text), "{text:?}");
        }
        assert!(is_date_text("2024-02-29 00:00:00"), "a leap day");
    }

    #[test]
    fn other_forms_are_refused() {
        for text in [
            "yesterday",
            "2025/08/24 16:43:26",
            "2025-08-24",
            "2025-08-24 16:43",
            "2025-08-24T16:43:26",
            "2025-08-24T16:43:26+09:00",
            "2025-08-24T16:43:26z",
            "2025-08-24 16:43:26Z",
            "2025-08-24 16:43:26.5",
            "2025-08-24 16:43:26.1234",
            " 2025-08-24 16:43:26",
            "２０２５-08-24 16:43:26",
        ] {
            assert_eq!(
                parse_date_text(text, Tz::UTC),
                Err(DateTextError::Format),
                "{text:?}"
            );
            assert!(!is_date_text(text), "{text:?}");
        }
    }

    #[test]
    fn the_parser_refuses_a_timestamp_text_in_another_form_where_it_is_written() {
        // Q6: a writing mistake found before anything is fetched.
        let problem = parse_query("filter @timestamp > \"yesterday\"").unwrap_err();
        assert_eq!(problem.kind, ProblemKind::Syntax);
        assert_eq!(problem.message_key, keys::INVALID_DATE_TIME);
        assert_eq!(problem.span, Span::new(1, 21, 11));
        let in_list = parse_query("filter @timestamp in [1, '2025/08/24']").unwrap_err();
        assert_eq!(in_list.message_key, keys::INVALID_DATE_TIME);
        assert_eq!(in_list.span, Span::new(1, 26, 12));
        assert!(
            parse_query("filter @message = 'yesterday'").is_ok(),
            "only texts compared with the timestamp"
        );
    }

    #[test]
    fn running_a_plan_turns_its_date_texts_into_milliseconds_in_the_zone() {
        let plan = parse_query(
            "filter @timestamp > '2025-08-25 01:43:26' and toMillis(@timestamp) in ['2025-08-24T16:43:26Z']",
        )
        .unwrap();
        let spans_before: Vec<Span> = plan.filters().map(|condition| condition.span).collect();
        let resolved = resolve_date_texts(plan, TOKYO).unwrap();
        let condition = resolved.filters().next().unwrap();
        let ConditionKind::And(parts) = &condition.kind else {
            panic!("no and: {condition:?}");
        };
        let [left, right] = parts.as_slice() else {
            panic!("not two parts: {parts:?}");
        };
        assert_eq!(
            left.kind,
            ConditionKind::Compare {
                field: FieldRef::Timestamp,
                op: CompareOp::Gt,
                value: Literal::Instant(EDGE),
            }
        );
        assert_eq!(
            right.kind,
            ConditionKind::In {
                field: FieldRef::TimestampMillis,
                values: vec![Literal::Instant(EDGE)],
                negated: false,
            }
        );
        assert_eq!(
            resolved
                .filters()
                .map(|condition| condition.span)
                .collect::<Vec<_>>(),
            spans_before,
            "the places stay (U2:BR10.3)"
        );
        let skipped = parse_query("filter not (@timestamp < \"2024-03-10 02:30:00\")").unwrap();
        let problem = resolve_date_texts(skipped, NEW_YORK).unwrap_err();
        assert_eq!(problem.kind, ProblemKind::Syntax);
        assert_eq!(problem.message_key, keys::NONEXISTENT_DATE_TIME);
        assert_eq!(problem.span, Span::new(1, 26, 21));
    }

    #[test]
    fn date_texts_of_every_filter_are_resolved_and_other_commands_stay() {
        let plan = parse_query(
            "fields @timestamp | filter @timestamp > '2025-08-25 01:43:26' \
             | sort @timestamp desc | filter @timestamp <= '2025-08-24T16:43:26Z' | limit 3",
        )
        .unwrap();
        let resolved = resolve_date_texts(plan.clone(), TOKYO).unwrap();
        assert_eq!(resolved.commands.len(), 5);
        assert_eq!(resolved.commands[0], plan.commands[0]);
        assert_eq!(resolved.commands[2], plan.commands[2]);
        assert_eq!(resolved.commands[4], plan.commands[4]);
        for condition in resolved.filters() {
            let ConditionKind::Compare { value, .. } = &condition.kind else {
                panic!("no comparison: {condition:?}");
            };
            assert_eq!(value, &Literal::Instant(EDGE));
        }
    }
}
