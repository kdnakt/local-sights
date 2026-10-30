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
//! order of `sort` (BR2.2-BR2.4); since U2 also the texts of the distinct
//! values, the numbers for `pct` (ordered once, when the group is finished,
//! and ranked in whole numbers), Welford's running mean for `stddev`, and
//! the value of the first or last event in the result's order for
//! `earliest` and `latest` (U2:BR2.1-BR2.5). [`StatsAggregator::finish`] makes the rows
//! in ascending order of their keys, with "no value" last (BR2.7), and one
//! row for no event without `by` (BR2.6).
//!
//! The cells keep their type ([`CellValue`]); their text is made only when
//! they are shown, exported or printed, by [`cell_text`] with the screen's
//! time zone (BR2.8, BR4.2, A7). The details the documentation leaves open
//! (the key of events without the field, values that are no number, the row
//! for no event, the four decimal places, the default order) are provisional
//! until U4 compares them with the console.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, DefaultHasher, Hasher};

use crate::query::evaluator::number_of_digits;
use crate::query::fields::{EventFields, FieldValue};
use crate::query::natural::{SortKeyRef, SortValue, compare_sort_keys};
use crate::query::plan::{
    AggregateArgument, AggregateFunction, AggregateSpec, FieldRef, GroupKeyKind, Percentile,
    StatsCommand,
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

/// Where an event stands in the result's order (entities.md EventOrderKey):
/// its time, then its log group, stream and order in the stream
/// (261017-insights-query U2:BR4.4), here oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EventOrder {
    timestamp: i64,
    log_group_name: String,
    log_stream_name: String,
    sequence: u64,
}

impl EventOrder {
    fn of(fields: &EventFields<'_>) -> Self {
        let event = fields.event();
        Self {
            timestamp: event.timestamp,
            log_group_name: fields.log_group_name().to_string(),
            log_stream_name: event.log_stream_name.clone(),
            sequence: event.sequence,
        }
    }

    /// How the event of `fields` stands against this one, without copying
    /// its names.
    fn compare_with(&self, fields: &EventFields<'_>) -> Ordering {
        let event = fields.event();
        event
            .timestamp
            .cmp(&self.timestamp)
            .then_with(|| fields.log_group_name().cmp(self.log_group_name.as_str()))
            .then_with(|| event.log_stream_name.as_str().cmp(&self.log_stream_name))
            .then_with(|| event.sequence.cmp(&self.sequence))
    }
}

/// The running value of one aggregate of one group (entities.md
/// Accumulator).
#[derive(Debug, Clone)]
enum Accumulator {
    /// `count(*)` or `count(field)`: the events counted.
    Count(i64),
    /// `sum` or `avg`: the total and how many values were added.
    Sum { total: f64, added: u64 },
    /// `min`, `max`, `sortsFirst` or `sortsLast`: the value kept so far,
    /// with its order (U1:BR2.4, U2:BR2.4).
    Extreme(Option<(SortValue, CellValue)>),
    /// `count_distinct`: the 64-bit hashes of the texts of the values seen
    /// (U2:BR2.1, BR2.6's first remedy: no text is copied).
    Distinct(HashSet<u64, BuildHasherDefault<HashedKey>>),
    /// `pct`: every value read as a number, ordered at the end (U2:BR2.2).
    Values(Vec<f64>),
    /// `stddev`: Welford's count, mean and sum of squared differences
    /// (U2:BR2.3).
    Spread { count: u64, mean: f64, m2: f64 },
    /// `earliest` or `latest`: the value of the first (last) event in the
    /// result's order so far (U2:BR2.5).
    Chosen(Option<(EventOrder, CellValue)>),
}

impl Accumulator {
    fn new(function: AggregateFunction) -> Self {
        match function {
            AggregateFunction::Count => Accumulator::Count(0),
            AggregateFunction::Sum | AggregateFunction::Avg => Accumulator::Sum {
                total: 0.0,
                added: 0,
            },
            AggregateFunction::Min
            | AggregateFunction::Max
            | AggregateFunction::SortsFirst
            | AggregateFunction::SortsLast => Accumulator::Extreme(None),
            AggregateFunction::CountDistinct => Accumulator::Distinct(HashSet::default()),
            AggregateFunction::Pct => Accumulator::Values(Vec::new()),
            AggregateFunction::Stddev => Accumulator::Spread {
                count: 0,
                mean: 0.0,
                m2: 0.0,
            },
            AggregateFunction::Earliest | AggregateFunction::Latest => Accumulator::Chosen(None),
        }
    }

    /// Adds the event of `fields` (U1:BR2.2-BR2.4, U2:BR2.1-BR2.5).
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
                let wanted = match spec.function {
                    AggregateFunction::Min | AggregateFunction::SortsFirst => Ordering::Less,
                    _ => Ordering::Greater,
                };
                // Of two values in the same place, the first seen stays.
                let replaces = kept.as_ref().is_none_or(|(held, _)| {
                    compare_sort_keys(order.as_key(), held.as_key()) == wanted
                });
                if replaces {
                    *kept = Some((order, CellValue::of_value(field, value)));
                }
            }
            Accumulator::Distinct(seen) => {
                if let Some(text) = distinct_text(field, value) {
                    seen.insert(text_hash(&text));
                }
            }
            Accumulator::Values(values) => {
                if let Some(number) = number_of(value) {
                    values.push(number);
                }
            }
            Accumulator::Spread { count, mean, m2 } => {
                if let Some(number) = number_of(value) {
                    // Welford: no large sums of squares, so no lost digits.
                    *count += 1;
                    let delta = number - *mean;
                    *mean += delta / *count as f64;
                    *m2 += delta * (number - *mean);
                }
            }
            Accumulator::Chosen(chosen) => {
                if value == FieldValue::Missing {
                    return;
                }
                let wanted = if spec.function == AggregateFunction::Earliest {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                let replaces = chosen
                    .as_ref()
                    .is_none_or(|(held, _)| held.compare_with(fields) == wanted);
                if replaces {
                    *chosen = Some((EventOrder::of(fields), CellValue::of_value(field, value)));
                }
            }
        }
    }

    /// The cell of the finished aggregate of `spec`.
    fn into_cell(self, spec: &AggregateSpec) -> CellValue {
        match self {
            Accumulator::Count(count) => CellValue::Integer(count),
            Accumulator::Sum { added: 0, .. } => CellValue::Missing,
            Accumulator::Sum { total, added } => {
                if spec.function == AggregateFunction::Avg {
                    // At most 2^64 values: the quotient is what f64 can say.
                    CellValue::Decimal(total / added as f64)
                } else {
                    CellValue::Decimal(total)
                }
            }
            Accumulator::Extreme(kept) => kept.map_or(CellValue::Missing, |(_, cell)| cell),
            Accumulator::Distinct(seen) => {
                CellValue::Integer(i64::try_from(seen.len()).unwrap_or(i64::MAX))
            }
            Accumulator::Values(mut values) => {
                let rank = spec
                    .percentile
                    .and_then(|percentile| percentile_rank(percentile, values.len()));
                let Some(rank) = rank else {
                    return CellValue::Missing;
                };
                // Ordered once per group, when it is finished (BR2.2).
                values.sort_unstable_by(f64::total_cmp);
                values
                    .get(rank - 1)
                    .map_or(CellValue::Missing, |value| CellValue::Decimal(*value))
            }
            Accumulator::Spread { count: 0, .. } => CellValue::Missing,
            Accumulator::Spread { count: 1, .. } => CellValue::Decimal(0.0),
            Accumulator::Spread { count, m2, .. } => {
                CellValue::Decimal((m2 / (count - 1) as f64).sqrt())
            }
            Accumulator::Chosen(chosen) => chosen.map_or(CellValue::Missing, |(_, cell)| cell),
        }
    }
}

/// The 64-bit hash `count_distinct` keeps of a value's text (BR2.6): two
/// different texts share one with a chance of about n² / 2^65, below one in
/// ten million for a million values.
fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

/// The hasher of a set of [`text_hash`]es: the key is a hash already, so it
/// is used as it is.
#[derive(Debug, Clone, Copy, Default)]
struct HashedKey(u64);

impl Hasher for HashedKey {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // Only u64 keys are hashed here; any other input is folded in.
        for byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(*byte);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

/// The text `count_distinct` tells a value apart by (U2:BR2.1): the key text
/// of U1:BR2.1, borrowed when the value is a text; `None` for no value.
fn distinct_text<'a>(field: &FieldRef, value: FieldValue<'a>) -> Option<Cow<'a, str>> {
    match value {
        FieldValue::Missing => None,
        FieldValue::Text(text) => Some(Cow::Borrowed(text)),
        FieldValue::Boolean(flag) => Some(Cow::Borrowed(if flag { "1" } else { "0" })),
        FieldValue::Number(_) => key_text(&CellValue::of_value(field, value)).map(Cow::Owned),
    }
}

/// The 1-based rank of `percentile` among `count` ordered values
/// (261025-insights-stats U2:BR2.2): the nearest rank
/// `max(1, ceil(numerator × count ÷ (100 × 10^scale)))`, worked out in whole
/// numbers so that it is exact; `None` for no value.
pub fn percentile_rank(percentile: Percentile, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let whole = percentile.whole();
    // numerator ≤ 100 × 10^15 and count < 2^64: the product fits in u128.
    let product = u128::from(percentile.numerator).checked_mul(count as u128)?;
    let rank = product.div_ceil(whole).max(1);
    usize::try_from(rank).ok().map(|rank| rank.min(count))
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

    /// The `stats` command aggregated.
    pub fn stats(&self) -> &StatsCommand {
        &self.stats
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
        let specs = &self.stats.aggregates;
        let rows = groups
            .into_iter()
            .map(|group| {
                let mut cells = group.keys;
                cells.extend(
                    group
                        .accumulators
                        .into_iter()
                        .zip(specs)
                        .map(|(accumulator, spec)| accumulator.into_cell(spec)),
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

    // ---- 261025-insights-stats U2:BR2.1-BR2.5 ------------------------------

    /// An event of `stream` at `timestamp`, `sequence` in its stream.
    fn event_in(timestamp: i64, stream: &str, sequence: u64, message: &str) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: message.to_string(),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn decimal_of(cell: &CellValue) -> f64 {
        match cell {
            CellValue::Decimal(value) => *value,
            other => panic!("{other:?} is no decimal"),
        }
    }

    #[test]
    fn count_distinct_counts_values_shown_alike_once() {
        // BR2.1: 200 and "200" are one value, true and "1" too; a missing
        // value is not counted.
        let events = [
            json(1, r#"{"v":200}"#),
            json(2, r#"{"v":"200"}"#),
            json(3, r#"{"v":"a"}"#),
            json(4, r#"{"v":"a"}"#),
            json(5, r#"{"v":null}"#),
            json(6, "plain"),
            json(7, r#"{"v":true}"#),
            json(8, r#"{"v":"1"}"#),
            json(9, r#"{"v":2.00001}"#),
            json(10, r#"{"v":2}"#),
        ];
        assert_eq!(
            rows("stats count_distinct(v)", &events),
            vec![vec![CellValue::Integer(4)]],
            "200, a, 1 and 2 (2.00001 is shown 2)"
        );
    }

    #[test]
    fn count_distinct_counts_each_group_apart_and_zero_for_nothing() {
        // BR2.1, U1:BR2.6.
        let events = [
            json(1, r#"{"g":"x","u":"a"}"#),
            json(2, r#"{"g":"x","u":"b"}"#),
            json(3, r#"{"g":"y","u":"a"}"#),
            json(4, r#"{"g":"y","u":"a"}"#),
            json(5, r#"{"g":"z"}"#),
        ];
        assert_eq!(
            rows("stats count_distinct(u) by g", &events),
            vec![
                vec![text("x"), CellValue::Integer(2)],
                vec![text("y"), CellValue::Integer(1)],
                vec![text("z"), CellValue::Integer(0)],
            ]
        );
        assert_eq!(
            rows("stats count_distinct(u)", &[]),
            vec![vec![CellValue::Integer(0)]]
        );
    }

    #[test]
    fn count_distinct_of_the_time_counts_the_times() {
        // BR2.1: the time is told apart by its milliseconds.
        let events = [json(FOUR_PM, "a"), json(FOUR_PM, "b"), json(FOUR_PM + 1, "c")];
        assert_eq!(
            rows("stats count_distinct(@timestamp), count_distinct(@message)", &events),
            vec![vec![CellValue::Integer(2), CellValue::Integer(3)]]
        );
    }

    #[test]
    fn stddev_is_the_sample_standard_deviation_of_the_numbers() {
        // BR2.3: divided by n - 1; texts of digits are numbers, other values
        // are left out as in sum.
        let events = [
            json(1, r#"{"n":2}"#),
            json(2, r#"{"n":4}"#),
            json(3, r#"{"n":"4"}"#),
            json(4, r#"{"n":4}"#),
            json(5, r#"{"n":5}"#),
            json(6, r#"{"n":5}"#),
            json(7, r#"{"n":7}"#),
            json(8, r#"{"n":9}"#),
            json(9, r#"{"n":"abc"}"#),
            json(10, r#"{"n":true}"#),
            json(11, "plain"),
        ];
        let found = decimal_of(&rows("stats stddev(n)", &events)[0][0]);
        assert!((found - (32.0_f64 / 7.0).sqrt()).abs() < 1e-12, "{found}");
    }

    #[test]
    fn stddev_of_one_value_is_zero_and_of_none_no_value() {
        // BR2.3.
        assert_eq!(
            rows("stats stddev(n)", &[json(1, r#"{"n":42}"#)]),
            vec![vec![CellValue::Decimal(0.0)]]
        );
        assert_eq!(
            rows("stats stddev(n)", &[json(1, r#"{"m":42}"#)]),
            vec![vec![CellValue::Missing]]
        );
        assert_eq!(rows("stats stddev(n)", &[]), vec![vec![CellValue::Missing]]);
    }

    #[test]
    fn stddev_of_large_close_values_loses_no_digit() {
        // BR2.3: Welford's method, so values near 1e9 keep their spread.
        let events: Vec<LogEvent> = [4, 7, 13, 16]
            .iter()
            .map(|offset| json(1, &format!(r#"{{"n":{}}}"#, 1_000_000_000 + offset)))
            .collect();
        let found = decimal_of(&rows("stats stddev(n)", &events)[0][0]);
        assert!((found - 30.0_f64.sqrt()).abs() < 1e-9, "{found}");
        let many: Vec<LogEvent> = (0..10_000)
            .map(|index: i64| json(index, &format!(r#"{{"n":{}}}"#, 1_000_000_000 + index % 2)))
            .collect();
        let found = decimal_of(&rows("stats stddev(n)", &many)[0][0]);
        // Half 0s and half 1s around 1e9: sqrt(n / (4 (n - 1))).
        let expected = (10_000.0_f64 / (4.0 * 9_999.0)).sqrt();
        assert!((found - expected).abs() < 1e-9, "{found}");
    }

    #[test]
    fn sorts_first_and_sorts_last_are_the_values_min_and_max_give() {
        // BR2.4 (Q5 A): the order of sort; texts before numbers.
        let mixed = [
            json(1, r#"{"v":5}"#),
            json(2, r#"{"v":"x10"}"#),
            json(3, r#"{"v":"x2"}"#),
            json(4, r#"{"v":1.25}"#),
            json(5, "plain"),
        ];
        assert_eq!(
            rows("stats sortsFirst(v), sortsLast(v), min(v), max(v)", &mixed),
            vec![vec![
                text("x2"),
                CellValue::Integer(5),
                text("x2"),
                CellValue::Integer(5)
            ]]
        );
        let numbers = [json(1, r#"{"v":3}"#), json(2, r#"{"v":-1.5}"#)];
        assert_eq!(
            rows("stats sortsFirst(v), sortsLast(v)", &numbers),
            vec![vec![CellValue::Decimal(-1.5), CellValue::Integer(3)]]
        );
        assert_eq!(
            rows("stats sortsFirst(gone), sortsLast(gone)", &numbers),
            vec![vec![CellValue::Missing, CellValue::Missing]]
        );
    }

    #[test]
    fn earliest_and_latest_are_the_values_of_the_first_and_last_events_in_time() {
        // BR2.5: whatever order the events come in.
        let events = [
            json(FOUR_PM + 20, r#"{"level":"c"}"#),
            json(FOUR_PM, r#"{"level":"a"}"#),
            json(FOUR_PM + 30, r#"{"level":"d"}"#),
            json(FOUR_PM + 10, r#"{"level":"b"}"#),
        ];
        assert_eq!(
            rows("stats earliest(level), latest(level)", &events),
            vec![vec![text("a"), text("d")]]
        );
        assert_eq!(
            rows("stats earliest(@timestamp), latest(@timestamp)", &events),
            vec![vec![
                CellValue::Timestamp(FOUR_PM),
                CellValue::Timestamp(FOUR_PM + 30)
            ]],
            "of the time, times"
        );
    }

    #[test]
    fn earliest_and_latest_pass_over_events_without_the_field() {
        // BR2.5: a missing value is no candidate.
        let events = [
            json(FOUR_PM, "plain"),
            json(FOUR_PM + 1, r#"{"n":1}"#),
            json(FOUR_PM + 2, r#"{"n":2}"#),
            json(FOUR_PM + 3, r#"{"n":null}"#),
        ];
        assert_eq!(
            rows("stats earliest(n), latest(n)", &events),
            vec![vec![CellValue::Integer(1), CellValue::Integer(2)]]
        );
        assert_eq!(
            rows("stats earliest(gone), latest(gone)", &events),
            vec![vec![CellValue::Missing, CellValue::Missing]]
        );
    }

    #[test]
    fn of_events_at_the_same_time_earliest_takes_the_first_and_latest_the_last_in_order() {
        // BR2.5 (FR review R-07): equal times are ordered as the result
        // rows are, by log group, stream and order in the stream.
        let events = [
            event_in(FOUR_PM, "b", 0, r#"{"v":"b0"}"#),
            event_in(FOUR_PM, "a", 1, r#"{"v":"a1"}"#),
            event_in(FOUR_PM, "a", 0, r#"{"v":"a0"}"#),
            event_in(FOUR_PM, "b", 1, r#"{"v":"b1"}"#),
        ];
        assert_eq!(
            rows("stats earliest(v), latest(v)", &events),
            vec![vec![text("a0"), text("b1")]]
        );
        let mut aggregator = StatsAggregator::new(stats("stats earliest(v), latest(v)"));
        aggregator.add(&EventFields::new(&events[0], "/z"));
        aggregator.add(&EventFields::new(&events[3], "/a"));
        assert_eq!(
            aggregator.finish().rows,
            vec![vec![text("b1"), text("b0")]],
            "the log group comes before the stream"
        );
    }

    #[test]
    fn every_function_by_a_key_gives_the_keys_then_the_aggregates() {
        // U1:BR3.3 with U2's functions.
        let events = [
            json(FOUR_PM, r#"{"level":"e","latency":10,"user":"u1"}"#),
            json(FOUR_PM + 1, r#"{"level":"e","latency":30,"user":"u2"}"#),
            json(FOUR_PM + 2, r#"{"level":"i","latency":5,"user":"u1"}"#),
        ];
        let table = table(
            "stats pct(latency, 50) as p, stddev(latency), count_distinct(user), \
             earliest(user), latest(user), sortsFirst(user), sortsLast(user) by level",
            &events,
        );
        assert_eq!(
            table.columns,
            vec![
                "level",
                "p",
                "stddev(latency)",
                "count_distinct(user)",
                "earliest(user)",
                "latest(user)",
                "sortsFirst(user)",
                "sortsLast(user)"
            ]
        );
        assert_eq!(
            table.rows,
            vec![
                vec![
                    text("e"),
                    CellValue::Decimal(10.0),
                    CellValue::Decimal(200.0_f64.sqrt()),
                    CellValue::Integer(2),
                    text("u1"),
                    text("u2"),
                    text("u1"),
                    text("u2"),
                ],
                vec![
                    text("i"),
                    CellValue::Decimal(5.0),
                    CellValue::Decimal(0.0),
                    CellValue::Integer(1),
                    text("u1"),
                    text("u1"),
                    text("u1"),
                    text("u1"),
                ],
            ]
        );
    }

    #[test]
    fn no_event_gives_no_value_for_the_value_functions() {
        // U1:BR2.6 with U2's functions.
        assert_eq!(
            rows(
                "stats pct(a, 50), stddev(a), earliest(a), latest(a), sortsFirst(a), sortsLast(a)",
                &[]
            ),
            vec![vec![CellValue::Missing; 6]]
        );
    }

    // ---- 261025-insights-stats U2:BR2.2: the rank of pct ------------------

    /// The `pct(n, p)` of the numbers 1 to `count`, given in reverse.
    fn pct_of_one_to(count: i64, percentile: &str) -> CellValue {
        let events: Vec<LogEvent> = (1..=count)
            .rev()
            .map(|value| json(value, &format!(r#"{{"n":{value}}}"#)))
            .collect();
        let text = format!("stats pct(n, {percentile})");
        rows(&text, &events).remove(0).remove(0)
    }

    #[test]
    fn pct_of_one_value_is_that_value_for_every_percentile() {
        for percentile in ["0", "7", "50", "99.9", "100"] {
            assert_eq!(
                pct_of_one_to(1, percentile),
                CellValue::Decimal(1.0),
                "p {percentile}"
            );
        }
    }

    #[test]
    fn pct_of_a_hundred_values_is_the_nearest_rank() {
        for (percentile, rank) in [("0", 1.0), ("7", 7.0), ("50", 50.0), ("99.9", 100.0), ("100", 100.0)] {
            assert_eq!(
                pct_of_one_to(100, percentile),
                CellValue::Decimal(rank),
                "p {percentile}"
            );
        }
    }

    #[test]
    fn pct_of_a_hundred_and_one_values_rounds_the_rank_up() {
        // ceil(7 × 101 / 100) = 8, ceil(50.5) = 51, ceil(100.899) = 101.
        for (percentile, rank) in [("0", 1.0), ("7", 8.0), ("50", 51.0), ("99.9", 101.0), ("100", 101.0)] {
            assert_eq!(
                pct_of_one_to(101, percentile),
                CellValue::Decimal(rank),
                "p {percentile}"
            );
        }
    }

    #[test]
    fn pct_of_no_value_is_no_value() {
        assert_eq!(rows("stats pct(n, 50)", &[]), vec![vec![CellValue::Missing]]);
        assert_eq!(
            rows("stats pct(n, 50)", &[json(1, r#"{"n":"abc"}"#)]),
            vec![vec![CellValue::Missing]]
        );
    }

    #[test]
    fn pct_reads_only_what_reads_as_a_number() {
        // BR2.2: as sum does (U1:BR2.3).
        let events = [
            json(1, r#"{"n":"abc"}"#),
            json(2, r#"{"n":true}"#),
            json(3, r#"{"n":20}"#),
            json(4, "plain"),
            json(5, r#"{"n":"10"}"#),
            json(6, r#"{"n":null}"#),
        ];
        assert_eq!(
            rows("stats pct(n, 50), pct(n, 51)", &events),
            vec![vec![CellValue::Decimal(10.0), CellValue::Decimal(20.0)]]
        );
    }

    #[test]
    fn pct_orders_negative_numbers_and_fractions() {
        let events = [
            json(1, r#"{"n":0.5}"#),
            json(2, r#"{"n":-3}"#),
            json(3, r#"{"n":2}"#),
            json(4, r#"{"n":-0.25}"#),
        ];
        assert_eq!(
            rows("stats pct(n, 25), pct(n, 26), pct(n, 75), pct(n, 100)", &events),
            vec![vec![
                CellValue::Decimal(-3.0),
                CellValue::Decimal(-0.25),
                CellValue::Decimal(0.5),
                CellValue::Decimal(2.0)
            ]]
        );
    }

    #[test]
    fn the_rank_is_worked_out_in_whole_numbers() {
        // BR2.2 (FR review R-05): no floating point, even for large counts.
        use crate::query::plan::Percentile;
        let p = |text: &str| Percentile::parse(text).unwrap();
        assert_eq!(percentile_rank(p("50"), 0), None);
        assert_eq!(percentile_rank(p("0"), 5), Some(1));
        assert_eq!(percentile_rank(p("100"), 5), Some(5));
        assert_eq!(percentile_rank(p("99.9"), 1_000_000_007), Some(999_000_007));
        assert_eq!(percentile_rank(p("0.000000000000001"), 1_000_000), Some(1));
        assert_eq!(percentile_rank(p("33.3"), 1_000), Some(333));
        assert_eq!(percentile_rank(p("33.3"), 1_001), Some(334));
    }
}
