//! LogCache: the optional disk cache of fetched logs (U6).
//!
//! One file per [`CacheKey`] lives directly in the cache folder, named by the
//! SHA-256 of the key (BR2.2). The file is JSON Lines (code-generation Q1):
//! the first line is the [`CacheHeader`] (format version, key and covered
//! ranges), so a lookup can decide without reading the events (BR4.2); every
//! following line is one [`CachedEvent`] in `(timestamp, logStreamName,
//! sequence)` order.
//!
//! The pure decisions are in [`plan`]; this module only reads and writes
//! files. A file is written to a temporary file in the same folder and then
//! renamed over the old one, so a failed or interrupted write leaves the
//! previous cache as it was (BR3.4). The folder is created owner-only
//! (0700) and every file owner-only (0600); a looser existing folder is
//! tightened, and when that fails nothing is written (BR3.6). Only the
//! header and the events are written: nothing here takes a credential
//! (project.md Forbidden, NFR5).
//!
//! Every call blocks on the file system; the fetch runs them on a blocking
//! thread (BR3.5, BR4.2). Diagnostics go to standard error only and never
//! contain a key, a message or a credential.
//!
//! Since U7 (U6 review R-03) the header also holds the number of events in
//! the file and in each covered range (format version 2). Reading the whole
//! file compares the total, and a lookup compares the count of the range it
//! is served from, so a file cut exactly at a line boundary is broken
//! (BR4.1). Since U7 (U6 review R-02) a [`WriteTracker`] tells while a write
//! runs, even after the fetch that started it ended abnormally.

pub mod plan;
pub mod settings;

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use plan::{
    CacheHeader, CacheKey, CachedEvent, Corruption, CoveredRange, EventCheck, EventCounter,
    FORMAT_VERSION, FileNameKind, classify_file_name, count_events_by_range, find_covering_range,
    merge_ranges, replace_events_in_range, validate_header,
};

/// Permission bits of the cache and settings folders: owner only (BR3.6).
pub const PRIVATE_DIR_MODE: u32 = 0o700;
/// Permission bits of the cache and settings files: owner only (BR3.6).
pub const PRIVATE_FILE_MODE: u32 = 0o600;

/// Why a cache or settings file could not be written or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CacheError {
    /// A file system call failed.
    #[error("{operation} failed: {kind}")]
    Io {
        /// What was being done, e.g. "create the temporary file".
        operation: &'static str,
        /// The kind of the I/O error (no path, no content).
        kind: io::ErrorKind,
    },
    /// Something other than a folder is where the folder should be.
    #[error("the folder is not a directory")]
    NotADirectory,
    /// The folder is open to others and could not be made owner-only.
    #[error("the folder could not be made owner-only")]
    LoosePermissions,
    /// The content could not be encoded as JSON.
    #[error("the content could not be encoded")]
    Encode,
}

impl CacheError {
    fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

/// Result of reading only the first line of a cache file (BR4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderRead {
    /// There is no cache file for the key: simply no cache (review R-13).
    Missing,
    /// The file exists but cannot be opened or read (for example its
    /// permissions): the fetch goes to AWS, the file is kept (R-13).
    Unreadable,
    /// The header is broken (BR4.1).
    Corrupt(Corruption),
    /// A good header for the key.
    Found(CacheHeader),
}

/// Result of looking a requested range up in the cache (BR2.3, BR2.4,
/// BR4.1, review R-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheLookup {
    /// No cache file for the key.
    Missing,
    /// The file could not be opened or read; it was kept.
    Unreadable,
    /// The file was broken; it was removed (when removal failed, a
    /// diagnostic was logged).
    Corrupt(Corruption),
    /// The cache is good but no single cached range holds the request.
    NotCovered,
    /// Every event of the requested range, checked, in order.
    Hit(Vec<CachedEvent>),
}

/// How [`LogCache::write`] stored the new range (BR3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    /// There was no cache for the key.
    Created,
    /// The new range was merged into a good cache.
    Merged,
    /// The old cache was unreadable or broken and was replaced by the new
    /// range alone.
    Rebuilt,
}

/// Counts the cache writes under way (U6 review R-02). Clones share the
/// count; the app keeps one in AppSession and hands it to every
/// [`LogCache`] it makes, so a write that outlives its fetch (the fetch task
/// ended abnormally) still keeps the settings dialog closed until it ends.
#[derive(Debug, Clone, Default)]
pub struct WriteTracker {
    in_flight: Arc<AtomicUsize>,
}

/// Marks one write under way until dropped.
#[derive(Debug)]
pub struct WriteGuard {
    in_flight: Arc<AtomicUsize>,
}

impl WriteTracker {
    /// A tracker with no write under way.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a write is under way.
    pub fn is_writing(&self) -> bool {
        self.in_flight.load(Ordering::SeqCst) > 0
    }

    /// Marks a write under way until the guard is dropped.
    pub fn start(&self) -> WriteGuard {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        WriteGuard {
            in_flight: Arc::clone(&self.in_flight),
        }
    }
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Two trackers are equal when they share the same count.
impl PartialEq for WriteTracker {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.in_flight, &other.in_flight)
    }
}

impl Eq for WriteTracker {}

/// The cache folder (entities.md CacheSettings.cacheDirectory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogCache {
    directory: PathBuf,
    writes: WriteTracker,
}

/// A whole cache file read back.
enum EntryRead {
    Missing,
    Unreadable,
    Corrupt(Corruption),
    Found(CacheHeader, Vec<CachedEvent>),
}

/// Which events [`read_entry`] keeps.
#[derive(Clone, Copy)]
enum EventSelection {
    /// Every event; the total is compared with the header.
    All,
    /// The events of `[start_ms, end_ms]`, which lies in the covered range
    /// `covering`; reading stops after `covering`, whose count is compared
    /// with the header (U6 review R-03).
    Range {
        start_ms: i64,
        end_ms: i64,
        covering: CoveredRange,
    },
}

impl LogCache {
    /// A cache in `directory` (the app's own folder in the OS cache folder,
    /// passed in at startup, BR1.6). Nothing is touched until a call.
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            writes: WriteTracker::new(),
        }
    }

    /// The same cache, counting its writes in `writes` (U6 review R-02).
    pub fn with_writes(mut self, writes: WriteTracker) -> Self {
        self.writes = writes;
        self
    }

    /// The tracker of this cache's writes.
    pub fn writes(&self) -> &WriteTracker {
        &self.writes
    }

    /// The cache folder, shown in the settings dialog (FR7.3).
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Path of the cache file of `key` (BR2.2).
    pub fn entry_path(&self, key: &CacheKey) -> PathBuf {
        self.directory.join(key.file_name())
    }

    /// Reads and checks only the first line of the cache file of `key`
    /// (BR4.2, BR4.1, BR2.2). Removes nothing.
    pub fn read_header(&self, key: &CacheKey) -> HeaderRead {
        let mut reader = match open_entry(&self.entry_path(key)) {
            Ok(reader) => reader,
            Err(OpenError::Missing) => return HeaderRead::Missing,
            Err(OpenError::Unreadable) => return HeaderRead::Unreadable,
        };
        match read_header_line(&mut reader, key) {
            Ok(header) => HeaderRead::Found(header),
            Err(LineError::Corrupt(corruption)) => HeaderRead::Corrupt(corruption),
            Err(LineError::Unreadable) => HeaderRead::Unreadable,
        }
    }

    /// Looks `[start_ms, end_ms]` up (BR2.3, BR2.4). Reads the header first
    /// and the events only when one cached range holds the request; then
    /// reads the events up to `end_ms`, checking each one read (BR4.1,
    /// review R-13). A broken file is removed (BR4.1); a missing file is no
    /// cache; an unreadable one is kept.
    pub fn lookup(&self, key: &CacheKey, start_ms: i64, end_ms: i64) -> CacheLookup {
        let path = self.entry_path(key);
        let read = read_entry(&path, key, |header| {
            find_covering_range(&header.covered_ranges, start_ms, end_ms).map(|covering| {
                EventSelection::Range {
                    start_ms,
                    end_ms,
                    covering,
                }
            })
        });
        match read {
            ReadOutcome::Entry(EntryRead::Missing) => CacheLookup::Missing,
            ReadOutcome::Entry(EntryRead::Unreadable) => CacheLookup::Unreadable,
            ReadOutcome::Entry(EntryRead::Corrupt(corruption)) => {
                self.remove_broken(key, corruption);
                CacheLookup::Corrupt(corruption)
            }
            ReadOutcome::Entry(EntryRead::Found(_, events)) => CacheLookup::Hit(events),
            ReadOutcome::Skipped => CacheLookup::NotCovered,
        }
    }

    /// Writes the events of `range` for `key` (BR3.3, BR3.4, BR3.6).
    ///
    /// The existing cache is read and checked; when good, its events in
    /// `range` are replaced by `events` and the ranges joined, otherwise it
    /// is replaced by `range` and `events` alone. Everything goes to a
    /// temporary file that is renamed over the cache file only once fully
    /// written; leftover temporary files of the key are removed first.
    /// `events` outside `range` are ignored.
    ///
    /// # Errors
    /// [`CacheError`] when the folder cannot be prepared (BR3.6) or the file
    /// cannot be written; the previous cache is then unchanged.
    pub fn write(
        &self,
        key: &CacheKey,
        range: CoveredRange,
        events: Vec<CachedEvent>,
    ) -> Result<WriteKind, CacheError> {
        let _writing = self.writes.start();
        ensure_private_dir(&self.directory)?;
        self.remove_leftover_temp_files(key);
        let path = self.entry_path(key);
        let existing = match read_entry(&path, key, |_| Some(EventSelection::All)) {
            ReadOutcome::Entry(entry) => entry,
            ReadOutcome::Skipped => EntryRead::Missing,
        };
        let (kind, ranges, merged) = match existing {
            EntryRead::Found(header, held) => (
                WriteKind::Merged,
                merge_ranges(&header.covered_ranges, range),
                replace_events_in_range(held, range, events),
            ),
            EntryRead::Missing => (
                WriteKind::Created,
                vec![range],
                replace_events_in_range(Vec::new(), range, events),
            ),
            EntryRead::Unreadable | EntryRead::Corrupt(_) => {
                eprintln!(
                    "local-sights: cache: the existing cache could not be used and was rebuilt from this fetch only"
                );
                (
                    WriteKind::Rebuilt,
                    vec![range],
                    replace_events_in_range(Vec::new(), range, events),
                )
            }
        };
        let range_event_counts = count_events_by_range(&merged, &ranges);
        let header = CacheHeader {
            format_version: FORMAT_VERSION,
            key: key.clone(),
            event_count: merged.len() as u64,
            range_event_counts,
            covered_ranges: ranges,
        };
        let temp_name = key.temp_file_name(&random_suffix());
        write_atomically(&self.directory, &key.file_name(), &temp_name, |out| {
            write_json_line(out, &header)?;
            merged
                .iter()
                .try_for_each(|event| write_json_line(out, event))
        })?;
        Ok(kind)
    }

    /// Removes the cache file of `key`; a missing file is not an error.
    ///
    /// # Errors
    /// [`CacheError::Io`] when the file exists but cannot be removed.
    pub fn remove_entry(&self, key: &CacheKey) -> Result<(), CacheError> {
        match fs::remove_file(self.entry_path(key)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(CacheError::io("remove the cache file", &error)),
        }
    }

    /// Removes every cache file and temporary file of this app (BR1.3,
    /// BR1.4): only regular files directly in the folder whose names have
    /// the cache or temporary shape. Symbolic links are neither followed nor
    /// removed, sub-folders are not entered, other names are kept. A missing
    /// folder has nothing to remove. Returns how many files were removed.
    ///
    /// # Errors
    /// [`CacheError::Io`] when the folder cannot be listed or a matching
    /// file cannot be removed (the others are still removed).
    pub fn clear_all(&self) -> Result<usize, CacheError> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(CacheError::io("list the cache folder", &error)),
        };
        let mut removed = 0;
        let mut first_error = None;
        for entry in entries {
            let result = entry
                .map_err(|error| CacheError::io("list the cache folder", &error))
                .and_then(|entry| remove_if_cache_file(&entry));
            match result {
                Ok(true) => removed += 1,
                Ok(false) => {}
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(removed),
        }
    }

    fn remove_broken(&self, key: &CacheKey, corruption: Corruption) {
        eprintln!("local-sights: cache: a broken cache file was found ({corruption})");
        if let Err(error) = self.remove_entry(key) {
            eprintln!("local-sights: cache: the broken cache file could not be removed: {error}");
        }
    }

    /// BR3.4: temporary files left by an interrupted write of `key`.
    fn remove_leftover_temp_files(&self, key: &CacheKey) {
        let prefix = key.temp_file_name("");
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return;
        };
        for entry in entries.flatten() {
            let is_leftover = entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix));
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            if is_leftover && is_file && fs::remove_file(entry.path()).is_err() {
                eprintln!("local-sights: cache: a leftover temporary file could not be removed");
            }
        }
    }
}

/// Removes `entry` when it is a regular file with a cache or temporary file
/// name (BR1.4); returns whether it was removed.
fn remove_if_cache_file(entry: &fs::DirEntry) -> Result<bool, CacheError> {
    let name = entry.file_name();
    let matches = name
        .to_str()
        .is_some_and(|name| classify_file_name(name) != FileNameKind::Other);
    if !matches {
        return Ok(false);
    }
    // `DirEntry::file_type` does not follow symbolic links.
    let file_type = entry
        .file_type()
        .map_err(|error| CacheError::io("inspect a cache file", &error))?;
    if !file_type.is_file() {
        return Ok(false);
    }
    fs::remove_file(entry.path()).map_err(|error| CacheError::io("remove a cache file", &error))?;
    Ok(true)
}

/// Why a line could not be used.
enum LineError {
    Unreadable,
    Corrupt(Corruption),
}

/// What [`read_entry`] did.
enum ReadOutcome {
    Entry(EntryRead),
    /// The header was good but the caller did not want the events.
    Skipped,
}

/// Why a cache file could not be opened.
enum OpenError {
    /// There is no file: simply no cache.
    Missing,
    /// The file exists but cannot be opened.
    Unreadable,
}

fn open_entry(path: &Path) -> Result<BufReader<File>, OpenError> {
    match File::open(path) {
        Ok(file) => Ok(BufReader::new(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(OpenError::Missing),
        Err(_) => Err(OpenError::Unreadable),
    }
}

/// Reads the header, asks `select` which events to read (`None`: none),
/// then reads them, checking every event read (BR4.1, review R-13) and
/// their count (U6 review R-03).
fn read_entry(
    path: &Path,
    key: &CacheKey,
    select: impl FnOnce(&CacheHeader) -> Option<EventSelection>,
) -> ReadOutcome {
    let mut reader = match open_entry(path) {
        Ok(reader) => reader,
        Err(OpenError::Missing) => return ReadOutcome::Entry(EntryRead::Missing),
        Err(OpenError::Unreadable) => return ReadOutcome::Entry(EntryRead::Unreadable),
    };
    let header = match read_header_line(&mut reader, key) {
        Ok(header) => header,
        Err(error) => return ReadOutcome::Entry(entry_error(error)),
    };
    let Some(selection) = select(&header) else {
        return ReadOutcome::Skipped;
    };
    match read_events(&mut reader, &header, selection) {
        Ok(events) => ReadOutcome::Entry(EntryRead::Found(header, events)),
        Err(error) => ReadOutcome::Entry(entry_error(error)),
    }
}

fn entry_error(error: LineError) -> EntryRead {
    match error {
        LineError::Unreadable => EntryRead::Unreadable,
        LineError::Corrupt(corruption) => EntryRead::Corrupt(corruption),
    }
}

fn read_header_line(
    reader: &mut BufReader<File>,
    key: &CacheKey,
) -> Result<CacheHeader, LineError> {
    let mut line = String::new();
    if !read_line(reader, &mut line)? {
        return Err(LineError::Corrupt(Corruption::Truncated));
    }
    let header: CacheHeader =
        serde_json::from_str(&line).map_err(|_| LineError::Corrupt(Corruption::Unparsable))?;
    validate_header(&header, key).map_err(LineError::Corrupt)?;
    Ok(header)
}

fn read_events(
    reader: &mut BufReader<File>,
    header: &CacheHeader,
    selection: EventSelection,
) -> Result<Vec<CachedEvent>, LineError> {
    let mut check = EventCheck::new(&header.covered_ranges);
    let mut counter = EventCounter::new(header);
    let mut events = Vec::new();
    let mut line = String::new();
    while read_line(reader, &mut line)? {
        let event: CachedEvent =
            serde_json::from_str(&line).map_err(|_| LineError::Corrupt(Corruption::Unparsable))?;
        check.check(&event).map_err(LineError::Corrupt)?;
        match selection {
            EventSelection::All => {
                counter.count(&event);
                events.push(event);
            }
            EventSelection::Range {
                start_ms,
                end_ms,
                covering,
            } => {
                if event.timestamp > covering.end_ms {
                    break;
                }
                counter.count(&event);
                if (start_ms..=end_ms).contains(&event.timestamp) {
                    events.push(event);
                }
            }
        }
    }
    let counted = match selection {
        EventSelection::All => counter.verify_all(),
        EventSelection::Range { covering, .. } => counter.verify_range(covering),
    };
    counted.map_err(LineError::Corrupt)?;
    Ok(events)
}

/// Reads one line into `line` without its line break. `Ok(false)` at the
/// end of the file; a last line without a line break is a cut file.
fn read_line(reader: &mut BufReader<File>, line: &mut String) -> Result<bool, LineError> {
    line.clear();
    match reader.read_line(line) {
        Ok(0) => Ok(false),
        Ok(_) if line.ends_with('\n') => {
            line.pop();
            Ok(true)
        }
        Ok(_) => Err(LineError::Corrupt(Corruption::Truncated)),
        // Not UTF-8: the content is broken, not the disk.
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            Err(LineError::Corrupt(Corruption::Unparsable))
        }
        Err(_) => Err(LineError::Unreadable),
    }
}

fn write_json_line<T: serde::Serialize>(out: &mut BufWriter<File>, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value).map_err(io::Error::from)?;
    out.write_all(b"\n")
}

/// A random suffix for a temporary file name.
fn random_suffix() -> String {
    format!("{:016x}", fastrand::u64(..))
}

/// Creates `dir` (and its parents) owner-only, or tightens an existing
/// folder that others can open (BR3.6).
///
/// # Errors
/// [`CacheError::NotADirectory`] when something else is there,
/// [`CacheError::LoosePermissions`] when the folder cannot be tightened,
/// [`CacheError::Io`] when it cannot be created.
pub(crate) fn ensure_private_dir(dir: &Path) -> Result<(), CacheError> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(PRIVATE_DIR_MODE);
    }
    if let Err(error) = builder.create(dir) {
        return Err(if dir.exists() {
            CacheError::NotADirectory
        } else {
            CacheError::io("create the folder", &error)
        });
    }
    let metadata =
        fs::metadata(dir).map_err(|error| CacheError::io("inspect the folder", &error))?;
    if !metadata.is_dir() {
        return Err(CacheError::NotADirectory);
    }
    restrict_dir(dir, &metadata)
}

#[cfg(unix)]
fn restrict_dir(dir: &Path, metadata: &fs::Metadata) -> Result<(), CacheError> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o777 == PRIVATE_DIR_MODE {
        return Ok(());
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(PRIVATE_DIR_MODE))
        .map_err(|_| CacheError::LoosePermissions)
}

#[cfg(not(unix))]
fn restrict_dir(_dir: &Path, _metadata: &fs::Metadata) -> Result<(), CacheError> {
    Ok(())
}

/// Writes `dir/temp_name` owner-only with `write`, flushes it to disk and
/// renames it to `dir/final_name` (BR3.4, BR3.6). On any failure the
/// temporary file is removed and `final_name` is left as it was.
pub(crate) fn write_atomically(
    dir: &Path,
    final_name: &str,
    temp_name: &str,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> Result<(), CacheError> {
    let temp_path = dir.join(temp_name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(PRIVATE_FILE_MODE);
    }
    let file = options
        .open(&temp_path)
        .map_err(|error| CacheError::io("create the temporary file", &error))?;
    let result = fill_and_rename(file, write, &temp_path, &dir.join(final_name));
    if result.is_err() && fs::remove_file(&temp_path).is_err() {
        // A leftover is removed by the next write or a clear (BR3.4).
        eprintln!(
            "local-sights: cache: a temporary file could not be removed after a failed write"
        );
    }
    result
}

fn fill_and_rename(
    file: File,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
    temp_path: &Path,
    final_path: &Path,
) -> Result<(), CacheError> {
    let mut out = BufWriter::new(file);
    write(&mut out).map_err(|error| match error.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput => CacheError::Encode,
        _ => CacheError::io("write the temporary file", &error),
    })?;
    let file = out
        .into_inner()
        .map_err(|error| CacheError::io("write the temporary file", error.error()))?;
    file.sync_all()
        .map_err(|error| CacheError::io("flush the temporary file", &error))?;
    drop(file);
    fs::rename(temp_path, final_path).map_err(|error| CacheError::io("replace the file", &error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ProfileSelector;

    fn key() -> CacheKey {
        CacheKey::new(
            ProfileSelector::Named("dev".to_string()),
            "ap-northeast-1".to_string(),
            "/aws/lambda/orders".to_string(),
        )
    }

    fn other_key() -> CacheKey {
        CacheKey::new(
            ProfileSelector::SdkDefault,
            "ap-northeast-1".to_string(),
            "/aws/lambda/orders".to_string(),
        )
    }

    fn range(start_ms: i64, end_ms: i64) -> CoveredRange {
        CoveredRange { start_ms, end_ms }
    }

    fn cached(timestamp: i64, stream: &str, sequence: u64) -> CachedEvent {
        CachedEvent {
            timestamp,
            ingestion_time: None,
            message: format!("{stream} at {timestamp}\nsecond line"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn timestamps(lookup: &CacheLookup) -> Vec<i64> {
        match lookup {
            CacheLookup::Hit(events) => events.iter().map(|e| e.timestamp).collect(),
            other => panic!("expected a hit, got {other:?}"),
        }
    }

    /// A cache in a fresh folder, with `[100, 199]` holding four events.
    fn written() -> (tempfile::TempDir, LogCache) {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().join("cache"));
        let events = vec![
            cached(100, "a", 0),
            cached(120, "b", 0),
            cached(150, "a", 1),
            cached(199, "a", 2),
        ];
        assert_eq!(
            cache.write(&key(), range(100, 199), events),
            Ok(WriteKind::Created)
        );
        (dir, cache)
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn written_events_are_read_back_only_for_the_requested_range() {
        let (_dir, cache) = written();
        let whole = cache.lookup(&key(), 100, 199);
        assert_eq!(timestamps(&whole), vec![100, 120, 150, 199]);
        if let CacheLookup::Hit(events) = &whole {
            assert_eq!(
                events[0].message, "a at 100\nsecond line",
                "kept as fetched"
            );
        }
        assert_eq!(timestamps(&cache.lookup(&key(), 110, 150)), vec![120, 150]);
        assert_eq!(
            timestamps(&cache.lookup(&key(), 160, 190)),
            Vec::<i64>::new()
        );
        assert_eq!(cache.lookup(&key(), 50, 150), CacheLookup::NotCovered);
        assert_eq!(cache.lookup(&other_key(), 100, 199), CacheLookup::Missing);
        match cache.read_header(&key()) {
            HeaderRead::Found(header) => {
                assert_eq!(header.covered_ranges, vec![range(100, 199)]);
                assert_eq!(header.key, key());
            }
            other => panic!("expected a header, got {other:?}"),
        }
        let text = fs::read_to_string(cache.entry_path(&key())).unwrap();
        assert_eq!(text.lines().count(), 5, "one header and four events");
        assert!(text.lines().next().unwrap().contains("\"formatVersion\":2"));
    }

    #[test]
    fn a_failed_write_leaves_the_previous_file_and_no_temporary_file() {
        let (_dir, cache) = written();
        let before = fs::read(cache.entry_path(&key())).unwrap();
        let result = write_atomically(
            cache.directory(),
            &key().file_name(),
            &key().temp_file_name("t1"),
            |out| {
                out.write_all(b"half of a line")?;
                Err(io::Error::other("disk full"))
            },
        );
        assert_eq!(
            result,
            Err(CacheError::Io {
                operation: "write the temporary file",
                kind: io::ErrorKind::Other
            })
        );
        assert_eq!(fs::read(cache.entry_path(&key())).unwrap(), before);
        assert_eq!(file_names(cache.directory()), vec![key().file_name()]);
        // A leftover temporary file of the key goes with the next write.
        fs::write(cache.directory().join(key().temp_file_name("old")), b"x").unwrap();
        cache.write(&key(), range(300, 399), Vec::new()).unwrap();
        assert_eq!(file_names(cache.directory()), vec![key().file_name()]);
    }

    #[cfg(unix)]
    #[test]
    fn files_are_owner_only_and_a_loose_folder_is_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, cache) = written();
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(cache.directory()), PRIVATE_DIR_MODE);
        assert_eq!(mode(&cache.entry_path(&key())), PRIVATE_FILE_MODE);

        fs::set_permissions(cache.directory(), fs::Permissions::from_mode(0o755)).unwrap();
        cache.write(&key(), range(300, 399), Vec::new()).unwrap();
        assert_eq!(mode(cache.directory()), PRIVATE_DIR_MODE);
        assert_eq!(mode(&cache.entry_path(&key())), PRIVATE_FILE_MODE);
    }

    #[cfg(unix)]
    #[test]
    fn clearing_removes_only_regular_files_with_cache_names() {
        let (dir, cache) = written();
        let folder = cache.directory();
        fs::write(folder.join(other_key().temp_file_name("left")), b"x").unwrap();
        fs::write(folder.join("notes.txt"), b"keep").unwrap();
        fs::create_dir(folder.join(other_key().file_name())).unwrap();
        let outside = dir.path().join("outside.cache");
        fs::write(&outside, b"not ours").unwrap();
        let link_name =
            CacheKey::new(ProfileSelector::SdkDefault, "x".into(), "y".into()).file_name();
        std::os::unix::fs::symlink(&outside, folder.join(&link_name)).unwrap();

        assert_eq!(
            cache.clear_all(),
            Ok(2),
            "the cache file and the temporary file"
        );
        let mut kept = vec![link_name, other_key().file_name(), "notes.txt".to_string()];
        kept.sort();
        assert_eq!(file_names(folder), kept);
        assert_eq!(
            fs::read(&outside).unwrap(),
            b"not ours",
            "the link target stays"
        );
        assert_eq!(cache.lookup(&key(), 100, 199), CacheLookup::Missing);

        let missing = LogCache::new(dir.path().join("never-created"));
        assert_eq!(missing.clear_all(), Ok(0));
    }

    /// Turns the text of a good cache file into a broken one.
    type Breakage = Box<dyn Fn(&str) -> String>;

    #[test]
    fn broken_files_are_reported_and_removed() {
        let breakages: Vec<(&str, Breakage, Corruption)> = vec![
            (
                "cut in a line",
                Box::new(|text| text[..text.len() - 8].to_string()),
                Corruption::Truncated,
            ),
            ("empty", Box::new(|_| String::new()), Corruption::Truncated),
            (
                "not JSON",
                Box::new(|text| text.replacen("{", "<", 1)),
                Corruption::Unparsable,
            ),
            (
                "unknown version",
                Box::new(|text| text.replacen("\"formatVersion\":2", "\"formatVersion\":9", 1)),
                Corruption::UnknownVersion,
            ),
            (
                "another key",
                Box::new(|text| text.replacen("/aws/lambda/orders", "/aws/lambda/other", 1)),
                Corruption::KeyMismatch,
            ),
            (
                "overlapping ranges",
                Box::new(|text| {
                    text.replacen(
                        "[{\"startMs\":100,\"endMs\":199}]",
                        "[{\"startMs\":100,\"endMs\":199},{\"startMs\":150,\"endMs\":299}]",
                        1,
                    )
                }),
                Corruption::BadRanges,
            ),
            (
                "events out of order",
                Box::new(|text| {
                    let mut lines: Vec<&str> = text.lines().collect();
                    lines.swap(1, 2);
                    lines.join("\n") + "\n"
                }),
                Corruption::EventsOutOfOrder,
            ),
            (
                "an event outside the ranges",
                Box::new(|text| text.replacen("\"timestamp\":120", "\"timestamp\":99", 1)),
                Corruption::EventOutsideRanges,
            ),
        ];
        for (what, breakage, expected) in breakages {
            let (_dir, cache) = written();
            let path = cache.entry_path(&key());
            let text = fs::read_to_string(&path).unwrap();
            fs::write(&path, breakage(&text)).unwrap();
            assert_eq!(
                cache.lookup(&key(), 100, 199),
                CacheLookup::Corrupt(expected),
                "{what}"
            );
            assert!(!path.exists(), "{what}: removed");
        }
    }

    #[test]
    fn a_file_cut_at_a_line_boundary_is_broken_and_removed() {
        for (from, to) in [(100, 199), (110, 150)] {
            let (_dir, cache) = written();
            let path = cache.entry_path(&key());
            let text = fs::read_to_string(&path).unwrap();
            // Drop the last event line whole: every line left is valid JSON.
            let lines: Vec<&str> = text.lines().collect();
            fs::write(&path, lines[..lines.len() - 1].join("\n") + "\n").unwrap();
            assert_eq!(
                cache.lookup(&key(), from, to),
                CacheLookup::Corrupt(Corruption::CountMismatch),
                "[{from}, {to}]"
            );
            assert!(!path.exists(), "removed");
        }
        let (_dir, cache) = written();
        let path = cache.entry_path(&key());
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        fs::write(&path, lines[..lines.len() - 1].join("\n") + "\n").unwrap();
        assert_eq!(
            cache.write(&key(), range(300, 399), Vec::new()),
            Ok(WriteKind::Rebuilt),
            "a whole read of a cut file sees it too"
        );
    }

    #[test]
    fn the_header_records_the_counts_and_writes_are_tracked() {
        let (_dir, cache) = written();
        cache
            .write(&key(), range(300, 399), vec![cached(310, "c", 0)])
            .unwrap();
        match cache.read_header(&key()) {
            HeaderRead::Found(header) => {
                assert_eq!(header.event_count, 5);
                assert_eq!(header.range_event_counts, vec![4, 1]);
            }
            other => panic!("expected a header, got {other:?}"),
        }
        let tracker = WriteTracker::new();
        let tracked = cache.clone().with_writes(tracker.clone());
        assert_eq!(tracked.writes(), &tracker);
        assert!(!tracker.is_writing());
        let guard = tracker.start();
        assert!(tracked.writes().is_writing());
        drop(guard);
        tracked.write(&key(), range(500, 599), Vec::new()).unwrap();
        assert!(
            !tracker.is_writing(),
            "the write's own mark is gone after it"
        );
        assert_ne!(WriteTracker::new(), tracker);
    }

    #[test]
    fn a_missing_file_is_no_cache_and_an_unreadable_one_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LogCache::new(dir.path().to_path_buf());
        assert_eq!(cache.lookup(&key(), 0, 1), CacheLookup::Missing);
        assert_eq!(cache.read_header(&key()), HeaderRead::Missing);
        // A folder where the file should be: it opens but cannot be read.
        fs::create_dir(cache.entry_path(&key())).unwrap();
        assert_eq!(cache.lookup(&key(), 0, 1), CacheLookup::Unreadable);
        assert_eq!(cache.read_header(&key()), HeaderRead::Unreadable);
        assert!(cache.entry_path(&key()).is_dir(), "kept");
    }

    #[test]
    fn a_good_cache_is_merged_and_a_broken_one_rebuilt_from_the_new_range() {
        let (_dir, cache) = written();
        let kind = cache.write(
            &key(),
            range(150, 249),
            vec![cached(160, "c", 0), cached(240, "c", 1)],
        );
        assert_eq!(kind, Ok(WriteKind::Merged));
        assert_eq!(
            timestamps(&cache.lookup(&key(), 100, 249)),
            vec![100, 120, 160, 240],
            "150 and 199 were replaced by the new fetch"
        );
        cache.write(&key(), range(250, 299), Vec::new()).unwrap();
        match cache.read_header(&key()) {
            HeaderRead::Found(header) => assert_eq!(header.covered_ranges, vec![range(100, 299)]),
            other => panic!("expected a header, got {other:?}"),
        }

        let path = cache.entry_path(&key());
        let text = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            text.replacen("\"formatVersion\":2", "\"formatVersion\":0", 1),
        )
        .unwrap();
        let kind = cache.write(&key(), range(500, 599), vec![cached(510, "d", 0)]);
        assert_eq!(kind, Ok(WriteKind::Rebuilt));
        match cache.read_header(&key()) {
            HeaderRead::Found(header) => assert_eq!(header.covered_ranges, vec![range(500, 599)]),
            other => panic!("expected a header, got {other:?}"),
        }
        assert_eq!(timestamps(&cache.lookup(&key(), 500, 599)), vec![510]);
    }

    #[test]
    fn a_file_where_the_folder_should_be_makes_writing_fail_and_reading_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("cache");
        fs::write(&folder, b"a regular file").unwrap();
        let cache = LogCache::new(folder.clone());
        assert_eq!(
            cache.write(&key(), range(0, 9), vec![cached(1, "a", 0)]),
            Err(CacheError::NotADirectory)
        );
        assert_eq!(cache.lookup(&key(), 0, 9), CacheLookup::Unreadable);
        assert!(cache.clear_all().is_err());
        assert_eq!(fs::read(&folder).unwrap(), b"a regular file");
        assert!(cache.remove_entry(&key()).is_err());
    }
}
