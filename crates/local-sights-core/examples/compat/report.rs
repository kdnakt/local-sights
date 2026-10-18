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

use crate::compat::compare::{QueryComparison, Verdict};
use crate::compat::format::Row;

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
        "{id}  {}  insights={}  app={}",
        comparison.verdict.as_str(),
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
    if let Some(message) = &comparison.insights_message {
        detail("Logs Insights refused", cut(message, MESSAGE_CHARS));
    }
    if let Some(message) = &comparison.app_message {
        detail("app refused", cut(message, MESSAGE_CHARS));
    }
    lines
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
    use crate::compat::compare::{AppRows, AppSide, CompareSpec, compare_query};
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
            &AppSide::Rejected("Unsupported query.unsupported.fieldComparison (true)".into()),
            &InsightsSide::Rejected(format!("MalformedQueryException: {}", "y".repeat(300))),
        );
        let lines = query_lines("p05-bool-literal", &refused, false);
        assert_eq!(
            lines[0],
            "p05-bool-literal  BothRejected  insights=-  app=-"
        );
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
}
