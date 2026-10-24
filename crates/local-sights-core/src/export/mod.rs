//! Export of the shown rows as CSV or Markdown (261022-small-features U2,
//! ADR-001, ADR-004).
//!
//! [`formatter`] holds the pure writers: RFC 4180 CSV (U2:BR4.2) and a
//! Markdown table (U2:BR4.4), written row by row to any writer, with cells
//! changed only by the quoting and escaping the format needs (FR4.1.4, AS3).
//! [`exporter`] holds the one export job of the app (U2:BR4.8), the request,
//! the copied rows and their text, the notice (U2:BR4.9, review R-02) and the
//! default file name (U2:BR4.5).
//!
//! What is done under the app's locks and what is not (functional-design
//! review R-01): under the session lock (and the LogView lock for the Log
//! Management screen) only the availability check, the job turning Running,
//! the previous notice going, the copy of the source rows and the log group
//! name and zone for the file name. The rows are turned into text and
//! written outside the locks, on a blocking thread (NFR2). The dialog, the
//! file and the clipboard belong to the app (`src-tauri`), not to this crate.

pub mod exporter;
pub mod formatter;
