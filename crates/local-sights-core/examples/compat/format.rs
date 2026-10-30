//! The app's values written the way Logs Insights writes them
//! (261017-insights-query U4:BR2.2), and one row as both sides are compared
//! ([`Row`]).
//!
//! `@timestamp` is UTC `YYYY-MM-DD HH:MM:SS.sss` (U2 Q1b's output), a JSON
//! number is written as serde_json writes it (whole numbers exactly,
//! decimals in their shortest form: the text written in the log is not kept
//! by the app, so `1.0` or `1e3` would come out as `1.0` and `1000.0`), a
//! boolean as `1` / `0` as Logs Insights writes it (the cells the app shows
//! since U2 loop-back 2, compat run p04) and a string as it is. A column without a
//! value is left out of the row, as Logs Insights leaves the field out of
//! its row. `@ptr` is the console's own field and never compared (BR2.1).

use chrono::DateTime;
use local_sights_core::query::evaluator::{ResultRow, row_cells};
use local_sights_core::query::plan::FieldRef;

/// The console's pointer to the event, never compared (BR2.1).
pub const PTR_FIELD: &str = "@ptr";

/// One result row as compared: `(column, value)` pairs in column name
/// order, only the columns with a value, never `@ptr`. A row without any
/// pair is "a row without columns" on both sides (review R-06).
pub type Row = Vec<(String, String)>;

/// BR2.2: epoch milliseconds as Logs Insights writes `@timestamp`, UTC
/// `YYYY-MM-DD HH:MM:SS.sss`; a time outside the calendar as the number.
pub fn insights_timestamp(timestamp: i64) -> String {
    match DateTime::from_timestamp_millis(timestamp) {
        Some(instant) => instant.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        None => timestamp.to_string(),
    }
}

/// A Logs Insights result row (`field`, `value` pairs in its order) as a
/// [`Row`]: `@ptr` left out, the rest in column name order.
pub fn insights_row(cells: Vec<(String, String)>) -> Row {
    let mut row: Row = cells
        .into_iter()
        .filter(|(field, _)| field != PTR_FIELD)
        .collect();
    row.sort();
    row
}

/// The app's event row for `columns` as a [`Row`] (BR2.2): the cells as
/// the result shows them (U2:BR7.1), `@timestamp` written as Logs Insights
/// writes it, columns without a value left out. Only event rows are
/// compared here; an aggregate row (`stats`, 261025-insights-stats U1) is
/// compared from U4 and gives no pair.
pub fn app_row(columns: &[FieldRef], row: &ResultRow) -> Row {
    let Some(fields) = row.event_fields() else {
        return Row::new();
    };
    let mut pairs: Row = columns
        .iter()
        .zip(row_cells(columns, &fields, &insights_timestamp))
        .filter_map(|(column, cell)| cell.map(|value| (column.name().to_string(), value)))
        .collect();
    pairs.sort();
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_sights_core::event::LogEvent;
    use local_sights_core::query::parser::parse_query;

    fn result_row(timestamp: i64, message: &str) -> ResultRow {
        ResultRow::Event {
            event: LogEvent {
                timestamp,
                ingestion_time: None,
                message: message.to_string(),
                log_stream_name: "123456789012_CloudTrail_us-east-1".to_string(),
                sequence: 0,
            },
            log_group_name: "CloudTrail".to_string(),
        }
    }

    fn pairs(list: &[(&str, &str)]) -> Row {
        list.iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn the_timestamp_is_written_in_utc_with_milliseconds_as_logs_insights_does() {
        // 2026-01-03 10:26:21.414 UTC, U2 Q1b 1a.
        assert_eq!(
            insights_timestamp(1_767_435_981_414),
            "2026-01-03 10:26:21.414"
        );
        assert_eq!(
            insights_timestamp(1_767_435_981_000),
            "2026-01-03 10:26:21.000"
        );
        assert_eq!(insights_timestamp(i64::MAX), i64::MAX.to_string());
    }

    #[test]
    fn json_numbers_booleans_and_strings_are_written_as_logs_insights_writes_them() {
        let plan = parse_query(
            "fields @timestamp, eventVersion, readOnly, count, ratio, userIdentity.type, missing",
        )
        .unwrap();
        let row = result_row(
            1_767_435_981_414,
            r#"{"eventVersion":"1.08","readOnly":true,"count":42,"ratio":0.5,
                "userIdentity":{"type":"AWSService"},"missing":null}"#,
        );
        assert_eq!(
            app_row(&plan.columns(), &row),
            pairs(&[
                ("@timestamp", "2026-01-03 10:26:21.414"),
                ("count", "42"),
                ("eventVersion", "1.08"),
                ("ratio", "0.5"),
                ("readOnly", "1"),
                ("userIdentity.type", "AWSService"),
            ]),
            "true is 1 as Logs Insights writes it (compat run p04); null is no value and is \
             left out, as the console leaves it out"
        );
    }

    /// Compat run p04 (loop-back 2): Logs Insights writes a JSON `false` as
    /// `0`, also in a nested field; a string "false" stays as it is.
    #[test]
    fn json_false_is_written_as_0_and_the_string_false_as_it_is() {
        let plan = parse_query("fields readOnly, flags.managed, text").unwrap();
        let row = result_row(
            1_767_435_981_414,
            r#"{"readOnly":false,"flags":{"managed":true},"text":"false"}"#,
        );
        assert_eq!(
            app_row(&plan.columns(), &row),
            pairs(&[("flags.managed", "1"), ("readOnly", "0"), ("text", "false")])
        );
    }

    #[test]
    fn the_console_pointer_is_left_out_and_the_columns_are_put_in_name_order() {
        assert_eq!(
            insights_row(pairs(&[
                ("eventName", "Decrypt"),
                ("@timestamp", "2026-01-03 10:26:21.414"),
                ("@ptr", "CmAKJgoi...")
            ])),
            pairs(&[
                ("@timestamp", "2026-01-03 10:26:21.414"),
                ("eventName", "Decrypt")
            ])
        );
        assert_eq!(insights_row(pairs(&[("@ptr", "x")])), Row::new());
    }

    #[test]
    fn an_aggregate_row_gives_no_pair_until_u4() {
        // 261025-insights-stats U1: only event rows are compared here.
        use local_sights_core::query::stats::CellValue;
        let row = ResultRow::Aggregate {
            cells: vec![CellValue::Integer(3)],
        };
        let plan = parse_query("fields eventName").unwrap();
        assert_eq!(app_row(&plan.columns(), &row), Row::new());
    }
}
