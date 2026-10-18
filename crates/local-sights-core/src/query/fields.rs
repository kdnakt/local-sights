//! EventFieldReader, the fixed fields (261017-insights-query U1:BR4.1).
//!
//! The four fixed fields always have a value: the time is a number of
//! epoch milliseconds, the others are the texts as fetched. `@log` is the
//! log group name the evaluation passes in, without the console's
//! "account ID:" prefix, since finding the account would need an API other
//! than the three read operations (NFR5, project.md Forbidden).

use crate::event::LogEvent;
use crate::query::plan::FieldRef;

/// The value of one field of one event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldValue<'a> {
    /// `@timestamp` and `toMillis(@timestamp)`: epoch milliseconds.
    Number(i64),
    /// `@message`, `@logStream` and `@log`.
    Text(&'a str),
}

/// Reads `field` of `event`, which belongs to `log_group_name` (U1:BR4.1).
pub fn read_field<'a>(
    field: FieldRef,
    event: &'a LogEvent,
    log_group_name: &'a str,
) -> FieldValue<'a> {
    match field {
        FieldRef::Timestamp | FieldRef::TimestampMillis => FieldValue::Number(event.timestamp),
        FieldRef::Message => FieldValue::Text(&event.message),
        FieldRef::LogStream => FieldValue::Text(&event.log_stream_name),
        FieldRef::Log => FieldValue::Text(log_group_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(message: &str) -> LogEvent {
        LogEvent {
            timestamp: 1_756_053_806_123,
            ingestion_time: Some(1_756_053_807_000),
            message: message.to_string(),
            log_stream_name: "2025/08/24/[$LATEST]abc".to_string(),
            sequence: 7,
        }
    }

    const GROUP: &str = "/aws/lambda/orders";

    #[test]
    fn the_timestamp_is_the_event_time_in_milliseconds() {
        let event = event("hello");
        assert_eq!(
            read_field(FieldRef::Timestamp, &event, GROUP),
            FieldValue::Number(1_756_053_806_123)
        );
    }

    #[test]
    fn to_millis_reads_the_same_number_as_the_timestamp() {
        let event = event("hello");
        assert_eq!(
            read_field(FieldRef::TimestampMillis, &event, GROUP),
            read_field(FieldRef::Timestamp, &event, GROUP)
        );
    }

    #[test]
    fn the_message_is_the_whole_text_with_its_line_breaks() {
        let event = event("ERROR first line\n  at second line\r\n");
        assert_eq!(
            read_field(FieldRef::Message, &event, GROUP),
            FieldValue::Text("ERROR first line\n  at second line\r\n")
        );
    }

    #[test]
    fn the_log_stream_is_the_stream_name() {
        let event = event("hello");
        assert_eq!(
            read_field(FieldRef::LogStream, &event, GROUP),
            FieldValue::Text("2025/08/24/[$LATEST]abc")
        );
    }

    #[test]
    fn the_log_is_the_group_name_only_without_an_account() {
        let event = event("hello");
        assert_eq!(
            read_field(FieldRef::Log, &event, GROUP),
            FieldValue::Text("/aws/lambda/orders")
        );
        assert_eq!(
            read_field(FieldRef::Log, &event, "other"),
            FieldValue::Text("other"),
            "the name passed in, whatever the event"
        );
    }
}
