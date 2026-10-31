//! The query set (261017-insights-query U4:BR3.1, entities.md CompatQuery;
//! 261025-insights-stats U4:BR1.3, BR1.4, BR2.8).
//!
//! One query a line: `id<TAB>order|set|keyorder<TAB>purpose<TAB>query`,
//! with an optional fifth field of column tolerances,
//! `<column>=decimal|approximate` separated by `;` (BR1.3). A query text
//! cannot hold a tab, and a column name cannot hold `=` or `;`. A line that
//! starts with `#` is a note and blank lines are skipped; a note
//! `# provisional: <rule> = <id>, <id>` (or `= notChecked`) ties a
//! provisional rule of U1 or U2 to the queries that check it (BR1.4). The id
//! is made of ASCII letters, digits and `-` only, since it also names the
//! export file (`<id>.json`), and no two queries share one. The file holds
//! query texts only, never log contents (NFR4).
//! `scripts/compat/insights-export.sh` reads the same file with the same
//! rules (it takes the fourth field as the query).

use std::collections::HashSet;
use std::fmt;

/// Most characters of a query text (entities.md CompatQuery.text, the
/// screen's limit).
pub const MAX_QUERY_CHARS: usize = 10_000;

/// The start of a provisional note, after `#` (BR1.4).
const PROVISIONAL_NOTE: &str = "provisional:";

/// The ids of a provisional note for a rule no query checks (BR1.4).
const NOT_CHECKED: &str = "notChecked";

/// How the rows of a query are compared (entities.md
/// CompatQuery.compareOrder, U4:BR2.3; 261025-insights-stats U4 entities.md
/// ComparedQuery.mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// The order counts too: a query with `sort` (D18).
    Order,
    /// Only the collection of rows counts.
    Set,
    /// The collection, then the order of the key values of a `stats`
    /// without `sort` (BR2.8, U1:BR2.7).
    KeyOrder,
}

/// How the cells of one column are compared (entities.md
/// ColumnTolerance.kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToleranceKind {
    /// Numbers within 1e-6 relative or 0.0001 absolute are the same
    /// (BR2.2).
    Decimal,
    /// Whole numbers within 2% are the same: `count_distinct` (BR2.3).
    Approximate,
}

/// The tolerance of one column (entities.md ColumnTolerance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnTolerance {
    /// The result column's name, as written in the query.
    pub column: String,
    /// Decimal or approximate.
    pub kind: ToleranceKind,
}

/// One query to compare (entities.md CompatQuery, ComparedQuery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatQuery {
    /// Letters, digits and `-`; also the export file's name.
    pub id: String,
    /// Order, set or keyorder.
    pub mode: CompareMode,
    /// What the query checks (entities.md CompatQuery.purpose and note).
    pub purpose: String,
    /// The query as the console and the app read it.
    pub text: String,
    /// The column tolerances of the fifth field; empty when every cell is
    /// compared exactly (BR1.3).
    pub tolerances: Vec<ColumnTolerance>,
}

/// The tolerance of `column` among `tolerances`, when the fifth field gives
/// one.
pub fn tolerance_of(tolerances: &[ColumnTolerance], column: &str) -> Option<ToleranceKind> {
    tolerances
        .iter()
        .find(|tolerance| tolerance.column == column)
        .map(|tolerance| tolerance.kind)
}

/// A provisional rule of U1 or U2 and the queries that check it (entities.md
/// ProvisionalRuleCheck, BR1.4); no query for a rule recorded as not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionalRuleCheck {
    /// The rule, for example `U1:BR2.1`.
    pub rule: String,
    /// The ids of the queries that check it; empty for `notChecked`.
    pub query_ids: Vec<String>,
}

/// A whole query set: its queries and its provisional notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySet {
    /// The queries in their order.
    pub queries: Vec<CompatQuery>,
    /// The provisional notes in their order (BR1.4).
    pub provisional: Vec<ProvisionalRuleCheck>,
}

/// What is wrong with the fifth field (BR1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToleranceProblem {
    /// An entry without `=`.
    MissingEquals,
    /// An entry with no column name before `=`.
    EmptyColumn,
    /// A kind other than `decimal` or `approximate`.
    UnknownKind(String),
    /// A column given twice.
    DuplicateColumn(String),
}

/// What is wrong with a provisional note (BR1.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionalProblem {
    /// No `=` between the rule and the queries.
    MissingEquals,
    /// No rule, or a rule with a space.
    EmptyRule,
    /// No query id and not `notChecked`.
    NoQuery,
    /// A query id that is not letters, digits and `-`.
    BadId,
    /// A query id that is not in the set.
    UnknownQuery(String),
}

/// What is wrong with one line of the query set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuerySetProblem {
    /// Not four or five fields separated by tabs.
    FieldCount,
    /// The id has a character other than letters, digits and `-`, or none.
    BadId,
    /// The id was used on an earlier line.
    DuplicateId(String),
    /// The second field is neither `order`, `set` nor `keyorder`.
    BadMode,
    /// The purpose is blank.
    EmptyPurpose,
    /// The query text is blank.
    EmptyQuery,
    /// The query text is longer than [`MAX_QUERY_CHARS`].
    TooLong,
    /// The fifth field cannot be read (BR1.3).
    BadTolerance(ToleranceProblem),
    /// A provisional note cannot be read (BR1.4).
    BadProvisional(ProvisionalProblem),
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
                "expected id<TAB>order|set|keyorder<TAB>purpose<TAB>query[<TAB>tolerances]"
                    .to_string()
            }
            QuerySetProblem::BadId => "the id must be letters, digits and '-' only".to_string(),
            QuerySetProblem::DuplicateId(id) => format!("the id {id} is used twice"),
            QuerySetProblem::BadMode => {
                "the second field must be 'order', 'set' or 'keyorder'".to_string()
            }
            QuerySetProblem::BadTolerance(problem) => match problem {
                ToleranceProblem::MissingEquals => {
                    "a tolerance must be <column>=decimal or <column>=approximate".to_string()
                }
                ToleranceProblem::EmptyColumn => "a tolerance has no column name".to_string(),
                ToleranceProblem::UnknownKind(kind) => {
                    format!("the tolerance {kind} is neither 'decimal' nor 'approximate'")
                }
                ToleranceProblem::DuplicateColumn(column) => {
                    format!("the column {column} has two tolerances")
                }
            },
            QuerySetProblem::BadProvisional(problem) => match problem {
                ProvisionalProblem::MissingEquals => {
                    "a provisional note must be '# provisional: <rule> = <id>, ...'".to_string()
                }
                ProvisionalProblem::EmptyRule => {
                    "a provisional note needs a rule without spaces".to_string()
                }
                ProvisionalProblem::NoQuery => {
                    format!("a provisional note needs query ids or {NOT_CHECKED}")
                }
                ProvisionalProblem::BadId => {
                    "a provisional note's ids must be letters, digits and '-' only".to_string()
                }
                ProvisionalProblem::UnknownQuery(id) => {
                    format!("the provisional note names {id}, which is not in the set")
                }
            },
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

/// Reads the queries of the query set `text` (U4:BR3.1), without its
/// provisional notes; compat_check itself reads the whole set with
/// [`read_query_set`].
#[cfg(test)]
pub fn parse_query_set(text: &str) -> Result<Vec<CompatQuery>, QuerySetError> {
    read_query_set(text).map(|set| set.queries)
}

/// Reads the query set `text` with its provisional notes (BR1.3, BR1.4):
/// every id a note names must be a query of the set.
pub fn read_query_set(text: &str) -> Result<QuerySet, QuerySetError> {
    let mut queries = Vec::new();
    let mut notes: Vec<(usize, ProvisionalRuleCheck)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let at = |problem| QuerySetError {
            line: index + 1,
            problem,
        };
        if let Some(note) = line.trim_start().strip_prefix('#') {
            if let Some(check) = parse_provisional(note) {
                notes.push((
                    index + 1,
                    check.map_err(|p| at(QuerySetProblem::BadProvisional(p)))?,
                ));
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
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
    for (line, check) in &notes {
        if let Some(unknown) = check.query_ids.iter().find(|id| !seen.contains(*id)) {
            return Err(QuerySetError {
                line: *line,
                problem: QuerySetProblem::BadProvisional(ProvisionalProblem::UnknownQuery(
                    unknown.clone(),
                )),
            });
        }
    }
    Ok(QuerySet {
        queries,
        provisional: notes.into_iter().map(|(_, check)| check).collect(),
    })
}

/// BR1.4: the note after `#`, when it is a provisional note.
fn parse_provisional(note: &str) -> Option<Result<ProvisionalRuleCheck, ProvisionalProblem>> {
    let body = note.trim_start().strip_prefix(PROVISIONAL_NOTE)?;
    Some(parse_provisional_body(body))
}

fn parse_provisional_body(body: &str) -> Result<ProvisionalRuleCheck, ProvisionalProblem> {
    let (rule, ids) = body
        .split_once('=')
        .ok_or(ProvisionalProblem::MissingEquals)?;
    let rule = rule.trim();
    if rule.is_empty() || rule.contains(char::is_whitespace) {
        return Err(ProvisionalProblem::EmptyRule);
    }
    let ids = ids.trim();
    if ids.is_empty() {
        return Err(ProvisionalProblem::NoQuery);
    }
    let query_ids = if ids == NOT_CHECKED {
        Vec::new()
    } else {
        ids.split(',')
            .map(str::trim)
            .map(|id| {
                is_valid_id(id)
                    .then(|| id.to_string())
                    .ok_or(ProvisionalProblem::BadId)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(ProvisionalRuleCheck {
        rule: rule.to_string(),
        query_ids,
    })
}

/// BR1.3: the fifth field, `<column>=<kind>` separated by `;`; spaces
/// around a name or a kind are not part of it, and empty entries are
/// skipped.
fn parse_tolerances(field: &str) -> Result<Vec<ColumnTolerance>, ToleranceProblem> {
    let mut tolerances: Vec<ColumnTolerance> = Vec::new();
    for entry in field.split(';').map(str::trim).filter(|e| !e.is_empty()) {
        let (column, kind) = entry
            .split_once('=')
            .ok_or(ToleranceProblem::MissingEquals)?;
        let column = column.trim();
        if column.is_empty() {
            return Err(ToleranceProblem::EmptyColumn);
        }
        let kind = match kind.trim() {
            "decimal" => ToleranceKind::Decimal,
            "approximate" => ToleranceKind::Approximate,
            other => return Err(ToleranceProblem::UnknownKind(other.to_string())),
        };
        if tolerance_of(&tolerances, column).is_some() {
            return Err(ToleranceProblem::DuplicateColumn(column.to_string()));
        }
        tolerances.push(ColumnTolerance {
            column: column.to_string(),
            kind,
        });
    }
    Ok(tolerances)
}

/// One line that is neither a note nor blank.
fn parse_line(line: &str) -> Result<CompatQuery, QuerySetProblem> {
    // BR1.3: splitn(5), so a line of four fields reads as before.
    let fields: Vec<&str> = line.splitn(5, '\t').collect();
    let (id, mode, purpose, text, tolerances) = match fields.as_slice() {
        [id, mode, purpose, text] => (*id, *mode, *purpose, *text, ""),
        [id, mode, purpose, text, tolerances] => (*id, *mode, *purpose, *text, *tolerances),
        _ => return Err(QuerySetProblem::FieldCount),
    };
    if !is_valid_id(id) {
        return Err(QuerySetProblem::BadId);
    }
    let mode = match mode {
        "order" => CompareMode::Order,
        "set" => CompareMode::Set,
        "keyorder" => CompareMode::KeyOrder,
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
    let tolerances = parse_tolerances(tolerances).map_err(QuerySetProblem::BadTolerance)?;
    Ok(CompatQuery {
        id: id.to_string(),
        mode,
        purpose: purpose.to_string(),
        text: text.to_string(),
        tolerances,
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

    /// Whether `plan` sorts its result rows: a `sort` among its commands,
    /// or, for a `stats`, a `sort` after it (261025-insights-stats U2:BR1.4).
    fn sorts_its_rows(plan: &local_sights_core::query::plan::QueryPlan) -> bool {
        use local_sights_core::query::plan::AfterStatsCommand;
        match plan.stats() {
            Some(stats) => stats
                .after
                .iter()
                .any(|command| matches!(command, AfterStatsCommand::Sort(_))),
            None => plan.has_sort(),
        }
    }

    #[test]
    fn the_shipped_query_set_is_read_by_the_app_and_order_queries_sort() {
        use local_sights_core::query::parser::parse_query;
        let set = parse_query_set(include_str!("../compat_queries.txt")).unwrap();
        assert_eq!(
            set.len(),
            39,
            "functional-spec 3 lists 20 queries; loop-back 2 adds p10-p12; \
             261025-insights-stats U4 adds s01-s16"
        );
        for query in &set {
            // BR3.1: no @log value and no date text in the set.
            assert!(!query.text.contains("@log "), "{}", query.id);
            // A query the app refuses would still be compared: both
            // refusing is no difference (BR2.4).
            match parse_query(&query.text) {
                Ok(plan) => assert_eq!(
                    sorts_its_rows(&plan),
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

    /// 261025-insights-stats U4 Step 8: both shipped sets are read, their
    /// ids do not overlap, every id a provisional note names is in its set,
    /// every rule of rules.md "仮の決まりと確かめるクエリ" has a note in one of
    /// them, and no stats query is keyed by @log (BR3.2).
    #[test]
    fn the_two_shipped_sets_are_read_with_their_provisional_notes() {
        use local_sights_core::query::parser::parse_query;
        let cloudtrail = read_query_set(include_str!("../compat_queries.txt")).unwrap();
        let numeric = read_query_set(include_str!("../compat_queries_numeric.txt")).unwrap();
        assert_eq!(numeric.queries.len(), 6, "BR1.2: n01-n06");
        let mut ids: HashSet<&str> = HashSet::new();
        for query in cloudtrail.queries.iter().chain(&numeric.queries) {
            assert!(ids.insert(&query.id), "{} is in both sets", query.id);
            assert!(!query.text.contains("by @log"), "{}", query.id);
        }
        for set in [&cloudtrail, &numeric] {
            for check in &set.provisional {
                for id in &check.query_ids {
                    assert!(
                        set.queries.iter().any(|query| &query.id == id),
                        "{}: {id}",
                        check.rule
                    );
                }
            }
        }
        let rules: HashSet<&str> = cloudtrail
            .provisional
            .iter()
            .chain(&numeric.provisional)
            .map(|check| check.rule.as_str())
            .collect();
        for rule in [
            "U1:BR1.1", "U1:BR2.1", "U1:BR2.3", "U1:BR2.5", "U1:BR2.6", "U1:BR2.7", "U1:BR2.8",
            "U2:BR1.1", "U2:BR1.4", "U2:BR2.1", "U2:BR2.2", "U2:BR2.3", "U2:BR2.4", "U2:BR2.5",
            "U2:BR3.2",
        ] {
            assert!(rules.contains(rule), "{rule} has no provisional note");
        }
        let not_checked = cloudtrail
            .provisional
            .iter()
            .find(|check| check.rule == "U2:BR3.2")
            .unwrap();
        assert!(
            not_checked.query_ids.is_empty(),
            "notChecked (FR review R-03)"
        );
        // Every numeric query is read by the app, and each tolerance names a
        // column of its result.
        for query in &numeric.queries {
            let plan = parse_query(&query.text).unwrap_or_else(|p| panic!("{}: {p:?}", query.id));
            assert_eq!(
                sorts_its_rows(&plan),
                query.mode == CompareMode::Order,
                "{}",
                query.id
            );
            let columns = plan.aggregate_columns();
            for tolerance in &query.tolerances {
                assert!(
                    columns.contains(&tolerance.column),
                    "{}: {}",
                    query.id,
                    tolerance.column
                );
            }
        }
        // The keyorder and tolerant queries of the CloudTrail set, too.
        for query in &cloudtrail.queries {
            let Ok(plan) = parse_query(&query.text) else {
                continue;
            };
            if query.mode == CompareMode::KeyOrder {
                assert!(plan.has_stats() && !sorts_its_rows(&plan), "{}", query.id);
            }
            for tolerance in &query.tolerances {
                assert!(
                    plan.aggregate_columns().contains(&tolerance.column),
                    "{}: {}",
                    query.id,
                    tolerance.column
                );
            }
        }
    }

    // 261025-insights-stats U4 (BR1.3, BR1.4, BR2.8): the fifth field, the
    // keyorder mode and the provisional notes.

    fn tolerance(column: &str, kind: ToleranceKind) -> ColumnTolerance {
        ColumnTolerance {
            column: column.to_string(),
            kind,
        }
    }

    #[test]
    fn a_line_of_four_fields_is_read_as_before_with_no_tolerance() {
        // BR1.3: splitn(5) reads a four-field line as splitn(4) did.
        let set =
            parse_query_set("q01\torder\tdoc\tfields @timestamp | sort @timestamp desc\n").unwrap();
        assert_eq!(set[0].text, "fields @timestamp | sort @timestamp desc");
        assert!(set[0].tolerances.is_empty());
        // A fifth field with nothing in it is no tolerance either.
        let blank = parse_query_set("q01\tset\tdoc\tlimit 1\t  \n").unwrap();
        assert!(blank[0].tolerances.is_empty());
    }

    #[test]
    fn the_fifth_field_gives_each_column_its_tolerance() {
        let set = parse_query_set(
            "n01\tset\tavg\tstats avg(latency), count_distinct(status) by status\t\
             avg(latency)=decimal;count_distinct(status)=approximate\n",
        )
        .unwrap();
        assert_eq!(
            set[0].text,
            "stats avg(latency), count_distinct(status) by status"
        );
        assert_eq!(
            set[0].tolerances,
            vec![
                tolerance("avg(latency)", ToleranceKind::Decimal),
                tolerance("count_distinct(status)", ToleranceKind::Approximate),
            ]
        );
        assert_eq!(
            tolerance_of(&set[0].tolerances, "avg(latency)"),
            Some(ToleranceKind::Decimal)
        );
        assert_eq!(tolerance_of(&set[0].tolerances, "status"), None);
    }

    #[test]
    fn spaces_around_a_column_name_go_and_spaces_inside_it_stay() {
        // `pct(latency, 50)` is the column name as written, with its space.
        let set = parse_query_set(
            "n02\tset\tpct\tstats pct(latency, 50)\t pct(latency, 50) = decimal ; \n",
        )
        .unwrap();
        assert_eq!(
            set[0].tolerances,
            vec![tolerance("pct(latency, 50)", ToleranceKind::Decimal)]
        );
    }

    #[test]
    fn a_fifth_field_that_cannot_be_read_is_an_error_of_its_line() {
        let cases = [
            (
                "avg(x)=exact",
                ToleranceProblem::UnknownKind("exact".to_string()),
            ),
            ("avg(x)", ToleranceProblem::MissingEquals),
            ("=decimal", ToleranceProblem::EmptyColumn),
            (
                "avg(x)=decimal;avg(x)=approximate",
                ToleranceProblem::DuplicateColumn("avg(x)".to_string()),
            ),
        ];
        for (field, expected) in cases {
            let error = problem(&format!("# note\nn01\tset\tavg\tstats avg(x)\t{field}\n"));
            assert_eq!(error.line, 2, "{field}");
            assert_eq!(
                error.problem,
                QuerySetProblem::BadTolerance(expected),
                "{field}"
            );
            assert!(error.to_string().contains("line 2"), "{error}");
        }
    }

    #[test]
    fn the_keyorder_mode_is_read() {
        // BR2.8: the order of the key values of a stats without sort.
        let set = parse_query_set("s16\tkeyorder\tkeys\tstats count(*) by eventSource\n").unwrap();
        assert_eq!(set[0].mode, CompareMode::KeyOrder);
        assert_eq!(
            problem("s16\tkey-order\tkeys\tstats count(*)").problem,
            QuerySetProblem::BadMode
        );
    }

    #[test]
    fn provisional_notes_name_their_rule_and_queries() {
        // BR1.4: `# provisional: <rule> = <ids>`, or `notChecked`.
        let set = read_query_set(
            "# provisional: U1:BR2.1 = s03\n\
             #provisional:U1:BR2.5 = s01, n03\n\
             # provisional: U2:BR3.2 = notChecked\n\
             # a plain note: provisional rules are listed above\n\
             s01\tset\tbin\tstats count(*) by bin(5m)\n\
             s03\tset\tmissing key\tstats count(*) by errorCode\n\
             n03\tset\tavg bin\tstats avg(latency) by bin(5m)\tavg(latency)=decimal\n",
        )
        .unwrap();
        assert_eq!(set.queries.len(), 3);
        let check = |rule: &str, ids: &[&str]| ProvisionalRuleCheck {
            rule: rule.to_string(),
            query_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        };
        assert_eq!(
            set.provisional,
            vec![
                check("U1:BR2.1", &["s03"]),
                check("U1:BR2.5", &["s01", "n03"]),
                check("U2:BR3.2", &[]),
            ]
        );
        // parse_query_set still gives the queries alone.
        assert_eq!(
            parse_query_set("s01\tset\tbin\tstats count(*)\n")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_provisional_note_that_cannot_be_read_or_names_no_query_of_the_set_is_an_error() {
        let query = "s01\tset\tbin\tstats count(*) by bin(5m)\n";
        for (note, expected) in [
            ("# provisional: U1:BR2.1", ProvisionalProblem::MissingEquals),
            ("# provisional:  = s01", ProvisionalProblem::EmptyRule),
            ("# provisional: U1:BR2.1 = ", ProvisionalProblem::NoQuery),
            (
                "# provisional: U1:BR2.1 = s01,,s02",
                ProvisionalProblem::BadId,
            ),
            (
                "# provisional: U1:BR2.1 = s99",
                ProvisionalProblem::UnknownQuery("s99".to_string()),
            ),
        ] {
            let error = read_query_set(&format!("{query}{note}\n")).unwrap_err();
            assert_eq!(error.line, 2, "{note}");
            assert_eq!(
                error.problem,
                QuerySetProblem::BadProvisional(expected),
                "{note}"
            );
        }
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
