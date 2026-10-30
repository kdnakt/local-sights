//! QueryCatalog: the one list of the words a query is made of
//! (261025-insights-stats U2:BR4.1, ADR-005).
//!
//! The commands, the aggregate functions of `stats`, the other functions,
//! the keywords and the fixed fields are listed here once, with whether this
//! version runs them. The parser (which commands and functions it reads and
//! which are Unsupported, which words are never field names), the completion
//! candidates, and through `get_query_catalog` the screen's colouring and
//! its "commands you can use" text are all made from this list; a test keeps
//! the README's table in step with it (BR4.4).
//!
//! The names are written as the AWS documentation writes them. Commands and
//! keywords are read in any case (as the parser always did); functions only
//! as written (BR1.1). A reserved keyword (`and`, `or`, `not`, `like`, `in`)
//! is never read as a field name; `by`, `as`, `asc` and `desc` are keywords
//! only where they stand, so a JSON field named `desc` is still read
//! (FR review R-04).

use serde::Serialize;

/// The function `toMillis`, written with this case (U1:BR3.7).
pub const TO_MILLIS: &str = "toMillis";

/// The function `bin` of a `by` key (261025-insights-stats U1:BR1.5).
pub const BIN: &str = "bin";

/// What a word of the catalog is (entities.md QueryVocabulary.kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VocabularyKind {
    /// A command such as `filter`.
    Command,
    /// An aggregate function of `stats` such as `count`.
    AggregateFunction,
    /// Another function: `toMillis`, `bin`.
    Function,
    /// A keyword such as `by` or `and`.
    Keyword,
    /// One of the four `@` fields.
    FixedField,
}

/// One word of the catalog (entities.md QueryVocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryVocabulary {
    /// The word as the documentation writes it.
    pub name: &'static str,
    /// What it is.
    pub kind: VocabularyKind,
    /// Whether this version runs it; a command or function that is not is
    /// an Unsupported problem when read.
    pub supported: bool,
    /// A keyword never read as a field name (`and`, `or`, `not`, `like`,
    /// `in`).
    pub reserved: bool,
}

const fn word(name: &'static str, kind: VocabularyKind, supported: bool) -> QueryVocabulary {
    QueryVocabulary {
        name,
        kind,
        supported,
        reserved: false,
    }
}

const fn command(name: &'static str, supported: bool) -> QueryVocabulary {
    word(name, VocabularyKind::Command, supported)
}

const fn aggregate(name: &'static str) -> QueryVocabulary {
    word(name, VocabularyKind::AggregateFunction, true)
}

const fn keyword(name: &'static str, reserved: bool) -> QueryVocabulary {
    QueryVocabulary {
        name,
        kind: VocabularyKind::Keyword,
        supported: true,
        reserved,
    }
}

const fn fixed_field(name: &'static str) -> QueryVocabulary {
    word(name, VocabularyKind::FixedField, true)
}

/// Every word, in the order the screen and the completion list them.
const VOCABULARY: [QueryVocabulary; 42] = [
    command("fields", true),
    command("filter", true),
    command("sort", true),
    command("limit", true),
    command("stats", true),
    // The console's other commands (U2:BR2.1).
    command("parse", false),
    command("dedup", false),
    command("display", false),
    command("pattern", false),
    command("diff", false),
    command("unmask", false),
    command("unnest", false),
    command("lookup", false),
    command("filterIndex", false),
    command("source", false),
    // The aggregate functions of the documentation's `stats` table (BR1.1).
    aggregate("count"),
    aggregate("count_distinct"),
    aggregate("sum"),
    aggregate("avg"),
    aggregate("min"),
    aggregate("max"),
    aggregate("pct"),
    aggregate("stddev"),
    aggregate("earliest"),
    aggregate("latest"),
    aggregate("sortsFirst"),
    aggregate("sortsLast"),
    word(TO_MILLIS, VocabularyKind::Function, true),
    word(BIN, VocabularyKind::Function, true),
    keyword("by", false),
    keyword("as", false),
    keyword("and", true),
    keyword("or", true),
    keyword("not", true),
    keyword("like", true),
    keyword("in", true),
    keyword("asc", false),
    keyword("desc", false),
    fixed_field("@timestamp"),
    fixed_field("@message"),
    fixed_field("@logStream"),
    fixed_field("@log"),
];

/// The catalog of the query words (ADR-005). It is fixed: it does not change
/// while the app runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryCatalog;

impl QueryCatalog {
    /// Every word, in the catalog's order.
    pub fn vocabulary() -> &'static [QueryVocabulary] {
        &VOCABULARY
    }

    /// The words of `kind` that are (or are not) `supported`, in order.
    pub fn names(kind: VocabularyKind, supported: bool) -> Vec<&'static str> {
        VOCABULARY
            .iter()
            .filter(|word| word.kind == kind && word.supported == supported)
            .map(|word| word.name)
            .collect()
    }

    /// The command `word`, in any case.
    pub fn command(word: &str) -> Option<&'static QueryVocabulary> {
        Self::find(VocabularyKind::Command, |name| name.eq_ignore_ascii_case(word))
    }

    /// The aggregate function `name`, exactly as written (BR1.1).
    pub fn aggregate_function(name: &str) -> Option<&'static QueryVocabulary> {
        Self::find(VocabularyKind::AggregateFunction, |listed| listed == name)
    }

    /// The other function `name`, exactly as written.
    pub fn function(name: &str) -> Option<&'static QueryVocabulary> {
        Self::find(VocabularyKind::Function, |listed| listed == name)
    }

    /// Whether `word` is a reserved keyword, in any case: never a field
    /// name.
    pub fn is_reserved(word: &str) -> bool {
        Self::find(VocabularyKind::Keyword, |name| name.eq_ignore_ascii_case(word))
            .is_some_and(|keyword| keyword.reserved)
    }

    fn find(
        kind: VocabularyKind,
        matches: impl Fn(&str) -> bool,
    ) -> Option<&'static QueryVocabulary> {
        VOCABULARY
            .iter()
            .find(|word| word.kind == kind && matches(word.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(kind: VocabularyKind, supported: bool) -> Vec<&'static str> {
        QueryCatalog::names(kind, supported)
    }

    #[test]
    fn the_commands_are_the_five_read_ones_and_the_ten_of_the_console_not_read() {
        // BR4.1.
        assert_eq!(
            names(VocabularyKind::Command, true),
            vec!["fields", "filter", "sort", "limit", "stats"]
        );
        assert_eq!(
            names(VocabularyKind::Command, false),
            vec![
                "parse",
                "dedup",
                "display",
                "pattern",
                "diff",
                "unmask",
                "unnest",
                "lookup",
                "filterIndex",
                "source"
            ]
        );
    }

    #[test]
    fn the_functions_are_the_twelve_aggregates_and_to_millis_and_bin() {
        // BR1.1, BR4.1.
        assert_eq!(
            names(VocabularyKind::AggregateFunction, true),
            vec![
                "count",
                "count_distinct",
                "sum",
                "avg",
                "min",
                "max",
                "pct",
                "stddev",
                "earliest",
                "latest",
                "sortsFirst",
                "sortsLast"
            ]
        );
        assert!(names(VocabularyKind::AggregateFunction, false).is_empty());
        assert_eq!(
            names(VocabularyKind::Function, true),
            vec![TO_MILLIS, BIN]
        );
        assert_eq!((TO_MILLIS, BIN), ("toMillis", "bin"));
    }

    #[test]
    fn only_and_or_not_like_and_in_are_reserved_keywords() {
        // BR4.1, FR review R-04.
        assert_eq!(
            names(VocabularyKind::Keyword, true),
            vec!["by", "as", "and", "or", "not", "like", "in", "asc", "desc"]
        );
        let reserved: Vec<&str> = QueryCatalog::vocabulary()
            .iter()
            .filter(|word| word.reserved)
            .map(|word| word.name)
            .collect();
        assert_eq!(reserved, vec!["and", "or", "not", "like", "in"]);
        for word in ["AND", "Or", "not", "LIKE", "In"] {
            assert!(QueryCatalog::is_reserved(word), "{word}");
        }
        for word in ["by", "as", "asc", "desc", "level", "stats"] {
            assert!(!QueryCatalog::is_reserved(word), "{word}");
        }
    }

    #[test]
    fn the_fixed_fields_are_the_four_at_names() {
        assert_eq!(
            names(VocabularyKind::FixedField, true),
            vec!["@timestamp", "@message", "@logStream", "@log"]
        );
    }

    #[test]
    fn every_word_is_listed_once() {
        let all = QueryCatalog::vocabulary();
        for (index, word) in all.iter().enumerate() {
            assert!(
                all[index + 1..].iter().all(|other| other.name != word.name),
                "{} twice",
                word.name
            );
        }
        assert_eq!(all.len(), 15 + 14 + 9 + 4);
    }

    #[test]
    fn a_command_is_found_in_any_case_and_a_function_only_as_written() {
        // BR4.1 (commands ignore case as now), BR1.1 (functions as written).
        let filter_index = QueryCatalog::command("FILTERINDEX");
        assert_eq!(filter_index.map(|word| word.name), Some("filterIndex"));
        assert_eq!(filter_index.map(|word| word.supported), Some(false));
        assert_eq!(
            QueryCatalog::command("Stats").map(|word| word.supported),
            Some(true)
        );
        assert_eq!(QueryCatalog::command("by"), None, "a keyword is no command");
        assert_eq!(QueryCatalog::command("unknown"), None);
        assert_eq!(
            QueryCatalog::aggregate_function("sortsFirst").map(|word| word.name),
            Some("sortsFirst")
        );
        assert_eq!(QueryCatalog::aggregate_function("sortsfirst"), None);
        assert_eq!(QueryCatalog::aggregate_function("bin"), None);
        assert_eq!(
            QueryCatalog::function("toMillis").map(|word| word.kind),
            Some(VocabularyKind::Function)
        );
        assert_eq!(QueryCatalog::function("tomillis"), None);
    }

    #[test]
    fn the_catalog_goes_to_the_screen_in_camel_case() {
        // BR4.2.
        let json = serde_json::to_value(QueryCatalog::vocabulary()).unwrap();
        let count_distinct = json
            .as_array()
            .unwrap()
            .iter()
            .find(|word| word["name"] == "count_distinct")
            .unwrap();
        assert_eq!(
            count_distinct,
            &serde_json::json!({
                "name": "count_distinct",
                "kind": "aggregateFunction",
                "supported": true,
                "reserved": false
            })
        );
        let kinds: Vec<serde_json::Value> = [
            VocabularyKind::Command,
            VocabularyKind::Function,
            VocabularyKind::Keyword,
            VocabularyKind::FixedField,
        ]
        .iter()
        .map(|kind| serde_json::to_value(kind).unwrap())
        .collect();
        assert_eq!(
            kinds,
            vec!["command", "function", "keyword", "fixedField"]
        );
    }
}
