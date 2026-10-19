//! EventFieldReader: the fields of one event (261017-insights-query U1:BR4.1,
//! U2:BR1.1-BR1.4, BR1.7, BR7.1, BR7.3).
//!
//! The four fixed fields always have a value: the time is a number of
//! epoch milliseconds, the others are the texts as fetched. `@log` is the
//! log group name the evaluation passes in, without the console's
//! "account ID:" prefix, since finding the account would need an API other
//! than the three read operations (NFR5, project.md Forbidden).
//!
//! A JSON field is read from the message when the whole message, blanks
//! around it aside, is a JSON object; a message that is no JSON, broken
//! JSON, an array or a lone string or number has no JSON field, which is no
//! error (U2:BR1.2, FR4.3). Strings, numbers and booleans are values; null,
//! objects and arrays are no value (Missing, U2:BR1.4). [`EventFields`]
//! reads the message as JSON once, the first time a JSON field is asked
//! for, and keeps it only as long as itself: one application of one command
//! to one event (functional-design review R-02).

use std::cell::OnceCell;
use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::event::LogEvent;
use crate::query::plan::FieldRef;

/// Events whose JSON field names are gathered for completion (U2:BR1.7).
pub const FIELD_NAME_SCAN_EVENTS: usize = 10_000;

/// Most JSON field names kept for completion, the first in name order
/// (U2:BR1.7).
pub const MAX_FIELD_NAMES: usize = 500;

/// The value of one field of one event (entities.md FieldValue).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldValue<'a> {
    /// No value: a JSON field the event does not have, or null, an object
    /// or an array (U2:BR1.4).
    Missing,
    /// `@timestamp` and `toMillis(@timestamp)` in epoch milliseconds (exact
    /// below 2^53), or a JSON number.
    Number(f64),
    /// `@message`, `@logStream`, `@log`, or a JSON string.
    Text(&'a str),
    /// A JSON `true` or `false`.
    Boolean(bool),
}

/// Reads fixed `field` of `event`, which belongs to `log_group_name`
/// (U1:BR4.1); a JSON field has no value here (see [`EventFields`]).
pub fn read_field<'a>(
    field: &FieldRef,
    event: &'a LogEvent,
    log_group_name: &'a str,
) -> FieldValue<'a> {
    match field {
        // Epoch milliseconds of any real time are below 2^53: exact.
        FieldRef::Timestamp | FieldRef::TimestampMillis => {
            FieldValue::Number(event.timestamp as f64)
        }
        FieldRef::Message => FieldValue::Text(&event.message),
        FieldRef::LogStream => FieldValue::Text(&event.log_stream_name),
        FieldRef::Log => FieldValue::Text(log_group_name),
        FieldRef::Json(_) => FieldValue::Missing,
    }
}

/// The message as a JSON object, when the whole message (blanks around it
/// aside) is one (U2:BR1.2). Anything that does not start with `{` is not
/// read at all.
pub fn parse_message(message: &str) -> Option<Map<String, Value>> {
    let trimmed = message.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(object)) => Some(object),
        _ => None,
    }
}

/// The JSON value at `keys` inside `object`, if there is one.
fn raw_value<'a>(object: &'a Map<String, Value>, keys: &[String]) -> Option<&'a Value> {
    let (first, rest) = keys.split_first()?;
    let mut value = object.get(first)?;
    for key in rest {
        value = value.as_object()?.get(key)?;
    }
    Some(value)
}

/// The value at `keys` inside `object` (U2:BR1.3, BR1.4): a path through
/// something else than objects, null, an object and an array are Missing.
pub fn json_value<'a>(object: &'a Map<String, Value>, keys: &[String]) -> FieldValue<'a> {
    match raw_value(object, keys) {
        Some(Value::String(text)) => FieldValue::Text(text),
        Some(Value::Number(number)) => number
            .as_f64()
            .map_or(FieldValue::Missing, FieldValue::Number),
        Some(Value::Bool(flag)) => FieldValue::Boolean(*flag),
        _ => FieldValue::Missing,
    }
}

/// The text of a JSON value as a cell shows it (U2:BR7.1), `None` for no
/// value. A boolean is shown `1` or `0` as the console shows it
/// (261017-insights-query Loop-back 2, p04: `1` checked, `0` provisional
/// until the next console check); comparisons, `like` and `=~` still see the
/// text `true` or `false`.
fn written(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// Whether `key` can be written without backquotes and read back as the
/// same key: a letter or `_` first, then letters, digits, `_` and `-`
/// (the parser's bare part of a name, U2:BR1.1).
fn is_plain_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// A JSON key path as the screen writes it: keys joined by `.`, a key in
/// backquotes when it is not a plain name, so that the name read back is the
/// same field (U2:BR1.3, functional-design review R-09).
pub fn display_name(keys: &[&str]) -> String {
    keys.iter()
        .map(|key| {
            if is_plain_key(key) {
                (*key).to_string()
            } else {
                format!("`{key}`")
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Calls `found` with the key path and value of every leaf of `object`
/// (strings, numbers, booleans and null; arrays are left out, U2:BR1.4), in
/// key order within each object.
fn walk_leaves<'a>(
    object: &'a Map<String, Value>,
    path: &mut Vec<&'a str>,
    found: &mut impl FnMut(&[&'a str], &'a Value),
) {
    for (key, value) in object {
        path.push(key);
        match value {
            Value::Object(inner) => walk_leaves(inner, path, found),
            Value::Array(_) => {}
            leaf => found(path, leaf),
        }
        path.pop();
    }
}

/// The fields of one event, its message read as JSON at most once.
#[derive(Debug)]
pub struct EventFields<'a> {
    event: &'a LogEvent,
    log_group_name: &'a str,
    parsed: OnceCell<Option<Map<String, Value>>>,
}

impl<'a> EventFields<'a> {
    /// The fields of `event` of `log_group_name`; nothing is read yet.
    pub fn new(event: &'a LogEvent, log_group_name: &'a str) -> Self {
        Self {
            event,
            log_group_name,
            parsed: OnceCell::new(),
        }
    }

    /// The event.
    pub fn event(&self) -> &'a LogEvent {
        self.event
    }

    /// The log group, the value of `@log`.
    pub fn log_group_name(&self) -> &'a str {
        self.log_group_name
    }

    /// Whether the message was read as JSON already.
    pub fn is_parsed(&self) -> bool {
        self.parsed.get().is_some()
    }

    /// The message as a JSON object, read the first time it is asked for.
    fn object(&self) -> Option<&Map<String, Value>> {
        self.parsed
            .get_or_init(|| parse_message(&self.event.message))
            .as_ref()
    }

    /// The value of `field` (U1:BR4.1, U2:BR1.2-BR1.4).
    pub fn value(&self, field: &FieldRef) -> FieldValue<'_> {
        match field {
            FieldRef::Json(path) => self.object().map_or(FieldValue::Missing, |object| {
                json_value(object, path.keys())
            }),
            fixed => read_field(fixed, self.event, self.log_group_name),
        }
    }

    /// The text of `field` as a result cell shows it (U2:BR7.1), `None`
    /// without a value; the time is not written here (its zone is the
    /// screen's). A JSON number is written as serde_json writes it: whole
    /// numbers exactly, decimals in their shortest form.
    pub fn display(&self, field: &FieldRef) -> Option<String> {
        match field {
            FieldRef::Json(path) => written(
                self.object()
                    .and_then(|object| raw_value(object, path.keys())),
            ),
            FieldRef::Timestamp | FieldRef::TimestampMillis => {
                Some(self.event.timestamp.to_string())
            }
            FieldRef::Message => Some(self.event.message.clone()),
            FieldRef::LogStream => Some(self.event.log_stream_name.clone()),
            FieldRef::Log => Some(self.log_group_name.to_string()),
        }
    }

    /// Every JSON field with a value or null, nested ones as `a.b`, in name
    /// order, written as [`display_name`] writes them (U2:BR7.3); arrays are
    /// left out (U2:BR1.4).
    pub fn json_fields(&self) -> Vec<(String, Option<String>)> {
        let Some(object) = self.object() else {
            return Vec::new();
        };
        let mut fields = Vec::new();
        walk_leaves(object, &mut Vec::new(), &mut |path, value| {
            fields.push((display_name(path), written(Some(value))));
        });
        fields.sort_by(|left, right| left.0.cmp(&right.0));
        fields
    }
}

/// The JSON field names of `messages`, nested ones as `a.b`, written as
/// [`display_name`] writes them, the first [`MAX_FIELD_NAMES`] in name order
/// (U2:BR1.7). Only the names are kept, never the values. The caller passes
/// at most [`FIELD_NAME_SCAN_EVENTS`] messages.
pub fn collect_json_field_names<'m>(messages: impl IntoIterator<Item = &'m str>) -> Vec<String> {
    let mut names = BTreeSet::new();
    for message in messages {
        let Some(object) = parse_message(message) else {
            continue;
        };
        walk_leaves(&object, &mut Vec::new(), &mut |path, _| {
            names.insert(display_name(path));
            if names.len() > MAX_FIELD_NAMES {
                names.pop_last();
            }
        });
    }
    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::plan::JsonPath;

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

    /// The JSON field written `text` with these keys.
    fn json(keys: &[&str]) -> FieldRef {
        FieldRef::Json(JsonPath::new(
            keys.iter().map(|key| (*key).to_string()).collect(),
            keys.join("."),
        ))
    }

    #[test]
    fn the_timestamp_is_the_event_time_in_milliseconds() {
        let event = event("hello");
        assert_eq!(
            read_field(&FieldRef::Timestamp, &event, GROUP),
            FieldValue::Number(1_756_053_806_123.0)
        );
    }

    #[test]
    fn to_millis_reads_the_same_number_as_the_timestamp() {
        let event = event("hello");
        assert_eq!(
            read_field(&FieldRef::TimestampMillis, &event, GROUP),
            read_field(&FieldRef::Timestamp, &event, GROUP)
        );
    }

    #[test]
    fn the_message_is_the_whole_text_with_its_line_breaks() {
        let event = event("ERROR first line\n  at second line\r\n");
        assert_eq!(
            read_field(&FieldRef::Message, &event, GROUP),
            FieldValue::Text("ERROR first line\n  at second line\r\n")
        );
    }

    #[test]
    fn the_log_stream_is_the_stream_name() {
        let event = event("hello");
        assert_eq!(
            read_field(&FieldRef::LogStream, &event, GROUP),
            FieldValue::Text("2025/08/24/[$LATEST]abc")
        );
    }

    #[test]
    fn the_log_is_the_group_name_only_without_an_account() {
        let event = event("hello");
        assert_eq!(
            read_field(&FieldRef::Log, &event, GROUP),
            FieldValue::Text("/aws/lambda/orders")
        );
        assert_eq!(
            read_field(&FieldRef::Log, &event, "other"),
            FieldValue::Text("other"),
            "the name passed in, whatever the event"
        );
    }

    // ---- 261017-insights-query U2:BR1.2-BR1.4, BR1.7, BR7.1, BR7.3 -------

    #[test]
    fn the_acceptance_message_has_its_string_and_nested_number() {
        // Acceptance criterion of FR4.2.
        let event = event(r#"{"level":"error","http":{"status":500}}"#);
        let fields = EventFields::new(&event, GROUP);
        assert!(
            !fields.is_parsed(),
            "nothing read before a JSON field is asked"
        );
        assert_eq!(fields.value(&FieldRef::Log), FieldValue::Text(GROUP));
        assert!(!fields.is_parsed(), "fixed fields need no JSON");
        assert_eq!(fields.value(&json(&["level"])), FieldValue::Text("error"));
        assert!(fields.is_parsed());
        assert_eq!(
            fields.value(&json(&["http", "status"])),
            FieldValue::Number(500.0)
        );
        assert_eq!(
            fields.value(&json(&["http"])),
            FieldValue::Missing,
            "an object"
        );
        assert_eq!(fields.value(&json(&["status"])), FieldValue::Missing);
        assert_eq!(
            fields.value(&json(&["level", "x"])),
            FieldValue::Missing,
            "inside a string"
        );
    }

    #[test]
    fn messages_that_are_no_json_object_have_no_json_field() {
        // U2:BR1.2, FR4.3: not an error, and the fixed fields stay.
        for message in [
            "ERROR not json",
            r#"{"level":"error""#,
            r#"[{"level":"error"}]"#,
            r#""level""#,
            "42",
            "",
            r#"{"level":"error"} trailing"#,
        ] {
            let event = event(message);
            let fields = EventFields::new(&event, GROUP);
            assert_eq!(
                fields.value(&json(&["level"])),
                FieldValue::Missing,
                "{message:?}"
            );
            assert_eq!(fields.value(&FieldRef::Message), FieldValue::Text(message));
            assert!(fields.json_fields().is_empty(), "{message:?}");
        }
        let padded = event("  \n{\"level\":\"warn\"}\r\n");
        assert_eq!(
            EventFields::new(&padded, GROUP).value(&json(&["level"])),
            FieldValue::Text("warn"),
            "blanks around the object are ignored"
        );
    }

    #[test]
    fn null_objects_and_arrays_are_no_value_and_booleans_are_values() {
        // U2:BR1.4.
        let event = event(r#"{"a":null,"b":{"c":1},"d":[1,2],"e":true,"f":false,"g":-1.5}"#);
        let fields = EventFields::new(&event, GROUP);
        for keys in [&["a"][..], &["b"], &["d"], &["a", "x"]] {
            assert_eq!(fields.value(&json(keys)), FieldValue::Missing, "{keys:?}");
        }
        assert_eq!(fields.value(&json(&["e"])), FieldValue::Boolean(true));
        assert_eq!(fields.value(&json(&["f"])), FieldValue::Boolean(false));
        assert_eq!(fields.value(&json(&["g"])), FieldValue::Number(-1.5));
        assert_eq!(fields.value(&json(&["b", "c"])), FieldValue::Number(1.0));
    }

    #[test]
    fn a_key_with_a_dot_is_another_field_than_the_nested_one() {
        // U2:BR1.3: `user.id` in backquotes is the key itself.
        let event = event(r#"{"user.id":"u1","user":{"id":"u2"}}"#);
        let fields = EventFields::new(&event, GROUP);
        assert_eq!(fields.value(&json(&["user.id"])), FieldValue::Text("u1"));
        assert_eq!(fields.value(&json(&["user", "id"])), FieldValue::Text("u2"));
        assert_eq!(
            fields.json_fields(),
            vec![
                ("`user.id`".to_string(), Some("u1".to_string())),
                ("user.id".to_string(), Some("u2".to_string())),
            ],
            "R-09: the two are told apart by the backquotes"
        );
    }

    #[test]
    fn names_are_backquoted_only_when_they_are_no_plain_name() {
        // R-09, U2:BR1.3: what the parser reads back as the same keys.
        assert_eq!(display_name(&["http", "status"]), "http.status");
        assert_eq!(display_name(&["user-agent", "_x9"]), "user-agent._x9");
        assert_eq!(display_name(&["user.id"]), "`user.id`");
        assert_eq!(
            display_name(&["a b", "0", "@t", "-x", ""]),
            "`a b`.`0`.`@t`.`-x`.``"
        );
        assert_eq!(display_name(&["ログ", "種類"]), "ログ.種類");
    }

    #[test]
    fn cells_are_written_as_the_values_are() {
        // U2:BR7.1: strings as they are, numbers as written (whole numbers
        // exactly), nothing without a value. Booleans: see the p04 tests.
        let event =
            event(r#"{"s":"a\nb","n":500,"big":1234567890123456789,"d":1.5,"t":true,"z":null}"#);
        let fields = EventFields::new(&event, GROUP);
        let shown = |keys: &[&str]| fields.display(&json(keys));
        assert_eq!(shown(&["s"]).as_deref(), Some("a\nb"));
        assert_eq!(shown(&["n"]).as_deref(), Some("500"));
        assert_eq!(shown(&["big"]).as_deref(), Some("1234567890123456789"));
        assert_eq!(shown(&["d"]).as_deref(), Some("1.5"));
        assert_eq!(shown(&["t"]).as_deref(), Some("1"));
        assert_eq!(shown(&["z"]), None);
        assert_eq!(shown(&["nothing"]), None);
        assert_eq!(
            fields.display(&FieldRef::LogStream).as_deref(),
            Some("2025/08/24/[$LATEST]abc")
        );
        assert_eq!(
            fields.display(&FieldRef::TimestampMillis).as_deref(),
            Some("1756053806123")
        );
    }

    #[test]
    fn the_fields_of_a_row_are_listed_in_name_order_without_arrays() {
        // U2:BR7.3.
        let event = event(r#"{"z":1,"a":{"c":"x","b":null},"arr":[1],"m":{"n":{"o":false}}}"#);
        assert_eq!(
            EventFields::new(&event, GROUP).json_fields(),
            vec![
                ("a.b".to_string(), None),
                ("a.c".to_string(), Some("x".to_string())),
                ("m.n.o".to_string(), Some("0".to_string())),
                ("z".to_string(), Some("1".to_string())),
            ]
        );
    }

    #[test]
    fn p04_a_true_boolean_is_shown_as_1_in_cells_and_opened_rows() {
        // Checked at the console in Loop-back 2: readOnly true is shown 1.
        let event = event(r#"{"readOnly":true,"flag":"true"}"#);
        let fields = EventFields::new(&event, GROUP);
        assert_eq!(fields.display(&json(&["readOnly"])).as_deref(), Some("1"));
        assert_eq!(
            fields.display(&json(&["flag"])).as_deref(),
            Some("true"),
            "a string true stays as it is"
        );
        assert_eq!(
            fields.json_fields(),
            vec![
                ("flag".to_string(), Some("true".to_string())),
                ("readOnly".to_string(), Some("1".to_string())),
            ]
        );
        // The value compared is still the boolean (p04 at the console).
        assert_eq!(
            fields.value(&json(&["readOnly"])),
            FieldValue::Boolean(true)
        );
    }

    #[test]
    fn provisional_p04_a_false_boolean_is_shown_as_0() {
        // Not checked at the console yet (Loop-back 2): the next
        // compatibility check confirms it.
        let event = event(r#"{"readOnly":false,"m":{"n":false}}"#);
        let fields = EventFields::new(&event, GROUP);
        assert_eq!(fields.display(&json(&["readOnly"])).as_deref(), Some("0"));
        assert_eq!(fields.display(&json(&["m", "n"])).as_deref(), Some("0"));
        assert_eq!(fields.json_fields()[0].1.as_deref(), Some("0"));
    }

    #[test]
    fn field_names_are_gathered_in_name_order_up_to_500() {
        // U2:BR1.7: names only, nested as a.b, arrays left out.
        let names = collect_json_field_names([
            r#"{"level":"error","http":{"status":500,"path":"/a"}}"#,
            "not json",
            r#"{"level":"info","user.id":"u","tags":["x"],"gone":null}"#,
        ]);
        assert_eq!(
            names,
            vec!["`user.id`", "gone", "http.path", "http.status", "level"]
        );
        let many: Vec<String> = (0..700).map(|i| format!(r#"{{"k{i:03}":1}}"#)).collect();
        let names = collect_json_field_names(many.iter().map(String::as_str));
        assert_eq!(names.len(), MAX_FIELD_NAMES);
        assert_eq!(names.first().map(String::as_str), Some("k000"));
        assert_eq!(names.last().map(String::as_str), Some("k499"));
        assert!(collect_json_field_names(["[1]", "plain"]).is_empty());
    }
}
