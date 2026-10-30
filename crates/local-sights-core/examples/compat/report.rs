//! The report and the exit code (261017-insights-query U4:BR2.6, BR3.2,
//! review R-02, R-03).
//!
//! One line a query: its id, verdict and both row counts, with marks for a
//! tie left out at the cut (R-01), the first row out of order (BR2.3) and
//! events in the two boundary seconds of the period (R-03); then, for a
//! difference, the column names on one side only and the first five rows
//! on one side only, each as its `@timestamp` and the other columns with the
//! first 40 characters of their values (entities.md RowDigest). The export
//! as a whole is never printed, and nothing here ever holds a credential
//! (BR3.2, NFR4). Last, the number of queries of each verdict.
//!
//! 261025-insights-stats U4: a query the same only within a column
//! tolerance says so with its cell count, "Same (within decimal tolerance:
//! n cells)" or "(within approximate tolerance: n cells)", and lists its
//! first five such cells (BR2.5); rows that differ in the first or the last
//! bin of the period are listed apart as period edges (BR2.7 (5)); after
//! the summary, each provisional rule of the query set is summed up as
//! matches, differs or notChecked by the outcome of its queries (BR1.4).
//! The exit code is unchanged: cells within a tolerance are no difference.

use crate::compat::compare::{CellDifference, QueryComparison, Verdict, WithinCell};
use crate::compat::format::Row;
use crate::compat::queries::ProvisionalRuleCheck;

/// The outcome of a provisional rule (entities.md
/// ProvisionalRuleCheck.outcome, BR1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisionalOutcome {
    /// Every query that checks it was compared, none with a difference.
    Matches,
    /// A query that checks it differs.
    Differs,
    /// No query checks it, or one was not compared.
    NotChecked,
}

impl ProvisionalOutcome {
    /// The name in the report and the record (BR3.1).
    pub fn as_str(self) -> &'static str {
        match self {
            ProvisionalOutcome::Matches => "matches",
            ProvisionalOutcome::Differs => "differs",
            ProvisionalOutcome::NotChecked => "notChecked",
        }
    }
}

/// Exit code: every compared query is Same or BothRejected (BR2.6).
pub const EXIT_COMPATIBLE: u8 = 0;
/// Exit code: some query differs (BR2.6).
pub const EXIT_DIFFERENT: u8 = 1;
/// Exit code: the comparison could not be made, or not for every query
/// (wrong conditions, a failed fetch, an export missing or not Complete, or
/// no query compared at all; BR2.6, R-02).
pub const EXIT_NOT_COMPARED: u8 = 2;

/// Characters of a value shown in a row digest (BR3.2).
pub const VALUE_CHARS: usize = 40;

/// Characters of a refusal message shown (BR2.4).
pub const MESSAGE_CHARS: usize = 120;

/// BR2.6, R-02: the exit code of the whole run.
pub fn exit_code(verdicts: &[Verdict]) -> u8 {
    let not_compared = verdicts.contains(&Verdict::NotCompared);
    let compared = verdicts.iter().any(|verdict| verdict.is_compared());
    if not_compared || !compared {
        return EXIT_NOT_COMPARED;
    }
    if verdicts.iter().any(|verdict| verdict.is_difference()) {
        return EXIT_DIFFERENT;
    }
    EXIT_COMPATIBLE
}

/// BR3.2: the report lines of one query; `boundary_events` is whether the
/// app's result has events in the first or the last second of the period
/// (R-03).
pub fn query_lines(id: &str, comparison: &QueryComparison, boundary_events: bool) -> Vec<String> {
    let count = |value: Option<usize>| value.map_or_else(|| "-".to_string(), |n| n.to_string());
    let mut first = format!(
        "{id}  {}{}  insights={}  app={}",
        comparison.verdict.as_str(),
        within_suffix(comparison),
        count(comparison.insights_count),
        count(comparison.app_count)
    );
    if let Some((insights, app)) = comparison.tied_at_cut {
        first.push_str(&format!("  tied-at-cut={insights}/{app}"));
    }
    if let Some(row) = comparison.first_order_mismatch {
        first.push_str(&format!("  first-out-of-order-row={row}"));
    }
    if boundary_events {
        first.push_str("  boundary-events");
    }
    if !comparison.edge_missing_in_app.is_empty() || !comparison.edge_missing_in_insights.is_empty()
    {
        first.push_str("  period-edge-differences");
    }
    let mut lines = vec![first];
    let mut detail = |label: &str, text: String| lines.push(format!("    {label}: {text}"));
    if let Some(note) = &comparison.note {
        detail("note", cut(note, MESSAGE_CHARS));
    }
    if let Some(columns) = &comparison.columns {
        if !columns.only_insights.is_empty() {
            detail(
                "columns only in Logs Insights",
                columns.only_insights.join(", "),
            );
        }
        if !columns.only_app.is_empty() {
            detail("columns only in app", columns.only_app.join(", "));
        }
    }
    for row in &comparison.missing_in_app {
        detail("only in Logs Insights", row_digest(row));
    }
    for row in &comparison.missing_in_insights {
        detail("only in app", row_digest(row));
    }
    for row in &comparison.edge_missing_in_app {
        detail("only in Logs Insights (period edge)", row_digest(row));
    }
    for row in &comparison.edge_missing_in_insights {
        detail("only in app (period edge)", row_digest(row));
    }
    for cell in &comparison.within_examples {
        detail(within_label(cell.kind), within_digest(cell));
    }
    if let Some(message) = &comparison.insights_message {
        detail("Logs Insights refused", cut(message, MESSAGE_CHARS));
    }
    if let Some(message) = &comparison.app_message {
        detail("app refused", cut(message, MESSAGE_CHARS));
    }
    lines
}

/// BR2.5: ` (within decimal tolerance: n cells; within approximate
/// tolerance: n cells)` with the kinds that have cells, or nothing.
fn within_suffix(comparison: &QueryComparison) -> String {
    let cells = |n: usize| {
        if n == 1 {
            "1 cell".to_string()
        } else {
            format!("{n} cells")
        }
    };
    let parts: Vec<String> = [
        (CellDifference::WithinDecimal, comparison.within_decimal),
        (
            CellDifference::WithinApproximate,
            comparison.within_approximate,
        ),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(kind, n)| format!("{}: {}", within_label(kind), cells(n)))
    .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

/// The name of a kind of cell within a tolerance.
fn within_label(kind: CellDifference) -> &'static str {
    match kind {
        CellDifference::WithinApproximate => "within approximate tolerance",
        _ => "within decimal tolerance",
    }
}

/// One cell within a tolerance: its column and both values, each cut to
/// [`VALUE_CHARS`] characters.
fn within_digest(cell: &WithinCell) -> String {
    format!(
        "{} insights={} app={}",
        one_line(&cell.column),
        cut(&cell.insights, VALUE_CHARS),
        cut(&cell.app, VALUE_CHARS)
    )
}

/// BR1.4: the outcome of `check` from the verdicts of the run, `(id,
/// verdict)`.
pub fn provisional_outcome(
    check: &ProvisionalRuleCheck,
    verdicts: &[(String, Verdict)],
) -> ProvisionalOutcome {
    let found: Vec<Option<Verdict>> = check
        .query_ids
        .iter()
        .map(|id| verdict_of(verdicts, id))
        .collect();
    if found
        .iter()
        .flatten()
        .any(|verdict| verdict.is_difference())
    {
        return ProvisionalOutcome::Differs;
    }
    let all_compared = !found.is_empty()
        && found
            .iter()
            .all(|verdict| verdict.is_some_and(Verdict::is_compared));
    if all_compared {
        ProvisionalOutcome::Matches
    } else {
        ProvisionalOutcome::NotChecked
    }
}

/// BR1.4: the summary of the provisional rules, one line a rule with the
/// verdict of each of its queries, then the count of each outcome; nothing
/// for a set without provisional notes.
pub fn provisional_lines(
    checks: &[ProvisionalRuleCheck],
    verdicts: &[(String, Verdict)],
) -> Vec<String> {
    if checks.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["provisional rules:".to_string()];
    let mut outcomes = Vec::new();
    for check in checks {
        let outcome = provisional_outcome(check, verdicts);
        outcomes.push(outcome);
        let queries = if check.query_ids.is_empty() {
            "(no query)".to_string()
        } else {
            check
                .query_ids
                .iter()
                .map(|id| {
                    let verdict = verdict_of(verdicts, id).map_or("-", Verdict::as_str);
                    format!("{id}={verdict}")
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        lines.push(format!("  {}  {}  {queries}", check.rule, outcome.as_str()));
    }
    let counts: Vec<String> = [
        ProvisionalOutcome::Matches,
        ProvisionalOutcome::Differs,
        ProvisionalOutcome::NotChecked,
    ]
    .iter()
    .map(|kind| {
        let n = outcomes.iter().filter(|outcome| *outcome == kind).count();
        format!("{}={n}", kind.as_str())
    })
    .collect();
    lines.push(format!("provisional: {}", counts.join("  ")));
    lines
}

/// The verdict of query `id` in this run.
fn verdict_of(verdicts: &[(String, Verdict)], id: &str) -> Option<Verdict> {
    verdicts
        .iter()
        .find(|(query, _)| query == id)
        .map(|(_, verdict)| *verdict)
}

/// entities.md RowDigest: the row's `@timestamp` (or `-`), then its other
/// columns as `name=value`, every value cut to [`VALUE_CHARS`] characters
/// and put on one line.
pub fn row_digest(row: &Row) -> String {
    let time = row
        .iter()
        .find(|(name, _)| name == "@timestamp")
        .map_or("-", |(_, value)| value.as_str());
    let columns: Vec<String> = row
        .iter()
        .filter(|(name, _)| name != "@timestamp")
        .map(|(name, value)| format!("{name}={}", cut(value, VALUE_CHARS)))
        .collect();
    if row.is_empty() {
        return "- (no columns)".to_string();
    }
    format!("{} {}", one_line(time), columns.join(", "))
        .trim_end()
        .to_string()
}

/// The last line: how many queries got each verdict.
pub fn summary_line(verdicts: &[Verdict]) -> String {
    let all = [
        Verdict::Same,
        Verdict::Different,
        Verdict::BothRejected,
        Verdict::OnlyAppRejected,
        Verdict::OnlyInsightsRejected,
        Verdict::Skipped,
        Verdict::NotCompared,
    ];
    let counts: Vec<String> = all
        .iter()
        .map(|kind| {
            let n = verdicts.iter().filter(|verdict| *verdict == kind).count();
            format!("{}={n}", kind.as_str())
        })
        .collect();
    format!("queries: {}  {}", verdicts.len(), counts.join("  "))
}

/// `text` on one line, cut to `limit` characters with `…` when longer.
fn cut(text: &str, limit: usize) -> String {
    let flat = one_line(text);
    if flat.chars().count() <= limit {
        return flat;
    }
    let mut shown: String = flat.chars().take(limit).collect();
    shown.push('…');
    shown
}

/// Every line break becomes a space.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::compare::{AppRows, AppSide, CompareSpec, LISTED_ROWS, compare_query};
    use crate::compat::export::InsightsSide;
    use crate::compat::queries::CompareMode;

    fn row(list: &[(&str, &str)]) -> Row {
        let mut row: Row = list
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        row.sort();
        row
    }

    #[test]
    fn every_query_same_or_both_refused_is_compatible_and_skipped_alone_does_not_fail() {
        assert_eq!(
            exit_code(&[Verdict::Same, Verdict::BothRejected, Verdict::Skipped]),
            EXIT_COMPATIBLE
        );
    }

    #[test]
    fn a_difference_or_a_one_sided_refusal_fails() {
        for verdict in [
            Verdict::Different,
            Verdict::OnlyAppRejected,
            Verdict::OnlyInsightsRejected,
        ] {
            assert_eq!(
                exit_code(&[Verdict::Same, verdict]),
                EXIT_DIFFERENT,
                "{verdict:?}"
            );
        }
    }

    #[test]
    fn a_query_not_compared_means_the_run_could_not_compare() {
        assert_eq!(
            exit_code(&[Verdict::Same, Verdict::Different, Verdict::NotCompared]),
            EXIT_NOT_COMPARED,
            "R-02: a missing or unfinished export outweighs a difference"
        );
    }

    #[test]
    fn no_query_compared_at_all_means_the_run_could_not_compare() {
        assert_eq!(exit_code(&[]), EXIT_NOT_COMPARED);
        assert_eq!(
            exit_code(&[Verdict::Skipped, Verdict::Skipped]),
            EXIT_NOT_COMPARED
        );
    }

    #[test]
    fn a_row_digest_shows_the_time_and_at_most_40_characters_of_each_value_on_one_line() {
        let message = format!(
            "{{\"eventName\":\"Decrypt\",\n\"secret-ish\":\"{}\"}}",
            "x".repeat(200)
        );
        let digest = row_digest(&row(&[
            ("@timestamp", "2026-01-03 10:26:21.414"),
            ("@message", &message),
            ("eventName", "Decrypt"),
        ]));
        assert!(digest.starts_with("2026-01-03 10:26:21.414 "), "{digest}");
        assert!(digest.contains("eventName=Decrypt"), "{digest}");
        let shown: String = message
            .chars()
            .take(VALUE_CHARS)
            .collect::<String>()
            .replace('\n', " ");
        assert!(digest.contains(&format!("@message={shown}…")), "{digest}");
        assert!(
            !digest.contains(&"x".repeat(41)),
            "the whole message is never shown"
        );
        assert!(!digest.contains('\n'));
        assert_eq!(row_digest(&Row::new()), "- (no columns)");
    }

    #[test]
    fn the_report_names_the_verdict_counts_and_differences_and_sums_up() {
        let spec = CompareSpec {
            mode: CompareMode::Set,
            sorted: false,
            sort_keys: Vec::new(),
            cap: 10_000,
            tolerances: Vec::new(),
            key_columns: Vec::new(),
            period_edges: Vec::new(),
        };
        let only_app = row(&[
            ("@timestamp", "2026-01-03 11:24:00.620"),
            ("eventName", "GetObject"),
        ]);
        let comparison = compare_query(
            &spec,
            &AppSide::Rows(AppRows {
                columns: vec!["@timestamp".to_string(), "eventName".to_string()],
                rows: vec![only_app],
                cut: false,
                next_row: None,
            }),
            &InsightsSide::Rows(vec![]),
        );
        let lines = query_lines("q11-nosort-set", &comparison, true);
        assert_eq!(
            lines[0],
            "q11-nosort-set  Different  insights=0  app=1  boundary-events"
        );
        assert_eq!(
            lines[1],
            "    only in app: 2026-01-03 11:24:00.620 eventName=GetObject"
        );
        let refused = compare_query(
            &spec,
            // `filter eventName like eventSource`: still refused by the
            // app (loop-back 2; p05's bare `true` is read now).
            &AppSide::Rejected(
                "Unsupported query.unsupported.fieldComparison (eventSource)".into(),
            ),
            &InsightsSide::Rejected(format!("MalformedQueryException: {}", "y".repeat(300))),
        );
        let lines = query_lines("x01-like-field", &refused, false);
        assert_eq!(lines[0], "x01-like-field  BothRejected  insights=-  app=-");
        assert!(
            lines
                .iter()
                .any(|line| line.contains("app refused: Unsupported"))
        );
        assert!(
            lines
                .iter()
                .all(|line| line.chars().count() < MESSAGE_CHARS + 40)
        );
        assert_eq!(
            summary_line(&[
                Verdict::Same,
                Verdict::Same,
                Verdict::Different,
                Verdict::Skipped
            ]),
            "queries: 4  Same=2  Different=1  BothRejected=0  OnlyAppRejected=0  \
             OnlyInsightsRejected=0  Skipped=1  NotCompared=0"
        );
    }

    // 261025-insights-stats U4 (BR1.4, BR2.5, BR2.7 (5)).

    fn within(column: &str, insights: &str, app: &str, kind: CellDifference) -> WithinCell {
        WithinCell {
            column: column.to_string(),
            insights: insights.to_string(),
            app: app.to_string(),
            kind,
        }
    }

    fn same_within(
        decimal: usize,
        approximate: usize,
        examples: Vec<WithinCell>,
    ) -> QueryComparison {
        QueryComparison {
            verdict: Verdict::Same,
            insights_count: Some(2),
            app_count: Some(2),
            missing_in_app: Vec::new(),
            missing_in_insights: Vec::new(),
            edge_missing_in_app: Vec::new(),
            edge_missing_in_insights: Vec::new(),
            within_decimal: decimal,
            within_approximate: approximate,
            within_examples: examples,
            first_order_mismatch: None,
            columns: None,
            tied_at_cut: None,
            insights_message: None,
            app_message: None,
            note: None,
        }
    }

    /// BR2.5: "Same", "Same (within the decimal tolerance: n cells)",
    /// "Same (within the approximate tolerance: n cells)", "Different".
    #[test]
    fn a_query_same_within_a_tolerance_says_so_with_its_cell_count() {
        let decimal = same_within(
            3,
            0,
            vec![within(
                "avg(latency)",
                "12.34565",
                "12.3456",
                CellDifference::WithinDecimal,
            )],
        );
        let lines = query_lines("n01-num-aggregates", &decimal, false);
        assert_eq!(
            lines[0],
            "n01-num-aggregates  Same (within decimal tolerance: 3 cells)  insights=2  app=2"
        );
        assert_eq!(
            lines[1],
            "    within decimal tolerance: avg(latency) insights=12.34565 app=12.3456"
        );
        let approximate = same_within(
            0,
            1,
            vec![within(
                "count_distinct(eventName)",
                "102",
                "100",
                CellDifference::WithinApproximate,
            )],
        );
        assert_eq!(
            query_lines("s05", &approximate, false)[0],
            "s05  Same (within approximate tolerance: 1 cell)  insights=2  app=2"
        );
        let both = same_within(2, 1, Vec::new());
        assert_eq!(
            query_lines("x", &both, false)[0],
            "x  Same (within decimal tolerance: 2 cells; within approximate tolerance: 1 cell)  \
             insights=2  app=2"
        );
        let exact = same_within(0, 0, Vec::new());
        assert_eq!(
            query_lines("x", &exact, false),
            vec!["x  Same  insights=2  app=2"]
        );
    }

    #[test]
    fn only_the_first_five_cells_within_a_tolerance_are_listed() {
        let examples: Vec<WithinCell> = (0..LISTED_ROWS)
            .map(|n| {
                within(
                    &format!("c{n}"),
                    "1",
                    "1.00001",
                    CellDifference::WithinDecimal,
                )
            })
            .collect();
        let lines = query_lines("n01", &same_within(9, 0, examples), false);
        assert!(lines[0].contains("9 cells"), "{}", lines[0]);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("within decimal tolerance: c"))
                .count(),
            LISTED_ROWS
        );
    }

    /// BR2.7 (5): the rows of the first or the last bin of the period that
    /// differ are listed apart, and the query is still a difference.
    #[test]
    fn differences_at_a_period_edge_are_listed_apart() {
        let edge = row(&[("bin(5m)", "2024-01-02 00:00:00.000"), ("count(*)", "4")]);
        let middle = row(&[("bin(5m)", "2024-01-02 00:05:00.000"), ("count(*)", "2")]);
        let comparison = QueryComparison {
            verdict: Verdict::Different,
            missing_in_app: vec![middle],
            edge_missing_in_app: vec![edge],
            ..same_within(0, 0, Vec::new())
        };
        let lines = query_lines("s01-bin5m", &comparison, false);
        assert_eq!(
            lines[0],
            "s01-bin5m  Different  insights=2  app=2  period-edge-differences"
        );
        assert!(
            lines.contains(
                &"    only in Logs Insights (period edge): - bin(5m)=2024-01-02 00:00:00.000, \
                  count(*)=4"
                    .to_string()
            ),
            "{lines:?}"
        );
        assert!(
            lines.contains(
                &"    only in Logs Insights: - bin(5m)=2024-01-02 00:05:00.000, count(*)=2"
                    .to_string()
            ),
            "{lines:?}"
        );
        assert_eq!(exit_code(&[comparison.verdict]), EXIT_DIFFERENT);
    }

    /// BR2.5: differences within a tolerance only keep the run compatible.
    #[test]
    fn only_differences_within_a_tolerance_exit_with_0() {
        let comparison = same_within(4, 2, Vec::new());
        assert_eq!(
            exit_code(&[comparison.verdict, Verdict::Same]),
            EXIT_COMPATIBLE
        );
    }

    /// BR1.4: per provisional rule, matches when every query that checks it
    /// was compared without a difference, differs when one differs, and
    /// notChecked otherwise; then the counts.
    #[test]
    fn the_provisional_rules_are_summed_up_by_the_outcome_of_their_queries() {
        let check = |rule: &str, ids: &[&str]| ProvisionalRuleCheck {
            rule: rule.to_string(),
            query_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        };
        let checks = vec![
            check("U1:BR2.1", &["s03"]),
            check("U1:BR2.5", &["s01", "s10"]),
            check("U2:BR1.1", &["s13", "s14"]),
            check("U2:BR3.2", &[]),
            check("U2:BR2.4", &["s07", "s99"]),
        ];
        let verdicts = vec![
            ("s01".to_string(), Verdict::Different),
            ("s03".to_string(), Verdict::Same),
            ("s07".to_string(), Verdict::Same),
            ("s10".to_string(), Verdict::Same),
            ("s13".to_string(), Verdict::BothRejected),
            ("s14".to_string(), Verdict::Skipped),
        ];
        assert_eq!(
            provisional_outcome(&checks[0], &verdicts),
            ProvisionalOutcome::Matches
        );
        assert_eq!(
            provisional_outcome(&checks[1], &verdicts),
            ProvisionalOutcome::Differs
        );
        assert_eq!(
            provisional_outcome(&checks[2], &verdicts),
            ProvisionalOutcome::NotChecked,
            "s14 was skipped"
        );
        assert_eq!(
            provisional_outcome(&checks[3], &verdicts),
            ProvisionalOutcome::NotChecked
        );
        assert_eq!(
            provisional_outcome(&checks[4], &verdicts),
            ProvisionalOutcome::NotChecked,
            "s99 was not run"
        );
        assert_eq!(
            provisional_lines(&checks, &verdicts),
            vec![
                "provisional rules:".to_string(),
                "  U1:BR2.1  matches  s03=Same".to_string(),
                "  U1:BR2.5  differs  s01=Different, s10=Same".to_string(),
                "  U2:BR1.1  notChecked  s13=BothRejected, s14=Skipped".to_string(),
                "  U2:BR3.2  notChecked  (no query)".to_string(),
                "  U2:BR2.4  notChecked  s07=Same, s99=-".to_string(),
                "provisional: matches=1  differs=1  notChecked=3".to_string(),
            ]
        );
        assert!(provisional_lines(&[], &verdicts).is_empty());
    }
}
