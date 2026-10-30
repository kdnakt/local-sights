// 261025-insights-stats U2:BR4.1, BR4.2: the screen's colours and the lists of
// a problem's words come from the core's QueryCatalog.
import { describe, expect, it } from "vitest";
import catalogRs from "../crates/local-sights-core/src/query/catalog.rs?raw";
import { highlightRules, highlightTokens, problemText } from "./analytics";
import type { QueryProblem } from "./api";
import { createTranslator } from "./i18n/messages";
import { QUERY_CATALOG, t } from "./test/fixtures";

const rules = highlightRules(QUERY_CATALOG);

/** The coloured pieces of `line`, plain text left out. */
function kinds(line: string, from = rules) {
  return highlightTokens(line, from)
    .filter((token) => token.type !== "text")
    .map((token) => [token.type, token.value]);
}

function problem(messageKey: string, word: string): QueryProblem {
  return { kind: "Unsupported", messageKey, word, span: { line: 1, column: 1, length: 1 } };
}

describe("the colouring rules made from the catalog (BR4.2)", () => {
  it("colours stats, its functions and its keywords", () => {
    expect(kinds("stats count_distinct(user) as n by bin(5m)")).toEqual([
      ["keyword", "stats"],
      ["support.function", "count_distinct"],
      ["paren", "("],
      ["variable", "user"],
      ["paren", ")"],
      ["keyword.operator", "as"],
      ["variable", "n"],
      ["keyword.operator", "by"],
      ["support.function", "bin"],
      ["paren", "("],
      ["constant.numeric", "5"],
      ["variable", "m"],
      ["paren", ")"],
    ]);
  });

  it("colours the commands this version does not run too, in any case", () => {
    expect(kinds("PARSE x | filterIndex y")).toEqual([
      ["keyword", "PARSE"],
      ["variable", "x"],
      ["keyword.operator", "|"],
      ["keyword", "filterIndex"],
      ["variable", "y"],
    ]);
  });

  it("colours a function only as the catalog writes it", () => {
    expect(kinds("sortsFirst(a) sortsfirst(b) toMillis(@timestamp)")).toEqual([
      ["support.function", "sortsFirst"],
      ["paren", "("],
      ["variable", "a"],
      ["paren", ")"],
      ["variable", "sortsfirst"],
      ["paren", "("],
      ["variable", "b"],
      ["paren", ")"],
      ["support.function", "toMillis"],
      ["paren", "("],
      ["variable.language", "@timestamp"],
      ["paren", ")"],
    ]);
  });

  it("makes no rule without the catalog, so the field stays plain text", () => {
    expect(highlightRules(null)).toEqual([]);
    expect(highlightRules([])).toEqual([]);
    expect(kinds("stats count(*) by level", highlightRules(null))).toEqual([]);
  });

  it("makes no rule of a kind the catalog has no word of", () => {
    const commandsOnly = QUERY_CATALOG.filter((word) => word.kind === "command");
    const tokens = kinds("stats count(*) by level", highlightRules(commandsOnly));
    expect(tokens[0]).toEqual(["keyword", "stats"]);
    expect(tokens).toContainEqual(["variable", "count"]);
    expect(tokens).toContainEqual(["variable", "by"]);
  });
});

describe("the lists in a problem's words (BR4.2)", () => {
  it("names the commands this version runs from the catalog, stats included", () => {
    expect(problemText(problem("query.unsupported.command", "parse"), t, QUERY_CATALOG)).toBe(
      'Line 1, column 1: "parse" is not supported in this version. Supported commands: fields, filter, sort, limit, stats.',
    );
    const ja = createTranslator("ja");
    expect(problemText(problem("query.unsupported.command", "parse"), ja, QUERY_CATALOG)).toBe(
      "1 行 1 列：「parse」にはこの版では対応していません。 使える命令：fields、filter、sort、limit、stats",
    );
  });

  it("names no list before the catalog has come", () => {
    expect(problemText(problem("query.unsupported.command", "parse"), t)).toBe(
      'Line 1, column 1: "parse" is not supported in this version.',
    );
    expect(problemText(problem("query.unsupported.function", "median"), t, null)).toBe(
      'Line 1, column 1: The function "median" is not supported here in this version.',
    );
  });

  it("names the functions this version runs for an unsupported function", () => {
    const words = problemText(problem("query.unsupported.function", "median"), t, QUERY_CATALOG);
    expect(words).toBe(
      'Line 1, column 1: The function "median" is not supported here in this version. ' +
        "Supported functions: count, count_distinct, sum, avg, min, max, pct, stddev, earliest, " +
        "latest, sortsFirst, sortsLast, toMillis, bin.",
    );
    const other = problemText(problem("query.syntax.unknownResultColumn", "m"), t, QUERY_CATALOG);
    expect(other).not.toContain("Supported");
  });
});

describe("the catalog fixture", () => {
  it("is the core's catalog.rs, word for word and in order", () => {
    // The VOCABULARY entries of catalog.rs: command("x", true), aggregate("x"),
    // word(TO_MILLIS, ...), keyword("x", reserved), fixed_field("x").
    const block = /const VOCABULARY[^=]*= \[([\s\S]*?)\n\];/.exec(catalogRs)?.[1] ?? "";
    const constants: Record<string, string> = { TO_MILLIS: "toMillis", BIN: "bin" };
    const entries = [
      ...block.matchAll(
        /(command|aggregate|keyword|fixed_field|word)\((?:"([^"]+)"|([A-Z_]+))(?:, (?:VocabularyKind::\w+, )?(true|false))?\)/g,
      ),
    ].map((match) => {
      const [, maker, quoted, constant, flag] = match;
      const name = quoted ?? constants[constant ?? ""] ?? "";
      const kind = {
        command: "command",
        aggregate: "aggregateFunction",
        keyword: "keyword",
        fixed_field: "fixedField",
        word: "function",
      }[maker ?? "word"];
      return {
        name,
        kind,
        supported: maker === "command" ? flag === "true" : true,
        reserved: maker === "keyword" ? flag === "true" : false,
      };
    });
    expect(entries).toHaveLength(42);
    expect(entries).toEqual([...QUERY_CATALOG]);
  });
});
