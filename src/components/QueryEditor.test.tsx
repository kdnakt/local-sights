import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  QUERY_EXAMPLES,
  completionItems,
  highlightRules,
  highlightTokens,
  offsetOf,
  problemRange,
} from "../analytics";
import type { CompletionCandidate, QueryProblem } from "../api";
import { QUERY_CATALOG, t } from "../test/fixtures";
import { PLAIN_LANGUAGE, PROBLEM_MARK_CLASS, QueryEditor } from "./QueryEditor";

/** The rules the page makes from the core's catalog (261025-insights-stats U2:BR4.2). */
const QUERY_HIGHLIGHT_RULES = highlightRules(QUERY_CATALOG);

const { registerQueryMode } = vi.hoisted(() => ({ registerQueryMode: vi.fn() }));

/** Stands in for Ace's Range: the four numbers only. */
const { FakeRange } = vi.hoisted(() => ({
  FakeRange: class {
    constructor(
      public startRow: number,
      public startColumn: number,
      public endRow: number,
      public endColumn: number,
    ) {}
  },
}));

/** Ace's config as far as completion uses it: modules are there at once. */
const fakeConfig = {
  loadModule: (_name: string, onLoad: () => void) => onLoad(),
  setModuleLoader: () => undefined,
};

vi.mock("../aceLoader", () => ({
  loadAce: () => Promise.resolve({ Range: FakeRange, config: fakeConfig }),
  registerQueryMode,
}));
vi.mock("@cloudscape-design/components/code-editor", () => import("../test/codeEditorMock"));

const problem: QueryProblem = {
  kind: "Unsupported",
  messageKey: "query.unsupported.command",
  word: "stats",
  span: { line: 2, column: 3, length: 5 },
};

function renderEditor(props: Partial<Parameters<typeof QueryEditor>[0]> = {}) {
  const handlers = { onChange: vi.fn(), onRun: vi.fn() };
  const view = render(
    <QueryEditor
      value={"filter @message like /a/\n| stats count(*)"}
      readOnly={false}
      problem={null}
      focusRequest={0}
      mac={false}
      t={t}
      {...handlers}
      {...props}
    />,
  );
  return { ...handlers, ...view };
}

const field = () => screen.getByRole("textbox", { name: "Query" });

describe("QueryEditor (261017-insights-query U1:BR1.6, BR3.9, interaction-spec QueryEditor)", () => {
  it("runs with Ctrl+Enter outside macOS, never with Enter alone or Cmd+Enter", () => {
    const { onRun } = renderEditor({ mac: false });
    fireEvent.keyDown(field(), { key: "Enter" });
    fireEvent.keyDown(field(), { key: "Enter", metaKey: true });
    fireEvent.keyDown(field(), { key: "Enter", ctrlKey: true, shiftKey: true });
    expect(onRun).not.toHaveBeenCalled();
    fireEvent.keyDown(field(), { key: "Enter", ctrlKey: true });
    expect(onRun).toHaveBeenCalledTimes(1);
  });

  it("runs with Cmd+Enter on macOS", () => {
    const { onRun } = renderEditor({ mac: true });
    fireEvent.keyDown(field(), { key: "Enter", ctrlKey: true });
    expect(onRun).not.toHaveBeenCalled();
    fireEvent.keyDown(field(), { key: "Enter", metaKey: true });
    expect(onRun).toHaveBeenCalledTimes(1);
  });

  it("keeps Tab away from the editor so the focus moves on (no keyboard trap)", () => {
    renderEditor();
    const seen = vi.fn();
    field().addEventListener("keydown", seen);
    fireEvent.keyDown(field(), { key: "Tab" });
    expect(seen).not.toHaveBeenCalled();
    fireEvent.keyDown(field(), { key: "a" });
    expect(seen).toHaveBeenCalledTimes(1);
  });

  it("reports changes, but not while the text is read-only (U1:BR2.4)", () => {
    const writable = renderEditor();
    fireEvent.change(field(), { target: { value: "filter @log = 'g'" } });
    expect(writable.onChange).toHaveBeenCalledWith("filter @log = 'g'");
    writable.unmount();
    const locked = renderEditor({ readOnly: true });
    fireEvent.change(field(), { target: { value: "filter @log = 'h'" } });
    expect(locked.onChange).not.toHaveBeenCalled();
  });

  it("takes the focus with the cursor at the problem on each new request (U1:BR3.9)", async () => {
    const { rerender, onChange, onRun } = renderEditor();
    expect(field()).not.toHaveFocus();
    rerender(
      <QueryEditor
        value={"filter @message like /a/\n| stats count(*)"}
        readOnly={false}
        problem={problem}
        focusRequest={1}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />,
    );
    await act(async () => undefined);
    expect(field()).toHaveFocus();
    const input = field() as HTMLTextAreaElement;
    expect(input.selectionStart).toBe(offsetOf(input.value, 2, 3));
    expect(input.value.slice(input.selectionStart ?? 0, (input.selectionStart ?? 0) + 5)).toBe(
      "stats",
    );
  });

  it("finds a line and a column counted in characters (pure)", () => {
    expect(offsetOf("abc\ndef", 1, 1)).toBe(0);
    expect(offsetOf("abc\ndef", 2, 2)).toBe(5);
    expect(offsetOf("日本\n語x", 2, 2)).toBe(4);
    // A surrogate pair is one character.
    expect(offsetOf("😀a", 1, 2)).toBe(2);
    expect(offsetOf("ab", 9, 9)).toBe(2);
  });
});

/**
 * Puts a stand-in for the Ace editor CodeEditor makes into the field, as
 * the real one would be found (`.ace_editor` with `env.editor`).
 */
function attachAce() {
  const session = {
    addMarker: vi.fn(() => 7),
    removeMarker: vi.fn(),
    setAnnotations: vi.fn(),
    clearAnnotations: vi.fn(),
  };
  const editor = {
    focus: vi.fn(),
    gotoLine: vi.fn(),
    setReadOnly: vi.fn(),
    setOptions: vi.fn(),
    completers: undefined as unknown,
    session,
  };
  const element = document.createElement("div");
  element.className = "ace_editor";
  Object.assign(element, { env: { editor } });
  screen.getByTestId("query-editor").appendChild(element);
  return Object.assign(session, { editor });
}

describe("QueryEditor's problem marks (261017-insights-query U2:BR6.1)", () => {
  const text = "fields @message\n| stats count(*)";

  it("underlines the problem's words and marks their line with the same words", async () => {
    const { rerender, onChange, onRun } = renderEditor({ value: text });
    const session = attachAce();
    await act(async () => undefined);
    rerender(
      <QueryEditor
        value={text}
        readOnly={false}
        problem={problem}
        problemMessage='Line 2, column 3: "stats" is not supported in this version.'
        focusRequest={0}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />,
    );
    await act(async () => undefined);
    expect(session.addMarker).toHaveBeenCalledWith(
      new FakeRange(1, 2, 1, 7),
      PROBLEM_MARK_CLASS,
      "text",
    );
    expect(session.setAnnotations).toHaveBeenLastCalledWith([
      {
        row: 1,
        column: 2,
        text: 'Line 2, column 3: "stats" is not supported in this version.',
        type: "error",
      },
    ]);
  });

  it("takes the underline and the mark away when the problem goes", async () => {
    const { rerender, onChange, onRun } = renderEditor({ value: text, problem });
    const session = attachAce();
    rerender(
      <QueryEditor
        value={text}
        readOnly={false}
        problem={{ ...problem }}
        focusRequest={0}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />,
    );
    await act(async () => undefined);
    expect(session.addMarker).toHaveBeenCalled();
    rerender(
      <QueryEditor
        value={`${text} `}
        readOnly={false}
        problem={null}
        focusRequest={0}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />,
    );
    await act(async () => undefined);
    expect(session.removeMarker).toHaveBeenCalledWith(7);
    expect(session.clearAnnotations).toHaveBeenCalled();
  });

  it("finds the editor's rows and UTF-16 columns of a span (pure)", () => {
    // See below.
    expect(problemRange("", { line: 1, column: 1, length: 1 })).toEqual({
      startRow: 0,
      startColumn: 0,
      endRow: 0,
      endColumn: 0,
    });
  });

  it("finds the rows and columns of longer spans (pure)", () => {
    expect(problemRange("filter (a", { line: 1, column: 8, length: 1 })).toEqual({
      startRow: 0,
      startColumn: 7,
      endRow: 0,
      endColumn: 8,
    });
    // A span over a line break, and characters outside the BMP.
    expect(problemRange("a😀b\ncd", { line: 1, column: 3, length: 3 })).toEqual({
      startRow: 0,
      startColumn: 3,
      endRow: 1,
      endColumn: 1,
    });
    expect(problemRange("ab", { line: 5, column: 9, length: 4 })).toEqual({
      startRow: 0,
      startColumn: 2,
      endRow: 0,
      endColumn: 2,
    });
  });
});

const CANDIDATES: CompletionCandidate[] = [
  { text: "fields", kind: "Command" },
  { text: "filter", kind: "Command" },
  { text: "like", kind: "Keyword" },
  { text: "@timestamp", kind: "FixedField" },
  { text: "level", kind: "JsonField" },
  { text: "`user.id`", kind: "JsonField" },
];

describe("QueryEditor's help in writing (261017-insights-query U2:BR9.1, BR9.2, Should)", () => {
  it("hands Ace a completer that offers the latest candidates, snippets off", async () => {
    const { rerender, onChange, onRun } = renderEditor({ candidates: [] });
    const session = attachAce();
    await act(async () => undefined);
    expect(session.editor.setOptions).toHaveBeenCalledWith({
      enableBasicAutocompletion: true,
      enableLiveAutocompletion: true,
      enableSnippets: false,
    });
    const [completer] = session.editor.completers as {
      getCompletions: (...args: unknown[]) => void;
      identifierRegexps: RegExp[];
    }[];
    const asked = (prefix: string) => {
      let items: { caption: string; meta: string }[] = [];
      completer!.getCompletions(null, null, null, prefix, (_: null, found: typeof items) => {
        items = found;
      });
      return items;
    };
    // No JSON field names before a fetch.
    expect(asked("le")).toEqual([]);
    rerender(
      <QueryEditor
        value=""
        readOnly={false}
        problem={null}
        candidates={CANDIDATES}
        focusRequest={0}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />,
    );
    await act(async () => undefined);
    expect(asked("le").map((item) => [item.caption, item.meta])).toEqual([["level", "JSON field"]]);
    expect(asked("FI").map((item) => item.caption)).toEqual(["fields", "filter"]);
    expect(completer!.identifierRegexps[0]!.test("@")).toBe(true);
    expect(completer!.identifierRegexps[0]!.test(".")).toBe(true);
  });

  it("lists candidates by prefix in the core's order with their kind (pure)", () => {
    const items = completionItems("", CANDIDATES, t);
    expect(items.map((item) => item.caption)).toEqual(CANDIDATES.map((c) => c.text));
    expect(items.map((item) => item.meta)).toEqual([
      "command",
      "command",
      "keyword",
      "field",
      "JSON field",
      "JSON field",
    ]);
    expect(items[0]!.score).toBeGreaterThan(items[5]!.score);
    expect(completionItems("@t", CANDIDATES, t).map((item) => item.value)).toEqual(["@timestamp"]);
    expect(completionItems("`u", CANDIDATES, t).map((item) => item.value)).toEqual(["`user.id`"]);
  });

  it("colours commands, fields, strings, numbers, expressions and operators apart (pure)", () => {
    const kinds = (line: string) =>
      highlightTokens(line, QUERY_HIGHLIGHT_RULES)
        .filter((token) => token.type !== "text")
        .map((token) => [token.type, token.value]);
    expect(kinds('FILTER level = "error" and http.status >= 500 # note')).toEqual([
      ["keyword", "FILTER"],
      ["variable", "level"],
      ["keyword.operator", "="],
      ["string", '"error"'],
      ["keyword.operator", "and"],
      ["variable", "http.status"],
      ["keyword.operator", ">="],
      ["constant.numeric", "500"],
      ["comment", "# note"],
    ]);
    expect(kinds("| sort @timestamp desc | limit 20")).toEqual([
      ["keyword.operator", "|"],
      ["keyword", "sort"],
      ["variable.language", "@timestamp"],
      ["keyword.operator", "desc"],
      ["keyword.operator", "|"],
      ["keyword", "limit"],
      ["constant.numeric", "20"],
    ]);
    expect(kinds("filter `user.id` like /a#b/ or filters = 'x'")).toEqual([
      ["keyword", "filter"],
      ["variable", "`user.id`"],
      ["keyword.operator", "like"],
      ["string.regexp", "/a#b/"],
      ["keyword.operator", "or"],
      ["variable", "filters"],
      ["keyword.operator", "="],
      ["string", "'x'"],
    ]);
  });

  it("builds the query's mode on Ace's text mode with the highlight rules", async () => {
    // U2:BR9.2: the loader registers it as ace/mode/logsinsights.
    const actual = await vi.importActual<typeof import("../aceLoader")>("../aceLoader");
    class TextMode {}
    class TextHighlightRules {
      $rules: unknown = { start: [] };
    }
    const ace = {
      require: (name: string) =>
        name === "ace/mode/text" ? { Mode: TextMode } : { TextHighlightRules },
    };
    const { Mode } = actual.queryMode(
      ace as unknown as Parameters<typeof actual.queryMode>[0],
      QUERY_HIGHLIGHT_RULES,
    );
    const mode = new Mode() as {
      $id: string;
      HighlightRules: new () => { $rules: { start: unknown[] } };
    };
    expect(mode).toBeInstanceOf(TextMode);
    expect(mode.$id).toBe("ace/mode/logsinsights");
    expect(new mode.HighlightRules().$rules.start).toEqual([...QUERY_HIGHLIGHT_RULES]);
  });

  it("stays plain text until the catalog's rules come, then registers the query's mode once (261025-insights-stats U2:BR4.2)", async () => {
    registerQueryMode.mockClear();
    const { rerender, onChange, onRun } = renderEditor();
    await act(async () => undefined);
    expect(field()).toHaveAttribute("data-language", PLAIN_LANGUAGE);
    expect(registerQueryMode).not.toHaveBeenCalled();
    const withRules = (rules: typeof QUERY_HIGHLIGHT_RULES) => (
      <QueryEditor
        value=""
        readOnly={false}
        problem={null}
        highlightRules={rules}
        focusRequest={0}
        mac={false}
        t={t}
        onChange={onChange}
        onRun={onRun}
      />
    );
    rerender(withRules(QUERY_HIGHLIGHT_RULES));
    await act(async () => undefined);
    expect(registerQueryMode).toHaveBeenCalledTimes(1);
    expect(registerQueryMode.mock.calls[0]?.[1]).toBe(QUERY_HIGHLIGHT_RULES);
    expect(field()).toHaveAttribute("data-language", "logsinsights");
    rerender(withRules(QUERY_HIGHLIGHT_RULES));
    await act(async () => undefined);
    expect(registerQueryMode).toHaveBeenCalledTimes(1);
  });

  it("has examples the query language reads: only its four commands (U2:BR9.3)", () => {
    expect(QUERY_EXAMPLES.length).toBeGreaterThanOrEqual(5);
    for (const example of QUERY_EXAMPLES) {
      const commands = example.text
        .split("|")
        .map((part) => part.trim().split(/\s/)[0]?.toLowerCase());
      for (const command of commands) {
        expect(["fields", "filter", "sort", "limit"], example.id).toContain(command);
      }
      expect(t(example.labelKey), example.id).not.toBe(example.labelKey);
    }
  });
});
