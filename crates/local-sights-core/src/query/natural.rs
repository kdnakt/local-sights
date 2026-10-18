//! The order of `sort` (261017-insights-query U2:BR4.2, BR4.3, AWS's sort
//! documentation, functional-design review R-05).
//!
//! In ascending order a missing value is the smallest (Q1a 5h, 5i), then
//! come all values that are no numbers, then all numbers. Numbers are
//! `@timestamp` (milliseconds) and JSON numbers; a text is never a number
//! here, even when it is only digits, and a boolean is the text `true` or
//! `false` (R-05). Numbers compare as numbers. Texts compare in the natural
//! order of the documentation: each is cut into runs of digits and runs of
//! other characters; two runs of other characters compare in Unicode order,
//! two runs of digits by their number of digits and then their value, and a
//! run of digits against another run by their first characters. Descending
//! is the exact reverse.

use std::cmp::Ordering;

use crate::query::fields::FieldValue;

/// One `sort` value read from an event, owned so it outlives the read
/// (functional-design review R-02: read once per row, then only compared).
#[derive(Debug, Clone, PartialEq)]
pub enum SortValue {
    /// No value: the smallest.
    Missing,
    /// Not a number: a text, or a boolean as its text.
    Text(String),
    /// A number.
    Number(f64),
}

impl SortValue {
    /// The sort value of `value` (R-05).
    pub fn of(value: FieldValue<'_>) -> Self {
        match SortKeyRef::of(value) {
            SortKeyRef::Missing => SortValue::Missing,
            SortKeyRef::Number(number) => SortValue::Number(number),
            SortKeyRef::Text(text) => SortValue::Text(text.to_string()),
        }
    }

    /// The same value, borrowed.
    pub fn as_key(&self) -> SortKeyRef<'_> {
        match self {
            SortValue::Missing => SortKeyRef::Missing,
            SortValue::Text(text) => SortKeyRef::Text(text),
            SortValue::Number(number) => SortKeyRef::Number(*number),
        }
    }
}

/// A `sort` value borrowed from where it is held: a fixed field of an
/// event, or a [`SortValue`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKeyRef<'a> {
    /// No value: the smallest.
    Missing,
    /// Not a number.
    Text(&'a str),
    /// A number.
    Number(f64),
}

impl<'a> SortKeyRef<'a> {
    /// The sort value of `value` (R-05): a boolean is its text.
    pub fn of(value: FieldValue<'a>) -> Self {
        match value {
            FieldValue::Missing => SortKeyRef::Missing,
            FieldValue::Number(number) => SortKeyRef::Number(number),
            FieldValue::Text(text) => SortKeyRef::Text(text),
            FieldValue::Boolean(true) => SortKeyRef::Text("true"),
            FieldValue::Boolean(false) => SortKeyRef::Text("false"),
        }
    }

    /// Missing, then texts, then numbers.
    fn rank(self) -> u8 {
        match self {
            SortKeyRef::Missing => 0,
            SortKeyRef::Text(_) => 1,
            SortKeyRef::Number(_) => 2,
        }
    }
}

/// `left` against `right` in ascending order (U2:BR4.2, BR4.3).
pub fn compare_sort_keys(left: SortKeyRef<'_>, right: SortKeyRef<'_>) -> Ordering {
    match (left, right) {
        (SortKeyRef::Text(left), SortKeyRef::Text(right)) => natural_cmp(left, right),
        (SortKeyRef::Number(left), SortKeyRef::Number(right)) => left.total_cmp(&right),
        _ => left.rank().cmp(&right.rank()),
    }
}

/// `left` against `right` in ascending order (U2:BR4.2, BR4.3).
pub fn compare_sort_values(left: &SortValue, right: &SortValue) -> Ordering {
    compare_sort_keys(left.as_key(), right.as_key())
}

/// The runs of `text`: each a run of ASCII digits or a run of other
/// characters.
fn runs(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let digits = first.is_ascii_digit();
        let end = rest
            .char_indices()
            .find(|(_, c)| c.is_ascii_digit() != digits)
            .map_or(rest.len(), |(index, _)| index);
        let (run, after) = rest.split_at(end);
        rest = after;
        Some(run)
    })
}

/// Two runs: digits by their number and then their value, others in
/// Unicode order, a run of digits against another by the first characters.
fn compare_runs(left: &str, right: &str) -> Ordering {
    let is_digits = |run: &str| run.starts_with(|c: char| c.is_ascii_digit());
    match (is_digits(left), is_digits(right)) {
        // ASCII digits are one byte each: the length counts the digits, and
        // of the same length the text order is the value order.
        (true, true) => left.len().cmp(&right.len()).then_with(|| left.cmp(right)),
        (false, false) => left.cmp(right),
        _ => left.chars().next().cmp(&right.chars().next()),
    }
}

/// Two texts in the natural order of the documentation (U2:BR4.2).
pub fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left_runs = runs(left);
    let mut right_runs = runs(right);
    loop {
        match (left_runs.next(), right_runs.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left), Some(right)) => {
                let order = compare_runs(left, right);
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> SortValue {
        SortValue::Text(value.to_string())
    }

    fn ascending(mut values: Vec<SortValue>) -> Vec<SortValue> {
        values.sort_by(compare_sort_values);
        values
    }

    #[test]
    fn the_documentation_example_puts_texts_in_natural_order_before_numbers() {
        // AWS's sort documentation, ascending. Its "01" cannot be a JSON
        // number, so the numbers here are 0, 1, 2 and 3.
        let expected = vec![
            text("!:"),
            text("#"),
            text("*%04"),
            text("0#"),
            text("5A"),
            text("111A"),
            text("2345_"),
            text("@"),
            text("@_"),
            text("A"),
            text("A9876fghj"),
            text("a12345hfh"),
            SortValue::Number(0.0),
            SortValue::Number(1.0),
            SortValue::Number(2.0),
            SortValue::Number(3.0),
        ];
        let mut shuffled = expected.clone();
        shuffled.reverse();
        shuffled.swap(0, 7);
        shuffled.swap(3, 12);
        assert_eq!(ascending(shuffled), expected);
    }

    #[test]
    fn runs_of_digits_compare_by_their_length_then_their_value() {
        // U2:BR4.2.
        assert_eq!(natural_cmp("x2", "x10"), Ordering::Less);
        assert_eq!(natural_cmp("x10", "x9"), Ordering::Greater);
        assert_eq!(
            natural_cmp("x4", "x04"),
            Ordering::Less,
            "more digits is larger"
        );
        assert_eq!(natural_cmp("v1.10", "v1.9"), Ordering::Greater);
        assert_eq!(natural_cmp("abc", "abc"), Ordering::Equal);
        assert_eq!(natural_cmp("ab", "abc"), Ordering::Less);
        assert_eq!(natural_cmp("エラー2", "エラー10"), Ordering::Less);
    }

    #[test]
    fn q1a_5h_5i_a_missing_value_is_the_smallest() {
        // Ascending puts it first (5h), descending last (5i).
        let values = vec![SortValue::Number(1.0), SortValue::Missing, text("a")];
        let sorted = ascending(values.clone());
        assert_eq!(sorted.first(), Some(&SortValue::Missing));
        let mut descending = values;
        descending.sort_by(|left, right| compare_sort_values(right, left));
        assert_eq!(descending.last(), Some(&SortValue::Missing));
        assert_eq!(
            compare_sort_values(&SortValue::Missing, &SortValue::Missing),
            Ordering::Equal
        );
    }

    #[test]
    fn a_text_of_digits_stays_with_the_texts_and_booleans_are_texts() {
        // R-05.
        assert_eq!(SortValue::of(FieldValue::Text("5")), text("5"));
        assert_eq!(
            compare_sort_values(&text("5"), &SortValue::Number(1.0)),
            Ordering::Less
        );
        assert_eq!(SortValue::of(FieldValue::Boolean(true)), text("true"));
        assert_eq!(SortValue::of(FieldValue::Boolean(false)), text("false"));
        assert_eq!(SortValue::of(FieldValue::Missing), SortValue::Missing);
    }

    #[test]
    fn the_timestamp_and_json_numbers_sort_as_numbers() {
        // `sort @timestamp`: milliseconds, not their digits.
        let early = SortValue::of(FieldValue::Number(999.0));
        let late = SortValue::of(FieldValue::Number(1_756_053_806_123.0));
        assert_eq!(late, SortValue::Number(1_756_053_806_123.0));
        assert_eq!(compare_sort_values(&early, &late), Ordering::Less);
        assert_eq!(
            compare_sort_values(&SortValue::Number(-1.5), &SortValue::Number(-1.0)),
            Ordering::Less
        );
    }
}
