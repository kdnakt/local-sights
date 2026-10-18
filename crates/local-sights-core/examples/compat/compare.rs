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

use std::collections::{BTreeSet, HashMap};

use crate::compat::export::InsightsSide;
use crate::compat::format::Row;
use crate::compat::queries::CompareMode;

/// Rows of each side listed when the collections differ (BR3.2).
pub const LISTED_ROWS: usize = 5;

/// What the app made of one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppSide {
    /// The app refused the query (a QueryProblem), with its description.
    Rejected(String),
    /// The app's result.
    Rows(AppRows),
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
    /// The row cap: the last `limit` from 1, or 10,000 (U2:BR5.1, BR5.2).
    pub cap: usize,
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
    /// The first row (from 1) whose sort values differ (BR2.3).
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
        (AppSide::Rows(app_rows), InsightsSide::Rejected(insights_message)) => QueryComparison {
            insights_message: Some(insights_message.clone()),
            app_count: Some(app_rows.rows.len()),
            ..QueryComparison::new(Verdict::OnlyInsightsRejected)
        },
        (AppSide::Rows(app_rows), InsightsSide::Rows(rows)) => compare_rows(spec, app_rows, rows),
    }
}

fn not_compared(note: &str) -> QueryComparison {
    QueryComparison {
        note: Some(note.to_string()),
        ..QueryComparison::new(Verdict::NotCompared)
    }
}

/// Both sides have rows: steps 3 to 7 of the module.
fn compare_rows(spec: &CompareSpec, app: &AppRows, insights: &[Row]) -> QueryComparison {
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
        tie_at_cut(&spec.sort_keys, app)
    } else {
        None
    };
    let keep = |row: &&Row| {
        tie.as_ref()
            .is_none_or(|value| !has_sort_value(row, &spec.sort_keys, value))
    };
    let insights_kept: Vec<&Row> = insights.iter().filter(keep).collect();
    let app_kept: Vec<&Row> = app.rows.iter().filter(keep).collect();
    let tied_at_cut = tie.as_ref().map(|_| {
        (
            insights.len() - insights_kept.len(),
            app.rows.len() - app_kept.len(),
        )
    });
    let (missing_in_app, missing_in_insights) = multiset_difference(&insights_kept, &app_kept);
    let mut result = QueryComparison {
        tied_at_cut,
        ..counted(Verdict::Same)
    };
    if !missing_in_app.is_empty() || !missing_in_insights.is_empty() {
        result.verdict = Verdict::Different;
        result.missing_in_app = missing_in_app;
        result.missing_in_insights = missing_in_insights;
        return result;
    }
    if insights.len() != app.rows.len() {
        // Only possible when the tied rows were left out: their counts.
        result.verdict = Verdict::Different;
        return result;
    }
    if spec.mode == CompareMode::Order && spec.sorted && !spec.sort_keys.is_empty() {
        result.first_order_mismatch =
            first_order_mismatch(&spec.sort_keys, &insights_kept, &app_kept);
        if result.first_order_mismatch.is_some() {
            result.verdict = Verdict::Different;
        }
    }
    result
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

/// R-01: the sort value shared by the last row kept and the first row cut,
/// when they share one.
fn tie_at_cut(keys: &[String], app: &AppRows) -> Option<Vec<Option<String>>> {
    let last = sort_key(app.rows.last()?, keys);
    let next = sort_key(app.next_row.as_ref()?, keys);
    (last == next).then(|| {
        last.into_iter()
            .map(|value| value.map(str::to_string))
            .collect()
    })
}

/// Whether `row`'s sort values are `value`.
fn has_sort_value(row: &Row, keys: &[String], value: &[Option<String>]) -> bool {
    let key = sort_key(row, keys);
    key.len() == value.len()
        && key
            .iter()
            .zip(value)
            .all(|(left, right)| *left == right.as_deref())
}

/// BR2.1: the rows of each side that the other does not have, counting
/// repeated rows, the first [`LISTED_ROWS`] of each in their side's order.
fn multiset_difference(insights: &[&Row], app: &[&Row]) -> (Vec<Row>, Vec<Row>) {
    (only_in(insights, app), only_in(app, insights))
}

/// The rows of `side` left when every row of `other` takes one equal row
/// away.
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
    only.into_iter().take(LISTED_ROWS).cloned().collect()
}

/// BR2.3: the first row (from 1) whose sort values differ; rows with the
/// same sort values may come in any order (FR3.4).
fn first_order_mismatch(keys: &[String], insights: &[&Row], app: &[&Row]) -> Option<usize> {
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
        }
    }

    fn sorted_spec(key: &str, cap: usize) -> CompareSpec {
        CompareSpec {
            mode: CompareMode::Order,
            sorted: true,
            sort_keys: vec![key.to_string()],
            cap,
        }
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
        let refused_app = AppSide::Rejected("Unsupported query.unsupported.fieldComparison".into());
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
            Some("Unsupported query.unsupported.fieldComparison")
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
