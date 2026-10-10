//! FilterEngine: narrows the held events to those whose message contains a
//! text, ignoring case (U5:BR1.1-BR1.5, BR2.1-BR2.4, BR3.6).
//!
//! The result is kept as the ordering keys `(timestamp, logStreamName,
//! sequence)` of the matching events, in key order (BR2.3), so adding
//! events never shifts it (review R-01). Stream names are numbered once per
//! timeline and the keys hold the number, so no name is copied per event
//! (review R-09).
//!
//! The whole timeline is judged by a scan that runs in chunks (BR1.5): the
//! caller takes the lock, judges one chunk with [`FilterEngine::scan_chunk`]
//! and releases the lock. Pages that arrive meanwhile are judged by
//! [`FilterEngine::on_added`] only up to the scan cursor; the keys after it
//! are left to the scan, so nothing is missed or counted twice (BR2.1).
//! Each piece of work carries a [`ScanTicket`]; a ticket of an older text or
//! an older timeline is stale and its work is dropped (BR2.4).
//!
//! The engine is pure: it never locks, never waits and calls no API
//! (FR6.2). [`crate::log_view::LogView`] puts it behind one lock together
//! with the [`crate::timeline::EventTimeline`] (BR2.5, review R-02).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;

use crate::event::LogEvent;

/// BR1.1: the filter text without surrounding white space. An empty result
/// means "no filter". There is no length limit.
pub fn normalize_filter_text(raw: &str) -> String {
    raw.trim().to_string()
}

/// Least time between two progress reports of one scan (U5 review R-03).
pub const FILTER_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Whether a scan reports its progress now (U5:BR3.6, U5 review R-03): the
/// first report always goes, the last one (the scan is Ready) always goes,
/// and in between at most one per [`FILTER_PROGRESS_INTERVAL`].
/// `since_last_report` is the time since the previous report, `None` when
/// none was sent yet.
pub fn should_report_progress(since_last_report: Option<Duration>, ready: bool) -> bool {
    ready || since_last_report.is_none_or(|elapsed| elapsed >= FILTER_PROGRESS_INTERVAL)
}

/// The condition in force (entities.md FilterCondition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterCondition {
    /// Incremented whenever the text changes (BR1.4).
    pub filter_id: u64,
    /// The trimmed text; empty means no filter (BR1.1).
    pub text: String,
    /// The text in lower case, made once when the condition is set (Q1).
    needle: String,
}

impl FilterCondition {
    /// A condition numbered `filter_id` for `text`, trimmed here (BR1.1).
    pub fn new(filter_id: u64, text: &str) -> Self {
        let text = normalize_filter_text(text);
        let needle = text.to_lowercase();
        Self {
            filter_id,
            text,
            needle,
        }
    }

    /// Whether the condition filters at all (its text is not empty).
    pub fn is_active(&self) -> bool {
        !self.text.is_empty()
    }

    /// BR1.2: whether the whole `message` (every line of it) contains the
    /// text, both compared in Unicode lower case. Only the message is
    /// compared, never the time or the stream name.
    pub fn matches(&self, message: &str) -> bool {
        message.to_lowercase().contains(&self.needle)
    }
}

/// Whether the whole timeline has been filtered yet (entities.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FilterStatus {
    /// The scan of the held events is still running; the result holds the
    /// matching events found so far (BR1.5).
    Filtering,
    /// Every held event has been judged.
    Ready,
}

/// What the screen shows of the filter (entities.md
/// SessionState.filterSummary, BR3.3, BR3.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSummary {
    /// The condition the result belongs to.
    pub filter_id: u64,
    /// Number of matching events found so far.
    pub matched_count: u64,
    /// Number of held events when the result was last updated.
    pub all_count: u64,
    /// Filtering or Ready.
    pub status: FilterStatus,
    /// Incremented whenever the result changes.
    pub result_version: u64,
}

/// The summary together with the result version, also when no filter is
/// in force, so the screen can order the messages it receives (BR3.6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterState {
    /// Result version of the engine; it never decreases.
    pub result_version: u64,
    /// `None` when the text is empty (entities.md).
    pub summary: Option<FilterSummary>,
}

/// Identifies the work a scan belongs to (BR2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanTicket {
    /// The condition the scan was started for.
    pub filter_id: u64,
    /// The timeline generation the scan was started for.
    pub timeline_epoch: u64,
}

/// What a text change did (BR1.1, BR1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterChange {
    /// The trimmed text is the same as before: nothing happened.
    Unchanged,
    /// The text became empty: no filter any more.
    Cleared,
    /// A new filter started with an empty result; scan the held events
    /// with this ticket (BR1.5).
    Started(ScanTicket),
}

/// Outcome of one scan chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStep {
    /// Some events were judged; more remain.
    Progressed,
    /// The end of the timeline was reached: the result is Ready.
    Finished,
    /// The ticket belongs to an older text or an older timeline: the scan
    /// must stop and nothing was changed (BR2.4).
    Stale,
}

/// Ordering key of a matched event; the stream is a number into the
/// engine's stream table (review R-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatchKey {
    timestamp: i64,
    stream: u32,
    sequence: u64,
}

/// Key of the last scanned event (BR1.5). One per chunk, so the copied
/// name costs nothing per event.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CursorKey {
    timestamp: i64,
    stream: String,
    sequence: u64,
}

impl CursorKey {
    fn of(event: &LogEvent) -> Self {
        Self {
            timestamp: event.timestamp,
            stream: event.log_stream_name.clone(),
            sequence: event.sequence,
        }
    }

    fn as_key(&self) -> (i64, &str, u64) {
        (self.timestamp, self.stream.as_str(), self.sequence)
    }
}

/// Stream names numbered in order of first use. Keys compare the names,
/// not the numbers, so the order stays the timeline's (U3:BR4.1).
#[derive(Debug, Clone, Default)]
struct StreamTable {
    ids: HashMap<String, u32>,
    names: Vec<String>,
}

impl StreamTable {
    /// The number of `name`, numbering it when new (the only copy).
    fn id_of(&mut self, name: &str) -> u32 {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        let id = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.names.push(name.to_string());
        self.ids.insert(name.to_string(), id);
        id
    }

    fn name(&self, id: u32) -> &str {
        usize::try_from(id)
            .ok()
            .and_then(|index| self.names.get(index))
            .map_or("", String::as_str)
    }

    fn clear(&mut self) {
        self.ids.clear();
        self.names.clear();
    }
}

/// Condition and result of the log filter (entities.md FilterCondition,
/// FilterResult).
#[derive(Debug, Clone)]
pub struct FilterEngine {
    condition: FilterCondition,
    /// Incremented by every discard of the timeline (BR2.2).
    timeline_epoch: u64,
    /// matchedKeys, in key order without duplicates (BR2.3).
    matched: Vec<MatchKey>,
    streams: StreamTable,
    /// Only while Filtering, and only once a chunk was judged (review R-07).
    scan_cursor: Option<CursorKey>,
    status: FilterStatus,
    all_count: u64,
    result_version: u64,
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterEngine {
    /// No filter, filter ID 0, timeline epoch 0, result version 0.
    pub fn new() -> Self {
        Self {
            condition: FilterCondition::new(0, ""),
            timeline_epoch: 0,
            matched: Vec::new(),
            streams: StreamTable::default(),
            scan_cursor: None,
            status: FilterStatus::Ready,
            all_count: 0,
            result_version: 0,
        }
    }

    /// Sets the text; `held` is the number of held events.
    ///
    /// The text is trimmed (BR1.1). The same trimmed text again changes
    /// nothing (BR1.4). A different one increments the filter ID, which
    /// makes every running scan stale, and replaces the result by an empty
    /// one: Ready with no filter when the text is empty, otherwise Filtering
    /// with no cursor, to be filled by a scan with the returned ticket
    /// (BR1.5).
    pub fn set_text(&mut self, raw: &str, held: usize) -> FilterChange {
        let text = normalize_filter_text(raw);
        if text == self.condition.text {
            return FilterChange::Unchanged;
        }
        self.condition = FilterCondition::new(self.condition.filter_id + 1, &text);
        self.matched.clear();
        self.streams.clear();
        self.scan_cursor = None;
        self.all_count = held as u64;
        self.result_version += 1;
        if self.condition.is_active() {
            self.status = FilterStatus::Filtering;
            FilterChange::Started(self.ticket())
        } else {
            self.status = FilterStatus::Ready;
            FilterChange::Cleared
        }
    }

    /// The ticket of the current text and timeline.
    pub fn ticket(&self) -> ScanTicket {
        ScanTicket {
            filter_id: self.condition.filter_id,
            timeline_epoch: self.timeline_epoch,
        }
    }

    fn is_current(&self, ticket: ScanTicket) -> bool {
        ticket == self.ticket()
    }

    /// Index in `events` (the held events, in order) of the first event the
    /// next chunk judges: the first one whose key is greater than the
    /// cursor, found by binary search (review R-07); 0 before the first
    /// chunk.
    pub fn scan_start(&self, events: &[LogEvent]) -> usize {
        match &self.scan_cursor {
            None => 0,
            Some(cursor) => {
                let cursor = cursor.as_key();
                events.partition_point(|event| event.sort_key() <= cursor)
            }
        }
    }

    /// Judges at most `max_rows` held events after the cursor (BR1.5).
    ///
    /// `events` must be the held events, in order. A stale ticket changes
    /// nothing (BR2.4). Otherwise the matching events are added to the
    /// result, the cursor moves to the last judged event, the held count is
    /// refreshed and the result version is incremented; at the end of the
    /// events the result becomes Ready and the cursor goes away.
    pub fn scan_chunk(
        &mut self,
        events: &[LogEvent],
        ticket: ScanTicket,
        max_rows: usize,
    ) -> ScanStep {
        if !self.is_current(ticket) || !self.condition.is_active() {
            return ScanStep::Stale;
        }
        if self.status == FilterStatus::Ready {
            return ScanStep::Finished;
        }
        let start = self.scan_start(events);
        let end = start.saturating_add(max_rows.max(1)).min(events.len());
        let chunk = &events[start..end];
        for event in chunk {
            if self.condition.matches(&event.message) {
                self.insert(event);
            }
        }
        self.all_count = events.len() as u64;
        self.result_version += 1;
        if end >= events.len() {
            self.status = FilterStatus::Ready;
            self.scan_cursor = None;
            ScanStep::Finished
        } else {
            self.scan_cursor = chunk.last().map(CursorKey::of);
            ScanStep::Progressed
        }
    }

    /// Judges a page that is being added to the timeline; `held_after` is
    /// the held count once it is added (BR2.1).
    ///
    /// Without a filter nothing changes. When Ready every event of the page
    /// is judged; while Filtering only the events whose key is not greater
    /// than the cursor (none before the first chunk): the others are judged
    /// later by the scan. The held count is refreshed and the result
    /// version incremented. The cost is proportional to the page.
    pub fn on_added(&mut self, batch: &[LogEvent], held_after: usize) {
        if !self.condition.is_active() {
            self.all_count = held_after as u64;
            return;
        }
        let limit = match (self.status, &self.scan_cursor) {
            (FilterStatus::Ready, _) => None,
            (FilterStatus::Filtering, Some(cursor)) => Some(cursor.clone()),
            (FilterStatus::Filtering, None) => {
                self.all_count = held_after as u64;
                self.result_version += 1;
                return;
            }
        };
        for event in batch {
            let scanned = limit
                .as_ref()
                .is_none_or(|cursor| event.sort_key() <= cursor.as_key());
            if scanned && self.condition.matches(&event.message) {
                self.insert(event);
            }
        }
        self.all_count = held_after as u64;
        self.result_version += 1;
    }

    /// The timeline was discarded (BR2.2): the timeline epoch is incremented
    /// (so running work becomes stale) and the result is replaced by an
    /// empty Ready one; the text stays.
    pub fn on_discarded(&mut self) {
        self.timeline_epoch += 1;
        self.matched.clear();
        self.streams.clear();
        self.scan_cursor = None;
        self.status = FilterStatus::Ready;
        self.all_count = 0;
        self.result_version += 1;
    }

    /// Adds the key of `event` in key order unless it is already there
    /// (idempotent, review R-02).
    fn insert(&mut self, event: &LogEvent) {
        let probe = event.sort_key();
        let at_end = self
            .matched
            .last()
            .is_none_or(|last| self.compare(last, probe).is_lt());
        let position = if at_end {
            self.matched.len()
        } else {
            match self
                .matched
                .binary_search_by(|held| self.compare(held, probe))
            {
                Ok(_) => return,
                Err(position) => position,
            }
        };
        let key = MatchKey {
            timestamp: event.timestamp,
            stream: self.streams.id_of(&event.log_stream_name),
            sequence: event.sequence,
        };
        self.matched.insert(position, key);
    }

    /// Compares a matched key with `(timestamp, stream name, sequence)`.
    fn compare(&self, held: &MatchKey, probe: (i64, &str, u64)) -> Ordering {
        held.timestamp
            .cmp(&probe.0)
            .then_with(|| self.streams.name(held.stream).cmp(probe.1))
            .then_with(|| held.sequence.cmp(&probe.2))
    }

    /// The condition in force.
    pub fn condition(&self) -> &FilterCondition {
        &self.condition
    }

    /// Whether a filter is in force (the text is not empty).
    pub fn is_active(&self) -> bool {
        self.condition.is_active()
    }

    /// Current filter ID.
    pub fn filter_id(&self) -> u64 {
        self.condition.filter_id
    }

    /// Current timeline generation (entities.md FilterResult.timelineEpoch).
    pub fn timeline_epoch(&self) -> u64 {
        self.timeline_epoch
    }

    /// Filtering or Ready.
    pub fn status(&self) -> FilterStatus {
        self.status
    }

    /// Current result version.
    pub fn result_version(&self) -> u64 {
        self.result_version
    }

    /// Number of matched events found so far.
    pub fn matched_count(&self) -> usize {
        self.matched.len()
    }

    /// The key `(timestamp, logStreamName, sequence)` of the matched event
    /// at `index` in the result (BR3.1).
    pub fn matched_key(&self, index: usize) -> Option<(i64, &str, u64)> {
        let key = self.matched.get(index)?;
        Some((key.timestamp, self.streams.name(key.stream), key.sequence))
    }

    /// The key of the last scanned event, while Filtering after the first
    /// chunk.
    pub fn scan_cursor(&self) -> Option<(i64, &str, u64)> {
        self.scan_cursor.as_ref().map(CursorKey::as_key)
    }

    /// Position in the result of the event with this key, by binary search
    /// (BR3.2, review R-08); `None` when it is not in the result.
    pub fn position_of(
        &self,
        timestamp: i64,
        log_stream_name: &str,
        sequence: u64,
    ) -> Option<usize> {
        self.streams.ids.get(log_stream_name)?;
        let probe = (timestamp, log_stream_name, sequence);
        self.matched
            .binary_search_by(|held| self.compare(held, probe))
            .ok()
    }

    /// What the screen shows; `None` when no filter is in force (BR3.3).
    pub fn summary(&self) -> Option<FilterSummary> {
        self.condition.is_active().then_some(FilterSummary {
            filter_id: self.condition.filter_id,
            matched_count: self.matched.len() as u64,
            all_count: self.all_count,
            status: self.status,
            result_version: self.result_version,
        })
    }

    /// The summary with the result version (BR3.6).
    pub fn state(&self) -> FilterState {
        FilterState {
            result_version: self.result_version,
            summary: self.summary(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(timestamp: i64, stream: &str, sequence: u64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn sorted(mut events: Vec<LogEvent>) -> Vec<LogEvent> {
        events.sort_by(crate::event::compare_events);
        events
    }

    fn matched(engine: &FilterEngine) -> Vec<(i64, String, u64)> {
        (0..engine.matched_count())
            .filter_map(|i| engine.matched_key(i))
            .map(|(t, s, q)| (t, s.to_string(), q))
            .collect()
    }

    fn key(timestamp: i64, stream: &str, sequence: u64) -> (i64, String, u64) {
        (timestamp, stream.to_string(), sequence)
    }

    /// Scans `events` to the end in chunks of `chunk`.
    fn scan_all(engine: &mut FilterEngine, events: &[LogEvent], chunk: usize) {
        let ticket = engine.ticket();
        for _ in 0..=events.len() {
            if engine.scan_chunk(events, ticket, chunk) != ScanStep::Progressed {
                return;
            }
        }
        panic!("the scan did not finish");
    }

    #[test]
    fn the_text_is_trimmed_and_an_empty_text_means_no_filter() {
        assert_eq!(normalize_filter_text("  error \n"), "error");
        assert_eq!(normalize_filter_text(" \t "), "");
        let mut engine = FilterEngine::new();
        assert_eq!(engine.set_text("   ", 3), FilterChange::Unchanged);
        assert!(!engine.is_active());
        assert_eq!(engine.summary(), None);

        let FilterChange::Started(ticket) = engine.set_text(" Err ", 3) else {
            panic!("a non-empty text starts a filter");
        };
        assert_eq!(engine.condition().text, "Err");
        assert_eq!(ticket.filter_id, engine.filter_id());
        let summary = engine.summary().unwrap();
        assert_eq!(summary.status, FilterStatus::Filtering);
        assert_eq!((summary.matched_count, summary.all_count), (0, 3));
        assert_eq!(engine.scan_cursor(), None, "nothing scanned yet");

        assert_eq!(engine.set_text("  ", 3), FilterChange::Cleared);
        assert!(!engine.is_active());
        assert_eq!(engine.summary(), None);
        assert_eq!(engine.status(), FilterStatus::Ready);
    }

    #[test]
    fn matching_ignores_case_reads_every_line_and_compares_only_the_message() {
        let condition = FilterCondition::new(1, "error");
        assert!(condition.is_active());
        assert!(condition.matches("Error: timeout"));
        assert!(condition.matches("first line\nsecond line has an ERROR"));
        assert!(!condition.matches("all good"));
        assert!(FilterCondition::new(2, "タイムアウト").matches("接続がタイムアウトしました"));
        assert!(FilterCondition::new(3, "ÄRGER").matches("großer ärger"));
        assert!(!FilterCondition::new(4, "").is_active());

        let events = sorted(vec![
            ev(1, "s", 0, "Error: timeout"),
            ev(2, "s", 1, "first\nsecond ERROR here"),
            ev(3, "error-stream", 0, "ok"),
            ev(4, "s", 2, "接続がタイムアウトしました"),
        ]);
        let mut engine = FilterEngine::new();
        engine.set_text("error", events.len());
        scan_all(&mut engine, &events, 10);
        assert_eq!(matched(&engine), vec![key(1, "s", 0), key(2, "s", 1)]);
        engine.set_text("タイムアウト", events.len());
        scan_all(&mut engine, &events, 10);
        assert_eq!(matched(&engine), vec![key(4, "s", 2)]);
    }

    #[test]
    fn the_same_text_keeps_the_filter_id_and_a_new_text_increments_it() {
        let events = sorted(vec![ev(1, "s", 0, "a b"), ev(2, "s", 1, "b")]);
        let mut engine = FilterEngine::new();
        let FilterChange::Started(first) = engine.set_text("a", 2) else {
            panic!("started");
        };
        assert_eq!(first.filter_id, 1);
        assert_eq!(engine.set_text(" a ", 2), FilterChange::Unchanged);
        assert_eq!(engine.filter_id(), 1);
        let FilterChange::Started(second) = engine.set_text("b", 2) else {
            panic!("started");
        };
        assert_eq!(second.filter_id, 2);
        assert_eq!(engine.scan_chunk(&events, first, 10), ScanStep::Stale);
        assert_eq!(engine.matched_count(), 0, "the old scan adds nothing");
        assert_eq!(engine.scan_chunk(&events, second, 10), ScanStep::Finished);
        assert_eq!(engine.matched_count(), 2);
        assert_eq!(engine.set_text("", 2), FilterChange::Cleared);
        assert_eq!(engine.filter_id(), 3, "clearing is a change too");
    }

    #[test]
    fn results_stay_in_key_order_without_duplicates() {
        let mut engine = FilterEngine::new();
        engine.set_text("x", 0);
        scan_all(&mut engine, &[], 10);
        assert_eq!(engine.status(), FilterStatus::Ready);
        // Stream "b" is numbered before "a", but "a" sorts first.
        let page = sorted(vec![
            ev(5, "b", 0, "x"),
            ev(5, "a", 0, "x"),
            ev(1, "b", 1, "x"),
            ev(9, "a", 1, "no"),
        ]);
        engine.on_added(&page, 4);
        engine.on_added(&page[..1], 4);
        engine.on_added(&[ev(3, "c", 0, "X")], 5);
        assert_eq!(
            matched(&engine),
            vec![
                key(1, "b", 1),
                key(3, "c", 0),
                key(5, "a", 0),
                key(5, "b", 0)
            ]
        );
        assert_eq!(engine.summary().unwrap().matched_count, 4);
        assert_eq!(engine.position_of(5, "b", 0), Some(3));
        assert_eq!(engine.position_of(9, "a", 1), None);
        assert_eq!(engine.position_of(5, "zz", 0), None);
    }

    #[test]
    fn a_scan_runs_in_chunks_moves_its_cursor_and_ends_ready() {
        let events = sorted((0..5).map(|i| ev(i, "s", i as u64, "hit")).collect());
        let mut engine = FilterEngine::new();
        let FilterChange::Started(ticket) = engine.set_text("HIT", 5) else {
            panic!("started");
        };
        assert_eq!(engine.scan_chunk(&events, ticket, 2), ScanStep::Progressed);
        assert_eq!(engine.scan_cursor(), Some((1, "s", 1)));
        assert_eq!(engine.status(), FilterStatus::Filtering);
        assert_eq!(engine.matched_count(), 2, "rows found so far");
        assert_eq!(engine.scan_chunk(&events, ticket, 2), ScanStep::Progressed);
        assert_eq!(engine.scan_cursor(), Some((3, "s", 3)));
        assert_eq!(engine.scan_chunk(&events, ticket, 2), ScanStep::Finished);
        assert_eq!(engine.status(), FilterStatus::Ready);
        assert_eq!(engine.scan_cursor(), None);
        assert_eq!(engine.matched_count(), 5);
        assert_eq!(engine.summary().unwrap().all_count, 5);
        assert_eq!(engine.scan_chunk(&events, ticket, 2), ScanStep::Finished);
    }

    #[test]
    fn pages_added_while_filtering_are_judged_only_up_to_the_cursor() {
        let mut held = sorted(vec![
            ev(10, "s", 0, "m"),
            ev(20, "s", 1, "m"),
            ev(30, "s", 2, "m"),
            ev(40, "s", 3, "m"),
        ]);
        let mut engine = FilterEngine::new();
        let FilterChange::Started(ticket) = engine.set_text("m", held.len()) else {
            panic!("started");
        };
        // Before the first chunk nothing is scanned: a page is left to the scan.
        let early = vec![ev(5, "t", 0, "m")];
        engine.on_added(&early, held.len() + 1);
        assert_eq!(engine.matched_count(), 0);
        held = sorted([held, early].concat());

        engine.scan_chunk(&held, ticket, 2);
        assert_eq!(engine.scan_cursor(), Some((10, "s", 0)));
        let page = vec![
            ev(7, "t", 1, "m"),
            ev(15, "t", 2, "m"),
            ev(16, "t", 3, "other"),
            ev(35, "t", 4, "m"),
        ];
        engine.on_added(&page, held.len() + page.len());
        assert_eq!(
            matched(&engine),
            vec![key(5, "t", 0), key(7, "t", 1), key(10, "s", 0)],
            "only keys up to the cursor are judged now"
        );
        held = sorted([held, page].concat());
        scan_all(&mut engine, &held, 2);
        assert_eq!(
            matched(&engine),
            vec![
                key(5, "t", 0),
                key(7, "t", 1),
                key(10, "s", 0),
                key(15, "t", 2),
                key(20, "s", 1),
                key(30, "s", 2),
                key(35, "t", 4),
                key(40, "s", 3),
            ],
            "nothing missed, nothing twice"
        );
        // Ready: every added event is judged.
        engine.on_added(&[ev(1, "u", 0, "m"), ev(50, "u", 1, "m")], held.len() + 2);
        assert_eq!(engine.matched_count(), 10);
        assert_eq!(engine.summary().unwrap().all_count, held.len() as u64 + 2);
    }

    #[test]
    fn a_discard_or_a_new_text_makes_running_work_stale() {
        let events = sorted((0..4).map(|i| ev(i, "s", i as u64, "m")).collect());
        let mut engine = FilterEngine::new();
        let FilterChange::Started(ticket) = engine.set_text("m", 4) else {
            panic!("started");
        };
        engine.scan_chunk(&events, ticket, 2);
        assert_eq!(engine.matched_count(), 2);
        let epoch = engine.timeline_epoch();
        engine.on_discarded();
        assert_eq!(engine.timeline_epoch(), epoch + 1);
        assert_eq!(engine.matched_count(), 0);
        assert_eq!(engine.status(), FilterStatus::Ready);
        assert_eq!(engine.scan_cursor(), None);
        assert_eq!(engine.condition().text, "m", "the text stays");
        let summary = engine.summary().unwrap();
        assert_eq!((summary.matched_count, summary.all_count), (0, 0));
        assert_eq!(engine.scan_chunk(&events, ticket, 2), ScanStep::Stale);
        assert_eq!(engine.matched_count(), 0, "the old scan adds nothing");
        assert_eq!(engine.ticket().timeline_epoch, epoch + 1);
    }

    #[test]
    fn every_change_of_the_result_increments_the_result_version() {
        let events = sorted((0..3).map(|i| ev(i, "s", i as u64, "m")).collect());
        let mut engine = FilterEngine::new();
        let mut last = engine.result_version();
        let mut grew = |engine: &FilterEngine, what: &str| {
            let now = engine.result_version();
            assert!(now > last, "{what}: {now} > {last}");
            last = now;
        };
        engine.on_added(&events, 3);
        assert_eq!(engine.result_version(), 0, "no filter, no result");
        engine.set_text("m", 3);
        grew(&engine, "new text");
        let ticket = engine.ticket();
        engine.scan_chunk(&events, ticket, 2);
        grew(&engine, "chunk");
        engine.scan_chunk(&events, ticket, 2);
        grew(&engine, "last chunk");
        engine.on_added(&[ev(9, "s", 3, "m")], 4);
        grew(&engine, "page");
        engine.on_discarded();
        grew(&engine, "discard");
        let before = engine.result_version();
        engine.set_text("m ", 0);
        assert_eq!(engine.result_version(), before, "same text");
        engine.set_text("", 0);
        grew(&engine, "cleared");
        assert_eq!(engine.state().result_version, engine.result_version());
        assert_eq!(engine.state().summary, None);
    }
}

#[cfg(test)]
mod should_report_progress_tests {
    use std::time::Duration;

    use super::{FILTER_PROGRESS_INTERVAL, should_report_progress};

    #[test]
    fn the_first_report_always_goes() {
        assert!(should_report_progress(None, false));
        assert!(should_report_progress(None, true));
    }

    #[test]
    fn reports_closer_than_the_interval_are_skipped() {
        assert!(!should_report_progress(Some(Duration::ZERO), false));
        assert!(!should_report_progress(
            Some(FILTER_PROGRESS_INTERVAL - Duration::from_millis(1)),
            false
        ));
    }

    #[test]
    fn a_report_goes_once_the_interval_has_passed() {
        assert_eq!(FILTER_PROGRESS_INTERVAL, Duration::from_millis(100));
        assert!(should_report_progress(
            Some(FILTER_PROGRESS_INTERVAL),
            false
        ));
        assert!(should_report_progress(Some(Duration::from_secs(5)), false));
    }

    #[test]
    fn the_ready_report_always_goes_whatever_the_interval() {
        assert!(should_report_progress(Some(Duration::ZERO), true));
        assert!(should_report_progress(Some(Duration::from_millis(1)), true));
    }
}
