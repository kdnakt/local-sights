//! Comparing one query (261017-insights-query U4:BR2.1, BR2.3-BR2.5,
//! entities.md QueryComparison).
//!
//! In this order:
//!
//! 1. An export that is missing or did not complete is not compared
//!    (NotCompared, review R-02).
//! 2. A refusal: both refusing is no difference (BothRejected), one side
//!    refusing is (OnlyAppRejected, OnlyInsightsRejected) (BR2.4).
//! 3. A result cut at the row cap (the `limit`, or 10,000) of a query
//!    without `sort` is not compared: which events are kept depends on each
//!    side's order (Skipped, BR2.5).
//! 4. The column names, `@ptr` aside, must be the same when both sides have
//!    rows (BR2.1); a side without rows has no columns.
//! 5. A sorted result cut where the last kept row and the first row cut
//!    share their sort value ("a tie at the cut", review R-01): the rows with
//!    that value are left out of the comparison on both sides, since either
//!    side may keep any of them, and only the counts are compared for them.
//! 6. The rows are compared as multisets of `(column, value)` pairs (BR2.1).
//! 7. For a query compared in order, the sort values are compared row by
//!    row; rows with the same sort values may come in any order (BR2.3).
//!
//! 261025-insights-stats U4 compares aggregate rows (`stats`) through the
//! same seven steps (BR2.1), with these additions:
//!
//! - Step 3: an aggregates result without `sort` cut by its `limit` or at
//!   10,000 is Skipped too (BR2.4).
//! - Step 5: for a sorted aggregates result that was cut, the rows with the
//!   last shown row's sort value are left out on both sides and only their
//!   counts compared, since the console's next row is not in its export
//!   (BR2.7 (4)).
//! - Step 6: with a column tolerance (the query set's fifth field), the rows
//!   are paired by their columns without a tolerance and only the tolerant
//!   cells of a pair are compared, decimals within 1e-6 relative or 0.0001
//!   absolute (BR2.2) and `count_distinct` within 2% (BR2.3) being the same
//!   (BR2.6). The cells within a tolerance are counted apart (BR2.5). A
//!   difference in a row of the first or the last bin of the period is
//!   listed apart as a period edge and is still a difference (BR2.7 (5)).
//! - Steps 5 and 7: decimal sort values within the tolerance are the same
//!   rank (BR2.2).
//! - After step 6, a keyorder query compares the order of the key values
//!   (BR2.8).

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use crate::compat::export::InsightsSide;
use crate::compat::format::Row;
use crate::compat::queries::{ColumnTolerance, CompareMode, ToleranceKind, tolerance_of};

/// Rows of each side listed when the collections differ (BR3.2), and cells
/// within a tolerance listed as examples (U4:BR2.5).
pub const LISTED_ROWS: usize = 5;

/// BR2.2: the absolute error of two decimals taken as the same.
pub const DECIMAL_ABSOLUTE: f64 = 0.000_1;

/// BR2.2: the error relative to the larger of two decimals taken as the
/// same.
pub const DECIMAL_RELATIVE: f64 = 1e-6;

/// BR2.3: the error of two `count_distinct` counts taken as the same, as a
/// fraction of the larger (2% = 1/50).
pub const APPROXIMATE_PARTS: i128 = 50;

/// The binary error of reading the decimal texts: `0.5001 - 0.5` is a little
/// above 0.0001 as an `f64`. Far below the four decimal places written.
const READING_SLACK: f64 = 1e-12;

/// What the app made of one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppSide {
    /// The app refused the query (a QueryProblem), with its description.
    Rejected(String),
    /// The app's event rows.
    Rows(AppRows),
    /// The app's aggregate rows (`stats`, 261025-insights-stats U4:BR2.1,
    /// entities.md AppRows.shape).
    Aggregates(AppRows),
}

/// How one cell compares (entities.md CellDifference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellDifference {
    /// The same text.
    Exact,
    /// Two numbers within the decimal tolerance (BR2.2).
    WithinDecimal,
    /// Two whole numbers within the approximate tolerance (BR2.3).
    WithinApproximate,
    /// A difference.
    Different,
}

/// A cell taken as the same within a tolerance (BR2.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinCell {
    /// The column.
    pub column: String,
    /// Logs Insights' value.
    pub insights: String,
    /// The app's value.
    pub app: String,
    /// WithinDecimal or WithinApproximate.
    pub kind: CellDifference,
}

/// The app's result of one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRows {
    /// The result's column names, empty columns left out (U2:BR3.4).
    pub columns: Vec<String>,
    /// The rows in the result's order.
    pub rows: Vec<Row>,
    /// Whether rows that matched were cut by the `limit` or at 10,000.
    pub cut: bool,
    /// When cut and sorted, the first row cut (review R-01).
    pub next_row: Option<Row>,
}

/// How one query is compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareSpec {
    /// Order or set (the query set's second field).
    pub mode: CompareMode,
    /// Whether the query has a `sort`.
    pub sorted: bool,
    /// The fields of the last `sort`, when every one of them is a column of
    /// the result; empty when one is not (the sort values cannot be seen).
    pub sort_keys: Vec<String>,
    /// The row cap: the last `limit` from 1, or 10,000 (U2:BR5.1, BR5.2);
    /// for aggregates the last `limit` after `stats` (BR2.7 (3)).
    pub cap: usize,
    /// The column tolerances (BR1.3); empty when every cell is compared
    /// exactly.
    pub tolerances: Vec<ColumnTolerance>,
    /// The key columns of an aggregates result, whose order a keyorder
    /// query compares (BR2.8).
    pub key_columns: Vec<String>,
    /// `(column, value)` of the first and the last bin of the period
    /// (BR2.7 (5)); empty without a `bin` key.
    pub period_edges: Vec<(String, String)>,
}

/// The outcome of one query (entities.md QueryComparison.verdict, R-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Same rows (and order, when compared in order).
    Same,
    /// A difference.
    Different,
    /// Both refused the query: no difference (BR2.4).
    BothRejected,
    /// Only the app refused it.
    OnlyAppRejected,
    /// Only Logs Insights refused it.
    OnlyInsightsRejected,
    /// Cut at the row cap without `sort` (BR2.5), or the sort values are
    /// not among the columns.
    Skipped,
    /// The export is missing or did not complete (R-02).
    NotCompared,
}

impl Verdict {
    /// The name in the report.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Same => "Same",
            Verdict::Different => "Different",
            Verdict::BothRejected => "BothRejected",
            Verdict::OnlyAppRejected => "OnlyAppRejected",
            Verdict::OnlyInsightsRejected => "OnlyInsightsRejected",
            Verdict::Skipped => "Skipped",
            Verdict::NotCompared => "NotCompared",
        }
    }

    /// Whether the two sides were really compared (R-02).
    pub fn is_compared(self) -> bool {
        !matches!(self, Verdict::Skipped | Verdict::NotCompared)
    }

    /// Whether this is a difference (BR2.6).
    pub fn is_difference(self) -> bool {
        matches!(
            self,
            Verdict::Different | Verdict::OnlyAppRejected | Verdict::OnlyInsightsRejected
        )
    }
}

/// The column names on one side only (BR2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDifference {
    /// Columns only Logs Insights has.
    pub only_insights: Vec<String>,
    /// Columns only the app has.
    pub only_app: Vec<String>,
}

/// One query's comparison (entities.md QueryComparison).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryComparison {
    /// The outcome.
    pub verdict: Verdict,
    /// Logs Insights' row count, when it has rows.
    pub insights_count: Option<usize>,
    /// The app's row count, when it has rows.
    pub app_count: Option<usize>,
    /// Rows only Logs Insights has, the first [`LISTED_ROWS`].
    pub missing_in_app: Vec<Row>,
    /// Rows only the app has, the first [`LISTED_ROWS`].
    pub missing_in_insights: Vec<Row>,
    /// Rows only Logs Insights has in the first or the last bin of the
    /// period, the first [`LISTED_ROWS`] (U4:BR2.7 (5)).
    pub edge_missing_in_app: Vec<Row>,
    /// Rows only the app has in the first or the last bin of the period,
    /// the first [`LISTED_ROWS`].
    pub edge_missing_in_insights: Vec<Row>,
    /// Cells taken as the same within the decimal tolerance (U4:BR2.5).
    pub within_decimal: usize,
    /// Cells taken as the same within the approximate tolerance.
    pub within_approximate: usize,
    /// The first [`LISTED_ROWS`] cells within a tolerance.
    pub within_examples: Vec<WithinCell>,
    /// The first row (from 1) whose sort values differ (BR2.3), or, for a
    /// keyorder query, whose key values differ (U4:BR2.8).
    pub first_order_mismatch: Option<usize>,
    /// The column names on one side only (BR2.1).
    pub columns: Option<ColumnDifference>,
    /// R-01: the rows left out as a tie at the cut, `(Logs Insights, app)`.
    pub tied_at_cut: Option<(usize, usize)>,
    /// Logs Insights' refusal (BR2.4).
    pub insights_message: Option<String>,
    /// The app's refusal (BR2.4).
    pub app_message: Option<String>,
    /// Why the query was Skipped or NotCompared.
    pub note: Option<String>,
}

impl QueryComparison {
    fn new(verdict: Verdict) -> Self {
        Self {
            verdict,
            insights_count: None,
            app_count: None,
            missing_in_app: Vec::new(),
            missing_in_insights: Vec::new(),
            edge_missing_in_app: Vec::new(),
            edge_missing_in_insights: Vec::new(),
            within_decimal: 0,
            within_approximate: 0,
            within_examples: Vec::new(),
            first_order_mismatch: None,
            columns: None,
            tied_at_cut: None,
            insights_message: None,
            app_message: None,
            note: None,
        }
    }
}

/// Compares one query (BR2.1, BR2.3-BR2.5, R-01, R-02).
pub fn compare_query(
    spec: &CompareSpec,
    app: &AppSide,
    insights: &InsightsSide,
) -> QueryComparison {
    match (app, insights) {
        (_, InsightsSide::Missing) => not_compared("no export file for this query"),
        (_, InsightsSide::NotRun(status)) => not_compared(&format!(
            "the Logs Insights query ended {status}, not Complete"
        )),
        // 261017-insights-query U4 code review R-02: a file left from an
        // earlier run of the script.
        (_, InsightsSide::OtherRun { export, settings }) => {
            let stamp =
                |value: &Option<i64>| value.map_or_else(|| "none".to_string(), |v| v.to_string());
            not_compared(&format!(
                "the export is from another run (exportedAtSeconds {}; settings.json {})",
                stamp(export),
                stamp(settings)
            ))
        }
        (AppSide::Rejected(app_message), InsightsSide::Rejected(insights_message)) => {
            QueryComparison {
                app_message: Some(app_message.clone()),
                insights_message: Some(insights_message.clone()),
                ..QueryComparison::new(Verdict::BothRejected)
            }
        }
        (AppSide::Rejected(app_message), InsightsSide::Rows(rows)) => QueryComparison {
            app_message: Some(app_message.clone()),
            insights_count: Some(rows.len()),
            ..QueryComparison::new(Verdict::OnlyAppRejected)
        },
        (
            AppSide::Rows(app_rows) | AppSide::Aggregates(app_rows),
            InsightsSide::Rejected(insights_message),
        ) => QueryComparison {
            insights_message: Some(insights_message.clone()),
            app_count: Some(app_rows.rows.len()),
            ..QueryComparison::new(Verdict::OnlyInsightsRejected)
        },
        (AppSide::Rows(app_rows), InsightsSide::Rows(rows)) => {
            compare_rows(spec, app_rows, rows, Shape::Events)
        }
        (AppSide::Aggregates(app_rows), InsightsSide::Rows(rows)) => {
            compare_rows(spec, app_rows, rows, Shape::Aggregates)
        }
    }
}

/// The shape of the app's rows, for the step of the tie at the cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Events,
    Aggregates,
}

/// U4:BR2.2, BR2.3: how Logs Insights' cell `insights` and the app's `app`
/// compare under `tolerance`. The same text is exact; otherwise only two
/// finite numbers (decimal) or two whole numbers (approximate) can be within
/// the tolerance, and anything else is a difference.
pub fn compare_cell(insights: &str, app: &str, tolerance: Option<ToleranceKind>) -> CellDifference {
    if insights == app {
        return CellDifference::Exact;
    }
    match tolerance {
        Some(ToleranceKind::Decimal) => match (finite_number(insights), finite_number(app)) {
            (Some(left), Some(right)) if within_decimal(left, right) => {
                CellDifference::WithinDecimal
            }
            _ => CellDifference::Different,
        },
        Some(ToleranceKind::Approximate) => {
            match (insights.trim().parse::<i64>(), app.trim().parse::<i64>()) {
                (Ok(left), Ok(right)) if within_approximate(left, right) => {
                    CellDifference::WithinApproximate
                }
                _ => CellDifference::Different,
            }
        }
        None => CellDifference::Different,
    }
}

/// A text read as a finite number.
fn finite_number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

/// BR2.2: |a − b| ≤ 0.0001, or ≤ 1e-6 × max(|a|, |b|).
fn within_decimal(left: f64, right: f64) -> bool {
    let difference = (left - right).abs();
    difference <= DECIMAL_ABSOLUTE + READING_SLACK
        || difference <= DECIMAL_RELATIVE * left.abs().max(right.abs())
}

/// BR2.3: |a − b| ≤ 0.02 × max(|a|, |b|), in whole numbers.
fn within_approximate(left: i64, right: i64) -> bool {
    let (left, right) = (i128::from(left), i128::from(right));
    (left - right).abs() * APPROXIMATE_PARTS <= left.abs().max(right.abs())
}

fn not_compared(note: &str) -> QueryComparison {
    QueryComparison {
        note: Some(note.to_string()),
        ..QueryComparison::new(Verdict::NotCompared)
    }
}

/// Both sides have rows: steps 3 to 7 of the module.
fn compare_rows(
    spec: &CompareSpec,
    app: &AppRows,
    insights: &[Row],
    shape: Shape,
) -> QueryComparison {
    let counted = |verdict| QueryComparison {
        insights_count: Some(insights.len()),
        app_count: Some(app.rows.len()),
        ..QueryComparison::new(verdict)
    };
    let cut = app.cut || insights.len() >= spec.cap;
    if cut && (!spec.sorted || spec.sort_keys.is_empty()) {
        let reason = if spec.sorted {
            "a sort field is not among the columns, so the rows kept at the cut cannot be compared"
                .to_string()
        } else {
            format!(
                "a result reached the row cap {} without sort; which rows are kept depends on \
                 each side's order (BR2.5)",
                spec.cap
            )
        };
        return QueryComparison {
            note: Some(reason),
            ..counted(Verdict::Skipped)
        };
    }
    // A side without rows has no columns; the rows tell the difference.
    let both_have_rows = !insights.is_empty() && !app.rows.is_empty();
    if let Some(columns) = column_difference(&app.columns, insights).filter(|_| both_have_rows) {
        return QueryComparison {
            columns: Some(columns),
            ..counted(Verdict::Different)
        };
    }
    let tie = if spec.sorted && app.cut {
        match shape {
            Shape::Events => tie_at_cut(spec, app),
            // BR2.7 (4): the console's next row is not in its export.
            Shape::Aggregates => app.rows.last(),
        }
    } else {
        None
    };
    let keep = |row: &&Row| tie.is_none_or(|tied| !same_sort_values(spec, row, tied));
    let insights_kept: Vec<&Row> = insights.iter().filter(keep).collect();
    let app_kept: Vec<&Row> = app.rows.iter().filter(keep).collect();
    let tied_at_cut = tie.map(|_| {
        (
            insights.len() - insights_kept.len(),
            app.rows.len() - app_kept.len(),
        )
    });
    let matched = match_rows(&spec.tolerances, &insights_kept, &app_kept);
    let mut result = QueryComparison {
        tied_at_cut,
        within_decimal: matched.within_decimal,
        within_approximate: matched.within_approximate,
        within_examples: matched.within_examples,
        ..counted(Verdict::Same)
    };
    if !matched.only_insights.is_empty() || !matched.only_app.is_empty() {
        result.verdict = Verdict::Different;
        (result.edge_missing_in_app, result.missing_in_app) =
            split_edges(matched.only_insights, &spec.period_edges);
        (result.edge_missing_in_insights, result.missing_in_insights) =
            split_edges(matched.only_app, &spec.period_edges);
        return result;
    }
    if insights.len() != app.rows.len() {
        // Only possible when the tied rows were left out: their counts.
        result.verdict = Verdict::Different;
        return result;
    }
    result.first_order_mismatch = match spec.mode {
        CompareMode::Order if spec.sorted && !spec.sort_keys.is_empty() => {
            first_order_mismatch(spec, &insights_kept, &app_kept)
        }
        CompareMode::KeyOrder if !spec.key_columns.is_empty() => {
            first_key_order_mismatch(&spec.key_columns, &insights_kept, &app_kept)
        }
        _ => None,
    };
    if result.first_order_mismatch.is_some() {
        result.verdict = Verdict::Different;
    }
    result
}

/// What pairing the rows of both sides left (BR2.6): the rows of each side
/// the other has no match for, every one in its side's order, and the cells
/// within a tolerance.
#[derive(Debug, Default)]
struct RowMatch {
    only_insights: Vec<Row>,
    only_app: Vec<Row>,
    within_decimal: usize,
    within_approximate: usize,
    within_examples: Vec<WithinCell>,
}

impl RowMatch {
    fn note_within(&mut self, cell: WithinCell) {
        match cell.kind {
            CellDifference::WithinDecimal => self.within_decimal += 1,
            CellDifference::WithinApproximate => self.within_approximate += 1,
            CellDifference::Exact | CellDifference::Different => return,
        }
        if self.within_examples.len() < LISTED_ROWS {
            self.within_examples.push(cell);
        }
    }
}

/// BR2.1, BR2.6: without a tolerance the rows are multisets; with one, the
/// rows are paired by their columns without a tolerance, rows sharing that
/// key in the ascending order of their tolerant values, and a pair whose
/// tolerant cells differ beyond the tolerance is a row on each side only.
fn match_rows(tolerances: &[ColumnTolerance], insights: &[&Row], app: &[&Row]) -> RowMatch {
    if tolerances.is_empty() {
        return RowMatch {
            only_insights: only_in(insights, app),
            only_app: only_in(app, insights),
            ..RowMatch::default()
        };
    }
    let insights_groups = group_by_key(tolerances, insights);
    let mut app_groups = group_by_key(tolerances, app);
    let mut matched = RowMatch::default();
    for (key, mut insights_rows) in insights_groups {
        let position = app_groups.iter().position(|(other, _)| *other == key);
        let mut app_rows = position.map_or_else(Vec::new, |at| app_groups.remove(at).1);
        insights_rows.sort_by(|left, right| compare_tolerant(tolerances, left, right));
        app_rows.sort_by(|left, right| compare_tolerant(tolerances, left, right));
        let pairs = insights_rows.len().min(app_rows.len());
        for (insights_row, app_row) in insights_rows.iter().zip(&app_rows) {
            pair_rows(tolerances, insights_row, app_row, &mut matched);
        }
        matched
            .only_insights
            .extend(insights_rows[pairs..].iter().map(|row| (*row).clone()));
        matched
            .only_app
            .extend(app_rows[pairs..].iter().map(|row| (*row).clone()));
    }
    for (_, rows) in app_groups {
        matched.only_app.extend(rows.into_iter().cloned());
    }
    matched
}

/// The rows of `rows` grouped by their columns without a tolerance, the
/// groups in the order their first row comes.
fn group_by_key<'a>(tolerances: &[ColumnTolerance], rows: &[&'a Row]) -> Vec<(Row, Vec<&'a Row>)> {
    let mut groups: Vec<(Row, Vec<&'a Row>)> = Vec::new();
    for row in rows {
        let key: Row = row
            .iter()
            .filter(|(name, _)| tolerance_of(tolerances, name).is_none())
            .cloned()
            .collect();
        match groups.iter_mut().find(|(other, _)| *other == key) {
            Some((_, members)) => members.push(row),
            None => groups.push((key, vec![row])),
        }
    }
    groups
}

/// The value of `column` in `row`.
fn cell<'a>(row: &'a Row, column: &str) -> Option<&'a str> {
    row.iter()
        .find(|(name, _)| name == column)
        .map(|(_, value)| value.as_str())
}

/// The ascending order of two rows by their tolerant values, in the order of
/// the tolerances: numbers as numbers, other texts as texts, no value last.
fn compare_tolerant(tolerances: &[ColumnTolerance], left: &Row, right: &Row) -> Ordering {
    tolerances
        .iter()
        .map(|tolerance| {
            match (
                cell(left, &tolerance.column),
                cell(right, &tolerance.column),
            ) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => match (finite_number(left), finite_number(right)) {
                    (Some(a), Some(b)) => a.total_cmp(&b),
                    _ => left.cmp(right),
                },
            }
        })
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// BR2.6: compares the tolerant cells of one pair; a cell beyond its
/// tolerance (or a value on one side only) makes the pair a row on each side
/// only, otherwise the cells within a tolerance are noted.
fn pair_rows(tolerances: &[ColumnTolerance], insights: &Row, app: &Row, matched: &mut RowMatch) {
    let mut within = Vec::new();
    for tolerance in tolerances {
        let compared = match (
            cell(insights, &tolerance.column),
            cell(app, &tolerance.column),
        ) {
            (None, None) => continue,
            (Some(left), Some(right)) => compare_cell(left, right, Some(tolerance.kind)),
            _ => CellDifference::Different,
        };
        if compared == CellDifference::Different {
            matched.only_insights.push(insights.clone());
            matched.only_app.push(app.clone());
            return;
        }
        within.push(WithinCell {
            column: tolerance.column.clone(),
            insights: cell(insights, &tolerance.column)
                .unwrap_or_default()
                .to_string(),
            app: cell(app, &tolerance.column).unwrap_or_default().to_string(),
            kind: compared,
        });
    }
    for cell in within {
        matched.note_within(cell);
    }
}

/// BR2.7 (5): `rows` split into those of the first or the last bin of the
/// period and the others, the first [`LISTED_ROWS`] of each.
fn split_edges(rows: Vec<Row>, edges: &[(String, String)]) -> (Vec<Row>, Vec<Row>) {
    let (edge, other): (Vec<Row>, Vec<Row>) = rows.into_iter().partition(|row| {
        edges
            .iter()
            .any(|(column, value)| cell(row, column) == Some(value.as_str()))
    });
    (
        edge.into_iter().take(LISTED_ROWS).collect(),
        other.into_iter().take(LISTED_ROWS).collect(),
    )
}

/// BR2.1: the column names of each side, `@ptr` aside (the rows hold none),
/// when they differ. Logs Insights' columns are the fields of its rows.
fn column_difference(app_columns: &[String], insights: &[Row]) -> Option<ColumnDifference> {
    let app: BTreeSet<&str> = app_columns.iter().map(String::as_str).collect();
    let insights: BTreeSet<&str> = insights
        .iter()
        .flat_map(|row| row.iter().map(|(name, _)| name.as_str()))
        .collect();
    if app == insights {
        return None;
    }
    Some(ColumnDifference {
        only_insights: insights
            .difference(&app)
            .map(|name| (*name).to_string())
            .collect(),
        only_app: app
            .difference(&insights)
            .map(|name| (*name).to_string())
            .collect(),
    })
}

/// The values of the sort fields of `row`, `None` for a field without a
/// value.
fn sort_key<'a>(row: &'a Row, keys: &[String]) -> Vec<Option<&'a str>> {
    keys.iter()
        .map(|key| {
            row.iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.as_str())
        })
        .collect()
}

/// R-01: the last row kept, when it and the first row cut share their sort
/// value.
fn tie_at_cut<'a>(spec: &CompareSpec, app: &'a AppRows) -> Option<&'a Row> {
    let last = app.rows.last()?;
    let next = app.next_row.as_ref()?;
    same_sort_values(spec, last, next).then_some(last)
}

/// Whether two rows have the same sort values; decimal values within the
/// tolerance are the same rank (U4:BR2.2).
fn same_sort_values(spec: &CompareSpec, left: &Row, right: &Row) -> bool {
    spec.sort_keys.iter().all(|key| {
        let decimal = tolerance_of(&spec.tolerances, key).filter(|k| *k == ToleranceKind::Decimal);
        match (cell(left, key), cell(right, key)) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                compare_cell(left, right, decimal) != CellDifference::Different
            }
            _ => false,
        }
    })
}

/// BR2.1: the rows of `side` left when every row of `other` takes one equal
/// row away, counting repeated rows, in `side`'s order.
fn only_in(side: &[&Row], other: &[&Row]) -> Vec<Row> {
    let mut available: HashMap<&Row, usize> = HashMap::new();
    for row in other {
        *available.entry(*row).or_default() += 1;
    }
    let mut only = Vec::new();
    for row in side {
        match available.get_mut(*row) {
            Some(count) if *count > 0 => *count -= 1,
            _ => only.push(*row),
        }
    }
    only.into_iter().cloned().collect()
}

/// BR2.3: the first row (from 1) whose sort values differ; rows with the
/// same sort values may come in any order (FR3.4).
fn first_order_mismatch(spec: &CompareSpec, insights: &[&Row], app: &[&Row]) -> Option<usize> {
    insights
        .iter()
        .zip(app)
        .position(|(left, right)| !same_sort_values(spec, left, right))
        .map(|index| index + 1)
}

/// U4:BR2.8: the first row (from 1) whose key values differ, both sides in
/// the order they gave their rows.
fn first_key_order_mismatch(keys: &[String], insights: &[&Row], app: &[&Row]) -> Option<usize> {
    insights
        .iter()
        .zip(app)
        .position(|(left, right)| sort_key(left, keys) != sort_key(right, keys))
        .map(|index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(list: &[(&str, &str)]) -> Row {
        let mut row: Row = list
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        row.sort();
        row
    }

    fn event(name: &str) -> Row {
        row(&[("eventName", name)])
    }

    fn set_spec() -> CompareSpec {
        CompareSpec {
            mode: CompareMode::Set,
            sorted: false,
            sort_keys: Vec::new(),
            cap: 10_000,
            tolerances: Vec::new(),
            key_columns: Vec::new(),
            period_edges: Vec::new(),
        }
    }

    fn sorted_spec(key: &str, cap: usize) -> CompareSpec {
        CompareSpec {
            mode: CompareMode::Order,
            sorted: true,
            sort_keys: vec![key.to_string()],
            cap,
            ..set_spec()
        }
    }

    fn tolerant(list: &[(&str, ToleranceKind)]) -> Vec<ColumnTolerance> {
        list.iter()
            .map(|(column, kind)| ColumnTolerance {
                column: (*column).to_string(),
                kind: *kind,
            })
            .collect()
    }

    fn aggregates(columns: &[&str], rows: Vec<Row>) -> AppSide {
        AppSide::Aggregates(AppRows {
            columns: columns.iter().map(|name| (*name).to_string()).collect(),
            rows,
            cut: false,
            next_row: None,
        })
    }

    fn aggregates_cut(columns: &[&str], rows: Vec<Row>) -> AppSide {
        AppSide::Aggregates(AppRows {
            columns: columns.iter().map(|name| (*name).to_string()).collect(),
            rows,
            cut: true,
            next_row: None,
        })
    }

    fn app(columns: &[&str], rows: Vec<Row>) -> AppSide {
        AppSide::Rows(AppRows {
            columns: columns.iter().map(|name| (*name).to_string()).collect(),
            rows,
            cut: false,
            next_row: None,
        })
    }

    fn app_cut(columns: &[&str], rows: Vec<Row>, next_row: Row) -> AppSide {
        AppSide::Rows(AppRows {
            columns: columns.iter().map(|name| (*name).to_string()).collect(),
            rows,
            cut: true,
            next_row: Some(next_row),
        })
    }

    #[test]
    fn the_same_rows_in_another_order_are_the_same_collection() {
        let result = compare_query(
            &set_spec(),
            &app(&["eventName"], vec![event("A"), event("B"), event("A")]),
            &InsightsSide::Rows(vec![event("A"), event("A"), event("B")]),
        );
        assert_eq!(result.verdict, Verdict::Same);
        assert_eq!(result.insights_count, Some(3));
        assert_eq!(result.app_count, Some(3));
    }

    #[test]
    fn rows_on_one_side_only_are_a_difference_listing_the_first_five() {
        let insights: Vec<Row> = (0..8).map(|n| event(&format!("I{n}"))).collect();
        let result = compare_query(
            &set_spec(),
            &app(&["eventName"], vec![event("A"), event("A")]),
            &InsightsSide::Rows([vec![event("A")], insights].concat()),
        );
        assert_eq!(result.verdict, Verdict::Different);
        assert_eq!(result.missing_in_app.len(), LISTED_ROWS);
        assert_eq!(result.missing_in_app[0], event("I0"));
        assert_eq!(result.missing_in_insights, vec![event("A")], "a multiset");
        assert_eq!(
            (result.insights_count, result.app_count),
            (Some(9), Some(2))
        );
    }

    #[test]
    fn different_column_names_are_a_difference_of_columns() {
        let result = compare_query(
            &set_spec(),
            &app(
                &["eventName", "readOnly"],
                vec![row(&[("eventName", "A"), ("readOnly", "true")])],
            ),
            &InsightsSide::Rows(vec![row(&[("eventName", "A"), ("awsRegion", "us-east-1")])]),
        );
        assert_eq!(result.verdict, Verdict::Different);
        assert_eq!(
            result.columns,
            Some(ColumnDifference {
                only_insights: vec!["awsRegion".to_string()],
                only_app: vec!["readOnly".to_string()],
            })
        );
    }

    #[test]
    fn a_different_order_of_sort_values_names_the_first_row_that_differs() {
        let rows = |names: &[&str]| -> Vec<Row> { names.iter().map(|name| event(name)).collect() };
        let result = compare_query(
            &sorted_spec("eventName", 200),
            &app(&["eventName"], rows(&["A", "B", "C", "D"])),
            &InsightsSide::Rows(rows(&["A", "B", "D", "C"])),
        );
        assert_eq!(result.verdict, Verdict::Different);
        assert_eq!(result.first_order_mismatch, Some(3), "rows count from 1");
        assert!(result.missing_in_app.is_empty() && result.missing_in_insights.is_empty());
        let as_set = compare_query(
            &CompareSpec {
                mode: CompareMode::Set,
                ..sorted_spec("eventName", 200)
            },
            &app(&["eventName"], rows(&["A", "B", "C", "D"])),
            &InsightsSide::Rows(rows(&["A", "B", "D", "C"])),
        );
        assert_eq!(
            as_set.verdict,
            Verdict::Same,
            "set queries ignore the order"
        );
    }

    #[test]
    fn rows_with_the_same_sort_value_may_come_in_any_order() {
        let sorted = |name: &str, time: &str| row(&[("eventName", name), ("@timestamp", time)]);
        let result = compare_query(
            &sorted_spec("eventName", 200),
            &app(
                &["@timestamp", "eventName"],
                vec![sorted("A", "1"), sorted("A", "2"), sorted("B", "3")],
            ),
            &InsightsSide::Rows(vec![sorted("A", "2"), sorted("A", "1"), sorted("B", "3")]),
        );
        assert_eq!(result.verdict, Verdict::Same);
        assert_eq!(result.first_order_mismatch, None);
    }

    #[test]
    fn a_tie_at_the_cut_leaves_out_the_tied_rows_and_compares_the_counts() {
        let sorted = |name: &str, time: &str| row(&[("eventName", name), ("@timestamp", time)]);
        // limit 3: "B" is the last row kept and also the first row cut, so
        // either side may keep any of the "B" rows (R-01).
        let spec = sorted_spec("eventName", 3);
        let app_side = app_cut(
            &["@timestamp", "eventName"],
            vec![sorted("A", "1"), sorted("B", "2"), sorted("B", "3")],
            sorted("B", "4"),
        );
        let same = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![sorted("A", "1"), sorted("B", "4"), sorted("B", "2")]),
        );
        assert_eq!(same.verdict, Verdict::Same);
        assert_eq!(same.tied_at_cut, Some((2, 2)));
        let other_first = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![sorted("A", "9"), sorted("B", "4"), sorted("B", "2")]),
        );
        assert_eq!(
            other_first.verdict,
            Verdict::Different,
            "rows before the tie still count"
        );
        let no_tie = compare_query(
            &spec,
            &app_cut(
                &["@timestamp", "eventName"],
                vec![sorted("A", "1"), sorted("B", "2"), sorted("B", "3")],
                sorted("C", "4"),
            ),
            &InsightsSide::Rows(vec![sorted("A", "1"), sorted("B", "4"), sorted("B", "2")]),
        );
        assert_eq!(
            no_tie.verdict,
            Verdict::Different,
            "without a tie every row counts"
        );
        assert_eq!(no_tie.tied_at_cut, None);
    }

    #[test]
    fn rows_without_columns_on_both_sides_are_the_same() {
        // R-06: a Logs Insights row with only @ptr and an app row whose
        // chosen columns all lack a value are both "a row without columns".
        let result = compare_query(
            &set_spec(),
            &app(
                &["errorCode"],
                vec![Row::new(), row(&[("errorCode", "AccessDenied")])],
            ),
            &InsightsSide::Rows(vec![row(&[("errorCode", "AccessDenied")]), Row::new()]),
        );
        assert_eq!(result.verdict, Verdict::Same);
        let none = compare_query(
            &set_spec(),
            &app(&[], vec![Row::new(), Row::new()]),
            &InsightsSide::Rows(vec![Row::new(), Row::new()]),
        );
        assert_eq!(none.verdict, Verdict::Same);
    }

    #[test]
    fn both_refusing_is_no_difference_and_one_side_refusing_is() {
        // What the app gives for `filter eventName like eventSource`, which
        // the core still refuses (loop-back 2; p05 is read now).
        let refused_app =
            AppSide::Rejected("Unsupported query.unsupported.fieldComparison (eventSource)".into());
        let refused_insights = InsightsSide::Rejected("MalformedQueryException".into());
        let both = compare_query(&set_spec(), &refused_app, &refused_insights);
        assert_eq!(both.verdict, Verdict::BothRejected);
        let only_app = compare_query(
            &set_spec(),
            &refused_app,
            &InsightsSide::Rows(vec![event("A")]),
        );
        assert_eq!(only_app.verdict, Verdict::OnlyAppRejected);
        assert_eq!(
            only_app.app_message.as_deref(),
            Some("Unsupported query.unsupported.fieldComparison (eventSource)")
        );
        assert_eq!(only_app.insights_count, Some(1));
        let only_insights =
            compare_query(&set_spec(), &app(&["eventName"], vec![]), &refused_insights);
        assert_eq!(only_insights.verdict, Verdict::OnlyInsightsRejected);
        assert_eq!(
            only_insights.insights_message.as_deref(),
            Some("MalformedQueryException")
        );
    }

    // 261025-insights-stats U4: aggregate rows (BR2.1-BR2.4, BR2.6-BR2.8).

    /// BR2.2: within 0.0001, or within 1e-6 of the larger value, is the
    /// same; just outside is not.
    #[test]
    fn decimal_cells_within_the_absolute_or_the_relative_error_are_the_same() {
        let decimal = Some(ToleranceKind::Decimal);
        assert_eq!(
            compare_cell("1.2346", "1.2346", decimal),
            CellDifference::Exact
        );
        assert_eq!(
            compare_cell("1", "1.0001", decimal),
            CellDifference::WithinDecimal
        );
        assert_eq!(
            compare_cell("0.5", "0.5001", decimal),
            CellDifference::WithinDecimal
        );
        assert_eq!(
            compare_cell("1", "1.00011", decimal),
            CellDifference::Different
        );
        assert_eq!(
            compare_cell("1000000", "1000000.9", decimal),
            CellDifference::WithinDecimal,
            "1e-6 of 1,000,000.9 is above 0.9"
        );
        assert_eq!(
            compare_cell("1000000", "1000001.5", decimal),
            CellDifference::Different
        );
        assert_eq!(
            compare_cell("-2.5", "-2.50005", decimal),
            CellDifference::WithinDecimal
        );
    }

    #[test]
    fn a_cell_that_is_not_a_number_is_compared_as_text() {
        let decimal = Some(ToleranceKind::Decimal);
        assert_eq!(
            compare_cell("Decrypt", "Decrypt", decimal),
            CellDifference::Exact
        );
        assert_eq!(
            compare_cell("Decrypt", "Encrypt", decimal),
            CellDifference::Different
        );
        assert_eq!(compare_cell("1", "x", decimal), CellDifference::Different);
        assert_eq!(compare_cell("NaN", "NaN", decimal), CellDifference::Exact);
        assert_eq!(
            compare_cell("inf", "1e400", decimal),
            CellDifference::Different
        );
        // Without a tolerance only the same text is the same.
        assert_eq!(compare_cell("1.0", "1", None), CellDifference::Different);
        assert_eq!(compare_cell("1", "1", None), CellDifference::Exact);
    }

    /// BR2.3: whole numbers within 2% of the larger are the same.
    #[test]
    fn approximate_cells_within_two_percent_are_the_same() {
        let approximate = Some(ToleranceKind::Approximate);
        assert_eq!(
            compare_cell("100", "102", approximate),
            CellDifference::WithinApproximate
        );
        assert_eq!(
            compare_cell("102", "100", approximate),
            CellDifference::WithinApproximate
        );
        assert_eq!(
            compare_cell("100", "103", approximate),
            CellDifference::Different
        );
        assert_eq!(compare_cell("0", "0", approximate), CellDifference::Exact);
        assert_eq!(
            compare_cell("0", "1", approximate),
            CellDifference::Different
        );
        assert_eq!(
            compare_cell("100", "100.5", approximate),
            CellDifference::Different,
            "not whole numbers: compared as text"
        );
    }

    /// BR2.6: without a tolerant column the rows are multisets, as for
    /// events.
    #[test]
    fn aggregate_rows_without_a_tolerance_are_compared_as_multisets() {
        let count = |source: &str, n: &str| row(&[("eventSource", source), ("count(*)", n)]);
        let app_side = aggregates(
            &["eventSource", "count(*)"],
            vec![count("s3", "2"), count("kms", "1")],
        );
        let same = compare_query(
            &set_spec(),
            &app_side,
            &InsightsSide::Rows(vec![count("kms", "1"), count("s3", "2")]),
        );
        assert_eq!(same.verdict, Verdict::Same);
        assert_eq!((same.within_decimal, same.within_approximate), (0, 0));
        let different = compare_query(
            &set_spec(),
            &app_side,
            &InsightsSide::Rows(vec![count("kms", "1"), count("s3", "3")]),
        );
        assert_eq!(different.verdict, Verdict::Different);
        assert_eq!(different.missing_in_app, vec![count("s3", "3")]);
        assert_eq!(different.missing_in_insights, vec![count("s3", "2")]);
    }

    /// BR2.6: the rows are paired by their columns without a tolerance; the
    /// tolerant cells of a pair are compared with BR2.2 / BR2.3.
    #[test]
    fn rows_are_paired_by_the_columns_without_a_tolerance() {
        let spec = CompareSpec {
            tolerances: tolerant(&[
                ("avg(latency)", ToleranceKind::Decimal),
                ("count_distinct(eventName)", ToleranceKind::Approximate),
            ]),
            ..set_spec()
        };
        let stat = |status: &str, avg: &str, distinct: &str| {
            row(&[
                ("status", status),
                ("avg(latency)", avg),
                ("count_distinct(eventName)", distinct),
            ])
        };
        let columns = ["status", "avg(latency)", "count_distinct(eventName)"];
        let app_side = aggregates(
            &columns,
            vec![stat("200", "12.3456", "100"), stat("500", "3", "7")],
        );
        let within = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![stat("500", "3", "7"), stat("200", "12.34565", "102")]),
        );
        assert_eq!(within.verdict, Verdict::Same, "{within:?}");
        assert_eq!((within.within_decimal, within.within_approximate), (1, 1));
        assert_eq!(
            within.within_examples[0],
            WithinCell {
                column: "avg(latency)".to_string(),
                insights: "12.34565".to_string(),
                app: "12.3456".to_string(),
                kind: CellDifference::WithinDecimal,
            }
        );
        let beyond = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![stat("500", "3", "7"), stat("200", "12.5", "100")]),
        );
        assert_eq!(beyond.verdict, Verdict::Different);
        assert_eq!(beyond.missing_in_app, vec![stat("200", "12.5", "100")]);
        assert_eq!(
            beyond.missing_in_insights,
            vec![stat("200", "12.3456", "100")]
        );
    }

    /// BR2.6: rows sharing their key (a `stats` without `by` is one row with
    /// no key) are paired in the ascending order of their tolerant values;
    /// a key on one side only is a difference.
    #[test]
    fn rows_sharing_a_key_are_paired_in_order_and_a_key_on_one_side_is_a_difference() {
        let spec = CompareSpec {
            tolerances: tolerant(&[("avg(x)", ToleranceKind::Decimal)]),
            ..set_spec()
        };
        let avg = |value: &str| row(&[("avg(x)", value)]);
        let single = compare_query(
            &spec,
            &aggregates(&["avg(x)"], vec![avg("2.00001")]),
            &InsightsSide::Rows(vec![avg("2")]),
        );
        assert_eq!(single.verdict, Verdict::Same);
        assert_eq!(single.within_decimal, 1);
        let shared = compare_query(
            &spec,
            &aggregates(&["avg(x)"], vec![avg("5.00001"), avg("1")]),
            &InsightsSide::Rows(vec![avg("1.00005"), avg("5")]),
        );
        assert_eq!(shared.verdict, Verdict::Same, "{shared:?}");
        assert_eq!(shared.within_decimal, 2);
        let keyed = |key: &str, value: &str| row(&[("k", key), ("avg(x)", value)]);
        let one_side = compare_query(
            &spec,
            &aggregates(&["k", "avg(x)"], vec![keyed("a", "1")]),
            &InsightsSide::Rows(vec![keyed("a", "1"), keyed("b", "1")]),
        );
        assert_eq!(one_side.verdict, Verdict::Different);
        assert_eq!(one_side.missing_in_app, vec![keyed("b", "1")]);
        assert!(one_side.missing_in_insights.is_empty());
    }

    /// BR2.2 (FR review R-05): in the order step decimal sort values within
    /// the tolerance are the same rank.
    #[test]
    fn decimal_sort_values_within_the_tolerance_are_the_same_rank() {
        let spec = CompareSpec {
            tolerances: tolerant(&[("s", ToleranceKind::Decimal)]),
            ..sorted_spec("s", 10_000)
        };
        let sum = |status: &str, value: &str| row(&[("status", status), ("s", value)]);
        let result = compare_query(
            &spec,
            &aggregates(
                &["status", "s"],
                vec![sum("200", "9.00004"), sum("500", "1")],
            ),
            &InsightsSide::Rows(vec![sum("200", "9"), sum("500", "1")]),
        );
        assert_eq!(result.verdict, Verdict::Same, "{result:?}");
        assert_eq!(result.first_order_mismatch, None);
        let swapped = compare_query(
            &spec,
            &aggregates(&["status", "s"], vec![sum("500", "1"), sum("200", "9")]),
            &InsightsSide::Rows(vec![sum("200", "9"), sum("500", "1")]),
        );
        assert_eq!(swapped.first_order_mismatch, Some(1));
    }

    /// BR2.8: keyorder compares the collection, then the order of the key
    /// values, naming the first row out of order.
    #[test]
    fn keyorder_compares_the_order_of_the_key_values_after_the_collection() {
        let spec = CompareSpec {
            mode: CompareMode::KeyOrder,
            key_columns: vec!["eventSource".to_string()],
            ..set_spec()
        };
        let count = |source: &str, n: &str| row(&[("eventSource", source), ("count(*)", n)]);
        let insights =
            InsightsSide::Rows(vec![count("kms", "1"), count("s3", "2"), count("sts", "4")]);
        let ascending = compare_query(
            &spec,
            &aggregates(
                &["eventSource", "count(*)"],
                vec![count("kms", "1"), count("s3", "2"), count("sts", "4")],
            ),
            &insights,
        );
        assert_eq!(ascending.verdict, Verdict::Same);
        let other_order = compare_query(
            &spec,
            &aggregates(
                &["eventSource", "count(*)"],
                vec![count("kms", "1"), count("sts", "4"), count("s3", "2")],
            ),
            &insights,
        );
        assert_eq!(other_order.verdict, Verdict::Different);
        assert_eq!(other_order.first_order_mismatch, Some(2));
        assert!(other_order.missing_in_app.is_empty());
        let as_set = compare_query(
            &CompareSpec {
                mode: CompareMode::Set,
                ..spec
            },
            &aggregates(
                &["eventSource", "count(*)"],
                vec![count("kms", "1"), count("sts", "4"), count("s3", "2")],
            ),
            &insights,
        );
        assert_eq!(as_set.verdict, Verdict::Same);
    }

    /// BR2.4: an aggregates result without `sort` cut by its `limit` or at
    /// 10,000 is not compared.
    #[test]
    fn an_unsorted_aggregates_result_cut_at_its_cap_is_skipped() {
        let count = |source: &str| row(&[("eventSource", source), ("count(*)", "1")]);
        let rows = vec![count("kms"), count("s3")];
        let cut = compare_query(
            &CompareSpec {
                cap: 2,
                ..set_spec()
            },
            &aggregates_cut(&["eventSource", "count(*)"], rows.clone()),
            &InsightsSide::Rows(rows.clone()),
        );
        assert_eq!(cut.verdict, Verdict::Skipped);
        let not_cut = compare_query(
            &set_spec(),
            &aggregates(&["eventSource", "count(*)"], rows.clone()),
            &InsightsSide::Rows(rows),
        );
        assert_eq!(not_cut.verdict, Verdict::Same);
    }

    /// BR2.4, BR2.7 (4): a sorted aggregates result that was cut leaves out,
    /// on both sides, the rows with the last shown row's sort value (decimal
    /// values within the tolerance included) and compares their counts.
    #[test]
    fn a_cut_sorted_aggregates_result_leaves_out_the_rows_tied_with_the_last_row() {
        let spec = CompareSpec {
            tolerances: tolerant(&[("n", ToleranceKind::Decimal)]),
            ..sorted_spec("n", 2)
        };
        let count = |source: &str, n: &str| row(&[("eventSource", source), ("n", n)]);
        let app_side = aggregates_cut(
            &["eventSource", "n"],
            vec![count("s3", "9"), count("kms", "4")],
        );
        let same = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![count("s3", "9"), count("sts", "4.00001")]),
        );
        assert_eq!(same.verdict, Verdict::Same, "{same:?}");
        assert_eq!(same.tied_at_cut, Some((1, 1)));
        let fewer = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![count("s3", "9")]),
        );
        assert_eq!(
            fewer.verdict,
            Verdict::Different,
            "the counts of the tied rows"
        );
        let other_first = compare_query(
            &spec,
            &app_side,
            &InsightsSide::Rows(vec![count("s3", "8"), count("kms", "4")]),
        );
        assert_eq!(other_first.verdict, Verdict::Different);
    }

    /// BR2.7 (5): a difference in a row whose bin is the first or the last
    /// of the period is listed apart as a period edge, and is still a
    /// difference.
    #[test]
    fn a_difference_in_the_first_or_last_bin_is_listed_as_a_period_edge() {
        let spec = CompareSpec {
            period_edges: vec![
                ("bin(5m)".to_string(), "2024-01-02 00:00:00.000".to_string()),
                ("bin(5m)".to_string(), "2024-01-02 00:55:00.000".to_string()),
            ],
            ..set_spec()
        };
        let bin = |at: &str, n: &str| row(&[("bin(5m)", at), ("count(*)", n)]);
        let result = compare_query(
            &spec,
            &aggregates(
                &["bin(5m)", "count(*)"],
                vec![
                    bin("2024-01-02 00:00:00.000", "3"),
                    bin("2024-01-02 00:05:00.000", "2"),
                    bin("2024-01-02 00:55:00.000", "1"),
                ],
            ),
            &InsightsSide::Rows(vec![
                bin("2024-01-02 00:00:00.000", "4"),
                bin("2024-01-02 00:05:00.000", "2"),
                bin("2024-01-02 00:55:00.000", "1"),
            ]),
        );
        assert_eq!(result.verdict, Verdict::Different);
        assert!(result.missing_in_app.is_empty() && result.missing_in_insights.is_empty());
        assert_eq!(
            result.edge_missing_in_app,
            vec![bin("2024-01-02 00:00:00.000", "4")]
        );
        assert_eq!(
            result.edge_missing_in_insights,
            vec![bin("2024-01-02 00:00:00.000", "3")]
        );
        let middle = compare_query(
            &spec,
            &aggregates(
                &["bin(5m)", "count(*)"],
                vec![bin("2024-01-02 00:05:00.000", "2")],
            ),
            &InsightsSide::Rows(vec![bin("2024-01-02 00:05:00.000", "5")]),
        );
        assert_eq!(middle.missing_in_app.len(), 1, "not an edge");
        assert!(middle.edge_missing_in_app.is_empty());
    }

    /// The refusal steps are the same for an aggregates result.
    #[test]
    fn an_aggregates_result_and_a_refusal_are_one_sided_refusals() {
        let app_side = aggregates(&["count(*)"], vec![row(&[("count(*)", "3")])]);
        let refused = InsightsSide::Rejected("MalformedQueryException".into());
        let only_insights = compare_query(&set_spec(), &app_side, &refused);
        assert_eq!(only_insights.verdict, Verdict::OnlyInsightsRejected);
        assert_eq!(only_insights.app_count, Some(1));
        let missing = compare_query(&set_spec(), &app_side, &InsightsSide::Missing);
        assert_eq!(missing.verdict, Verdict::NotCompared);
    }

    #[test]
    fn an_unsorted_result_cut_at_the_cap_is_skipped() {
        let full: Vec<Row> = (0..3).map(|n| event(&format!("E{n}"))).collect();
        let at_cap = compare_query(
            &CompareSpec {
                cap: 3,
                ..set_spec()
            },
            &app(&["eventName"], full.clone()),
            &InsightsSide::Rows(full.clone()),
        );
        assert_eq!(at_cap.verdict, Verdict::Skipped);
        assert!(
            at_cap.note.as_deref().unwrap_or("").contains('3'),
            "{:?}",
            at_cap.note
        );
        let app_cut_only = compare_query(
            &CompareSpec {
                cap: 3,
                ..set_spec()
            },
            &app_cut(&["eventName"], full.clone(), event("E9")),
            &InsightsSide::Rows(full[..2].to_vec()),
        );
        assert_eq!(app_cut_only.verdict, Verdict::Skipped);
        let unseen_sort = compare_query(
            &CompareSpec {
                sort_keys: Vec::new(),
                ..sorted_spec("eventTime", 3)
            },
            &app_cut(&["eventName"], full.clone(), event("E9")),
            &InsightsSide::Rows(full),
        );
        assert_eq!(
            unseen_sort.verdict,
            Verdict::Skipped,
            "a cut by a sort field that is not a column cannot be compared"
        );
    }

    #[test]
    fn a_missing_or_unfinished_export_is_not_compared() {
        let app_side = app(&["eventName"], vec![event("A")]);
        let missing = compare_query(&set_spec(), &app_side, &InsightsSide::Missing);
        assert_eq!(missing.verdict, Verdict::NotCompared);
        let failed = compare_query(
            &set_spec(),
            &AppSide::Rejected("x".into()),
            &InsightsSide::NotRun("Timeout".into()),
        );
        assert_eq!(failed.verdict, Verdict::NotCompared);
        assert!(failed.note.as_deref().unwrap_or("").contains("Timeout"));
    }

    /// Code review R-02: an export of another run is not compared, and the
    /// note names both stamps.
    #[test]
    fn an_export_from_another_run_is_not_compared() {
        let app_side = app(&["eventName"], vec![event("A")]);
        let stale = compare_query(
            &set_spec(),
            &app_side,
            &InsightsSide::OtherRun {
                export: Some(100),
                settings: Some(200),
            },
        );
        assert_eq!(stale.verdict, Verdict::NotCompared);
        let note = stale.note.unwrap_or_default();
        assert!(
            note.contains("exportedAtSeconds 100") && note.contains("settings.json 200"),
            "{note}"
        );
        let unstamped = compare_query(
            &set_spec(),
            &app_side,
            &InsightsSide::OtherRun {
                export: None,
                settings: Some(200),
            },
        );
        assert_eq!(unstamped.verdict, Verdict::NotCompared);
        assert!(
            unstamped
                .note
                .as_deref()
                .unwrap_or("")
                .contains("exportedAtSeconds none"),
            "{unstamped:?}"
        );
    }
}
