//! The shape of a read query (261017-insights-query U1 and U2, entities.md
//! QueryPlan, Command, SortKey, Condition, FieldRef, Literal, Span,
//! QueryProblem).
//!
//! A [`QueryPlan`] is the list of the query's commands (`fields`, `filter`,
//! `sort`, `limit`) in the order they were written; they are applied in that
//! order (261017-insights-query U2:BR3.1). Every part of a condition tree has
//! its place in the query text (U2:BR10.3). A [`QueryProblem`] is why a query
//! cannot be read, with its place (U1:BR3.9, U2:BR2.2).

use std::hash::{Hash, Hasher};

use regex::Regex;
use serde::Serialize;

/// Most rows a `limit` can ask for (U2:BR5.1, Q1a 7a, 7b).
pub const MAX_LIMIT: u32 = 100_000;

/// Rows of a result when no `limit` from 1 to [`MAX_LIMIT`] is written
/// (U2:BR5.2, OQ1).
pub const DEFAULT_RESULT_LIMIT: usize = 10_000;

/// A place in the query text (entities.md Span). Lines and columns start at
/// 1 and count Unicode scalar values, so a Japanese character is one column.
/// A span may run over several lines; its length then counts the line
/// breaks too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    /// Line, from 1.
    pub line: u32,
    /// Column within the line, from 1.
    pub column: u32,
    /// Number of characters, at least 1.
    pub length: u32,
}

impl Span {
    /// A span of `length` characters (at least 1) at `line` and `column`.
    pub fn new(line: u32, column: u32, length: u32) -> Self {
        Self {
            line,
            column,
            length: length.max(1),
        }
    }
}

/// Whether a query is mistyped or uses what this version cannot run
/// (entities.md QueryProblem.kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProblemKind {
    /// A writing mistake: an unclosed parenthesis or quote, a date text in
    /// no accepted form, a regular expression that cannot be built, ...
    Syntax,
    /// A command, function, field or operator this version does not run.
    Unsupported,
}

/// Why a query cannot be run, found before anything is fetched (U1:BR3.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProblem {
    /// Syntax or Unsupported.
    pub kind: ProblemKind,
    /// Message-catalog key of the screen text (FR6.4, NFR9).
    pub message_key: &'static str,
    /// The word of the query the problem is about (for example `stats`);
    /// only text taken from the query (FR6.4).
    pub word: Option<String>,
    /// Where in the query text.
    pub span: Span,
}

impl QueryProblem {
    /// A writing mistake at `span`.
    pub fn syntax(message_key: &'static str, span: Span) -> Self {
        Self {
            kind: ProblemKind::Syntax,
            message_key,
            word: None,
            span,
        }
    }

    /// Something this version does not run, named by `word`, at `span`.
    pub fn unsupported(message_key: &'static str, word: impl Into<String>, span: Span) -> Self {
        Self {
            kind: ProblemKind::Unsupported,
            message_key,
            word: Some(word.into()),
            span,
        }
    }

    /// The same problem naming `word`.
    pub fn with_word(mut self, word: impl Into<String>) -> Self {
        self.word = Some(word.into());
        self
    }
}

/// A JSON field: the keys from the outermost object inwards, and the name
/// as written in the query (U2:BR1.1, BR1.3). `http.status` is the keys
/// `http`, `status`; `` `user.id` `` is the one key `user.id`. Two paths are
/// the same field when their keys are the same, however they were written.
#[derive(Debug, Clone, Eq)]
pub struct JsonPath {
    keys: Vec<String>,
    text: String,
}

impl JsonPath {
    /// The path of `keys`, written `text` in the query.
    pub fn new(keys: Vec<String>, text: impl Into<String>) -> Self {
        Self {
            keys,
            text: text.into(),
        }
    }

    /// The keys, outermost first.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The name as written in the query (the column heading).
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl PartialEq for JsonPath {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
    }
}

impl Hash for JsonPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.keys.hash(state);
    }
}

/// A field a query can read (U1:BR3.6, BR4.1, U2:BR1.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldRef {
    /// `@timestamp`: the event time in epoch milliseconds.
    Timestamp,
    /// `toMillis(@timestamp)`: the same value as `@timestamp` (U1:BR3.7).
    TimestampMillis,
    /// `@message`: the whole message.
    Message,
    /// `@logStream`: the stream name.
    LogStream,
    /// `@log`: the log group name, without an account ID (U1:BR4.1).
    Log,
    /// A field of the message read as a JSON object (U2:BR1.1-BR1.4).
    Json(JsonPath),
}

impl FieldRef {
    /// Whether the field holds the event time (a number).
    pub fn is_timestamp(&self) -> bool {
        matches!(self, FieldRef::Timestamp | FieldRef::TimestampMillis)
    }

    /// The name of the field: the fixed name, or the JSON field as written
    /// (the column heading, entities.md FieldRef.text).
    pub fn name(&self) -> &str {
        match self {
            FieldRef::Timestamp => "@timestamp",
            FieldRef::TimestampMillis => "toMillis(@timestamp)",
            FieldRef::Message => "@message",
            FieldRef::LogStream => "@logStream",
            FieldRef::Log => "@log",
            FieldRef::Json(path) => path.text(),
        }
    }
}

/// A comparison operator (U1:BR5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `=`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
}

/// A regular expression of the query, built once when the query is read
/// (U1:BR3.8). Two are equal when they were written the same.
#[derive(Debug, Clone)]
pub struct QueryRegex {
    source: String,
    regex: Regex,
}

impl QueryRegex {
    /// Builds `source` with the default size limit of the regex crate,
    /// whose matching time grows linearly with the input (ADR-002).
    pub fn new(source: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            source: source.to_string(),
            regex: Regex::new(source)?,
        })
    }

    /// The expression as written between the slashes (`\/` read as `/`).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether some part of `text` matches (U1:BR5.4).
    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

impl PartialEq for QueryRegex {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// A value of a condition (entities.md Literal).
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// A quoted string, with `\"`, `\'` and `\\` read as one character.
    Text(String),
    /// An integer or a decimal number, possibly negative.
    Number(f64),
    /// A date text compared with `@timestamp`, not yet turned into
    /// milliseconds; `span` is where it was written (U1:BR5.9).
    DateText {
        /// The text inside the quotes.
        text: String,
        /// Where the quoted text was written.
        span: Span,
    },
    /// A date text turned into epoch milliseconds in the time zone chosen
    /// when the query ran (U1:BR5.9).
    Instant(i64),
}

/// The value of `like` / `not like` (U1:BR5.3, BR5.4).
#[derive(Debug, Clone, PartialEq)]
pub enum LikePattern {
    /// Contains this text (case-sensitive).
    Text(String),
    /// Some part matches this expression.
    Regex(QueryRegex),
}

/// One part of a condition tree with its place in the query text
/// (entities.md Condition, U2:BR10.3).
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// What the part is.
    pub kind: ConditionKind,
    /// From its first word to its last, parentheses inside it included.
    pub span: Span,
}

impl Condition {
    /// The part `kind` written at `span`.
    pub fn new(kind: ConditionKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kinds of the parts of a condition tree (entities.md Condition.kind).
///
/// A run of `and` or of `or` is one list of two or more parts, not pairs
/// nested in each other, so a long `a or b or c …` is one level deep and is
/// read, evaluated and dropped by walking the list (261017-insights-query
/// U3:BR5.1, functional-design review R-02).
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionKind {
    /// Every part holds.
    And(Vec<Condition>),
    /// Some part holds.
    Or(Vec<Condition>),
    /// `not (…)`: does not hold (U2:BR2.4).
    Not(Box<Condition>),
    /// `field op value`.
    Compare {
        /// The field read.
        field: FieldRef,
        /// The comparison.
        op: CompareOp,
        /// The value compared with.
        value: Literal,
    },
    /// `field op other`: two fields of the same event compared, the right
    /// one written as a name without quotes, such as `a = b`, `a = true` or
    /// `@logStream = @log` (261017-insights-query U2 Loop-back 2, p05). The
    /// values compare as a field with a value does (U2:BR1.6); a missing
    /// field on either side meets no comparison, not even `!=` (Loop-back 3,
    /// p11: checked for the right side at the console; for the left side
    /// provisional). A field against a value keeps U2:BR1.5.
    CompareFields {
        /// The left field.
        field: FieldRef,
        /// The comparison.
        op: CompareOp,
        /// The right field.
        other: FieldRef,
    },
    /// `field like value` or `field not like value`.
    Like {
        /// The field read (never the timestamp, U1:BR5.6).
        field: FieldRef,
        /// The text or expression.
        pattern: LikePattern,
        /// `not like`.
        negated: bool,
    },
    /// `field =~ /regex/`.
    Match {
        /// The field read (never the timestamp, U1:BR5.6).
        field: FieldRef,
        /// The expression.
        regex: QueryRegex,
    },
    /// `field in [values]` or `field not in [values]`.
    In {
        /// The field read.
        field: FieldRef,
        /// At least one value.
        values: Vec<Literal>,
        /// `not in`.
        negated: bool,
    },
}

/// The direction of one `sort` field (U2:BR4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// `asc`, also when nothing is written (Q1b 1a).
    Asc,
    /// `desc`.
    Desc,
}

/// One field of a `sort` with its direction (entities.md SortKey).
#[derive(Debug, Clone, PartialEq)]
pub struct SortKey {
    /// The field compared.
    pub field: FieldRef,
    /// Ascending or descending.
    pub direction: SortDirection,
}

/// What one command does (entities.md Command.kind).
#[derive(Debug, Clone, PartialEq)]
pub enum CommandKind {
    /// `filter`: keeps the rows that meet the condition.
    Filter(Condition),
    /// `fields`: adds these columns, rows unchanged (U2:BR3.2).
    Fields(Vec<FieldRef>),
    /// `sort`: orders the rows by these keys (U2:BR4.1-BR4.4).
    Sort(Vec<SortKey>),
    /// `limit`: the first rows only; 0 keeps no row (U2:BR5.1, Loop-back 3,
    /// p08).
    Limit(u32),
}

/// One command with the place of its name (entities.md Command).
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// What it does.
    pub kind: CommandKind,
    /// Where its name was written.
    pub span: Span,
}

/// A read query: its commands in the order written (U2:BR3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    /// At least one command.
    pub commands: Vec<Command>,
}

impl QueryPlan {
    /// The conditions of the `filter` commands, in order.
    pub fn filters(&self) -> impl Iterator<Item = &Condition> {
        self.commands
            .iter()
            .filter_map(|command| match &command.kind {
                CommandKind::Filter(condition) => Some(condition),
                _ => None,
            })
    }

    /// Whether a `fields` command is written (entities.md QueryPlan.hasFields).
    pub fn has_fields(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command.kind, CommandKind::Fields(_)))
    }

    /// Whether a `sort` command is written (entities.md QueryPlan.hasSort).
    pub fn has_sort(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command.kind, CommandKind::Sort(_)))
    }

    /// The columns of the result before the empty ones are left out: the
    /// fields of every `fields` command in the order written, each field
    /// once (U2:BR3.2), or `@timestamp` and `@message` without one
    /// (U2:BR3.3, Q1a 2a, 2b).
    pub fn columns(&self) -> Vec<FieldRef> {
        let mut columns: Vec<FieldRef> = Vec::new();
        for command in &self.commands {
            if let CommandKind::Fields(fields) = &command.kind {
                for field in fields {
                    if !columns.contains(field) {
                        columns.push(field.clone());
                    }
                }
            }
        }
        if columns.is_empty() && !self.has_fields() {
            columns = vec![FieldRef::Timestamp, FieldRef::Message];
        }
        columns
    }

    /// Whether a `limit` from 0 to [`MAX_LIMIT`] is written; without one the
    /// result is cut at [`DEFAULT_RESULT_LIMIT`] (U2:BR5.2). `limit 0` is a
    /// limit of no row (261017-insights-query Loop-back 3, p08), replacing
    /// functional-design review R-04's provisional "limit 0 limits nothing".
    pub fn has_row_limit(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command.kind, CommandKind::Limit(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Span = Span {
        line: 1,
        column: 1,
        length: 1,
    };

    fn json(keys: &[&str], text: &str) -> FieldRef {
        FieldRef::Json(JsonPath::new(
            keys.iter().map(|key| (*key).to_string()).collect(),
            text,
        ))
    }

    fn command(kind: CommandKind) -> Command {
        Command { kind, span: AT }
    }

    #[test]
    fn json_paths_are_the_same_field_when_their_keys_are_the_same() {
        assert_eq!(json(&["a", "b"], "a.b"), json(&["a", "b"], "`a`.b"));
        assert_ne!(json(&["a.b"], "`a.b`"), json(&["a", "b"], "a.b"));
        assert_eq!(json(&["a", "b"], "`a`.b").name(), "`a`.b");
        assert_eq!(FieldRef::Message.name(), "@message");
        assert!(FieldRef::TimestampMillis.is_timestamp());
        assert!(!json(&["@timestamp"], "`@timestamp`").is_timestamp());
    }

    #[test]
    fn the_columns_are_the_fields_in_order_once_or_the_default_two() {
        // U2:BR3.2, BR3.3.
        let plan = QueryPlan {
            commands: vec![
                command(CommandKind::Fields(vec![
                    FieldRef::Timestamp,
                    json(&["a"], "a"),
                ])),
                command(CommandKind::Limit(5)),
                command(CommandKind::Fields(vec![
                    FieldRef::LogStream,
                    json(&["a"], "`a`"),
                ])),
            ],
        };
        assert_eq!(
            plan.columns(),
            vec![FieldRef::Timestamp, json(&["a"], "a"), FieldRef::LogStream]
        );
        assert!(plan.has_fields() && !plan.has_sort() && plan.has_row_limit());
        let bare = QueryPlan {
            commands: vec![command(CommandKind::Limit(0))],
        };
        assert_eq!(bare.columns(), vec![FieldRef::Timestamp, FieldRef::Message]);
        assert!(bare.has_row_limit(), "limit 0 is a limit of no row (p08)");
        let none = QueryPlan {
            commands: vec![command(CommandKind::Fields(vec![FieldRef::Timestamp]))],
        };
        assert!(!none.has_row_limit());
        assert_eq!(bare.filters().count(), 0);
    }
}
