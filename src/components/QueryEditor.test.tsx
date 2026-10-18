import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { offsetOf } from "../analytics";
import type { QueryProblem } from "../api";
import { t } from "../test/fixtures";
import { QueryEditor } from "./QueryEditor";

vi.mock("../aceLoader", () => ({ loadAce: () => Promise.resolve({}) }));
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
