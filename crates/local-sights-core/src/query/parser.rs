//! QueryParser: reads a query made of `filter` commands (261017-insights-query
//! U1:BR3.1-BR3.9, BR5.4, BR5.6, BR5.7).
//!
//! The text is read word by word, in order, and the first problem found is
//! returned with its place (U1:BR3.9). Blanks and line breaks only separate
//! words; `#` starts a note up to the end of the line outside quotes and
//! regular expressions (U1:BR3.1). Commands are separated by `|`; only
//! `filter` is read, the console's other commands are Unsupported and any
//! other word is a writing mistake (U1:BR3.2). Command names and the words
//! `and`, `or`, `not`, `like` and `in` ignore case; field names do not
//! (U1:BR3.3, provisional). Conditions bind, from the tightest, as
//! comparison / `like` / `=~` / `in`, then `not`, `and`, `or`, and
//! parentheses change that (U1:BR3.4, provisional).
//!
//! A `/` starts a regular expression and a `-` before a digit starts a
//! negative number only where a value is expected; elsewhere both are
//! arithmetic operators, which U1 does not run (U1:BR5.7). The word reader
//! is told which of the two it is reading, so a query is read in one pass.

use crate::query::instant::is_date_text;
use crate::query::plan::{
    CompareOp, Condition, FieldRef, LikePattern, Literal, QueryPlan, QueryProblem, QueryRegex, Span,
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
    /// A field compared with another field.
    pub const UNSUPPORTED_FIELD_COMPARISON: &str = "query.unsupported.fieldComparison";

    /// Every key above, for the screen's catalog check.
    pub const ALL: [&str; 31] = [
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
    ];
}

/// The console's commands other than `filter`, in lower case (U1:BR3.2).
/// `fields`, `sort` and `limit` are read from U2.
const OTHER_COMMANDS: [&str; 14] = [
    "fields",
    "sort",
    "limit",
    "stats",
    "parse",
    "dedup",
    "display",
    "pattern",
    "diff",
    "unmask",
    "unnest",
    "lookup",
    "filterindex",
    "source",
];

/// Words that join or negate conditions and name operators (U1:BR3.3).
const KEYWORDS: [&str; 5] = ["and", "or", "not", "like", "in"];

/// The only function U1 reads, written with this case (U1:BR3.7).
const TO_MILLIS: &str = "toMillis";

/// Reads `text` into a plan, or returns the first problem found, in the
/// order of the text (U1:BR3.9).
pub fn parse_query(text: &str) -> Result<QueryPlan, QueryProblem> {
    let mut parser = Parser::new(text);
    let mut filters = Vec::new();
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
        filters.push(parser.command(command)?);
        match parser.next()? {
            None => return Ok(QueryPlan { filters }),
            Some(token) if token.kind == TokenKind::Pipe => previous_pipe = Some(token.span),
            Some(token) => return Err(unexpected(token)),
        }
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
    /// `+ - * / ^ %`, which U1 does not run (U1:BR5.7).
    Arith,
    /// A bare word: a command, a field, a keyword or a function name.
    Word(String),
    /// A name in backquotes.
    Quoted(String),
    /// A quoted string, already unescaped.
    Text(String),
    Number(f64),
    /// A regular expression, `\/` already read as `/`.
    Regex(String),
}

/// One word with where it was written and how.
#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    span: Span,
    /// The characters as written.
    raw: String,
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
            '`' => self.quoted_name(start_span)?,
            '/' if mode == Mode::Value => self.regex(start_span)?,
            '-' if mode == Mode::Value && self.peek_char(1).is_some_and(|d| d.is_ascii_digit()) => {
                self.number(start_span)?
            }
            '+' | '-' | '*' | '/' | '^' | '%' => {
                self.bump();
                TokenKind::Arith
            }
            d if d.is_ascii_digit() => self.number(start_span)?,
            w if is_word_start(w) => {
                while self.peek_char(0).is_some_and(is_word_char) {
                    self.bump();
                }
                TokenKind::Word(self.chars[start..self.pos].iter().collect())
            }
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

    /// A name in backquotes.
    fn quoted_name(&mut self, start: Span) -> Result<TokenKind, QueryProblem> {
        self.bump();
        let mut name = String::new();
        loop {
            match self.bump() {
                None => return Err(QueryProblem::syntax(keys::UNCLOSED_NAME, start)),
                Some('`') => return Ok(TokenKind::Quoted(name)),
                Some(c) => name.push(c),
            }
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
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '@'
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '@'
}

fn is_keyword(word: &str) -> bool {
    KEYWORDS
        .iter()
        .any(|keyword| word.eq_ignore_ascii_case(keyword))
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
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            lexer: Lexer::new(text),
            peeked: None,
        }
    }

    fn next(&mut self) -> Result<Option<Token>, QueryProblem> {
        match self.peeked.take() {
            Some(token) => Ok(token),
            None => self.lexer.next(Mode::Normal),
        }
    }

    /// The next word where a value is expected.
    fn next_value(&mut self) -> Result<Option<Token>, QueryProblem> {
        match self.peeked.take() {
            Some(token) => Ok(token),
            None => self.lexer.next(Mode::Value),
        }
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
        Ok(self.peek()?.is_some_and(
            |token| matches!(&token.kind, TokenKind::Word(word) if word.eq_ignore_ascii_case(keyword)),
        ))
    }

    /// Where the text ended.
    fn end(&self) -> Span {
        self.lexer.here()
    }

    /// One command (U1:BR3.2).
    fn command(&mut self, token: Token) -> Result<Condition, QueryProblem> {
        let TokenKind::Word(word) = &token.kind else {
            return Err(unexpected(token));
        };
        if word.eq_ignore_ascii_case("filter") {
            if self.peek()?.is_none() || self.peek_is(&TokenKind::Pipe)? {
                return Err(QueryProblem::syntax(keys::MISSING_CONDITION, token.span));
            }
            return self.or_condition();
        }
        let lower = word.to_ascii_lowercase();
        if OTHER_COMMANDS.contains(&lower.as_str()) {
            return Err(QueryProblem::unsupported(
                keys::UNSUPPORTED_COMMAND,
                word.clone(),
                token.span,
            ));
        }
        Err(QueryProblem::syntax(keys::UNKNOWN_COMMAND, token.span).with_word(word.clone()))
    }

    fn or_condition(&mut self) -> Result<Condition, QueryProblem> {
        let mut left = self.and_condition()?;
        while self.peek_keyword("or")? {
            self.next()?;
            let right = self.and_condition()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and_condition(&mut self) -> Result<Condition, QueryProblem> {
        let mut left = self.not_condition()?;
        while self.peek_keyword("and")? {
            self.next()?;
            let right = self.not_condition()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn not_condition(&mut self) -> Result<Condition, QueryProblem> {
        if self.peek_keyword("not")? {
            self.next()?;
            return Ok(Condition::Not(Box::new(self.not_condition()?)));
        }
        self.primary_condition()
    }

    /// A parenthesized condition or one comparison of a field.
    fn primary_condition(&mut self) -> Result<Condition, QueryProblem> {
        let Some(token) = self.next()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        match &token.kind {
            TokenKind::LParen => {
                let inner = self.or_condition()?;
                match self.next()? {
                    Some(close) if close.kind == TokenKind::RParen => Ok(inner),
                    Some(close) if close.kind != TokenKind::Pipe => Err(unexpected(close)),
                    _ => Err(QueryProblem::syntax(keys::UNCLOSED_PAREN, token.span)),
                }
            }
            TokenKind::Word(word) if is_keyword(word) => Err(unexpected(token)),
            TokenKind::Word(_) | TokenKind::Quoted(_) => {
                let field = self.operand(token)?;
                self.predicate(field)
            }
            _ => Err(unexpected(token)),
        }
    }

    /// The field a condition reads: one of the fixed fields or
    /// `toMillis(@timestamp)` (U1:BR3.6, BR3.7).
    fn operand(&mut self, token: Token) -> Result<FieldRef, QueryProblem> {
        let word = match token.kind {
            TokenKind::Word(word) => word,
            TokenKind::Quoted(name) => {
                return Err(QueryProblem::unsupported(
                    keys::UNSUPPORTED_FIELD,
                    name,
                    token.span,
                ));
            }
            _ => return Err(unexpected(token)),
        };
        if self.peek_is(&TokenKind::LParen)? {
            return self.function(word, token.span);
        }
        field_named(&word)
            .ok_or_else(|| QueryProblem::unsupported(keys::UNSUPPORTED_FIELD, word, token.span))
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
            .is_some_and(|token| token.kind == TokenKind::Word("@timestamp".to_string()));
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
    fn predicate(&mut self, field: FieldRef) -> Result<Condition, QueryProblem> {
        let Some(operator) = self.next()? else {
            return Err(QueryProblem::syntax(keys::EXPECTED_OPERATOR, self.end()));
        };
        match &operator.kind {
            TokenKind::Compare(op) => Ok(Condition::Compare {
                field,
                op: *op,
                value: self.compare_value(field)?,
            }),
            TokenKind::Match => {
                if field.is_timestamp() {
                    return Err(timestamp_operator(operator));
                }
                Ok(Condition::Match {
                    field,
                    regex: self.match_value()?,
                })
            }
            TokenKind::Word(word) if word.eq_ignore_ascii_case("like") => {
                self.like(field, operator, false)
            }
            TokenKind::Word(word) if word.eq_ignore_ascii_case("in") => self.list(field, false),
            TokenKind::Word(word) if word.eq_ignore_ascii_case("not") => {
                match self.next()? {
                    Some(next) if matches!(&next.kind, TokenKind::Word(w) if w.eq_ignore_ascii_case("like")) => {
                        self.like(field, next, true)
                    }
                    Some(next) if matches!(&next.kind, TokenKind::Word(w) if w.eq_ignore_ascii_case("in")) => {
                        self.list(field, true)
                    }
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

    /// The value of a comparison (U1:BR5.1); a text compared with the
    /// timestamp is a date text (U1:BR5.9).
    fn compare_value(&mut self, field: FieldRef) -> Result<Literal, QueryProblem> {
        let Some(token) = self.next_value()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        self.scalar(field, token)
    }

    /// One string or number value of a comparison or a list.
    fn scalar(&mut self, field: FieldRef, token: Token) -> Result<Literal, QueryProblem> {
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
            TokenKind::Word(_) | TokenKind::Quoted(_) => Err(self.word_value(token)?),
            _ => Err(QueryProblem::syntax(keys::EXPECTED_VALUE, token.span).with_word(token.raw)),
        }
    }

    /// A bare word where a value is expected: a function call or another
    /// field, neither of which U1 runs.
    fn word_value(&mut self, token: Token) -> Result<QueryProblem, QueryProblem> {
        let name = match token.kind {
            TokenKind::Word(word) | TokenKind::Quoted(word) => word,
            _ => token.raw,
        };
        let key = if self.peek_is(&TokenKind::LParen)? {
            keys::UNSUPPORTED_FUNCTION
        } else {
            keys::UNSUPPORTED_FIELD_COMPARISON
        };
        Ok(QueryProblem::unsupported(key, name, token.span))
    }

    /// The regular expression after `=~` (U1:BR5.4).
    fn match_value(&mut self) -> Result<QueryRegex, QueryProblem> {
        let Some(token) = self.next_value()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        match token.kind {
            TokenKind::Regex(source) => compile(&source, token.span),
            TokenKind::Word(_) | TokenKind::Quoted(_) => Err(self.word_value(token)?),
            _ => Err(QueryProblem::syntax(keys::MATCH_NEEDS_REGEX, token.span)),
        }
    }

    /// `like` / `not like` (U1:BR5.3, BR5.4, BR5.6).
    fn like(
        &mut self,
        field: FieldRef,
        operator: Token,
        negated: bool,
    ) -> Result<Condition, QueryProblem> {
        if field.is_timestamp() {
            return Err(timestamp_operator(operator));
        }
        let Some(token) = self.next_value()? else {
            return Err(QueryProblem::syntax(keys::UNEXPECTED_END, self.end()));
        };
        let pattern = match token.kind {
            TokenKind::Text(text) => LikePattern::Text(text),
            TokenKind::Regex(source) => LikePattern::Regex(compile(&source, token.span)?),
            TokenKind::Word(_) | TokenKind::Quoted(_) => return Err(self.word_value(token)?),
            _ => return Err(QueryProblem::syntax(keys::LIKE_NEEDS_TEXT, token.span)),
        };
        Ok(Condition::Like {
            field,
            pattern,
            negated,
        })
    }

    /// `in [values]` / `not in [values]` (U1:BR3.5, BR5.5).
    fn list(&mut self, field: FieldRef, negated: bool) -> Result<Condition, QueryProblem> {
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
            values.push(self.scalar(field, value)?);
            match self.next()? {
                None => return Err(unclosed()),
                Some(token) if token.kind == TokenKind::Comma => {}
                Some(token) if token.kind == TokenKind::RBracket => break,
                Some(token) if token.kind == TokenKind::Pipe => return Err(unclosed()),
                Some(token) => return Err(unexpected(token)),
            }
        }
        Ok(Condition::In {
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
        CompareOp, Condition, FieldRef, LikePattern, Literal, ProblemKind, QueryRegex,
    };

    fn plan(text: &str) -> QueryPlan {
        match parse_query(text) {
            Ok(plan) => plan,
            Err(problem) => panic!("{text:?} was not read: {problem:?}"),
        }
    }

    fn only(text: &str) -> Condition {
        let mut filters = plan(text).filters;
        assert_eq!(filters.len(), 1, "{text:?}");
        filters.remove(0)
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
        Condition::Compare { field, op, value }
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
        Condition::And(Box::new(left), Box::new(right))
    }

    fn or(left: Condition, right: Condition) -> Condition {
        Condition::Or(Box::new(left), Box::new(right))
    }

    fn not(inner: Condition) -> Condition {
        Condition::Not(Box::new(inner))
    }

    #[test]
    fn commands_are_split_by_pipes_and_notes_are_skipped() {
        // U1:BR3.1, BR3.2: blanks and line breaks only separate words; `#`
        // starts a note outside quotes; two filters both stay.
        let read = plan(
            "filter @message = \"a # not a note\" # a note | stats\n\
             | filter @logStream = 's'",
        );
        assert_eq!(
            read.filters,
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
        // U1:BR3.2: the console's commands are named; anything else is a
        // writing mistake.
        let stats = problem("filter @message like /a/\n| stats count(*) by bin(5m)");
        assert_eq!(stats.kind, ProblemKind::Unsupported);
        assert_eq!(stats.message_key, keys::UNSUPPORTED_COMMAND);
        assert_eq!(stats.word.as_deref(), Some("stats"));
        assert_eq!(stats.span, at(2, 3, 5));
        for command in ["fields", "sort", "limit", "parse", "dedup", "SOURCE"] {
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
    fn command_names_and_words_ignore_case_but_field_names_do_not() {
        // U1:BR3.3 (provisional), BR3.6.
        assert_eq!(
            only("FILTER @message LIKE /a/ AND NOT @logStream IN [\"x\"]"),
            and(
                Condition::Like {
                    field: FieldRef::Message,
                    pattern: LikePattern::Regex(regex("a")),
                    negated: false,
                },
                not(Condition::In {
                    field: FieldRef::LogStream,
                    values: vec![Literal::Text("x".to_string())],
                    negated: false,
                }),
            )
        );
        assert_eq!(
            only("Filter @log Not Like 'x' Or @log not in [1]"),
            or(
                Condition::Like {
                    field: FieldRef::Log,
                    pattern: LikePattern::Text("x".to_string()),
                    negated: true,
                },
                Condition::In {
                    field: FieldRef::Log,
                    values: vec![Literal::Number(1.0)],
                    negated: true,
                },
            )
        );
        let wrong_case = problem("filter @Message = \"a\"");
        assert_eq!(wrong_case.message_key, keys::UNSUPPORTED_FIELD);
        assert_eq!(wrong_case.word.as_deref(), Some("@Message"));
    }

    #[test]
    fn not_binds_tighter_than_and_which_binds_tighter_than_or() {
        // U1:BR3.4 (provisional).
        let a = || message_is("a");
        let b = || message_is("b");
        let c = || message_is("c");
        assert_eq!(
            only("filter not @message = 'a' and @message = 'b'"),
            and(not(a()), b())
        );
        assert_eq!(
            only("filter @message = 'a' or @message = 'b' and @message = 'c'"),
            or(a(), and(b(), c()))
        );
        assert_eq!(
            only("filter (@message = 'a' or @message = 'b') and @message = 'c'"),
            and(or(a(), b()), c())
        );
        assert_eq!(only("filter not (not @message = 'a')"), not(not(a())));
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
            Condition::Like {
                field: FieldRef::Message,
                pattern: LikePattern::Regex(regex(r"a/b\d")),
                negated: false,
            }
        );
        assert_eq!(
            only("filter @logStream in ['a', 2, \"c\"]"),
            Condition::In {
                field: FieldRef::LogStream,
                values: vec![
                    Literal::Text("a".to_string()),
                    Literal::Number(2.0),
                    Literal::Text("c".to_string()),
                ],
                negated: false,
            }
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

    #[test]
    fn only_the_four_fixed_fields_can_be_read() {
        // U1:BR3.6; JSON fields come with U2.
        for (text, field) in [
            ("filter @timestamp > 1", FieldRef::Timestamp),
            ("filter @message > 'a'", FieldRef::Message),
            ("filter @logStream > 'a'", FieldRef::LogStream),
            ("filter @log > 'a'", FieldRef::Log),
        ] {
            let Condition::Compare { field: read, .. } = only(text) else {
                panic!("{text:?} is no comparison");
            };
            assert_eq!(read, field, "{text:?}");
        }
        for (text, word, span) in [
            ("filter @ingestionTime > 1", "@ingestionTime", at(1, 8, 14)),
            ("filter level = 'error'", "level", at(1, 8, 5)),
            ("filter `my field` = 'a'", "my field", at(1, 8, 10)),
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
            (
                "filter @message = @logStream",
                keys::UNSUPPORTED_FIELD_COMPARISON,
                "@logStream",
                at(1, 19, 10),
            ),
        ] {
            let found = problem(text);
            assert_eq!(found.kind, ProblemKind::Unsupported, "{text:?}");
            assert_eq!(found.message_key, key, "{text:?}");
            assert_eq!(found.word.as_deref(), Some(word), "{text:?}");
            assert_eq!(found.span, span, "{text:?}");
        }
    }

    #[test]
    fn operators_that_do_not_fit_the_field_or_value_are_refused() {
        // U1:BR5.1, BR5.4, BR5.6, BR5.7.
        assert_eq!(
            only("filter @message =~ /a+/"),
            Condition::Match {
                field: FieldRef::Message,
                regex: regex("a+"),
            }
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
        let found = problem("filter @message = \"エラー\"\n  and ログ = 'a'");
        assert_eq!(found.word.as_deref(), Some("ログ"));
        assert_eq!(found.span, at(2, 7, 2));
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
}
