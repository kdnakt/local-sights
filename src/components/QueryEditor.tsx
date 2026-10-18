import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import CodeEditor, { type CodeEditorProps } from "@cloudscape-design/components/code-editor";
import { loadAce, type AceModule } from "../aceLoader";
import { offsetOf } from "../analytics";
import type { QueryProblem } from "../api";
import type { Translate } from "../i18n/messages";
import { isShortcutModifier } from "../platform";

/** The editor's height before the user drags it: about five lines (mockups screen 2). */
const INITIAL_HEIGHT = 96;

export interface QueryEditorProps {
  value: string;
  /** The text cannot be changed (Fetching, Evaluating, a dialog open; U1:BR2.4). */
  readOnly: boolean;
  /** The reading problem to point at, or `null`. */
  problem: QueryProblem | null;
  /** Increases each time the editor must take the focus at the problem (U1:BR3.9). */
  focusRequest: number;
  /** Whether the shortcut is Cmd (macOS) rather than Ctrl. */
  mac: boolean;
  t: Translate;
  onChange: (value: string) => void;
  /** Cmd/Ctrl+Enter; the parent decides whether a run can start (U1:BR1.6). */
  onRun: () => void;
}

/** The Ace editor CodeEditor made inside `container`, when there is one. */
interface AceEditorLike {
  focus: () => void;
  gotoLine: (line: number, column: number, animate: boolean) => void;
  setReadOnly: (readOnly: boolean) => void;
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
 * the problem's words are shown by the parent under the field. Syntax
 * colours, completion and the underline come with U2.
 */
export function QueryEditor({
  value,
  readOnly,
  problem,
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
        language="text"
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
