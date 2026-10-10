//! LogEvent and its ordering key (U3:BR4.1, replacing U1:BR5.1).

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
    /// Order in which the API returned the event within its stream, from
    /// 0 without gaps (U3:BR3.2). With the stream name it identifies the
    /// event until the next fetch (U3:BR4.4).
    pub sequence: u64,
}

impl LogEvent {
    /// Ordering key `(timestamp, logStreamName, sequence)`; stream names
    /// compare as plain strings.
    pub fn sort_key(&self) -> (i64, &str, u64) {
        (self.timestamp, self.log_stream_name.as_str(), self.sequence)
    }
}

/// Compares two events by `(timestamp, logStreamName, sequence)`.
pub fn compare_events(a: &LogEvent, b: &LogEvent) -> Ordering {
    a.sort_key().cmp(&b.sort_key())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(timestamp: i64, stream: &str, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: format!("{stream}:{sequence}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    #[test]
    fn earlier_timestamp_comes_first_regardless_of_stream_and_sequence() {
        assert_eq!(
            compare_events(&event(5, "z", 9), &event(6, "a", 0)),
            Ordering::Less
        );
        assert_eq!(
            compare_events(&event(6, "a", 0), &event(5, "z", 9)),
            Ordering::Greater
        );
    }

    #[test]
    fn same_timestamp_is_ordered_by_stream_name_as_a_string() {
        // U3:BR4.1 replaces U1's (timestamp, sequence): the stream name
        // decides before the sequence, compared as plain strings.
        assert_eq!(
            compare_events(&event(5, "b", 0), &event(5, "a", 9)),
            Ordering::Greater
        );
        assert_eq!(
            compare_events(&event(5, "B", 9), &event(5, "a", 0)),
            Ordering::Less,
            "upper case sorts before lower case"
        );
    }

    #[test]
    fn same_timestamp_and_stream_is_ordered_by_sequence() {
        assert_eq!(
            compare_events(&event(5, "a", 1), &event(5, "a", 2)),
            Ordering::Less
        );
        assert_eq!(
            compare_events(&event(5, "a", 2), &event(5, "a", 2)),
            Ordering::Equal
        );
    }
}
