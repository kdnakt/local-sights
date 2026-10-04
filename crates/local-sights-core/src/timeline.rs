//! EventTimeline (U1 minimal version): append in fetch order, read all,
//! clear. It owns the fetched LogEvents (BR4.5) and never truncates by count
//! (BR3.3).

use crate::event::{LogEvent, compare_events};

/// In-memory list of every event of the current fetch.
#[derive(Debug, Clone, Default)]
pub struct EventTimeline {
    events: Vec<LogEvent>,
}

impl EventTimeline {
    /// An empty timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one page of events, keeping `(timestamp, sequence)` order.
    ///
    /// In U1 the API returns one stream oldest-first, so the order normally
    /// already holds; a stable sort keeps it correct when it does not.
    pub fn append(&mut self, batch: Vec<LogEvent>) {
        let already_ordered = match (self.events.last(), batch.first()) {
            (Some(last), Some(first)) => compare_events(last, first).is_le(),
            _ => true,
        } && batch
            .windows(2)
            .all(|pair| compare_events(&pair[0], &pair[1]).is_le());
        self.events.extend(batch);
        if !already_ordered {
            self.events.sort_by(compare_events);
        }
    }

    /// Every held event in `(timestamp, sequence)` order (BR5.1).
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

    /// Discards every held event (start of a new fetch).
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(timestamp: i64, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: Some(timestamp + 1),
            message: format!("line {sequence}\nsecond line"),
            log_stream_name: "s".to_string(),
            sequence,
        }
    }

    fn keys(timeline: &EventTimeline) -> Vec<(i64, u64)> {
        timeline.events().iter().map(LogEvent::sort_key).collect()
    }

    #[test]
    fn starts_empty() {
        let timeline = EventTimeline::new();
        assert!(timeline.is_empty());
        assert_eq!(timeline.len(), 0);
    }

    #[test]
    fn appended_pages_are_read_back_in_key_order() {
        let mut timeline = EventTimeline::new();
        timeline.append(vec![event(10, 0), event(20, 1)]);
        timeline.append(vec![event(20, 2), event(30, 3)]);
        assert_eq!(keys(&timeline), vec![(10, 0), (20, 1), (20, 2), (30, 3)]);
    }

    #[test]
    fn out_of_order_input_is_sorted_by_timestamp_then_sequence() {
        let mut timeline = EventTimeline::new();
        timeline.append(vec![event(30, 0), event(10, 2), event(10, 1)]);
        assert_eq!(keys(&timeline), vec![(10, 1), (10, 2), (30, 0)]);
    }

    #[test]
    fn messages_are_kept_unchanged() {
        let mut timeline = EventTimeline::new();
        timeline.append(vec![event(1, 0)]);
        assert_eq!(timeline.events()[0].message, "line 0\nsecond line");
    }

    #[test]
    fn clear_discards_everything() {
        let mut timeline = EventTimeline::new();
        timeline.append(vec![event(1, 0)]);
        timeline.clear();
        assert!(timeline.is_empty());
    }

    #[test]
    fn large_volumes_are_not_truncated() {
        let mut timeline = EventTimeline::new();
        for page in 0..30u64 {
            timeline.append(
                (0..10_000u64)
                    .map(|i| event(0, page * 10_000 + i))
                    .collect(),
            );
        }
        assert_eq!(timeline.len(), 300_000);
    }
}
