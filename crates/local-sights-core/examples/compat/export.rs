//! One exported Logs Insights result (261017-insights-query U4,
//! entities.md InsightsExport).
//!
//! `insights-export.sh` writes `<id>.json` for every query of the set:
//! `{"status": "Complete", "results": [[{"field": …, "value": …}, …], …]}`
//! as `get-query-results` returned it (statistics left out), or
//! `{"status": "Error", "message": …}` when `start-query` refused the query
//! (its request ID removed). Any other status (Failed, Cancelled, Timeout,
//! Unknown, or a query still running when the script gave up) means the
//! query did not run to its end: it is not compared (review R-02). The
//! script writes `Error` only for a malformed query; any other failure of
//! `start-query` (expired credentials, a denied access, …) is written as
//! `Unknown` with its message, so it is never taken for a refusal (code
//! review R-01).
//!
//! Every file also carries `exportedAtSeconds`, the same value the run of
//! the script wrote to `settings.json`; a file with another value (or none)
//! is left from an earlier run in the same folder and is not compared (code
//! review R-02).

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
    /// Code review R-02: the file is not from the run of `settings.json`
    /// (not compared): its `exportedAtSeconds` and that of `settings.json`.
    OtherRun {
        /// The file's `exportedAtSeconds`, if it has one.
        export: Option<i64>,
        /// The `exportedAtSeconds` of `settings.json`, if it has one.
        settings: Option<i64>,
    },
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
    #[serde(default, rename = "exportedAtSeconds")]
    exported_at_seconds: Option<i64>,
}

/// Reads one `<id>.json`, whatever run of the script wrote it (the tests
/// look at the script's files with it; `compat_check` uses
/// [`parse_export_of_run`]).
#[cfg(test)]
pub fn parse_export(json: &str) -> Result<InsightsSide, ExportError> {
    side_of(read_file(json)?)
}

/// Code review R-02: reads one `<id>.json` of the run whose
/// `exportedAtSeconds` is `run` (from `settings.json`). A file of another
/// run, or one without the value, is [`InsightsSide::OtherRun`]; so is
/// every file when `run` is `None`.
pub fn parse_export_of_run(json: &str, run: Option<i64>) -> Result<InsightsSide, ExportError> {
    let file = read_file(json)?;
    if run.is_none() || file.exported_at_seconds != run {
        return Ok(InsightsSide::OtherRun {
            export: file.exported_at_seconds,
            settings: run,
        });
    }
    side_of(file)
}

fn read_file(json: &str) -> Result<ExportFile, ExportError> {
    serde_json::from_str(json).map_err(|error| ExportError(error.to_string()))
}

fn side_of(file: ExportFile) -> Result<InsightsSide, ExportError> {
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

    /// Code review R-01: the script writes a start-query failure other than
    /// a malformed query as Unknown with its message; that query did not
    /// run and is not compared, whatever the message says.
    #[test]
    fn a_start_query_failure_written_as_unknown_did_not_run() {
        let json = r#"{"status":"Unknown","message":"An error occurred (ExpiredTokenException) when calling the StartQuery operation","results":[]}"#;
        assert_eq!(
            parse_export(json).unwrap(),
            InsightsSide::NotRun("Unknown".to_string())
        );
    }

    /// Code review R-02: an export is taken only when it carries the
    /// `exportedAtSeconds` of settings.json, so a file left in the folder by
    /// an earlier run is never compared as this run's result.
    #[test]
    fn an_export_from_another_run_is_not_taken_for_this_run() {
        let complete = |stamp: &str| {
            format!(
                r#"{{"status":"Complete","results":[[{{"field":"eventName","value":"Decrypt"}}]]{stamp}}}"#
            )
        };
        let this_run = complete(r#","exportedAtSeconds":1767450000"#);
        assert_eq!(
            parse_export_of_run(&this_run, Some(1_767_450_000)).unwrap(),
            InsightsSide::Rows(vec![vec![("eventName".to_string(), "Decrypt".to_string())]])
        );
        assert_eq!(
            parse_export_of_run(&this_run, Some(1_767_460_000)).unwrap(),
            InsightsSide::OtherRun {
                export: Some(1_767_450_000),
                settings: Some(1_767_460_000)
            }
        );
        assert_eq!(
            parse_export_of_run(&complete(""), Some(1_767_450_000)).unwrap(),
            InsightsSide::OtherRun {
                export: None,
                settings: Some(1_767_450_000)
            },
            "an export without the stamp is not taken"
        );
        assert_eq!(
            parse_export_of_run(&this_run, None).unwrap(),
            InsightsSide::OtherRun {
                export: Some(1_767_450_000),
                settings: None
            },
            "settings.json without the stamp matches no export"
        );
        let old_refusal = r#"{"status":"Error","message":"m","exportedAtSeconds":1}"#;
        assert_eq!(
            parse_export_of_run(old_refusal, Some(2)).unwrap(),
            InsightsSide::OtherRun {
                export: Some(1),
                settings: Some(2)
            }
        );
        assert!(parse_export_of_run("{", Some(1)).is_err());
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
