//! LogEvent and its ordering key (BR5.1).

use std::cmp::Ordering;

use serde::Serialize;

/// One fetched log event, owned by the EventTimeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    /// Event time, epoch milliseconds.
    pub timestamp: i64,
    /// Ingestion time, epoch milliseconds, when the API returned it.
    pub ingestion_time: Option<i64>,
    /// The message exactly as returned (may contain line breaks).
    pub message: String,
    /// Stream the event came from.
    pub log_stream_name: String,
    /// Order in which the API returned the event within one fetch, from 0.
    pub sequence: u64,
}

impl LogEvent {
    /// Ordering key `(timestamp, sequence)`.
    pub fn sort_key(&self) -> (i64, u64) {
        (self.timestamp, self.sequence)
    }
}

/// Compares two events by `(timestamp, sequence)`.
pub fn compare_events(a: &LogEvent, b: &LogEvent) -> Ordering {
    a.sort_key().cmp(&b.sort_key())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(timestamp: i64, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: format!("m{sequence}"),
            log_stream_name: "s".to_string(),
            sequence,
        }
    }

    #[test]
    fn sort_key_is_timestamp_then_sequence() {
        assert_eq!(event(10, 3).sort_key(), (10, 3));
    }

    #[test]
    fn earlier_timestamp_comes_first_regardless_of_sequence() {
        assert_eq!(compare_events(&event(5, 9), &event(6, 0)), Ordering::Less);
        assert_eq!(
            compare_events(&event(6, 0), &event(5, 9)),
            Ordering::Greater
        );
    }

    #[test]
    fn same_timestamp_is_ordered_by_sequence() {
        assert_eq!(compare_events(&event(5, 1), &event(5, 2)), Ordering::Less);
        assert_eq!(compare_events(&event(5, 2), &event(5, 2)), Ordering::Equal);
    }
}
