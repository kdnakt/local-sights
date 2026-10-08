//! LogView: the held events and the log filter behind one lock (U5 review
//! R-02, BR2.5).
//!
//! [`EventTimeline`] and [`FilterEngine`] live in one struct that the app
//! keeps in one `Mutex`. The fetch reaches it as a [`TimelineStore`]: adding
//! a page also judges it against the filter (BR2.1), and discarding the
//! timeline also empties the result and advances its epoch (BR2.2), both
//! within the same lock, so a viewport read never sees one without the
//! other. Rows and positions are read with the filter in mind (BR3.1,
//! BR3.2). The lock order of the app becomes session, then LogView.
//!
//! Since U7 the LogView owns the discard generation (review R-11): it
//! advances whenever held logs are discarded, in the same step and under
//! the same lock as the timeline version, so the screen knows when the
//! `(logStreamName, sequence)` keys of its expanded rows went stale
//! (U7:BR1.6). [`LogView::positions_of`] answers the positions of several
//! rows in one lock with the versions they belong to (U7:BR1.7).

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::cache::plan::CoveredRange;
use crate::coordinator::{HeldEvents, TimelineStore, copy_chunk_in_range};
use crate::event::LogEvent;
use crate::filter::{FilterChange, FilterEngine, FilterState, ScanStep, ScanTicket};
use crate::timeline::{EventTimeline, RowWindow};

/// The held events of the current fetch and the log filter over them.
#[derive(Debug, Clone, Default)]
pub struct LogView {
    timeline: EventTimeline,
    filter: FilterEngine,
    /// Times held logs were discarded (U7:BR1.6, review R-11).
    discard_generation: u64,
}

/// One held event as the screen names it (U3:BR4.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowKey {
    /// Stream of the event.
    pub log_stream_name: String,
    /// Position of the event within its stream in this fetch.
    pub sequence: u64,
}

/// Answer of [`LogView::positions_of`] (U7:BR1.7): one position per key, in
/// the order of the keys, and the versions they belong to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowPositions {
    /// Position in the current list, `None` when hidden by the filter or not
    /// held.
    pub positions: Vec<Option<u64>>,
    /// Version of the timeline (U3:BR4.5).
    pub timeline_version: u64,
    /// Version of the filter result (U5:BR3.6).
    pub result_version: u64,
    /// Discard generation (U7:BR1.6).
    pub discard_generation: u64,
    /// Number of rows of the current list: the held events, or the matching
    /// ones while a filter is in force.
    pub total_count: u64,
}

impl LogView {
    /// No events, no filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// The held events.
    pub fn timeline(&self) -> &EventTimeline {
        &self.timeline
    }

    /// The filter condition and result.
    pub fn filter(&self) -> &FilterEngine {
        &self.filter
    }

    /// Version of the timeline (U3:BR4.5).
    pub fn version(&self) -> u64 {
        self.timeline.version()
    }

    /// Adds one page (U3:BR4.2) and judges it against the filter in the same
    /// step (BR2.1); returns the new timeline version. An empty page changes
    /// nothing.
    pub fn append(&mut self, events: Vec<LogEvent>) -> u64 {
        if events.is_empty() {
            return self.timeline.version();
        }
        let held_after = self.timeline.len() + events.len();
        self.filter.on_added(&events, held_after);
        self.timeline.append(events)
    }

    /// Discards every event (U3:BR4.5), empties the result and advances its
    /// epoch, keeping the text (BR2.2); returns the new timeline version.
    /// When events were held, the discard generation advances too (U7:BR1.6,
    /// review R-11): the fetch start, a connection change and an abort all
    /// discard through here.
    pub fn clear(&mut self) -> u64 {
        if !self.timeline.is_empty() {
            self.discard_generation += 1;
        }
        self.filter.on_discarded();
        self.timeline.clear()
    }

    /// Times held logs were discarded (U7:BR1.6); adding never changes it.
    pub fn discard_generation(&self) -> u64 {
        self.discard_generation
    }

    /// U7:BR1.7: the current positions of `keys` in one lock (with a filter
    /// in force, positions in the result, as [`LogView::position_of`]), with
    /// the timeline version, the result version, the discard generation and
    /// the number of rows of the list.
    pub fn positions_of(&self, keys: &[RowKey]) -> RowPositions {
        let total_count = if self.filter.is_active() {
            self.filter.matched_count()
        } else {
            self.timeline.len()
        };
        RowPositions {
            positions: keys
                .iter()
                .map(|key| {
                    self.position_of(&key.log_stream_name, key.sequence)
                        .map(|position| position as u64)
                })
                .collect(),
            timeline_version: self.timeline.version(),
            result_version: self.filter.result_version(),
            discard_generation: self.discard_generation,
            total_count: total_count as u64,
        }
    }

    /// Sets the filter text (BR1.1, BR1.4); on [`FilterChange::Started`] the
    /// caller scans the held events with the ticket (BR1.5).
    pub fn set_filter(&mut self, text: &str) -> FilterChange {
        self.filter.set_text(text, self.timeline.len())
    }

    /// Judges one chunk of at most `max_rows` held events (BR1.5); the caller
    /// holds the lock only for this call. A stale ticket changes nothing
    /// (BR2.4).
    pub fn scan_chunk(&mut self, ticket: ScanTicket, max_rows: usize) -> ScanStep {
        self.filter
            .scan_chunk(self.timeline.events(), ticket, max_rows)
    }

    /// Index of the first held event the next chunk judges (review R-07).
    pub fn scan_start(&self) -> usize {
        self.filter.scan_start(self.timeline.events())
    }

    /// At most `limit` rows from `offset` (BR3.1). With a filter in force the
    /// offset counts in the result (also the rows found so far while
    /// Filtering), the total is the matched count and the rows are taken
    /// from the timeline by key; otherwise the rows are the timeline's
    /// (U3:BR4.3). Either way only the requested rows are copied.
    pub fn rows(&self, offset: usize, limit: usize) -> RowWindow {
        if !self.filter.is_active() {
            return self.timeline.rows(offset, limit);
        }
        let matched = self.filter.matched_count();
        let start = offset.min(matched);
        let end = start.saturating_add(limit).min(matched);
        let events = self.timeline.events();
        let rows = (start..end)
            .filter_map(|index| self.filter.matched_key(index))
            .filter_map(|key| {
                events
                    .binary_search_by(|held| held.sort_key().cmp(&key))
                    .ok()
                    .and_then(|position| events.get(position))
                    .cloned()
            })
            .collect();
        RowWindow {
            offset: offset as u64,
            rows,
            total_count: matched as u64,
            timeline_version: self.timeline.version(),
            filtered: true,
            all_count: self.timeline.len() as u64,
            result_version: Some(self.filter.result_version()),
        }
    }

    /// Current position of the event `(log_stream_name, sequence)` (U3:BR4.4,
    /// BR3.2). With a filter in force it is the position in the result: the
    /// timestamp comes from the timeline's index and the key is found in the
    /// result by binary search (review R-08). `None` when the event is not
    /// held, or not in the result.
    pub fn position_of(&self, log_stream_name: &str, sequence: u64) -> Option<usize> {
        if !self.filter.is_active() {
            return self.timeline.position_of(log_stream_name, sequence);
        }
        let timestamp = self.timeline.timestamp_of(log_stream_name, sequence)?;
        self.filter
            .position_of(timestamp, log_stream_name, sequence)
    }

    /// The filter summary with the result version (BR3.6).
    pub fn filter_state(&self) -> FilterState {
        self.filter.state()
    }
}

/// The fetch adds pages and discards through the same lock as the filter
/// (review R-02). A poisoned lock is recovered: every update is one step.
impl TimelineStore for Mutex<LogView> {
    fn discard(&self) -> u64 {
        self.lock().unwrap_or_else(PoisonError::into_inner).clear()
    }

    fn add(&self, events: Vec<LogEvent>) -> u64 {
        self.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .append(events)
    }
}

/// The cache write copies the held events one chunk per lock (U6:BR3.5).
impl HeldEvents for Mutex<LogView> {
    fn copy_events_in_range(
        &self,
        range: CoveredRange,
        cursor: Option<usize>,
        max: usize,
    ) -> (Vec<LogEvent>, Option<usize>) {
        let view = self.lock().unwrap_or_else(PoisonError::into_inner);
        copy_chunk_in_range(view.timeline.events(), range, cursor, max)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::coordinator::TimelineStore;
    use crate::event::LogEvent;
    use crate::filter::{FilterChange, FilterStatus, ScanStep};

    fn ev(timestamp: i64, stream: &str, sequence: u64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn keys(rows: &[LogEvent]) -> Vec<(i64, String, u64)> {
        rows.iter()
            .map(|e| (e.timestamp, e.log_stream_name.clone(), e.sequence))
            .collect()
    }

    fn key(timestamp: i64, stream: &str, sequence: u64) -> (i64, String, u64) {
        (timestamp, stream.to_string(), sequence)
    }

    /// Ten events of stream `s`; the even ones say "ERROR".
    fn ten() -> Vec<LogEvent> {
        (0..10)
            .map(|i| {
                let message = if i % 2 == 0 { "ERROR here" } else { "fine" };
                ev(i * 10, "s", i as u64, message)
            })
            .collect()
    }

    fn finish_scan(view: &mut LogView) {
        let ticket = view.filter().ticket();
        while view.scan_chunk(ticket, 3) == ScanStep::Progressed {}
    }

    #[test]
    fn adding_a_page_through_the_store_also_judges_it() {
        let store = Mutex::new(LogView::new());
        {
            let mut view = store.lock().unwrap();
            assert!(matches!(view.set_filter("error"), FilterChange::Started(_)));
            finish_scan(&mut view);
            assert_eq!(view.filter().status(), FilterStatus::Ready);
        }
        let version = store.add(ten());
        let view = store.lock().unwrap();
        assert_eq!(version, view.version());
        assert_eq!(view.timeline().len(), 10, "every event is held");
        let summary = view.filter_state().summary.unwrap();
        assert_eq!((summary.matched_count, summary.all_count), (5, 10));
        assert_eq!(
            keys(&view.rows(0, 10).rows),
            vec![
                key(0, "s", 0),
                key(20, "s", 2),
                key(40, "s", 4),
                key(60, "s", 6),
                key(80, "s", 8)
            ]
        );
    }

    #[test]
    fn a_discard_advances_the_epoch_empties_the_result_and_keeps_the_text() {
        let store = Mutex::new(LogView::new());
        store.add(ten());
        let epoch = {
            let mut view = store.lock().unwrap();
            view.set_filter("error");
            finish_scan(&mut view);
            view.filter().timeline_epoch()
        };
        let version = store.discard();
        let view = store.lock().unwrap();
        assert_eq!(version, view.version());
        assert!(view.timeline().is_empty());
        assert_eq!(view.filter().timeline_epoch(), epoch + 1);
        assert_eq!(view.filter().condition().text, "error");
        let summary = view.filter_state().summary.unwrap();
        assert_eq!((summary.matched_count, summary.all_count), (0, 0));
        assert_eq!(summary.status, FilterStatus::Ready);
        let window = view.rows(0, 10);
        assert!(window.rows.is_empty());
        assert!(window.filtered);
    }

    #[test]
    fn filtered_rows_are_positions_in_the_result() {
        let mut view = LogView::new();
        view.append(ten());
        view.set_filter("ERROR");
        finish_scan(&mut view);
        let window = view.rows(1, 2);
        assert!(window.filtered);
        assert_eq!(window.offset, 1);
        assert_eq!(keys(&window.rows), vec![key(20, "s", 2), key(40, "s", 4)]);
        assert_eq!(window.total_count, 5, "the result is the denominator");
        assert_eq!(window.all_count, 10);
        assert_eq!(window.result_version, Some(view.filter().result_version()));
        assert_eq!(window.timeline_version, view.version());
        assert!(view.rows(5, 10).rows.is_empty());
        assert_eq!(view.rows(4, 10).rows.len(), 1, "cut at the end");
    }

    #[test]
    fn without_a_filter_rows_are_the_timeline_rows() {
        let mut view = LogView::new();
        view.append(ten());
        let window = view.rows(2, 3);
        assert!(!window.filtered);
        assert_eq!(window.total_count, 10);
        assert_eq!(window.all_count, 10);
        assert_eq!(window.result_version, None);
        assert_eq!(window, view.timeline().rows(2, 3));
        view.set_filter("error");
        view.set_filter("  ");
        assert!(!view.rows(0, 1).filtered, "cleared again");
    }

    #[test]
    fn the_rows_found_so_far_are_returned_while_filtering() {
        let mut view = LogView::new();
        view.append(ten());
        let FilterChange::Started(ticket) = view.set_filter("error") else {
            panic!("started");
        };
        assert!(view.rows(0, 10).rows.is_empty(), "nothing scanned yet");
        assert_eq!(view.scan_chunk(ticket, 4), ScanStep::Progressed);
        let window = view.rows(0, 10);
        assert_eq!(keys(&window.rows), vec![key(0, "s", 0), key(20, "s", 2)]);
        assert_eq!(window.total_count, 2);
        assert_eq!(window.all_count, 10);
        assert_eq!(view.filter().status(), FilterStatus::Filtering);
    }

    #[test]
    fn the_position_while_filtering_is_inside_the_result() {
        let mut view = LogView::new();
        view.append(ten());
        assert_eq!(view.position_of("s", 4), Some(4), "no filter: timeline");
        view.set_filter("error");
        finish_scan(&mut view);
        assert_eq!(view.position_of("s", 4), Some(2));
        assert_eq!(view.position_of("s", 3), None, "held but not matched");
        assert_eq!(view.position_of("s", 99), None, "not held");
        assert_eq!(view.position_of("other", 0), None, "no such stream");
    }

    #[test]
    fn a_scan_resumes_after_its_cursor_by_binary_search() {
        let mut view = LogView::new();
        view.append(ten());
        let FilterChange::Started(ticket) = view.set_filter("error") else {
            panic!("started");
        };
        view.scan_chunk(ticket, 4);
        assert_eq!(view.filter().scan_cursor(), Some((30, "s", 3)));
        assert_eq!(view.scan_start(), 4);
        // Events arrive before and after the cursor.
        view.append(vec![
            ev(5, "t", 0, "error early"),
            ev(25, "t", 1, "an Error"),
            ev(35, "t", 2, "error late"),
        ]);
        assert_eq!(view.scan_start(), 6, "first event after (30, s, 3)");
        finish_scan(&mut view);
        let expected: Vec<_> = view
            .timeline()
            .events()
            .iter()
            .filter(|e| e.message.to_lowercase().contains("error"))
            .map(|e| (e.timestamp, e.log_stream_name.clone(), e.sequence))
            .collect();
        assert_eq!(keys(&view.rows(0, 100).rows), expected);
        assert_eq!(expected.len(), 8);
    }

    #[test]
    fn the_filter_state_follows_the_engine() {
        let mut view = LogView::new();
        assert_eq!(view.filter_state().summary, None);
        view.append(ten());
        view.set_filter("fine");
        let state = view.filter_state();
        assert_eq!(state.result_version, view.filter().result_version());
        let summary = state.summary.unwrap();
        assert_eq!(summary.status, FilterStatus::Filtering);
        assert_eq!(summary.filter_id, view.filter().filter_id());
        assert_eq!(view.clear(), view.version());
    }

    fn row_key(stream: &str, sequence: u64) -> RowKey {
        RowKey {
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    #[test]
    fn positions_of_answers_every_key_in_one_call_with_the_versions() {
        let mut view = LogView::new();
        view.append(ten());
        let keys = [row_key("s", 4), row_key("s", 99), row_key("other", 0)];
        let answer = view.positions_of(&keys);
        assert_eq!(answer.positions, vec![Some(4), None, None]);
        assert_eq!(answer.timeline_version, view.version());
        assert_eq!(answer.result_version, view.filter().result_version());
        assert_eq!(answer.discard_generation, view.discard_generation());
        assert_eq!(answer.total_count, 10);
        assert!(view.positions_of(&[]).positions.is_empty());
    }

    #[test]
    fn positions_of_with_a_filter_are_inside_the_result_and_hidden_rows_have_none() {
        let mut view = LogView::new();
        view.append(ten());
        view.set_filter("error");
        finish_scan(&mut view);
        let keys = [row_key("s", 4), row_key("s", 3), row_key("s", 8)];
        let answer = view.positions_of(&keys);
        assert_eq!(answer.positions, vec![Some(2), None, Some(4)]);
        assert_eq!(answer.total_count, 5, "the result is the list");
        assert_eq!(answer.result_version, view.filter().result_version());
        // The same key, the filter cleared: shown again at its timeline position.
        view.set_filter("");
        assert_eq!(
            view.positions_of(&keys).positions,
            vec![Some(4), Some(3), Some(8)]
        );
    }

    #[test]
    fn the_discard_generation_advances_only_when_held_logs_are_discarded() {
        let store = Mutex::new(LogView::new());
        assert_eq!(store.lock().unwrap().discard_generation(), 0);
        store.add(ten());
        assert_eq!(
            store.lock().unwrap().discard_generation(),
            0,
            "adding is not a discard"
        );
        store.discard();
        assert_eq!(store.lock().unwrap().discard_generation(), 1);
        store.discard();
        assert_eq!(
            store.lock().unwrap().discard_generation(),
            1,
            "nothing held, nothing discarded"
        );
        let mut view = store.lock().unwrap();
        view.append(ten());
        view.set_filter("error");
        view.clear();
        assert_eq!(view.discard_generation(), 2);
        assert_eq!(view.positions_of(&[row_key("s", 0)]).discard_generation, 2);
    }

    #[test]
    fn positions_serialize_with_camel_case_names_for_the_screen() {
        let mut view = LogView::new();
        view.append(ten());
        let json = serde_json::to_value(view.positions_of(&[row_key("s", 1)])).unwrap();
        assert_eq!(json["positions"], serde_json::json!([1]));
        assert!(json.get("timelineVersion").is_some());
        assert!(json.get("resultVersion").is_some());
        assert!(json.get("discardGeneration").is_some());
        assert_eq!(json["totalCount"], 10);
        let key: RowKey =
            serde_json::from_value(serde_json::json!({"logStreamName": "s", "sequence": 3}))
                .unwrap();
        assert_eq!(key, row_key("s", 3));
    }

    // ---- Speed (NFR1, NFR2): release build only ----
    //
    // `cargo test -p local-sights-core --release --lib -- filter:: log_view:: --ignored --nocapture`

    use std::time::{Duration, Instant};

    /// Rows per lock, as the app scans (src-tauri `SCAN_CHUNK_ROWS`).
    const CHUNK: usize = 4_096;

    /// `total` events over ten streams with interleaved times, added in
    /// pages of 10,000 like a fetch; about one message in a hundred says
    /// "ERROR".
    fn filled(total: usize) -> LogView {
        let streams = 10;
        let per_stream = total / streams;
        let mut view = LogView::new();
        for s in 0..streams {
            let name = format!("stream-{s:02}");
            let events: Vec<LogEvent> = (0..per_stream)
                .map(|i| {
                    let level = if i % 100 == 7 { "ERROR timeout" } else { "INFO ok" };
                    let message = format!(
                        "2024-01-02T03:04:05.{:03}Z {level} request id={i:08} path=/api/orders/{s}/{i} took {} ms",
                        i % 1_000,
                        i % 977
                    );
                    ev(((i * streams + s) / 3) as i64, &name, i as u64, &message)
                })
                .collect();
            for chunk in events.chunks(10_000) {
                view.append(chunk.to_vec());
            }
        }
        view
    }

    /// Scans the whole view like the app does and returns the time.
    fn timed_scan(view: &mut LogView, text: &str) -> Duration {
        let started = Instant::now();
        let FilterChange::Started(ticket) = view.set_filter(text) else {
            panic!("started");
        };
        while view.scan_chunk(ticket, CHUNK) == ScanStep::Progressed {}
        started.elapsed()
    }

    /// NFR1: 100,000 held events are filtered within 10 seconds.
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn one_hundred_thousand_events_are_filtered_within_ten_seconds() {
        let mut view = filled(100_000);
        let elapsed = timed_scan(&mut view, "error");
        let summary = view.filter_state().summary.unwrap();
        println!("filter of 100,000 events: {elapsed:?}");
        assert_eq!(summary.status, FilterStatus::Ready);
        assert_eq!(summary.matched_count, 1_000);
        assert!(elapsed <= Duration::from_secs(10), "took {elapsed:?}");
    }

    /// NFR2: 1,000,000 held events are filtered within 100 seconds; rows and
    /// positions of the result answer as fast as unfiltered ones (BR3.1,
    /// U3:BR4.3: 10 ms).
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn one_million_events_are_filtered_within_one_hundred_seconds() {
        let mut view = filled(1_000_000);
        let elapsed = timed_scan(&mut view, "Error");
        let summary = view.filter_state().summary.unwrap();
        println!("filter of 1,000,000 events: {elapsed:?}");
        assert_eq!(summary.status, FilterStatus::Ready);
        assert_eq!(summary.matched_count, 10_000);
        assert!(elapsed <= Duration::from_secs(100), "took {elapsed:?}");

        let limit = Duration::from_millis(10);
        let started = Instant::now();
        let window = view.rows(5_000, 100);
        let rows_time = started.elapsed();
        assert_eq!(window.rows.len(), 100);
        let started = Instant::now();
        let position = view.position_of("stream-07", 77_707);
        let position_time = started.elapsed();
        assert!(position.is_some());
        println!("filtered rows: {rows_time:?}, filtered position: {position_time:?}");
        assert!(rows_time <= limit, "rows took {rows_time:?}");
        assert!(position_time <= limit, "position took {position_time:?}");
    }

    /// BR2.1: one page of about 10,000 events added to one million held
    /// events while a filter is Ready, judged in the same step. The design
    /// gives no bound; the assertion is only a generous sanity bound (1 s),
    /// as for the unfiltered merge in timeline.rs.
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn one_page_is_added_and_judged_with_one_million_events_held() {
        let mut view = filled(1_000_000);
        timed_scan(&mut view, "error");
        let max_time = view.timeline().events().last().map_or(0, |e| e.timestamp);
        let page: Vec<LogEvent> = (0..10_000i64)
            .map(|i| {
                ev(
                    i * max_time / 10_000,
                    "stream-new",
                    i as u64,
                    "new ERROR line",
                )
            })
            .collect();
        let started = Instant::now();
        view.append(page);
        let elapsed = started.elapsed();
        println!("page of 10,000 into 1,000,000 with a filter: {elapsed:?}");
        assert_eq!(view.filter().matched_count(), 20_000);
        assert!(elapsed <= Duration::from_secs(1), "took {elapsed:?}");
    }
}
