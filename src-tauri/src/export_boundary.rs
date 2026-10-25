//! ExportBoundary: the save dialog, the atomic file write and the clipboard
//! of an export (261022-small-features U2:BR4.6, BR4.7, ADR-002).
//!
//! The dialog and the clipboard are the official Tauri plugins
//! (`tauri-plugin-dialog`, `tauri-plugin-clipboard-manager`), called from the
//! Rust side only (functional-design review R-04): the WebView is granted no
//! permission for them. Nothing here depends on the OS by `cfg`; the plugins
//! absorb the differences (NFR3). Every function runs on a blocking thread
//! with no lock held (review R-01). An OS error is written to standard error
//! only; the screen gets an [`ExportFailure`] kind and the chosen path, never
//! the OS's words (review R-02, NFR5).

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use local_sights_core::export::exporter::ExportFailure;
use local_sights_core::export::formatter::ExportFormat;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

/// The name and the extension of the save dialog's one filter (BR4.6).
pub(crate) fn dialog_filter(format: ExportFormat) -> (&'static str, &'static str) {
    match format {
        ExportFormat::Csv => ("CSV", "csv"),
        ExportFormat::Markdown => ("Markdown", "md"),
    }
}

/// BR4.6: shows the OS save dialog with `default_name` and the filter of
/// `format`; the chosen path, `None` when the user cancelled. Blocks until
/// the dialog closes, so it must run on a blocking thread, never on the main
/// thread.
///
/// # Errors
/// [`ExportFailure::Dialog`] when the dialog answered with something that is
/// not a file path.
pub(crate) fn ask_save_path(
    app: &AppHandle,
    default_name: &str,
    format: ExportFormat,
) -> Result<Option<PathBuf>, ExportFailure> {
    let (filter_name, extension) = dialog_filter(format);
    let Some(chosen) = app
        .dialog()
        .file()
        .set_file_name(default_name)
        .add_filter(filter_name, &[extension])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    match chosen.into_path() {
        Ok(path) => Ok(Some(path)),
        Err(error) => {
            eprintln!("local-sights: export: the save dialog gave no file path: {error}");
            Err(ExportFailure::Dialog)
        }
    }
}

/// BR4.7: puts `text` on the clipboard.
///
/// # Errors
/// [`ExportFailure::Clipboard`] when the clipboard refused it.
pub(crate) fn copy_to_clipboard(app: &AppHandle, text: String) -> Result<(), ExportFailure> {
    app.clipboard().write_text(text).map_err(|error| {
        eprintln!("local-sights: export: the clipboard could not be written: {error}");
        ExportFailure::Clipboard
    })
}

/// The failure kind the screen names for an I/O error (review R-02): no
/// permission, no space, a folder that is gone, or anything else.
pub(crate) fn failure_kind(kind: io::ErrorKind) -> ExportFailure {
    match kind {
        io::ErrorKind::PermissionDenied => ExportFailure::PermissionDenied,
        io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => ExportFailure::NoSpace,
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => ExportFailure::NotFound,
        _ => ExportFailure::Other,
    }
}

/// Logs `error` of `operation` (the OS's words go to standard error only)
/// and names its kind for the screen.
fn failure_of(operation: &str, error: &io::Error) -> ExportFailure {
    eprintln!("local-sights: export: could not {operation}: {error}");
    failure_kind(error.kind())
}

/// BR4.6: writes the output of `write` to `path` through a temporary file
/// `<name>.tmp-<16 hex digits>` in the same folder, flushed to disk and then
/// renamed over `path`, so a reader never sees a half-written file. On any
/// failure the temporary file is removed (a leftover is only logged) and
/// `path` is left as it was.
///
/// # Errors
/// The [`ExportFailure`] kind of the first failure.
pub(crate) fn write_file_atomically(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> Result<(), ExportFailure> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) else {
        eprintln!("local-sights: export: the chosen path has no file name");
        return Err(ExportFailure::Other);
    };
    let temp_path = dir.join(format!("{name}.tmp-{:016x}", fastrand::u64(..)));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| failure_of("create the temporary file", &error))?;
    let result = fill_and_rename(file, write, &temp_path, path);
    if result.is_err() && fs::remove_file(&temp_path).is_err() {
        eprintln!(
            "local-sights: export: a temporary file could not be removed after a failed write"
        );
    }
    result
}

fn fill_and_rename(
    file: File,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
    temp_path: &Path,
    final_path: &Path,
) -> Result<(), ExportFailure> {
    let mut out = BufWriter::new(file);
    write(&mut out).map_err(|error| failure_of("write the file", &error))?;
    let file = out
        .into_inner()
        .map_err(|error| failure_of("write the file", error.error()))?;
    file.sync_all()
        .map_err(|error| failure_of("flush the file", &error))?;
    drop(file);
    fs::rename(temp_path, final_path).map_err(|error| failure_of("rename the file", &error))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn temp_files(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp-"))
            .collect()
    }

    #[test]
    fn export_file_is_written_through_a_temporary_that_is_renamed_away() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.csv");
        write_file_atomically(&path, |out| out.write_all(b"\xEF\xBB\xBFa\r\n1\r\n")).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFa\r\n1\r\n");
        assert!(temp_files(dir.path()).is_empty());
        // Writing again replaces the file whole.
        write_file_atomically(&path, |out| out.write_all(b"b\r\n")).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"b\r\n");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn export_write_into_a_file_where_the_folder_should_be_fails_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_dir = dir.path().join("plain");
        fs::write(&not_a_dir, b"keep").unwrap();
        let failure = write_file_atomically(&not_a_dir.join("out.csv"), |out| out.write_all(b"x"))
            .unwrap_err();
        assert!(
            matches!(failure, ExportFailure::NotFound | ExportFailure::Other),
            "{failure:?}"
        );
        assert_eq!(fs::read(&not_a_dir).unwrap(), b"keep");
        assert!(temp_files(dir.path()).is_empty());
    }

    #[test]
    fn export_write_reports_not_found_for_a_missing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone").join("out.md");
        let failure = write_file_atomically(&missing, |out| out.write_all(b"x")).unwrap_err();
        assert_eq!(failure, ExportFailure::NotFound);
        assert!(!dir.path().join("gone").exists());
    }

    #[test]
    fn export_write_failure_removes_the_temporary_and_keeps_the_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.md");
        fs::write(&path, b"old").unwrap();
        let failure = write_file_atomically(&path, |out| {
            out.write_all(b"partial")?;
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no"))
        })
        .unwrap_err();
        assert_eq!(failure, ExportFailure::PermissionDenied);
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert!(temp_files(dir.path()).is_empty());
        assert_eq!(
            write_file_atomically(Path::new(""), |_| Ok(())).unwrap_err(),
            ExportFailure::Other
        );
    }

    #[test]
    fn export_failure_kinds_follow_the_io_error_kind() {
        assert_eq!(
            failure_kind(io::ErrorKind::PermissionDenied),
            ExportFailure::PermissionDenied
        );
        assert_eq!(
            failure_kind(io::ErrorKind::StorageFull),
            ExportFailure::NoSpace
        );
        assert_eq!(
            failure_kind(io::ErrorKind::QuotaExceeded),
            ExportFailure::NoSpace
        );
        assert_eq!(
            failure_kind(io::ErrorKind::NotFound),
            ExportFailure::NotFound
        );
        assert_eq!(
            failure_kind(io::ErrorKind::NotADirectory),
            ExportFailure::NotFound
        );
        assert_eq!(
            failure_kind(io::ErrorKind::Interrupted),
            ExportFailure::Other
        );
        assert_eq!(failure_kind(io::ErrorKind::Other), ExportFailure::Other);
        assert_eq!(dialog_filter(ExportFormat::Csv), ("CSV", "csv"));
        assert_eq!(dialog_filter(ExportFormat::Markdown), ("Markdown", "md"));
    }
}
