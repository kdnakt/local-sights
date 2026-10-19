/**
 * Pure helpers of the Log Analytics screen (261017-insights-query U1): the
 * two pages the navigation switches between (U1:BR1.1), the words of a
 * reading problem (U1:BR3.9) and the place of a problem in the query text.
 * Since U2 the range of a problem in the editor's own positions (U2:BR6.1)
 * and the first line of a cell (U2:BR7.1). Since U3 the choice of up to 50
 * log groups (U3:BR4.1).
 */

import type { CompletionCandidate, CompletionKind, QueryProblem, QuerySpan } from "./api";
import type { Translate } from "./i18n/messages";

/** The two screens of the app (FR1.1). */
export type AppPage = "logManagement" | "logAnalytics";

/** The pages in the order of the navigation and of Ctrl/Cmd+1, +2. */
export const APP_PAGES: readonly AppPage[] = ["logManagement", "logAnalytics"];

/** Message key of each page's name. */
export const PAGE_NAME_KEY: Record<AppPage, string> = {
  logManagement: "nav.logManagement",
  logAnalytics: "nav.logAnalytics",
};

/**
 * The words of a reading problem: the line and column, then what is wrong,
 * and for an unsupported command the commands this version runs (U1:BR3.9,
 * refined-mockups screen 7). Only the query's own word is put in (FR6.4).
 */
export function problemText(problem: QueryProblem, t: Translate): string {
  const place = t("insights.error.at", {
    line: problem.span.line,
    column: problem.span.column,
  });
  const words = t(problem.messageKey, { word: problem.word ?? "" });
  const supported =
    problem.messageKey === "query.unsupported.command"
      ? ` ${t("insights.error.supportedList")}`
      : "";
  return `${place}${words}${supported}`;
}

/**
 * The UTF-16 offset of `line` and `column` (both from 1, counted in
 * characters as the core counts them) in `text`, for a text field's
 * selection; past the end it is the end.
 */
export function offsetOf(text: string, line: number, column: number): number {
  const characters = Array.from(text);
  let currentLine = 1;
  let index = 0;
  while (index < characters.length && currentLine < line) {
    if (characters[index] === "\n") {
      currentLine += 1;
    }
    index += 1;
  }
  const characterIndex = Math.min(index + Math.max(column, 1) - 1, characters.length);
  return characters.slice(0, characterIndex).join("").length;
}

/** Where a problem's underline runs, in Ace's terms: rows from 0, UTF-16 columns. */
export interface ProblemRange {
  startRow: number;
  startColumn: number;
  endRow: number;
  endColumn: number;
}

/**
 * U2:BR6.1: the range of `span` (line and column from 1 and a length, all
 * in characters as the core counts them, line breaks included) in `text`,
 * in the editor's rows and UTF-16 columns. A span past the end stops there.
 */
export function problemRange(text: string, span: QuerySpan): ProblemRange {
  const lines = text.split("\n").map((line) => Array.from(line));
  const clampRow = (row: number) => Math.min(Math.max(row, 0), Math.max(lines.length - 1, 0));
  const utf16 = (row: number, characters: number) =>
    (lines[row] ?? []).slice(0, characters).join("").length;
  const startRow = clampRow(span.line - 1);
  let row = startRow;
  let character = Math.min(Math.max(span.column - 1, 0), lines[row]?.length ?? 0);
  const startColumn = utf16(row, character);
  for (let left = Math.max(span.length, 1); left > 0; left -= 1) {
    const length = lines[row]?.length ?? 0;
    if (character < length) {
      character += 1;
    } else if (row + 1 < lines.length) {
      row += 1;
      character = 0;
    } else {
      break;
    }
  }
  return { startRow, startColumn, endRow: row, endColumn: utf16(row, character) };
}

/** The language name the query field gives CodeEditor (`ace/mode/<name>`, U2:BR9.2). */
export const QUERY_LANGUAGE = "logsinsights";

/** One rule of the query's syntax colours: an Ace token name and its pattern. */
export interface HighlightRule {
  token: string;
  regex: string;
  caseInsensitive?: boolean;
}

/**
 * U2:BR9.2 (Should): the syntax colours of the query field, as Ace highlight
 * rules. Commands, fields, strings, numbers, regular expressions and
 * operators get tokens of their own, which the editor's light and dark
 * themes colour; the colours only help reading, a problem is always told in
 * words too (U2:BR6.1). At one place the first rule that matches wins, so
 * the words come before the JSON names they would also match.
 */
export const QUERY_HIGHLIGHT_RULES: readonly HighlightRule[] = [
  { token: "comment", regex: "#.*$" },
  { token: "string", regex: '"(?:[^"\\\\]|\\\\.)*"?' },
  { token: "string", regex: "'(?:[^'\\\\]|\\\\.)*'?" },
  { token: "string.regexp", regex: "/(?:[^/\\\\]|\\\\.)*/?" },
  {
    token: "keyword",
    regex: "\\b(?:fields|filter|sort|limit)\\b(?![.\\-])",
    caseInsensitive: true,
  },
  {
    token: "keyword.operator",
    regex: "\\b(?:and|or|not|like|in|asc|desc)\\b(?![.\\-])",
    caseInsensitive: true,
  },
  { token: "support.function", regex: "\\btoMillis\\b" },
  { token: "variable.language", regex: "@[A-Za-z]+" },
  { token: "variable", regex: "`[^`]*`?(?:\\.(?:`[^`]*`?|[\\w-]+))*" },
  {
    token: "variable",
    regex:
      "[A-Za-z_\\u00C0-\\uFFFF][\\w\\-\\u00C0-\\uFFFF]*(?:\\.(?:`[^`]*`?|[\\w\\-\\u00C0-\\uFFFF]+))*",
  },
  { token: "constant.numeric", regex: "-?\\d+(?:\\.\\d+)?" },
  { token: "keyword.operator", regex: "=~|!=|<=|>=|=|<|>|\\|" },
  { token: "paren", regex: "[()\\[\\],]" },
];

/** One coloured piece of a line. */
export interface HighlightToken {
  type: string;
  value: string;
}

/**
 * The pieces of `line` as {@link QUERY_HIGHLIGHT_RULES} colours them: at each
 * place the first rule that matches there, else one character of plain
 * text. Used by the tests; Ace runs the same rules itself.
 */
export function highlightTokens(line: string): HighlightToken[] {
  const rules = QUERY_HIGHLIGHT_RULES.map((rule) => ({
    token: rule.token,
    pattern: new RegExp(rule.regex, rule.caseInsensitive ? "iy" : "y"),
  }));
  const tokens: HighlightToken[] = [];
  const push = (type: string, value: string) => {
    const last = tokens[tokens.length - 1];
    if (last !== undefined && last.type === type && type === "text") {
      last.value += value;
    } else {
      tokens.push({ type, value });
    }
  };
  let index = 0;
  while (index < line.length) {
    let matched = false;
    for (const { token, pattern } of rules) {
      pattern.lastIndex = index;
      const found = pattern.exec(line);
      if (found !== null && found[0].length > 0) {
        push(token, found[0]);
        index += found[0].length;
        matched = true;
        break;
      }
    }
    if (!matched) {
      push("text", line[index] ?? "");
      index += 1;
    }
  }
  return tokens;
}

/** The message key naming each kind of completion candidate (U2:BR9.1). */
export const COMPLETION_KIND_KEY: Record<CompletionKind, string> = {
  Command: "insights.completion.command",
  Keyword: "insights.completion.keyword",
  FixedField: "insights.completion.field",
  JsonField: "insights.completion.jsonField",
};

/** One entry of the completion list as the editor shows it. */
export interface CompletionItem {
  caption: string;
  value: string;
  meta: string;
  score: number;
}

/**
 * U2:BR9.1 (Should): the candidates that start with `prefix` (any case), in
 * the core's order (commands, words, fixed fields, then the fetched JSON
 * fields), with their kind in words. Without a prefix, every candidate.
 */
export function completionItems(
  prefix: string,
  candidates: readonly CompletionCandidate[],
  t: Translate,
): CompletionItem[] {
  const wanted = prefix.toLowerCase();
  return candidates
    .filter((candidate) => candidate.text.toLowerCase().startsWith(wanted))
    .map((candidate, index) => ({
      caption: candidate.text,
      value: candidate.text,
      meta: t(COMPLETION_KIND_KEY[candidate.kind]),
      score: candidates.length - index,
    }));
}

/** One example query of the Examples list (U2:BR9.3). */
export interface QueryExample {
  id: string;
  /** Message key of its name. */
  labelKey: string;
  text: string;
}

/**
 * U2:BR9.3 (Should): examples of what this version runs: combined filters,
 * JSON fields, and fields with sort and limit (README's list).
 */
export const QUERY_EXAMPLES: readonly QueryExample[] = [
  {
    id: "errors",
    labelKey: "insights.examples.errors",
    text: "fields @timestamp, @message, @logStream, @log\n| filter @message like /ERROR/ and @message not like /timeout/\n| sort @timestamp desc\n| limit 100",
  },
  {
    id: "jsonLevel",
    labelKey: "insights.examples.jsonLevel",
    text: 'fields @timestamp, level, @message\n| filter level = "error" or level = "warn"\n| sort @timestamp desc\n| limit 100',
  },
  {
    id: "httpStatus",
    labelKey: "insights.examples.httpStatus",
    text: "fields @timestamp, http.status, http.path\n| filter http.status >= 500\n| sort http.status desc, @timestamp desc\n| limit 50",
  },
  {
    id: "excluded",
    labelKey: "insights.examples.excluded",
    text: 'fields @timestamp, errorCode, @message\n| filter errorCode not in ["AccessDenied", "ThrottlingException"]\n| limit 200',
  },
  {
    id: "stream",
    labelKey: "insights.examples.stream",
    text: "fields @timestamp, @logStream, @message\n| filter @logStream like /web/ and not (@message like /health/)\n| sort @timestamp asc\n| limit 1000",
  },
];

/** The first line of a text, as the result table shows a message (U2:BR7.1). */
export function firstLine(text: string): string {
  const end = text.search(/\r?\n/);
  return end < 0 ? text : text.slice(0, end);
}

/**
 * 261017-insights-query U3:BR4.1: the chosen log groups, each once, in the
 * order chosen, at most `max` (the 51st is not taken). Replaces U1's one
 * group that a new choice replaced.
 */
export function limitSelection(values: readonly string[], max: number): string[] {
  const chosen: string[] = [];
  for (const value of values) {
    if (value !== "" && !chosen.includes(value) && chosen.length < max) {
      chosen.push(value);
    }
  }
  return chosen;
}
