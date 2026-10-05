//! EventTimeline: every event of the current fetch, always in
//! `(timestamp, logStreamName, sequence)` order (U3:BR4.1, BR4.2).
//!
//! Pages are merged in as they arrive. The screen never holds the events: it
//! asks for the rows of its viewport by offset and limit (BR4.3) and for the
//! position of one event by `(logStreamName, sequence)` (BR4.4). Every
//! addition and every discard increments `timelineVersion` (BR4.5).

use std::collections::HashMap;

use serde::Serialize;

use crate::event::{LogEvent, compare_events};

/// Index entry for a sequence number that was never added.
const MISSING: i64 = i64::MIN;

/// The rows of one viewport request (entities.md RowWindow).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowWindow {
    /// Position of the first row.
    pub offset: u64,
    /// At most `limit` events from `offset`, in order.
    pub rows: Vec<LogEvent>,
    /// Number of held events when the rows were taken.
    pub total_count: u64,
    /// Version of the timeline when the rows were taken.
    pub timeline_version: u64,
}

/// In-memory, ordered list of every event of the current fetch.
#[derive(Debug, Clone, Default)]
pub struct EventTimeline {
    events: Vec<LogEvent>,
    /// Per stream, the timestamp of each sequence number (review R-10).
    timestamps: HashMap<String, Vec<i64>>,
    version: u64,
}

impl EventTimeline {
    /// An empty timeline at version 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges one page into place and returns the new version (BR4.2).
    ///
    /// The page is sorted by the key, then merged with the held events from
    /// the first position it reaches; the events before that position are
    /// not touched. An empty page changes nothing.
    pub fn append(&mut self, mut batch: Vec<LogEvent>) -> u64 {
        if batch.is_empty() {
            return self.version;
        }
        batch.sort_by(compare_events);
        for event in &batch {
            self.index(event);
        }
        let start = self
            .events
            .partition_point(|held| compare_events(held, &batch[0]).is_lt());
        let tail = self.events.split_off(start);
        self.events.reserve(tail.len() + batch.len());
        let mut tail = tail.into_iter().peekable();
        let mut batch = batch.into_iter().peekable();
        loop {
            let take_tail = match (tail.peek(), batch.peek()) {
                (Some(held), Some(new)) => compare_events(held, new).is_le(),
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };
            let next = if take_tail { tail.next() } else { batch.next() };
            self.events.extend(next);
        }
        self.version += 1;
        self.version
    }

    /// Every held event, in order.
    pub fn events(&self) -> &[LogEvent] {
        &self.events
    }

    /// Number of held events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no event is held.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Current version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Discards every event and returns the new version (BR4.5).
    pub fn clear(&mut self) -> u64 {
        self.events.clear();
        self.timestamps.clear();
        self.version += 1;
        self.version
    }

    /// At most `limit` events from `offset` (BR4.3). Copies only the
    /// requested rows, so the cost does not depend on the held count.
    pub fn rows(&self, offset: usize, limit: usize) -> RowWindow {
        let start = offset.min(self.events.len());
        let end = start.saturating_add(limit).min(self.events.len());
        RowWindow {
            offset: offset as u64,
            rows: self.events[start..end].to_vec(),
            total_count: self.events.len() as u64,
            timeline_version: self.version,
        }
    }

    /// Current position of the event `(log_stream_name, sequence)` (BR4.4):
    /// its timestamp comes from the per-stream index, then the full key is
    /// found by binary search, without scanning the events.
    pub fn position_of(&self, log_stream_name: &str, sequence: u64) -> Option<usize> {
        let index = usize::try_from(sequence).ok()?;
        let timestamp = *self.timestamps.get(log_stream_name)?.get(index)?;
        if timestamp == MISSING {
            return None;
        }
        let key = (timestamp, log_stream_name, sequence);
        self.events
            .binary_search_by(|held| held.sort_key().cmp(&key))
            .ok()
    }

    /// Records the timestamp of one event under its stream and sequence.
    /// Sequences are normally contiguous from 0; a gap is filled with a
    /// marker that never matches.
    fn index(&mut self, event: &LogEvent) {
        let Ok(index) = usize::try_from(event.sequence) else {
            return;
        };
        let timestamps = self
            .timestamps
            .entry(event.log_stream_name.clone())
            .or_default();
        if timestamps.len() <= index {
            timestamps.resize(index + 1, MISSING);
        }
        timestamps[index] = event.timestamp;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn event(timestamp: i64, stream: &str, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: Some(timestamp + 1),
            message: format!("{stream}:{sequence}\nsecond line"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    /// One page of `stream`, numbering from `first_sequence`.
    fn page(stream: &str, first_sequence: u64, timestamps: &[i64]) -> Vec<LogEvent> {
        timestamps
            .iter()
            .zip(first_sequence..)
            .map(|(timestamp, sequence)| event(*timestamp, stream, sequence))
            .collect()
    }

    fn keys(timeline: &EventTimeline) -> Vec<(i64, String, u64)> {
        timeline
            .events()
            .iter()
            .map(|e| (e.timestamp, e.log_stream_name.clone(), e.sequence))
            .collect()
    }

    fn key(timestamp: i64, stream: &str, sequence: u64) -> (i64, String, u64) {
        (timestamp, stream.to_string(), sequence)
    }

    fn is_sorted(timeline: &EventTimeline) -> bool {
        timeline.events().windows(2).all(|pair| {
            (
                pair[0].timestamp,
                &pair[0].log_stream_name,
                pair[0].sequence,
            ) <= (
                pair[1].timestamp,
                &pair[1].log_stream_name,
                pair[1].sequence,
            )
        })
    }

    #[test]
    fn starts_empty_at_version_zero() {
        let timeline = EventTimeline::new();
        assert!(timeline.is_empty());
        assert_eq!(timeline.version(), 0);
        assert_eq!(timeline.position_of("a", 0), None);
    }

    #[test]
    fn pages_of_several_streams_are_merged_by_time_then_stream_then_sequence() {
        let mut timeline = EventTimeline::new();
        timeline.append(page("b", 0, &[10, 20, 30]));
        assert!(is_sorted(&timeline));
        timeline.append(page("a", 0, &[5, 20, 40]));
        assert!(is_sorted(&timeline));
        timeline.append(page("B", 0, &[20]));
        timeline.append(page("b", 3, &[30]));
        assert_eq!(
            keys(&timeline),
            vec![
                key(5, "a", 0),
                key(10, "b", 0),
                key(20, "B", 0),
                key(20, "a", 1),
                key(20, "b", 1),
                key(30, "b", 2),
                key(30, "b", 3),
                key(40, "a", 2),
            ]
        );
    }

    #[test]
    fn a_page_out_of_order_within_itself_is_still_placed_correctly() {
        let mut timeline = EventTimeline::new();
        timeline.append(page("s", 0, &[30, 10, 20]));
        assert_eq!(
            keys(&timeline),
            vec![key(10, "s", 1), key(20, "s", 2), key(30, "s", 0)]
        );
        assert_eq!(timeline.position_of("s", 0), Some(2));
    }

    #[test]
    fn rows_are_taken_by_offset_and_limit_with_count_and_version() {
        let mut timeline = EventTimeline::new();
        let version = timeline.append(page("s", 0, &[1, 2, 3, 4, 5]));
        let window = timeline.rows(1, 3);
        assert_eq!(window.offset, 1);
        assert_eq!(
            window.rows.iter().map(|e| e.sequence).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(window.total_count, 5);
        assert_eq!(window.timeline_version, version);
        assert_eq!(timeline.rows(3, 10).rows.len(), 2, "cut at the end");
        assert!(timeline.rows(5, 10).rows.is_empty());
        assert!(timeline.rows(99, 10).rows.is_empty());
        assert!(timeline.rows(0, 0).rows.is_empty());
        assert_eq!(window.rows[0].message, "s:1\nsecond line", "kept unchanged");
    }

    #[test]
    fn the_position_of_an_event_follows_later_insertions() {
        let mut timeline = EventTimeline::new();
        timeline.append(page("b", 0, &[10, 20, 30]));
        assert_eq!(timeline.position_of("b", 1), Some(1));
        timeline.append(page("a", 0, &[1, 2, 15]));
        assert_eq!(timeline.position_of("b", 1), Some(4));
        assert_eq!(timeline.position_of("a", 2), Some(3));
        assert_eq!(timeline.position_of("b", 3), None, "no such sequence");
        assert_eq!(timeline.position_of("c", 0), None, "no such stream");
    }

    #[test]
    fn every_addition_and_every_discard_increments_the_version() {
        let mut timeline = EventTimeline::new();
        let first = timeline.append(page("s", 0, &[1]));
        assert_eq!(first, 1);
        let second = timeline.append(page("s", 1, &[2]));
        assert_eq!(second, 2);
        assert_eq!(timeline.append(Vec::new()), 2, "an empty page adds nothing");
        let cleared = timeline.clear();
        assert_eq!(cleared, 3);
        assert!(timeline.is_empty());
        assert_eq!(timeline.position_of("s", 0), None);
        assert_eq!(timeline.clear(), 4, "a discard always counts");
        assert_eq!(timeline.version(), 4);
    }

    /// Ten streams of interleaved times, each read in pages, like a fetch.
    fn filled(total: usize, streams: usize, page_size: usize) -> EventTimeline {
        let per_stream = total / streams;
        let mut timeline = EventTimeline::new();
        for s in 0..streams {
            let name = format!("stream-{s:02}");
            let times: Vec<i64> = (0..per_stream)
                .map(|i| (i * streams + s) as i64 / 3)
                .collect();
            for (chunk, first) in times.chunks(page_size).zip((0u64..).step_by(page_size)) {
                timeline.append(page(&name, first, chunk));
            }
        }
        timeline
    }

    #[test]
    fn many_events_stay_ordered_and_findable() {
        let timeline = filled(20_000, 10, 1_000);
        assert_eq!(timeline.len(), 20_000);
        assert!(is_sorted(&timeline));
        for (s, sequence) in [(0, 0u64), (3, 999), (9, 1_999)] {
            let name = format!("stream-{s:02}");
            let position = timeline.position_of(&name, sequence).unwrap();
            let found = &timeline.events()[position];
            assert_eq!(
                (found.log_stream_name.as_str(), found.sequence),
                (name.as_str(), sequence)
            );
        }
        assert_eq!(timeline.rows(19_990, 50).rows.len(), 10);
    }

    /// NFR2 / BR4.4 with one million events. Run in a release build:
    /// `cargo test -p local-sights-core --release --lib -- timeline:: --ignored`
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn one_million_events_answer_rows_and_positions_within_ten_milliseconds() {
        let timeline = filled(1_000_000, 10, 10_000);
        assert_eq!(timeline.len(), 1_000_000);
        let limit = Duration::from_millis(10);

        let started = Instant::now();
        let window = timeline.rows(654_321, 100);
        let rows_time = started.elapsed();
        assert_eq!(window.rows.len(), 100);

        let started = Instant::now();
        let position = timeline.position_of("stream-07", 77_777);
        let position_time = started.elapsed();
        assert!(position.is_some());

        println!("rows: {rows_time:?}, position: {position_time:?}");
        assert!(rows_time <= limit, "rows took {rows_time:?}");
        assert!(position_time <= limit, "position took {position_time:?}");
    }
}
