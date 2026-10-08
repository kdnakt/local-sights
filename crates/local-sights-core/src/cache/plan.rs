//! LogCache, pure logic (U6): every decision about the disk cache that needs
//! no file system.
//!
//! - which part of a fetch may be recorded: up to five minutes before the
//!   fetch started (BR3.1), and only after a fetch whose streams all
//!   succeeded (BR3.2);
//! - whether a requested range is served from one cached range (BR2.3,
//!   BR2.4), and how a new range joins the cached ones (BR3.3);
//! - how the events of the recorded range replace the cached ones (BR3.3)
//!   and how the events read back are numbered again (BR4.3);
//! - the key, its SHA-256 file name and the names of temporary files
//!   (BR2.1, BR2.2, BR1.4);
//! - the checks that tell a broken cache from a good one (BR4.1, review
//!   R-13), and since U7 the event counts of the header that tell a file cut
//!   at a line boundary from a whole one (U6 review R-03);
//! - the status line notices made from a finished fetch (BR5.3).
//!
//! The types written to disk are only [`CacheHeader`] (format version, key,
//! covered ranges and event counts) and [`CachedEvent`]; none of them can hold a
//! credential (BR3.6, project.md Forbidden).

use std::cmp::Ordering;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::catalog::ProfileSelector;
use crate::coordinator::JobStatus;
use crate::event::LogEvent;
use crate::request::FetchRequest;

/// Version of the cache file format (BR4.1). Version 2 (U7, U6 review R-03)
/// adds the event counts to the header; a version 1 file is an unknown
/// version, so it is removed and fetched again (BR4.1).
pub const FORMAT_VERSION: u32 = 2;
/// The newest part of a fetch that is never recorded: five minutes before
/// the fetch started (BR3.1).
pub const RECORD_MARGIN_MS: i64 = 300_000;
/// Suffix of a cache file name (BR2.2).
pub const CACHE_FILE_SUFFIX: &str = ".cache";
/// What follows the digest in a temporary file name (BR1.4, BR3.4).
pub const TEMP_FILE_INFIX: &str = ".cache.tmp-";
/// Length of the hexadecimal SHA-256 digest of a key (BR2.2).
pub const DIGEST_HEX_LEN: usize = 64;

/// What a cache is looked up by: the profile (kind and name), the region and
/// the log group (BR2.1). Two keys are the same only when all three match;
/// a Named profile called `default` is not the SDK default.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheKey {
    /// SdkDefault (no name) or Named with its name.
    pub profile: ProfileSelector,
    /// Region code, e.g. `ap-northeast-1`.
    pub region: String,
    /// Log group name.
    pub log_group_name: String,
}

/// One cached time range, both ends included (epoch milliseconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoveredRange {
    /// First millisecond (included).
    pub start_ms: i64,
    /// Last millisecond (included).
    pub end_ms: i64,
}

/// One event as stored in the cache (a copy of a fetched [`LogEvent`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedEvent {
    /// Event time, epoch milliseconds.
    pub timestamp: i64,
    /// Ingestion time, epoch milliseconds, when the API returned it.
    #[serde(default)]
    pub ingestion_time: Option<i64>,
    /// The message exactly as the API returned it.
    pub message: String,
    /// Stream the event came from.
    pub log_stream_name: String,
    /// Order within its stream in the fetch that stored it; numbered again
    /// when read back (BR4.3).
    pub sequence: u64,
}

/// First line of a cache file: what can be checked without reading the
/// events (BR4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheHeader {
    /// Format version; an unknown one is a broken cache (BR4.1).
    pub format_version: u32,
    /// The key the file belongs to (checked against the looked-up key, BR2.2).
    pub key: CacheKey,
    /// Cached ranges: no overlap, no adjacency, earliest first (BR3.3).
    pub covered_ranges: Vec<CoveredRange>,
    /// Number of events in the file (U6 review R-03); absent in version 1.
    #[serde(default)]
    pub event_count: u64,
    /// Number of events in each covered range, in the same order (U6 review
    /// R-03); absent in version 1.
    #[serde(default)]
    pub range_event_counts: Vec<u64>,
}

/// Why a cache file is taken as broken (BR4.1, review R-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Corruption {
    /// A line is not valid JSON of the expected shape.
    #[error("a line cannot be parsed")]
    Unparsable,
    /// The file ends in the middle of a line, or has no header.
    #[error("the file is cut short")]
    Truncated,
    /// The format version is not [`FORMAT_VERSION`].
    #[error("unknown format version")]
    UnknownVersion,
    /// The key in the file is not the looked-up key.
    #[error("the key does not match")]
    KeyMismatch,
    /// The covered ranges overlap, touch, are out of order, empty or reversed.
    #[error("the covered ranges are malformed")]
    BadRanges,
    /// An event lies outside every covered range.
    #[error("an event lies outside the covered ranges")]
    EventOutsideRanges,
    /// The events are not in `(timestamp, logStreamName, sequence)` order.
    #[error("the events are out of order")]
    EventsOutOfOrder,
    /// The events read are not as many as the header says (a file cut at a
    /// line boundary), or the header's counts disagree (U6 review R-03).
    #[error("the event count does not match")]
    CountMismatch,
}

/// Which kind of file a name in the cache folder is (BR1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileNameKind {
    /// `<64 hex digits>.cache`.
    Cache,
    /// `<64 hex digits>.cache.tmp-<anything>`.
    Temp,
    /// Anything else; never touched.
    Other,
}

/// What the cache did for one fetch (entities.md FetchJob.cacheOutcome).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum CacheOutcome {
    /// The cache is disabled (BR1.5).
    #[default]
    NotUsed,
    /// Served from the cache without calling AWS (BR2.3).
    Hit,
    /// Fetched from AWS and written (BR3.5).
    Saved,
    /// Fetched from AWS and not written: a failure, an abort or nothing
    /// recordable (BR3.1, BR3.2).
    NotSaved,
    /// Fetched from AWS but the write failed (BR3.5).
    SaveFailed,
}

/// One cache notice of the status line (BR5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CacheNotice {
    /// `cache.hit`.
    Hit,
    /// `cache.readFailed`.
    ReadFailed,
    /// `cache.saveFailed`.
    SaveFailed,
}

/// What [`decide_write`] looks at (BR3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteCheck {
    /// Whether the cache is enabled.
    pub cache_enabled: bool,
    /// Whether the fetch was served from the cache.
    pub served_from_cache: bool,
    /// Final status of the fetch.
    pub status: JobStatus,
    /// Number of failed streams.
    pub failed_stream_count: usize,
    /// First millisecond of the fetched range.
    pub range_start_ms: i64,
    /// Last millisecond of the fetched range.
    pub range_end_ms: i64,
    /// When the fetch started, epoch milliseconds.
    pub fetch_started_at_ms: i64,
}

impl CacheKey {
    /// A key from its three parts.
    pub fn new(profile: ProfileSelector, region: String, log_group_name: String) -> Self {
        Self {
            profile,
            region,
            log_group_name,
        }
    }

    /// The key of a validated request; `None` when it has no region (only
    /// the `fetch_check` example omits it, and it never uses the cache).
    pub fn from_request(request: &FetchRequest) -> Option<Self> {
        Some(Self::new(
            request.profile().clone(),
            request.region()?.to_string(),
            request.log_group_name().to_string(),
        ))
    }

    /// SHA-256 of the key as 64 lowercase hexadecimal digits (BR2.2).
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"local-sights cache key v1\0");
        match &self.profile {
            ProfileSelector::SdkDefault => hasher.update(b"S"),
            ProfileSelector::Named(name) => {
                hasher.update(b"N");
                update_with_length(&mut hasher, name);
            }
        }
        update_with_length(&mut hasher, &self.region);
        update_with_length(&mut hasher, &self.log_group_name);
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// `<digest>.cache` (BR2.2).
    pub fn file_name(&self) -> String {
        format!("{}{CACHE_FILE_SUFFIX}", self.digest())
    }

    /// `<digest>.cache.tmp-<suffix>` (BR1.4, BR3.4).
    pub fn temp_file_name(&self, suffix: &str) -> String {
        format!("{}{TEMP_FILE_INFIX}{suffix}", self.digest())
    }
}

/// Feeds one part of the key with its length first, so no two different
/// keys feed the same bytes (BR2.2).
fn update_with_length(hasher: &mut Sha256, part: &str) {
    hasher.update((part.len() as u64).to_be_bytes());
    hasher.update(part.as_bytes());
}

impl CoveredRange {
    /// Whether `instant` lies in the range (both ends included).
    pub fn contains_instant(&self, instant: i64) -> bool {
        self.start_ms <= instant && instant <= self.end_ms
    }

    /// Whether `[start_ms, end_ms]` lies wholly in the range.
    pub fn contains_range(&self, start_ms: i64, end_ms: i64) -> bool {
        self.start_ms <= start_ms && end_ms <= self.end_ms
    }
}

impl From<LogEvent> for CachedEvent {
    fn from(event: LogEvent) -> Self {
        Self {
            timestamp: event.timestamp,
            ingestion_time: event.ingestion_time,
            message: event.message,
            log_stream_name: event.log_stream_name,
            sequence: event.sequence,
        }
    }
}

impl CachedEvent {
    /// Ordering key `(timestamp, logStreamName, sequence)` (U3:BR4.1).
    pub fn sort_key(&self) -> (i64, &str, u64) {
        (self.timestamp, self.log_stream_name.as_str(), self.sequence)
    }
}

/// Compares two cached events by `(timestamp, logStreamName, sequence)`.
pub fn compare_cached(a: &CachedEvent, b: &CachedEvent) -> Ordering {
    a.sort_key().cmp(&b.sort_key())
}

/// BR3.1: the range to record from a fetch of `[start_ms, end_ms]` that
/// started at `fetch_started_at_ms`. Its end is the earlier of the fetch end
/// and five minutes before the start; `None` when that falls before
/// `start_ms`.
pub fn record_range(start_ms: i64, end_ms: i64, fetch_started_at_ms: i64) -> Option<CoveredRange> {
    let record_end = end_ms.min(fetch_started_at_ms.saturating_sub(RECORD_MARGIN_MS));
    (record_end >= start_ms).then_some(CoveredRange {
        start_ms,
        end_ms: record_end,
    })
}

/// BR2.3, BR2.4: the one cached range that holds all of `[start_ms,
/// end_ms]`, boundaries included; `None` when no single range does (also
/// when two touching ranges together would).
pub fn find_covering_range(
    ranges: &[CoveredRange],
    start_ms: i64,
    end_ms: i64,
) -> Option<CoveredRange> {
    ranges
        .iter()
        .copied()
        .find(|range| range.contains_range(start_ms, end_ms))
}

/// BR3.3: adds `added` to `ranges` and joins every overlapping or adjacent
/// pair (`a.end_ms + 1 == b.start_ms`) into one; the result is earliest
/// first. Separate ranges stay separate.
pub fn merge_ranges(ranges: &[CoveredRange], added: CoveredRange) -> Vec<CoveredRange> {
    let mut all: Vec<CoveredRange> = ranges.iter().copied().chain([added]).collect();
    all.sort_by_key(|range| (range.start_ms, range.end_ms));
    let mut merged: Vec<CoveredRange> = Vec::with_capacity(all.len());
    for range in all {
        match merged.last_mut() {
            Some(last) if range.start_ms <= last.end_ms.saturating_add(1) => {
                last.end_ms = last.end_ms.max(range.end_ms);
            }
            _ => merged.push(range),
        }
    }
    merged
}

/// BR3.3: keeps the `existing` events outside `range`, drops those inside
/// it and adds the `new_events` inside it, in `(timestamp, logStreamName,
/// sequence)` order. `existing` must already be in that order.
pub fn replace_events_in_range(
    existing: Vec<CachedEvent>,
    range: CoveredRange,
    new_events: Vec<CachedEvent>,
) -> Vec<CachedEvent> {
    let mut added: Vec<CachedEvent> = new_events
        .into_iter()
        .filter(|event| range.contains_instant(event.timestamp))
        .collect();
    if !added.is_sorted_by(|a, b| compare_cached(a, b).is_le()) {
        added.sort_by(compare_cached);
    }
    let mut merged = Vec::with_capacity(existing.len() + added.len());
    let mut kept = existing
        .into_iter()
        .filter(|event| !range.contains_instant(event.timestamp))
        .peekable();
    let mut added = added.into_iter().peekable();
    loop {
        let take_kept = match (kept.peek(), added.peek()) {
            (Some(old), Some(new)) => compare_cached(old, new).is_le(),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        let next = if take_kept { kept.next() } else { added.next() };
        merged.extend(next);
    }
    merged
}

/// BR3.2: the range to write after a fetch, or `None` when nothing may be
/// written: the cache is disabled, the fetch was served from the cache, it
/// did not complete (failure, partial listing, abort), a stream failed, or
/// nothing is recordable (BR3.1).
pub fn decide_write(check: &WriteCheck) -> Option<CoveredRange> {
    let complete = check.status == JobStatus::Completed && check.failed_stream_count == 0;
    if !check.cache_enabled || check.served_from_cache || !complete {
        return None;
    }
    record_range(
        check.range_start_ms,
        check.range_end_ms,
        check.fetch_started_at_ms,
    )
}

/// BR4.3: turns cached events (in `(timestamp, logStreamName, sequence)`
/// order) into timeline events numbered again from 0 per stream without
/// gaps, keeping their order.
pub fn resequence(events: Vec<CachedEvent>) -> Vec<LogEvent> {
    let mut next_sequence: HashMap<String, u64> = HashMap::new();
    events
        .into_iter()
        .map(|event| {
            let counter = match next_sequence.get_mut(&event.log_stream_name) {
                Some(counter) => counter,
                None => next_sequence
                    .entry(event.log_stream_name.clone())
                    .or_insert(0),
            };
            let sequence = *counter;
            *counter += 1;
            LogEvent {
                timestamp: event.timestamp,
                ingestion_time: event.ingestion_time,
                message: event.message,
                log_stream_name: event.log_stream_name,
                sequence,
            }
        })
        .collect()
}

/// BR3.3 shape of the covered ranges: at least one, each with `start_ms <=
/// end_ms`, earliest first, with neither overlap nor adjacency.
pub fn ranges_are_well_formed(ranges: &[CoveredRange]) -> bool {
    !ranges.is_empty()
        && ranges.iter().all(|range| range.start_ms <= range.end_ms)
        && ranges
            .windows(2)
            .all(|pair| pair[0].end_ms.saturating_add(1) < pair[1].start_ms)
}

/// BR4.1, BR2.2: checks the header against the looked-up key.
pub fn validate_header(header: &CacheHeader, expected: &CacheKey) -> Result<(), Corruption> {
    if header.format_version != FORMAT_VERSION {
        return Err(Corruption::UnknownVersion);
    }
    if &header.key != expected {
        return Err(Corruption::KeyMismatch);
    }
    if !ranges_are_well_formed(&header.covered_ranges) {
        return Err(Corruption::BadRanges);
    }
    let counts_agree = header.range_event_counts.len() == header.covered_ranges.len()
        && header
            .range_event_counts
            .iter()
            .try_fold(0u64, |sum, count| sum.checked_add(*count))
            == Some(header.event_count);
    if !counts_agree {
        return Err(Corruption::CountMismatch);
    }
    Ok(())
}

/// U6 review R-03: the number of `events` in each of `ranges` (well formed,
/// earliest first), in the order of the ranges; what the header records.
pub fn count_events_by_range(events: &[CachedEvent], ranges: &[CoveredRange]) -> Vec<u64> {
    let mut counts = vec![0u64; ranges.len()];
    for event in events {
        if let Some(index) = range_index(ranges, event.timestamp) {
            counts[index] += 1;
        }
    }
    counts
}

/// Index of the range of well-formed `ranges` holding `instant`.
fn range_index(ranges: &[CoveredRange], instant: i64) -> Option<usize> {
    let index = ranges.partition_point(|range| range.end_ms < instant);
    ranges
        .get(index)
        .filter(|range| range.contains_instant(instant))
        .map(|_| index)
}

/// Counts the events read from a cache file and compares them with the
/// header (U6 review R-03): a file cut exactly at a line boundary passes
/// every line check, but has fewer events than its header says.
#[derive(Debug, Clone)]
pub struct EventCounter {
    ranges: Vec<CoveredRange>,
    expected_total: u64,
    expected_by_range: Vec<u64>,
    total: u64,
    by_range: Vec<u64>,
}

impl EventCounter {
    /// A counter for the events of a file with a validated `header`.
    pub fn new(header: &CacheHeader) -> Self {
        Self {
            ranges: header.covered_ranges.clone(),
            expected_total: header.event_count,
            expected_by_range: header.range_event_counts.clone(),
            total: 0,
            by_range: vec![0; header.covered_ranges.len()],
        }
    }

    /// Counts one event read (after [`EventCheck::check`] accepted it).
    pub fn count(&mut self, event: &CachedEvent) {
        self.total += 1;
        if let Some(index) = range_index(&self.ranges, event.timestamp) {
            if let Some(count) = self.by_range.get_mut(index) {
                *count += 1;
            }
        }
    }

    /// After reading the whole file: as many events as the header says.
    pub fn verify_all(&self) -> Result<(), Corruption> {
        if self.total == self.expected_total && self.by_range == self.expected_by_range {
            Ok(())
        } else {
            Err(Corruption::CountMismatch)
        }
    }

    /// After reading every event of `range` (a covered range of the header):
    /// as many events in it as the header says.
    pub fn verify_range(&self, range: CoveredRange) -> Result<(), Corruption> {
        let index = self.ranges.iter().position(|covered| *covered == range);
        let counted = index.and_then(|index| self.by_range.get(index));
        let expected = index.and_then(|index| self.expected_by_range.get(index));
        match (counted, expected) {
            (Some(counted), Some(expected)) if counted == expected => Ok(()),
            _ => Err(Corruption::CountMismatch),
        }
    }
}

/// Checks events one by one as they are read (BR4.1, review R-13): each
/// must lie in a covered range and come after the previous one.
#[derive(Debug, Clone)]
pub struct EventCheck {
    ranges: Vec<CoveredRange>,
    previous: Option<(i64, String, u64)>,
}

impl EventCheck {
    /// A check against well-formed `ranges`.
    pub fn new(ranges: &[CoveredRange]) -> Self {
        Self {
            ranges: ranges.to_vec(),
            previous: None,
        }
    }

    /// Checks the next event read.
    pub fn check(&mut self, event: &CachedEvent) -> Result<(), Corruption> {
        let index = self
            .ranges
            .partition_point(|range| range.end_ms < event.timestamp);
        let inside = self
            .ranges
            .get(index)
            .is_some_and(|range| range.contains_instant(event.timestamp));
        if !inside {
            return Err(Corruption::EventOutsideRanges);
        }
        if let Some((timestamp, stream, sequence)) = &self.previous {
            let previous = (*timestamp, stream.as_str(), *sequence);
            if previous > event.sort_key() {
                return Err(Corruption::EventsOutOfOrder);
            }
        }
        match &mut self.previous {
            // Reuse the stream name buffer: most events repeat a stream.
            Some((timestamp, stream, sequence)) => {
                *timestamp = event.timestamp;
                if *stream != event.log_stream_name {
                    stream.clone_from(&event.log_stream_name);
                }
                *sequence = event.sequence;
            }
            None => {
                self.previous = Some((
                    event.timestamp,
                    event.log_stream_name.clone(),
                    event.sequence,
                ));
            }
        }
        Ok(())
    }
}

/// Checks a whole list of events with [`EventCheck`].
pub fn validate_events(events: &[CachedEvent], ranges: &[CoveredRange]) -> Result<(), Corruption> {
    let mut check = EventCheck::new(ranges);
    events.iter().try_for_each(|event| check.check(event))
}

/// BR1.4: which kind of file `name` is.
pub fn classify_file_name(name: &str) -> FileNameKind {
    let Some(digest) = name.get(..DIGEST_HEX_LEN) else {
        return FileNameKind::Other;
    };
    let is_digest = digest
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_digest {
        return FileNameKind::Other;
    }
    let rest = &name[DIGEST_HEX_LEN..];
    if rest == CACHE_FILE_SUFFIX {
        FileNameKind::Cache
    } else if rest.starts_with(TEMP_FILE_INFIX) {
        FileNameKind::Temp
    } else {
        FileNameKind::Other
    }
}

/// BR5.3: the notices of a finished fetch, in the order Hit, ReadFailed,
/// SaveFailed. ReadFailed and SaveFailed may both appear; Saved, NotSaved
/// and NotUsed show nothing.
pub fn cache_notices(
    served_from_cache: bool,
    read_failed: bool,
    outcome: CacheOutcome,
) -> Vec<CacheNotice> {
    let mut notices = Vec::new();
    if served_from_cache || outcome == CacheOutcome::Hit {
        notices.push(CacheNotice::Hit);
    }
    if read_failed {
        notices.push(CacheNotice::ReadFailed);
    }
    if outcome == CacheOutcome::SaveFailed {
        notices.push(CacheNotice::SaveFailed);
    }
    notices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start_ms: i64, end_ms: i64) -> CoveredRange {
        CoveredRange { start_ms, end_ms }
    }

    fn cached(timestamp: i64, stream: &str, sequence: u64) -> CachedEvent {
        CachedEvent {
            timestamp,
            ingestion_time: Some(timestamp + 1),
            message: format!("{stream}:{timestamp}:{sequence}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn key(profile: ProfileSelector) -> CacheKey {
        CacheKey::new(
            profile,
            "ap-northeast-1".to_string(),
            "/aws/lambda/orders".to_string(),
        )
    }

    fn check(status: JobStatus, failed: usize) -> WriteCheck {
        WriteCheck {
            cache_enabled: true,
            served_from_cache: false,
            status,
            failed_stream_count: failed,
            range_start_ms: 1_000_000,
            range_end_ms: 2_000_999,
            fetch_started_at_ms: 10_000_000,
        }
    }

    #[test]
    fn the_recorded_range_ends_five_minutes_before_the_fetch_started() {
        let started = 10_000_000;
        // The fetch ended long before: the whole range is recorded.
        assert_eq!(
            record_range(1_000, 2_999, started),
            Some(range(1_000, 2_999))
        );
        // The fetch ended after "five minutes before": cut there.
        assert_eq!(
            record_range(1_000, started, started),
            Some(range(1_000, started - RECORD_MARGIN_MS))
        );
        // Exactly at the boundary: one millisecond is still recorded.
        let edge = started - RECORD_MARGIN_MS;
        assert_eq!(
            record_range(edge, edge + 999, started),
            Some(range(edge, edge))
        );
        // The start is newer than "five minutes before": nothing.
        assert_eq!(record_range(edge + 1, edge + 999, started), None);
    }

    #[test]
    fn a_range_is_served_only_from_one_range_that_holds_it_with_boundaries() {
        let ranges = [range(100, 199), range(300, 399)];
        assert_eq!(
            find_covering_range(&ranges, 100, 199),
            Some(range(100, 199))
        );
        assert_eq!(
            find_covering_range(&ranges, 120, 150),
            Some(range(100, 199))
        );
        assert_eq!(
            find_covering_range(&ranges, 300, 300),
            Some(range(300, 399))
        );
        assert_eq!(find_covering_range(&ranges, 99, 150), None, "one ms before");
        assert_eq!(find_covering_range(&ranges, 150, 200), None, "one ms after");
        assert_eq!(find_covering_range(&ranges, 150, 350), None, "two ranges");
        assert_eq!(find_covering_range(&[], 1, 2), None, "no cache");
        // Two touching ranges never come to the check (they are joined), but
        // even then one range must hold all of it.
        let touching = [range(0, 9), range(10, 19)];
        assert_eq!(find_covering_range(&touching, 5, 15), None);
    }

    #[test]
    fn ranges_join_when_they_overlap_or_touch_and_stay_apart_otherwise() {
        assert_eq!(
            merge_ranges(&[range(100, 199)], range(150, 250)),
            vec![range(100, 250)],
            "overlap"
        );
        assert_eq!(
            merge_ranges(&[range(100, 199)], range(200, 299)),
            vec![range(100, 299)],
            "adjacent: end + 1 = start"
        );
        assert_eq!(
            merge_ranges(&[range(100, 199)], range(201, 299)),
            vec![range(100, 199), range(201, 299)],
            "one millisecond apart"
        );
        assert_eq!(
            merge_ranges(&[range(300, 399), range(500, 599)], range(0, 99)),
            vec![range(0, 99), range(300, 399), range(500, 599)],
            "earliest first"
        );
        assert_eq!(
            merge_ranges(
                &[range(0, 99), range(200, 299), range(400, 499)],
                range(100, 399)
            ),
            vec![range(0, 499)],
            "a bridge joins three"
        );
        assert_eq!(merge_ranges(&[], range(5, 5)), vec![range(5, 5)]);
        assert_eq!(
            merge_ranges(&[range(0, 999)], range(10, 20)),
            vec![range(0, 999)],
            "inside"
        );
    }

    #[test]
    fn events_of_the_recorded_range_are_replaced_and_the_rest_kept_in_order() {
        let existing = vec![
            cached(50, "a", 0),
            cached(100, "a", 1),
            cached(150, "b", 0),
            cached(200, "a", 2),
            cached(250, "b", 1),
        ];
        let new_events = vec![
            cached(210, "c", 0),
            cached(120, "c", 1),
            cached(150, "a", 7),
            // Outside the recorded range: never added.
            cached(260, "c", 2),
        ];
        let merged = replace_events_in_range(existing, range(100, 200), new_events);
        let keys: Vec<(i64, &str, u64)> = merged.iter().map(CachedEvent::sort_key).collect();
        assert_eq!(
            keys,
            vec![(50, "a", 0), (120, "c", 1), (150, "a", 7), (250, "b", 1)],
            "the old events from 100 to 200 are gone, nothing is held twice"
        );
        let untouched = replace_events_in_range(vec![cached(5, "a", 0)], range(10, 20), Vec::new());
        assert_eq!(untouched, vec![cached(5, "a", 0)]);
    }

    #[test]
    fn only_a_completed_fetch_without_failures_and_with_a_recordable_range_is_written() {
        let ok = check(JobStatus::Completed, 0);
        assert_eq!(decide_write(&ok), Some(range(1_000_000, 2_000_999)));
        let refused = [
            WriteCheck {
                cache_enabled: false,
                ..ok
            },
            WriteCheck {
                served_from_cache: true,
                ..ok
            },
            check(JobStatus::Completed, 1),
            check(JobStatus::CompletedWithFailures, 0),
            check(JobStatus::Failed, 0),
            check(JobStatus::Aborted, 0),
            check(JobStatus::Running, 0),
            WriteCheck {
                fetch_started_at_ms: 1_000_000 + RECORD_MARGIN_MS - 1,
                ..ok
            },
        ];
        for refused in refused {
            assert_eq!(decide_write(&refused), None, "{refused:?}");
        }
        let near_now = WriteCheck {
            range_end_ms: 9_999_999,
            ..ok
        };
        assert_eq!(
            decide_write(&near_now),
            Some(range(1_000_000, 10_000_000 - RECORD_MARGIN_MS))
        );
    }

    #[test]
    fn events_read_back_are_numbered_again_from_zero_per_stream() {
        let events = vec![
            cached(10, "a", 40),
            cached(10, "b", 7),
            cached(20, "a", 3),
            cached(30, "a", 99),
            cached(30, "b", 8),
        ];
        let timeline = resequence(events);
        let keys: Vec<(i64, &str, u64)> = timeline.iter().map(LogEvent::sort_key).collect();
        assert_eq!(
            keys,
            vec![
                (10, "a", 0),
                (10, "b", 0),
                (20, "a", 1),
                (30, "a", 2),
                (30, "b", 1)
            ]
        );
        assert_eq!(timeline[0].message, "a:10:40", "the rest is unchanged");
        assert_eq!(timeline[0].ingestion_time, Some(11));
        assert!(resequence(Vec::new()).is_empty());
    }

    #[test]
    fn the_key_digest_tells_kinds_apart_and_names_files() {
        let sdk = key(ProfileSelector::SdkDefault);
        let named_default = key(ProfileSelector::Named("default".to_string()));
        let named_empty = key(ProfileSelector::Named(String::new()));
        assert_ne!(sdk.digest(), named_default.digest(), "kind differs");
        assert_ne!(sdk.digest(), named_empty.digest());
        assert_eq!(sdk.digest(), key(ProfileSelector::SdkDefault).digest());
        // The parts are length-prefixed, so shifting text between them
        // gives another digest.
        let shifted_a = CacheKey::new(ProfileSelector::SdkDefault, "ab".into(), "c".into());
        let shifted_b = CacheKey::new(ProfileSelector::SdkDefault, "a".into(), "bc".into());
        assert_ne!(shifted_a.digest(), shifted_b.digest());

        let digest = sdk.digest();
        assert_eq!(digest.len(), DIGEST_HEX_LEN);
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(sdk.file_name(), format!("{digest}.cache"));
        assert_eq!(sdk.temp_file_name("1f"), format!("{digest}.cache.tmp-1f"));
        assert!(
            !sdk.file_name().contains("orders"),
            "no log group text in the name"
        );

        assert_eq!(classify_file_name(&sdk.file_name()), FileNameKind::Cache);
        assert_eq!(
            classify_file_name(&sdk.temp_file_name("x9")),
            FileNameKind::Temp
        );
        for other in [
            "settings.json",
            "notes.cache",
            &format!("{}.cache", &digest[1..]),
            &format!("{}.cache", digest.to_uppercase()),
            &format!("{digest}.cache.bak"),
            &format!("{digest}.cachex"),
            &format!("x{digest}.cache"),
        ] {
            assert_eq!(classify_file_name(other), FileNameKind::Other, "{other}");
        }
    }

    #[test]
    fn a_key_comes_from_a_request_with_a_region_only() {
        let request = FetchRequest::new(
            ProfileSelector::Named("dev".to_string()),
            Some("eu-west-1".to_string()),
            "/g".to_string(),
        );
        assert_eq!(
            CacheKey::from_request(&request),
            Some(CacheKey::new(
                ProfileSelector::Named("dev".to_string()),
                "eu-west-1".to_string(),
                "/g".to_string()
            ))
        );
        let no_region = FetchRequest::new(ProfileSelector::SdkDefault, None, "/g".to_string());
        assert_eq!(CacheKey::from_request(&no_region), None);
    }

    #[test]
    fn malformed_ranges_in_the_header_make_it_broken() {
        let expected = key(ProfileSelector::SdkDefault);
        let header = |ranges: Vec<CoveredRange>| CacheHeader {
            format_version: FORMAT_VERSION,
            key: expected.clone(),
            event_count: 0,
            range_event_counts: vec![0; ranges.len()],
            covered_ranges: ranges,
        };
        assert_eq!(
            validate_header(&header(vec![range(0, 9), range(20, 29)]), &expected),
            Ok(())
        );
        assert_eq!(
            validate_header(&header(vec![range(5, 5)]), &expected),
            Ok(())
        );
        for bad in [
            vec![range(0, 9), range(5, 29)],  // overlap
            vec![range(0, 9), range(10, 29)], // adjacent
            vec![range(20, 29), range(0, 9)], // reversed order
            vec![range(9, 0)],                // start > end
            vec![],                           // nothing covered
        ] {
            assert_eq!(
                validate_header(&header(bad.clone()), &expected),
                Err(Corruption::BadRanges),
                "{bad:?}"
            );
        }
        let other_version = CacheHeader {
            format_version: FORMAT_VERSION + 1,
            ..header(vec![range(0, 9)])
        };
        assert_eq!(
            validate_header(&other_version, &expected),
            Err(Corruption::UnknownVersion)
        );
        let other_key = header(vec![range(0, 9)]);
        let named = key(ProfileSelector::Named("default".to_string()));
        assert_eq!(
            validate_header(&other_key, &named),
            Err(Corruption::KeyMismatch)
        );
        assert!(ranges_are_well_formed(&[range(0, 0)]));
        assert!(!ranges_are_well_formed(&[]));
    }

    // ---- U7: the event counts of the header (U6 review R-03) ----

    fn counted_header(ranges: Vec<CoveredRange>, counts: Vec<u64>) -> CacheHeader {
        CacheHeader {
            format_version: FORMAT_VERSION,
            key: key(ProfileSelector::SdkDefault),
            covered_ranges: ranges,
            event_count: counts.iter().sum(),
            range_event_counts: counts,
        }
    }

    #[test]
    fn the_header_counts_the_events_of_each_range_and_in_total() {
        let ranges = vec![range(100, 199), range(300, 399)];
        let events = vec![
            cached(100, "a", 0),
            cached(150, "b", 0),
            cached(300, "a", 1),
        ];
        assert_eq!(count_events_by_range(&events, &ranges), vec![2, 1]);
        assert_eq!(count_events_by_range(&[], &ranges), vec![0, 0]);
        let good = counted_header(ranges, vec![2, 1]);
        let expected = good.key.clone();
        assert_eq!(validate_header(&good, &expected), Ok(()));
        let wrong_total = CacheHeader {
            event_count: 4,
            ..good.clone()
        };
        assert_eq!(
            validate_header(&wrong_total, &expected),
            Err(Corruption::CountMismatch)
        );
        let missing_range_count = CacheHeader {
            range_event_counts: vec![3],
            event_count: 3,
            ..good
        };
        assert_eq!(
            validate_header(&missing_range_count, &expected),
            Err(Corruption::CountMismatch)
        );
    }

    #[test]
    fn a_file_cut_at_a_line_boundary_is_broken_as_a_whole_and_in_its_range() {
        let header = counted_header(vec![range(100, 199), range(300, 399)], vec![2, 1]);
        let all = [
            cached(100, "a", 0),
            cached(150, "b", 0),
            cached(300, "a", 1),
        ];
        let mut whole = EventCounter::new(&header);
        all.iter().for_each(|event| whole.count(event));
        assert_eq!(whole.verify_all(), Ok(()));
        assert_eq!(whole.verify_range(range(100, 199)), Ok(()));
        assert_eq!(whole.verify_range(range(300, 399)), Ok(()));
        // The last line is missing, cut after a whole line.
        let mut cut = EventCounter::new(&header);
        all[..2].iter().for_each(|event| cut.count(event));
        assert_eq!(cut.verify_all(), Err(Corruption::CountMismatch));
        assert_eq!(
            cut.verify_range(range(100, 199)),
            Ok(()),
            "that range is whole"
        );
        assert_eq!(
            cut.verify_range(range(300, 399)),
            Err(Corruption::CountMismatch)
        );
        assert_eq!(
            cut.verify_range(range(500, 599)),
            Err(Corruption::CountMismatch),
            "not a range of the header"
        );
    }

    #[test]
    fn a_version_1_header_is_an_unknown_version_and_version_2_writes_the_counts() {
        assert_eq!(FORMAT_VERSION, 2);
        let expected = key(ProfileSelector::SdkDefault);
        let version_1 = serde_json::json!({
            "formatVersion": 1,
            "key": expected,
            "coveredRanges": [{ "startMs": 100, "endMs": 199 }],
        });
        let parsed: CacheHeader = serde_json::from_value(version_1).unwrap();
        assert_eq!(
            validate_header(&parsed, &expected),
            Err(Corruption::UnknownVersion)
        );
        let written = serde_json::to_value(counted_header(vec![range(100, 199)], vec![4])).unwrap();
        assert_eq!(written["formatVersion"], 2);
        assert_eq!(written["eventCount"], 4);
        assert_eq!(written["rangeEventCounts"], serde_json::json!([4]));
    }

    #[test]
    fn events_read_must_lie_in_a_covered_range_and_keep_the_order() {
        let ranges = [range(0, 99), range(200, 299)];
        let good = vec![
            cached(0, "a", 0),
            cached(50, "a", 1),
            cached(50, "b", 0),
            cached(299, "a", 0),
        ];
        assert_eq!(validate_events(&good, &ranges), Ok(()));
        assert_eq!(validate_events(&[], &ranges), Ok(()));
        assert_eq!(
            validate_events(&[cached(150, "a", 0)], &ranges),
            Err(Corruption::EventOutsideRanges)
        );
        assert_eq!(
            validate_events(&[cached(300, "a", 0)], &ranges),
            Err(Corruption::EventOutsideRanges)
        );
        assert_eq!(
            validate_events(&[cached(50, "b", 0), cached(50, "a", 1)], &ranges),
            Err(Corruption::EventsOutOfOrder)
        );
        assert_eq!(
            validate_events(&[cached(60, "a", 0), cached(50, "a", 1)], &ranges),
            Err(Corruption::EventsOutOfOrder)
        );
        let mut one_by_one = EventCheck::new(&ranges);
        assert_eq!(one_by_one.check(&cached(10, "a", 0)), Ok(()));
        assert_eq!(one_by_one.check(&cached(10, "a", 1)), Ok(()));
        assert_eq!(
            one_by_one.check(&cached(5, "a", 2)),
            Err(Corruption::EventsOutOfOrder)
        );
    }

    #[test]
    fn notices_come_from_hit_read_failure_and_save_failure() {
        assert_eq!(
            cache_notices(true, false, CacheOutcome::Hit),
            vec![CacheNotice::Hit]
        );
        assert_eq!(
            cache_notices(false, true, CacheOutcome::SaveFailed),
            vec![CacheNotice::ReadFailed, CacheNotice::SaveFailed],
            "both appear"
        );
        assert_eq!(
            cache_notices(false, true, CacheOutcome::Saved),
            vec![CacheNotice::ReadFailed]
        );
        assert_eq!(
            cache_notices(false, false, CacheOutcome::SaveFailed),
            vec![CacheNotice::SaveFailed]
        );
        for quiet in [
            CacheOutcome::Saved,
            CacheOutcome::NotSaved,
            CacheOutcome::NotUsed,
        ] {
            assert!(cache_notices(false, false, quiet).is_empty(), "{quiet:?}");
        }
    }

    #[test]
    fn a_fetched_event_becomes_a_cached_event_and_compares_by_the_key() {
        let event = LogEvent {
            timestamp: 7,
            ingestion_time: None,
            message: "line 1\nline 2".to_string(),
            log_stream_name: "s".to_string(),
            sequence: 3,
        };
        let stored = CachedEvent::from(event);
        assert_eq!(stored.sort_key(), (7, "s", 3));
        assert_eq!(stored.message, "line 1\nline 2");
        assert_eq!(
            compare_cached(&cached(1, "b", 0), &cached(1, "a", 9)),
            Ordering::Greater
        );
        assert!(range(10, 20).contains_instant(10) && range(10, 20).contains_instant(20));
        assert!(!range(10, 20).contains_instant(21));
        assert!(range(10, 20).contains_range(10, 20) && !range(10, 20).contains_range(9, 20));
    }
}
