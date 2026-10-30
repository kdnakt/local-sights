//! QueryParser: reads a query of `fields`, `filter`, `sort`, `limit` and
//! `stats` commands (261017-insights-query U1:BR3.1-BR3.9, BR5.4, BR5.6,
//! BR5.7, U2:BR1.1, BR1.3, BR1.4, BR2.1-BR2.5, BR4.1, BR5.1, BR10.3;
//! 261025-insights-stats U1:BR1.1-BR1.9).
//!
//! `stats` takes aggregates (the twelve functions of the documentation, each
//! with an optional `as name`; `pct` also takes its percentile) and
//! optionally `by` keys (fields or `bin(period)`). Inside `bin(` alone the
//! period is one word, so `5m` is read there and stays a mistake elsewhere
//! (BR1.8). A column without `as` is named by its text as written, blanks
//! made one and notes left out (BR1.9). After `stats`, `filter`, `sort` and
//! `limit` are read into it and name the columns of its rows, not fields of
//! the events; `fields` and a second `stats` are Unsupported
//! (261025-insights-stats U2:BR1.1-BR1.5).
//!
//! Which commands are read and which are Unsupported, which aggregate
//! functions there are and which keywords are never fields all come from the
//! [`QueryCatalog`] (U2:BR4.1).
//!
//! The text is read word by word, in order, and the first problem found is
//! returned with its place, kind and words (U1:BR3.9, U2:BR2.2). Blanks and
//! line breaks only separate words; `#` starts a note up to the end of the
//! line outside quotes and regular expressions (U1:BR3.1). Commands are
//! separated by `|`; the console's other commands are Unsupported and any
//! other word is a writing mistake (U2:BR2.1). Command names, `asc`, `desc`
//! and the words `and`, `or`, `not`, `like` and `in` ignore case; field
//! names do not (U1:BR3.3: FILTER, LIKE, IN, Fields, Sort, Desc and Limit as
//! checked at the console, Q1a 3a, 3c, 3d; AND, OR, NOT, NOT LIKE and NOT IN
//! provisional until U4, review R-03). Conditions bind, from the tightest, as
//! comparison / `like` / `=~` / `in`, then `and`, then `or`, and parentheses
//! change that (U2:BR2.5, Q1a 4a-4c). At the start of a condition `not` must
//! be followed by a parenthesized condition; after a field, `not like` and
//! `not in` are operators of their own (U2:BR2.4, Q1a 3b, 4d-4f, 5e, R-07).
//!
//! A field is one of the four fixed `@` names or a JSON field: keys joined by
//! `.`, a key in backquotes when it holds other characters (U2:BR1.1,
//! BR1.3). A key of digits only is an array element, which U2 does not read;
//! in backquotes it is an object key like any other (U2:BR1.4, R-06). Every
//! part of a condition keeps its place, from its first word to its last
//! (U2:BR10.3).
//!
//! On the right of a comparison (`=`, `!=`, `<`, `<=`, `>`, `>=`) a name
//! without quotes is another field, read as the left one is, so `readOnly =
//! true` compares with a field named `true` (Loop-back 2, p05, as the
//! console reads it). After `like`, `=~` and in an `in` list only values are
//! taken.
//!
//! A `/` starts a regular expression and a `-` before a digit starts a
//! negative number only where a value is expected; elsewhere both are
//! arithmetic operators, which are not run (U1:BR5.7). The word reader is
//! told which of the two it is reading, so a query is read in one pass.

use crate::query::catalog::{BIN, QueryCatalog, TO_MILLIS};
use crate::query::instant::is_date_text;
use crate::query::plan::{
    AfterStatsCommand, AggregateArgument, AggregateFunction, AggregateSpec, BinPeriod, BinUnit,
    ColumnRef, Command, CommandKind, CompareOp, Condition, ConditionKind, FieldRef, GroupKey,
    GroupKeyKind, JsonPath, LikePattern, Literal, MAX_LIMIT, Percentile, QueryPlan, QueryProblem,
    QueryRegex, SortDirection, SortKey, Span, StatsCommand,
};

/// Message-catalog keys of the reading problems (the screen's
/// `query.syntax.*` and `query.unsupported.*` texts).
pub mod keys {
    /// The query has no command (only blanks and notes).
    pub const NO_COMMAND: &str = "query.syntax.noCommand";
    /// A `|` with no command before or after it (U1:BR3.1).
    pub const EMPTY_COMMAND: &str = "query.syntax.emptyCommand";
    /// A word that is no command (U1:BR3.2).
    pub const UNKNOWN_COMMAND: &str = "query.syntax.unknownCommand";
    /// `filter` with no condition after it (U1:BR3.2).
    pub const MISSING_CONDITION: &str = "query.syntax.missingCondition";
    /// An opening parenthesis that is never closed (U1:BR3.4).
    pub const UNCLOSED_PAREN: &str = "query.syntax.unclosedParen";
    /// A closing parenthesis that closes nothing (U1:BR3.4).
    pub const UNMATCHED_PAREN: &str = "query.syntax.unmatchedParen";
    /// A quote that is never closed (U1:BR3.5).
    pub const UNCLOSED_STRING: &str = "query.syntax.unclosedString";
    /// A regular expression whose closing `/` is missing (U1:BR3.5).
    pub const UNCLOSED_REGEX: &str = "query.syntax.unclosedRegex";
    /// A `[` list that is never closed (U1:BR3.5).
    pub const UNCLOSED_LIST: &str = "query.syntax.unclosedList";
    /// A backquoted name that is never closed.
    pub const UNCLOSED_NAME: &str = "query.syntax.unclosedName";
    /// An empty `[]` list (U1:BR3.5).
    pub const EMPTY_LIST: &str = "query.syntax.emptyList";
    /// A word or sign where it cannot stand.
    pub const UNEXPECTED_TOKEN: &str = "query.syntax.unexpectedToken";
    /// The query ends in the middle of a condition.
    pub const UNEXPECTED_END: &str = "query.syntax.unexpectedEnd";
    /// A field with no operator after it.
    pub const EXPECTED_OPERATOR: &str = "query.syntax.expectedOperator";
    /// An operator with no value after it.
    pub const EXPECTED_VALUE: &str = "query.syntax.expectedValue";
    /// `in` with no `[` list after it.
    pub const EXPECTED_LIST: &str = "query.syntax.expectedList";
    /// A character no query uses.
    pub const UNEXPECTED_CHARACTER: &str = "query.syntax.unexpectedCharacter";
    /// A number followed by letters, such as `5m`.
    pub const INVALID_NUMBER: &str = "query.syntax.invalidNumber";
    /// A regular expression that cannot be built (U1:BR3.8).
    pub const INVALID_REGEX: &str = "query.syntax.invalidRegex";
    /// A comparison or `in` with a regular expression (U1:BR5.1).
    pub const REGEX_NOT_ALLOWED: &str = "query.syntax.regexNotAllowed";
    /// `=~` with something else than a regular expression (U1:BR5.4).
    pub const MATCH_NEEDS_REGEX: &str = "query.syntax.matchNeedsRegex";
    /// `like` with something else than a string or a regular expression.
    pub const LIKE_NEEDS_TEXT: &str = "query.syntax.likeNeedsText";
    /// A date text in neither accepted form (U1:BR5.9).
    pub const INVALID_DATE_TIME: &str = "query.syntax.invalidDateTime";
    /// A date text the chosen time zone skips (U1:BR5.9).
    pub const NONEXISTENT_DATE_TIME: &str = "query.syntax.nonexistentDateTime";
    /// A command this version does not run (U1:BR3.2).
    pub const UNSUPPORTED_COMMAND: &str = "query.unsupported.command";
    /// A field this version does not read (U1:BR3.6).
    pub const UNSUPPORTED_FIELD: &str = "query.unsupported.field";
    /// A function this version does not run (U1:BR3.7).
    pub const UNSUPPORTED_FUNCTION: &str = "query.unsupported.function";
    /// `toMillis` of something else than `@timestamp` (U1:BR3.7).
    pub const UNSUPPORTED_TO_MILLIS_ARGUMENT: &str = "query.unsupported.toMillisArgument";
    /// `like` or `=~` on `@timestamp` (U1:BR5.6).
    pub const UNSUPPORTED_TIMESTAMP_OPERATOR: &str = "query.unsupported.timestampOperator";
    /// An arithmetic operator (U1:BR5.7).
    pub const UNSUPPORTED_OPERATOR: &str = "query.unsupported.operator";
    /// A field after `like`, `not like`, `=~`, or in an `in` list; only the
    /// comparisons take another field (Loop-back 2, p05).
    pub const UNSUPPORTED_FIELD_COMPARISON: &str = "query.unsupported.fieldComparison";
    /// `not` at the start of a condition without `(` after it (U2:BR2.4).
    pub const NOT_NEEDS_PAREN: &str = "query.syntax.notNeedsParen";
    /// `fields` or `sort` with no field after it (U2:BR2.2).
    pub const MISSING_FIELDS: &str = "query.syntax.missingFields";
    /// Something else than a field where a field is expected (U2:BR2.2).
    pub const EXPECTED_FIELD: &str = "query.syntax.expectedField";
    /// A `sort` direction other than `asc` and `desc` (U2:BR4.1).
    pub const INVALID_SORT_DIRECTION: &str = "query.syntax.invalidSortDirection";
    /// `limit` with no number after it (U2:BR5.1).
    pub const MISSING_LIMIT: &str = "query.syntax.missingLimit";
    /// A `limit` that is no whole number from 0 (U2:BR5.1).
    pub const INVALID_LIMIT: &str = "query.syntax.invalidLimit";
    /// A `limit` above 100,000 (U2:BR5.1, Q1a 7b).
    pub const LIMIT_TOO_LARGE: &str = "query.syntax.limitTooLarge";
    /// An array element such as `a.0` or `a[0]` (U2:BR1.4, R-06).
    pub const UNSUPPORTED_ARRAY_ELEMENT: &str = "query.unsupported.arrayElement";
    /// `fields ... as name` (U2:BR2.1).
    pub const UNSUPPORTED_ALIAS: &str = "query.unsupported.alias";
    /// `limit any` (U2:BR2.1).
    pub const UNSUPPORTED_LIMIT_ANY: &str = "query.unsupported.limitAny";
    /// Parentheses nested deeper than
    /// [`MAX_NESTING`](super::MAX_NESTING) (261017-insights-query U3:BR5.1).
    pub const NESTING_TOO_DEEP: &str = "query.syntax.nestingTooDeep";
    /// `stats` with no aggregate after it (261025-insights-stats BR1.7).
    pub const MISSING_AGGREGATE: &str = "query.syntax.missingAggregate";
    /// Something else than `function(...)` where an aggregate is expected
    /// (BR1.1).
    pub const EXPECTED_AGGREGATE: &str = "query.syntax.expectedAggregate";
    /// `by` with no key after it (BR1.7).
    pub const MISSING_GROUP_KEY: &str = "query.syntax.missingGroupKey";
    /// A `,` with nothing after it in `stats` (BR1.7).
    pub const TRAILING_COMMA: &str = "query.syntax.trailingComma";
    /// An aggregate function with the wrong number of arguments (BR1.7).
    pub const ARGUMENT_COUNT: &str = "query.syntax.argumentCount";
    /// `*` as the argument of another function than `count` (BR1.3).
    pub const AGGREGATE_STAR: &str = "query.syntax.aggregateStar";
    /// `as` with no column name after it (BR1.4).
    pub const EXPECTED_ALIAS: &str = "query.syntax.expectedAlias";
    /// Two `stats` columns of the same name (BR1.4).
    pub const DUPLICATE_COLUMN: &str = "query.syntax.duplicateColumn";
    /// A `bin()` period in no accepted form (BR1.5, BR1.8).
    pub const INVALID_BIN_PERIOD: &str = "query.syntax.invalidBinPeriod";
    /// A `bin()` unit of the console this version does not read (BR1.5).
    pub const UNSUPPORTED_BIN_UNIT: &str = "query.unsupported.binUnit";
    /// An aggregate function the QueryCatalog lists as not run (BR1.2; since
    /// U2 the catalog runs all twelve).
    pub const UNSUPPORTED_AGGREGATE: &str = "query.unsupported.aggregateFunction";
    /// A `pct` percentile outside 0 to 100 (U2:BR1.2).
    pub const PCT_OUT_OF_RANGE: &str = "query.syntax.pctOutOfRange";
    /// A name after `stats` that is no column of its rows (U2:BR1.4).
    pub const UNKNOWN_RESULT_COLUMN: &str = "query.syntax.unknownResultColumn";
    /// `fields` after `stats` (U2:BR1.5).
    pub const FIELDS_AFTER_STATS: &str = "query.unsupported.fieldsAfterStats";
    /// A second `stats` (U2:BR1.5).
    pub const SECOND_STATS: &str = "query.unsupported.secondStats";

    /// Every key above, for the screen's catalog check.
    pub const ALL: [&str; 57] = [
        NO_COMMAND,
        EMPTY_COMMAND,
        UNKNOWN_COMMAND,
        MISSING_CONDITION,
        UNCLOSED_PAREN,
        UNMATCHED_PAREN,
        UNCLOSED_STRING,
        UNCLOSED_REGEX,
        UNCLOSED_LIST,
        UNCLOSED_NAME,
        EMPTY_LIST,
        UNEXPECTED_TOKEN,
        UNEXPECTED_END,
        EXPECTED_OPERATOR,
        EXPECTED_VALUE,
        EXPECTED_LIST,
        UNEXPECTED_CHARACTER,
        INVALID_NUMBER,
        INVALID_REGEX,
        REGEX_NOT_ALLOWED,
        MATCH_NEEDS_REGEX,
        LIKE_NEEDS_TEXT,
        INVALID_DATE_TIME,
        NONEXISTENT_DATE_TIME,
        UNSUPPORTED_COMMAND,
        UNSUPPORTED_FIELD,
        UNSUPPORTED_FUNCTION,
        UNSUPPORTED_TO_MILLIS_ARGUMENT,
        UNSUPPORTED_TIMESTAMP_OPERATOR,
        UNSUPPORTED_OPERATOR,
        UNSUPPORTED_FIELD_COMPARISON,
        NOT_NEEDS_PAREN,
        MISSING_FIELDS,
        EXPECTED_FIELD,
        INVALID_SORT_DIRECTION,
        MISSING_LIMIT,
        INVALID_LIMIT,
        LIMIT_TOO_LARGE,
        UNSUPPORTED_ARRAY_ELEMENT,
        UNSUPPORTED_ALIAS,
        UNSUPPORTED_LIMIT_ANY,
        NESTING_TOO_DEEP,
        MISSING_AGGREGATE,
        EXPECTED_AGGREGATE,
        MISSING_GROUP_KEY,
        TRAILING_COMMA,
        ARGUMENT_COUNT,
        AGGREGATE_STAR,
        EXPECTED_ALIAS,
        DUPLICATE_COLUMN,
        INVALID_BIN_PERIOD,
        UNSUPPORTED_BIN_UNIT,
        UNSUPPORTED_AGGREGATE,
        PCT_OUT_OF_RANGE,
        UNKNOWN_RESULT_COLUMN,
        FIELDS_AFTER_STATS,
        SECOND_STATS,
    ];
}

/// Deepest nesting of parentheses a condition may have, a `not (` counting
/// as one (261017-insights-query U3:BR5.1, U2 code review R-01).
pub const MAX_NESTING: usize = 100;

/// The `bin()` units of the console not read yet (BR1.5, Q1 A).
const LATER_BIN_UNITS: [&str; 5] = ["ms", "w", "mo", "q", "y"];

/// Largest count of a `bin()` period (BR1.8): with days it stays far inside
/// `i64` milliseconds.
const MAX_BIN_COUNT: u64 = 1_000_000;

/// Reads `text` into a plan, or returns the first problem found, in the
/// order of the text (U1:BR3.9, U2:BR2.2).
pub fn parse_query(text: &str) -> Result<QueryPlan, QueryProblem> {
    let mut parser = Parser::new(text);
    let mut commands = Vec::new();
    let mut previous_pipe: Option<Span> = None;
    loop {
        let command = match parser.next()? {
            None => {
                return Err(match previous_pipe {
                    Some(span) => QueryProblem::syntax(keys::EMPTY_COMMAND, span),
                    None => QueryProblem::syntax(keys::NO_COMMAND, Span::new(1, 1, 1)),
                });
            }
            Some(token) if token.kind == TokenKind::Pipe => {
                return Err(QueryProblem::syntax(keys::EMPTY_COMMAND, token.span));
            }
            Some(token) => token,
        };
        let Command { kind, span } = parser.command(command)?;
        // U2:BR1.4: a command after `stats` goes into it.
        match (commands.last_mut().map(|last: &mut Command| &mut last.kind), kind) {
            (Some(CommandKind::Stats(stats)), kind) => stats.after.push(after_stats(kind, span)?),
            (_, CommandKind::Stats(stats)) => {
                parser.result_columns = Some(stats.result_columns());
                commands.push(Command {
                    kind: CommandKind::Stats(stats),
                    span,
                });
            }
            (_, kind) => commands.push(Command { kind, span }),
        }
        match parser.next()? {
            None => return Ok(QueryPlan { commands }),
            Some(token) if token.kind == TokenKind::Pipe => previous_pipe = Some(token.span),
            Some(token) => return Err(unexpected(token)),
        }
    }
}

/// A command read after `stats`, written at `span`, as one of its commands
/// (U2:BR1.4, BR1.5). The reader refuses `fields` and `stats` there already.
fn after_stats(kind: CommandKind, span: Span) -> Result<AfterStatsCommand, QueryProblem> {
    match kind {
        CommandKind::Filter(condition) => Ok(AfterStatsCommand::Filter(condition)),
        CommandKind::Sort(sort_keys) => Ok(AfterStatsCommand::Sort(sort_keys)),
        CommandKind::Limit(count) => Ok(AfterStatsCommand::Limit(count)),
        CommandKind::Fields(_) => Err(QueryProblem::unsupported(
            keys::FIELDS_AFTER_STATS,
            "fields",
            span,
        )),
        CommandKind::Stats(_) => Err(QueryProblem::unsupported(
            keys::SECOND_STATS,
            "stats",
            span,
        )),
    }
}

/// The kinds of words a query is made of.
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Pipe,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Compare(CompareOp),
    /// `=~`
    Match,
    /// `+ - * / ^ %`, which are not run (U1:BR5.7).
    Arith,
    /// A name: a command, a field, a keyword or a function name; parts
    /// joined by `.`, each bare or in backquotes (U2:BR1.1, BR1.3).
    Name(Vec<NamePart>),
    /// A quoted string, already unescaped.
    Text(String),
    Number(f64),
    /// A regular expression, `\/` already read as `/`.
    Regex(String),
}

/// One part of a name between dots.
#[derive(Debug, Clone, PartialEq)]
struct NamePart {
    /// The characters of the part, without backquotes.
    text: String,
    /// Written in backquotes.
    quoted: bool,
}

impl TokenKind {
    /// The word of a name of one bare part (a command, a keyword, a
    /// function or a fixed field), or `None`.
    fn bare(&self) -> Option<&str> {
        match self {
            TokenKind::Name(parts) => match parts.as_slice() {
                [part] if !part.quoted => Some(part.text.as_str()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether the token is the bare word `word`, in any case.
    fn is_word(&self, word: &str) -> bool {
        self.bare()
            .is_some_and(|bare| bare.eq_ignore_ascii_case(word))
    }
}

/// One word with where it was written and how.
#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    span: Span,
    /// The characters as written.
    raw: String,
    /// Offset of the first character in the text, in characters.
    start: usize,
    /// Offset just after the last character, in characters.
    end: usize,
}

/// Whether a value is expected next: then `/` starts a regular expression
/// and `-` before a digit starts a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Value,
}

/// Cuts the text into words, keeping lines and columns in characters.
struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    column: u32,
}

impl Lexer {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek_char(0)?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// One character at the current place.
    fn here(&self) -> Span {
        Span::new(self.line, self.column, 1)
    }

    /// Skips blanks, line breaks and notes (U1:BR3.1).
    fn skip_blanks_and_notes(&mut self) {
        while let Some(c) = self.peek_char(0) {
            if c.is_whitespace() {
                self.bump();
            } else if c == '#' {
                while self.peek_char(0).is_some_and(|c| c != '\n') {
                    self.bump();
                }
            } else {
                return;
            }
        }
    }

    /// The next word, or `None` at the end of the text.
    fn next(&mut self, mode: Mode) -> Result<Option<Token>, QueryProblem> {
        self.skip_blanks_and_notes();
        let start_span = self.here();
        let start = self.pos;
        let Some(c) = self.peek_char(0) else {
            return Ok(None);
        };
        let kind = match c {
            '|' | '(' | ')' | '[' | ']' | ',' => {
                self.bump();
                match c {
                    '|' => TokenKind::Pipe,
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    _ => TokenKind::Comma,
                }
            }
            '=' | '!' | '<' | '>' => self.operator(c, start_span)?,
            '"' | '\'' => self.string(c, start_span)?,
            '`' => self.name()?,
            '/' if mode == Mode::Value => self.regex(start_span)?,
            '-' if mode == Mode::Value && self.peek_char(1).is_some_and(|d| d.is_ascii_digit()) => {
                self.number(start_span)?
            }
            '+' | '-' | '*' | '/' | '^' | '%' => {
                self.bump();
                TokenKind::Arith
            }
            d if d.is_ascii_digit() => self.number(start_span)?,
            w if is_word_start(w) => self.name()?,
            other => {
                self.bump();
                return Err(QueryProblem::syntax(keys::UNEXPECTED_CHARACTER, start_span)
                    .with_word(other.to_string()));
            }
        };
        Ok(Some(self.token(kind, start, start_span)))
    }

    fn token(&self, kind: TokenKind, start: usize, start_span: Span) -> Token {
        let length = u32::try_from(self.pos - start).unwrap_or(u32::MAX);
        Token {
            kind,
            span: Span::new(start_span.line, start_span.column, length),
            raw: self.chars[start..self.pos].iter().collect(),
            start,
            end: self.pos,
        }
    }

    /// `=`, `=~`, `!=`, `<`, `<=`, `>` and `>=`.
    fn operator(&mut self, first: char, start: Span) -> Result<TokenKind, QueryProblem> {
        self.bump();
        let followed_by = |lexer: &mut Self, c: char| {
            let found = lexer.peek_char(0) == Some(c);
            if found {
                lexer.bump();
            }
            found
        };
        Ok(match first {
            '=' if followed_by(self, '~') => TokenKind::Match,
            '=' => TokenKind::Compare(CompareOp::Eq),
            '!' if followed_by(self, '=') => TokenKind::Compare(CompareOp::NotEq),
            '<' if followed_by(self, '=') => TokenKind::Compare(CompareOp::LtEq),
            '<' => TokenKind::Compare(CompareOp::Lt),
            '>' if followed_by(self, '=') => TokenKind::Compare(CompareOp::GtEq),
            '>' => TokenKind::Compare(CompareOp::Gt),
            _ => {
                return Err(
                    QueryProblem::syntax(keys::UNEXPECTED_CHARACTER, start).with_word(first)
                );
            }
        })
    }

    /// A string in `quote`s; `\"`, `\'` and `\\` are one character, any
    /// other backslash stays as written (U1:BR3.5).
    fn string(&mut self, quote: char, start: Span) -> Result<TokenKind, QueryProblem> {
        self.bump();
        let mut text = String::new();
        loop {
            match self.bump() {
                None => return Err(QueryProblem::syntax(keys::UNCLOSED_STRING, start)),
                Some('\\') => match self.peek_char(0) {
                    Some(c @ ('"' | '\'' | '\\')) => {
                        self.bump();
                        text.push(c);
                    }
                    _ => text.push('\\'),
                },
                Some(c) if c == quote => return Ok(TokenKind::Text(text)),
                Some(c) => text.push(c),
            }
        }
    }

    /// A name: parts joined by `.`, each a run of name characters or any
    /// characters in backquotes (U2:BR1.1, BR1.3). A `.` belongs to the name
    /// only when another part follows it.
    fn name(&mut self) -> Result<TokenKind, QueryProblem> {
        let mut parts = Vec::new();
        loop {
            if self.peek_char(0) == Some('`') {
                let open = self.here();
                self.bump();
                let mut text = String::new();
                loop {
                    match self.bump() {
                        None => return Err(QueryProblem::syntax(keys::UNCLOSED_NAME, open)),
                        Some('`') => break,
                        Some(c) => text.push(c),
                    }
                }
                parts.push(NamePart { text, quoted: true });
            } else {
                let first = self.pos;
                while self.peek_char(0).is_some_and(is_name_char) {
                    self.bump();
                }
                parts.push(NamePart {
                    text: self.chars[first..self.pos].iter().collect(),
                    quoted: false,
                });
            }
            let continues = self.peek_char(0) == Some('.')
                && self
                    .peek_char(1)
                    .is_some_and(|c| c == '`' || is_name_char(c));
            if !continues {
                return Ok(TokenKind::Name(parts));
            }
            self.bump();
        }
    }

    /// A regular expression up to the next `/` on the same line; `\/` is a
    /// `/` and every other backslash pair stays as written (U1:BR3.5).
    fn regex(&mut self, start: Span) -> Result<TokenKind, QueryProblem> {
        self.bump();
        let mut source = String::new();
        loop {
            match self.peek_char(0) {
                None | Some('\n') => {
                    return Err(QueryProblem::syntax(keys::UNCLOSED_REGEX, start));
                }
                Some('/') => {
                    self.bump();
                    return Ok(TokenKind::Regex(source));
                }
                Some('\\') => {
                    self.bump();
                    match self.peek_char(0) {
                        Some('/') => {
                            self.bump();
                            source.push('/');
                        }
                        Some(c) if c != '\n' => {
                            self.bump();
                            source.push('\\');
                            source.push(c);
                        }
                        _ => source.push('\\'),
                    }
                }
                Some(c) => {
                    self.bump();
                    source.push(c);
                }
            }
        }
    }

    /// An integer or a decimal, possibly with a leading `-` (U1:BR3.5). A
    /// number run into letters (`5m`) is a mistake.
    fn number(&mut self, start: Span) -> Result<TokenKind, QueryProblem> {
        let first = self.pos;
        if self.peek_char(0) == Some('-') {
            self.bump();
        }
        self.digits();
        if self.peek_char(0) == Some('.') && self.peek_char(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            self.digits();
        }
        let glued = self.peek_char(0).is_some_and(is_word_char);
        while self.peek_char(0).is_some_and(is_word_char) {
            self.bump();
        }
        let length = u32::try_from(self.pos - first).unwrap_or(u32::MAX);
        let invalid = || {
            QueryProblem::syntax(
                keys::INVALID_NUMBER,
                Span::new(start.line, start.column, length),
            )
        };
        if glued {
            return Err(invalid());
        }
        let text: String = self.chars[first..self.pos].iter().collect();
        text.parse::<f64>()
            .map(TokenKind::Number)
            .map_err(|_| invalid())
    }

    fn digits(&mut self) {
        while self.peek_char(0).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
    }

    fn skip_blanks(&mut self) {
        while self.peek_char(0).is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    /// The period just after `bin(`, read as one word up to a blank, `)` or
    /// `|` (261025-insights-stats BR1.8), so that `5m` is not a number run
    /// into letters here; an empty word has the place of what follows.
    fn bin_word(&mut self) -> (String, Span) {
        self.skip_blanks();
        let start = self.here();
        let first = self.pos;
        while self
            .peek_char(0)
            .is_some_and(|c| !c.is_whitespace() && c != ')' && c != '|')
        {
            self.bump();
        }
        let length = u32::try_from(self.pos - first).unwrap_or(u32::MAX);
        (
            self.chars[first..self.pos].iter().collect(),
            Span::new(start.line, start.column, length),
        )
    }

    /// What follows the period of `bin(`: its `)`, taken, or why not.
    fn close_bin(&mut self) -> BinClose {
        self.skip_blanks();
        match self.peek_char(0) {
            Some(')') => {
                self.bump();
                BinClose::Closed
            }
            None | Some('|') => BinClose::Unclosed,
            Some(_) => BinClose::Other,
        }
    }
}

/// What follows the period of `bin(` (BR1.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinClose {
    /// Its `)`.
    Closed,
    /// The end of the text or of the command.
    Unclosed,
    /// Something else.
    Other,
}

/// The period `word` of `bin(`, written at `span` (BR1.5, BR1.8): a whole
/// number from 1 to [`MAX_BIN_COUNT`] and one of `s`, `m`, `h`, `d`; the
/// console's other units are Unsupported, anything else a mistake.
fn bin_period_of(word: &str, span: Span) -> Result<BinPeriod, QueryProblem> {
    let invalid = || QueryProblem::syntax(keys::INVALID_BIN_PERIOD, span).with_word(word);
    let digits_end = word
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(word.len());
    let (digits, unit) = word.split_at(digits_end);
    if digits.is_empty() || unit.is_empty() {
        return Err(invalid());
    }
    let unit = match unit {
        "s" => BinUnit::Seconds,
        "m" => BinUnit::Minutes,
        "h" => BinUnit::Hours,
        "d" => BinUnit::Days,
        later if LATER_BIN_UNITS.contains(&later) => {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_BIN_UNIT,
                word,
                span,
            ));
        }
        _ => return Err(invalid()),
    };
    let count = digits
        .parse::<u64>()
        .ok()
        .filter(|count| (1..=MAX_BIN_COUNT).contains(count))
        .and_then(|count| u32::try_from(count).ok())
        .ok_or_else(invalid)?;
    Ok(BinPeriod { count, unit })
}

/// Records the column `name` written at `span`, unless a column of that
/// name was written before (BR1.4).
fn push_column(names: &mut Vec<String>, name: &str, span: Span) -> Result<(), QueryProblem> {
    if names.iter().any(|seen| seen == name) {
        return Err(QueryProblem::syntax(keys::DUPLICATE_COLUMN, span).with_word(name));
    }
    names.push(name.to_string());
    Ok(())
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '@'
}

/// Characters that glue to a number and make it a mistake (`5m`).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '@'
}

/// Characters of a bare part of a name: letters, digits, `_`, `-` (U2:BR1.1)
/// and `@` (the fixed fields).
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '@'
}

/// Whether `word` is a reserved keyword of the catalog, never a field name
/// (U1:BR3.3, U2:BR4.1).
fn is_keyword(word: &str) -> bool {
    QueryCatalog::is_reserved(word)
}

/// The field a bare word names, with its exact case (U1:BR3.6).
fn field_named(word: &str) -> Option<FieldRef> {
    match word {
        "@timestamp" => Some(FieldRef::Timestamp),
        "@message" => Some(FieldRef::Message),
        "@logStream" => Some(FieldRef::LogStream),
        "@log" => Some(FieldRef::Log),
        _ => None,
    }
}

/// The field a name token names (U2:BR1.1, BR1.3, BR1.4): a bare first part
/// starting with `@` must be one of the fixed fields alone; a bare part of
/// digits only is an array element; anything else is a JSON field.
fn field_of(token: &Token) -> Result<FieldRef, QueryProblem> {
    let TokenKind::Name(parts) = &token.kind else {
        return Err(unexpected(token.clone()));
    };
    let unsupported = |key| QueryProblem::unsupported(key, token.raw.clone(), token.span);
    if let Some(first) = parts.first().filter(|part| !part.quoted) {
        if first.text.starts_with('@') {
            return match parts.as_slice() {
                [only] => {
                    field_named(&only.text).ok_or_else(|| unsupported(keys::UNSUPPORTED_FIELD))
                }
                _ => Err(unsupported(keys::UNSUPPORTED_FIELD)),
            };
        }
    }
    let is_index = |part: &NamePart| !part.quoted && part.text.chars().all(|c| c.is_ascii_digit());
    if parts.iter().any(is_index) {
        return Err(unsupported(keys::UNSUPPORTED_ARRAY_ELEMENT));
    }
    Ok(FieldRef::Json(JsonPath::new(
        parts.iter().map(|part| part.text.clone()).collect(),
        token.raw.clone(),
    )))
}

/// Where a part of the query started, to give it a span once its last word
/// is read (U2:BR10.3).
#[derive(Debug, Clone, Copy)]
struct Start {
    line: u32,
    column: u32,
    offset: usize,
}

impl Start {
    fn of(token: &Token) -> Self {
        Self {
            line: token.span.line,
            column: token.span.column,
            offset: token.start,
        }
    }
}

/// The problem of a word where it cannot stand: an arithmetic operator is
/// Unsupported (U1:BR5.7), a `)` closes nothing (U1:BR3.4), anything else is
/// a writing mistake.
fn unexpected(token: Token) -> QueryProblem {
    match token.kind {
        TokenKind::Arith => {
            QueryProblem::unsupported(keys::UNSUPPORTED_OPERATOR, token.raw, token.span)
        }
        TokenKind::RParen => QueryProblem::syntax(keys::UNMATCHED_PAREN, token.span),
        _ => QueryProblem::syntax(keys::UNEXPECTED_TOKEN, token.span).with_word(token.raw),
    }
}

fn compile(source: &str, span: Span) -> Result<QueryRegex, QueryProblem> {
    QueryRegex::new(source).map_err(|_| QueryProblem::syntax(keys::INVALID_REGEX, span))
}

/// Reads commands and conditions from the words of a [`Lexer`].
struct Parser {
    lexer: Lexer,
    /// A word looked at but not taken yet (always read in `Mode::Normal`).
    peeked: Option<Option<Token>>,
    /// Offset just after the last word taken, in characters.
    last_end: usize,
    /// After `stats`: the columns of its rows, which the names of the
    /// commands after it refer to (U2:BR1.4).
    result_columns: Option<Vec<ColumnRef>>,
}

/// What opened one level of a condition being read
/// (261017-insights-query U3:BR5.1).
#[derive(Debug, Clone, Copy)]
enum Opening {
    /// The whole condition of a `filter`.
    Whole,
    /// `(` at `open`, the start of the parenthesized condition.
    Paren { open: Span, start: Start },
    /// `not (`: the negation starts at the `not`; its `(` is at `open`.
    Not { open: Span, start: Start },
}

/// The parts of one run of `and` or of `or` read so far, with where the
/// first one started.
#[derive(Debug, Default)]
struct Run {
    parts: Vec<Condition>,
    start: Option<Start>,
}

impl Run {
    fn push(&mut self, part: Condition, start: Start) {
        self.start.get_or_insert(start);
        self.parts.push(part);
    }

    /// The run as one condition: its one part, or a list made by `list`
    /// from its first word to the last word taken. The run is empty after.
    fn close(
        &mut self,
        parser: &Parser,
        list: fn(Vec<Condition>) -> ConditionKind,
    ) -> (Condition, Start) {
        let start = self.start.take().unwrap_or(Start {
            line: 1,
            column: 1,
            offset: 0,
        });
        let mut parts = std::mem::take(&mut self.parts);
        let condition = match parts.pop() {
            Some(only) if parts.is_empty() => only,
            Some(last) => {
                parts.push(last);
                Condition::new(list(parts), parser.span_since(start))
            }
            None => Condition::new(list(Vec::new()), parser.span_since(start)),
        };
        (condition, start)
    }
}

/// One open level: the run of `or` of runs of `and` read inside it.
#[derive(Debug)]
struct Level {
    opening: Opening,
    or: Run,
    and: Run,
}

impl Level {
    fn new(opening: Opening) -> Self {
        Self {
            opening,
            or: Run::default(),
            and: Run::default(),
        }
    }
}

/// Opens a level for the parenthesis of `opening`, unless it would be level
/// [`MAX_NESTING`] + 1 (U3:BR5.1): the first level is the whole condition,
/// so the levels open below it are the parentheses.
fn open_level(levels: &mut Vec<Level>, opening: Opening) -> Result<(), QueryProblem> {
    let open = match opening {
        Opening::Paren { open, .. } | Opening::Not { open, .. } => open,
        Opening::Whole => return Ok(()),
    };
    if levels.len() > MAX_NESTING {
        return Err(QueryProblem::syntax(keys::NESTING_TOO_DEEP, open));
    }
    levels.push(Level::new(opening));
    Ok(())
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            lexer: Lexer::new(text),
            peeked: None,
            last_end: 0,
            result_columns: None,
        }
    }

    fn next(&mut self) -> Result<Option<Token>, QueryProblem> {
        self.take(Mode::Normal)
    }

    /// The next word where a value is expected.
    fn next_value(&mut self) -> Result<Option<Token>, QueryProblem> {
        self.take(Mode::Value)
    }

    fn take(&mut self, mode: Mode) -> Result<Option<Token>, QueryProblem> {
        let token = match self.peeked.take() {
            Some(token) => token,
            None => self.lexer.next(mode)?,
        };
        if let Some(token) = &token {
            self.last_end = token.end;
        }
        Ok(token)
    }

    /// Gives `token` back, as if it had only been looked at; `last_end` is
    /// where the word before it ended.
    fn unread(&mut self, token: Token, last_end: usize) {
        self.peeked = Some(Some(token));
        self.last_end = last_end;
    }

    /// From `start` to the end of the last word taken (U2:BR10.3).
    fn span_since(&self, start: Start) -> Span {
        let length = self.last_end.saturating_sub(start.offset);
        Span::new(
            start.line,
            start.column,
            u32::try_from(length).unwrap_or(u32::MAX),
        )
    }

    fn peek(&mut self) -> Result<Option<&Token>, QueryProblem> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next(Mode::Normal)?);
        }
        Ok(self.peeked.as_ref().and_then(Option::as_ref))
    }

    fn peek_is(&mut self, kind: &TokenKind) -> Result<bool, QueryProblem> {
        Ok(self.peek()?.is_some_and(|token| token.kind == *kind))
    }

    fn peek_keyword(&mut self, keyword: &str) -> Result<bool, QueryProblem> {
        Ok(self
            .peek()?
            .is_some_and(|token| token.kind.is_word(keyword)))
    }

    /// Whether the end of the text or a `|` comes next.
    fn at_command_end(&mut self) -> Result<bool, QueryProblem> {
        Ok(self.peek()?.is_none() || self.peek_is(&TokenKind::Pipe)?)
    }

    /// Where the text ended.
    fn end(&self) -> Span {
        self.lexer.here()
    }

    /// One command (U2:BR2.1); after `stats`, `fields` and `stats` are
    /// Unsupported (261025-insights-stats U2:BR1.5). Which names are
    /// commands, and which of them are run, is the catalog's (BR4.1).
    fn command(&mut self, token: Token) -> Result<Command, QueryProblem> {
        let Some(word) = token.kind.bare().map(str::to_string) else {
            if matches!(token.kind, TokenKind::Name(_)) {
                return Err(
                    QueryProblem::syntax(keys::UNKNOWN_COMMAND, token.span).with_word(token.raw)
                );
            }
            return Err(unexpected(token));
        };
        let span = token.span;
        let Some(listed) = QueryCatalog::command(&word) else {
            return Err(QueryProblem::syntax(keys::UNKNOWN_COMMAND, span).with_word(word));
        };
        if !listed.supported {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_COMMAND,
                word,
                span,
            ));
        }
        let after_stats = self.result_columns.is_some();
        let kind = match listed.name {
            "fields" if after_stats => {
                return Err(QueryProblem::unsupported(
                    keys::FIELDS_AFTER_STATS,
                    word,
                    span,
                ));
            }
            "stats" if after_stats => {
                return Err(QueryProblem::unsupported(keys::SECOND_STATS, word, span));
            }
            "filter" => {
                if self.at_command_end()? {
                    return Err(QueryProblem::syntax(keys::MISSING_CONDITION, span));
                }
                CommandKind::Filter(self.condition()?)
            }
            "fields" => CommandKind::Fields(self.fields(&word, span)?),
            "sort" => CommandKind::Sort(self.sort_keys(&word, span)?),
            "limit" => CommandKind::Limit(self.limit(span)?),
            "stats" => CommandKind::Stats(self.stats(span)?),
            // A supported command of the catalog that is not read here: none
            // today; the catalog test keeps the two in step.
            _ => {
                return Err(QueryProblem::unsupported(
                    keys::UNSUPPORTED_COMMAND,
                    word,
                    span,
                ));
            }
        };
        Ok(Command { kind, span })
    }

    /// The fields of `fields` (U2:BR3.2): at least one, separated by
    /// commas; `as` is Unsupported (U2:BR2.1).
    fn fields(&mut self, command: &str, span: Span) -> Result<Vec<FieldRef>, QueryProblem> {
        if self.at_command_end()? {
            return Err(QueryProblem::syntax(keys::MISSING_FIELDS, span).with_word(command));
        }
        let mut fields = Vec::new();
        loop {
            fields.push(self.output_field()?);
            if self.at_command_end()? {
                return Ok(fields);
            }
            match self.next()? {
                None => return Ok(fields),
                Some(token) if token.kind == TokenKind::Comma => {}
                Some(token) if token.kind.is_word("as") => {
                    return Err(QueryProblem::unsupported(
                        keys::UNSUPPORTED_ALIAS,
                        token.raw,
                        token.span,
                    ));
                }
                Some(token) => return Err(unexpected(token)),
            }
        }
    }

    /// The fields and directions of `sort` (U2:BR4.1): at least one field,
    /// each followed by `asc`, `desc` or nothing (ascending).
    fn sort_keys(&mut self, command: &str, span: Span) -> Result<Vec<SortKey>, QueryProblem> {
        if self.at_command_end()? {
            return Err(QueryProblem::syntax(keys::MISSING_FIELDS, span).with_word(command));
        }
        let mut sort_keys = Vec::new();
        loop {
            let field = self.output_field()?;
            let direction = self.sort_direction()?;
            sort_keys.push(SortKey { field, direction });
            if self.at_command_end()? {
                return Ok(sort_keys);
            }
            match self.next()? {
                None => return Ok(sort_keys),
                Some(token) if token.kind == TokenKind::Comma => {}
                Some(token) => return Err(unexpected(token)),
            }
        }
    }

    /// `asc`, `desc` (any case) or nothing, which is ascending (U2:BR4.1,
    /// Q1b 1a); another word is a writing mistake.
    fn sort_direction(&mut self) -> Result<SortDirection, QueryProblem> {
        let is_word = self
            .peek()?
            .is_some_and(|token| token.kind.bare().is_some());
        if !is_word {
            return Ok(SortDirection::Asc);
        }
        let Some(token) = self.next()? else {
            return Ok(SortDirection::Asc);
        };
        if token.kind.is_word("asc") {
            Ok(SortDirection::Asc)
        } else if token.kind.is_word("desc") {
            Ok(SortDirection::Desc)
        } else {
            Err(QueryProblem::syntax(keys::INVALID_SORT_DIRECTION, token.span).with_word(token.raw))
        }
    }

    /// The number of `limit` (U2:BR5.1): a whole number from 0 to
    /// [`MAX_LIMIT`]; `limit any` is Unsupported (U2:BR2.1).
    fn limit(&mut self, span: Span) -> Result<u32, QueryProblem> {
        let before = self.last_end;
        let token = match self.next_value()? {
            None => return Err(QueryProblem::syntax(keys::MISSING_LIMIT, span)),
            Some(token) if token.kind == TokenKind::Pipe => {
                self.unread(token, before);
                return Err(QueryProblem::syntax(keys::MISSING_LIMIT, span));
            }
            Some(token) => token,
        };
        if token.kind.is_word("any") {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_LIMIT_ANY,
                token.raw,
                token.span,
            ));
        }
        let invalid =
            || QueryProblem::syntax(keys::INVALID_LIMIT, token.span).with_word(token.raw.clone());
        let TokenKind::Number(number) = token.kind else {
            return Err(invalid());
        };
        if token.raw.contains('.') || number < 0.0 {
            return Err(invalid());
        }
        if number > f64::from(MAX_LIMIT) {
            return Err(
                QueryProblem::syntax(keys::LIMIT_TOO_LARGE, token.span).with_word(token.raw)
            );
        }
        // A whole number from 0 to MAX_LIMIT: the conversion is exact.
        u32::try_from(number as u64).map_err(|_| invalid())
    }

    /// The aggregates and `by` keys of `stats` (261025-insights-stats
    /// BR1.1-BR1.9): at least one aggregate, separated by commas, then
    /// optionally `by` and at least one key.
    fn stats(&mut self, span: Span) -> Result<StatsCommand, QueryProblem> {
        if self.at_command_end()? || self.peek_keyword("by")? {
            return Err(QueryProblem::syntax(keys::MISSING_AGGREGATE, span));
        }
        let mut names = Vec::new();
        let mut aggregates = Vec::new();
        loop {
            let (aggregate, name_span) = self.aggregate()?;
            push_column(&mut names, &aggregate.column_name, name_span)?;
            aggregates.push(aggregate);
            if self.at_command_end()? {
                break;
            }
            match self.next()? {
                None => break,
                Some(token) if token.kind == TokenKind::Comma => {
                    if self.at_command_end()? || self.peek_keyword("by")? {
                        return Err(QueryProblem::syntax(keys::TRAILING_COMMA, token.span));
                    }
                }
                Some(token) if token.kind.is_word("by") => {
                    let group_keys = self.group_keys(token.span, &mut names)?;
                    return Ok(StatsCommand {
                        aggregates,
                        group_keys,
                        after: Vec::new(),
                    });
                }
                Some(token) => return Err(unexpected(token)),
            }
        }
        Ok(StatsCommand {
            aggregates,
            group_keys: Vec::new(),
            after: Vec::new(),
        })
    }

    /// The keys after `by`, written at `by` (BR1.1, BR1.7).
    fn group_keys(
        &mut self,
        by: Span,
        names: &mut Vec<String>,
    ) -> Result<Vec<GroupKey>, QueryProblem> {
        if self.at_command_end()? {
            return Err(QueryProblem::syntax(keys::MISSING_GROUP_KEY, by));
        }
        let mut group_keys = Vec::new();
        loop {
            let (key, name_span) = self.group_key()?;
            push_column(names, &key.column_name, name_span)?;
            group_keys.push(key);
            if self.at_command_end()? {
                return Ok(group_keys);
            }
            match self.next()? {
                None => return Ok(group_keys),
                Some(token) if token.kind == TokenKind::Comma => {
                    if self.at_command_end()? {
                        return Err(QueryProblem::syntax(keys::TRAILING_COMMA, token.span));
                    }
                }
                Some(token) => return Err(unexpected(token)),
            }
        }
    }

    /// One aggregate, `function(argument)[ as name]` (BR1.2-BR1.4, BR1.9),
    /// with where its column name was written.
    fn aggregate(&mut self) -> Result<(AggregateSpec, Span), QueryProblem> {
        let Some(token) = self.next()? else {
            return Err(QueryProblem::syntax(keys::EXPECTED_AGGREGATE, self.end()));
        };
        let Some(name) = token.kind.bare().map(str::to_string) else {
            if matches!(token.kind, TokenKind::Name(_)) {
                return Err(
                    QueryProblem::syntax(keys::EXPECTED_AGGREGATE, token.span).with_word(token.raw)
                );
            }
            return Err(unexpected(token));
        };
        if !self.peek_is(&TokenKind::LParen)? {
            return Err(
                QueryProblem::syntax(keys::EXPECTED_AGGREGATE, token.span).with_word(token.raw)
            );
        }
        // The functions are the catalog's, written as the documentation
        // writes them (BR1.1, U2:BR1.1, BR4.1).
        let listed = QueryCatalog::aggregate_function(&name)
            .and_then(|listed| AggregateFunction::from_name(listed.name).map(|f| (listed, f)));
        let function = match listed {
            Some((listed, function)) if listed.supported => function,
            Some(_) => {
                return Err(QueryProblem::unsupported(
                    keys::UNSUPPORTED_AGGREGATE,
                    name,
                    token.span,
                ));
            }
            None => {
                return Err(QueryProblem::unsupported(
                    keys::UNSUPPORTED_FUNCTION,
                    name,
                    token.span,
                ));
            }
        };
        let Some(open) = self.next()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        let (argument, percentile) = if function == AggregateFunction::Pct {
            let (field, percentile) = self.pct_arguments(open.span)?;
            (field, Some(percentile))
        } else {
            (self.aggregate_argument(function, open.span)?, None)
        };
        let written = self.source_text(token.start, self.last_end);
        let written_span = self.span_since(Start::of(&token));
        let (column_name, name_span) = self.alias()?.unwrap_or((written, written_span));
        Ok((
            AggregateSpec {
                function,
                argument,
                percentile,
                column_name,
            },
            name_span,
        ))
    }

    /// The field, `,`, percentile and `)` of `pct(`, the `(` being at
    /// `open` (U2:BR1.2): a percentile from 0 to 100, kept as written.
    fn pct_arguments(&mut self, open: Span) -> Result<(AggregateArgument, Percentile), QueryProblem> {
        let unclosed = || QueryProblem::syntax(keys::UNCLOSED_PAREN, open);
        let argument = match self.aggregate_field(AggregateFunction::Pct, open)? {
            Some(argument) => argument,
            None => return Err(QueryProblem::syntax(keys::ARGUMENT_COUNT, open)),
        };
        match self.next()? {
            Some(comma) if comma.kind == TokenKind::Comma => {}
            Some(close) if close.kind == TokenKind::RParen => {
                return Err(QueryProblem::syntax(keys::ARGUMENT_COUNT, open));
            }
            Some(other) if other.kind != TokenKind::Pipe => return Err(unexpected(other)),
            _ => return Err(unclosed()),
        }
        let token = match self.next_value()? {
            Some(token) if token.kind == TokenKind::Pipe => return Err(unclosed()),
            Some(token) if token.kind == TokenKind::RParen => {
                return Err(QueryProblem::syntax(keys::ARGUMENT_COUNT, open));
            }
            Some(token) => token,
            None => return Err(unclosed()),
        };
        let percentile = match token.kind {
            TokenKind::Number(_) => Percentile::parse(&token.raw).ok_or_else(|| {
                QueryProblem::syntax(keys::PCT_OUT_OF_RANGE, token.span)
                    .with_word(token.raw.clone())
            })?,
            _ => {
                return Err(
                    QueryProblem::syntax(keys::EXPECTED_VALUE, token.span).with_word(token.raw)
                );
            }
        };
        self.close_arguments(open)?;
        Ok((argument, percentile))
    }

    /// The `)` after the last argument of a function opened at `open`; a
    /// `,` means one argument too many (BR1.7).
    fn close_arguments(&mut self, open: Span) -> Result<(), QueryProblem> {
        match self.next()? {
            Some(close) if close.kind == TokenKind::RParen => Ok(()),
            Some(comma) if comma.kind == TokenKind::Comma => {
                Err(QueryProblem::syntax(keys::ARGUMENT_COUNT, open))
            }
            Some(other) if other.kind != TokenKind::Pipe => Err(unexpected(other)),
            _ => Err(QueryProblem::syntax(keys::UNCLOSED_PAREN, open)),
        }
    }

    /// The argument of an aggregate and its `)`, the `(` being at `open`
    /// (BR1.3, BR1.7): `*` for `count` only (and `count()`, which counts the
    /// same), else one field.
    fn aggregate_argument(
        &mut self,
        function: AggregateFunction,
        open: Span,
    ) -> Result<AggregateArgument, QueryProblem> {
        match self.aggregate_field(function, open)? {
            // `count()`: its `)` is taken.
            None => Ok(AggregateArgument::All),
            Some(argument) => {
                self.close_arguments(open)?;
                Ok(argument)
            }
        }
    }

    /// The first argument of an aggregate opened at `open`: `*` (for
    /// `count` only) or one field; `None` when `count()` closes at once,
    /// its `)` taken (BR1.3).
    fn aggregate_field(
        &mut self,
        function: AggregateFunction,
        open: Span,
    ) -> Result<Option<AggregateArgument>, QueryProblem> {
        let unclosed = || QueryProblem::syntax(keys::UNCLOSED_PAREN, open);
        let Some(token) = self.next()? else {
            return Err(unclosed());
        };
        match &token.kind {
            TokenKind::RParen if function == AggregateFunction::Count => Ok(None),
            TokenKind::RParen => Err(QueryProblem::syntax(keys::ARGUMENT_COUNT, open)),
            TokenKind::Pipe => Err(unclosed()),
            TokenKind::Arith if token.raw == "*" => {
                if function != AggregateFunction::Count {
                    return Err(QueryProblem::syntax(keys::AGGREGATE_STAR, token.span));
                }
                Ok(Some(AggregateArgument::All))
            }
            TokenKind::Name(_) if token.kind.bare().is_some_and(is_keyword) => {
                Err(unexpected(token))
            }
            TokenKind::Name(_) => Ok(Some(AggregateArgument::Field(self.operand(token)?))),
            TokenKind::Arith | TokenKind::LParen => Err(unexpected(token)),
            _ => Err(QueryProblem::syntax(keys::EXPECTED_FIELD, token.span).with_word(token.raw)),
        }
    }

    /// One `by` key, a field or `bin(period)`, `[ as name]` (BR1.3, BR1.4,
    /// BR1.8), with where its column name was written.
    fn group_key(&mut self) -> Result<(GroupKey, Span), QueryProblem> {
        let Some(token) = self.next()? else {
            return Err(QueryProblem::syntax(keys::EXPECTED_FIELD, self.end()));
        };
        match &token.kind {
            TokenKind::Name(_) if token.kind.bare().is_some_and(is_keyword) => {
                return Err(unexpected(token));
            }
            TokenKind::Name(_) => {}
            TokenKind::Arith | TokenKind::RParen => return Err(unexpected(token)),
            _ => {
                return Err(
                    QueryProblem::syntax(keys::EXPECTED_FIELD, token.span).with_word(token.raw)
                );
            }
        }
        let start = Start::of(&token);
        let function = match token.kind.bare() {
            Some(word) if self.peek_is(&TokenKind::LParen)? => Some(word.to_string()),
            _ => None,
        };
        let kind = match function {
            Some(word) if word == BIN => GroupKeyKind::Bin(self.bin_period()?),
            // toMillis(@timestamp) and any other function is no key (Q6 A).
            Some(word) => {
                return Err(QueryProblem::unsupported(
                    keys::UNSUPPORTED_FUNCTION,
                    word,
                    token.span,
                ));
            }
            None => {
                let field = field_of(&token)?;
                if self.peek_is(&TokenKind::LBracket)? {
                    return Err(QueryProblem::unsupported(
                        keys::UNSUPPORTED_ARRAY_ELEMENT,
                        token.raw,
                        token.span,
                    ));
                }
                GroupKeyKind::Field(field)
            }
        };
        let written = self.source_text(token.start, self.last_end);
        let written_span = self.span_since(start);
        let (column_name, name_span) = self.alias()?.unwrap_or((written, written_span));
        Ok((GroupKey { kind, column_name }, name_span))
    }

    /// The period of `bin(` and its `)`; the `(` is the next word (BR1.5,
    /// BR1.8). The period is read by the lexer as one word.
    fn bin_period(&mut self) -> Result<BinPeriod, QueryProblem> {
        let Some(open) = self.next()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        let (word, span) = self.lexer.bin_word();
        let close = self.lexer.close_bin();
        self.last_end = self.lexer.pos;
        match close {
            BinClose::Closed => bin_period_of(&word, span),
            BinClose::Unclosed => Err(QueryProblem::syntax(keys::UNCLOSED_PAREN, open.span)),
            BinClose::Other => {
                Err(QueryProblem::syntax(keys::INVALID_BIN_PERIOD, span).with_word(word))
            }
        }
    }

    /// `as name` after an aggregate or a key, when written (BR1.4): the name
    /// of one part, bare or in backquotes, and where it was written.
    fn alias(&mut self) -> Result<Option<(String, Span)>, QueryProblem> {
        if !self.peek_keyword("as")? {
            return Ok(None);
        }
        let before = self.last_end;
        let Some(word) = self.next()? else {
            return Ok(None);
        };
        let expected = || QueryProblem::syntax(keys::EXPECTED_ALIAS, word.span);
        let token = match self.next()? {
            None => return Err(expected()),
            Some(token) if token.kind == TokenKind::Pipe => {
                self.unread(token, before);
                return Err(expected());
            }
            Some(token) => token,
        };
        let is_word = |word: &str| is_keyword(word) || word.eq_ignore_ascii_case("by");
        match &token.kind {
            TokenKind::Name(parts)
                if parts.len() == 1 && !token.kind.bare().is_some_and(is_word) =>
            {
                Ok(parts.first().map(|part| (part.text.clone(), token.span)))
            }
            _ => Err(QueryProblem::syntax(keys::EXPECTED_ALIAS, token.span).with_word(token.raw)),
        }
    }

    /// The text from character `start` to `end` as a column name (BR1.9):
    /// notes left out, every run of blanks and line breaks made one blank,
    /// none at either end; quoted parts as written.
    fn source_text(&self, start: usize, end: usize) -> String {
        let chars = &self.lexer.chars;
        let end = end.min(chars.len());
        let mut text = String::new();
        let mut quote: Option<char> = None;
        let mut blank = false;
        let mut index = start.min(end);
        while index < end {
            let c = chars[index];
            index += 1;
            if let Some(open) = quote {
                text.push(c);
                if c == '\\' && open != '`' && index < end {
                    text.push(chars[index]);
                    index += 1;
                } else if c == open {
                    quote = None;
                }
            } else if c == '#' {
                while index < end && chars[index] != '\n' {
                    index += 1;
                }
                blank = true;
            } else if c.is_whitespace() {
                blank = true;
            } else {
                if blank && !text.is_empty() {
                    text.push(' ');
                }
                blank = false;
                if matches!(c, '"' | '\'' | '`') {
                    quote = Some(c);
                }
                text.push(c);
            }
        }
        text
    }

    /// One field of `fields` or `sort`: a fixed field, a JSON field or
    /// `toMillis(@timestamp)` (U2:BR1.1).
    fn output_field(&mut self) -> Result<FieldRef, QueryProblem> {
        match self.next()? {
            None => Err(QueryProblem::syntax(keys::EXPECTED_FIELD, self.end())),
            Some(token) if matches!(token.kind, TokenKind::Name(_)) => {
                if token.kind.bare().is_some_and(is_keyword) {
                    return Err(unexpected(token));
                }
                self.operand(token)
            }
            Some(token) if matches!(token.kind, TokenKind::Arith | TokenKind::RParen) => {
                Err(unexpected(token))
            }
            Some(token) => {
                Err(QueryProblem::syntax(keys::EXPECTED_FIELD, token.span).with_word(token.raw))
            }
        }
    }

    /// A whole condition: `or` of `and`s of comparisons, `not (…)` and
    /// parenthesized conditions (U2:BR2.4, BR2.5), read in one loop
    /// (261017-insights-query U3:BR5.1, functional-design review R-02).
    ///
    /// Every open parenthesis is one [`Level`] of an explicit stack instead
    /// of a call, so how deep the parentheses go never uses up the thread's
    /// stack. A run of `and` or of `or` is one list, so its length adds no
    /// depth. The parenthesis that would open level [`MAX_NESTING`] + 1 is a
    /// writing mistake at its place, found before anything deeper is read.
    /// The problems found and their order are those of the recursive reader
    /// of U2.
    fn condition(&mut self) -> Result<Condition, QueryProblem> {
        let mut levels = vec![Level::new(Opening::Whole)];
        loop {
            let (mut part, mut start) = self.next_part(&mut levels)?;
            // Joins the part to its level, then closes what ends after it.
            loop {
                let Some(level) = levels.last_mut() else {
                    return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
                };
                level.and.push(part, start);
                if self.peek_keyword("and")? {
                    self.next()?;
                    break;
                }
                let (joined, joined_start) = level.and.close(self, ConditionKind::And);
                level.or.push(joined, joined_start);
                if self.peek_keyword("or")? {
                    self.next()?;
                    break;
                }
                let (whole, _) = level.or.close(self, ConditionKind::Or);
                let opening = level.opening;
                match opening {
                    Opening::Whole => return Ok(whole),
                    Opening::Paren { open, start: paren } => {
                        self.close_paren(open)?;
                        levels.pop();
                        // A parenthesized condition keeps its own place; its
                        // start is the `(`.
                        (part, start) = (whole, paren);
                    }
                    Opening::Not { open, start: not } => {
                        self.close_paren(open)?;
                        levels.pop();
                        let negation = ConditionKind::Not(Box::new(whole));
                        (part, start) = (Condition::new(negation, self.span_since(not)), not);
                    }
                }
            }
        }
    }

    /// Reads up to the next comparison of a field, opening a level for every
    /// `(` and `not (` on the way. At the start of a condition `not` must be
    /// followed by `(` (U2:BR2.4, R-07).
    fn next_part(&mut self, levels: &mut Vec<Level>) -> Result<(Condition, Start), QueryProblem> {
        loop {
            if self.peek_keyword("not")? {
                let Some(not) = self.next()? else {
                    return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
                };
                if !self.peek_is(&TokenKind::LParen)? {
                    return Err(
                        QueryProblem::syntax(keys::NOT_NEEDS_PAREN, not.span).with_word(not.raw)
                    );
                }
                let Some(open) = self.next()? else {
                    return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
                };
                open_level(
                    levels,
                    Opening::Not {
                        open: open.span,
                        start: Start::of(&not),
                    },
                )?;
                continue;
            }
            let Some(token) = self.next()? else {
                return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
            };
            let start = Start::of(&token);
            match &token.kind {
                TokenKind::LParen => {
                    let open = token.span;
                    open_level(levels, Opening::Paren { open, start })?;
                }
                TokenKind::Name(_) if token.kind.bare().is_some_and(is_keyword) => {
                    return Err(unexpected(token));
                }
                TokenKind::Name(_) => {
                    let field = self.operand(token)?;
                    let kind = self.predicate(field)?;
                    return Ok((Condition::new(kind, self.span_since(start)), start));
                }
                _ => return Err(unexpected(token)),
            }
        }
    }

    /// The `)` of the parenthesis opened at `open`.
    fn close_paren(&mut self, open: Span) -> Result<(), QueryProblem> {
        match self.next()? {
            Some(close) if close.kind == TokenKind::RParen => Ok(()),
            Some(close) if close.kind != TokenKind::Pipe => Err(unexpected(close)),
            _ => Err(QueryProblem::syntax(keys::UNCLOSED_PAREN, open)),
        }
    }

    /// The field a name reads: a fixed field, a JSON field or
    /// `toMillis(@timestamp)` (U1:BR3.6, BR3.7, U2:BR1.1). A `[` after it is
    /// an array element (U2:BR1.4).
    fn operand(&mut self, token: Token) -> Result<FieldRef, QueryProblem> {
        if self.result_columns.is_some() {
            return self.result_column(token);
        }
        if let Some(word) = token.kind.bare() {
            if self.peek_is(&TokenKind::LParen)? {
                return self.function(word.to_string(), token.span);
            }
        }
        let field = field_of(&token)?;
        if self.peek_is(&TokenKind::LBracket)? {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_ARRAY_ELEMENT,
                token.raw,
                token.span,
            ));
        }
        Ok(field)
    }

    /// The result column a name refers to after `stats` (U2:BR1.4 (b)): the
    /// parts joined by `.`, backquotes left out, the same as a column name
    /// in case too. A name followed by `(` is no column: `count(*)` is
    /// written in backquotes.
    fn result_column(&mut self, token: Token) -> Result<FieldRef, QueryProblem> {
        let unknown = |word: &str| {
            QueryProblem::syntax(keys::UNKNOWN_RESULT_COLUMN, token.span).with_word(word)
        };
        let TokenKind::Name(parts) = &token.kind else {
            return Err(unexpected(token));
        };
        if self.peek_is(&TokenKind::LParen)? {
            return Err(unknown(&token.raw));
        }
        let name = parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join(".");
        self.result_columns
            .iter()
            .flatten()
            .find(|column| column.name == name)
            .map(|column| FieldRef::Column(column.clone()))
            .ok_or_else(|| unknown(&token.raw))
    }

    /// `name(`: only `toMillis(@timestamp)` is read (U1:BR3.7).
    fn function(&mut self, name: String, span: Span) -> Result<FieldRef, QueryProblem> {
        if name != TO_MILLIS {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_FUNCTION,
                name,
                span,
            ));
        }
        let open = self.next()?;
        let argument = self.next()?;
        let close = self.next()?;
        let is_timestamp = argument
            .as_ref()
            .is_some_and(|token| token.kind.bare() == Some("@timestamp"));
        match close {
            Some(close) if close.kind == TokenKind::RParen && is_timestamp => {
                Ok(FieldRef::TimestampMillis)
            }
            None if is_timestamp => Err(QueryProblem::syntax(
                keys::UNCLOSED_PAREN,
                open.map_or(span, |open| open.span),
            )),
            _ => Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_TO_MILLIS_ARGUMENT,
                name,
                span,
            )),
        }
    }

    /// The operator and value after `field`.
    fn predicate(&mut self, field: FieldRef) -> Result<ConditionKind, QueryProblem> {
        let Some(operator) = self.next()? else {
            return Err(QueryProblem::syntax(keys::EXPECTED_OPERATOR, self.end()));
        };
        match &operator.kind {
            TokenKind::Compare(op) => {
                let op = *op;
                let Some(token) = self.next_value()? else {
                    return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
                };
                if !matches!(token.kind, TokenKind::Name(_)) {
                    let value = self.scalar(&field, token)?;
                    return Ok(ConditionKind::Compare { field, op, value });
                }
                // A name without quotes on the right is another field, read
                // as the left one is (Loop-back 2, p05).
                if token.kind.bare().is_some_and(is_keyword) {
                    return Err(unexpected(token));
                }
                let other = self.operand(token)?;
                Ok(ConditionKind::CompareFields { field, op, other })
            }
            TokenKind::Match => {
                if field.is_timestamp() {
                    return Err(timestamp_operator(operator));
                }
                Ok(ConditionKind::Match {
                    field,
                    regex: self.match_value()?,
                })
            }
            kind if kind.is_word("like") => self.like(field, operator, false),
            kind if kind.is_word("in") => self.list(field, false),
            // `not like` and `not in` after a field (U2:BR2.4, R-07).
            kind if kind.is_word("not") => {
                match self.next()? {
                    Some(next) if next.kind.is_word("like") => self.like(field, next, true),
                    Some(next) if next.kind.is_word("in") => self.list(field, true),
                    Some(next) => Err(QueryProblem::syntax(keys::EXPECTED_OPERATOR, next.span)
                        .with_word(next.raw)),
                    None => Err(QueryProblem::syntax(keys::EXPECTED_OPERATOR, self.end())),
                }
            }
            TokenKind::Arith => Err(unexpected(operator)),
            _ => Err(QueryProblem::syntax(keys::EXPECTED_OPERATOR, operator.span)
                .with_word(operator.raw)),
        }
    }

    /// One string or number value of a comparison or a list (U1:BR5.1); a
    /// text compared with the timestamp is a date text (U1:BR5.9).
    fn scalar(&mut self, field: &FieldRef, token: Token) -> Result<Literal, QueryProblem> {
        match token.kind {
            TokenKind::Number(number) => Ok(Literal::Number(number)),
            TokenKind::Text(text) if field.is_timestamp() => {
                if !is_date_text(&text) {
                    return Err(QueryProblem::syntax(keys::INVALID_DATE_TIME, token.span));
                }
                Ok(Literal::DateText {
                    text,
                    span: token.span,
                })
            }
            TokenKind::Text(text) => Ok(Literal::Text(text)),
            TokenKind::Regex(_) => Err(QueryProblem::syntax(keys::REGEX_NOT_ALLOWED, token.span)),
            TokenKind::Name(_) => Err(self.word_value(token)?),
            _ => Err(QueryProblem::syntax(keys::EXPECTED_VALUE, token.span).with_word(token.raw)),
        }
    }

    /// A name where only a value is taken (in a `[` list, after `like` or
    /// `=~`): a function call or another field, neither of which is run
    /// there. A comparison takes another field (Loop-back 2, p05).
    fn word_value(&mut self, token: Token) -> Result<QueryProblem, QueryProblem> {
        let key = if self.peek_is(&TokenKind::LParen)? {
            keys::UNSUPPORTED_FUNCTION
        } else {
            keys::UNSUPPORTED_FIELD_COMPARISON
        };
        Ok(QueryProblem::unsupported(key, token.raw, token.span))
    }

    /// The regular expression after `=~` (U1:BR5.4).
    fn match_value(&mut self) -> Result<QueryRegex, QueryProblem> {
        let Some(token) = self.next_value()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        match token.kind {
            TokenKind::Regex(source) => compile(&source, token.span),
            TokenKind::Name(_) => Err(self.word_value(token)?),
            _ => Err(QueryProblem::syntax(keys::MATCH_NEEDS_REGEX, token.span)),
        }
    }

    /// `like` / `not like` (U1:BR5.3, BR5.4, BR5.6).
    fn like(
        &mut self,
        field: FieldRef,
        operator: Token,
        negated: bool,
    ) -> Result<ConditionKind, QueryProblem> {
        if field.is_timestamp() {
            return Err(timestamp_operator(operator));
        }
        let Some(token) = self.next_value()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        let pattern = match token.kind {
            TokenKind::Text(text) => LikePattern::Text(text),
            TokenKind::Regex(source) => LikePattern::Regex(compile(&source, token.span)?),
            TokenKind::Name(_) => return Err(self.word_value(token)?),
            _ => return Err(QueryProblem::syntax(keys::LIKE_NEEDS_TEXT, token.span)),
        };
        Ok(ConditionKind::Like {
            field,
            pattern,
            negated,
        })
    }

    /// `in [values]` / `not in [values]` (U1:BR3.5, BR5.5).
    fn list(&mut self, field: FieldRef, negated: bool) -> Result<ConditionKind, QueryProblem> {
        let open = match self.next()? {
            Some(open) if open.kind == TokenKind::LBracket => open,
            Some(other) => return Err(QueryProblem::syntax(keys::EXPECTED_LIST, other.span)),
            None => return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end())),
        };
        let unclosed = || QueryProblem::syntax(keys::UNCLOSED_LIST, open.span);
        let mut values = Vec::new();
        loop {
            let value = match self.next_value()? {
                None => return Err(unclosed()),
                Some(token) if token.kind == TokenKind::RBracket && values.is_empty() => {
                    return Err(QueryProblem::syntax(keys::EMPTY_LIST, open.span));
                }
                Some(token) => token,
            };
            values.push(self.scalar(&field, value)?);
            match self.next()? {
                None => return Err(unclosed()),
                Some(token) if token.kind == TokenKind::Comma => {}
                Some(token) if token.kind == TokenKind::RBracket => break,
                Some(token) if token.kind == TokenKind::Pipe => return Err(unclosed()),
                Some(token) => return Err(unexpected(token)),
            }
        }
        Ok(ConditionKind::In {
            field,
            values,
            negated,
        })
    }
}

/// `like` or `=~` on the timestamp (U1:BR5.6).
fn timestamp_operator(operator: Token) -> QueryProblem {
    QueryProblem::unsupported(
        keys::UNSUPPORTED_TIMESTAMP_OPERATOR,
        operator.raw,
        operator.span,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::plan::{
        CompareOp, Condition, ConditionKind, FieldRef, JsonPath, LikePattern, Literal, ProblemKind,
        QueryRegex, SortDirection, SortKey,
    };

    /// The place of a condition built in a test; [`bare`] gives every part
    /// of a read condition this place, so trees compare by their shape.
    const NO_SPAN: Span = Span {
        line: 0,
        column: 0,
        length: 0,
    };

    fn plan(text: &str) -> QueryPlan {
        match parse_query(text) {
            Ok(plan) => plan,
            Err(problem) => panic!("{text:?} was not read: {problem:?}"),
        }
    }

    /// `condition` with [`NO_SPAN`] everywhere.
    fn bare(condition: &Condition) -> Condition {
        let all = |parts: &[Condition]| parts.iter().map(bare).collect();
        let kind = match &condition.kind {
            ConditionKind::And(parts) => ConditionKind::And(all(parts)),
            ConditionKind::Or(parts) => ConditionKind::Or(all(parts)),
            ConditionKind::Not(inner) => ConditionKind::Not(Box::new(bare(inner))),
            other => other.clone(),
        };
        Condition::new(kind, NO_SPAN)
    }

    /// The conditions of the `filter` commands of `text`, bare.
    fn filters(text: &str) -> Vec<Condition> {
        plan(text).filters().map(bare).collect()
    }

    fn only(text: &str) -> Condition {
        let mut filters = filters(text);
        assert_eq!(filters.len(), 1, "{text:?}");
        filters.remove(0)
    }

    fn kinds(plan: &QueryPlan) -> Vec<&'static str> {
        plan.commands
            .iter()
            .map(|command| match command.kind {
                CommandKind::Filter(_) => "filter",
                CommandKind::Fields(_) => "fields",
                CommandKind::Sort(_) => "sort",
                CommandKind::Limit(_) => "limit",
                CommandKind::Stats(_) => "stats",
            })
            .collect()
    }

    fn json(keys: &[&str], text: &str) -> FieldRef {
        FieldRef::Json(JsonPath::new(
            keys.iter().map(|key| (*key).to_string()).collect(),
            text,
        ))
    }

    fn condition(kind: ConditionKind) -> Condition {
        Condition::new(kind, NO_SPAN)
    }

    fn problem(text: &str) -> QueryProblem {
        match parse_query(text) {
            Ok(plan) => panic!("{text:?} was read: {plan:?}"),
            Err(problem) => problem,
        }
    }

    fn at(line: u32, column: u32, length: u32) -> Span {
        Span::new(line, column, length)
    }

    fn compare(field: FieldRef, op: CompareOp, value: Literal) -> Condition {
        condition(ConditionKind::Compare { field, op, value })
    }

    fn like(field: FieldRef, pattern: LikePattern, negated: bool) -> Condition {
        condition(ConditionKind::Like {
            field,
            pattern,
            negated,
        })
    }

    fn within(field: FieldRef, values: Vec<Literal>, negated: bool) -> Condition {
        condition(ConditionKind::In {
            field,
            values,
            negated,
        })
    }

    fn message_is(text: &str) -> Condition {
        compare(
            FieldRef::Message,
            CompareOp::Eq,
            Literal::Text(text.to_string()),
        )
    }

    fn regex(source: &str) -> QueryRegex {
        QueryRegex::new(source).unwrap()
    }

    fn and(left: Condition, right: Condition) -> Condition {
        condition(ConditionKind::And(vec![left, right]))
    }

    fn or(left: Condition, right: Condition) -> Condition {
        condition(ConditionKind::Or(vec![left, right]))
    }

    fn not(inner: Condition) -> Condition {
        condition(ConditionKind::Not(Box::new(inner)))
    }

    #[test]
    fn commands_are_split_by_pipes_and_notes_are_skipped() {
        // U1:BR3.1, BR3.2: blanks and line breaks only separate words; `#`
        // starts a note outside quotes; two filters both stay.
        let read = filters(
            "filter @message = \"a # not a note\" # a note | stats\n\
             | filter @logStream = 's'",
        );
        assert_eq!(
            read,
            vec![
                message_is("a # not a note"),
                compare(
                    FieldRef::LogStream,
                    CompareOp::Eq,
                    Literal::Text("s".to_string())
                ),
            ]
        );
        for (text, span) in [
            ("| filter @message = \"a\"", at(1, 1, 1)),
            ("filter @message = \"a\" |", at(1, 23, 1)),
            (
                "filter @message = \"a\" | | filter @log = \"g\"",
                at(1, 25, 1),
            ),
        ] {
            let found = problem(text);
            assert_eq!(found.message_key, keys::EMPTY_COMMAND, "{text:?}");
            assert_eq!(found.kind, ProblemKind::Syntax);
            assert_eq!(found.span, span, "{text:?}");
        }
        assert_eq!(problem("# only a note").message_key, keys::NO_COMMAND);
    }

    #[test]
    fn other_console_commands_are_unsupported_and_unknown_words_are_mistakes() {
        // U1:BR3.2, U2:BR2.1: the console's other commands are named;
        // anything else is a writing mistake. `fields`, `sort` and `limit`
        // are read since U2, `stats` since 261025-insights-stats U1, so
        // `parse` stands for the commands still not read here.
        let parse = problem("filter @message like /a/\n| parse @message 'a*b' as x");
        assert_eq!(parse.kind, ProblemKind::Unsupported);
        assert_eq!(parse.message_key, keys::UNSUPPORTED_COMMAND);
        assert_eq!(parse.word.as_deref(), Some("parse"));
        assert_eq!(parse.span, at(2, 3, 5));
        for command in ["display", "parse", "dedup", "SOURCE", "unnest"] {
            let found = problem(&format!("{command} @message"));
            assert_eq!(found.message_key, keys::UNSUPPORTED_COMMAND, "{command}");
            assert_eq!(found.word.as_deref(), Some(command));
        }
        let typo = problem("flter @message = \"a\"");
        assert_eq!(typo.kind, ProblemKind::Syntax);
        assert_eq!(typo.message_key, keys::UNKNOWN_COMMAND);
        assert_eq!(typo.word.as_deref(), Some("flter"));
        assert_eq!(typo.span, at(1, 1, 5));
        let bare = problem("filter | filter @log = \"g\"");
        assert_eq!(bare.message_key, keys::MISSING_CONDITION);
        assert_eq!(bare.span, at(1, 1, 6));
    }

    #[test]
    fn q1a_3a_3c_3d_command_names_and_the_checked_words_ignore_case() {
        // U1:BR3.3 as checked at the console (Q1a 3a, 3c, 3d): FILTER,
        // LIKE, IN, Fields, Sort, Desc and Limit are read. Field names keep
        // their case (U1:BR3.6).
        let read = plan(
            "FILTER @message LIKE /a/ | filter @logStream IN ['x'] \
             | Fields @timestamp | Sort @timestamp Desc | Limit 5",
        );
        assert_eq!(
            kinds(&read),
            vec!["filter", "filter", "fields", "sort", "limit"]
        );
        assert_eq!(
            read.filters().map(bare).collect::<Vec<_>>(),
            vec![
                like(FieldRef::Message, LikePattern::Regex(regex("a")), false),
                within(
                    FieldRef::LogStream,
                    vec![Literal::Text("x".to_string())],
                    false
                ),
            ]
        );
        assert_eq!(
            read.commands[2].kind,
            CommandKind::Fields(vec![FieldRef::Timestamp])
        );
        assert_eq!(
            read.commands[3].kind,
            CommandKind::Sort(vec![SortKey {
                field: FieldRef::Timestamp,
                direction: SortDirection::Desc,
            }])
        );
        assert_eq!(read.commands[4].kind, CommandKind::Limit(5));
        let wrong_case = problem("filter @Message = \"a\"");
        assert_eq!(wrong_case.message_key, keys::UNSUPPORTED_FIELD);
        assert_eq!(wrong_case.word.as_deref(), Some("@Message"));
    }

    #[test]
    fn provisional_and_or_not_not_like_and_not_in_ignore_case_too() {
        // Functional-design review R-03: AND, OR, NOT, NOT LIKE and NOT IN
        // were not checked at the console on their own; read without case
        // until U4 compares them.
        assert_eq!(
            only("filter @message = 'a' AND @message = 'b' OR NOT (@message = 'c')"),
            or(and(message_is("a"), message_is("b")), not(message_is("c")))
        );
        assert_eq!(
            only("Filter @log NOT LIKE 'x' And @log Not In [1]"),
            and(
                like(FieldRef::Log, LikePattern::Text("x".to_string()), true),
                within(FieldRef::Log, vec![Literal::Number(1.0)], true),
            )
        );
    }

    #[test]
    fn q1a_4a_4c_and_binds_tighter_than_or_and_parentheses_change_it() {
        // U2:BR2.5: `a or b and c` is `a or (b and c)` (4a = 4b), and
        // `(a or b) and c` differs (4c).
        let a = || message_is("a");
        let b = || message_is("b");
        let c = || message_is("c");
        assert_eq!(
            only("filter @message = 'a' or @message = 'b' and @message = 'c'"),
            or(a(), and(b(), c()))
        );
        assert_eq!(
            only("filter @message = 'a' and @message = 'b' or @message = 'c'"),
            or(and(a(), b()), c())
        );
        assert_eq!(
            only("filter (@message = 'a' or @message = 'b') and @message = 'c'"),
            and(or(a(), b()), c())
        );
        let unclosed = problem("filter (@message = 'a' or\n  (@message = 'b')");
        assert_eq!(unclosed.message_key, keys::UNCLOSED_PAREN);
        assert_eq!(unclosed.span, at(1, 8, 1), "the opening parenthesis");
        let unmatched = problem("filter @message = 'a' ) and @message = 'b'");
        assert_eq!(unmatched.message_key, keys::UNMATCHED_PAREN);
        assert_eq!(unmatched.span, at(1, 23, 1));
        let dangling = problem("filter @message = 'a' and");
        assert_eq!(dangling.message_key, keys::UNEXPECTED_END);
    }

    #[test]
    fn values_are_strings_numbers_expressions_and_lists() {
        // U1:BR3.5.
        assert_eq!(
            only(r#"filter @message = "say \"hi\" \\ 'x'""#),
            message_is(r#"say "hi" \ 'x'"#)
        );
        assert_eq!(only(r"filter @message = 'it\'s'"), message_is("it's"));
        assert_eq!(
            only(r"filter @message = 'a\nb'"),
            message_is(r"a\nb"),
            "other escapes stay as written"
        );
        assert_eq!(
            only("filter @timestamp >= -1.5"),
            compare(FieldRef::Timestamp, CompareOp::GtEq, Literal::Number(-1.5))
        );
        assert_eq!(
            only(r"filter @message like /a\/b\d/"),
            like(
                FieldRef::Message,
                LikePattern::Regex(regex(r"a/b\d")),
                false
            )
        );
        assert_eq!(
            only("filter @logStream in ['a', 2, \"c\"]"),
            within(
                FieldRef::LogStream,
                vec![
                    Literal::Text("a".to_string()),
                    Literal::Number(2.0),
                    Literal::Text("c".to_string()),
                ],
                false
            )
        );
        for (text, key, span) in [
            (
                "filter @message = \"abc",
                keys::UNCLOSED_STRING,
                at(1, 19, 1),
            ),
            (
                "filter\n  @message = 'a",
                keys::UNCLOSED_STRING,
                at(2, 14, 1),
            ),
            (
                "filter @message like /abc",
                keys::UNCLOSED_REGEX,
                at(1, 22, 1),
            ),
            (
                "filter @log in ['a', 'b'",
                keys::UNCLOSED_LIST,
                at(1, 16, 1),
            ),
            ("filter @log in []", keys::EMPTY_LIST, at(1, 16, 1)),
            ("filter @log in 'a'", keys::EXPECTED_LIST, at(1, 16, 3)),
            ("filter @timestamp > 5m", keys::INVALID_NUMBER, at(1, 21, 2)),
            ("filter @message = ", keys::UNEXPECTED_END, at(1, 19, 1)),
            ("filter @message 'a'", keys::EXPECTED_OPERATOR, at(1, 17, 3)),
            (
                "filter @message = & 'a'",
                keys::UNEXPECTED_CHARACTER,
                at(1, 19, 1),
            ),
        ] {
            let found = problem(text);
            assert_eq!((found.message_key, found.span), (key, span), "{text:?}");
            assert_eq!(found.kind, ProblemKind::Syntax, "{text:?}");
        }
    }

    /// The field of the comparison `text` holds.
    fn compared_field(text: &str) -> FieldRef {
        let ConditionKind::Compare { field, .. } = only(text).kind else {
            panic!("{text:?} is no comparison");
        };
        field
    }

    #[test]
    fn the_four_fixed_fields_are_read_and_other_at_names_are_unsupported() {
        // U1:BR3.6, U2:BR1.1: a name starting with @ is one of the four
        // fixed fields or Unsupported; every other name is a JSON field.
        for (text, field) in [
            ("filter @timestamp > 1", FieldRef::Timestamp),
            ("filter @message > 'a'", FieldRef::Message),
            ("filter @logStream > 'a'", FieldRef::LogStream),
            ("filter @log > 'a'", FieldRef::Log),
            ("filter level = 'error'", json(&["level"], "level")),
            ("filter `my field` = 'a'", json(&["my field"], "`my field`")),
        ] {
            assert_eq!(compared_field(text), field, "{text:?}");
        }
        for (text, word, span) in [
            ("filter @ingestionTime > 1", "@ingestionTime", at(1, 8, 14)),
            ("filter @timestamp.x = 1", "@timestamp.x", at(1, 8, 12)),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Unsupported, "{text:?}");
            assert_eq!(found.message_key, keys::UNSUPPORTED_FIELD);
            assert_eq!(found.word.as_deref(), Some(word));
            assert_eq!(found.span, span, "{text:?}");
        }
        assert_eq!(
            problem("filter `unclosed = 'a'").message_key,
            keys::UNCLOSED_NAME
        );
    }

    #[test]
    fn to_millis_of_the_timestamp_is_the_only_function() {
        // U1:BR3.7.
        assert_eq!(
            only("filter toMillis(@timestamp) > 1756053806000"),
            compare(
                FieldRef::TimestampMillis,
                CompareOp::Gt,
                Literal::Number(1_756_053_806_000.0)
            )
        );
        for (text, key, word, span) in [
            (
                "filter toMillis(@message) > 1",
                keys::UNSUPPORTED_TO_MILLIS_ARGUMENT,
                "toMillis",
                at(1, 8, 8),
            ),
            (
                "filter tomillis(@timestamp) > 1",
                keys::UNSUPPORTED_FUNCTION,
                "tomillis",
                at(1, 8, 8),
            ),
            (
                "filter ispresent(@message)",
                keys::UNSUPPORTED_FUNCTION,
                "ispresent",
                at(1, 8, 9),
            ),
            (
                "filter @timestamp > now()",
                keys::UNSUPPORTED_FUNCTION,
                "now",
                at(1, 21, 3),
            ),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Unsupported, "{text:?}");
            assert_eq!(found.message_key, key, "{text:?}");
            assert_eq!(found.word.as_deref(), Some(word), "{text:?}");
            assert_eq!(found.span, span, "{text:?}");
        }
    }

    // ---- Loop-back 2 (p05): a name on the right of a comparison ----------

    fn compare_fields(field: FieldRef, op: CompareOp, other: FieldRef) -> Condition {
        condition(ConditionKind::CompareFields { field, op, other })
    }

    #[test]
    fn p05_a_name_on_the_right_of_a_comparison_is_another_field() {
        // The console reads the unquoted `true` as a field name (p05).
        assert_eq!(
            only("filter readOnly = true"),
            compare_fields(
                json(&["readOnly"], "readOnly"),
                CompareOp::Eq,
                json(&["true"], "true")
            )
        );
        for (text, field, op, other) in [
            (
                "filter a = b",
                json(&["a"], "a"),
                CompareOp::Eq,
                json(&["b"], "b"),
            ),
            (
                "filter a != false",
                json(&["a"], "a"),
                CompareOp::NotEq,
                json(&["false"], "false"),
            ),
            (
                "filter http.status < limits.`max.status`",
                json(&["http", "status"], "http.status"),
                CompareOp::Lt,
                json(&["limits", "max.status"], "limits.`max.status`"),
            ),
            (
                "filter @message = @logStream",
                FieldRef::Message,
                CompareOp::Eq,
                FieldRef::LogStream,
            ),
            (
                "filter @logStream = @log",
                FieldRef::LogStream,
                CompareOp::Eq,
                FieldRef::Log,
            ),
            (
                "filter @timestamp >= toMillis(@timestamp)",
                FieldRef::Timestamp,
                CompareOp::GtEq,
                FieldRef::TimestampMillis,
            ),
            (
                "filter startedAt <= toMillis(@timestamp)",
                json(&["startedAt"], "startedAt"),
                CompareOp::LtEq,
                FieldRef::TimestampMillis,
            ),
        ] {
            assert_eq!(only(text), compare_fields(field, op, other), "{text:?}");
        }
    }

    #[test]
    fn p05_a_field_comparison_keeps_its_place_and_joins_other_conditions() {
        let read = plan("filter a = b and c = 'x'");
        let Some(CommandKind::Filter(whole)) = read.commands.first().map(|c| &c.kind) else {
            panic!("no filter");
        };
        let ConditionKind::And(parts) = &whole.kind else {
            panic!("not an and: {whole:?}");
        };
        assert_eq!(parts[0].span, at(1, 8, 5), "`a = b`");
        assert!(matches!(parts[0].kind, ConditionKind::CompareFields { .. }));
        assert!(matches!(parts[1].kind, ConditionKind::Compare { .. }));
    }

    #[test]
    fn p05_the_right_field_is_read_as_the_left_one_is() {
        // What a field cannot be on the left it cannot be on the right.
        for (text, kind, key, word, span) in [
            (
                "filter a = @ingestionTime",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_FIELD,
                Some("@ingestionTime"),
                at(1, 12, 14),
            ),
            (
                "filter a = now()",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_FUNCTION,
                Some("now"),
                at(1, 12, 3),
            ),
            (
                "filter a = toMillis(@message)",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_TO_MILLIS_ARGUMENT,
                Some("toMillis"),
                at(1, 12, 8),
            ),
            (
                "filter a = b.0",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_ARRAY_ELEMENT,
                Some("b.0"),
                at(1, 12, 3),
            ),
            (
                "filter a = b[0]",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_ARRAY_ELEMENT,
                Some("b"),
                at(1, 12, 1),
            ),
            (
                "filter a = and",
                ProblemKind::Syntax,
                keys::UNEXPECTED_TOKEN,
                Some("and"),
                at(1, 12, 3),
            ),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, kind, "{text:?}");
            assert_eq!(found.message_key, key, "{text:?}");
            assert_eq!(found.word.as_deref(), word, "{text:?}");
            assert_eq!(found.span, span, "{text:?}");
        }
    }

    #[test]
    fn p05_like_in_and_match_still_take_values_only() {
        // Only the comparisons take a field on the right (Loop-back 2).
        for (text, word, span) in [
            ("filter a like b", "b", at(1, 15, 1)),
            ("filter a not like b", "b", at(1, 19, 1)),
            ("filter a =~ b", "b", at(1, 13, 1)),
            ("filter a in [b]", "b", at(1, 14, 1)),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Unsupported, "{text:?}");
            assert_eq!(
                found.message_key,
                keys::UNSUPPORTED_FIELD_COMPARISON,
                "{text:?}"
            );
            assert_eq!(found.word.as_deref(), Some(word), "{text:?}");
            assert_eq!(found.span, span, "{text:?}");
        }
    }

    #[test]
    fn operators_that_do_not_fit_the_field_or_value_are_refused() {
        // U1:BR5.1, BR5.4, BR5.6, BR5.7.
        assert_eq!(
            only("filter @message =~ /a+/"),
            condition(ConditionKind::Match {
                field: FieldRef::Message,
                regex: regex("a+"),
            })
        );
        for (text, kind, key, span) in [
            (
                "filter @message =~ 'a'",
                ProblemKind::Syntax,
                keys::MATCH_NEEDS_REGEX,
                at(1, 20, 3),
            ),
            (
                "filter @message = /a/",
                ProblemKind::Syntax,
                keys::REGEX_NOT_ALLOWED,
                at(1, 19, 3),
            ),
            (
                "filter @log in [/a/]",
                ProblemKind::Syntax,
                keys::REGEX_NOT_ALLOWED,
                at(1, 17, 3),
            ),
            (
                "filter @message like 5",
                ProblemKind::Syntax,
                keys::LIKE_NEEDS_TEXT,
                at(1, 22, 1),
            ),
            (
                "filter @timestamp like /1/",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_TIMESTAMP_OPERATOR,
                at(1, 19, 4),
            ),
            (
                "filter toMillis(@timestamp) =~ /1/",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_TIMESTAMP_OPERATOR,
                at(1, 29, 2),
            ),
            (
                "filter @timestamp + 1 > 5",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_OPERATOR,
                at(1, 19, 1),
            ),
            (
                "filter @timestamp > 5 * 2",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_OPERATOR,
                at(1, 23, 1),
            ),
            (
                "filter (@timestamp > 5 / 2)",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_OPERATOR,
                at(1, 24, 1),
            ),
        ] {
            let found = problem(text);
            assert_eq!(
                (found.kind, found.message_key, found.span),
                (kind, key, span),
                "{text:?}"
            );
        }
    }

    #[test]
    fn expressions_that_cannot_be_built_are_mistakes() {
        // U1:BR3.8: unbalanced, look-around, back-references and the size
        // limit of the regex crate.
        for text in [
            "filter @message like /(/",
            "filter @message like /a(?=b)/",
            r"filter @message =~ /(a)\1/",
            "filter @message like /a{1000}{1000}{1000}/",
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Syntax, "{text:?}");
            assert_eq!(found.message_key, keys::INVALID_REGEX, "{text:?}");
        }
        let place = problem("filter @log = 'g' and\n @message like /[a/");
        assert_eq!(place.span, at(2, 16, 4));
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        // entities.md Span: Unicode scalar values.
        let found = problem("filter @message = \"日本語のログ\" | stats");
        assert_eq!(found.span, at(1, 30, 5));
        let found = problem("filter @message = \"エラー\"\n  and @ログ = 'a'");
        assert_eq!(found.word.as_deref(), Some("@ログ"));
        assert_eq!(found.span, at(2, 7, 3));
        assert_eq!(
            compared_field("filter ログ.種類 = 'a'"),
            json(&["ログ", "種類"], "ログ.種類"),
            "a JSON field may have Japanese keys"
        );
    }

    #[test]
    fn a_text_compared_with_the_timestamp_is_kept_as_a_date_text() {
        // U1:BR5.9: turned into milliseconds when the query runs.
        assert_eq!(
            only("filter @timestamp > \"2025-08-25 01:43:26\""),
            compare(
                FieldRef::Timestamp,
                CompareOp::Gt,
                Literal::DateText {
                    text: "2025-08-25 01:43:26".to_string(),
                    span: at(1, 21, 21),
                }
            )
        );
        assert_eq!(
            only("filter @message > \"2025-08-25 01:43:26\""),
            message_is_greater("2025-08-25 01:43:26"),
            "a text field keeps the text"
        );
    }

    fn message_is_greater(text: &str) -> Condition {
        compare(
            FieldRef::Message,
            CompareOp::Gt,
            Literal::Text(text.to_string()),
        )
    }

    // ---- 261017-insights-query U2 ---------------------------------------

    #[test]
    fn q1a_3b_4d_4e_5e_not_without_parentheses_is_a_mistake() {
        // U2:BR2.4, functional-design review R-07: at the start of a
        // condition `not` must be followed by `(`; the console could not
        // compile any of these.
        for (text, word, span) in [
            (
                "FILTER @message LIKE /a/ AND NOT @message LIKE /b/",
                "NOT",
                at(1, 30, 3),
            ),
            (
                "filter not @message like /a/ and @message like /b/",
                "not",
                at(1, 8, 3),
            ),
            (
                "filter (not @message like /a/) and @message like /b/",
                "not",
                at(1, 9, 3),
            ),
            ("filter not errorCode = \"zzz\"", "not", at(1, 8, 3)),
            ("filter not like /x/", "not", at(1, 8, 3)),
            ("filter not not (@log = 'g')", "not", at(1, 8, 3)),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Syntax, "{text:?}");
            assert_eq!(found.message_key, keys::NOT_NEEDS_PAREN, "{text:?}");
            assert_eq!(found.word.as_deref(), Some(word), "{text:?}");
            assert_eq!(found.span, at(span.line, span.column, span.length));
        }
    }

    #[test]
    fn q1a_4f_not_of_a_parenthesized_condition_is_read() {
        // U2:BR2.4: `not (a and b)`, and `not like` / `not in` after a
        // field stay operators of their own (R-07).
        assert_eq!(
            only("filter not (@message like /a/ and @message like /b/)"),
            not(and(
                like(FieldRef::Message, LikePattern::Regex(regex("a")), false),
                like(FieldRef::Message, LikePattern::Regex(regex("b")), false),
            ))
        );
        assert_eq!(
            only("filter not (not (@log = 'g'))"),
            not(not(compare(
                FieldRef::Log,
                CompareOp::Eq,
                Literal::Text("g".to_string())
            )))
        );
        assert_eq!(
            only("filter errorCode not in ['zzz']"),
            within(
                json(&["errorCode"], "errorCode"),
                vec![Literal::Text("zzz".to_string())],
                true
            )
        );
    }

    #[test]
    fn json_fields_are_read_with_nesting_and_backquotes() {
        // U2:BR1.1, BR1.3.
        assert_eq!(
            only("filter http.status >= 500"),
            compare(
                json(&["http", "status"], "http.status"),
                CompareOp::GtEq,
                Literal::Number(500.0)
            )
        );
        for (text, keys) in [
            ("filter `user.id` = 'u1'", vec!["user.id"]),
            ("filter a.`b.c`.d = 1", vec!["a", "b.c", "d"]),
            ("filter user-agent = 'curl'", vec!["user-agent"]),
            ("filter a.`0` = 1", vec!["a", "0"]),
        ] {
            let FieldRef::Json(path) = compared_field(text) else {
                panic!("{text:?} reads no JSON field");
            };
            assert_eq!(path.keys(), keys.as_slice(), "{text:?}");
        }
        assert_eq!(
            compared_field("filter a.`b.c`.d = 1").name(),
            "a.`b.c`.d",
            "the name as written is the column heading"
        );
        let unclosed = problem("filter a.`b = 1");
        assert_eq!(
            (unclosed.message_key, unclosed.span),
            (keys::UNCLOSED_NAME, at(1, 10, 1))
        );
    }

    #[test]
    fn array_elements_are_unsupported() {
        // U2:BR1.4, functional-design review R-06: a part of only digits is
        // an array element, which U2 does not read (a backquoted number is
        // a key, above).
        for (text, word, span) in [
            ("filter a.0 = 1", "a.0", at(1, 8, 3)),
            (
                "filter resources.0.arn like 'x'",
                "resources.0.arn",
                at(1, 8, 15),
            ),
            ("filter a[0] = 1", "a", at(1, 8, 1)),
            ("fields resources.0", "resources.0", at(1, 8, 11)),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Unsupported, "{text:?}");
            assert_eq!(found.message_key, keys::UNSUPPORTED_ARRAY_ELEMENT);
            assert_eq!(found.word.as_deref(), Some(word), "{text:?}");
            assert_eq!(found.span, span, "{text:?}");
        }
    }

    #[test]
    fn fields_sort_and_limit_are_read_in_the_order_written() {
        // U2:BR2.1, BR3.1, BR4.1 (no direction is ascending, Q1b 1a).
        let read = plan(
            "fields @timestamp, @message, http.status\n\
             | filter level = 'error'\n\
             | sort http.status desc, @timestamp\n\
             | limit 20\n\
             | fields @log, toMillis(@timestamp)",
        );
        assert_eq!(
            kinds(&read),
            vec!["fields", "filter", "sort", "limit", "fields"]
        );
        let spans: Vec<Span> = read.commands.iter().map(|command| command.span).collect();
        assert_eq!(
            spans,
            vec![
                at(1, 1, 6),
                at(2, 3, 6),
                at(3, 3, 4),
                at(4, 3, 5),
                at(5, 3, 6)
            ]
        );
        let status = json(&["http", "status"], "http.status");
        assert_eq!(
            read.commands[0].kind,
            CommandKind::Fields(vec![FieldRef::Timestamp, FieldRef::Message, status.clone()])
        );
        assert_eq!(
            read.commands[2].kind,
            CommandKind::Sort(vec![
                SortKey {
                    field: status,
                    direction: SortDirection::Desc,
                },
                SortKey {
                    field: FieldRef::Timestamp,
                    direction: SortDirection::Asc,
                },
            ])
        );
        assert_eq!(read.commands[3].kind, CommandKind::Limit(20));
        assert_eq!(
            read.commands[4].kind,
            CommandKind::Fields(vec![FieldRef::Log, FieldRef::TimestampMillis])
        );
        assert_eq!(
            plan("sort @timestamp ASC").commands[0].kind,
            CommandKind::Sort(vec![SortKey {
                field: FieldRef::Timestamp,
                direction: SortDirection::Asc,
            }])
        );
    }

    #[test]
    fn q1a_7a_7b_7c_limit_reads_0_to_100000_and_refuses_the_rest() {
        // U2:BR5.1.
        assert_eq!(plan("limit 0").commands[0].kind, CommandKind::Limit(0));
        assert_eq!(
            plan("limit 100000").commands[0].kind,
            CommandKind::Limit(100_000)
        );
        let too_large = problem("limit 100001");
        assert_eq!(too_large.kind, ProblemKind::Syntax);
        assert_eq!(too_large.message_key, keys::LIMIT_TOO_LARGE);
        assert_eq!(too_large.span, at(1, 7, 6));
        for (text, span) in [
            ("limit -1", at(1, 7, 2)),
            ("limit 1.5", at(1, 7, 3)),
            ("limit abc", at(1, 7, 3)),
            ("limit '5'", at(1, 7, 3)),
        ] {
            let found = problem(text);
            assert_eq!(
                (found.kind, found.message_key, found.span),
                (ProblemKind::Syntax, keys::INVALID_LIMIT, span),
                "{text:?}"
            );
        }
        for text in ["limit", "limit | filter @log = 'g'"] {
            let found = problem(text);
            assert_eq!(
                (found.message_key, found.span),
                (keys::MISSING_LIMIT, at(1, 1, 5)),
                "{text:?}"
            );
        }
        let any = problem("limit any 5");
        assert_eq!(any.kind, ProblemKind::Unsupported);
        assert_eq!(any.message_key, keys::UNSUPPORTED_LIMIT_ANY);
        assert_eq!((any.word.as_deref(), any.span), (Some("any"), at(1, 7, 3)));
    }

    #[test]
    fn mistakes_and_unsupported_forms_of_fields_and_sort() {
        // U2:BR2.1, BR2.2, BR4.1.
        for (text, kind, key, word, span) in [
            (
                "fields",
                ProblemKind::Syntax,
                keys::MISSING_FIELDS,
                Some("fields"),
                at(1, 1, 6),
            ),
            (
                "Sort | limit 5",
                ProblemKind::Syntax,
                keys::MISSING_FIELDS,
                Some("Sort"),
                at(1, 1, 4),
            ),
            (
                "fields @message,",
                ProblemKind::Syntax,
                keys::EXPECTED_FIELD,
                None,
                at(1, 17, 1),
            ),
            (
                "fields 5",
                ProblemKind::Syntax,
                keys::EXPECTED_FIELD,
                Some("5"),
                at(1, 8, 1),
            ),
            (
                "fields @message as m",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_ALIAS,
                Some("as"),
                at(1, 17, 2),
            ),
            (
                "sort @timestamp up",
                ProblemKind::Syntax,
                keys::INVALID_SORT_DIRECTION,
                Some("up"),
                at(1, 17, 2),
            ),
            (
                "fields @message @log",
                ProblemKind::Syntax,
                keys::UNEXPECTED_TOKEN,
                Some("@log"),
                at(1, 17, 4),
            ),
            (
                "fields count(*)",
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_FUNCTION,
                Some("count"),
                at(1, 8, 5),
            ),
        ] {
            let found = problem(text);
            assert_eq!(
                (
                    found.kind,
                    found.message_key,
                    found.word.as_deref(),
                    found.span
                ),
                (kind, key, word, span),
                "{text:?}"
            );
        }
    }

    #[test]
    fn every_part_of_a_condition_has_its_place() {
        // U2:BR10.3 (U1 code review R-04): from its first word to its last,
        // over line breaks too.
        let read = plan("filter @log = 'g' and\n  (not (@message like /a/) or b in [1, 2])");
        let condition = read.filters().next().unwrap();
        assert_eq!(condition.span, at(1, 8, 57));
        let ConditionKind::And(parts) = &condition.kind else {
            panic!("no and: {condition:?}");
        };
        let [left, right] = parts.as_slice() else {
            panic!("not two parts: {parts:?}");
        };
        assert_eq!(left.span, at(1, 8, 10));
        assert_eq!(right.span, at(2, 4, 38), "the or inside the parentheses");
        let ConditionKind::Or(parts) = &right.kind else {
            panic!("no or: {right:?}");
        };
        let [negation, list] = parts.as_slice() else {
            panic!("not two parts: {parts:?}");
        };
        assert_eq!(negation.span, at(2, 4, 23));
        assert_eq!(list.span, at(2, 31, 11));
        let ConditionKind::Not(inner) = &negation.kind else {
            panic!("no not: {negation:?}");
        };
        assert_eq!(inner.span, at(2, 9, 17));
    }

    #[test]
    fn every_problem_key_has_a_screen_text_and_is_listed_once() {
        // U2:BR2.3: the screen's catalog check reads keys::ALL.
        let mut all = keys::ALL.to_vec();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), keys::ALL.len());
        for key in keys::ALL {
            assert!(
                key.starts_with("query.syntax.") || key.starts_with("query.unsupported."),
                "{key}"
            );
        }
        assert!(keys::ALL.contains(&keys::NOT_NEEDS_PAREN));
        assert!(keys::ALL.contains(&keys::UNSUPPORTED_ARRAY_ELEMENT));
        assert!(keys::ALL.contains(&keys::NESTING_TOO_DEEP));
    }

    // ---- 261017-insights-query U3:BR5.1 (U2 code review R-01, U3 R-02) ---

    /// Runs `work` on a thread with a stack of 1 MB, the size of Windows'
    /// main thread, and fails when the thread does not end normally.
    fn on_a_small_stack(work: impl FnOnce() + Send + 'static) {
        let thread = std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(work)
            .unwrap();
        assert!(thread.join().is_ok(), "the work ended abnormally");
    }

    /// A `filter` of `terms` comparisons joined by `word`, as short as one
    /// can be written, so that the most of them fit in the longest query.
    fn chain(word: &str, terms: usize) -> String {
        let parts: Vec<&str> = std::iter::repeat_n("a=1", terms).collect();
        format!("filter {}", parts.join(&format!(" {word} ")))
    }

    /// The most `terms` of [`chain`] that fit in the longest query the
    /// screen runs (10,000 characters).
    fn longest_chain(word: &str) -> String {
        let longest = crate::insights_session::MAX_QUERY_CHARS;
        let terms = (longest - "filter ".len() + word.len() + 2) / ("a=1".len() + word.len() + 2);
        let text = chain(word, terms);
        assert!(text.chars().count() <= longest && chain(word, terms + 1).len() > longest);
        text
    }

    /// Reads `text`, turns its date texts into milliseconds, evaluates it on
    /// one event and drops the plan, as a run does.
    fn read_evaluate_and_drop(text: &str) -> bool {
        let plan = parse_query(text).unwrap();
        let plan = crate::query::instant::resolve_date_texts(plan, chrono_tz::Tz::UTC).unwrap();
        let event = crate::event::LogEvent {
            timestamp: 1,
            ingestion_time: None,
            message: r#"{"a":1}"#.to_string(),
            log_stream_name: "s".to_string(),
            sequence: 0,
        };
        let item = crate::query::evaluator::EvalItem {
            event: &event,
            log_group_name: "/g",
        };
        let held = crate::query::evaluator::matches(&plan, &item);
        drop(plan);
        held
    }

    #[test]
    fn the_longest_or_chain_is_read_evaluated_and_dropped_on_a_one_megabyte_stack() {
        // R-02: about 1,400 terms of `a=1 or a=1 …` in 10,000 characters.
        on_a_small_stack(|| {
            let text = longest_chain("or");
            assert!(read_evaluate_and_drop(&text));
        });
    }

    #[test]
    fn the_longest_and_chain_is_read_evaluated_and_dropped_on_a_one_megabyte_stack() {
        on_a_small_stack(|| {
            let text = longest_chain("and");
            assert!(read_evaluate_and_drop(&text));
        });
    }

    /// `filter` with `levels` parentheses, each opened by `open`, around
    /// `a=1 or a=2`.
    fn nested(open: &str, levels: usize) -> String {
        format!(
            "filter {}a=1 or a=2{}",
            open.repeat(levels),
            ")".repeat(levels)
        )
    }

    #[test]
    fn exactly_one_hundred_levels_of_parentheses_are_read_on_a_one_megabyte_stack() {
        on_a_small_stack(|| {
            assert!(read_evaluate_and_drop(&nested("(", MAX_NESTING)));
            assert!(
                read_evaluate_and_drop(&nested("not (", MAX_NESTING)),
                "an even count of not"
            );
            let mixed = format!(
                "filter {}a=1{}",
                "(a=2 or (a=1 and ".repeat(50),
                "))".repeat(50)
            );
            assert!(read_evaluate_and_drop(&mixed), "and and or taking turns");
        });
    }

    #[test]
    fn the_hundred_and_first_level_is_a_syntax_problem_at_its_parenthesis() {
        for (open, column) in [("(", 8 + 100), ("not (", 8 + 100 * 5 + 4)] {
            let text = nested(open, MAX_NESTING + 1);
            let found = problem(&text);
            assert_eq!(found.kind, ProblemKind::Syntax, "{open:?}");
            assert_eq!(found.message_key, keys::NESTING_TOO_DEEP, "{open:?}");
            assert_eq!(
                found.span,
                at(1, column, 1),
                "the 101st opening parenthesis"
            );
        }
        let far = problem(&nested("(", 5_000));
        assert_eq!(
            far.message_key,
            keys::NESTING_TOO_DEEP,
            "found before going deeper"
        );
    }

    #[test]
    fn nesting_counts_only_parentheses_open_at_once() {
        // 150 parentheses one after another are never more than 1 deep.
        let side_by_side = format!("filter {}", vec!["(a=1)"; 150].join(" and "));
        assert!(parse_query(&side_by_side).is_ok());
        let unclosed = problem(&format!("filter {}a=1", "(".repeat(MAX_NESTING)));
        assert_eq!(
            unclosed.message_key,
            keys::UNCLOSED_PAREN,
            "100 deep but unclosed is still unclosed"
        );
    }

    #[test]
    fn and_binds_tighter_than_or_in_long_chains_too() {
        // U2:BR2.5 kept with the lists of R-02: `a or b and c or d` is
        // `a or (b and c) or d`.
        let holds = |text: &str, message: &str| {
            let plan = parse_query(text).unwrap();
            let event = crate::event::LogEvent {
                timestamp: 1,
                ingestion_time: None,
                message: message.to_string(),
                log_stream_name: "s".to_string(),
                sequence: 0,
            };
            crate::query::evaluator::matches(
                &plan,
                &crate::query::evaluator::EvalItem {
                    event: &event,
                    log_group_name: "/g",
                },
            )
        };
        let query = "filter a = 1 or b = 1 and c = 1 or d = 1";
        assert!(holds(query, r#"{"a":1}"#));
        assert!(holds(query, r#"{"d":1}"#));
        assert!(!holds(query, r#"{"b":1}"#), "b alone is not enough");
        assert!(holds(query, r#"{"b":1,"c":1}"#));
        let negated = "filter not (a = 1 or b = 1) and c = 1";
        assert!(holds(negated, r#"{"c":1}"#));
        assert!(!holds(negated, r#"{"a":1,"c":1}"#));
        let a_is = |n: f64| compare(json(&["a"], "a"), CompareOp::Eq, Literal::Number(n));
        assert_eq!(
            only("filter a=1 or a=2 and a=3 or a=4"),
            condition(ConditionKind::Or(vec![
                a_is(1.0),
                and(a_is(2.0), a_is(3.0)),
                a_is(4.0)
            ])),
            "a run of or is one list of its parts"
        );
    }

    // ---- 261025-insights-stats U1:BR1.1-BR1.9 ----------------------------

    /// The `stats` command of `text`, which must be read.
    fn stats_of(text: &str) -> crate::query::plan::StatsCommand {
        match plan(text).stats() {
            Some(stats) => stats.clone(),
            None => panic!("{text:?} has no stats"),
        }
    }

    fn aggregate(
        function: crate::query::plan::AggregateFunction,
        argument: crate::query::plan::AggregateArgument,
        name: &str,
    ) -> crate::query::plan::AggregateSpec {
        crate::query::plan::AggregateSpec {
            function,
            argument,
            percentile: None,
            column_name: name.to_string(),
        }
    }

    fn field_key(field: FieldRef, name: &str) -> crate::query::plan::GroupKey {
        crate::query::plan::GroupKey {
            kind: crate::query::plan::GroupKeyKind::Field(field),
            column_name: name.to_string(),
        }
    }

    fn bin_key(
        count: u32,
        unit: crate::query::plan::BinUnit,
        name: &str,
    ) -> crate::query::plan::GroupKey {
        crate::query::plan::GroupKey {
            kind: crate::query::plan::GroupKeyKind::Bin(crate::query::plan::BinPeriod {
                count,
                unit,
            }),
            column_name: name.to_string(),
        }
    }

    fn assert_problem(text: &str, kind: ProblemKind, key: &str, span: Span) -> QueryProblem {
        let found = problem(text);
        assert_eq!(found.kind, kind, "{text:?}: {found:?}");
        assert_eq!(found.message_key, key, "{text:?}: {found:?}");
        assert_eq!(found.span, span, "{text:?}: {found:?}");
        found
    }

    #[test]
    fn stats_counts_by_a_bin_of_five_minutes() {
        // BR1.1, BR1.5, BR1.8: the walking skeleton's query.
        use crate::query::plan::{AggregateArgument, AggregateFunction, BinUnit};
        let read = plan("filter @message like /ERROR/\n| stats count(*) by bin(5m)");
        assert_eq!(kinds(&read), vec!["filter", "stats"]);
        let stats = stats_of("stats count(*) by bin(5m)");
        assert_eq!(
            stats.aggregates,
            vec![aggregate(
                AggregateFunction::Count,
                AggregateArgument::All,
                "count(*)"
            )]
        );
        assert_eq!(
            stats.group_keys,
            vec![bin_key(5, BinUnit::Minutes, "bin(5m)")]
        );
    }

    #[test]
    fn stats_reads_several_aggregates_with_as_names_and_no_by() {
        // BR1.1, BR1.4: `as` names the column; without `by` there is no key.
        use crate::query::plan::{AggregateArgument, AggregateFunction};
        let latency = || AggregateArgument::Field(json(&["latency"], "latency"));
        let stats = stats_of("stats avg(latency) as a, max(latency) as m");
        assert_eq!(
            stats.aggregates,
            vec![
                aggregate(AggregateFunction::Avg, latency(), "a"),
                aggregate(AggregateFunction::Max, latency(), "m"),
            ]
        );
        assert!(stats.group_keys.is_empty());
        let stats = stats_of(
            "STATS count(level), sum(bytes), min(@timestamp), \
             sum(toMillis(@timestamp)) By level, @logStream as stream",
        );
        assert_eq!(
            stats.aggregates,
            vec![
                aggregate(
                    AggregateFunction::Count,
                    AggregateArgument::Field(json(&["level"], "level")),
                    "count(level)"
                ),
                aggregate(
                    AggregateFunction::Sum,
                    AggregateArgument::Field(json(&["bytes"], "bytes")),
                    "sum(bytes)"
                ),
                aggregate(
                    AggregateFunction::Min,
                    AggregateArgument::Field(FieldRef::Timestamp),
                    "min(@timestamp)"
                ),
                aggregate(
                    AggregateFunction::Sum,
                    AggregateArgument::Field(FieldRef::TimestampMillis),
                    "sum(toMillis(@timestamp))"
                ),
            ]
        );
        assert_eq!(
            stats.group_keys,
            vec![
                field_key(json(&["level"], "level"), "level"),
                field_key(FieldRef::LogStream, "stream"),
            ]
        );
        assert_eq!(
            stats_of("stats count() by http.status").aggregates[0].argument,
            AggregateArgument::All,
            "count() counts every event as count(*) does"
        );
    }

    #[test]
    fn a_column_without_as_is_the_text_as_written_with_blanks_made_one() {
        // BR1.9: cut from the text, line breaks and runs of blanks made one
        // blank, notes left out.
        let stats = stats_of("stats count( * ),  avg(\n   latency\t)\n# a note\n  by bin( 30s )");
        assert_eq!(
            stats.column_names(),
            vec!["bin( 30s )", "count( * )", "avg( latency )"]
        );
        let noted = stats_of("stats count(\n# a note\n*)");
        assert_eq!(noted.column_names(), vec!["count( *)"]);
    }

    #[test]
    fn bin_reads_seconds_minutes_hours_and_days() {
        // BR1.5, BR1.8.
        use crate::query::plan::BinUnit;
        for (text, count, unit) in [
            ("bin(30s)", 30, BinUnit::Seconds),
            ("bin(5m)", 5, BinUnit::Minutes),
            ("bin(2h)", 2, BinUnit::Hours),
            ("bin(1d)", 1, BinUnit::Days),
            ("bin(1000000m)", 1_000_000, BinUnit::Minutes),
        ] {
            let stats = stats_of(&format!("stats count(*) by {text}"));
            assert_eq!(stats.group_keys, vec![bin_key(count, unit, text)], "{text}");
        }
    }

    #[test]
    fn bin_units_of_the_console_not_read_yet_are_unsupported() {
        // BR1.5 (Q1 A): ms, w, mo, q and y.
        for unit in ["ms", "w", "mo", "q", "y"] {
            let text = format!("stats count(*) by bin(5{unit})");
            let length = u32::try_from(unit.len()).unwrap() + 1;
            let found = assert_problem(
                &text,
                ProblemKind::Unsupported,
                keys::UNSUPPORTED_BIN_UNIT,
                at(1, 23, length),
            );
            assert_eq!(found.word, Some(format!("5{unit}")));
        }
    }

    #[test]
    fn a_bin_period_of_zero_above_the_limit_or_of_no_form_is_a_mistake() {
        // BR1.5, BR1.8: 0, above 1,000,000, no number, no unit, an unknown
        // unit; an unclosed bin( is the unclosed parenthesis.
        for (period, span) in [
            ("0m", at(1, 23, 2)),
            ("1000001m", at(1, 23, 8)),
            ("99999999999999999999m", at(1, 23, 21)),
            ("m", at(1, 23, 1)),
            ("5", at(1, 23, 1)),
            ("5x", at(1, 23, 2)),
            ("5M", at(1, 23, 2)),
            ("-5m", at(1, 23, 3)),
            ("", at(1, 23, 1)),
        ] {
            let text = format!("stats count(*) by bin({period})");
            assert_problem(&text, ProblemKind::Syntax, keys::INVALID_BIN_PERIOD, span);
        }
        assert_problem(
            "stats count(*) by bin(5m",
            ProblemKind::Syntax,
            keys::UNCLOSED_PAREN,
            at(1, 22, 1),
        );
        assert_problem(
            "stats count(*) by bin(5m | limit 1",
            ProblemKind::Syntax,
            keys::UNCLOSED_PAREN,
            at(1, 22, 1),
        );
    }

    #[test]
    fn the_console_functions_are_read_and_others_unknown() {
        // U1:BR1.2 named the rest of the documentation's functions
        // Unsupported; U2:BR1.1 reads them (changed as BR5.1 allows). Any
        // other name (or another case) is still an unknown function.
        use crate::query::plan::{AggregateArgument, AggregateFunction};
        for name in [
            "count_distinct",
            "stddev",
            "earliest",
            "latest",
            "sortsFirst",
            "sortsLast",
        ] {
            let stats = stats_of(&format!("stats {name}(x)"));
            assert_eq!(
                stats.aggregates,
                vec![aggregate(
                    AggregateFunction::from_name(name).unwrap(),
                    AggregateArgument::Field(json(&["x"], "x")),
                    &format!("{name}(x)")
                )],
                "{name}"
            );
        }
        assert_eq!(
            stats_of("stats pct(x, 50)").aggregates[0].function,
            AggregateFunction::Pct
        );
        for name in ["COUNT", "toMillis", "median", "sortsfirst"] {
            let text = format!("stats {name}(x)");
            let found = problem(&text);
            assert_eq!(found.message_key, keys::UNSUPPORTED_FUNCTION, "{text}");
            assert_eq!(found.word.as_deref(), Some(name));
        }
    }

    #[test]
    fn only_count_takes_a_star_and_arguments_are_fields() {
        // BR1.3.
        for function in ["sum", "avg", "min", "max"] {
            let text = format!("stats {function}(*)");
            let column = u32::try_from(function.len()).unwrap() + 8;
            assert_problem(
                &text,
                ProblemKind::Syntax,
                keys::AGGREGATE_STAR,
                at(1, column, 1),
            );
        }
        let operator = problem("stats sum(a + b)");
        assert_eq!(operator.message_key, keys::UNSUPPORTED_OPERATOR);
        assert_eq!(operator.span, at(1, 13, 1));
        let key_function = problem("stats count(*) by toMillis(@timestamp)");
        assert_eq!(key_function.kind, ProblemKind::Unsupported);
        assert_eq!(key_function.message_key, keys::UNSUPPORTED_FUNCTION);
        let key_operator = problem("stats count(*) by bin(5m) + 1");
        assert_eq!(key_operator.message_key, keys::UNSUPPORTED_OPERATOR);
        let value = problem("stats sum(5)");
        assert_eq!(value.message_key, keys::EXPECTED_FIELD);
        assert_eq!(value.span, at(1, 11, 1));
        let array = problem("stats max(a.0)");
        assert_eq!(array.message_key, keys::UNSUPPORTED_ARRAY_ELEMENT);
    }

    #[test]
    fn stats_mistakes_are_found_with_their_place() {
        // BR1.7.
        for (text, key, span) in [
            ("stats count(*) by", keys::MISSING_GROUP_KEY, at(1, 16, 2)),
            (
                "stats count(*) by | limit 1",
                keys::MISSING_GROUP_KEY,
                at(1, 16, 2),
            ),
            ("stats count(*", keys::UNCLOSED_PAREN, at(1, 12, 1)),
            ("stats sum(a, b)", keys::ARGUMENT_COUNT, at(1, 10, 1)),
            ("stats sum()", keys::ARGUMENT_COUNT, at(1, 10, 1)),
            ("stats count(*),", keys::TRAILING_COMMA, at(1, 15, 1)),
            (
                "stats count(*), by level",
                keys::TRAILING_COMMA,
                at(1, 15, 1),
            ),
            (
                "stats count(*) by level,",
                keys::TRAILING_COMMA,
                at(1, 24, 1),
            ),
            ("stats by level", keys::MISSING_AGGREGATE, at(1, 1, 5)),
            ("stats", keys::MISSING_AGGREGATE, at(1, 1, 5)),
            ("stats | limit 1", keys::MISSING_AGGREGATE, at(1, 1, 5)),
            ("stats level", keys::EXPECTED_AGGREGATE, at(1, 7, 5)),
            ("stats count(*) as", keys::EXPECTED_ALIAS, at(1, 16, 2)),
            ("stats count(*) as 5", keys::EXPECTED_ALIAS, at(1, 19, 1)),
        ] {
            assert_problem(text, ProblemKind::Syntax, key, span);
        }
        assert_eq!(
            problem("stats count(*) level").message_key,
            keys::UNEXPECTED_TOKEN
        );
    }

    #[test]
    fn two_columns_of_the_same_name_are_a_mistake_at_the_second() {
        // BR1.4: the keys and the aggregates share their names.
        for (text, span) in [
            ("stats count(*), count(*)", at(1, 17, 8)),
            ("stats count(*) as n, sum(x) as n", at(1, 32, 1)),
            ("stats count(*) as level by level", at(1, 28, 5)),
            ("stats count(*) by level, level", at(1, 26, 5)),
        ] {
            assert_problem(text, ProblemKind::Syntax, keys::DUPLICATE_COLUMN, span);
        }
        assert_eq!(
            stats_of("stats count(*) as n, count(*)").column_names(),
            vec!["n", "count(*)"]
        );
    }

    #[test]
    fn after_stats_fields_and_a_second_stats_are_unsupported_and_others_read() {
        // U1:BR1.6 made stats the last command; U2:BR1.4, BR1.5 read filter,
        // sort and limit after it and keep only fields and a second stats
        // Unsupported (changed as BR5.1 allows). The ones before it are read.
        for (text, word, key, span) in [
            (
                "stats count(*) | fields a",
                "fields",
                keys::FIELDS_AFTER_STATS,
                at(1, 18, 6),
            ),
            (
                "stats count(*)\n| stats count(*)",
                "stats",
                keys::SECOND_STATS,
                at(2, 3, 5),
            ),
            (
                "stats count(*) | Parse x",
                "Parse",
                keys::UNSUPPORTED_COMMAND,
                at(1, 18, 5),
            ),
        ] {
            let found = assert_problem(text, ProblemKind::Unsupported, key, span);
            assert_eq!(found.word.as_deref(), Some(word));
        }
        assert_problem(
            "stats count(*) | flter x = 1",
            ProblemKind::Syntax,
            keys::UNKNOWN_COMMAND,
            at(1, 18, 5),
        );
        assert_eq!(
            stats_of("stats count(*) | limit 5").after,
            vec![crate::query::plan::AfterStatsCommand::Limit(5)]
        );
        let read = plan("fields a | filter a = 1 | sort @timestamp | limit 9 | stats count(*)");
        assert_eq!(
            kinds(&read),
            vec!["fields", "filter", "sort", "limit", "stats"]
        );
        assert_eq!(problem("stats count(*) |").message_key, keys::EMPTY_COMMAND);
    }

    #[test]
    fn the_number_and_alias_mistakes_outside_stats_stay_as_they_were() {
        // BR1.8: `5m` outside bin( is still a number run into letters, and
        // `fields ... as` is still Unsupported.
        let number = problem("filter @timestamp > 5m");
        assert_eq!(number.message_key, keys::INVALID_NUMBER);
        assert_eq!(number.span, at(1, 21, 2));
        assert_eq!(
            problem("fields @message as m").message_key,
            keys::UNSUPPORTED_ALIAS
        );
        assert_eq!(
            problem("filter bin(5m) = 1").message_key,
            keys::UNSUPPORTED_FUNCTION
        );
    }

    #[test]
    fn every_key_is_listed_once() {
        let mut all = keys::ALL.to_vec();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), keys::ALL.len());
        for key in [
            keys::MISSING_AGGREGATE,
            keys::EXPECTED_AGGREGATE,
            keys::MISSING_GROUP_KEY,
            keys::TRAILING_COMMA,
            keys::ARGUMENT_COUNT,
            keys::AGGREGATE_STAR,
            keys::EXPECTED_ALIAS,
            keys::DUPLICATE_COLUMN,
            keys::INVALID_BIN_PERIOD,
            keys::UNSUPPORTED_BIN_UNIT,
            keys::UNSUPPORTED_AGGREGATE,
            // U2 (U1's UNSUPPORTED_AFTER_STATS is replaced by the two
            // narrower keys of BR1.5).
            keys::PCT_OUT_OF_RANGE,
            keys::UNKNOWN_RESULT_COLUMN,
            keys::FIELDS_AFTER_STATS,
            keys::SECOND_STATS,
        ] {
            assert!(keys::ALL.contains(&key), "{key}");
        }
    }

    // ---- 261025-insights-stats U2:BR1.1-BR1.5, BR4.1, BR5.1 ---------------

    fn column(index: usize, name: &str, is_timestamp: bool) -> FieldRef {
        FieldRef::Column(crate::query::plan::ColumnRef {
            index,
            name: name.to_string(),
            is_timestamp,
        })
    }

    /// The commands after `stats` in `text`, their conditions bare.
    fn after_of(text: &str) -> Vec<crate::query::plan::AfterStatsCommand> {
        use crate::query::plan::AfterStatsCommand;
        stats_of(text)
            .after
            .iter()
            .map(|command| match command {
                AfterStatsCommand::Filter(condition) => AfterStatsCommand::Filter(bare(condition)),
                other => other.clone(),
            })
            .collect()
    }

    #[test]
    fn the_twelve_functions_are_read_each_with_its_one_field() {
        // BR1.1, BR1.2.
        use crate::query::plan::{AggregateArgument, AggregateFunction};
        let stats = stats_of(
            "stats count(*), count(user), count_distinct(user), sum(latency), avg(latency), \
             min(latency), max(latency), pct(latency, 95), stddev(latency), \
             earliest(@message), latest(level), sortsFirst(user), sortsLast(user)",
        );
        let functions: Vec<AggregateFunction> = stats
            .aggregates
            .iter()
            .map(|aggregate| aggregate.function)
            .collect();
        use AggregateFunction::*;
        assert_eq!(
            functions,
            vec![
                Count,
                Count,
                CountDistinct,
                Sum,
                Avg,
                Min,
                Max,
                Pct,
                Stddev,
                Earliest,
                Latest,
                SortsFirst,
                SortsLast
            ]
        );
        assert_eq!(
            stats.aggregates[9].argument,
            AggregateArgument::Field(FieldRef::Message)
        );
        assert!(
            stats
                .aggregates
                .iter()
                .all(|aggregate| (aggregate.function == Pct) == aggregate.percentile.is_some())
        );
        let unknown = problem("stats sortsfirst(user)");
        assert_eq!(unknown.kind, ProblemKind::Unsupported);
        assert_eq!(unknown.message_key, keys::UNSUPPORTED_FUNCTION);
        assert_eq!(unknown.word.as_deref(), Some("sortsfirst"));
    }

    #[test]
    fn pct_takes_a_field_and_a_percentile_from_0_to_100() {
        // BR1.2, BR1.3: the column is the text as written.
        use crate::query::plan::Percentile;
        for (text, numerator, scale) in [
            ("pct(latency, 95)", 95, 0),
            ("pct(latency, 99.9)", 999, 1),
            ("pct(latency, 0)", 0, 0),
            ("pct(latency, 100)", 100, 0),
        ] {
            let stats = stats_of(&format!("stats {text}"));
            let aggregate = &stats.aggregates[0];
            assert_eq!(
                aggregate.percentile,
                Some(Percentile { numerator, scale }),
                "{text}"
            );
            assert_eq!(aggregate.column_name, text);
        }
        assert_eq!(
            stats_of("stats pct(latency,95) as p95").column_names(),
            vec!["p95"]
        );
        assert_eq!(
            stats_of("stats pct( latency ,  95 )").column_names(),
            vec!["pct( latency , 95 )"]
        );
    }

    #[test]
    fn a_percentile_out_of_range_or_no_number_is_a_mistake_at_its_place() {
        // BR1.2.
        for (text, span) in [
            ("stats pct(latency, 101)", at(1, 20, 3)),
            ("stats pct(latency, -1)", at(1, 20, 2)),
            ("stats pct(latency, 100.5)", at(1, 20, 5)),
        ] {
            let found = assert_problem(text, ProblemKind::Syntax, keys::PCT_OUT_OF_RANGE, span);
            assert!(found.word.is_some(), "{text}");
        }
        assert_problem(
            "stats pct(latency, x)",
            ProblemKind::Syntax,
            keys::EXPECTED_VALUE,
            at(1, 20, 1),
        );
        assert_problem(
            "stats pct(latency, 95",
            ProblemKind::Syntax,
            keys::UNCLOSED_PAREN,
            at(1, 10, 1),
        );
    }

    #[test]
    fn every_function_takes_its_number_of_arguments() {
        // BR1.2, U1:BR1.3, BR1.7.
        for (text, key, span) in [
            ("stats pct(latency)", keys::ARGUMENT_COUNT, at(1, 10, 1)),
            ("stats pct(latency, 1, 2)", keys::ARGUMENT_COUNT, at(1, 10, 1)),
            ("stats count_distinct(a, b)", keys::ARGUMENT_COUNT, at(1, 21, 1)),
            ("stats latest()", keys::ARGUMENT_COUNT, at(1, 13, 1)),
            ("stats stddev(*)", keys::AGGREGATE_STAR, at(1, 14, 1)),
            ("stats pct(*, 95)", keys::AGGREGATE_STAR, at(1, 11, 1)),
        ] {
            assert_problem(text, ProblemKind::Syntax, key, span);
        }
    }

    #[test]
    fn filter_sort_and_limit_after_stats_are_kept_in_order_on_the_result_columns() {
        // BR1.4, UC1.
        use crate::query::plan::AfterStatsCommand;
        let text = "stats count(*) as n by level | filter n > 10 | sort n desc | limit 5";
        assert_eq!(kinds(&plan(text)), vec!["stats"]);
        assert_eq!(
            after_of(text),
            vec![
                AfterStatsCommand::Filter(compare(
                    column(1, "n", false),
                    CompareOp::Gt,
                    Literal::Number(10.0)
                )),
                AfterStatsCommand::Sort(vec![SortKey {
                    field: column(1, "n", false),
                    direction: SortDirection::Desc,
                }]),
                AfterStatsCommand::Limit(5),
            ]
        );
        assert_eq!(
            after_of("stats count(*) as n | limit 1 | filter n = 2 | limit 0"),
            vec![
                AfterStatsCommand::Limit(1),
                AfterStatsCommand::Filter(compare(
                    column(0, "n", false),
                    CompareOp::Eq,
                    Literal::Number(2.0)
                )),
                AfterStatsCommand::Limit(0),
            ]
        );
    }

    #[test]
    fn a_result_column_is_named_as_written_backquotes_left_out() {
        // BR1.4 (b), (d), (e).
        use crate::query::plan::AfterStatsCommand;
        assert_eq!(
            after_of("stats count(*) by level | filter `count(*)` >= 1"),
            vec![AfterStatsCommand::Filter(compare(
                column(1, "count(*)", false),
                CompareOp::GtEq,
                Literal::Number(1.0)
            ))]
        );
        assert_eq!(
            after_of("stats count(*) by http.status, @logStream | sort http.status, `@logStream`"),
            vec![AfterStatsCommand::Sort(vec![
                SortKey {
                    field: column(0, "http.status", false),
                    direction: SortDirection::Asc,
                },
                SortKey {
                    field: column(1, "@logStream", false),
                    direction: SortDirection::Asc,
                },
            ])]
        );
        assert_eq!(
            after_of("stats count(*) as n, sum(x) as m | filter n != m"),
            vec![AfterStatsCommand::Filter(condition(
                ConditionKind::CompareFields {
                    field: column(0, "n", false),
                    op: CompareOp::NotEq,
                    other: column(1, "m", false),
                }
            ))]
        );
        assert_eq!(
            after_of("stats count(*) as n by level | filter level in ['a'] and not (n < 2)"),
            vec![AfterStatsCommand::Filter(and(
                within(
                    column(0, "level", false),
                    vec![Literal::Text("a".to_string())],
                    false
                ),
                not(compare(
                    column(1, "n", false),
                    CompareOp::Lt,
                    Literal::Number(2.0)
                ))
            ))]
        );
    }

    #[test]
    fn a_name_that_is_no_result_column_is_a_mistake_at_its_place() {
        // BR1.4 (b): exact case; a name like count(*) needs backquotes.
        for (text, word, span) in [
            ("stats count(*) as n | filter level = 'a'", "level", at(1, 30, 5)),
            ("stats count(*) as n | sort N", "N", at(1, 28, 1)),
            (
                "stats count(*) by level | filter count(*) > 1",
                "count",
                at(1, 34, 5),
            ),
            ("stats count(*) as n | filter n = m", "m", at(1, 34, 1)),
        ] {
            let found = assert_problem(text, ProblemKind::Syntax, keys::UNKNOWN_RESULT_COLUMN, span);
            assert_eq!(found.word.as_deref(), Some(word), "{text}");
        }
    }

    #[test]
    fn a_limit_after_stats_has_the_bounds_of_every_limit() {
        // BR1.4 (e).
        use crate::query::plan::AfterStatsCommand;
        assert_eq!(
            stats_of("stats count(*) | limit 100000").after,
            vec![AfterStatsCommand::Limit(100_000)]
        );
        assert_problem(
            "stats count(*) | limit 100001",
            ProblemKind::Syntax,
            keys::LIMIT_TOO_LARGE,
            at(1, 24, 6),
        );
        assert_problem(
            "stats count(*) | limit",
            ProblemKind::Syntax,
            keys::MISSING_LIMIT,
            at(1, 18, 5),
        );
        assert_problem(
            "stats count(*) | sort",
            ProblemKind::Syntax,
            keys::MISSING_FIELDS,
            at(1, 18, 4),
        );
        assert_problem(
            "stats count(*) | filter",
            ProblemKind::Syntax,
            keys::MISSING_CONDITION,
            at(1, 18, 6),
        );
    }

    #[test]
    fn a_time_column_takes_date_texts_and_refuses_like_as_the_timestamp_does() {
        // BR1.4 (c): a time column is read as @timestamp is.
        use crate::query::plan::AfterStatsCommand;
        assert_eq!(
            after_of("stats count(*) by bin(5m) as t | filter t > \"2025-08-25 01:43:26\""),
            vec![AfterStatsCommand::Filter(compare(
                column(0, "t", true),
                CompareOp::Gt,
                Literal::DateText {
                    text: "2025-08-25 01:43:26".to_string(),
                    span: at(1, 45, 21),
                }
            ))]
        );
        assert_problem(
            "stats count(*) by bin(5m) as t | filter t like 'x'",
            ProblemKind::Unsupported,
            keys::UNSUPPORTED_TIMESTAMP_OPERATOR,
            at(1, 43, 4),
        );
        assert_problem(
            "stats count(*) by bin(5m) as t | filter t > 'yesterday'",
            ProblemKind::Syntax,
            keys::INVALID_DATE_TIME,
            at(1, 45, 11),
        );
        assert_eq!(
            after_of("stats count(*) as n | filter n = '5'"),
            vec![AfterStatsCommand::Filter(compare(
                column(0, "n", false),
                CompareOp::Eq,
                Literal::Text("5".to_string())
            ))],
            "a column that holds no time keeps the text"
        );
    }

    #[test]
    fn the_catalog_decides_the_unsupported_commands_and_the_reserved_words() {
        // BR4.1: commands in any case; only the reserved keywords are never
        // fields, so `desc`, `by`, `as` and `asc` are still read as fields
        // (FR review R-04, BR5.1).
        use crate::query::catalog::{QueryCatalog, VocabularyKind};
        for name in QueryCatalog::names(VocabularyKind::Command, false) {
            for written in [name.to_string(), name.to_ascii_uppercase()] {
                let found = problem(&format!("{written} x"));
                assert_eq!(found.kind, ProblemKind::Unsupported, "{written}");
                assert_eq!(found.message_key, keys::UNSUPPORTED_COMMAND, "{written}");
                assert_eq!(found.word.as_deref(), Some(written.as_str()));
            }
        }
        for word in ["and", "or", "not", "like", "in"] {
            assert!(parse_query(&format!("filter {word} = 1")).is_err(), "{word}");
            assert!(parse_query(&format!("sort {word}")).is_err(), "{word}");
        }
        for word in ["by", "as", "asc", "desc"] {
            assert_eq!(
                only(&format!("filter {word} = 1")),
                compare(json(&[word], word), CompareOp::Eq, Literal::Number(1.0)),
                "{word}"
            );
        }
        let read = plan("filter desc = 'x' | sort by desc");
        let CommandKind::Sort(sort_keys) = &read.commands[1].kind else {
            panic!("no sort: {read:?}");
        };
        assert_eq!(
            sort_keys,
            &vec![SortKey {
                field: json(&["by"], "by"),
                direction: SortDirection::Desc,
            }]
        );
    }
}
