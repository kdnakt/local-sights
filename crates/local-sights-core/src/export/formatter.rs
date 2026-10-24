//! CSV and Markdown writers of an export (261022-small-features U2:BR4.2,
//! BR4.4). Pure: they write rows one by one to a writer, so an export of a
//! million rows is never held as one string (FR2.4, FR3.4). Cells are
//! changed only by the quoting and escaping each format needs (FR4.1.4,
//! AS3): a value starting with `=` goes out as it is.

use std::borrow::Cow;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

/// The output format of an export (entities.md ExportRequest.format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// RFC 4180 CSV (BR4.2).
    Csv,
    /// A Markdown table (BR4.4).
    Markdown,
}

impl ExportFormat {
    /// The file extension of the format (BR4.5).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Markdown => "md",
        }
    }
}

/// Writes `columns` and `rows` as `format` to `out`, one row at a time.
///
/// # Errors
/// The first I/O error of the writer.
pub fn write<W: Write>(
    out: &mut W,
    format: ExportFormat,
    columns: &[String],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> io::Result<()> {
    match format {
        ExportFormat::Csv => write_csv(out, columns, rows),
        ExportFormat::Markdown => write_markdown(out, columns, rows),
    }
}

/// BR4.2: RFC 4180. The header first, then every row, each line ended by
/// CRLF (the last one too). A cell holding a comma, a double quote, CR or LF
/// is quoted, with its double quotes doubled; every other cell goes out as
/// it is.
///
/// # Errors
/// The first I/O error of the writer.
pub fn write_csv<W: Write>(
    out: &mut W,
    columns: &[String],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> io::Result<()> {
    write_csv_line(out, columns)?;
    for row in rows {
        write_csv_line(out, &row)?;
    }
    Ok(())
}

fn write_csv_line<W: Write>(out: &mut W, cells: &[String]) -> io::Result<()> {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            out.write_all(b",")?;
        }
        out.write_all(csv_cell(cell).as_bytes())?;
    }
    out.write_all(b"\r\n")
}

/// One CSV cell (BR4.2): quoted with its double quotes doubled when it holds
/// `,`, `"`, CR or LF; untouched otherwise (AS3).
pub(crate) fn csv_cell(cell: &str) -> Cow<'_, str> {
    if cell.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", cell.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(cell)
    }
}

/// BR4.4: a Markdown table. The header line `| a | b |`, the separator
/// `| --- | --- |`, then one line per row, each ended by LF (the last one
/// too). A cell's `|` becomes `\|` and each line break (CRLF, LF or CR)
/// becomes `<br>`; an empty cell is written as the two spaces around it.
///
/// # Errors
/// The first I/O error of the writer.
pub fn write_markdown<W: Write>(
    out: &mut W,
    columns: &[String],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> io::Result<()> {
    write_markdown_line(out, columns)?;
    out.write_all(b"|")?;
    for _ in columns {
        out.write_all(b" --- |")?;
    }
    out.write_all(b"\n")?;
    for row in rows {
        write_markdown_line(out, &row)?;
    }
    Ok(())
}

fn write_markdown_line<W: Write>(out: &mut W, cells: &[String]) -> io::Result<()> {
    out.write_all(b"|")?;
    for cell in cells {
        out.write_all(b" ")?;
        out.write_all(markdown_cell(cell).as_bytes())?;
        out.write_all(b" |")?;
    }
    out.write_all(b"\n")
}

/// One Markdown cell (BR4.4): `|` escaped and every line break turned into
/// `<br>`; nothing else is touched (AS3).
pub(crate) fn markdown_cell(cell: &str) -> Cow<'_, str> {
    if !cell.contains(['|', '\r', '\n']) {
        return Cow::Borrowed(cell);
    }
    let mut escaped = String::with_capacity(cell.len() + 8);
    let mut chars = cell.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '|' => escaped.push_str("\\|"),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                escaped.push_str("<br>");
            }
            '\n' => escaped.push_str("<br>"),
            other => escaped.push(other),
        }
    }
    Cow::Owned(escaped)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::{self, Write};
    use std::rc::Rc;

    use super::*;

    fn columns(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|cell| cell.to_string()).collect()
    }

    fn csv(columns: &[String], rows: Vec<Vec<String>>) -> String {
        let mut out = Vec::new();
        write_csv(&mut out, columns, rows).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn markdown(columns: &[String], rows: Vec<Vec<String>>) -> String {
        let mut out = Vec::new();
        write_markdown(&mut out, columns, rows).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn csv_writes_a_header_then_rows_with_crlf() {
        let text = csv(
            &columns(&["@timestamp", "@message"]),
            vec![
                row(&["2026/10/24 21:30:45.000", "hello"]),
                row(&["t2", "日本語と絵文字 🎉"]),
            ],
        );
        assert_eq!(
            text,
            "@timestamp,@message\r\n2026/10/24 21:30:45.000,hello\r\nt2,日本語と絵文字 🎉\r\n"
        );
    }

    #[test]
    fn csv_quotes_cells_with_comma_quote_or_newline_and_doubles_quotes() {
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("\"q\""), "\"\"\"q\"\"\"");
        assert_eq!(csv_cell("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_cell("cr\rhere"), "\"cr\rhere\"");
        assert_eq!(csv_cell("both\r\n"), "\"both\r\n\"");
        let text = csv(&columns(&["a"]), vec![row(&["x,\"y\"\nz"])]);
        assert_eq!(text, "a\r\n\"x,\"\"y\"\"\nz\"\r\n");
    }

    #[test]
    fn csv_leaves_plain_cells_and_formula_like_values_untouched() {
        // AS3: no quoting or escaping beyond RFC 4180 (`=`, `+`, `-`, `@` stay).
        for cell in ["=SUM(1)", "+1", "-1", "@cmd", "plain text", "", "日本語"] {
            assert_eq!(csv_cell(cell), cell);
        }
        let text = csv(&columns(&["v"]), vec![row(&["=SUM(1)"]), row(&["+1"])]);
        assert_eq!(text, "v\r\n=SUM(1)\r\n+1\r\n");
    }

    #[test]
    fn csv_with_no_rows_is_just_the_header() {
        assert_eq!(
            csv(
                &columns(&["@timestamp", "@logStream", "@message"]),
                Vec::new()
            ),
            "@timestamp,@logStream,@message\r\n"
        );
    }

    #[test]
    fn markdown_writes_header_separator_and_rows() {
        let text = markdown(
            &columns(&["a", "b"]),
            vec![row(&["1", "x"]), row(&["2", "日本語"])],
        );
        assert_eq!(
            text,
            "| a | b |\n| --- | --- |\n| 1 | x |\n| 2 | 日本語 |\n"
        );
    }

    #[test]
    fn markdown_escapes_pipes_and_replaces_newlines_with_br() {
        assert_eq!(markdown_cell("x|y"), "x\\|y");
        assert_eq!(markdown_cell("a\r\nb"), "a<br>b");
        assert_eq!(markdown_cell("a\nb"), "a<br>b");
        assert_eq!(markdown_cell("a\rb"), "a<br>b");
        assert_eq!(markdown_cell("a\r\n\nb"), "a<br><br>b");
        // Nothing else is touched (AS3).
        assert_eq!(markdown_cell("=SUM(1) *bold* <b>"), "=SUM(1) *bold* <b>");
        let text = markdown(&columns(&["m"]), vec![row(&["x|y\nz"])]);
        assert_eq!(text, "| m |\n| --- |\n| x\\|y<br>z |\n");
    }

    #[test]
    fn markdown_empty_cell_is_a_single_space() {
        assert_eq!(markdown_cell(""), "");
        let text = markdown(&columns(&["a", "b"]), vec![row(&["", "v"])]);
        assert_eq!(text, "| a | b |\n| --- | --- |\n|  | v |\n");
        assert_eq!(markdown(&columns(&["a"]), Vec::new()), "| a |\n| --- |\n");
    }

    /// A writer that notes, for every write, how many rows the formatter
    /// had pulled from its iterator so far.
    struct Recording {
        pulled: Rc<Cell<usize>>,
        writes: Vec<usize>,
    }

    impl Write for Recording {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.push(self.pulled.get());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn formatter_writes_row_by_row_to_the_writer() {
        for format in [ExportFormat::Csv, ExportFormat::Markdown] {
            let pulled = Rc::new(Cell::new(0));
            let counter = Rc::clone(&pulled);
            let rows = (0..3).map(move |n| {
                counter.set(counter.get() + 1);
                vec![format!("row {n}")]
            });
            let mut out = Recording {
                pulled: Rc::clone(&pulled),
                writes: Vec::new(),
            };
            write(&mut out, format, &columns(&["c"]), rows).unwrap();
            assert_eq!(pulled.get(), 3);
            // The header went out before any row was pulled, and each row
            // was written before the next one was pulled: nothing is held
            // as one string (FR2.4, FR3.4).
            assert_eq!(out.writes.first(), Some(&0));
            for n in 1..=3 {
                assert!(
                    out.writes.contains(&n),
                    "{format:?}: no write after row {n}"
                );
            }
        }
    }

    #[test]
    fn the_format_is_spelled_in_lowercase_for_the_screen() {
        assert_eq!(
            serde_json::to_string(&ExportFormat::Csv).unwrap(),
            "\"csv\""
        );
        let parsed: ExportFormat = serde_json::from_str("\"markdown\"").unwrap();
        assert_eq!(parsed, ExportFormat::Markdown);
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Markdown.extension(), "md");
    }
}
