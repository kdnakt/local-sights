//! The shape of a read query (261017-insights-query U1, entities.md
//! QueryPlan, Condition, FieldRef, Literal, Span, QueryProblem).
//!
//! A [`QueryPlan`] is the list of `filter` conditions of one query, in the
//! order they were written; an event is kept when it meets every one of them
//! (U1:BR3.2). A [`QueryProblem`] is why a query cannot be read, with the
//! place in the query text (U1:BR3.9).

use regex::Regex;
use serde::Serialize;

/// A place in the query text (entities.md Span). Lines and columns start at
/// 1 and count Unicode scalar values, so a Japanese character is one column.
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

/// A field a U1 condition can read (U1:BR3.6, BR4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl FieldRef {
    /// Whether the field holds the event time (a number).
    pub fn is_timestamp(self) -> bool {
        matches!(self, FieldRef::Timestamp | FieldRef::TimestampMillis)
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

/// The condition tree of one `filter` (entities.md Condition).
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// Both hold.
    And(Box<Condition>, Box<Condition>),
    /// Either holds.
    Or(Box<Condition>, Box<Condition>),
    /// Does not hold.
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

/// A read query: the conditions of its `filter` commands, in order
/// (U1:BR3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    /// At least one condition; an event is kept when it meets all of them.
    pub filters: Vec<Condition>,
}
