//! QueryEvaluator: applies a read plan to the events the query screen
//! fetched (261017-insights-query U1:BR5.1-BR5.5, BR5.8, BR5.10).
//!
//! The entrance takes the events in time order with the log group each
//! belongs to ([`EvalItem`], unit-of-work-dependency's evaluation entrance)
//! and goes through them [`EVALUATION_CHUNK`] at a time. Between two chunks
//! the caller can stop it (a [Cancel], a newer run, a connection change),
//! and the rows found so far are then dropped (U1:BR5.8). The result keeps
//! the positions of the matching events in the holdings, in time order.

use std::cmp::Ordering;
use std::ops::Range;

use crate::event::LogEvent;
use crate::query::fields::{FieldValue, read_field};
use crate::query::plan::{CompareOp, Condition, LikePattern, Literal, QueryPlan};

/// Events evaluated between two looks at the stop signal (U1:BR5.8). May
/// be changed after measuring NFR1 and NFR2.
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

/// Whether `item` meets every filter of `plan` (U1:BR3.2).
pub fn matches(plan: &QueryPlan, item: &EvalItem<'_>) -> bool {
    plan.filters
        .iter()
        .all(|condition| condition_holds(condition, item))
}

/// Whether `condition` holds for `item`, from left to right, stopping as
/// soon as the answer is known (U1:BR5.2).
pub fn condition_holds(condition: &Condition, item: &EvalItem<'_>) -> bool {
    let field = |field| read_field(field, item.event, item.log_group_name);
    match condition {
        Condition::And(left, right) => condition_holds(left, item) && condition_holds(right, item),
        Condition::Or(left, right) => condition_holds(left, item) || condition_holds(right, item),
        Condition::Not(inner) => !condition_holds(inner, item),
        Condition::Compare {
            field: name,
            op,
            value,
        } => compare(field(*name), *op, value),
        Condition::Like {
            field: name,
            pattern,
            negated,
        } => like(field(*name), pattern) != *negated,
        Condition::Match { field: name, regex } => match field(*name) {
            FieldValue::Text(text) => regex.is_match(text),
            FieldValue::Number(_) => false,
        },
        Condition::In {
            field: name,
            values,
            negated,
        } => {
            let value = field(*name);
            values
                .iter()
                .any(|literal| compare(value, CompareOp::Eq, literal))
                != *negated
        }
    }
}

/// U1:BR5.1, BR5.10: the time against a number or the milliseconds of a
/// date text, a text against a text in character order. Values of
/// different types never meet a condition except `!=` (provisional until
/// U2, OQ2); an unresolved date text counts as a different type.
fn compare(value: FieldValue<'_>, op: CompareOp, literal: &Literal) -> bool {
    let ordering = match (value, literal) {
        (FieldValue::Number(time), Literal::Number(number)) => {
            // Exact for every time an i64 of milliseconds can show today
            // (below 2^53).
            (time as f64).partial_cmp(number)
        }
        (FieldValue::Number(time), Literal::Instant(instant)) => Some(time.cmp(instant)),
        (FieldValue::Text(text), Literal::Text(other)) => Some(text.cmp(other.as_str())),
        _ => None,
    };
    let Some(ordering) = ordering else {
        return op == CompareOp::NotEq;
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

/// U1:BR5.3, BR5.4: a text contains the text (case-sensitive), or some part
/// of it matches the expression.
fn like(value: FieldValue<'_>, pattern: &LikePattern) -> bool {
    let FieldValue::Text(text) = value else {
        return false;
    };
    match pattern {
        LikePattern::Text(part) => text.contains(part.as_str()),
        LikePattern::Regex(regex) => regex.is_match(text),
    }
}

/// One evaluation in progress, chunk by chunk (U1:BR5.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    total: usize,
    next: usize,
    rows: Vec<usize>,
}

impl Evaluation {
    /// An evaluation of `total` held events, none evaluated yet.
    pub fn new(total: usize) -> Self {
        Self {
            total,
            next: 0,
            rows: Vec::new(),
        }
    }

    /// The positions of the next chunk, or `None` when every event was
    /// evaluated.
    pub fn next_chunk(&self) -> Option<Range<usize>> {
        (self.next < self.total)
            .then(|| self.next..self.next.saturating_add(EVALUATION_CHUNK).min(self.total))
    }

    /// Evaluates `items`, the events at the positions of
    /// [`Evaluation::next_chunk`], in order. Items beyond the total are
    /// ignored.
    pub fn evaluate_chunk(&mut self, plan: &QueryPlan, items: &[EvalItem<'_>]) {
        let room = self.total - self.next;
        for (offset, item) in items.iter().take(room).enumerate() {
            if matches(plan, item) {
                self.rows.push(self.next + offset);
            }
        }
        self.next += items.len().min(room);
    }

    /// Events evaluated so far.
    pub fn evaluated(&self) -> usize {
        self.next
    }

    /// Events to evaluate in all.
    pub fn total(&self) -> usize {
        self.total
    }

    /// Positions of the matching events, in time order.
    pub fn into_rows(self) -> Vec<usize> {
        self.rows
    }
}

/// The result of one finished evaluation (entities.md QueryResult). The
/// rows are positions in the query holdings of `holdings_epoch`, in time
/// order (functional-design review R-05; entities.md's "BR7.3" for the
/// order is a slip for BR6.3); there is no upper limit on their number (Q3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    /// The run that made it.
    pub run_id: u64,
    /// The holdings epoch the positions point into.
    pub holdings_epoch: u64,
    /// Events evaluated.
    pub evaluated_count: usize,
    rows: Vec<usize>,
}

impl QueryResult {
    /// The result of run `run_id` over the holdings of `holdings_epoch`.
    pub fn new(run_id: u64, holdings_epoch: u64, evaluated_count: usize, rows: Vec<usize>) -> Self {
        Self {
            run_id,
            holdings_epoch,
            evaluated_count,
            rows,
        }
    }

    /// Number of matching events.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The positions of at most `limit` rows from `offset`, when the screen
    /// asks for this result's run and epoch; nothing otherwise, and the
    /// screen waits for the newer state (U1:BR6.2, BR6.4).
    pub fn window(
        &self,
        run_id: u64,
        holdings_epoch: u64,
        offset: usize,
        limit: usize,
    ) -> &[usize] {
        if run_id != self.run_id || holdings_epoch != self.holdings_epoch {
            return &[];
        }
        let start = offset.min(self.rows.len());
        let end = start.saturating_add(limit).min(self.rows.len());
        &self.rows[start..end]
    }
}

/// Evaluates `items` (the whole holdings in time order) chunk by chunk.
/// Before each chunk `stop` is asked; when it says so, the rows found so far
/// are dropped and `None` is returned. `progress` hears the evaluated and
/// total counts after each chunk (U1:BR2.4, BR5.8).
pub fn evaluate(
    plan: &QueryPlan,
    items: &[EvalItem<'_>],
    mut stop: impl FnMut() -> bool,
    mut progress: impl FnMut(usize, usize),
) -> Option<Vec<usize>> {
    let mut evaluation = Evaluation::new(items.len());
    while let Some(chunk) = evaluation.next_chunk() {
        if stop() {
            return None;
        }
        evaluation.evaluate_chunk(plan, &items[chunk]);
        progress(evaluation.evaluated(), evaluation.total());
    }
    Some(evaluation.into_rows())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::instant::resolve_date_texts;
    use crate::query::parser::parse_query;

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

    fn rows(text: &str, events: &[LogEvent]) -> Vec<usize> {
        evaluate(&query(text), &items(events), || false, |_, _| {}).unwrap()
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
    fn strings_compare_whole_and_in_character_order() {
        // U1:BR5.1.
        let event = event(1, "web-2", 0, "Hello");
        assert!(holds("filter @message = 'Hello'", &event));
        assert!(!holds("filter @message = 'hello'", &event), "case counts");
        assert!(!holds("filter @message = 'Hell'", &event), "the whole text");
        assert!(holds("filter @message != 'hello'", &event));
        assert!(holds("filter @logStream > 'web-1'", &event));
        assert!(holds("filter @logStream <= 'web-2'", &event));
        assert!(!holds("filter @logStream < 'web-2'", &event));
        assert!(holds("filter @log = '/aws/lambda/orders'", &event));
        assert!(
            holds("filter @message < 'a'", &event),
            "'H' sorts before 'a'"
        );
    }

    #[test]
    fn a_text_field_compared_with_a_number_only_meets_not_equal() {
        // U1:BR5.1 (provisional until U2, OQ2).
        let event = event(1, "s", 0, "42");
        assert!(!holds("filter @message = 42", &event));
        assert!(!holds("filter @message > 1", &event));
        assert!(!holds("filter @message <= 100", &event));
        assert!(holds("filter @message != 42", &event));
        assert!(!holds("filter @message in [42]", &event));
        assert!(holds("filter @message not in [42]", &event));
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
        // U1:BR5.10, BR5.9: numbers, decimals and date texts.
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
            vec![1, 2]
        );
        assert_eq!(
            rows("filter @timestamp < 1756053806000.5", &events),
            vec![0, 1]
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
                "filter @logStream = 'b' or @message like 'INFO' | filter not @message like 'z'",
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
            &items,
            || false,
            |done, total| seen.push((done, total)),
        );
        assert_eq!(all.map(|rows| rows.len()), Some(25_000));
        assert_eq!(
            seen,
            vec![(10_000, 25_000), (20_000, 25_000), (25_000, 25_000)]
        );

        let mut asked = 0;
        let mut seen = Vec::new();
        let stopped = evaluate(
            &plan,
            &items,
            || {
                asked += 1;
                asked > 1
            },
            |done, total| seen.push((done, total)),
        );
        assert_eq!(stopped, None, "the first chunk's rows are dropped");
        assert_eq!(seen, vec![(10_000, 25_000)]);

        let empty = evaluate(&plan, &[], || true, |_, _| panic!("no chunk"));
        assert_eq!(empty, Some(Vec::new()), "nothing to evaluate");
    }

    // ---- U1:BR6.3, BR6.4: reading the rows of a result -----------------

    fn result() -> QueryResult {
        QueryResult::new(3, 7, 10, vec![1, 4, 5, 8, 9])
    }

    #[test]
    fn the_rows_of_the_asked_run_and_epoch_are_given_by_offset_and_limit() {
        let result = result();
        assert_eq!(result.window(3, 7, 1, 3), &[4, 5, 8]);
        assert_eq!(result.window(3, 7, 0, 5), &[1, 4, 5, 8, 9]);
        assert_eq!(result.row_count(), 5);
        assert_eq!(result.evaluated_count, 10);
    }

    #[test]
    fn another_run_gets_no_rows() {
        assert!(result().window(2, 7, 0, 5).is_empty());
        assert!(result().window(4, 7, 0, 5).is_empty());
    }

    #[test]
    fn another_holdings_epoch_gets_no_rows() {
        // A connection change or a newer run discarded the holdings.
        assert!(result().window(3, 8, 0, 5).is_empty());
        assert!(result().window(3, 6, 0, 5).is_empty());
    }

    #[test]
    fn a_window_beyond_the_rows_is_cut_at_the_end() {
        let result = result();
        assert_eq!(result.window(3, 7, 3, 100), &[8, 9]);
        assert!(result.window(3, 7, 5, 10).is_empty());
        assert!(result.window(3, 7, usize::MAX, usize::MAX).is_empty());
        assert!(result.window(3, 7, 0, 0).is_empty());
    }

    #[test]
    fn a_result_without_matches_has_no_rows() {
        let empty = QueryResult::new(1, 1, 50, Vec::new());
        assert_eq!(empty.row_count(), 0);
        assert!(empty.window(1, 1, 0, 100).is_empty());
    }
}
