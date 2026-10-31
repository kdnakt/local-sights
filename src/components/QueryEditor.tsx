import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import CodeEditor, { type CodeEditorProps } from "@cloudscape-design/components/code-editor";
import { loadAce, registerQueryMode, type AceModule } from "../aceLoader";
import {
  QUERY_LANGUAGE,
  completionItems,
  offsetOf,
  problemRange,
  type CompletionItem,
  type HighlightRule,
  type ProblemRange,
} from "../analytics";
import type { CompletionCandidate, QueryProblem } from "../api";
import type { Translate } from "../i18n/messages";
import { isShortcutModifier } from "../platform";

/** The class of the underline Ace draws over a problem's words (styles.css). */
export const PROBLEM_MARK_CLASS = "query-problem-mark";

/** The editor's height before the user drags it: about five lines (mockups screen 2). */
const INITIAL_HEIGHT = 96;

/** Ace's plain text mode: the field before the query's colours can be made. */
export const PLAIN_LANGUAGE = "text";

/** No syntax colours (the catalog has not come). */
const NO_RULES: readonly HighlightRule[] = [];

export interface QueryEditorProps {
  value: string;
  /** The text cannot be changed (Fetching, Evaluating, a dialog open; U1:BR2.4). */
  readOnly: boolean;
  /** The reading problem to point at, or `null` (also once the text was changed, U2:BR6.1). */
  problem: QueryProblem | null;
  /** The words of the problem, put on the mark beside the line number. */
  problemMessage?: string;
  /** What Ctrl+Space and typing offer (U2:BR9.1, Should); JSON names only once fetched. */
  candidates?: readonly CompletionCandidate[];
  /**
   * The syntax colours, made from the core's QueryCatalog
   * (261025-insights-stats U2:BR4.2); none (plain text) until it has come.
   */
  highlightRules?: readonly HighlightRule[];
  /** Increases each time the editor must take the focus at the problem (U1:BR3.9). */
  focusRequest: number;
  /** Whether the shortcut is Cmd (macOS) rather than Ctrl. */
  mac: boolean;
  t: Translate;
  onChange: (value: string) => void;
  /** Cmd/Ctrl+Enter; the parent decides whether a run can start (U1:BR1.6). */
  onRun: () => void;
}

/** The parts of Ace's edit session the marks use. */
interface AceSessionLike {
  addMarker: (range: unknown, className: string, type: string, inFront?: boolean) => number;
  removeMarker: (id: number) => void;
  setAnnotations: (
    annotations: { row: number; column: number; text: string; type: string }[],
  ) => void;
  clearAnnotations: () => void;
}

/** What Ace's completion asks a completer (ext-language_tools). */
interface Completer {
  identifierRegexps: RegExp[];
  getCompletions: (
    editor: unknown,
    session: unknown,
    position: unknown,
    prefix: string,
    callback: (error: null, items: CompletionItem[]) => void,
  ) => void;
}

/** The Ace editor CodeEditor made inside `container`, when there is one. */
interface AceEditorLike {
  focus: () => void;
  gotoLine: (line: number, column: number, animate: boolean) => void;
  setReadOnly: (readOnly: boolean) => void;
  session?: AceSessionLike;
  setOptions?: (options: Record<string, boolean>) => void;
  completers?: Completer[];
}

/** The characters a completed word is made of: names, `@`, `.`, `-` and backquotes. */
const COMPLETION_WORD = /[\w@.`\-À-￿]/;

/** Ace's Range class, read from the loaded module when it has one. */
type RangeClass = new (
  startRow: number,
  startColumn: number,
  endRow: number,
  endColumn: number,
) => unknown;

function rangeOf(ace: AceModule | null, range: ProblemRange): unknown {
  const Range = (ace as { Range?: RangeClass } | null)?.Range;
  return Range === undefined
    ? null
    : new Range(range.startRow, range.startColumn, range.endRow, range.endColumn);
}

function aceEditorIn(container: HTMLElement | null): AceEditorLike | null {
  const element = container?.querySelector(".ace_editor") as
    (Element & { env?: { editor?: AceEditorLike } }) | null | undefined;
  return element?.env?.editor ?? null;
}

/**
 * The query field (261017-insights-query refined-mockups Q3, interaction-spec
 * QueryEditor): Cloudscape's CodeEditor on Ace, loaded when first shown,
 * with the visible label "Query", line numbers and a fixed look (its
 * settings are not taken, NFR11). Changing the text never runs the query
 * (U1:BR1.6): only Cmd+Enter on macOS or Ctrl+Enter elsewhere does, through
 * the parent. Tab is not taken by the editor for indenting: it moves on to
 * the next control, so the field is no keyboard trap. While the query runs
 * the text can be read but not changed. When reading the query failed, the
 * focus comes back to the field with the cursor at the problem (U1:BR3.9);
 * the problem's words are shown by the parent under the field. Since U2 the
 * problem's words are also underlined in the field and marked beside their
 * line number with the same words (U2:BR6.1); both go when the parent
 * stops passing the problem, as it does once the text is changed. The
 * query is coloured by its own mode (U2:BR9.2), and Ctrl+Space or typing a
 * word offers the commands, words, fixed fields and the fetched JSON field
 * names (U2:BR9.1); snippets are off. Both are Should and are skipped when
 * Ace lacks them.
 */
export function QueryEditor({
  value,
  readOnly,
  problem,
  problemMessage = "",
  candidates = [],
  highlightRules = NO_RULES,
  focusRequest,
  mac,
  t,
  onChange,
  onRun,
}: QueryEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [ace, setAce] = useState<AceModule | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [height, setHeight] = useState(INITIAL_HEIGHT);

  useEffect(() => {
    let disposed = false;
    loadAce().then(
      (module) => {
        if (!disposed) {
          setAce(module);
        }
      },
      () => {
        if (!disposed) {
          setLoadFailed(true);
        }
      },
    );
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    aceEditorIn(containerRef.current)?.setReadOnly(readOnly);
  }, [readOnly, ace]);

  // 261025-insights-stats U2:BR4.2: the query's mode is registered with the
  // catalog's rules once they have come (the page keeps them the same), and
  // before CodeEditor asks Ace for the mode; until then the field is plain
  // text. Since U3 (U2 code review R-05) the registration is an effect, not
  // part of the render: the field asks for the mode only once the effect has
  // registered it for these rules, a moment later.
  const [registeredRules, setRegisteredRules] = useState<readonly HighlightRule[] | null>(null);
  useEffect(() => {
    const config = (ace as { config?: Partial<AceModule["config"]> } | null)?.config;
    if (highlightRules.length === 0 || ace === null || !config?.setModuleLoader) {
      return undefined;
    }
    registerQueryMode(ace, highlightRules);
    let current = true;
    void Promise.resolve().then(() => {
      if (current) {
        setRegisteredRules(highlightRules);
      }
    });
    return () => {
      current = false;
    };
  }, [ace, highlightRules]);
  const coloured = ace !== null && highlightRules.length > 0 && registeredRules === highlightRules;

  // U2:BR9.1: the completer reads the latest candidates and words.
  const candidatesRef = useRef(candidates);
  const tRef = useRef(t);
  useEffect(() => {
    candidatesRef.current = candidates;
    tRef.current = t;
  }, [candidates, t]);
  useEffect(() => {
    const editor = aceEditorIn(containerRef.current);
    const config = (ace as { config?: Partial<AceModule["config"]> } | null)?.config;
    if (editor === null || config?.loadModule === undefined) {
      return;
    }
    const completer: Completer = {
      identifierRegexps: [COMPLETION_WORD],
      getCompletions: (_editor, _session, _position, prefix, callback) => {
        callback(null, completionItems(prefix, candidatesRef.current, tRef.current));
      },
    };
    config.loadModule("ace/ext/language_tools", () => {
      editor.setOptions?.({
        enableBasicAutocompletion: true,
        enableLiveAutocompletion: true,
        enableSnippets: false,
      });
      editor.completers = [completer];
    });
  }, [ace]);

  // U2:BR6.1: the underline and the mark of the problem, while it is shown.
  // The text the problem points into is the one it was found in, so the
  // range is taken from the value of that moment only.
  const valueRef = useRef(value);
  useEffect(() => {
    valueRef.current = value;
  }, [value]);
  useEffect(() => {
    const session = aceEditorIn(containerRef.current)?.session;
    if (problem === null || session === undefined) {
      return undefined;
    }
    const range = problemRange(valueRef.current, problem.span);
    const aceRange = rangeOf(ace, range);
    const marker =
      aceRange === null ? null : session.addMarker(aceRange, PROBLEM_MARK_CLASS, "text");
    session.setAnnotations([
      { row: range.startRow, column: range.startColumn, text: problemMessage, type: "error" },
    ]);
    return () => {
      if (marker !== null) {
        session.removeMarker(marker);
      }
      session.clearAnnotations();
    };
  }, [problem, problemMessage, ace]);

  // Only a new request moves the focus, not a re-render with the same problem.
  const problemRef = useRef(problem);
  useEffect(() => {
    problemRef.current = problem;
  }, [problem]);

  useEffect(() => {
    const target = problemRef.current;
    if (focusRequest === 0 || target === null) {
      return;
    }
    const { line, column } = target.span;
    const editor = aceEditorIn(containerRef.current);
    if (editor !== null) {
      editor.focus();
      editor.gotoLine(line, column - 1, false);
      return;
    }
    const field = containerRef.current?.querySelector("textarea");
    if (field) {
      field.focus();
      const offset = offsetOf(field.value, line, column);
      field.setSelectionRange(offset, offset);
    }
  }, [focusRequest]);

  const handleKeyDownCapture = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" && isShortcutModifier(event, mac)) {
      event.preventDefault();
      event.stopPropagation();
      onRun();
      return;
    }
    if (event.key === "Tab" && !event.ctrlKey && !event.metaKey && !event.altKey) {
      // Not for the editor: the browser moves the focus on (no trap).
      event.stopPropagation();
    }
  };

  const handleChange: CodeEditorProps["onChange"] = ({ detail }) => {
    if (!readOnly) {
      onChange(detail.value);
    }
  };

  return (
    <div
      ref={containerRef}
      className="query-editor"
      onKeyDownCapture={handleKeyDownCapture}
      data-testid="query-editor"
    >
      <CodeEditor
        ace={ace ?? undefined}
        value={value}
        language={coloured ? QUERY_LANGUAGE : PLAIN_LANGUAGE}
        languageLabel={t("insights.query.language")}
        onChange={handleChange}
        preferences={{ wrapLines: true }}
        onPreferencesChange={() => undefined}
        loading={ace === null && !loadFailed}
        editorContentHeight={height}
        onEditorContentResize={({ detail }) => setHeight(detail.height)}
        ariaLabel={t("insights.query.label")}
        i18nStrings={{
          loadingState: t("insights.query.loading"),
          errorState: t("insights.query.loadFailed"),
        }}
      />
    </div>
  );
}
