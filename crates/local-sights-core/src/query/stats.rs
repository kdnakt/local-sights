//! StatsAggregator: the `stats` command of a query, worked out on the
//! events the query screen fetched (261025-insights-stats U1:BR2.1-BR2.9,
//! ADR-002).
//!
//! The evaluator gives the aggregator the events a `filter` left, chunk by
//! chunk ([`StatsAggregator::add`], BR2.9). Each event goes into the group
//! of its `by` key values: two values that the table would show alike are
//! one key (the number 200 and the text "200"), and the events without a key
//! field share the "no value" key (BR2.1, Q3 A). A `bin(period)` key is the
//! start of the period the event falls in, a multiple of the period from the
//! epoch (BR2.5). Each group keeps one accumulator per aggregate: counts,
//! sums of what reads as a number, and the smallest and largest value in the
//! order of `sort` (BR2.2-BR2.4). [`StatsAggregator::finish`] makes the rows
//! in ascending order of their keys, with "no value" last (BR2.7), and one
//! row for no event without `by` (BR2.6).
//!
//! The cells keep their type ([`CellValue`]); their text is made only when
//! they are shown, exported or printed, by [`cell_text`] with the screen's
//! time zone (BR2.8, BR4.2, A7). The details the documentation leaves open
//! (the key of events without the field, values that are no number, the row
//! for no event, the four decimal places, the default order) are provisional
//! until U4 compares them with the console.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::query::evaluator::number_of_digits;
use crate::query::fields::{EventFields, FieldValue};
use crate::query::natural::{SortKeyRef, SortValue, compare_sort_keys};
use crate::query::plan::{
    AggregateArgument, AggregateFunction, AggregateSpec, FieldRef, GroupKeyKind, StatsCommand,
};

/// Largest whole number every smaller one of which an `f64` holds exactly:
/// 2^53.
const EXACT_INTEGER_LIMIT: f64 = 9_007_199_254_740_992.0;

/// Decimal places of an aggregate's text (BR2.8, Q5 A).
const DECIMAL_SCALE: f64 = 10_000.0;

/// The value of one cell of an aggregate row (entities.md CellValue): typed,
/// its text made when shown (DD review R-05).
#[derive(Debug, Clone)]
pub enum CellValue {
    /// No value: the key of the events without the field, or a `sum`,
    /// `avg`, `min` or `max` with nothing to aggregate.
    Missing,
    /// A whole number: a count, a whole JSON number, `toMillis(@timestamp)`.
    Integer(i64),
    /// A number kept unrounded (`sum`, `avg`, a JSON number with a
    /// fraction); rounded only when written (BR2.8).
    Decimal(f64),
    /// A text; a boolean is the text `1` or `0`, as a cell shows it.
    Text(String),
    /// Epoch milliseconds, written in the screen's zone (BR4.2): a `bin`
    /// key, a `by @timestamp` key, the `min` or `max` of `@timestamp`.
    Timestamp(i64),
}

impl PartialEq for CellValue {
    /// Decimals are equal when they are the same `f64`, bit for bit, so the
    /// equality is total.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Missing, CellValue::Missing) => true,
            (CellValue::Integer(left), CellValue::Integer(right))
            | (CellValue::Timestamp(left), CellValue::Timestamp(right)) => left == right,
            (CellValue::Decimal(left), CellValue::Decimal(right)) => {
                left.to_bits() == right.to_bits()
            }
            (CellValue::Text(left), CellValue::Text(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for CellValue {}

impl CellValue {
    /// A number as a cell: whole numbers below 2^53 as integers, others as
    /// decimals. `-0` is the integer 0.
    fn of_number(number: f64) -> Self {
        if number.fract() == 0.0 && number.abs() < EXACT_INTEGER_LIMIT {
            // A whole number below 2^53: the conversion is exact.
            CellValue::Integer(number as i64)
        } else {
            CellValue::Decimal(number)
        }
    }

    /// `value` of `field` as a cell: the time as a time, `toMillis` as a
    /// number, a boolean as `1` or `0`.
    fn of_value(field: &FieldRef, value: FieldValue<'_>) -> Self {
        match (field, value) {
            (_, FieldValue::Missing) => CellValue::Missing,
            // Epoch milliseconds of any real time are below 2^53: exact.
            (FieldRef::Timestamp, FieldValue::Number(time)) => CellValue::Timestamp(time as i64),
            (_, FieldValue::Number(number)) => CellValue::of_number(number),
            (_, FieldValue::Text(text)) => CellValue::Text(text.to_string()),
            (_, FieldValue::Boolean(flag)) => CellValue::Text(if flag { "1" } else { "0" }.into()),
        }
    }

    /// The order of the rows by this key (BR2.7): numbers and times as
    /// numbers, texts in natural order before them, "no value" last.
    fn compare_as_key(&self, other: &Self) -> Ordering {
        match (self.sort_key(), other.sort_key()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => compare_sort_keys(left, right),
        }
    }

    fn sort_key(&self) -> Option<SortKeyRef<'_>> {
        match self {
            CellValue::Missing => None,
            CellValue::Integer(number) | CellValue::Timestamp(number) => {
                Some(SortKeyRef::Number(*number as f64))
            }
            CellValue::Decimal(number) => Some(SortKeyRef::Number(*number)),
            CellValue::Text(text) => Some(SortKeyRef::Text(text)),
        }
    }
}

/// The text of an aggregate (BR2.8): `None` for infinity and not a number;
/// otherwise rounded to four decimal places, half away from zero, written
/// without trailing zeros, `-0` as `0`, whole numbers below 2^53 without a
/// fraction and larger ones in full.
pub fn format_aggregate(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    if value.abs() >= EXACT_INTEGER_LIMIT {
        // Every f64 this large is whole; written in full, shortest digits.
        return Some(format!("{value}"));
    }
    if value.fract() == 0.0 {
        // Whole already: not multiplied, so no digit is lost. `-0` is 0.
        return Some(format!("{}", value as i64));
    }
    let rounded = (value * DECIMAL_SCALE).round() / DECIMAL_SCALE;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    if rounded.fract() == 0.0 {
        // A whole number below 2^53: the conversion is exact.
        return Some(format!("{}", rounded as i64));
    }
    let text = format!("{rounded:.4}");
    Some(text.trim_end_matches('0').trim_end_matches('.').to_string())
}

/// The text of `cell` as the table, the opened row, the export and the
/// check program show it (BR2.8, BR4.2, A7): a time by `format_time` (the
/// screen's zone), `None` without a value.
pub fn cell_text(cell: &CellValue, format_time: &dyn Fn(i64) -> String) -> Option<String> {
    match cell {
        CellValue::Missing => None,
        CellValue::Integer(number) => Some(number.to_string()),
        CellValue::Decimal(number) => format_aggregate(*number),
        CellValue::Text(text) => Some(text.clone()),
        CellValue::Timestamp(time) => Some(format_time(*time)),
    }
}

/// The text a key value is told apart by (BR2.1): as the table would show
/// it, a time by its milliseconds; `None` for no value.
fn key_text(cell: &CellValue) -> Option<String> {
    cell_text(cell, &|time| time.to_string())
}

/// The running value of one aggregate of one group (entities.md
/// Accumulator).
#[derive(Debug, Clone)]
enum Accumulator {
    /// `count(*)` or `count(field)`: the events counted.
    Count(i64),
    /// `sum` or `avg`: the total and how many values were added.
    Sum { total: f64, added: u64 },
    /// `min` or `max`: the value kept so far, with its order.
    Extreme(Option<(SortValue, CellValue)>),
}

impl Accumulator {
    fn new(function: AggregateFunction) -> Self {
        match function {
            AggregateFunction::Count => Accumulator::Count(0),
            AggregateFunction::Sum | AggregateFunction::Avg => Accumulator::Sum {
                total: 0.0,
                added: 0,
            },
            AggregateFunction::Min | AggregateFunction::Max => Accumulator::Extreme(None),
        }
    }

    /// Adds the event of `fields` (BR2.2-BR2.4).
    fn add(&mut self, spec: &AggregateSpec, fields: &EventFields<'_>) {
        let field = match &spec.argument {
            AggregateArgument::All => {
                if let Accumulator::Count(count) = self {
                    *count = count.saturating_add(1);
                }
                return;
            }
            AggregateArgument::Field(field) => field,
        };
        let value = fields.value(field);
        match self {
            Accumulator::Count(count) => {
                if value != FieldValue::Missing {
                    *count = count.saturating_add(1);
                }
            }
            Accumulator::Sum { total, added } => {
                if let Some(number) = number_of(value) {
                    *total += number;
                    *added += 1;
                }
            }
            Accumulator::Extreme(kept) => {
                if value == FieldValue::Missing {
                    return;
                }
                let order = SortValue::of(value);
                let wanted = if spec.function == AggregateFunction::Min {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                // Of two values in the same place, the first seen stays.
                let replaces = kept.as_ref().is_none_or(|(held, _)| {
                    compare_sort_keys(order.as_key(), held.as_key()) == wanted
                });
                if replaces {
                    *kept = Some((order, CellValue::of_value(field, value)));
                }
            }
        }
    }

    /// The cell of the finished aggregate.
    fn into_cell(self, function: AggregateFunction) -> CellValue {
        match self {
            Accumulator::Count(count) => CellValue::Integer(count),
            Accumulator::Sum { added: 0, .. } => CellValue::Missing,
            Accumulator::Sum { total, added } => {
                if function == AggregateFunction::Avg {
                    // At most 2^64 values: the quotient is what f64 can say.
                    CellValue::Decimal(total / added as f64)
                } else {
                    CellValue::Decimal(total)
                }
            }
            Accumulator::Extreme(kept) => kept.map_or(CellValue::Missing, |(_, cell)| cell),
        }
    }
}

/// A value `sum` and `avg` add (BR2.3): a number, or a text of digits;
/// booleans and missing values are left out.
fn number_of(value: FieldValue<'_>) -> Option<f64> {
    match value {
        FieldValue::Number(number) => Some(number),
        FieldValue::Text(text) => number_of_digits(text),
        FieldValue::Boolean(_) | FieldValue::Missing => None,
    }
}

/// One group being aggregated (entities.md AggregateGroup).
#[derive(Debug, Clone)]
struct AggregateGroup {
    /// The key cells, one per `by` key, of the first event of the group.
    keys: Vec<CellValue>,
    /// One per aggregate.
    accumulators: Vec<Accumulator>,
}

/// The finished table of a `stats` (BR3.3): the column names, keys then
/// aggregates, and the rows in the order of BR2.7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateTable {
    /// The column names.
    pub columns: Vec<String>,
    /// One cell per column per row.
    pub rows: Vec<Vec<CellValue>>,
}

/// Aggregates the events given to it (ADR-002): `new`, then `add` for each
/// event a `filter` left, then `finish`.
#[derive(Debug, Clone)]
pub struct StatsAggregator {
    stats: StatsCommand,
    /// The group of each key, by the texts of its values.
    index: HashMap<Vec<Option<String>>, usize>,
    groups: Vec<AggregateGroup>,
}

impl StatsAggregator {
    /// An aggregator of `stats` with no event yet.
    pub fn new(stats: StatsCommand) -> Self {
        Self {
            stats,
            index: HashMap::new(),
            groups: Vec::new(),
        }
    }

    /// The groups made so far.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Adds the event of `fields` to its group (BR2.1-BR2.5).
    pub fn add(&mut self, fields: &EventFields<'_>) {
        let keys: Vec<CellValue> = self
            .stats
            .group_keys
            .iter()
            .map(|key| match &key.kind {
                GroupKeyKind::Field(field) => CellValue::of_value(field, fields.value(field)),
                GroupKeyKind::Bin(period) => {
                    let millis = period.millis().max(1);
                    let time = fields.event().timestamp;
                    CellValue::Timestamp(time.div_euclid(millis).saturating_mul(millis))
                }
            })
            .collect();
        let texts: Vec<Option<String>> = keys.iter().map(key_text).collect();
        let position = match self.index.get(&texts) {
            Some(position) => *position,
            None => {
                let position = self.groups.len();
                self.groups.push(self.empty_group(keys));
                self.index.insert(texts, position);
                position
            }
        };
        if let Some(group) = self.groups.get_mut(position) {
            for (accumulator, spec) in group.accumulators.iter_mut().zip(&self.stats.aggregates) {
                accumulator.add(spec, fields);
            }
        }
    }

    fn empty_group(&self, keys: Vec<CellValue>) -> AggregateGroup {
        AggregateGroup {
            keys,
            accumulators: self
                .stats
                .aggregates
                .iter()
                .map(|spec| Accumulator::new(spec.function))
                .collect(),
        }
    }

    /// The rows (BR2.6, BR2.7): one per group in ascending order of the
    /// keys, or one row for no event when there is no `by`.
    pub fn finish(mut self) -> AggregateTable {
        if self.groups.is_empty() && self.stats.group_keys.is_empty() {
            let empty = self.empty_group(Vec::new());
            self.groups.push(empty);
        }
        let mut groups = std::mem::take(&mut self.groups);
        groups.sort_by(|left, right| {
            left.keys
                .iter()
                .zip(&right.keys)
                .map(|(left, right)| left.compare_as_key(right))
                .find(|order| *order != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        });
        let functions: Vec<AggregateFunction> = self
            .stats
            .aggregates
            .iter()
            .map(|spec| spec.function)
            .collect();
        let rows = groups
            .into_iter()
            .map(|group| {
                let mut cells = group.keys;
                cells.extend(
                    group
                        .accumulators
                        .into_iter()
                        .zip(&functions)
                        .map(|(accumulator, function)| accumulator.into_cell(*function)),
                );
                cells
            })
            .collect();
        AggregateTable {
            columns: self.stats.column_names(),
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::LogEvent;
    use crate::query::fields::EventFields;
    use crate::query::parser::parse_query;
    use crate::query::plan::StatsCommand;

    const GROUP: &str = "/aws/lambda/orders";

    /// 2025-08-24 16:00:00 UTC in epoch milliseconds.
    const FOUR_PM: i64 = 1_756_051_200_000;

    fn event(timestamp: i64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: "stream-a".to_string(),
            sequence: 0,
        }
    }

    fn stats(text: &str) -> StatsCommand {
        match parse_query(text) {
            Ok(plan) => match plan.stats() {
                Some(stats) => stats.clone(),
                None => panic!("{text:?} has no stats"),
            },
            Err(problem) => panic!("{text:?} was not read: {problem:?}"),
        }
    }

    /// The table of `text` over `events`, all added one by one.
    fn table(text: &str, events: &[LogEvent]) -> AggregateTable {
        let mut aggregator = StatsAggregator::new(stats(text));
        for event in events {
            aggregator.add(&EventFields::new(event, GROUP));
        }
        aggregator.finish()
    }

    fn rows(text: &str, events: &[LogEvent]) -> Vec<Vec<CellValue>> {
        table(text, events).rows
    }

    fn json(timestamp: i64, message: &str) -> LogEvent {
        event(timestamp, message)
    }

    fn text(value: &str) -> CellValue {
        CellValue::Text(value.to_string())
    }

    #[test]
    fn events_are_grouped_by_the_key_and_counted() {
        // BR2.1, BR2.2.
        let events = [
            json(1, r#"{"level":"a"}"#),
            json(2, r#"{"level":"b"}"#),
            json(3, r#"{"level":"a"}"#),
        ];
        let table = table("stats count(*) by level", &events);
        assert_eq!(table.columns, vec!["level", "count(*)"]);
        assert_eq!(
            table.rows,
            vec![
                vec![text("a"), CellValue::Integer(2)],
                vec![text("b"), CellValue::Integer(1)],
            ]
        );
    }

    #[test]
    fn a_number_and_a_text_shown_alike_are_one_group_of_the_first_kind_seen() {
        // BR2.1 (functional-design review R-04): 200 and "200" are one key,
        // -0 and 0 too; the cell keeps the kind of the first value seen. A
        // text key comes before the numbers, as in `sort` (BR2.7).
        let events = [
            json(1, r#"{"status":200}"#),
            json(2, r#"{"status":"200"}"#),
            json(3, r#"{"status":-0}"#),
            json(4, r#"{"status":0}"#),
            json(5, r#"{"status":"404"}"#),
        ];
        assert_eq!(
            rows("stats count(*) by status", &events),
            vec![
                vec![text("404"), CellValue::Integer(1)],
                vec![CellValue::Integer(0), CellValue::Integer(2)],
                vec![CellValue::Integer(200), CellValue::Integer(2)],
            ]
        );
    }

    #[test]
    fn events_without_the_key_are_one_group_shown_last() {
        // BR2.1 (Q3 A), BR2.7: Missing is one key, after every other.
        let events = [
            json(1, r#"{"level":"b"}"#),
            json(2, "plain text"),
            json(3, r#"{"level":"a"}"#),
            json(4, r#"{"level":null}"#),
        ];
        assert_eq!(
            rows("stats count(*) by level", &events),
            vec![
                vec![text("a"), CellValue::Integer(1)],
                vec![text("b"), CellValue::Integer(1)],
                vec![CellValue::Missing, CellValue::Integer(2)],
            ]
        );
    }

    #[test]
    fn count_of_a_field_counts_the_events_that_have_it_even_empty() {
        // BR2.2.
        let events = [
            json(1, r#"{"user":"u1"}"#),
            json(2, r#"{"user":""}"#),
            json(3, r#"{"user":null}"#),
            json(4, r#"{"other":1}"#),
        ];
        assert_eq!(
            rows("stats count(user), count(*), count(@message)", &events),
            vec![vec![
                CellValue::Integer(2),
                CellValue::Integer(4),
                CellValue::Integer(4)
            ]]
        );
    }

    #[test]
    fn sum_and_avg_add_only_what_reads_as_a_number() {
        // BR2.3 (Q4 A): numbers and texts of digits are added; other texts,
        // booleans and missing values are left out.
        let events = [
            json(1, r#"{"latency":100}"#),
            json(2, r#"{"latency":"200.5"}"#),
            json(3, r#"{"latency":"abc"}"#),
            json(4, r#"{"latency":true}"#),
            json(5, r#"{"latency":" 7"}"#),
            json(6, "no json"),
            json(7, r#"{"latency":-0.5}"#),
        ];
        assert_eq!(
            rows("stats sum(latency), avg(latency)", &events),
            vec![vec![CellValue::Decimal(300.0), CellValue::Decimal(100.0)]]
        );
        assert_eq!(
            rows("stats sum(nothing), avg(nothing)", &events),
            vec![vec![CellValue::Missing, CellValue::Missing]],
            "nothing to add gives no value"
        );
    }

    #[test]
    fn min_and_max_compare_as_sort_does() {
        // BR2.4 (Q4 A): numbers as numbers; texts before numbers, in natural
        // order; missing values left out.
        let numbers = [
            json(1, r#"{"n":10}"#),
            json(2, r#"{"n":2}"#),
            json(3, r#"{"n":2.5}"#),
            json(4, "plain"),
        ];
        assert_eq!(
            rows("stats min(n), max(n)", &numbers),
            vec![vec![CellValue::Integer(2), CellValue::Integer(10)]]
        );
        let mixed = [
            json(1, r#"{"v":5}"#),
            json(2, r#"{"v":"x10"}"#),
            json(3, r#"{"v":"x2"}"#),
            json(4, r#"{"v":1.25}"#),
        ];
        assert_eq!(
            rows("stats min(v), max(v)", &mixed),
            vec![vec![text("x2"), CellValue::Integer(5)]]
        );
        assert_eq!(
            rows("stats min(gone), max(gone)", &mixed),
            vec![vec![CellValue::Missing, CellValue::Missing]]
        );
        let flags = [json(1, r#"{"ok":true}"#), json(2, r#"{"ok":false}"#)];
        assert_eq!(
            rows("stats min(ok), max(ok)", &flags),
            vec![vec![text("0"), text("1")]],
            "a boolean is shown 1 or 0, ordered as its text"
        );
    }

    #[test]
    fn min_and_max_of_the_time_are_times_and_of_to_millis_numbers() {
        // BR2.4, entities.md CellValue.
        let events = [json(FOUR_PM + 5, "a"), json(FOUR_PM, "b")];
        assert_eq!(
            rows(
                "stats min(@timestamp), max(@timestamp), max(toMillis(@timestamp)), sum(toMillis(@timestamp))",
                &events
            ),
            vec![vec![
                CellValue::Timestamp(FOUR_PM),
                CellValue::Timestamp(FOUR_PM + 5),
                CellValue::Integer(FOUR_PM + 5),
                CellValue::Decimal((2 * FOUR_PM + 5) as f64),
            ]]
        );
    }

    #[test]
    fn bins_start_at_multiples_of_the_period_from_the_epoch() {
        // BR2.5: empty bins give no row; a negative time is floored too.
        let events = [
            json(FOUR_PM, "a"),
            json(FOUR_PM + 299_999, "b"),
            json(FOUR_PM + 300_000, "c"),
            json(FOUR_PM + 900_001, "d"),
        ];
        assert_eq!(
            rows("stats count(*) by bin(5m)", &events),
            vec![
                vec![CellValue::Timestamp(FOUR_PM), CellValue::Integer(2)],
                vec![
                    CellValue::Timestamp(FOUR_PM + 300_000),
                    CellValue::Integer(1)
                ],
                vec![
                    CellValue::Timestamp(FOUR_PM + 900_000),
                    CellValue::Integer(1)
                ],
            ]
        );
        assert_eq!(
            rows("stats count(*) by bin(1s)", &[json(-1, "x"), json(0, "y")]),
            vec![
                vec![CellValue::Timestamp(-1_000), CellValue::Integer(1)],
                vec![CellValue::Timestamp(0), CellValue::Integer(1)],
            ]
        );
    }

    #[test]
    fn a_day_bin_starts_at_midnight_utc() {
        // BR2.5, FR2.3: the bins do not depend on the screen's zone.
        let midnight = 1_756_080_000_000; // 2025-08-25 00:00:00 UTC
        let events = [json(midnight - 1, "a"), json(midnight, "b")];
        assert_eq!(
            rows("stats count(*) by bin(1d)", &events),
            vec![
                vec![
                    CellValue::Timestamp(midnight - 86_400_000),
                    CellValue::Integer(1)
                ],
                vec![CellValue::Timestamp(midnight), CellValue::Integer(1)],
            ]
        );
    }

    #[test]
    fn no_event_gives_one_row_without_by_and_none_with_by() {
        // BR2.6 (Q2 A).
        assert_eq!(
            rows(
                "stats count(*), count(a), sum(a), avg(a), min(a), max(a)",
                &[]
            ),
            vec![vec![
                CellValue::Integer(0),
                CellValue::Integer(0),
                CellValue::Missing,
                CellValue::Missing,
                CellValue::Missing,
                CellValue::Missing,
            ]]
        );
        let table = table("stats count(*) by level", &[]);
        assert_eq!(table.columns, vec!["level", "count(*)"]);
        assert!(table.rows.is_empty());
    }

    #[test]
    fn rows_are_in_ascending_order_of_their_keys_in_the_order_written() {
        // BR2.7: numbers as numbers, texts in natural order, each key in
        // turn.
        let events = [
            json(1, r#"{"g":"x10","n":1}"#),
            json(2, r#"{"g":"x2","n":10}"#),
            json(3, r#"{"g":"x2","n":2}"#),
            json(4, r#"{"g":"x10","n":1}"#),
        ];
        assert_eq!(
            rows("stats count(*) by g, n", &events),
            vec![
                vec![text("x2"), CellValue::Integer(2), CellValue::Integer(1)],
                vec![text("x2"), CellValue::Integer(10), CellValue::Integer(1)],
                vec![text("x10"), CellValue::Integer(1), CellValue::Integer(2)],
            ]
        );
        assert_eq!(
            StatsAggregator::new(stats("stats count(*) by g")).group_count(),
            0
        );
    }

    #[test]
    fn fixed_field_keys_are_texts_and_the_time_key_a_time() {
        // BR2.1: `by @timestamp` keeps a time cell, the other fixed fields
        // are texts; the columns are the keys then the aggregates (BR3.3).
        let events = [json(FOUR_PM, "m"), json(FOUR_PM, "m")];
        let table = table(
            "stats count(*) as n by @timestamp, @logStream, @log as group",
            &events,
        );
        assert_eq!(
            table.columns,
            vec!["@timestamp", "@logStream", "group", "n"]
        );
        assert_eq!(
            table.rows,
            vec![vec![
                CellValue::Timestamp(FOUR_PM),
                text("stream-a"),
                text(GROUP),
                CellValue::Integer(2),
            ]]
        );
    }

    #[test]
    fn a_cell_is_written_as_the_table_shows_it() {
        // BR2.8, BR4.2: one function for every place that writes a cell.
        let format_time = |time: i64| format!("t{time}");
        assert_eq!(
            cell_text(&CellValue::Integer(600), &format_time).as_deref(),
            Some("600")
        );
        assert_eq!(
            cell_text(&CellValue::Decimal(200.333_333), &format_time).as_deref(),
            Some("200.3333")
        );
        assert_eq!(cell_text(&text("a"), &format_time).as_deref(), Some("a"));
        assert_eq!(
            cell_text(&CellValue::Timestamp(5), &format_time).as_deref(),
            Some("t5")
        );
        assert_eq!(cell_text(&CellValue::Missing, &format_time), None);
    }

    // ---- BR2.8 (Q5 A, functional-design review R-05): the rounding table --

    fn formatted(value: f64) -> Option<String> {
        format_aggregate(value)
    }

    #[test]
    fn a_whole_number_is_written_without_a_fraction() {
        assert_eq!(formatted(600.0).as_deref(), Some("600"));
        assert_eq!(formatted(-42.0).as_deref(), Some("-42"));
        assert_eq!(formatted(0.0).as_deref(), Some("0"));
    }

    #[test]
    fn a_fraction_is_rounded_to_four_places_and_its_trailing_zeros_dropped() {
        assert_eq!(formatted(200.333_333_3).as_deref(), Some("200.3333"));
        assert_eq!(formatted(1.5).as_deref(), Some("1.5"));
        assert_eq!(formatted(2.000_04).as_deref(), Some("2"));
        assert_eq!(formatted(-1.234_56).as_deref(), Some("-1.2346"));
    }

    #[test]
    fn half_of_the_last_place_rounds_away_from_zero() {
        assert_eq!(formatted(0.000_05).as_deref(), Some("0.0001"));
        assert_eq!(formatted(-0.000_05).as_deref(), Some("-0.0001"));
        assert_eq!(formatted(2.5).as_deref(), Some("2.5"));
    }

    #[test]
    fn a_value_that_rounds_to_zero_is_zero_without_a_sign() {
        assert_eq!(formatted(-0.000_04).as_deref(), Some("0"));
        assert_eq!(formatted(-0.0).as_deref(), Some("0"));
    }

    #[test]
    fn the_floating_point_sum_of_tenths_is_written_as_expected() {
        assert_eq!(formatted(0.1 + 0.2).as_deref(), Some("0.3"));
        assert_eq!(formatted(1.0 / 3.0).as_deref(), Some("0.3333"));
    }

    #[test]
    fn whole_numbers_from_two_to_the_fifty_third_are_written_in_full() {
        let two_53 = 9_007_199_254_740_992.0;
        assert_eq!(formatted(two_53).as_deref(), Some("9007199254740992"));
        assert_eq!(formatted(two_53 - 1.0).as_deref(), Some("9007199254740991"));
        assert_eq!(formatted(1e20).as_deref(), Some("100000000000000000000"));
        assert_eq!(formatted(-1e20).as_deref(), Some("-100000000000000000000"));
    }

    #[test]
    fn infinity_and_not_a_number_are_no_value() {
        assert_eq!(formatted(f64::INFINITY), None);
        assert_eq!(formatted(f64::NEG_INFINITY), None);
        assert_eq!(formatted(f64::NAN), None);
        assert_eq!(formatted(f64::MAX).map(|text| text.len()), Some(309));
    }

    #[test]
    fn a_large_fraction_keeps_its_four_places() {
        assert_eq!(
            formatted(123_456_789.123_456).as_deref(),
            Some("123456789.1235")
        );
        assert_eq!(formatted(1e-9).as_deref(), Some("0"));
    }
}
