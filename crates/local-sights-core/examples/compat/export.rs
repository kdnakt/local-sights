//! One exported Logs Insights result (261017-insights-query U4,
//! entities.md InsightsExport).
//!
//! `insights-export.sh` writes `<id>.json` for every query of the set:
//! `{"status": "Complete", "results": [[{"field": …, "value": …}, …], …]}`
//! as `get-query-results` returned it (statistics left out), or
//! `{"status": "Error", "message": …}` when `start-query` refused the query
//! (its request ID removed). Any other status (Failed, Cancelled, Timeout,
//! Unknown, or a query still running when the script gave up) means the
//! query did not run to its end: it is not compared (review R-02).

use std::fmt;

use serde::Deserialize;

use crate::compat::format::{Row, insights_row};

/// What one export says about its query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsightsSide {
    /// There is no `<id>.json` (R-02: not compared).
    Missing,
    /// The query did not run to its end: its status (R-02: not compared).
    NotRun(String),
    /// `start-query` refused the query (BR2.4), with its message.
    Rejected(String),
    /// The query completed with these rows, in their order.
    Rows(Vec<Row>),
}

/// Why an export file cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportError(pub String);

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the export cannot be read: {}", self.0)
    }
}

/// One `{field, value}` of a result row.
#[derive(Debug, Deserialize)]
struct Cell {
    field: String,
    #[serde(default)]
    value: Option<String>,
}

/// The file as the script writes it.
#[derive(Debug, Deserialize)]
struct ExportFile {
    status: String,
    #[serde(default)]
    results: Option<Vec<Vec<Cell>>>,
    #[serde(default)]
    message: Option<String>,
}

/// Reads one `<id>.json`.
pub fn parse_export(json: &str) -> Result<InsightsSide, ExportError> {
    let file: ExportFile =
        serde_json::from_str(json).map_err(|error| ExportError(error.to_string()))?;
    match file.status.as_str() {
        "Complete" => {
            let results = file
                .results
                .ok_or_else(|| ExportError("a Complete export has no results".to_string()))?;
            let rows = results
                .into_iter()
                .map(|cells| {
                    insights_row(
                        cells
                            .into_iter()
                            .filter_map(|cell| cell.value.map(|value| (cell.field, value)))
                            .collect(),
                    )
                })
                .collect();
            Ok(InsightsSide::Rows(rows))
        }
        "Error" => Ok(InsightsSide::Rejected(file.message.unwrap_or_default())),
        other => Ok(InsightsSide::NotRun(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_export_gives_its_rows_without_the_pointer() {
        let side = parse_export(
            r#"{"status":"Complete","results":[
                [{"field":"@timestamp","value":"2026-01-03 11:24:00.620"},
                 {"field":"eventName","value":"Decrypt"},{"field":"@ptr","value":"CmAK"}],
                [{"field":"@ptr","value":"CmAL"}]]}"#,
        )
        .unwrap();
        assert_eq!(
            side,
            InsightsSide::Rows(vec![
                vec![
                    (
                        "@timestamp".to_string(),
                        "2026-01-03 11:24:00.620".to_string()
                    ),
                    ("eventName".to_string(), "Decrypt".to_string()),
                ],
                vec![],
            ])
        );
        assert_eq!(
            parse_export(r#"{"status":"Complete","results":[]}"#).unwrap(),
            InsightsSide::Rows(vec![])
        );
    }

    #[test]
    fn a_refused_query_or_one_that_did_not_complete_says_so() {
        assert_eq!(
            parse_export(
                r#"{"status":"Error","message":"MalformedQueryException: Query cannot be compiled"}"#
            )
            .unwrap(),
            InsightsSide::Rejected("MalformedQueryException: Query cannot be compiled".to_string())
        );
        for status in ["Failed", "Cancelled", "Timeout", "Unknown", "Running"] {
            let json = format!(r#"{{"status":"{status}","results":[]}}"#);
            assert_eq!(
                parse_export(&json).unwrap(),
                InsightsSide::NotRun(status.to_string())
            );
        }
    }

    #[test]
    fn a_broken_export_is_an_error() {
        assert!(parse_export("{").is_err());
        assert!(parse_export(r#"{"results":[]}"#).is_err());
        assert!(
            parse_export(r#"{"status":"Complete"}"#).is_err(),
            "a complete export has results"
        );
    }
}
