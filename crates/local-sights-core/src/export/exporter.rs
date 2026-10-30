//! The export job, its snapshot and the default file name
//! (261022-small-features U2:BR2.2, BR3.1, BR4.3, BR4.5, BR4.8, ADR-004).
//!
//! What happens under the locks and what happens outside them
//! (functional-design review R-01): while the app holds the session lock (and,
//! for the Log Management screen, the LogView lock) only this is done: the
//! availability check (BR2.3, BR3.2), the job turning Running with the next
//! export number, the previous notice going, the copy of the source rows
//! (the `LogEvent`s of the shown list, or the `ResultRow`s and columns of the
//! result) into an [`ExportSnapshotSource`], and the log group name and zone
//! for the default file name. The time the locks are held grows with the
//! copy of the rows only. Turning the rows into text
//! ([`ExportSnapshotSource::into_snapshot`]: the display time, `row_cells`,
//! the three columns) and writing the output ([`render_to`]) happen on a
//! blocking thread with no lock held (NFR2). A result of aggregates
//! (261025-insights-stats BR4.4) is written with its column names and the
//! cells as the table writes them.

use std::io::{self, Write};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::event::LogEvent;
use crate::export::formatter::{self, ExportFormat};
use crate::query::evaluator::{ResultColumns, ResultRow, result_row_cells};
use crate::time_range::format_display_in_zone;

/// Most characters of the log group part of a default file name (BR4.5):
/// leaves room under both OSes' path limits.
pub const MAX_FILE_STEM_CHARS: usize = 100;

/// The stem used when the log group name leaves nothing usable (BR4.5).
pub const FALLBACK_FILE_STEM: &str = "export";

/// The UTF-8 byte order mark a CSV file starts with (BR4.3).
pub const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The three columns of the Log Management screen's export (BR2.2, Q1 A).
pub const LOG_COLUMNS: [&str; 3] = ["@timestamp", "@logStream", "@message"];

/// Which screen's list an export copies (entities.md ExportRequest.screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportScreen {
    /// The Log Management screen's list (FR2).
    LogManagement,
    /// The Log Analytics screen's result (FR3).
    LogAnalytics,
}

/// Where the output goes (entities.md ExportRequest.target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportTarget {
    /// A file chosen in the save dialog (BR4.6).
    File,
    /// The clipboard (BR4.7).
    Clipboard,
}

/// One export the user asked for (entities.md ExportRequest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    /// Which screen's list.
    pub screen: ExportScreen,
    /// CSV or Markdown.
    pub format: ExportFormat,
    /// File or clipboard.
    pub target: ExportTarget,
}

impl ExportRequest {
    /// BR4.3: only a CSV saved to a file starts with the BOM.
    pub fn writes_bom(self) -> bool {
        self.format == ExportFormat::Csv && self.target == ExportTarget::File
    }
}

/// Why an export cannot start now (entities.md ExportAvailability.blocker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExportBlocker {
    /// The list or the result has no row.
    NoRows,
    /// The Log Management screen is fetching.
    Fetching,
    /// The log filter is still scanning the held events.
    Filtering,
    /// The query screen is fetching or evaluating.
    Running,
    /// The query screen shows no result.
    NoResult,
    /// Another export is under way (BR4.8).
    Exporting,
}

/// Whether an export can start on one screen (entities.md
/// ExportAvailability); computed for every view, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExportAvailability {
    /// The first reason it cannot; `None` when it can.
    pub blocker: Option<ExportBlocker>,
}

impl ExportAvailability {
    /// An export can start.
    pub const AVAILABLE: Self = Self { blocker: None };

    /// Blocked by `blocker`.
    pub fn blocked(blocker: ExportBlocker) -> Self {
        Self {
            blocker: Some(blocker),
        }
    }

    /// Whether the menu can be pressed.
    pub fn is_available(self) -> bool {
        self.blocker.is_none()
    }
}

/// Status of the one export job (entities.md ExportJob.status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ExportStatus {
    /// Nothing exported yet, or the last dialog was cancelled.
    #[default]
    Idle,
    /// An export is under way; both menus are disabled (FR4.6.1).
    Running,
    /// The last export was written or copied.
    Done,
    /// The last export failed.
    Failed,
}

/// Why an export failed, as a key the screen translates (review R-02): the
/// OS's own words never reach the screen (NFR5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExportFailure {
    /// The file or its folder may not be written.
    PermissionDenied,
    /// The disk is full or a quota was reached.
    NoSpace,
    /// The folder of the chosen path is gone.
    NotFound,
    /// Any other failure to write or rename the file, or a lost thread.
    Other,
    /// The clipboard refused the text.
    Clipboard,
    /// The save dialog could not be shown.
    Dialog,
}

/// The notice of the last export (entities.md ExportNotice): a kind, and
/// the arguments the screen fills into its text (BR4.9, review R-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum ExportNotice {
    /// The file was written.
    #[serde(rename_all = "camelCase")]
    Saved {
        /// Where it was written (the user's own path, FR4.4.4).
        path: String,
        /// Rows written, the header not counted.
        row_count: u64,
    },
    /// The text was copied.
    #[serde(rename_all = "camelCase")]
    Copied {
        /// Rows copied, the header not counted.
        row_count: u64,
    },
    /// The file could not be written (BR4.6).
    #[serde(rename_all = "camelCase")]
    SaveFailed {
        /// Why.
        failure: ExportFailure,
        /// The chosen path, when the dialog gave one.
        path: Option<String>,
    },
    /// The text could not be copied (BR4.7).
    #[serde(rename_all = "camelCase")]
    CopyFailed {
        /// Why.
        failure: ExportFailure,
    },
}

/// How an export ended, told by the app to [`ExportJob::finish`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportOutcome {
    /// The file was written at `path`.
    Saved {
        /// The written path.
        path: String,
    },
    /// The text was copied.
    Copied,
    /// The save dialog was cancelled: nothing written, no notice (FR4.4.2).
    Cancelled,
    /// The export failed; `path` is the chosen file, when there was one.
    Failed {
        /// Why.
        failure: ExportFailure,
        /// The chosen path.
        path: Option<String>,
    },
}

/// The rows copied under the locks, before they are turned into text
/// (review R-01). Only copies: nothing refers back into the LogView or the
/// result (ADR-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportSnapshotSource {
    /// The Log Management screen's shown rows (BR2.1), as the app copied
    /// them from the LogView with `rows(0, total)`; their times are written
    /// in `zone` (BR2.2).
    LogManagement {
        /// The shown events, in list order.
        events: Vec<LogEvent>,
        /// The chosen time zone.
        zone: Tz,
    },
    /// The Log Analytics screen's result (BR3.1): its columns and rows in
    /// the result's order, events or aggregates (261025-insights-stats
    /// BR4.4).
    LogAnalytics {
        /// The result columns: fields of events, or names of aggregates.
        columns: ResultColumns,
        /// The result rows.
        rows: Vec<ResultRow>,
        /// The chosen time zone.
        zone: Tz,
    },
}

impl ExportSnapshotSource {
    /// The same source with `events` as the Log Management rows: the app
    /// copies them from the LogView after [`crate::session::AppSession::begin_export`]
    /// accepted the request, so nothing is copied for a refused one. For the
    /// Log Analytics source nothing changes.
    pub fn with_events(mut self, events: Vec<LogEvent>) -> Self {
        if let Self::LogManagement { events: held, .. } = &mut self {
            *held = events;
        }
        self
    }

    /// Number of rows the snapshot will have.
    pub fn row_count(&self) -> usize {
        match self {
            Self::LogManagement { events, .. } => events.len(),
            Self::LogAnalytics { rows, .. } => rows.len(),
        }
    }

    /// Turns the copied rows into the cells of the output, with no lock
    /// held (review R-01). Log Management: the three columns of BR2.2, the
    /// time as the list shows it, the stream name and the whole message.
    /// Log Analytics: the result columns, each cell as the table shows it
    /// (`row_cells`), `@message` as the whole message, a missing value as the
    /// empty string (BR3.1); an aggregate row's cells as the table writes
    /// them, times in `zone` (261025-insights-stats BR4.4).
    pub fn into_snapshot(self) -> ExportSnapshot {
        match self {
            Self::LogManagement { events, zone } => ExportSnapshot {
                columns: LOG_COLUMNS.iter().map(|name| name.to_string()).collect(),
                rows: events
                    .into_iter()
                    .map(|event| {
                        vec![
                            format_display_in_zone(event.timestamp, zone),
                            event.log_stream_name,
                            event.message,
                        ]
                    })
                    .collect(),
            },
            Self::LogAnalytics {
                columns,
                rows,
                zone,
            } => {
                let format_time = |time| format_display_in_zone(time, zone);
                ExportSnapshot {
                    columns: columns.names(),
                    rows: rows
                        .iter()
                        .map(|row| {
                            result_row_cells(&columns, row, &format_time)
                                .into_iter()
                                .map(Option::unwrap_or_default)
                                .collect()
                        })
                        .collect(),
                }
            }
        }
    }
}

/// The cells of an export (entities.md ExportSnapshot): the header and every
/// row as text, nothing else changed (FR4.1.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSnapshot {
    /// The column headings.
    pub columns: Vec<String>,
    /// One cell per column per row.
    pub rows: Vec<Vec<String>>,
}

/// Writes `snapshot` as `format` to `out`, row by row, after the BOM when
/// `with_bom` (BR4.3: a CSV saved to a file only).
///
/// # Errors
/// The first I/O error of the writer.
pub fn render_to<W: Write>(
    out: &mut W,
    snapshot: ExportSnapshot,
    format: ExportFormat,
    with_bom: bool,
) -> io::Result<()> {
    if with_bom {
        out.write_all(UTF8_BOM)?;
    }
    formatter::write(out, format, &snapshot.columns, snapshot.rows)
}

/// What the app needs after a request was accepted (functional-spec W1 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportBegun {
    /// The number of this export; results are matched by it (BR4.8).
    pub export_number: u64,
    /// The copied rows, to turn into text outside the locks. For the Log
    /// Management screen the app fills in the events with
    /// [`ExportSnapshotSource::with_events`].
    pub source: ExportSnapshotSource,
    /// The default name of the save dialog (BR4.5); `None` for the
    /// clipboard.
    pub default_file_name: Option<String>,
}

/// The one export job of the app (entities.md ExportJob, BR4.8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportJob {
    export_number: u64,
    status: ExportStatus,
    request: Option<ExportRequest>,
    row_count: Option<u64>,
    notice: Option<ExportNotice>,
}

/// The job as the screen shows it (entities.md SessionView.export).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJobView {
    /// The number of the current or last export; 0 before the first.
    pub export_number: u64,
    /// Idle, Running, Done or Failed.
    pub status: ExportStatus,
    /// The request, `None` while Idle.
    pub request: Option<ExportRequest>,
    /// Rows of the current or last export, `None` while Idle.
    pub row_count: Option<u64>,
}

impl ExportJob {
    /// Whether an export is under way (BR4.8).
    pub fn is_running(&self) -> bool {
        self.status == ExportStatus::Running
    }

    /// The number of the current or last export.
    pub fn export_number(&self) -> u64 {
        self.export_number
    }

    /// The notice of the last export, if any (BR4.9).
    pub fn notice(&self) -> Option<&ExportNotice> {
        self.notice.as_ref()
    }

    /// Accepts `request` for `row_count` rows: Running, the next number,
    /// the previous notice gone (BR4.8, BR4.9). The caller checked the
    /// availability first.
    pub fn begin(&mut self, request: ExportRequest, row_count: u64) -> u64 {
        self.export_number += 1;
        self.status = ExportStatus::Running;
        self.request = Some(request);
        self.row_count = Some(row_count);
        self.notice = None;
        self.export_number
    }

    /// Applies the end of export `export_number` (functional-spec state
    /// machine): Cancelled returns to Idle without a notice, Saved and
    /// Copied make Done with their notice, Failed makes Failed with its
    /// notice. A result of another number changes nothing and returns
    /// `false`.
    pub fn finish(&mut self, export_number: u64, outcome: ExportOutcome) -> bool {
        if !self.is_running() || export_number != self.export_number {
            return false;
        }
        let row_count = self.row_count.unwrap_or_default();
        let target = self.request.map(|request| request.target);
        let (status, notice) = match outcome {
            ExportOutcome::Cancelled => (ExportStatus::Idle, None),
            ExportOutcome::Saved { path } => (
                ExportStatus::Done,
                Some(ExportNotice::Saved { path, row_count }),
            ),
            ExportOutcome::Copied => (ExportStatus::Done, Some(ExportNotice::Copied { row_count })),
            ExportOutcome::Failed { failure, path } => {
                let notice = if target == Some(ExportTarget::Clipboard) {
                    ExportNotice::CopyFailed { failure }
                } else {
                    ExportNotice::SaveFailed { failure, path }
                };
                (ExportStatus::Failed, Some(notice))
            }
        };
        self.status = status;
        self.notice = notice;
        if status == ExportStatus::Idle {
            self.request = None;
            self.row_count = None;
        }
        true
    }

    /// The user closed the notice (BR4.9).
    pub fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    /// The job for the screen.
    pub fn view(&self) -> ExportJobView {
        ExportJobView {
            export_number: self.export_number,
            status: self.status,
            request: self.request,
            row_count: self.row_count,
        }
    }
}

/// BR4.5: `name` as a safe file stem. `/ \ : * ? " < > |` and the control
/// characters (U+0000-U+001F, U+007F) become `_`, the result is cut at
/// [`MAX_FILE_STEM_CHARS`] characters (never inside one), leading and
/// trailing spaces and dots are removed, and an empty result becomes
/// [`FALLBACK_FILE_STEM`]. Pure.
pub fn sanitize_file_stem(name: &str) -> String {
    let replaced: String = name
        .chars()
        .take(MAX_FILE_STEM_CHARS)
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let trimmed = replaced.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        FALLBACK_FILE_STEM.to_string()
    } else {
        trimmed.to_string()
    }
}

/// BR4.5: the default name of the saved file,
/// `<stem>_<yyyyMMdd-HHmmss>.<ext>`: the sanitized log group name (or
/// [`FALLBACK_FILE_STEM`] without one), the time the snapshot was taken
/// written in `zone` (the chosen time zone, Q3 A), and the format's
/// extension. Pure.
pub fn default_file_name(
    log_group_name: Option<&str>,
    taken_at: DateTime<Utc>,
    zone: Tz,
    format: ExportFormat,
) -> String {
    let stem = sanitize_file_stem(log_group_name.unwrap_or_default());
    let time = taken_at.with_timezone(&zone).format("%Y%m%d-%H%M%S");
    format!("{stem}_{time}.{}", format.extension())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use chrono_tz::Tz;

    use super::*;
    use crate::export::formatter::ExportFormat;

    /// 2026-10-24T12:30:45Z: 21:30:45 in Tokyo.
    fn taken_at() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 24, 12, 30, 45).unwrap()
    }

    #[test]
    fn sanitize_file_stem_replaces_unsafe_characters_with_underscore() {
        assert_eq!(
            sanitize_file_stem("/aws/lambda/x:y*z?"),
            "_aws_lambda_x_y_z_"
        );
        assert_eq!(sanitize_file_stem("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
        assert_eq!(sanitize_file_stem("back\\slash"), "back_slash");
        assert_eq!(
            sanitize_file_stem("tab\there\u{7f}del\u{1}"),
            "tab_here_del_"
        );
        // Letters of any script, digits, `-`, `_` and inner spaces stay.
        assert_eq!(sanitize_file_stem("my-func_01 日本語"), "my-func_01 日本語");
    }

    #[test]
    fn sanitize_file_stem_trims_dots_and_spaces_and_falls_back_to_export() {
        assert_eq!(sanitize_file_stem(" .name. "), "name");
        assert_eq!(sanitize_file_stem(""), "export");
        assert_eq!(sanitize_file_stem("..."), "export");
        assert_eq!(sanitize_file_stem("   "), "export");
        assert_eq!(sanitize_file_stem("/"), "_");
    }

    #[test]
    fn sanitize_file_stem_cuts_at_100_characters() {
        let long = "a".repeat(101);
        assert_eq!(sanitize_file_stem(&long).chars().count(), 100);
        // Cut by characters, never inside a multi-byte one.
        let mixed = format!("{}{}", "x".repeat(99), "日本");
        let stem = sanitize_file_stem(&mixed);
        assert_eq!(stem.chars().count(), 100);
        assert!(stem.ends_with("x日"));
        let exact = "字".repeat(100);
        assert_eq!(sanitize_file_stem(&exact), exact);
        // The cut is made before trimming, so a trailing dot at the cut goes.
        let dotted = format!("{}.rest", "b".repeat(99));
        assert_eq!(sanitize_file_stem(&dotted), "b".repeat(99));
    }

    #[test]
    fn default_file_name_joins_stem_time_and_extension() {
        assert_eq!(
            default_file_name(
                Some("/aws/lambda/my-func"),
                taken_at(),
                chrono_tz::Asia::Tokyo,
                ExportFormat::Csv
            ),
            "_aws_lambda_my-func_20261024-213045.csv"
        );
        assert_eq!(
            default_file_name(
                Some("/aws/lambda/my-func"),
                taken_at(),
                chrono_tz::Asia::Tokyo,
                ExportFormat::Markdown
            ),
            "_aws_lambda_my-func_20261024-213045.md"
        );
    }

    #[test]
    fn default_file_name_writes_the_time_in_the_chosen_zone() {
        let local = default_file_name(
            Some("g"),
            taken_at(),
            chrono_tz::Asia::Tokyo,
            ExportFormat::Csv,
        );
        let utc = default_file_name(Some("g"), taken_at(), Tz::UTC, ExportFormat::Csv);
        assert_eq!(local, "g_20261024-213045.csv");
        assert_eq!(utc, "g_20261024-123045.csv");
        assert_ne!(local, utc);
    }

    fn snapshot() -> ExportSnapshot {
        ExportSnapshot {
            columns: vec!["a".to_string()],
            rows: vec![vec!["1".to_string()]],
        }
    }

    #[test]
    fn render_prepends_a_bom_only_for_csv_files() {
        let mut csv_file = Vec::new();
        render_to(&mut csv_file, snapshot(), ExportFormat::Csv, true).unwrap();
        assert_eq!(csv_file, b"\xEF\xBB\xBFa\r\n1\r\n");
        let mut csv_clipboard = Vec::new();
        render_to(&mut csv_clipboard, snapshot(), ExportFormat::Csv, false).unwrap();
        assert_eq!(csv_clipboard, b"a\r\n1\r\n");
        let mut markdown = Vec::new();
        render_to(&mut markdown, snapshot(), ExportFormat::Markdown, false).unwrap();
        assert_eq!(markdown, b"| a |\n| --- |\n| 1 |\n");
        let request = |format, target| ExportRequest {
            screen: ExportScreen::LogManagement,
            format,
            target,
        };
        assert!(request(ExportFormat::Csv, ExportTarget::File).writes_bom());
        assert!(!request(ExportFormat::Csv, ExportTarget::Clipboard).writes_bom());
        assert!(!request(ExportFormat::Markdown, ExportTarget::File).writes_bom());
    }

    #[test]
    fn the_request_and_the_job_are_spelled_for_the_screen() {
        let request = ExportRequest {
            screen: ExportScreen::LogAnalytics,
            format: ExportFormat::Markdown,
            target: ExportTarget::Clipboard,
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({ "screen": "logAnalytics", "format": "markdown", "target": "clipboard" })
        );
        let parsed: ExportRequest = serde_json::from_str(
            "{\"screen\":\"logManagement\",\"format\":\"csv\",\"target\":\"file\"}",
        )
        .unwrap();
        assert_eq!(parsed.screen, ExportScreen::LogManagement);
        assert_eq!(parsed.target, ExportTarget::File);
        let mut job = ExportJob::default();
        assert_eq!(
            serde_json::to_value(job.view()).unwrap(),
            serde_json::json!({ "exportNumber": 0, "status": "Idle", "request": null, "rowCount": null })
        );
        assert_eq!(job.begin(parsed, 7), 1);
        assert!(job.is_running());
        assert_eq!(
            serde_json::to_value(job.view()).unwrap()["request"]["format"],
            serde_json::json!("csv")
        );
    }

    // ---- 261025-insights-stats U1:BR4.4 ----------------------------------

    use crate::query::plan::FieldRef;
    use crate::query::stats::CellValue;

    /// An aggregates result of two rows: a bin, a count, an average and a
    /// value missing in the second row.
    fn aggregates_source(zone: Tz) -> ExportSnapshotSource {
        let cells = |time: i64, count: i64, average: f64, last: CellValue| ResultRow::Aggregate {
            cells: vec![
                CellValue::Timestamp(time),
                CellValue::Integer(count),
                CellValue::Decimal(average),
                last,
            ],
        };
        ExportSnapshotSource::LogAnalytics {
            columns: ResultColumns::Aggregates(vec![
                "bin(5m)".to_string(),
                "count(*)".to_string(),
                "avg(latency)".to_string(),
                "max(level)".to_string(),
            ]),
            rows: vec![
                cells(
                    1_756_051_200_000,
                    3,
                    200.333_333,
                    CellValue::Text("warn, high".to_string()),
                ),
                cells(1_756_051_500_000, 1, 0.1 + 0.2, CellValue::Missing),
            ],
            zone,
        }
    }

    #[test]
    fn an_aggregates_result_is_written_as_csv_with_the_table_text() {
        let snapshot = aggregates_source(Tz::UTC).into_snapshot();
        assert_eq!(
            snapshot.columns,
            vec!["bin(5m)", "count(*)", "avg(latency)", "max(level)"]
        );
        let mut csv = Vec::new();
        render_to(&mut csv, snapshot, ExportFormat::Csv, false).unwrap();
        assert_eq!(
            String::from_utf8(csv).unwrap(),
            "bin(5m),count(*),avg(latency),max(level)\r\n\
             2025/08/24 16:00:00.000,3,200.3333,\"warn, high\"\r\n\
             2025/08/24 16:05:00.000,1,0.3,\r\n"
        );
    }

    #[test]
    fn an_aggregates_result_is_written_as_markdown_with_times_in_the_zone() {
        let snapshot = aggregates_source(chrono_tz::Asia::Tokyo).into_snapshot();
        let mut markdown = Vec::new();
        render_to(&mut markdown, snapshot, ExportFormat::Markdown, false).unwrap();
        assert_eq!(
            String::from_utf8(markdown).unwrap(),
            "| bin(5m) | count(*) | avg(latency) | max(level) |\n\
             | --- | --- | --- | --- |\n\
             | 2025/08/25 01:00:00.000 | 3 | 200.3333 | warn, high |\n\
             | 2025/08/25 01:05:00.000 | 1 | 0.3 |  |\n"
        );
    }

    #[test]
    fn an_events_result_is_still_written_from_its_fields() {
        // BR5.1: the events path is as it was.
        let source = ExportSnapshotSource::LogAnalytics {
            columns: ResultColumns::Events(vec![FieldRef::Timestamp, FieldRef::Message]),
            rows: vec![ResultRow::Event {
                event: LogEvent {
                    timestamp: 1_704_164_645_123,
                    ingestion_time: None,
                    message: "first\nsecond".to_string(),
                    log_stream_name: "s".to_string(),
                    sequence: 0,
                },
                log_group_name: "/g".to_string(),
            }],
            zone: Tz::UTC,
        };
        assert_eq!(source.row_count(), 1);
        assert_eq!(
            source.into_snapshot(),
            ExportSnapshot {
                columns: vec!["@timestamp".to_string(), "@message".to_string()],
                rows: vec![vec![
                    "2024/01/02 03:04:05.123".to_string(),
                    "first\nsecond".to_string()
                ]],
            }
        );
    }

    #[test]
    fn default_file_name_without_a_log_group_uses_export() {
        assert_eq!(
            default_file_name(None, taken_at(), Tz::UTC, ExportFormat::Markdown),
            "export_20261024-123045.md"
        );
        assert_eq!(
            default_file_name(Some(""), taken_at(), Tz::UTC, ExportFormat::Csv),
            "export_20261024-123045.csv"
        );
    }
}
