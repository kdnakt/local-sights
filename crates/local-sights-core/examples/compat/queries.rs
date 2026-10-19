//! The query set (261017-insights-query U4:BR3.1, entities.md CompatQuery).
//!
//! One query a line: `id<TAB>order|set<TAB>purpose<TAB>query`. A line that
//! starts with `#` is a note and blank lines are skipped. The id is made of
//! ASCII letters, digits and `-` only, since it also names the export file
//! (`<id>.json`), and no two queries share one. The file holds query texts
//! only, never log contents (NFR4). `scripts/compat/insights-export.sh`
//! reads the same file with the same rules.

use std::collections::HashSet;
use std::fmt;

/// Most characters of a query text (entities.md CompatQuery.text, the
/// screen's limit).
pub const MAX_QUERY_CHARS: usize = 10_000;

/// How the rows of a query are compared (entities.md
/// CompatQuery.compareOrder, U4:BR2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// The order counts too: a query with `sort` (D18).
    Order,
    /// Only the collection of rows counts.
    Set,
}

/// One query to compare (entities.md CompatQuery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatQuery {
    /// Letters, digits and `-`; also the export file's name.
    pub id: String,
    /// Order or set.
    pub mode: CompareMode,
    /// What the query checks (entities.md CompatQuery.purpose and note).
    pub purpose: String,
    /// The query as the console and the app read it.
    pub text: String,
}

/// What is wrong with one line of the query set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuerySetProblem {
    /// Not four fields separated by tabs.
    FieldCount,
    /// The id has a character other than letters, digits and `-`, or none.
    BadId,
    /// The id was used on an earlier line.
    DuplicateId(String),
    /// The second field is neither `order` nor `set`.
    BadMode,
    /// The purpose is blank.
    EmptyPurpose,
    /// The query text is blank.
    EmptyQuery,
    /// The query text is longer than [`MAX_QUERY_CHARS`].
    TooLong,
    /// The file has no query at all.
    NoQuery,
}

/// Why the query set cannot be read: the line (from 1; 0 for the whole
/// file) and the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySetError {
    /// Line number, from 1, or 0 for the whole file.
    pub line: usize,
    /// The problem.
    pub problem: QuerySetProblem,
}

impl fmt::Display for QuerySetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match &self.problem {
            QuerySetProblem::FieldCount => {
                "expected id<TAB>order|set<TAB>purpose<TAB>query".to_string()
            }
            QuerySetProblem::BadId => "the id must be letters, digits and '-' only".to_string(),
            QuerySetProblem::DuplicateId(id) => format!("the id {id} is used twice"),
            QuerySetProblem::BadMode => "the second field must be 'order' or 'set'".to_string(),
            QuerySetProblem::EmptyPurpose => "the purpose is empty".to_string(),
            QuerySetProblem::EmptyQuery => "the query is empty".to_string(),
            QuerySetProblem::TooLong => {
                format!("the query is longer than {MAX_QUERY_CHARS} characters")
            }
            QuerySetProblem::NoQuery => "the query set has no query".to_string(),
        };
        if self.line == 0 {
            write!(f, "query set: {what}")
        } else {
            write!(f, "query set line {}: {what}", self.line)
        }
    }
}

/// Reads the query set `text` (U4:BR3.1).
pub fn parse_query_set(text: &str) -> Result<Vec<CompatQuery>, QuerySetError> {
    let mut queries = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let at = |problem| QuerySetError {
            line: index + 1,
            problem,
        };
        let query = parse_line(line).map_err(at)?;
        if !seen.insert(query.id.clone()) {
            return Err(at(QuerySetProblem::DuplicateId(query.id)));
        }
        queries.push(query);
    }
    if queries.is_empty() {
        return Err(QuerySetError {
            line: 0,
            problem: QuerySetProblem::NoQuery,
        });
    }
    Ok(queries)
}

/// One line that is neither a note nor blank.
fn parse_line(line: &str) -> Result<CompatQuery, QuerySetProblem> {
    let fields: Vec<&str> = line.splitn(4, '\t').collect();
    let [id, mode, purpose, text] = fields.as_slice() else {
        return Err(QuerySetProblem::FieldCount);
    };
    if !is_valid_id(id) {
        return Err(QuerySetProblem::BadId);
    }
    let mode = match *mode {
        "order" => CompareMode::Order,
        "set" => CompareMode::Set,
        _ => return Err(QuerySetProblem::BadMode),
    };
    let purpose = purpose.trim();
    if purpose.is_empty() {
        return Err(QuerySetProblem::EmptyPurpose);
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(QuerySetProblem::EmptyQuery);
    }
    if text.chars().count() > MAX_QUERY_CHARS {
        return Err(QuerySetProblem::TooLong);
    }
    Ok(CompatQuery {
        id: (*id).to_string(),
        mode,
        purpose: purpose.to_string(),
        text: text.to_string(),
    })
}

/// ASCII letters, digits and `-` only, at least one (U4 entities.md
/// CompatQuery.id): the id is also a file name.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(text: &str) -> QuerySetError {
        parse_query_set(text).unwrap_err()
    }

    #[test]
    fn lines_are_read_into_queries_in_their_order() {
        let set = parse_query_set(
            "q01-doc-latest\torder\tdoc example\tfields @timestamp | sort @timestamp desc | limit 20\n\
             p03-text-order\tset\tprovisional\tfields eventName | filter eventName > \"M\"\r\n",
        )
        .unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set[0].id, "q01-doc-latest");
        assert_eq!(set[0].mode, CompareMode::Order);
        assert_eq!(set[0].purpose, "doc example");
        assert_eq!(
            set[0].text,
            "fields @timestamp | sort @timestamp desc | limit 20"
        );
        assert_eq!(set[1].mode, CompareMode::Set);
        assert_eq!(set[1].text, "fields eventName | filter eventName > \"M\"");
    }

    #[test]
    fn notes_and_blank_lines_are_skipped() {
        let set = parse_query_set(
            "# the query set\n\n   \n#\tq99\tset\tnote\tlimit 1\nq01\tset\tbasic\tlimit 1\n",
        )
        .unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(set[0].id, "q01");
        assert_eq!(
            problem("# only notes\n\n"),
            QuerySetError {
                line: 0,
                problem: QuerySetProblem::NoQuery
            }
        );
    }

    #[test]
    fn an_id_used_twice_is_an_error_on_its_second_line() {
        let error = problem("q01\tset\ta\tlimit 1\n# note\nq01\tset\tb\tlimit 2\n");
        assert_eq!(error.line, 3);
        assert_eq!(
            error.problem,
            QuerySetProblem::DuplicateId("q01".to_string())
        );
        assert!(error.to_string().contains("line 3"), "{error}");
    }

    #[test]
    fn an_id_with_other_characters_or_a_bad_mode_is_an_error() {
        for id in ["q/01", "q 01", "q_01", "../x", "ｑ01", ""] {
            let line = format!("{id}\tset\tpurpose\tlimit 1\n");
            assert_eq!(problem(&line).problem, QuerySetProblem::BadId, "{id:?}");
        }
        assert_eq!(
            problem("q01\tsorted\tpurpose\tlimit 1").problem,
            QuerySetProblem::BadMode
        );
        assert_eq!(
            problem("q01\tset limit 1").problem,
            QuerySetProblem::FieldCount
        );
    }

    #[test]
    fn an_empty_or_too_long_query_or_purpose_is_an_error() {
        assert_eq!(
            problem("q01\tset\tpurpose\t   ").problem,
            QuerySetProblem::EmptyQuery
        );
        assert_eq!(
            problem("q01\tset\t \tlimit 1").problem,
            QuerySetProblem::EmptyPurpose
        );
        let long = format!(
            "q01\tset\tpurpose\tfilter @message like \"{}\"",
            "あ".repeat(10_000)
        );
        assert_eq!(problem(&long).problem, QuerySetProblem::TooLong);
        let just = format!("q01\tset\tpurpose\t{}", "a".repeat(MAX_QUERY_CHARS));
        assert!(parse_query_set(&just).is_ok());
    }

    #[test]
    fn the_shipped_query_set_is_read_by_the_app_and_order_queries_sort() {
        use local_sights_core::query::parser::parse_query;
        let set = parse_query_set(include_str!("../compat_queries.txt")).unwrap();
        assert_eq!(
            set.len(),
            23,
            "functional-spec 3 lists 20 queries; loop-back 2 adds p10-p12"
        );
        for query in &set {
            // BR3.1: no @log value and no date text in the set.
            assert!(!query.text.contains("@log "), "{}", query.id);
            // A query the app refuses would still be compared: both
            // refusing is no difference (BR2.4).
            match parse_query(&query.text) {
                Ok(plan) => assert_eq!(
                    plan.has_sort(),
                    query.mode == CompareMode::Order,
                    "{}: order exactly when sorted (D18)",
                    query.id
                ),
                Err(problem) => {
                    assert_eq!(query.mode, CompareMode::Set, "{}: {problem:?}", query.id)
                }
            }
        }
        let p09 = set
            .iter()
            .find(|query| query.id == "p09-json-number")
            .unwrap();
        assert!(p09.purpose.contains("JSON の文字列"), "R-04");
        // Loop-back 2: p05's bare `true` is a field now (U2 loop-back 2), and
        // p10-p12 check the provisional rules U2's review left; the app reads
        // them all.
        for id in [
            "p05-bool-literal",
            "p10-field-eq",
            "p11-field-ne-missing",
            "p12-text-vs-digits",
        ] {
            let query = set.iter().find(|query| query.id == id).unwrap();
            assert!(parse_query(&query.text).is_ok(), "{id}");
        }
        let p05 = set
            .iter()
            .find(|query| query.id == "p05-bool-literal")
            .unwrap();
        assert!(p05.purpose.contains("項目"), "{}", p05.purpose);
    }

    /// The app-refused example the compare and report tests use in place of
    /// p05 (loop-back 2): a field after `like` is still refused by the core.
    #[test]
    fn a_field_after_like_is_still_refused_by_the_app() {
        use local_sights_core::query::parser::parse_query;
        use local_sights_core::query::plan::ProblemKind;
        let problem = parse_query("fields eventName | filter eventName like eventSource")
            .expect_err("a field after like is not taken");
        assert_eq!(problem.kind, ProblemKind::Unsupported);
        assert_eq!(problem.message_key, "query.unsupported.fieldComparison");
        assert_eq!(problem.word.as_deref(), Some("eventSource"));
    }
}
