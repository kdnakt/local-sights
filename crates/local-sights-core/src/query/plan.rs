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

/// Rows of a result when no `limit` command is written (U2:BR5.2, OQ1). A
/// written `limit` cuts at its own value instead, `limit 0` included, which
/// gives no row ([`QueryPlan::has_row_limit`], 261017-insights-query
/// Loop-back 3, p08; 261020-carryover-fixes FR3.1).
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
    /// A column of the aggregate rows, in a command after `stats`
    /// (261025-insights-stats U2:BR1.4). An event has no value for it.
    Column(ColumnRef),
}

/// A column of the aggregate rows named in a command after `stats`
/// (entities.md ColumnRef, U2:BR1.4 (a)): resolved from its name to its
/// place when the query is read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnRef {
    /// The place of the column: the `by` keys, then the aggregates.
    pub index: usize,
    /// The column name, which is also how it was written (backquotes left
    /// out).
    pub name: String,
    /// Whether the column holds times, read as `@timestamp` is (BR1.4 (c)).
    pub is_timestamp: bool,
}

impl FieldRef {
    /// Whether the field holds the event time (a number): `@timestamp`,
    /// `toMillis(@timestamp)`, or a result column of times.
    pub fn is_timestamp(&self) -> bool {
        match self {
            FieldRef::Timestamp | FieldRef::TimestampMillis => true,
            FieldRef::Column(column) => column.is_timestamp,
            FieldRef::Message | FieldRef::LogStream | FieldRef::Log | FieldRef::Json(_) => false,
        }
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
            FieldRef::Column(column) => &column.name,
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

/// The aggregate functions of `stats` (261025-insights-stats U1:BR1.2 read
/// `count`, `sum`, `avg`, `min` and `max`; U2:BR1.1 the rest of the
/// documentation's table). Their names are the catalog's
/// ([`crate::query::catalog::QueryCatalog`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    /// `count(*)` or `count(field)`.
    Count,
    /// `count_distinct(field)` (BR2.1).
    CountDistinct,
    /// `sum(field)`.
    Sum,
    /// `avg(field)`.
    Avg,
    /// `min(field)`.
    Min,
    /// `max(field)`.
    Max,
    /// `pct(field, percentile)` (BR1.2, BR2.2).
    Pct,
    /// `stddev(field)` (BR2.3).
    Stddev,
    /// `earliest(field)` (BR2.5).
    Earliest,
    /// `latest(field)` (BR2.5).
    Latest,
    /// `sortsFirst(field)` (BR2.4).
    SortsFirst,
    /// `sortsLast(field)` (BR2.4).
    SortsLast,
}

impl AggregateFunction {
    /// Every function, in the catalog's order.
    const ALL: [AggregateFunction; 12] = [
        AggregateFunction::Count,
        AggregateFunction::CountDistinct,
        AggregateFunction::Sum,
        AggregateFunction::Avg,
        AggregateFunction::Min,
        AggregateFunction::Max,
        AggregateFunction::Pct,
        AggregateFunction::Stddev,
        AggregateFunction::Earliest,
        AggregateFunction::Latest,
        AggregateFunction::SortsFirst,
        AggregateFunction::SortsLast,
    ];

    /// The name as the documentation and the catalog write it.
    pub fn name(self) -> &'static str {
        match self {
            AggregateFunction::Count => "count",
            AggregateFunction::CountDistinct => "count_distinct",
            AggregateFunction::Sum => "sum",
            AggregateFunction::Avg => "avg",
            AggregateFunction::Min => "min",
            AggregateFunction::Max => "max",
            AggregateFunction::Pct => "pct",
            AggregateFunction::Stddev => "stddev",
            AggregateFunction::Earliest => "earliest",
            AggregateFunction::Latest => "latest",
            AggregateFunction::SortsFirst => "sortsFirst",
            AggregateFunction::SortsLast => "sortsLast",
        }
    }

    /// The function named `name`, written exactly so (BR1.1).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.name() == name)
    }

    /// Whether the function keeps a value of the field as it is (so a
    /// column of it holds times when the field is `@timestamp`).
    pub fn keeps_a_value(self) -> bool {
        matches!(
            self,
            AggregateFunction::Min
                | AggregateFunction::Max
                | AggregateFunction::Earliest
                | AggregateFunction::Latest
                | AggregateFunction::SortsFirst
                | AggregateFunction::SortsLast
        )
    }
}

/// Most decimal places of a `pct` percentile, so that its rank is worked out
/// in `u128` without overflow (BR2.2).
pub const MAX_PERCENTILE_DECIMALS: usize = 15;

/// The percentile of `pct` (entities.md AggregateSpec.percentile, BR1.2):
/// `numerator / 10^scale`, from 0 to 100, kept as written so that its rank
/// is exact (BR2.2, FR review R-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percentile {
    /// The digits, without the decimal point: 999 for `99.9`.
    pub numerator: u64,
    /// The number of digits after the decimal point: 1 for `99.9`.
    pub scale: u32,
}

impl Percentile {
    /// The percentile written `text`: digits, optionally `.` and at most
    /// [`MAX_PERCENTILE_DECIMALS`] digits, from 0 to 100; `None` otherwise.
    pub fn parse(text: &str) -> Option<Self> {
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if whole.is_empty()
            || !digits(whole)
            || !digits(fraction)
            || (text.contains('.') && fraction.is_empty())
            || fraction.len() > MAX_PERCENTILE_DECIMALS
            || whole.len() > 3
        {
            return None;
        }
        let scale = u32::try_from(fraction.len()).ok()?;
        let numerator = format!("{whole}{fraction}").parse::<u64>().ok()?;
        let hundred = 100u64.checked_mul(10u64.checked_pow(scale)?)?;
        (numerator <= hundred).then_some(Self { numerator, scale })
    }

    /// The denominator: `100 × 10^scale`.
    pub fn whole(self) -> u128 {
        100 * 10u128.pow(self.scale)
    }
}

/// What an aggregate function reads (entities.md AggregateArgument, BR1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateArgument {
    /// `*`: every event (`count` only).
    All,
    /// A fixed field, `toMillis(@timestamp)` or a JSON field.
    Field(FieldRef),
}

/// One aggregate of `stats` (entities.md AggregateSpec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateSpec {
    /// The function.
    pub function: AggregateFunction,
    /// What it reads.
    pub argument: AggregateArgument,
    /// The percentile of `pct`; `None` for every other function (U2:BR1.2).
    pub percentile: Option<Percentile>,
    /// The column name: the `as` name, or the text as written (BR1.4,
    /// BR1.9).
    pub column_name: String,
}

/// The unit of a `bin()` period (entities.md BinPeriod.unit, BR1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinUnit {
    /// `s`.
    Seconds,
    /// `m`.
    Minutes,
    /// `h`.
    Hours,
    /// `d`.
    Days,
}

impl BinUnit {
    /// Milliseconds of one unit.
    pub fn millis(self) -> i64 {
        match self {
            BinUnit::Seconds => 1_000,
            BinUnit::Minutes => 60_000,
            BinUnit::Hours => 3_600_000,
            BinUnit::Days => 86_400_000,
        }
    }
}

/// The period of `bin()` (entities.md BinPeriod): a count from 1 to
/// 1,000,000 (BR1.8) of a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinPeriod {
    /// At least 1.
    pub count: u32,
    /// Seconds, minutes, hours or days.
    pub unit: BinUnit,
}

impl BinPeriod {
    /// The period in milliseconds; the bins are its multiples from the epoch
    /// (BR2.5). The largest count read keeps this inside `i64`.
    pub fn millis(&self) -> i64 {
        i64::from(self.count).saturating_mul(self.unit.millis())
    }
}

/// What a `by` key groups by (entities.md GroupKey.kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupKeyKind {
    /// A fixed field or a JSON field.
    Field(FieldRef),
    /// `bin(period)` of `@timestamp`.
    Bin(BinPeriod),
}

/// One `by` key of `stats` (entities.md GroupKey).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKey {
    /// What it groups by.
    pub kind: GroupKeyKind,
    /// The column name: the `as` name, or the text as written (BR1.4).
    pub column_name: String,
}

/// One command after `stats` (entities.md AfterStatsCommand, U2:BR1.4): it
/// works on the aggregate rows, its fields being [`FieldRef::Column`]s.
#[derive(Debug, Clone, PartialEq)]
pub enum AfterStatsCommand {
    /// `filter`: keeps the rows that meet the condition.
    Filter(Condition),
    /// `sort`: orders the rows by these columns, stably.
    Sort(Vec<SortKey>),
    /// `limit`: the first rows only; 0 keeps no row.
    Limit(u32),
}

/// The content of a `stats` command (entities.md StatsCommand): the
/// aggregates and the `by` keys, each in the order written, and the
/// commands written after it (U2:BR1.4).
#[derive(Debug, Clone, PartialEq)]
pub struct StatsCommand {
    /// At least one.
    pub aggregates: Vec<AggregateSpec>,
    /// None without `by`: the whole input is one group (BR2.6).
    pub group_keys: Vec<GroupKey>,
    /// `filter`, `sort` and `limit` after `stats`, in the order written
    /// (U2:BR1.4, BR3.1).
    pub after: Vec<AfterStatsCommand>,
}

impl StatsCommand {
    /// The columns of the aggregate rows, as commands after `stats` name
    /// them (BR1.4): the keys, then the aggregates. A `bin` key, a
    /// `by @timestamp` key and a function that keeps a value of
    /// `@timestamp` hold times.
    pub fn result_columns(&self) -> Vec<ColumnRef> {
        let keys = self.group_keys.iter().map(|key| {
            let is_timestamp = match &key.kind {
                GroupKeyKind::Bin(_) => true,
                GroupKeyKind::Field(field) => *field == FieldRef::Timestamp,
            };
            (key.column_name.clone(), is_timestamp)
        });
        let aggregates = self.aggregates.iter().map(|aggregate| {
            let is_timestamp = aggregate.function.keeps_a_value()
                && aggregate.argument == AggregateArgument::Field(FieldRef::Timestamp);
            (aggregate.column_name.clone(), is_timestamp)
        });
        keys.chain(aggregates)
            .enumerate()
            .map(|(index, (name, is_timestamp))| ColumnRef {
                index,
                name,
                is_timestamp,
            })
            .collect()
    }

    /// Whether a `limit` is written after `stats`; without one the rows are
    /// cut at [`DEFAULT_RESULT_LIMIT`] (U2:BR3.2).
    pub fn has_row_limit(&self) -> bool {
        self.after
            .iter()
            .any(|command| matches!(command, AfterStatsCommand::Limit(_)))
    }

    /// The column names: the keys, then the aggregates (BR3.3).
    pub fn column_names(&self) -> Vec<String> {
        self.group_keys
            .iter()
            .map(|key| key.column_name.clone())
            .chain(
                self.aggregates
                    .iter()
                    .map(|aggregate| aggregate.column_name.clone()),
            )
            .collect()
    }
}

/// The shape of a result (261025-insights-stats ADR-001, BR3.1): rows of
/// events, or rows of aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultShape {
    /// Event rows (no `stats`).
    Events,
    /// Aggregate rows (`stats`).
    Aggregates,
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
    /// `stats`: groups the rows and aggregates each group; one, the
    /// commands after it held in it (261025-insights-stats U1:BR1.6,
    /// U2:BR1.4, BR1.5).
    Stats(StatsCommand),
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

    /// The `stats` command, when one is written (261025-insights-stats U1).
    pub fn stats(&self) -> Option<&StatsCommand> {
        self.commands
            .iter()
            .find_map(|command| match &command.kind {
                CommandKind::Stats(stats) => Some(stats),
                _ => None,
            })
    }

    /// Whether a `stats` command is written.
    pub fn has_stats(&self) -> bool {
        self.stats().is_some()
    }

    /// Aggregates with `stats`, events without (BR3.1).
    pub fn result_shape(&self) -> ResultShape {
        if self.has_stats() {
            ResultShape::Aggregates
        } else {
            ResultShape::Events
        }
    }

    /// The column names of an aggregates result: the `by` keys, then the
    /// aggregates, as written (BR3.3); none without `stats`.
    pub fn aggregate_columns(&self) -> Vec<String> {
        self.stats()
            .map(StatsCommand::column_names)
            .unwrap_or_default()
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

    // ---- 261025-insights-stats U1:BR3.1, BR3.3 ----------------------------

    fn count_all() -> AggregateSpec {
        AggregateSpec {
            function: AggregateFunction::Count,
            argument: AggregateArgument::All,
            percentile: None,
            column_name: "count(*)".to_string(),
        }
    }

    fn avg_latency() -> AggregateSpec {
        AggregateSpec {
            function: AggregateFunction::Avg,
            argument: AggregateArgument::Field(json(&["latency"], "latency")),
            percentile: None,
            column_name: "a".to_string(),
        }
    }

    fn by_level() -> GroupKey {
        GroupKey {
            kind: GroupKeyKind::Field(json(&["level"], "level")),
            column_name: "level".to_string(),
        }
    }

    fn by_bin(count: u32, unit: BinUnit, name: &str) -> GroupKey {
        GroupKey {
            kind: GroupKeyKind::Bin(BinPeriod { count, unit }),
            column_name: name.to_string(),
        }
    }

    fn stats_plan(stats: StatsCommand) -> QueryPlan {
        QueryPlan {
            commands: vec![
                command(CommandKind::Filter(Condition::new(
                    ConditionKind::Compare {
                        field: FieldRef::Message,
                        op: CompareOp::Eq,
                        value: Literal::Text("x".to_string()),
                    },
                    AT,
                ))),
                command(CommandKind::Stats(stats)),
            ],
        }
    }

    #[test]
    fn a_plan_with_stats_has_the_aggregates_shape_and_one_without_the_events_shape() {
        // BR3.1.
        let stats = StatsCommand {
            aggregates: vec![count_all()],
            group_keys: Vec::new(),
            after: Vec::new(),
        };
        let plan = stats_plan(stats.clone());
        assert!(plan.has_stats());
        assert_eq!(plan.stats(), Some(&stats));
        assert_eq!(plan.result_shape(), ResultShape::Aggregates);
        let events = QueryPlan {
            commands: vec![command(CommandKind::Limit(5))],
        };
        assert!(!events.has_stats());
        assert_eq!(events.stats(), None);
        assert_eq!(events.result_shape(), ResultShape::Events);
        assert_eq!(events.aggregate_columns(), Vec::<String>::new());
    }

    #[test]
    fn the_aggregate_columns_are_the_keys_then_the_aggregates_as_written() {
        // BR3.3.
        let plan = stats_plan(StatsCommand {
            aggregates: vec![count_all(), avg_latency()],
            group_keys: vec![by_bin(5, BinUnit::Minutes, "bin(5m)"), by_level()],
            after: Vec::new(),
        });
        assert_eq!(
            plan.aggregate_columns(),
            vec!["bin(5m)", "level", "count(*)", "a"]
        );
    }

    #[test]
    fn a_stats_without_by_has_only_the_aggregate_columns() {
        let plan = stats_plan(StatsCommand {
            aggregates: vec![avg_latency(), count_all()],
            group_keys: Vec::new(),
            after: Vec::new(),
        });
        assert_eq!(plan.aggregate_columns(), vec!["a", "count(*)"]);
    }

    #[test]
    fn a_bin_period_is_its_count_times_the_milliseconds_of_its_unit() {
        // BR1.5.
        let period = |count, unit| BinPeriod { count, unit }.millis();
        assert_eq!(period(30, BinUnit::Seconds), 30_000);
        assert_eq!(period(5, BinUnit::Minutes), 300_000);
        assert_eq!(period(1, BinUnit::Hours), 3_600_000);
        assert_eq!(period(1, BinUnit::Days), 86_400_000);
        // The largest count read (BR1.8) stays inside i64.
        assert_eq!(period(1_000_000, BinUnit::Days), 86_400_000_000_000);
    }

    // ---- 261025-insights-stats U2:BR1.1, BR1.2, BR1.4 ---------------------

    #[test]
    fn every_aggregate_function_of_the_catalog_has_its_function() {
        // BR1.1, BR4.1: the names come from the catalog, as written.
        use crate::query::catalog::{QueryCatalog, VocabularyKind};
        let names = QueryCatalog::names(VocabularyKind::AggregateFunction, true);
        assert_eq!(names.len(), 12);
        for name in names {
            let function = AggregateFunction::from_name(name);
            assert_eq!(function.map(AggregateFunction::name), Some(name), "{name}");
        }
        assert_eq!(AggregateFunction::from_name("sortsfirst"), None);
        assert_eq!(AggregateFunction::from_name("COUNT"), None);
        assert_eq!(AggregateFunction::from_name("bin"), None);
    }

    #[test]
    fn a_percentile_is_a_number_from_0_to_100_kept_as_digits() {
        // BR1.2, BR2.2 (FR review R-05): no floating point.
        let percentile = |numerator, scale| Some(Percentile { numerator, scale });
        assert_eq!(Percentile::parse("95"), percentile(95, 0));
        assert_eq!(Percentile::parse("99.9"), percentile(999, 1));
        assert_eq!(Percentile::parse("0"), percentile(0, 0));
        assert_eq!(Percentile::parse("100"), percentile(100, 0));
        assert_eq!(Percentile::parse("100.000"), percentile(100_000, 3));
        assert_eq!(Percentile::parse("0.5"), percentile(5, 1));
        for out in ["101", "100.01", "-1", "-0.5", "", ".5", "5.", "1e2", "x"] {
            assert_eq!(Percentile::parse(out), None, "{out:?}");
        }
        let longest = format!("99.{}", "9".repeat(MAX_PERCENTILE_DECIMALS - 2));
        assert!(Percentile::parse(&longest).is_some());
        let longer = format!("99.{}", "9".repeat(MAX_PERCENTILE_DECIMALS + 1));
        assert_eq!(Percentile::parse(&longer), None);
    }

    #[test]
    fn the_result_columns_say_which_hold_times() {
        // BR1.4 (c): a bin key, a by @timestamp key and the extremes of
        // @timestamp are times; counts and sums are not.
        let min_time = AggregateSpec {
            function: AggregateFunction::Min,
            argument: AggregateArgument::Field(FieldRef::Timestamp),
            percentile: None,
            column_name: "first".to_string(),
        };
        let latest_time = AggregateSpec {
            function: AggregateFunction::Latest,
            argument: AggregateArgument::Field(FieldRef::Timestamp),
            percentile: None,
            column_name: "latest(@timestamp)".to_string(),
        };
        let sum_millis = AggregateSpec {
            function: AggregateFunction::Sum,
            argument: AggregateArgument::Field(FieldRef::Timestamp),
            percentile: None,
            column_name: "s".to_string(),
        };
        let stats = StatsCommand {
            aggregates: vec![count_all(), min_time, latest_time, sum_millis],
            group_keys: vec![by_bin(5, BinUnit::Minutes, "t"), by_level()],
            after: Vec::new(),
        };
        let columns = stats.result_columns();
        let columns: Vec<(usize, &str, bool)> = columns
            .iter()
            .map(|column| (column.index, column.name.as_str(), column.is_timestamp))
            .collect();
        assert_eq!(
            columns,
            vec![
                (0, "t", true),
                (1, "level", false),
                (2, "count(*)", false),
                (3, "first", true),
                (4, "latest(@timestamp)", true),
                (5, "s", false),
            ]
        );
        let column = FieldRef::Column(ColumnRef {
            index: 0,
            name: "t".to_string(),
            is_timestamp: true,
        });
        assert_eq!(column.name(), "t");
        assert!(column.is_timestamp());
        assert!(!stats.has_row_limit());
    }

    #[test]
    fn the_shape_is_spelled_for_the_screen() {
        assert_eq!(
            serde_json::to_value(ResultShape::Aggregates).unwrap(),
            serde_json::json!("aggregates")
        );
        assert_eq!(
            serde_json::to_value(ResultShape::Events).unwrap(),
            serde_json::json!("events")
        );
    }
}
