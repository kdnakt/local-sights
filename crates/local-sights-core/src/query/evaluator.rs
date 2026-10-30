//! QueryEvaluator: applies a read plan to the events the query screen
//! fetched (261017-insights-query U1:BR5.1-BR5.5, BR5.8, BR5.10, U2:BR1.5,
//! BR1.6, BR3.1-BR3.4, BR4.1-BR4.4, BR5.1, BR5.2, BR7.1, BR7.3).
//!
//! The entrance takes the events in time order with the log group each
//! belongs to ([`EvalItem`], unit-of-work-dependency's evaluation entrance;
//! [`EvalItems`] lends them by position). An [`Evaluation`] first puts the
//! rows in the fixed order (`@timestamp` newest first, equal times by log
//! group, stream and order in the stream, U2:BR4.4, Q1b 1d), then applies
//! the commands in the order written to the rows the previous one left
//! (U2:BR3.1): `filter` keeps rows, `sort` orders them stably, `limit` keeps
//! the first ones, `fields` adds columns. Without a `limit` from 1 to
//! 100,000 the result is cut at 10,000 rows (U2:BR5.2). Last, the columns no
//! result row has a value for are left out (U2:BR3.4, Q1a 5j).
//!
//! Work goes [`EVALUATION_CHUNK`] rows at a time, and between two chunks the
//! caller can stop it (a [Cancel], a newer run, a connection change); what
//! was found is then dropped (U1:BR5.8). A `filter` reads each row's message
//! as JSON at most once; a `sort` on JSON fields first takes each row's
//! values in chunks, once per row, and then orders the rows by those values
//! alone, which cannot be stopped in the middle; fixed fields are read in
//! place while ordering. The column check reads the result rows only, in
//! chunks (functional-design review R-02).
//!
//! With `stats` (261025-insights-stats) the rows the commands before it left
//! are aggregated chunk by chunk; the aggregate rows are then finished by
//! the `filter`, `sort` and `limit` written after `stats`, in that order, a
//! filter [`EVALUATION_CHUNK`] rows at a time, and cut at 10,000 rows without
//! a `limit`, as the event rows are (U2:BR3.1, BR3.2).

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::event::LogEvent;
use crate::query::fields::{EventFields, FieldSource, FieldValue, read_field};
use crate::query::natural::{SortKeyRef, SortValue, compare_sort_keys};
use crate::query::plan::{
    AfterStatsCommand, CommandKind, CompareOp, Condition, ConditionKind, DEFAULT_RESULT_LIMIT,
    FieldRef, LikePattern, Literal, QueryPlan, ResultShape, SortDirection, SortKey, StatsCommand,
};
use crate::query::stats::{AggregateTable, CellValue, StatsAggregator, cell_text};

/// Rows worked on between two looks at the stop signal (U1:BR5.8). May be
/// changed after measuring NFR1 and NFR2.
pub const EVALUATION_CHUNK: usize = 10_000;

/// One event to evaluate with the name of its log group (entities.md
/// EvalItem); the event type itself stays as it is (ADR-001).
#[derive(Debug, Clone, Copy)]
pub struct EvalItem<'a> {
    /// The event, as held.
    pub event: &'a LogEvent,
    /// The log group it was fetched from, the value of `@log`.
    pub log_group_name: &'a str,
}

/// The events to evaluate, in time order, lent by position.
pub trait EvalItems {
    /// Number of events.
    fn len(&self) -> usize;
    /// Whether there is no event.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The event at `position`, below [`EvalItems::len`].
    fn item(&self, position: usize) -> Option<EvalItem<'_>>;
}

impl EvalItems for [EvalItem<'_>] {
    fn len(&self) -> usize {
        <[EvalItem<'_>]>::len(self)
    }

    fn item(&self, position: usize) -> Option<EvalItem<'_>> {
        self.get(position).copied()
    }
}

/// Events of one log group, as the QueryStore holds them.
#[derive(Debug, Clone, Copy)]
pub struct GroupEvents<'a> {
    /// The events in time order.
    pub events: &'a [LogEvent],
    /// Their log group, the value of `@log`.
    pub log_group_name: &'a str,
}

impl EvalItems for GroupEvents<'_> {
    fn len(&self) -> usize {
        self.events.len()
    }

    fn item(&self, position: usize) -> Option<EvalItem<'_>> {
        self.events.get(position).map(|event| EvalItem {
            event,
            log_group_name: self.log_group_name,
        })
    }
}

/// Whether `item` meets every filter of `plan` (U1:BR3.2); its message is
/// read as JSON at most once.
pub fn matches(plan: &QueryPlan, item: &EvalItem<'_>) -> bool {
    let fields = EventFields::new(item.event, item.log_group_name);
    plan.filters()
        .all(|condition| condition_holds(condition, &fields))
}

/// Whether `condition` holds for the event of `fields` (or, after `stats`,
/// for an aggregate row, 261025-insights-stats U2:BR3.1), from left to
/// right, stopping as soon as the answer is known (U1:BR5.2).
pub fn condition_holds<F: FieldSource + ?Sized>(condition: &Condition, fields: &F) -> bool {
    match &condition.kind {
        ConditionKind::And(parts) => parts.iter().all(|part| condition_holds(part, fields)),
        ConditionKind::Or(parts) => parts.iter().any(|part| condition_holds(part, fields)),
        ConditionKind::Not(inner) => !condition_holds(inner, fields),
        ConditionKind::Compare { field, op, value } => compare(fields.value(field), *op, value),
        // Loop-back 2 (p05): both values read from the same event.
        ConditionKind::CompareFields { field, op, other } => {
            compare_fields(fields.value(field), *op, fields.value(other))
        }
        // U2:BR1.5: like on a missing field is false, so `not like` holds.
        ConditionKind::Like {
            field,
            pattern,
            negated,
        } => like(fields.value(field), pattern) != *negated,
        ConditionKind::Match { field, regex } => {
            as_text(fields.value(field)).is_some_and(|text| regex.is_match(&text))
        }
        // U2:BR1.5: no value equals a missing field, so `not in` holds.
        ConditionKind::In {
            field,
            values,
            negated,
        } => {
            let value = fields.value(field);
            values
                .iter()
                .any(|literal| compare(value, CompareOp::Eq, literal))
                != *negated
        }
    }
}

/// The number a text of digits stands for (U2:BR1.6): an optional `-`, then
/// digits, then optionally `.` and digits; no blank, sign `+` or exponent.
/// `sum` and `avg` read texts the same way (261025-insights-stats BR2.3).
pub(crate) fn number_of_digits(text: &str) -> Option<f64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
    let all_digits =
        |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    text.parse::<f64>().ok()
}

/// One side of a comparison: a field's value or a written value.
#[derive(Debug, Clone, Copy)]
enum Side<'a> {
    /// No value: a missing field, or a date text not yet resolved.
    Nothing,
    /// A text; a boolean is the text `true` or `false`.
    Text(&'a str),
    /// A number: a JSON number, the time, a written number or the
    /// milliseconds of a date text.
    Number(f64),
}

impl<'a> Side<'a> {
    fn of_value(value: FieldValue<'a>) -> Self {
        match value {
            FieldValue::Missing => Side::Nothing,
            FieldValue::Text(text) => Side::Text(text),
            FieldValue::Boolean(flag) => Side::Text(bool_text(flag)),
            FieldValue::Number(number) => Side::Number(number),
        }
    }

    fn of_literal(literal: &'a Literal) -> Self {
        match literal {
            Literal::Text(text) => Side::Text(text),
            Literal::Number(number) => Side::Number(*number),
            // Epoch milliseconds of any real time are below 2^53: exact.
            Literal::Instant(instant) => Side::Number(*instant as f64),
            Literal::DateText { .. } => Side::Nothing,
        }
    }
}

/// How two sides stand (U1:BR5.1, BR5.10, U2:BR1.5, BR1.6).
enum Standing {
    /// They compare in this order.
    Ordered(Ordering),
    /// Two texts of which either is no number: equal or not, but in no
    /// order (Loop-back 2, p03: `eventName > "M"` gave no row at the
    /// console; Loop-back 3, p12: nor did `eventName > "5"`).
    UnorderedTexts { equal: bool },
    /// They cannot be compared.
    Incomparable,
}

/// How `left` stands against `right`:
///
/// - nothing compares with anything (5c, 5d; two fields with one missing
///   never get here, see [`compare_fields`]);
/// - two numbers as numbers (the time, JSON numbers, written numbers, the
///   milliseconds of a date text);
/// - a text of digits against a number as numbers (6b; `<` and `>`
///   provisional, review R-01), any other text against a number not at all
///   (6c-6e, 6g, 6h);
/// - two texts of digits as numbers (6f); two texts of which either is no
///   number equal or not, in no order (p03, Loop-back 3 p12: `eventName >
///   "5"` gave no row at the console);
/// - a boolean as the text `true` or `false` (`= "true"` checked, p04;
///   the rest provisional, review R-01).
fn standing(left: Side<'_>, right: Side<'_>) -> Standing {
    let ordered =
        |ordering: Option<Ordering>| ordering.map_or(Standing::Incomparable, Standing::Ordered);
    match (left, right) {
        (Side::Nothing, _) | (_, Side::Nothing) => Standing::Incomparable,
        (Side::Number(left), Side::Number(right)) => ordered(left.partial_cmp(&right)),
        (Side::Number(number), Side::Text(text)) => {
            ordered(number_of_digits(text).and_then(|other| number.partial_cmp(&other)))
        }
        (Side::Text(text), Side::Number(number)) => {
            ordered(number_of_digits(text).and_then(|other| other.partial_cmp(&number)))
        }
        (Side::Text(left), Side::Text(right)) => {
            match (number_of_digits(left), number_of_digits(right)) {
                (Some(left), Some(right)) => ordered(left.partial_cmp(&right)),
                // p03 (two texts no numbers) and p12 (one of them no
                // number): equal or not, in no order.
                _ => Standing::UnorderedTexts {
                    equal: left == right,
                },
            }
        }
    }
}

fn bool_text(flag: bool) -> &'static str {
    if flag { "true" } else { "false" }
}

/// A field's value against a written value (U1:BR5.1).
fn compare(value: FieldValue<'_>, op: CompareOp, literal: &Literal) -> bool {
    compare_sides(Side::of_value(value), op, Side::of_literal(literal))
}

/// Two fields of the same event (Loop-back 2, p05): a missing field on
/// either side meets no comparison, not even `!=` (Loop-back 3, p11:
/// `eventName != errorCode` kept only the events that have errorCode; for a
/// missing left field provisional). Otherwise as a field against a value.
fn compare_fields(left: FieldValue<'_>, op: CompareOp, right: FieldValue<'_>) -> bool {
    if left == FieldValue::Missing || right == FieldValue::Missing {
        return false;
    }
    compare_sides(Side::of_value(left), op, Side::of_value(right))
}

/// U1:BR5.1: values that cannot be compared meet only `!=` (U2:BR1.5,
/// BR1.6); two texts of which either is no number meet `=` and `!=` only
/// (Loop-back 2, p03; Loop-back 3, p12).
fn compare_sides(left: Side<'_>, op: CompareOp, right: Side<'_>) -> bool {
    let ordering = match standing(left, right) {
        Standing::Ordered(ordering) => ordering,
        Standing::UnorderedTexts { equal } => {
            return match op {
                CompareOp::Eq => equal,
                CompareOp::NotEq => !equal,
                CompareOp::Lt | CompareOp::LtEq | CompareOp::Gt | CompareOp::GtEq => false,
            };
        }
        Standing::Incomparable => return op == CompareOp::NotEq,
    };
    match op {
        CompareOp::Eq => ordering == Ordering::Equal,
        CompareOp::NotEq => ordering != Ordering::Equal,
        CompareOp::Lt => ordering == Ordering::Less,
        CompareOp::LtEq => ordering != Ordering::Greater,
        CompareOp::Gt => ordering == Ordering::Greater,
        CompareOp::GtEq => ordering != Ordering::Less,
    }
}

/// The text `like` and `=~` look into: a text as it is; a JSON number or
/// boolean as written (provisional, review R-01); nothing for a missing
/// value. The time never comes here (U1:BR5.6).
fn as_text(value: FieldValue<'_>) -> Option<Cow<'_, str>> {
    match value {
        FieldValue::Text(text) => Some(Cow::Borrowed(text)),
        FieldValue::Boolean(flag) => Some(Cow::Borrowed(bool_text(flag))),
        FieldValue::Number(number) => Some(Cow::Owned(number_text(number))),
        FieldValue::Missing => None,
    }
}

/// A JSON number as written: whole numbers without a fraction.
fn number_text(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        // A whole number well below 2^53: the conversion is exact.
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

/// U1:BR5.3, BR5.4: a text contains the text (case-sensitive), or some part
/// of it matches the expression.
fn like(value: FieldValue<'_>, pattern: &LikePattern) -> bool {
    let Some(text) = as_text(value) else {
        return false;
    };
    match pattern {
        LikePattern::Text(part) => text.contains(part.as_str()),
        LikePattern::Regex(regex) => regex.is_match(&text),
    }
}

/// What an evaluation leaves when it ends (entities.md QueryResult), in the
/// shape of the plan (261025-insights-stats BR3.1, BR3.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalOutput {
    /// No `stats`: rows of events.
    Events(EventsOutput),
    /// `stats`: rows of aggregates.
    Aggregates(AggregatesOutput),
}

impl EvalOutput {
    /// Events evaluated: every held event.
    pub fn evaluated_count(&self) -> usize {
        match self {
            EvalOutput::Events(output) => output.evaluated_count,
            EvalOutput::Aggregates(output) => output.evaluated_count,
        }
    }
}

/// The aggregate rows of a `stats` (U1:BR3.3, BR3.5): no column left out;
/// after the commands written after `stats`, cut as the event rows are
/// (U2:BR3.1, BR3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatesOutput {
    /// The column names: the keys, then the aggregates.
    pub columns: Vec<String>,
    /// One cell per column per row: in the order of BR2.7, or of a `sort`
    /// after `stats`.
    pub rows: Vec<Vec<CellValue>>,
    /// Events evaluated: every held event.
    pub evaluated_count: usize,
    /// Rows left before the last `limit` and the default cut; a `limit`
    /// with a `filter` after it is not undone (U2:BR3.2, as
    /// [`EventsOutput::matched_count`]).
    pub matched_count: usize,
    /// The number of rows shown, when a `limit` or the default 10,000 cut
    /// rows (U2:BR3.2).
    pub truncated_at: Option<usize>,
}

/// The cells of one aggregate row as a [`FieldSource`]: a result column
/// reads its cell, any other field has no value (U2:BR3.1).
struct CellFields<'a> {
    cells: &'a [CellValue],
}

impl FieldSource for CellFields<'_> {
    fn value(&self, field: &FieldRef) -> FieldValue<'_> {
        match field {
            FieldRef::Column(column) => self
                .cells
                .get(column.index)
                .map_or(FieldValue::Missing, cell_value),
            _ => FieldValue::Missing,
        }
    }
}

/// A cell as a field value (U2:BR3.1): integers, decimals and times are
/// numbers (a time in milliseconds), texts are texts.
fn cell_value(cell: &CellValue) -> FieldValue<'_> {
    match cell {
        CellValue::Missing => FieldValue::Missing,
        // A count or a whole JSON number below 2^53: exact.
        CellValue::Integer(number) | CellValue::Timestamp(number) => {
            FieldValue::Number(*number as f64)
        }
        CellValue::Decimal(number) => FieldValue::Number(*number),
        CellValue::Text(text) => FieldValue::Text(text),
    }
}

/// The aggregate rows being finished by the commands after `stats`
/// (U2:BR3.1): command `next` of them is applied next.
#[derive(Debug)]
struct Finishing {
    columns: Vec<String>,
    rows: Vec<Vec<CellValue>>,
    matched: usize,
    next: usize,
    /// The `filter` being applied: rows judged so far, and which hold.
    keep: Vec<bool>,
}

/// The finished aggregate rows, cut (U2:BR3.2).
#[derive(Debug)]
struct Aggregated {
    columns: Vec<String>,
    rows: Vec<Vec<CellValue>>,
    matched: usize,
    truncated_at: Option<usize>,
}

impl Finishing {
    fn new(table: AggregateTable) -> Self {
        Self {
            matched: table.rows.len(),
            columns: table.columns,
            rows: table.rows,
            next: 0,
            keep: Vec::new(),
        }
    }

    /// Whether every command after `stats` was applied.
    fn is_finished(&self, stats: &StatsCommand) -> bool {
        self.next >= stats.after.len()
    }

    /// One piece of the commands after `stats`: [`EVALUATION_CHUNK`] rows of
    /// a `filter`, a whole `sort` (which cannot be stopped in the middle,
    /// like the events' sort) or a `limit`.
    fn work(&mut self, stats: &StatsCommand) {
        let Some(command) = stats.after.get(self.next) else {
            return;
        };
        match command {
            AfterStatsCommand::Filter(condition) => {
                let start = self.keep.len();
                let end = start.saturating_add(EVALUATION_CHUNK).min(self.rows.len());
                for cells in &self.rows[start..end] {
                    self.keep.push(condition_holds(condition, &CellFields { cells }));
                }
                if end < self.rows.len() {
                    return;
                }
                let keep = std::mem::take(&mut self.keep);
                let rows = std::mem::take(&mut self.rows);
                self.rows = rows
                    .into_iter()
                    .zip(keep)
                    .filter_map(|(row, kept)| kept.then_some(row))
                    .collect();
                self.matched = self.rows.len();
            }
            AfterStatsCommand::Sort(keys) => {
                // Stable: equal rows keep the order they had.
                self.rows.sort_by(|left, right| compare_cell_rows(keys, left, right));
            }
            AfterStatsCommand::Limit(count) => {
                // 0 keeps none (261017-insights-query Loop-back 3, p08).
                self.rows
                    .truncate(usize::try_from(*count).unwrap_or(usize::MAX));
            }
        }
        self.next += 1;
    }

    /// The default cut, unless a `limit` is written after `stats`, and what
    /// was cut (U2:BR3.2): the events' rule.
    fn cut(mut self, has_row_limit: bool) -> Aggregated {
        if !has_row_limit {
            self.rows.truncate(DEFAULT_RESULT_LIMIT);
        }
        let truncated_at =
            (!self.rows.is_empty() && self.rows.len() < self.matched).then_some(self.rows.len());
        Aggregated {
            columns: self.columns,
            rows: self.rows,
            matched: self.matched,
            truncated_at,
        }
    }
}

/// Two aggregate rows by the columns of a `sort` after `stats`, in the
/// natural order of `sort` (U2:BR3.1).
fn compare_cell_rows(keys: &[SortKey], left: &[CellValue], right: &[CellValue]) -> Ordering {
    for sort_key in keys {
        let order = compare_sort_keys(
            cell_sort_key(left, &sort_key.field),
            cell_sort_key(right, &sort_key.field),
        );
        let order = match sort_key.direction {
            SortDirection::Asc => order,
            SortDirection::Desc => order.reverse(),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

/// The sort value of the cell `field` names in `cells`.
fn cell_sort_key<'a>(cells: &'a [CellValue], field: &FieldRef) -> SortKeyRef<'a> {
    let FieldRef::Column(column) = field else {
        return SortKeyRef::Missing;
    };
    cells
        .get(column.index)
        .map_or(SortKeyRef::Missing, |cell| SortKeyRef::of(cell_value(cell)))
}

/// The event rows of a plan without `stats`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventsOutput {
    /// The result rows: positions in the holdings, in the result's order.
    pub rows: Vec<usize>,
    /// The columns, empty ones left out (U2:BR3.2-BR3.4).
    pub columns: Vec<FieldRef>,
    /// Rows the commands left before the last `limit` and the default cut;
    /// a `limit` with a `filter` after it is not undone (review R-08).
    pub matched_count: usize,
    /// The number of rows shown, when a `limit` or the default 10,000 cut
    /// rows that matched (U2:BR5.3).
    pub truncated_at: Option<usize>,
    /// Events evaluated: every held event.
    pub evaluated_count: usize,
}

/// One evaluation in progress, chunk by chunk (U1:BR5.8, U2:BR3.1).
#[derive(Debug)]
pub struct Evaluation {
    total: usize,
    stage: Stage,
    rows: Vec<usize>,
    matched: usize,
    passes_planned: usize,
    passes_done: usize,
}

/// Where an evaluation stands.
#[derive(Debug)]
enum Stage {
    /// The rows are not in the fixed order yet.
    Ordering,
    /// Command `index` is applied; `pass` is its chunked work so far.
    Command { index: usize, pass: Pass },
    /// `stats`: the rows up to `next` are given to the aggregator
    /// (261025-insights-stats BR2.9, BR3.5).
    Aggregate {
        next: usize,
        aggregator: Box<StatsAggregator>,
    },
    /// The result's columns are being checked for values (U2:BR3.4).
    Columns {
        columns: Vec<FieldRef>,
        present: Vec<bool>,
        next: usize,
    },
    /// The aggregate rows are being finished by the commands after `stats`
    /// (U2:BR3.1).
    Finishing(Box<Finishing>),
    /// Finished with event rows.
    Done { columns: Vec<FieldRef> },
    /// Finished with aggregate rows (U1:BR3.5, U2:BR3.2).
    Aggregated(Box<Aggregated>),
}

/// The chunked work of one command.
#[derive(Debug)]
enum Pass {
    /// Not started.
    Fresh,
    /// `filter`: rows up to `next` are judged, `kept` are those that hold.
    Filter { next: usize, kept: Vec<usize> },
    /// `sort` on JSON fields: rows up to `next` have their values taken.
    SortValues { next: usize, keyed: Vec<KeyedRow> },
}

/// A row with the values of the JSON fields of a `sort`, taken once.
#[derive(Debug)]
struct KeyedRow {
    position: usize,
    values: Box<[SortValue]>,
}

/// Whether `command` needs chunks: a `filter`, a `sort` on a JSON field, or
/// a `stats` (BR3.5: one chunked pass, so the progress moves).
fn is_chunked(command: &CommandKind) -> bool {
    match command {
        CommandKind::Filter(_) | CommandKind::Stats(_) => true,
        CommandKind::Sort(keys) => keys.iter().any(|key| is_json(&key.field)),
        CommandKind::Fields(_) | CommandKind::Limit(_) => false,
    }
}

fn is_json(field: &FieldRef) -> bool {
    matches!(field, FieldRef::Json(_))
}

/// The fixed order of U2:BR4.4 between two events: newer first, then the
/// log group, the stream and the order in the stream, each ascending.
fn fixed_order(left: EvalItem<'_>, right: EvalItem<'_>) -> Ordering {
    right
        .event
        .timestamp
        .cmp(&left.event.timestamp)
        .then_with(|| left.log_group_name.cmp(right.log_group_name))
        .then_with(|| left.event.log_stream_name.cmp(&right.event.log_stream_name))
        .then_with(|| left.event.sequence.cmp(&right.event.sequence))
}

/// The positions of `items` in the fixed order. They come in time order, so
/// from the last one back they are newest first already, but for the order
/// of equal times; the stable sort does little more than check that.
fn fixed_order_of<I: EvalItems + ?Sized>(items: &I) -> Vec<usize> {
    let mut positions: Vec<usize> = (0..items.len()).rev().collect();
    positions.sort_by(
        |left, right| match (items.item(*left), items.item(*right)) {
            (Some(left), Some(right)) => fixed_order(left, right),
            _ => Ordering::Equal,
        },
    );
    positions
}

/// The sort value of key `index` of `row`: a fixed field read in place, a
/// JSON field from the values taken before.
fn key_of<'a, I: EvalItems + ?Sized>(
    keys: &[SortKey],
    index: usize,
    row: &'a KeyedRow,
    items: &'a I,
) -> SortKeyRef<'a> {
    let field = &keys[index].field;
    if is_json(field) {
        let slot = keys[..index]
            .iter()
            .filter(|key| is_json(&key.field))
            .count();
        return row
            .values
            .get(slot)
            .map_or(SortKeyRef::Missing, SortValue::as_key);
    }
    items
        .item(row.position)
        .map_or(SortKeyRef::Missing, |item| {
            SortKeyRef::of(read_field(field, item.event, item.log_group_name))
        })
}

/// Two rows by the keys of a `sort` (U2:BR4.1-BR4.3).
fn compare_keyed<I: EvalItems + ?Sized>(
    keys: &[SortKey],
    left: &KeyedRow,
    right: &KeyedRow,
    items: &I,
) -> Ordering {
    for (index, key) in keys.iter().enumerate() {
        let order = compare_sort_keys(
            key_of(keys, index, left, items),
            key_of(keys, index, right, items),
        );
        let order = match key.direction {
            SortDirection::Asc => order,
            SortDirection::Desc => order.reverse(),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

impl Evaluation {
    /// An evaluation of `plan` over `total` held events, nothing done yet.
    pub fn new(plan: &QueryPlan, total: usize) -> Self {
        Self {
            total,
            stage: Stage::Ordering,
            rows: Vec::new(),
            matched: total,
            passes_planned: plan
                .commands
                .iter()
                .filter(|command| is_chunked(&command.kind))
                .count(),
            passes_done: 0,
        }
    }

    /// Does what needs no chunk, up to the next chunk or the end.
    pub fn advance<I: EvalItems + ?Sized>(&mut self, plan: &QueryPlan, items: &I) {
        while !self.is_done() && !self.chunk_pending() {
            self.work(plan, items);
        }
    }

    /// Does the next chunk of work and what follows it without a chunk.
    pub fn step<I: EvalItems + ?Sized>(&mut self, plan: &QueryPlan, items: &I) {
        self.advance(plan, items);
        if !self.is_done() {
            self.work(plan, items);
        }
        self.advance(plan, items);
    }

    /// Whether the next piece of work is a chunk of rows. Starting a pass
    /// is not one.
    fn chunk_pending(&self) -> bool {
        match &self.stage {
            Stage::Command {
                pass: Pass::Filter { next, .. } | Pass::SortValues { next, .. },
                ..
            } => *next < self.rows.len(),
            Stage::Aggregate { next, .. } => *next < self.rows.len(),
            // Every piece of the commands after `stats` is one (U2:BR3.1).
            Stage::Finishing(_) => true,
            Stage::Columns { present, next, .. } => {
                *next < self.rows.len() && present.iter().any(|seen| !seen)
            }
            Stage::Ordering | Stage::Command { .. } | Stage::Done { .. } | Stage::Aggregated(_) => {
                false
            }
        }
    }

    /// One piece of work: a chunk of rows, or one step that needs none.
    fn work<I: EvalItems + ?Sized>(&mut self, plan: &QueryPlan, items: &I) {
        let stage = std::mem::replace(
            &mut self.stage,
            Stage::Done {
                columns: Vec::new(),
            },
        );
        self.stage = match stage {
            Stage::Ordering => {
                self.rows = fixed_order_of(items);
                Stage::Command {
                    index: 0,
                    pass: Pass::Fresh,
                }
            }
            Stage::Command { index, pass } => match plan.commands.get(index) {
                Some(command) => self.apply(&command.kind, index, pass, items),
                None => self.finish_commands(plan),
            },
            Stage::Aggregate { next, aggregator } => self.aggregate(next, aggregator, items),
            Stage::Finishing(finishing) => finish_aggregates(*finishing, plan.stats()),
            Stage::Columns {
                columns,
                present,
                next,
            } => self.check_columns(columns, present, next, items),
            done @ (Stage::Done { .. } | Stage::Aggregated(_)) => done,
        };
    }
}

/// One piece of the commands after `stats` of `stats`, then the cut once
/// they are all applied (U2:BR3.1, BR3.2).
fn finish_aggregates(mut finishing: Finishing, stats: Option<&StatsCommand>) -> Stage {
    // A plan without `stats` never comes here; then there is nothing after.
    let Some(stats) = stats else {
        return Stage::Aggregated(Box::new(finishing.cut(false)));
    };
    finishing.work(stats);
    if finishing.is_finished(stats) {
        return Stage::Aggregated(Box::new(finishing.cut(stats.has_row_limit())));
    }
    Stage::Finishing(Box::new(finishing))
}

impl Evaluation {

    /// One chunk of `stats` (BR2.9), or, after the last, the rows of the
    /// groups (BR3.5): ordering and making them is one short step that is
    /// not stopped. The commands before `stats` were applied. The commands
    /// after it are applied in later steps; without any, the rows are cut
    /// at once (U2:BR3.1, BR3.2). There is no column check (U1:BR3.2).
    fn aggregate<I: EvalItems + ?Sized>(
        &mut self,
        next: usize,
        mut aggregator: Box<StatsAggregator>,
        items: &I,
    ) -> Stage {
        let end = next.saturating_add(EVALUATION_CHUNK).min(self.rows.len());
        for position in &self.rows[next.min(end)..end] {
            if let Some(item) = items.item(*position) {
                aggregator.add(&EventFields::new(item.event, item.log_group_name));
            }
        }
        if end < self.rows.len() {
            return Stage::Aggregate {
                next: end,
                aggregator,
            };
        }
        self.passes_done += 1;
        let nothing_after = aggregator.stats().after.is_empty();
        let has_row_limit = aggregator.stats().has_row_limit();
        let finishing = Finishing::new(aggregator.finish());
        if nothing_after {
            return Stage::Aggregated(Box::new(finishing.cut(has_row_limit)));
        }
        Stage::Finishing(Box::new(finishing))
    }

    /// The next piece of work of command `index`.
    fn apply<I: EvalItems + ?Sized>(
        &mut self,
        command: &CommandKind,
        index: usize,
        pass: Pass,
        items: &I,
    ) -> Stage {
        let following = || Stage::Command {
            index: index + 1,
            pass: Pass::Fresh,
        };
        match (command, pass) {
            (CommandKind::Stats(stats), _) => Stage::Aggregate {
                next: 0,
                aggregator: Box::new(StatsAggregator::new(stats.clone())),
            },
            (CommandKind::Fields(_), _) => following(),
            (CommandKind::Limit(count), _) => {
                // U2:BR5.1: the first `count` rows; 0 keeps none (Loop-back
                // 3, p08: `limit 0` gave no row at the console).
                self.rows
                    .truncate(usize::try_from(*count).unwrap_or(usize::MAX));
                following()
            }
            (CommandKind::Filter(_), Pass::Fresh) => Stage::Command {
                index,
                pass: Pass::Filter {
                    next: 0,
                    kept: Vec::new(),
                },
            },
            (CommandKind::Filter(condition), Pass::Filter { next, mut kept }) => {
                let end = next.saturating_add(EVALUATION_CHUNK).min(self.rows.len());
                for position in &self.rows[next..end] {
                    let Some(item) = items.item(*position) else {
                        continue;
                    };
                    // One JSON read per row and filter (R-02).
                    let fields = EventFields::new(item.event, item.log_group_name);
                    if condition_holds(condition, &fields) {
                        kept.push(*position);
                    }
                }
                if end < self.rows.len() {
                    return Stage::Command {
                        index,
                        pass: Pass::Filter { next: end, kept },
                    };
                }
                self.rows = kept;
                self.matched = self.rows.len();
                self.passes_done += 1;
                following()
            }
            (CommandKind::Sort(_), Pass::Fresh) if is_chunked(command) => Stage::Command {
                index,
                pass: Pass::SortValues {
                    next: 0,
                    keyed: Vec::with_capacity(self.rows.len()),
                },
            },
            (CommandKind::Sort(keys), Pass::Fresh) => {
                // Only fixed fields: ordered at once, reading them in place.
                let mut keyed: Vec<KeyedRow> = self
                    .rows
                    .iter()
                    .map(|position| KeyedRow {
                        position: *position,
                        values: Box::new([]),
                    })
                    .collect();
                keyed.sort_by(|left, right| compare_keyed(keys, left, right, items));
                self.rows = keyed.into_iter().map(|row| row.position).collect();
                following()
            }
            (CommandKind::Sort(keys), Pass::SortValues { next, mut keyed }) => {
                let end = next.saturating_add(EVALUATION_CHUNK).min(self.rows.len());
                for position in &self.rows[next..end] {
                    keyed.push(KeyedRow {
                        position: *position,
                        values: json_sort_values(keys, items, *position),
                    });
                }
                if end < self.rows.len() {
                    return Stage::Command {
                        index,
                        pass: Pass::SortValues { next: end, keyed },
                    };
                }
                // The values are taken; ordering by them cannot be stopped
                // (measured in Step 17).
                keyed.sort_by(|left, right| compare_keyed(keys, left, right, items));
                self.rows = keyed.into_iter().map(|row| row.position).collect();
                self.passes_done += 1;
                following()
            }
            (_, Pass::Filter { .. } | Pass::SortValues { .. }) => following(),
        }
    }

    /// Every command was applied: the default cut (U2:BR5.2), then the
    /// column check (U2:BR3.4).
    fn finish_commands(&mut self, plan: &QueryPlan) -> Stage {
        if !plan.has_row_limit() {
            self.rows.truncate(DEFAULT_RESULT_LIMIT);
        }
        let columns = plan.columns();
        let present = columns.iter().map(|column| !is_json(column)).collect();
        Stage::Columns {
            columns,
            present,
            next: 0,
        }
    }

    /// One chunk of the column check, or its end.
    fn check_columns<I: EvalItems + ?Sized>(
        &mut self,
        columns: Vec<FieldRef>,
        mut present: Vec<bool>,
        next: usize,
        items: &I,
    ) -> Stage {
        let missing_any = present.iter().any(|seen| !seen);
        if !missing_any || next >= self.rows.len() {
            let columns = columns
                .into_iter()
                .zip(present)
                .filter_map(|(column, seen)| seen.then_some(column))
                .collect();
            return Stage::Done { columns };
        }
        let end = next.saturating_add(EVALUATION_CHUNK).min(self.rows.len());
        for position in &self.rows[next..end] {
            let Some(item) = items.item(*position) else {
                continue;
            };
            let fields = EventFields::new(item.event, item.log_group_name);
            for (column, seen) in columns.iter().zip(present.iter_mut()) {
                if !*seen && fields.value(column) != FieldValue::Missing {
                    *seen = true;
                }
            }
        }
        Stage::Columns {
            columns,
            present,
            next: end,
        }
    }

    /// Whether every command was applied and the columns checked, or the
    /// aggregate rows made.
    pub fn is_done(&self) -> bool {
        matches!(self.stage, Stage::Done { .. } | Stage::Aggregated(_))
    }

    /// Events evaluated so far: the held events times the share of the
    /// chunked passes (`filter`s and `sort`s on JSON fields) done.
    pub fn evaluated(&self) -> usize {
        if self.passes_planned == 0 || self.is_done() {
            return if self.is_done() { self.total } else { 0 };
        }
        let (next, length) = match &self.stage {
            Stage::Command {
                pass: Pass::Filter { next, .. } | Pass::SortValues { next, .. },
                ..
            }
            | Stage::Aggregate { next, .. } => (*next, self.rows.len()),
            _ => (0, 1),
        };
        // done/planned of the total, with the current pass's share, in
        // whole numbers so that the counts come out exact.
        let total = self.total as u128;
        let done = self.passes_done as u128 * total * length.max(1) as u128 + next as u128 * total;
        let shown = done / (self.passes_planned as u128 * length.max(1) as u128);
        usize::try_from(shown).unwrap_or(usize::MAX).min(self.total)
    }

    /// Events to evaluate in all: every held event.
    pub fn total(&self) -> usize {
        self.total
    }

    /// The output, once done.
    pub fn into_output(self) -> Option<EvalOutput> {
        let columns = match self.stage {
            Stage::Done { columns } => columns,
            Stage::Aggregated(done) => {
                let done = *done;
                return Some(EvalOutput::Aggregates(AggregatesOutput {
                    columns: done.columns,
                    rows: done.rows,
                    evaluated_count: self.total,
                    matched_count: done.matched,
                    truncated_at: done.truncated_at,
                }));
            }
            _ => return None,
        };
        // U2:BR5.3: a cut is shown only when rows are left; `limit 0` leaves
        // none, so nothing is shown as cut (Loop-back 3, p08).
        let truncated_at =
            (!self.rows.is_empty() && self.rows.len() < self.matched).then_some(self.rows.len());
        Some(EvalOutput::Events(EventsOutput {
            rows: self.rows,
            columns,
            matched_count: self.matched,
            truncated_at,
            evaluated_count: self.total,
        }))
    }
}

/// The values of the JSON keys of a `sort` for the event at `position`, its
/// message read once (R-02).
fn json_sort_values<I: EvalItems + ?Sized>(
    keys: &[SortKey],
    items: &I,
    position: usize,
) -> Box<[SortValue]> {
    let Some(item) = items.item(position) else {
        return Box::new([]);
    };
    let fields = EventFields::new(item.event, item.log_group_name);
    keys.iter()
        .filter(|key| is_json(&key.field))
        .map(|key| SortValue::of(fields.value(&key.field)))
        .collect()
}

/// One result row, a copy that refers to no holdings (entities.md
/// ResultRow, 261017-insights-query U3:BR3.2, 261025-insights-stats
/// ADR-001, DD review R-01): an event with its log group, or the cells of an
/// aggregate row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultRow {
    /// An event row.
    Event {
        /// The event as held.
        event: LogEvent,
        /// Its log group, the value of `@log`.
        log_group_name: String,
    },
    /// An aggregate row: one cell per column (BR3.3).
    Aggregate {
        /// The cells, typed; their text is made when shown (BR4.2).
        cells: Vec<CellValue>,
    },
}

impl ResultRow {
    /// The event of an event row.
    pub fn event(&self) -> Option<&LogEvent> {
        match self {
            ResultRow::Event { event, .. } => Some(event),
            ResultRow::Aggregate { .. } => None,
        }
    }

    /// The log group of an event row.
    pub fn log_group_name(&self) -> Option<&str> {
        match self {
            ResultRow::Event { log_group_name, .. } => Some(log_group_name),
            ResultRow::Aggregate { .. } => None,
        }
    }

    /// The cells of an aggregate row.
    pub fn cells(&self) -> Option<&[CellValue]> {
        match self {
            ResultRow::Aggregate { cells } => Some(cells),
            ResultRow::Event { .. } => None,
        }
    }

    /// The fields of an event row, its message not read yet.
    pub fn event_fields(&self) -> Option<EventFields<'_>> {
        match self {
            ResultRow::Event {
                event,
                log_group_name,
            } => Some(EventFields::new(event, log_group_name)),
            ResultRow::Aggregate { .. } => None,
        }
    }
}

/// The columns of a result (261025-insights-stats BR3.6, FR review R-02):
/// the fields of an events result, from which its cells are made, or the
/// names of an aggregates result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultColumns {
    /// The fields, in order (U2:BR3.2-BR3.4).
    Events(Vec<FieldRef>),
    /// The names, keys then aggregates (BR3.3).
    Aggregates(Vec<String>),
}

impl ResultColumns {
    /// The column headings: the fields' names as written, or the names.
    pub fn names(&self) -> Vec<String> {
        match self {
            ResultColumns::Events(fields) => fields
                .iter()
                .map(|field| field.name().to_string())
                .collect(),
            ResultColumns::Aggregates(names) => names.clone(),
        }
    }

    /// The shape these columns belong to.
    pub fn shape(&self) -> ResultShape {
        match self {
            ResultColumns::Events(_) => ResultShape::Events,
            ResultColumns::Aggregates(_) => ResultShape::Aggregates,
        }
    }

    /// The fields of an events result; none for aggregates.
    pub fn fields(&self) -> &[FieldRef] {
        match self {
            ResultColumns::Events(fields) => fields,
            ResultColumns::Aggregates(_) => &[],
        }
    }
}

/// The result of one finished evaluation (entities.md QueryResult). Since
/// 261017-insights-query U3 the rows are copies of the events in the
/// result's order (U2:BR3.1, BR4.4), at most 100,000 of them since the
/// result is cut by `limit` or at 10,000 (U3:BR3.2), so the result can be
/// read whatever the holdings become: a later run that fetches more or
/// discards its staging events does not change it (U3:BR3.4). Since
/// 261025-insights-stats U1 a result has a shape: event rows, or the
/// aggregate rows of a `stats`, which are copies too (ADR-001, BR3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    /// The run that made it; the screen tells a result apart by it alone
    /// (the rows are copies and depend on no holdings, U3:BR3.2,
    /// 261020-carryover-fixes FR2.3.1).
    pub run_id: u64,
    /// Events evaluated.
    pub evaluated_count: usize,
    rows: Vec<ResultRow>,
    columns: ResultColumns,
    matched_count: usize,
    truncated_at: Option<usize>,
}

impl QueryResult {
    /// The result of run `run_id` with the two default columns, nothing
    /// cut.
    pub fn new(run_id: u64, evaluated_count: usize, rows: Vec<ResultRow>) -> Self {
        Self {
            run_id,
            evaluated_count,
            matched_count: rows.len(),
            rows,
            columns: ResultColumns::Events(vec![FieldRef::Timestamp, FieldRef::Message]),
            truncated_at: None,
        }
    }

    /// The result `output` of run `run_id`, with `rows`, the copies of the
    /// events at `output.rows`, in that order.
    pub fn from_output(run_id: u64, output: EventsOutput, rows: Vec<ResultRow>) -> Self {
        Self {
            run_id,
            evaluated_count: output.evaluated_count,
            rows,
            columns: ResultColumns::Events(output.columns),
            matched_count: output.matched_count,
            truncated_at: output.truncated_at,
        }
    }

    /// The aggregate rows `output` of run `run_id` (U1:BR3.1, BR3.4), with
    /// what was cut (U2:BR3.2).
    pub fn from_aggregates(run_id: u64, output: AggregatesOutput) -> Self {
        let rows: Vec<ResultRow> = output
            .rows
            .into_iter()
            .map(|cells| ResultRow::Aggregate { cells })
            .collect();
        Self {
            run_id,
            evaluated_count: output.evaluated_count,
            matched_count: output.matched_count,
            rows,
            columns: ResultColumns::Aggregates(output.columns),
            truncated_at: output.truncated_at,
        }
    }

    /// Events or aggregates (BR3.1).
    pub fn shape(&self) -> ResultShape {
        self.columns.shape()
    }

    /// Number of result rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The columns (U2:BR3.2-BR3.4, BR3.6).
    pub fn columns(&self) -> &ResultColumns {
        &self.columns
    }

    /// Rows that matched before the cut (review R-08).
    pub fn matched_count(&self) -> usize {
        self.matched_count
    }

    /// Rows shown when a cut happened (U2:BR5.3).
    pub fn truncated_at(&self) -> Option<usize> {
        self.truncated_at
    }

    /// At most `limit` rows from `offset`, when the screen asks for this
    /// result's run; nothing otherwise, and the screen waits for the newer
    /// state (U1:BR6.2; U3:BR3.2 checks the run number where U1:BR6.4
    /// checked the holdings epoch).
    pub fn window(&self, run_id: u64, offset: usize, limit: usize) -> &[ResultRow] {
        if run_id != self.run_id {
            return &[];
        }
        let start = offset.min(self.rows.len());
        let end = start.saturating_add(limit).min(self.rows.len());
        &self.rows[start..end]
    }

    /// Result row `index` of this run, or `None` (U2:BR7.3, BR7.4).
    pub fn row_at(&self, run_id: u64, index: usize) -> Option<&ResultRow> {
        self.window(run_id, index, 1).first()
    }
}

/// The cells of one result row for `columns` (U2:BR7.1): `@timestamp` as
/// `format_time` writes it, `toMillis(@timestamp)` in milliseconds, the
/// other values as [`EventFields::display`] writes them, `None` without a
/// value.
pub fn row_cells(
    columns: &[FieldRef],
    fields: &EventFields<'_>,
    format_time: &dyn Fn(i64) -> String,
) -> Vec<Option<String>> {
    columns
        .iter()
        .map(|column| match column {
            FieldRef::Timestamp => Some(format_time(fields.event().timestamp)),
            other => fields.display(other),
        })
        .collect()
}

/// The cells of one result row as the table, the opened row and the export
/// show them (U2:BR7.1, 261025-insights-stats BR4.2, BR4.4): an event row's
/// for the fields of `columns` ([`row_cells`]), an aggregate row's cells
/// written by [`cell_text`], times by `format_time`.
pub fn result_row_cells(
    columns: &ResultColumns,
    row: &ResultRow,
    format_time: &dyn Fn(i64) -> String,
) -> Vec<Option<String>> {
    match row {
        ResultRow::Event {
            event,
            log_group_name,
        } => row_cells(
            columns.fields(),
            &EventFields::new(event, log_group_name),
            format_time,
        ),
        ResultRow::Aggregate { cells } => cells
            .iter()
            .map(|cell| cell_text(cell, format_time))
            .collect(),
    }
}

/// The fields and values of one opened row (U2:BR7.3): `@timestamp`,
/// `@log`, `@logStream` and `@message`, then the JSON fields in name order.
pub fn row_detail(
    fields: &EventFields<'_>,
    format_time: &dyn Fn(i64) -> String,
) -> Vec<(String, Option<String>)> {
    let fixed = [
        FieldRef::Timestamp,
        FieldRef::Log,
        FieldRef::LogStream,
        FieldRef::Message,
    ];
    let mut detail: Vec<(String, Option<String>)> = fixed
        .iter()
        .zip(row_cells(&fixed, fields, format_time))
        .map(|(field, value)| (field.name().to_string(), value))
        .collect();
    detail.extend(fields.json_fields());
    detail
}

/// Evaluates `plan` on `items` (the whole holdings in time order) chunk by
/// chunk. Before each chunk `stop` is asked; when it says so, what was found
/// is dropped and `None` is returned. `progress` hears the evaluated and
/// total counts after each chunk (U1:BR2.4, BR5.8).
pub fn evaluate<I: EvalItems + ?Sized>(
    plan: &QueryPlan,
    items: &I,
    mut stop: impl FnMut() -> bool,
    mut progress: impl FnMut(usize, usize),
) -> Option<EvalOutput> {
    let mut evaluation = Evaluation::new(plan, items.len());
    evaluation.advance(plan, items);
    while !evaluation.is_done() {
        if stop() {
            return None;
        }
        evaluation.step(plan, items);
        progress(evaluation.evaluated(), evaluation.total());
    }
    evaluation.into_output()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::instant::resolve_date_texts;
    use crate::query::parser::parse_query;
    use crate::query::plan::JsonPath;

    const GROUP: &str = "/aws/lambda/orders";

    fn event(timestamp: i64, stream: &str, sequence: u64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn query(text: &str) -> QueryPlan {
        resolve_date_texts(parse_query(text).unwrap(), chrono_tz::Asia::Tokyo).unwrap()
    }

    fn holds(text: &str, event: &LogEvent) -> bool {
        matches(
            &query(text),
            &EvalItem {
                event,
                log_group_name: GROUP,
            },
        )
    }

    fn items(events: &[LogEvent]) -> Vec<EvalItem<'_>> {
        events
            .iter()
            .map(|event| EvalItem {
                event,
                log_group_name: GROUP,
            })
            .collect()
    }

    fn output(text: &str, events: &[LogEvent]) -> EventsOutput {
        events_of(evaluate(
            &query(text),
            items(events).as_slice(),
            || false,
            |_, _| {},
        ))
    }

    /// The events output of an evaluation that ended (no `stats`).
    fn events_of(output: Option<EvalOutput>) -> EventsOutput {
        match output {
            Some(EvalOutput::Events(events)) => events,
            other => panic!("not an events output: {other:?}"),
        }
    }

    /// The result rows of `text`, newest first without a `sort` (U2:BR4.4).
    fn rows(text: &str, events: &[LogEvent]) -> Vec<usize> {
        output(text, events).rows
    }

    fn json(keys: &[&str]) -> FieldRef {
        FieldRef::Json(JsonPath::new(
            keys.iter().map(|key| (*key).to_string()).collect(),
            keys.join("."),
        ))
    }

    #[test]
    fn the_acceptance_query_keeps_errors_that_are_not_timeouts() {
        // Acceptance criterion of FR5.3.
        let events = [
            event(1, "a", 0, "ERROR connection reset"),
            event(2, "a", 1, "ERROR timeout after 3s"),
            event(3, "b", 0, "INFO started"),
            event(4, "b", 1, "error lower case"),
            event(5, "a", 2, "ERROR\nsecond line timeout"),
        ];
        assert_eq!(
            rows(
                "filter @message like /ERROR/ and @message not like /timeout/",
                &events
            ),
            vec![0]
        );
    }

    #[test]
    fn strings_compare_whole_and_texts_not_of_digits_have_no_order() {
        // U1:BR5.1 for `=` and `!=`; the order of two texts that are no
        // numbers follows the console check of Loop-back 2 (p03).
        let event = event(1, "web-2", 0, "Hello");
        assert!(holds("filter @message = 'Hello'", &event));
        assert!(!holds("filter @message = 'hello'", &event), "case counts");
        assert!(!holds("filter @message = 'Hell'", &event), "the whole text");
        assert!(holds("filter @message != 'hello'", &event));
        assert!(holds("filter @log = '/aws/lambda/orders'", &event));
        for op in ["<", "<=", ">", ">="] {
            for value in ["'web-1'", "'web-2'", "'web-3'"] {
                let text = format!("filter @logStream {op} {value}");
                assert!(!holds(&text, &event), "{text:?}");
            }
        }
        assert!(!holds("filter @message < 'a'", &event));
    }

    // ---- 261017-insights-query U2:BR1.5, BR1.6 (Q1a 5c-5g, 6b-6h) -------
    //
    // These replace U1:BR5.1's provisional "a text field and a number never
    // meet a condition except !=". Q1a 6a (`"10" as a, 9 as b | filter a >
    // b`) compared a text field with a number field made by `as`, which the
    // console refused as a type mismatch. The app has no `as`; since
    // Loop-back 2 it compares two fields of the message as it compares a
    // field with a value (p05), so a JSON text "10" against a JSON number 9
    // compares as numbers (provisional, see the p05 tests).

    /// An event whose message is `json`, with `errorCode` on some events
    /// only, as in the console check.
    fn json_event(json: &str) -> LogEvent {
        event(1, "web-2", 0, json)
    }

    #[test]
    fn q1a_5c_5d_a_missing_field_meets_only_not_equal_of_the_comparisons() {
        let without = json_event(r#"{"eventName":"AssumeRole"}"#);
        let plain = event(1, "s", 0, "not json at all");
        for event in [&without, &plain] {
            assert!(!holds("filter errorCode = \"zzz\"", event), "5c");
            assert!(holds("filter errorCode != \"zzz\"", event), "5d");
            for op in ["<", "<=", ">", ">="] {
                assert!(!holds(&format!("filter errorCode {op} 1"), event), "{op}");
            }
        }
    }

    #[test]
    fn q1a_5f_5g_a_missing_field_meets_not_like_and_not_in_only() {
        let without = json_event(r#"{"eventName":"AssumeRole"}"#);
        assert!(holds("filter errorCode not like /zzz/", &without), "5f");
        assert!(holds("filter errorCode not in [\"zzz\"]", &without), "5g");
        assert!(!holds("filter errorCode like /./", &without));
        assert!(!holds("filter errorCode in [\"zzz\"]", &without));
        assert!(!holds("filter errorCode =~ /.*/", &without));
        let with = json_event(r#"{"errorCode":"AccessDenied"}"#);
        assert!(holds("filter errorCode like /Denied/", &with));
        assert!(!holds("filter errorCode not like /Denied/", &with));
    }

    #[test]
    fn q1a_6b_a_text_of_digits_equals_the_number() {
        assert!(holds("filter @message = 10", &event(1, "s", 0, "10")));
        let json = json_event(r#"{"code":"10"}"#);
        assert!(holds("filter code = 10", &json));
        assert!(!holds("filter code != 10", &json));
        assert!(holds("filter code in [9, 10]", &json));
    }

    #[test]
    fn q1a_6c_6d_6e_6g_6h_a_text_not_of_digits_meets_only_not_equal_with_a_number() {
        let abc = event(1, "web-2", 0, "abc");
        assert!(!holds("filter @message > 5", &abc), "6c");
        assert!(!holds("filter @message < 5", &abc), "6d");
        assert!(holds("filter @message != 5", &abc), "6e");
        assert!(!holds("filter @logStream = 1", &abc), "6g");
        assert!(holds("filter @logStream != 1", &abc), "6h");
    }

    #[test]
    fn q1a_6f_two_texts_of_digits_compare_as_numbers() {
        let ten = event(1, "s", 0, "10");
        assert!(holds("filter @message > \"9\"", &ten), "6f");
        assert!(!holds("filter @message < \"9\"", &ten));
        assert!(holds("filter @message = \"10\"", &ten));
    }

    #[test]
    fn json_numbers_compare_as_numbers() {
        // FR4.2's acceptance criterion with a comparison.
        let error = json_event(r#"{"level":"error","http":{"status":500}}"#);
        assert!(holds(
            "filter level = \"error\" and http.status >= 500",
            &error
        ));
        assert!(!holds("filter http.status < 500", &error));
        assert!(holds("filter http.status in [404, 500]", &error));
    }

    #[test]
    fn provisional_a_text_of_digits_orders_as_a_number_against_a_number() {
        // Not checked at the console (review R-01); U4 compares it.
        let ten = event(1, "s", 0, "10");
        assert!(holds("filter @message > 9", &ten));
        assert!(!holds("filter @message < 9", &ten));
        assert!(holds("filter @message <= 10.0", &ten));
        assert!(holds("filter @message > -1", &event(1, "s", 0, "-0.5")));
        assert!(
            !holds("filter @message > 1", &event(1, "s", 0, " 10")),
            "blanks: no number"
        );
        assert!(!holds("filter @message > 1", &event(1, "s", 0, "1e3")));
    }

    #[test]
    fn p03_two_texts_not_of_digits_never_meet_an_order() {
        // Checked at the console in Loop-back 2: `eventName > "M"` gave no
        // row. `=` and `!=` still compare the whole texts.
        let event = json_event(r#"{"eventName":"AssumeRole","eventVersion":"1.08"}"#);
        for op in ["<", "<=", ">", ">="] {
            for value in ["\"M\"", "\"A\"", "\"AssumeRole\""] {
                let text = format!("filter eventName {op} {value}");
                assert!(!holds(&text, &event), "{text:?}");
            }
        }
        assert!(holds("filter eventName = \"AssumeRole\"", &event));
        assert!(holds("filter eventName != \"M\"", &event));
        assert!(!holds("filter eventName != \"AssumeRole\"", &event));
        // A text of digits against a number is unchanged (6b, 6f).
        assert!(holds("filter eventVersion > 1.05", &event));
        assert!(holds("filter eventVersion > \"1.05\"", &event));
        // A boolean is the text true or false: no order either.
        let flags = json_event(r#"{"ok":true}"#);
        assert!(!holds("filter ok > \"a\"", &flags));
        assert!(!holds("filter ok <= \"true\"", &flags));
    }

    #[test]
    fn p12_a_text_not_of_digits_against_a_text_of_digits_never_meets_an_order() {
        // Checked at the console in Loop-back 3 (p12): `eventName > "5"`
        // gave no row. It replaces the provisional Unicode order; p03 now
        // holds when either side is no number.
        let event = json_event(r#"{"eventName":"AssumeRole","eventVersion":"1.08","five":"5"}"#);
        for op in ["<", "<=", ">", ">="] {
            let text = format!("filter eventName {op} \"5\"");
            assert!(!holds(&text, &event), "{text:?}");
            // The same with the texts the other way round, as fields.
            for (left, right) in [("eventName", "five"), ("five", "eventName")] {
                let text = format!("filter {left} {op} {right}");
                assert!(!holds(&text, &event), "{text:?}");
            }
        }
        let v10 = self::event(1, "s", 0, "v10");
        assert!(!holds("filter @message > \"9\"", &v10));
        assert!(!holds("filter @message < \"9\"", &v10));
        let nine = self::event(1, "s", 0, "9");
        assert!(!holds("filter @message < \"v10\"", &nine));
        // `=` and `!=` still compare the whole texts.
        assert!(!holds("filter eventName = \"5\"", &event));
        assert!(holds("filter eventName != \"5\"", &event));
        assert!(holds("filter eventName != five", &event));
        // Both sides numbers: unchanged (p01, p02, 6b, 6f).
        assert!(holds("filter eventVersion > 1.05", &event));
        assert!(holds("filter eventVersion > \"1.05\"", &event));
        assert!(holds("filter eventVersion < five", &event));
    }

    #[test]
    fn provisional_booleans_and_json_numbers_against_texts() {
        // Not checked at the console (review R-01): a boolean is the text
        // true or false; a JSON number against a text of digits is a
        // number.
        let flags = json_event(r#"{"ok":true,"status":200}"#);
        assert!(holds("filter ok = \"true\"", &flags));
        assert!(!holds("filter ok = \"false\"", &flags));
        assert!(holds("filter ok like /tru/", &flags));
        assert!(holds("filter status = \"200\"", &flags));
        assert!(holds("filter status like /^20/", &flags));
        assert!(!holds("filter status = \"ok\"", &flags));
        assert!(holds("filter status != \"ok\"", &flags));
    }

    #[test]
    fn p04_a_true_boolean_equals_the_text_true_though_it_is_shown_as_1() {
        // Checked at the console in Loop-back 2: `readOnly = "true"` kept
        // the 830 rows whose readOnly is true; the cell shows 1, but the
        // comparison, like and =~ still see the text true.
        let events = [
            json_event(r#"{"readOnly":true}"#),
            json_event(r#"{"readOnly":false}"#),
            json_event(r#"{"eventName":"AssumeRole"}"#),
        ];
        assert_eq!(rows("filter readOnly = \"true\"", &events), vec![0]);
        assert!(!holds("filter readOnly = 1", &events[0]));
        assert!(holds("filter readOnly like /true/", &events[0]));
        assert!(holds("filter readOnly =~ /^false$/", &events[1]));
    }

    // ---- Loop-back 2 (p05): two fields compared --------------------------

    #[test]
    fn p05_an_unquoted_true_is_a_field_that_no_event_has() {
        // The console gave no row for `readOnly = true` (p05).
        let events = [
            json_event(r#"{"readOnly":true}"#),
            json_event(r#"{"readOnly":false}"#),
        ];
        assert!(rows("filter readOnly = true", &events).is_empty());
        assert!(rows("filter readOnly = false", &events).is_empty());
        // A field named true is read like any other.
        let named = json_event(r#"{"readOnly":true,"true":"true"}"#);
        assert!(holds("filter readOnly = true", &named));
    }

    #[test]
    fn p05_two_fields_meet_a_comparison_as_their_values_do() {
        let events = [
            json_event(r#"{"a":"x","b":"x","n":5,"m":7,"t":"5"}"#),
            json_event(r#"{"a":"x","b":"y","n":9,"m":7,"t":"10"}"#),
        ];
        let (same, other) = (&events[0], &events[1]);
        assert_eq!(rows("filter a = b", &events), vec![0]);
        assert_eq!(rows("filter a != b", &events), vec![1]);
        assert_eq!(rows("filter n < m", &events), vec![0]);
        assert_eq!(rows("filter n >= m", &events), vec![1]);
        // A text of digits and a number compare as numbers (U2:BR1.6, 6b).
        assert!(holds("filter t = n", same));
        assert!(holds("filter t > m", other));
        // Two texts that are no numbers have no order (p03).
        assert!(!holds("filter a < b", other));
        assert!(!holds("filter b > a", other));
    }

    #[test]
    fn p11_a_missing_right_field_meets_no_comparison_not_even_not_equal() {
        // Checked at the console in Loop-back 3 (p11): `eventName !=
        // errorCode` kept only the 73 events that have errorCode.
        let events = [
            json_event(r#"{"eventName":"AssumeRole"}"#),
            json_event(r#"{"eventName":"GetObject","errorCode":"AccessDenied"}"#),
            json_event(r#"{"eventName":"Same","errorCode":"Same"}"#),
        ];
        assert_eq!(rows("filter eventName != errorCode", &events), vec![1]);
        assert_eq!(rows("filter eventName = errorCode", &events), vec![2]);
        let event = json_event(r#"{"a":"x","n":1}"#);
        for op in ["=", "!=", "<", "<=", ">", ">="] {
            assert!(!holds(&format!("filter a {op} zzz"), &event), "a {op}");
            assert!(!holds(&format!("filter n {op} zzz"), &event), "n {op}");
        }
    }

    #[test]
    fn provisional_p11_a_missing_left_field_meets_no_comparison_either() {
        // Not checked at the console: treated as a missing right field is.
        let event = json_event(r#"{"a":"x","n":1}"#);
        for op in ["=", "!=", "<", "<=", ">", ">="] {
            assert!(!holds(&format!("filter zzz {op} a"), &event), "{op} a");
            assert!(!holds(&format!("filter zzz {op} n"), &event), "{op} n");
            assert!(!holds(&format!("filter zzz {op} yyy"), &event), "{op}");
        }
    }

    #[test]
    fn p11_a_missing_field_against_a_value_still_meets_not_equal() {
        // U2:BR1.5 (Q1a 5c, 5d) is unchanged for a value on the right.
        let event = json_event(r#"{"eventName":"AssumeRole"}"#);
        assert!(holds("filter errorCode != \"zzz\"", &event));
        assert!(holds("filter errorCode != 1", &event));
        assert!(!holds("filter errorCode = \"zzz\"", &event));
    }

    #[test]
    fn p05_fixed_fields_and_the_time_compare_with_each_other() {
        let event = event(1_756_053_806_000, "orders", 0, r#"{"at":1756053806000}"#);
        assert!(holds("filter @logStream != @log", &event));
        assert!(!holds("filter @logStream = @log", &event));
        let named = self::event(5, GROUP, 0, "x");
        assert!(holds("filter @logStream = @log", &named));
        assert!(holds("filter @timestamp = toMillis(@timestamp)", &event));
        assert!(holds("filter at = toMillis(@timestamp)", &event));
        assert!(holds("filter at <= @timestamp", &event));
        assert!(!holds("filter at < @timestamp", &event));
        assert!(holds("filter @message != @logStream", &event));
    }

    #[test]
    fn like_contains_with_case_and_regular_expressions_find_a_part() {
        // U1:BR5.3, BR5.4.
        let event = event(1, "s", 0, "Payment FAILED for order 17");
        assert!(holds("filter @message like 'FAILED'", &event));
        assert!(!holds("filter @message like 'failed'", &event));
        assert!(holds("filter @message not like 'failed'", &event));
        assert!(!holds("filter @message like /failed/", &event));
        assert!(holds("filter @message like /(?i)failed/", &event));
        assert!(holds("filter @message =~ /order \\d+$/", &event));
        assert!(!holds("filter @message =~ /^order/", &event));
        assert!(holds("filter @message not like /^order/", &event));
    }

    #[test]
    fn in_needs_one_whole_equal_value() {
        // U1:BR5.5.
        let event = event(1, "web-2", 0, "x");
        assert!(holds("filter @logStream in ['web-1', 'web-2']", &event));
        assert!(!holds("filter @logStream in ['web', 'eb-2']", &event));
        assert!(holds("filter @logStream not in ['web', 2]", &event));
        assert!(!holds("filter @logStream not in ['web-2']", &event));
        let timed = event_at(1_756_053_806_000);
        assert!(holds("filter @timestamp in [1, 1756053806000]", &timed));
        assert!(holds(
            "filter @timestamp in ['2025-08-25 01:43:26']",
            &timed
        ));
    }

    fn event_at(timestamp: i64) -> LogEvent {
        event(timestamp, "s", 0, "m")
    }

    #[test]
    fn the_timestamp_compares_as_milliseconds_with_or_without_to_millis() {
        // U1:BR5.10, BR5.9: numbers, decimals and date texts; the rows come
        // newest first (U2:BR4.4).
        let events = [
            event_at(1_756_053_805_999),
            event_at(1_756_053_806_000),
            event_at(1_756_053_806_001),
        ];
        let numeric = rows("filter @timestamp > 1756053806000", &events);
        assert_eq!(numeric, vec![2]);
        assert_eq!(
            rows("filter toMillis(@timestamp) > 1756053806000", &events),
            numeric
        );
        assert_eq!(
            rows("filter @timestamp > '2025-08-25 01:43:26'", &events),
            numeric,
            "Tokyo wall-clock time"
        );
        assert_eq!(
            rows("filter @timestamp >= \"2025-08-24T16:43:26.000Z\"", &events),
            vec![2, 1]
        );
        assert_eq!(
            rows("filter @timestamp < 1756053806000.5", &events),
            vec![1, 0]
        );
        assert_eq!(rows("filter @timestamp = 1756053806000", &events), vec![1]);
    }

    #[test]
    fn several_filters_keep_what_meets_all_of_them() {
        // U1:BR3.2, BR5.2.
        let events = [
            event(1, "a", 0, "ERROR x"),
            event(2, "b", 0, "ERROR y"),
            event(3, "a", 1, "INFO z"),
        ];
        assert_eq!(
            rows(
                "filter @message like 'ERROR' | filter @logStream = 'a'",
                &events
            ),
            vec![0]
        );
        assert_eq!(
            rows(
                "filter @logStream = 'b' or @message like 'INFO' | filter not (@message like 'z')",
                &events
            ),
            vec![1]
        );
    }

    #[test]
    fn chunks_report_progress_and_a_stop_drops_what_was_found() {
        // U1:BR5.8, BR2.7.
        let events: Vec<LogEvent> = (0..25_000u64)
            .map(|i| event(i64::try_from(i).unwrap(), "s", i, "ERROR"))
            .collect();
        let items = items(&events);
        let plan = query("filter @message like 'ERROR'");
        let mut seen = Vec::new();
        let all = evaluate(
            &plan,
            items.as_slice(),
            || false,
            |done, total| seen.push((done, total)),
        );
        assert_eq!(
            all.map(|output| events_of(Some(output)).matched_count),
            Some(25_000)
        );
        assert_eq!(
            seen,
            vec![(10_000, 25_000), (20_000, 25_000), (25_000, 25_000)]
        );

        let mut asked = 0;
        let mut seen = Vec::new();
        let stopped = evaluate(
            &plan,
            items.as_slice(),
            || {
                asked += 1;
                asked > 1
            },
            |done, total| seen.push((done, total)),
        );
        assert_eq!(stopped, None, "the first chunk's rows are dropped");
        assert_eq!(seen, vec![(10_000, 25_000)]);

        let none: &[EvalItem<'_>] = &[];
        let empty = evaluate(&plan, none, || true, |_, _| panic!("no chunk"));
        assert_eq!(
            empty.map(|output| events_of(Some(output)).rows),
            Some(Vec::new()),
            "nothing to evaluate"
        );
    }

    // ---- 261017-insights-query U2:BR3.1-BR3.4, BR4.1-BR4.4, BR5.1, BR5.2

    /// Events at the given seconds on stream `s`, the JSON of each message
    /// given.
    fn timed_json(entries: &[(i64, &str)]) -> Vec<LogEvent> {
        entries
            .iter()
            .enumerate()
            .map(|(index, (at, message))| event(*at, "s", index as u64, message))
            .collect()
    }

    #[test]
    fn q1b_1d_without_sort_rows_come_newest_first_and_equal_times_in_the_fixed_order() {
        // U2:BR4.4: equal times by stream, then order in the stream.
        let events = [
            event(1, "a", 0, "m"),
            event(2, "a", 1, "m"),
            event(2, "b", 0, "m"),
            event(2, "b", 1, "m"),
            event(3, "a", 2, "m"),
        ];
        assert_eq!(rows("fields @message", &events), vec![4, 1, 2, 3, 0]);
    }

    #[test]
    fn q1b_1a_sort_without_a_direction_is_ascending() {
        let events = [
            event(1, "a", 0, "m"),
            event(3, "a", 1, "m"),
            event(2, "b", 0, "m"),
        ];
        assert_eq!(rows("sort @timestamp", &events), vec![0, 2, 1], "1a");
        assert_eq!(rows("sort @timestamp asc", &events), vec![0, 2, 1], "1b");
        assert_eq!(rows("sort @timestamp desc", &events), vec![1, 2, 0], "1c");
    }

    #[test]
    fn sort_by_json_fields_keeps_the_fixed_order_for_equal_values() {
        // U2:BR4.1-BR4.4: keys in order, missing values smallest, stable.
        let events = timed_json(&[
            (1, r#"{"level":"warn","n":2}"#),
            (2, r#"{"level":"error","n":10}"#),
            (3, r#"{"n":1}"#),
            (4, r#"{"level":"warn","n":1}"#),
            (5, r#"{"level":"error","n":10}"#),
        ]);
        assert_eq!(
            rows("sort level, n desc", &events),
            vec![2, 4, 1, 0, 3],
            "missing first; error 10 twice, newest first; warn 2 then 1"
        );
        assert_eq!(
            rows("sort level desc", &events),
            vec![3, 0, 4, 1, 2],
            "descending is the reverse, ties still newest first"
        );
        assert_eq!(rows("sort n", &events), vec![3, 2, 0, 4, 1]);
    }

    #[test]
    fn commands_apply_in_the_order_written() {
        // U2:BR3.1: `limit 2 | filter …` filters the two newest only.
        let events = timed_json(&[
            (1, r#"{"level":"error"}"#),
            (2, r#"{"level":"error"}"#),
            (3, r#"{"level":"info"}"#),
            (4, r#"{"level":"error"}"#),
        ]);
        assert_eq!(rows("limit 2 | filter level = 'error'", &events), vec![3]);
        assert_eq!(
            rows("filter level = 'error' | limit 2", &events),
            vec![3, 1]
        );
        assert_eq!(
            rows("sort @timestamp | limit 2 | sort @timestamp desc", &events),
            vec![1, 0]
        );
    }

    #[test]
    fn fields_add_their_columns_once_in_order_and_the_default_is_two() {
        // U2:BR3.2, BR3.3 (Q1a 2a-2c).
        let events = timed_json(&[(1, r#"{"a":1,"b":2}"#)]);
        assert_eq!(
            output("fields a | filter a = 1 | fields @logStream, a, b", &events).columns,
            vec![json(&["a"]), FieldRef::LogStream, json(&["b"])]
        );
        assert_eq!(
            output("filter a = 1", &events).columns,
            vec![FieldRef::Timestamp, FieldRef::Message]
        );
    }

    #[test]
    fn q1a_5j_a_column_without_any_value_in_the_result_is_left_out() {
        // U2:BR3.4: checked on the result rows only (R-02).
        let events = timed_json(&[(1, r#"{"a":1}"#), (2, r#"{"a":2,"b":"x"}"#), (3, "plain")]);
        assert_eq!(
            output("fields @timestamp, a, b, zzz", &events).columns,
            vec![FieldRef::Timestamp, json(&["a"]), json(&["b"])]
        );
        assert_eq!(
            output("fields @timestamp, a, b | filter a = 1", &events).columns,
            vec![FieldRef::Timestamp, json(&["a"])],
            "b has a value on a row the filter dropped"
        );
        assert_eq!(
            output("fields a | filter a = 9", &events).columns,
            Vec::<FieldRef>::new(),
            "no row, no value"
        );
    }

    #[test]
    fn without_a_limit_the_result_is_cut_at_10000_and_says_so() {
        // U2:BR5.2, BR5.3: matched counts every row the commands kept.
        let events: Vec<LogEvent> = (0..12_000i64).map(|i| event(i, "s", 0, "m")).collect();
        let cut = output("filter @message = 'm'", &events);
        assert_eq!(cut.rows.len(), 10_000);
        assert_eq!(
            (cut.matched_count, cut.truncated_at),
            (12_000, Some(10_000))
        );
        assert_eq!(cut.rows.first(), Some(&11_999), "the newest");
        let limited = output("filter @message = 'm' | limit 11000", &events);
        assert_eq!(
            (
                limited.rows.len(),
                limited.matched_count,
                limited.truncated_at
            ),
            (11_000, 12_000, Some(11_000))
        );
        let whole = output("limit 100000", &events);
        assert_eq!((whole.rows.len(), whole.truncated_at), (12_000, None));
        let small = output("limit 5", &events[..3]);
        assert_eq!((small.rows.len(), small.truncated_at), (3, None));
    }

    #[test]
    fn p08_limit_0_gives_no_row_and_says_nothing_was_cut() {
        // Checked at the console in Loop-back 3 (p08): `fields @timestamp |
        // limit 0` gave no row. It replaces review R-04's provisional
        // "limit 0 limits nothing". matched still counts the rows before
        // the limit (review R-08), but nothing is shown as cut.
        let events: Vec<LogEvent> = (0..10_500i64).map(|i| event(i, "s", 0, "m")).collect();
        let zero = output("fields @timestamp | limit 0", &events);
        assert_eq!(
            (zero.rows.len(), zero.matched_count, zero.truncated_at),
            (0, 10_500, None)
        );
        let few = output("limit 0", &events[..7]);
        assert_eq!((few.rows.len(), few.truncated_at), (0, None));
    }

    #[test]
    fn p08_limit_0_before_or_after_a_filter_gives_no_row() {
        let events = timed_json(&[
            (1, r#"{"level":"error"}"#),
            (2, r#"{"level":"info"}"#),
            (3, r#"{"level":"error"}"#),
        ]);
        let before = output("limit 0 | filter level = 'error'", &events);
        assert_eq!(
            (before.rows.len(), before.matched_count, before.truncated_at),
            (0, 0, None)
        );
        let after = output("filter level = 'error' | limit 0", &events);
        assert_eq!(
            (after.rows.len(), after.matched_count, after.truncated_at),
            (0, 2, None)
        );
        let one = output("filter level = 'error' | limit 1", &events);
        assert_eq!(
            (one.rows, one.matched_count, one.truncated_at),
            (vec![2], 2, Some(1)),
            "limit 1 is unchanged"
        );
    }

    #[test]
    fn matched_does_not_count_the_rows_an_earlier_limit_dropped() {
        // Review R-08: `filter a | limit 5 | filter b`.
        let events = timed_json(&[
            (1, r#"{"a":1,"b":1}"#),
            (2, r#"{"a":1,"b":0}"#),
            (3, r#"{"a":1,"b":1}"#),
            (4, r#"{"a":1,"b":0}"#),
            (5, r#"{"a":1,"b":1}"#),
            (6, r#"{"a":1,"b":1}"#),
            (7, r#"{"a":1,"b":0}"#),
        ]);
        let result = output("filter a = 1 | limit 5 | filter b = 1", &events);
        assert_eq!(result.rows, vec![5, 4, 2]);
        assert_eq!((result.matched_count, result.truncated_at), (3, None));
        let trailing = output("filter a = 1 | limit 5 | sort b", &events);
        assert_eq!(
            (trailing.matched_count, trailing.truncated_at),
            (7, Some(5))
        );
    }

    #[test]
    fn a_stop_while_the_sort_values_are_taken_drops_everything() {
        // R-02, U1:BR5.8: the values of a JSON sort are taken chunk by
        // chunk, each chunk a chance to stop.
        let events: Vec<LogEvent> = (0..25_000i64)
            .map(|i| event(i, "s", 0, &format!("{{\"n\":{}}}", i % 7)))
            .collect();
        let items = items(&events);
        let plan = query("sort n desc | limit 3");
        let mut seen = Vec::new();
        let mut asked = 0;
        let stopped = evaluate(
            &plan,
            items.as_slice(),
            || {
                asked += 1;
                asked > 2
            },
            |done, total| seen.push((done, total)),
        );
        assert_eq!(stopped, None);
        assert_eq!(seen, vec![(10_000, 25_000), (20_000, 25_000)]);
        let all = events_of(evaluate(&plan, items.as_slice(), || false, |_, _| {}));
        assert_eq!(
            all.rows,
            vec![24_996, 24_989, 24_982],
            "n = 6, newest first"
        );
    }

    // ---- 261017-insights-query U2:BR7.1, BR7.3: reading result rows ------

    fn written_time(time: i64) -> String {
        format!("t{time}")
    }

    #[test]
    fn the_cells_of_a_row_follow_the_columns() {
        // U2:BR7.1.
        let event = event(42, "s", 0, r#"{"s":"x","n":500,"b":false}"#);
        let fields = EventFields::new(&event, GROUP);
        let columns = [
            FieldRef::Timestamp,
            json(&["s"]),
            json(&["n"]),
            json(&["b"]),
            json(&["gone"]),
            FieldRef::Log,
            FieldRef::TimestampMillis,
        ];
        assert_eq!(
            row_cells(&columns, &fields, &written_time),
            vec![
                Some("t42".to_string()),
                Some("x".to_string()),
                Some("500".to_string()),
                // Loop-back 2 (p04): a boolean is shown 1 or 0.
                Some("0".to_string()),
                None,
                Some(GROUP.to_string()),
                Some("42".to_string()),
            ]
        );
    }

    #[test]
    fn an_opened_row_lists_the_fixed_fields_then_the_json_fields_by_name() {
        // U2:BR7.3.
        let event = event(42, "s", 0, r#"{"z":1,"http":{"status":500},"user.id":"u"}"#);
        let detail = row_detail(&EventFields::new(&event, GROUP), &written_time);
        let names: Vec<&str> = detail.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "@timestamp",
                "@log",
                "@logStream",
                "@message",
                "`user.id`",
                "http.status",
                "z"
            ]
        );
        assert_eq!(detail[0].1.as_deref(), Some("t42"));
        assert_eq!(detail[2].1.as_deref(), Some("s"));
        let plain = self::event(1, "s", 0, "no json");
        assert_eq!(
            row_detail(&EventFields::new(&plain, GROUP), &written_time).len(),
            4
        );
    }

    // ---- U1:BR6.3, U3:BR3.2: reading the rows of a result --------------

    /// Result rows at the given times, on stream `s` of [`GROUP`].
    fn copied(times: &[i64]) -> Vec<ResultRow> {
        times
            .iter()
            .map(|time| ResultRow::Event {
                event: event(*time, "s", 0, "m"),
                log_group_name: GROUP.to_string(),
            })
            .collect()
    }

    fn times(rows: &[ResultRow]) -> Vec<i64> {
        rows.iter()
            .filter_map(|row| row.event().map(|event| event.timestamp))
            .collect()
    }

    fn result() -> QueryResult {
        QueryResult::new(3, 10, copied(&[1, 4, 5, 8, 9]))
    }

    #[test]
    fn the_rows_of_the_asked_run_are_given_by_offset_and_limit() {
        let result = result();
        assert_eq!(times(result.window(3, 1, 3)), vec![4, 5, 8]);
        assert_eq!(times(result.window(3, 0, 5)), vec![1, 4, 5, 8, 9]);
        assert_eq!(result.row_count(), 5);
        assert_eq!(result.evaluated_count, 10);
        assert_eq!(
            result
                .row_at(3, 1)
                .and_then(ResultRow::event)
                .map(|event| event.timestamp),
            Some(4)
        );
        assert_eq!(
            result.row_at(3, 1).and_then(ResultRow::log_group_name),
            Some(GROUP)
        );
        assert_eq!(result.row_at(3, 5), None);
    }

    #[test]
    fn another_run_gets_no_rows() {
        // U3:BR3.2: the run number is checked, the copies need no epoch.
        assert!(result().window(2, 0, 5).is_empty());
        assert!(result().window(4, 0, 5).is_empty());
        assert_eq!(result().row_at(4, 0), None);
    }

    #[test]
    fn a_window_beyond_the_rows_is_cut_at_the_end() {
        let result = result();
        assert_eq!(times(result.window(3, 3, 100)), vec![8, 9]);
        assert!(result.window(3, 5, 10).is_empty());
        assert!(result.window(3, usize::MAX, usize::MAX).is_empty());
        assert!(result.window(3, 0, 0).is_empty());
    }

    #[test]
    fn a_result_without_matches_has_no_rows() {
        let empty = QueryResult::new(1, 50, Vec::new());
        assert_eq!(empty.row_count(), 0);
        assert!(empty.window(1, 0, 100).is_empty());
    }

    #[test]
    fn a_result_keeps_the_columns_the_cut_and_the_copies_of_its_output() {
        let result = QueryResult::from_output(
            2,
            EventsOutput {
                rows: vec![3, 1],
                columns: vec![FieldRef::Log],
                matched_count: 9,
                truncated_at: Some(2),
                evaluated_count: 20,
            },
            copied(&[30, 10]),
        );
        assert_eq!(
            result.columns(),
            &ResultColumns::Events(vec![FieldRef::Log])
        );
        assert_eq!(
            (
                result.matched_count(),
                result.truncated_at(),
                result.evaluated_count,
            ),
            (9, Some(2), 20)
        );
        assert_eq!(times(result.window(2, 0, 9)), vec![30, 10]);
    }

    // ---- 261025-insights-stats U1:BR2.9, BR3.1, BR3.2, BR3.5 -------------

    /// The aggregates output of `text` over `events` (with `stats`).
    fn aggregates(text: &str, events: &[LogEvent]) -> AggregatesOutput {
        match evaluate(&query(text), items(events).as_slice(), || false, |_, _| {}) {
            Some(EvalOutput::Aggregates(output)) => output,
            other => panic!("not an aggregates output: {other:?}"),
        }
    }

    fn levels(count: usize) -> Vec<LogEvent> {
        (0..count)
            .map(|index| {
                let level = if index % 3 == 0 { "error" } else { "info" };
                let time = i64::try_from(index).unwrap_or(i64::MAX);
                event(
                    time,
                    "s",
                    index as u64,
                    &format!("{{\"level\":\"{level}\",\"n\":{index}}}"),
                )
            })
            .collect()
    }

    #[test]
    fn stats_aggregates_only_the_events_the_filter_kept() {
        // BR3.2: the filter's rows are the input; fields do not reach the
        // aggregate columns.
        let events = levels(9);
        let output = aggregates(
            "fields @message | filter level = 'error' | stats count(*), sum(n)",
            &events,
        );
        assert_eq!(output.columns, vec!["count(*)", "sum(n)"]);
        assert_eq!(
            output.rows,
            vec![vec![CellValue::Integer(3), CellValue::Decimal(9.0)]]
        );
        assert_eq!(output.evaluated_count, 9);
    }

    #[test]
    fn the_aggregate_columns_are_the_keys_then_the_aggregates() {
        // BR3.1, BR3.3.
        let output = aggregates("stats count(*) as n, max(n) by level", &levels(6));
        assert_eq!(output.columns, vec!["level", "n", "max(n)"]);
        assert_eq!(
            output.rows,
            vec![
                vec![
                    CellValue::Text("error".to_string()),
                    CellValue::Integer(2),
                    CellValue::Integer(3)
                ],
                vec![
                    CellValue::Text("info".to_string()),
                    CellValue::Integer(4),
                    CellValue::Integer(5)
                ],
            ]
        );
    }

    #[test]
    fn the_aggregate_stage_reports_progress_chunk_by_chunk() {
        // BR2.9, BR3.5: the aggregation counts as one chunked pass, so the
        // progress never stays at 0.
        let events = levels(25_000);
        let items = items(&events);
        let plan = query("stats count(*)");
        let mut seen = Vec::new();
        let output = evaluate(
            &plan,
            items.as_slice(),
            || false,
            |done, total| seen.push((done, total)),
        );
        assert!(matches!(output, Some(EvalOutput::Aggregates(_))));
        assert_eq!(
            seen,
            vec![(10_000, 25_000), (20_000, 25_000), (25_000, 25_000)]
        );
        let filtered = query("filter level = 'error' | stats count(*)");
        let mut seen = Vec::new();
        evaluate(
            &filtered,
            items.as_slice(),
            || false,
            |done, total| seen.push((done, total)),
        );
        assert_eq!(seen.first(), Some(&(5_000, 25_000)), "half of two passes");
        assert_eq!(seen.last(), Some(&(25_000, 25_000)));
    }

    #[test]
    fn a_stop_during_the_aggregation_drops_everything() {
        // BR2.9, U1:BR5.8.
        let events = levels(25_000);
        let items = items(&events);
        let plan = query("stats count(*) by level");
        let mut asked = 0;
        let mut seen = Vec::new();
        let stopped = evaluate(
            &plan,
            items.as_slice(),
            || {
                asked += 1;
                asked > 2
            },
            |done, total| seen.push((done, total)),
        );
        assert_eq!(stopped, None);
        assert_eq!(seen, vec![(10_000, 25_000), (20_000, 25_000)]);
    }

    #[test]
    fn aggregates_are_cut_at_ten_thousand_and_keep_columns_without_values() {
        // U1:BR3.2, BR3.5 gave no default cut; U2:BR3.2 cuts the aggregate
        // rows at 10,000 as the event rows are (U1:BR3.4 revised there).
        // Still no column check.
        let events = levels(12_000);
        let by_n = aggregates("stats count(*) by n", &events);
        assert_eq!(
            (by_n.rows.len(), by_n.matched_count, by_n.truncated_at),
            (10_000, 12_000, Some(10_000))
        );
        let gone = aggregates("stats count(*) by gone", &events);
        assert_eq!(gone.columns, vec!["gone", "count(*)"]);
        assert_eq!(
            gone.rows,
            vec![vec![CellValue::Missing, CellValue::Integer(12_000)]]
        );
        let none = aggregates("filter level = 'none' | stats count(*) by level", &events);
        assert!(none.rows.is_empty());
        let one = aggregates("filter level = 'none' | stats count(*)", &events);
        assert_eq!(one.rows, vec![vec![CellValue::Integer(0)]]);
    }

    #[test]
    fn stats_after_sort_and_limit_aggregates_their_rows() {
        // BR1.6, BR3.2: U1 applies the commands before stats as written.
        let events = levels(9);
        let output = aggregates("sort n desc | limit 2 | stats max(n), min(n)", &events);
        assert_eq!(
            output.rows,
            vec![vec![CellValue::Integer(8), CellValue::Integer(7)]]
        );
    }

    #[test]
    fn a_plan_without_stats_still_gives_events() {
        // BR5.1: the events path is unchanged.
        let events = levels(4);
        let output = evaluate(
            &query("filter level = 'info'"),
            items(&events).as_slice(),
            || false,
            |_, _| {},
        );
        let Some(EvalOutput::Events(events_output)) = output else {
            panic!("not events: {output:?}");
        };
        assert_eq!(events_output.rows, vec![2, 1]);
        assert_eq!(
            events_output.columns,
            vec![FieldRef::Timestamp, FieldRef::Message]
        );
    }

    #[test]
    fn an_aggregates_result_is_shaped_and_its_rows_are_the_cells() {
        // BR3.1, BR3.4, BR3.6.
        let output = aggregates("stats count(*) by level", &levels(6));
        let result = QueryResult::from_aggregates(4, output);
        assert_eq!(result.shape(), ResultShape::Aggregates);
        assert_eq!(
            result.columns(),
            &ResultColumns::Aggregates(vec!["level".to_string(), "count(*)".to_string()])
        );
        assert_eq!(result.columns().names(), vec!["level", "count(*)"]);
        assert_eq!((result.row_count(), result.matched_count()), (2, 2));
        assert_eq!(result.truncated_at(), None);
        assert_eq!(result.evaluated_count, 6);
        assert_eq!(
            result.row_at(4, 1).and_then(ResultRow::cells),
            Some(&[CellValue::Text("info".to_string()), CellValue::Integer(4)][..])
        );
        assert_eq!(result.row_at(4, 1).and_then(ResultRow::event), None);
        let events = QueryResult::new(1, 0, Vec::new());
        assert_eq!(events.shape(), ResultShape::Events);
        assert_eq!(events.columns().names(), vec!["@timestamp", "@message"]);
    }

    // ---- 261025-insights-stats U2:BR3.1, BR3.2, BR5.1 ----------------------

    fn text(value: &str) -> CellValue {
        CellValue::Text(value.to_string())
    }

    /// The first cells of the rows of `output`.
    fn keys_of(output: &AggregatesOutput) -> Vec<CellValue> {
        output.rows.iter().map(|row| row[0].clone()).collect()
    }

    #[test]
    fn a_filter_after_stats_compares_the_cells_as_fields_are_compared() {
        // BR3.1 (c): integer and decimal cells are numbers, texts texts;
        // comparison, and / or / not, like, regular expressions and in.
        let events = levels(9); // error: n 0, 3, 6; info: the other six
        let by_level = "stats count(*) as c, avg(n) as a by level";
        for (written, expected) in [
            ("filter c > 3", vec![text("info")]),
            ("filter c = '3'", vec![text("error")]),
            ("filter a = 3", vec![text("error")]),
            ("filter a < 4.4", vec![text("error")]),
            ("filter level = 'info' or c = 3", vec![text("error"), text("info")]),
            ("filter level like 'err' and c = 3", vec![text("error")]),
            ("filter level =~ /^i/", vec![text("info")]),
            ("filter level not like /r/", vec![text("info")]),
            ("filter c in [6, 7]", vec![text("info")]),
            ("filter not (c = 6)", vec![text("error")]),
            ("filter c > a", vec![text("info")]),
        ] {
            let output = aggregates(&format!("{by_level} | {written}"), &events);
            assert_eq!(keys_of(&output), expected, "{written}");
        }
    }

    #[test]
    fn a_missing_cell_meets_a_value_as_a_missing_field_does() {
        // BR3.1: Missing is no value; it meets `=`, `<` and `>` never, and
        // `!=` as a missing field of an event does (U2:BR1.5).
        let events = levels(6);
        let base = "stats max(gone) as g, count(*) as c by level";
        for (written, expected) in [
            ("filter g = 1", Vec::<CellValue>::new()),
            ("filter g > 0", Vec::new()),
            ("filter g like 'x'", Vec::new()),
            ("filter g = c", Vec::new()),
            ("filter g != 1", vec![text("error"), text("info")]),
        ] {
            let output = aggregates(&format!("{base} | {written}"), &events);
            assert_eq!(keys_of(&output), expected, "{written}");
        }
        let event_output = output("filter gone != 1", &events);
        assert_eq!(event_output.rows.len(), 6, "the same as an event's field");
    }

    #[test]
    fn a_time_column_compares_as_the_timestamp_does() {
        // BR1.4 (c), BR3.1: milliseconds, and a date text in the chosen
        // zone (Asia/Tokyo here, as `query` resolves it).
        let four_pm = 1_756_051_200_000; // 2025-08-25 01:00:00 in Tokyo
        let events: Vec<LogEvent> = (0..4i64)
            .map(|index| event(four_pm + index * 60_000, "s", 0, "m"))
            .collect();
        let base = "stats count(*) as c by bin(1m) as t";
        let output = aggregates(&format!("{base} | filter t >= {}", four_pm + 120_000), &events);
        assert_eq!(
            keys_of(&output),
            vec![
                CellValue::Timestamp(four_pm + 120_000),
                CellValue::Timestamp(four_pm + 180_000)
            ]
        );
        let output = aggregates(&format!("{base} | filter t < '2025-08-25 01:01:00'"), &events);
        assert_eq!(keys_of(&output), vec![CellValue::Timestamp(four_pm)]);
    }

    #[test]
    fn a_sort_after_stats_orders_naturally_and_keeps_equal_rows_in_place() {
        // BR3.1: the natural order of sort; equal values keep the ascending
        // order of the keys.
        let events = levels(9);
        let counted = aggregates("stats count(*) as c by n | sort c desc", &events);
        assert_eq!(
            keys_of(&counted),
            (0..9).map(CellValue::Integer).collect::<Vec<_>>(),
            "nine counts of 1: the key order stays"
        );
        let by_level = aggregates("stats count(*) as c by level | sort c desc, level", &events);
        assert_eq!(keys_of(&by_level), vec![text("info"), text("error")]);
        let mixed = aggregates(
            "stats count(*) as c by v | sort v desc",
            &timed_json(&[
                (1, r#"{"v":"x10"}"#),
                (2, r#"{"v":"x2"}"#),
                (3, r#"{"v":7}"#),
                (4, "plain"),
            ]),
        );
        assert_eq!(
            keys_of(&mixed),
            vec![
                CellValue::Integer(7),
                text("x10"),
                text("x2"),
                CellValue::Missing
            ],
            "descending: numbers, texts in reverse natural order, missing last"
        );
    }

    #[test]
    fn limits_after_stats_apply_in_order_and_limit_0_keeps_no_row() {
        // BR1.4, BR3.1, BR3.2.
        let events = levels(9);
        let zero = aggregates("stats count(*) by n | limit 0", &events);
        assert_eq!(
            (zero.rows.len(), zero.matched_count, zero.truncated_at),
            (0, 9, None)
        );
        let twice = aggregates("stats count(*) by n | limit 5 | sort n desc | limit 2", &events);
        assert_eq!(keys_of(&twice), vec![CellValue::Integer(4), CellValue::Integer(3)]);
        assert_eq!((twice.matched_count, twice.truncated_at), (9, Some(2)));
        let uc1 = aggregates(
            "stats count(*) as n by level | filter n > 1 | sort n desc | limit 1",
            &events,
        );
        assert_eq!(uc1.rows, vec![vec![text("info"), CellValue::Integer(6)]]);
        assert_eq!((uc1.matched_count, uc1.truncated_at), (2, Some(1)));
    }

    #[test]
    fn aggregate_rows_are_cut_at_ten_thousand_without_a_limit() {
        // BR3.2: as the event rows (U2:BR5.2, BR5.3).
        let events = levels(10_001);
        let cut = aggregates("stats count(*) by n", &events);
        assert_eq!(
            (cut.rows.len(), cut.matched_count, cut.truncated_at),
            (10_000, 10_001, Some(10_000))
        );
        assert_eq!(cut.rows.last().map(|row| row[0].clone()), Some(CellValue::Integer(9_999)));
        let exact = aggregates("stats count(*) by n", &events[..10_000]);
        assert_eq!(
            (exact.rows.len(), exact.matched_count, exact.truncated_at),
            (10_000, 10_000, None)
        );
        let filtered = aggregates("stats count(*) as c by n | filter n >= 5000", &events);
        assert_eq!(
            (filtered.rows.len(), filtered.matched_count, filtered.truncated_at),
            (5_001, 5_001, None),
            "the filter's rows are what is counted"
        );
        let large = aggregates("stats count(*) by n | limit 100000", &events);
        assert_eq!((large.rows.len(), large.truncated_at), (10_001, None));
    }

    #[test]
    fn a_limit_after_stats_says_what_it_cut_and_a_filter_after_it_is_not_undone() {
        // BR3.2 (FR review R-03): the events' rule (review R-08).
        let events = levels(9);
        let three = aggregates("stats count(*) by n | limit 3", &events);
        assert_eq!(
            (three.rows.len(), three.matched_count, three.truncated_at),
            (3, 9, Some(3))
        );
        let refiltered = aggregates("stats count(*) by n | limit 3 | filter n > 0", &events);
        assert_eq!(
            (refiltered.rows.len(), refiltered.matched_count, refiltered.truncated_at),
            (2, 2, None)
        );
        let trailing = aggregates("stats count(*) by n | limit 3 | sort n desc", &events);
        assert_eq!(
            (trailing.matched_count, trailing.truncated_at),
            (9, Some(3))
        );
        let none = aggregates("stats count(*) by n | filter n > 100", &events);
        assert_eq!((none.rows.len(), none.truncated_at), (0, None));
    }

    #[test]
    fn a_stop_while_the_rows_are_finished_drops_everything() {
        // BR3.1 (FR review R-06): the commands after stats are applied
        // 10,000 rows at a time, the stop signal asked between.
        let events = levels(25_000);
        let items = items(&events);
        let plan = query("stats count(*) as c by n | filter c = 1 | sort n desc");
        let mut asked = 0;
        let finished = evaluate(
            &plan,
            items.as_slice(),
            || {
                asked += 1;
                false
            },
            |_, _| {},
        );
        let Some(EvalOutput::Aggregates(finished)) = finished else {
            panic!("no aggregates: {finished:?}");
        };
        assert_eq!(finished.rows.len(), 10_000);
        assert_eq!(asked, 7, "3 chunks of stats, 3 of the filter, the sort");
        for stop_at in 4..=7 {
            let mut asked = 0;
            let stopped = evaluate(
                &plan,
                items.as_slice(),
                || {
                    asked += 1;
                    asked >= stop_at
                },
                |_, _| {},
            );
            assert_eq!(stopped, None, "stopped at look {stop_at}");
        }
    }

    #[test]
    fn a_stats_without_commands_after_it_gives_what_u1_gave() {
        // BR5.1: rows, columns, counts and the steps are unchanged.
        let events = levels(9);
        let output = aggregates("stats count(*) as c, sum(n) by level", &events);
        assert_eq!(
            output.rows,
            vec![
                vec![text("error"), CellValue::Integer(3), CellValue::Decimal(9.0)],
                vec![text("info"), CellValue::Integer(6), CellValue::Decimal(27.0)],
            ]
        );
        assert_eq!((output.matched_count, output.truncated_at), (2, None));
        let result = QueryResult::from_aggregates(1, output);
        assert_eq!((result.matched_count(), result.truncated_at()), (2, None));
        let events = levels(25_000);
        let mut asked = 0;
        evaluate(
            &query("stats count(*) by level"),
            items(&events).as_slice(),
            || {
                asked += 1;
                false
            },
            |_, _| {},
        );
        assert_eq!(asked, 3, "no extra step without commands after stats");
    }

    #[test]
    fn an_aggregates_result_keeps_its_matched_count_and_its_cut() {
        // BR3.2: carried to the screen's summary as the events' are.
        let output = aggregates("stats count(*) by n", &levels(10_005));
        let result = QueryResult::from_aggregates(2, output);
        assert_eq!(
            (result.row_count(), result.matched_count(), result.truncated_at()),
            (10_000, 10_005, Some(10_000))
        );
    }
}

/// 261017-insights-query U2:NFR1, OQ3, review R-02: how long evaluation
/// takes on 100,000 and 1,000,000 held events, and how late a [Cancel] can
/// be (the longest step between two looks at the stop signal). Run with
/// `cargo test --release -p local-sights-core query:: -- --ignored
/// --nocapture`. The pass marks are decided with the person at Build and
/// Test from these numbers; until then the tests only print the times and
/// check that each evaluation ends with a consistent result.
#[cfg(test)]
mod speed {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::query::parser::parse_query;

    const GROUP: &str = "/aws/lambda/orders";

    /// The four kinds of query measured (Step 17).
    const QUERIES: [(&str, &str); 4] = [
        ("filter only", "filter @message like /ERROR/"),
        (
            "JSON filter",
            "filter level = 'ERROR' and http.status >= 500",
        ),
        ("JSON sort", "sort latency desc"),
        (
            "fields+sort+limit",
            "fields @timestamp, level, http.status, latency\n\
             | filter level = 'ERROR'\n| sort latency desc\n| limit 100",
        ),
    ];

    /// `count` events in time order across 8 streams: nine in ten are JSON
    /// objects with a level, a nested status and a latency; one in ten is a
    /// plain text line, so JSON reading meets both kinds.
    fn held_events(count: usize) -> Vec<LogEvent> {
        (0..count)
            .map(|index| {
                let level = if index % 7 == 0 { "ERROR" } else { "INFO" };
                let status = if index % 11 == 0 { 503 } else { 200 };
                let latency = (index * 7919) % 100_003;
                let message = if index % 10 == 9 {
                    format!("plain text line {index} {level} without JSON")
                } else {
                    format!(
                        "{{\"level\":\"{level}\",\"http\":{{\"status\":{status},\"path\":\"/orders/{index}\"}},\"latency\":{latency},\"msg\":\"request {index}\"}}"
                    )
                };
                LogEvent {
                    timestamp: 1_756_000_000_000 + i64::try_from(index / 3).unwrap_or(i64::MAX),
                    ingestion_time: None,
                    message,
                    log_stream_name: format!("stream-{}", index % 8),
                    sequence: u64::try_from(index).unwrap_or(u64::MAX),
                }
            })
            .collect()
    }

    /// Evaluates `text` step by step as the runner does and returns the
    /// output, the whole time and the longest step.
    fn measure(text: &str, items: &GroupEvents<'_>) -> (EventsOutput, Duration, Duration) {
        let plan = parse_query(text).unwrap();
        let started = Instant::now();
        let mut longest = Duration::ZERO;
        let mut evaluation = Evaluation::new(&plan, items.len());
        let before = Instant::now();
        evaluation.advance(&plan, items);
        longest = longest.max(before.elapsed());
        while !evaluation.is_done() {
            let before = Instant::now();
            evaluation.step(&plan, items);
            longest = longest.max(before.elapsed());
        }
        let elapsed = started.elapsed();
        let Some(EvalOutput::Events(output)) = evaluation.into_output() else {
            panic!("{text:?} gave no events output");
        };
        (output, elapsed, longest)
    }

    fn measure_all(count: usize) {
        let events = held_events(count);
        let items = GroupEvents {
            events: &events,
            log_group_name: GROUP,
        };
        for (name, text) in QUERIES {
            let (output, elapsed, longest) = measure(text, &items);
            println!(
                "{name} over {count} events: {elapsed:?} (longest step {longest:?}), matched {}, rows {}",
                output.matched_count,
                output.rows.len()
            );
            assert_eq!(output.evaluated_count, count);
            assert!(output.rows.len() <= output.matched_count);
            assert!(output.rows.len() <= DEFAULT_RESULT_LIMIT);
        }
    }

    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn four_queries_over_one_hundred_thousand_events() {
        measure_all(100_000);
    }

    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn four_queries_over_one_million_events() {
        measure_all(1_000_000);
    }

    /// R-02: a [Cancel] is seen between two steps, so its delay is at most
    /// the longest step. The JSON sort's own sort of the values cannot be
    /// stopped; this measures it, with the stop raised as soon as that
    /// step begins.
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn a_cancel_during_a_json_sort_of_one_million_events() {
        let events = held_events(1_000_000);
        let items = GroupEvents {
            events: &events,
            log_group_name: GROUP,
        };
        let plan = parse_query("sort latency desc").unwrap();
        let (_, _, longest) = measure("sort latency desc", &items);
        // A stop raised after a few chunks ends the evaluation at the next
        // look, without a result.
        let mut looks = 0usize;
        let started = Instant::now();
        let output = evaluate(
            &plan,
            &items,
            || {
                looks += 1;
                looks > 3
            },
            |_, _| {},
        );
        let stopped = started.elapsed();
        assert!(output.is_none(), "a raised stop ends the evaluation");
        println!(
            "JSON sort of 1,000,000 events: longest step (the most a Cancel waits) {longest:?}; \
             stopped after 3 steps in {stopped:?}"
        );
    }

    /// Evaluates the `stats` query `text` step by step as the runner does
    /// and returns the output, the whole time and the longest step.
    fn measure_stats(text: &str, items: &GroupEvents<'_>) -> (AggregatesOutput, Duration, Duration) {
        let plan = parse_query(text).unwrap();
        let started = Instant::now();
        let mut longest = Duration::ZERO;
        let mut evaluation = Evaluation::new(&plan, items.len());
        evaluation.advance(&plan, items);
        while !evaluation.is_done() {
            let before = Instant::now();
            evaluation.step(&plan, items);
            longest = longest.max(before.elapsed());
        }
        let elapsed = started.elapsed();
        let Some(EvalOutput::Aggregates(output)) = evaluation.into_output() else {
            panic!("{text:?} gave no aggregates output");
        };
        (output, elapsed, longest)
    }

    /// 261025-insights-stats U2:BR2.6, NFR2 (DD review R-10, FR review
    /// R-06): the heavy `stats` (every @message kept for count_distinct, every
    /// latency for pct) over the same 1,000,000 events as U1's
    /// `stats count(*) by bin(5m)`, within three times its time. Every
    /// message of [`held_events`] differs, so count_distinct keeps 1,000,000
    /// texts: the worst case.
    #[test]
    #[ignore = "measures speed; run with --release --ignored"]
    fn the_heavy_stats_takes_at_most_three_times_the_bin_count() {
        let events = held_events(1_000_000);
        let items = GroupEvents {
            events: &events,
            log_group_name: GROUP,
        };
        let baseline = "stats count(*) by bin(5m)";
        let heavy = "stats count_distinct(@message), pct(latency, 99) by @logStream";
        let every = "stats count(*), count(level), count_distinct(level), sum(latency), \
                     avg(latency), min(latency), max(latency), pct(latency, 50), \
                     stddev(latency), earliest(level), latest(level), sortsFirst(level), \
                     sortsLast(level) by bin(5m)";
        // One run first, so the allocator and caches are warm for both.
        measure_stats(baseline, &items);
        for part in [
            "stats count(*) by @logStream",
            "stats count_distinct(@message) by @logStream",
            "stats pct(latency, 99) by @logStream",
            "stats sum(latency) by @logStream",
        ] {
            let (_, time, step) = measure_stats(part, &items);
            println!("  part {part}: {time:?} (longest step {step:?})");
        }
        let (base, base_time, base_step) = measure_stats(baseline, &items);
        let (heavy_output, heavy_time, heavy_step) = measure_stats(heavy, &items);
        let (every_output, every_time, every_step) = measure_stats(every, &items);
        println!(
            "stats over 1,000,000 events:\n  {baseline}: {base_time:?} (longest step {base_step:?}), {} rows\n  \
             {heavy}: {heavy_time:?} (longest step {heavy_step:?}), {} rows, {:.2}x\n  \
             all functions by bin(5m): {every_time:?} (longest step {every_step:?}), {} rows, {:.2}x",
            base.rows.len(),
            heavy_output.rows.len(),
            heavy_time.as_secs_f64() / base_time.as_secs_f64(),
            every_output.rows.len(),
            every_time.as_secs_f64() / base_time.as_secs_f64(),
        );
        assert_eq!(heavy_output.rows.len(), 8, "one row per stream");
        let distinct: i64 = heavy_output
            .rows
            .iter()
            .map(|row| match row[1] {
                CellValue::Integer(count) => count,
                _ => 0,
            })
            .sum();
        assert_eq!(distinct, 1_000_000, "every message counted once");
        assert!(
            heavy_time <= base_time * 3,
            "the heavy stats took {heavy_time:?}, more than 3 x {base_time:?}"
        );
    }
}
