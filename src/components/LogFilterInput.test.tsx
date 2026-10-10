import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { FILTER_DELAY_MS, LogFilterInput } from "./LogFilterInput";

/** The parent keeps the text, as App does. */
function Harness({
  onFilterChange,
  disabled = false,
  appliedText = "",
  failureCount = 0,
  locale = "en",
}: {
  onFilterChange: (text: string) => void;
  disabled?: boolean;
  appliedText?: string;
  failureCount?: number;
  locale?: "en" | "ja";
}) {
  const [value, setValue] = useState(appliedText);
  return (
    <LogFilterInput
      value={value}
      appliedText={appliedText}
      disabled={disabled}
      failureCount={failureCount}
      t={locale === "en" ? t : createTranslator("ja")}
      onChange={setValue}
      onFilterChange={onFilterChange}
    />
  );
}

function type(text: string) {
  fireEvent.change(screen.getByTestId("log-filter-input"), { target: { value: text } });
}

function wait(ms: number) {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

describe("LogFilterInput", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("hands on only the last text, 0.3 s after typing stops", () => {
    const onFilterChange = vi.fn();
    render(<Harness onFilterChange={onFilterChange} />);
    type("e");
    wait(200);
    type("er");
    wait(200);
    type("err");
    wait(FILTER_DELAY_MS - 1);
    expect(onFilterChange).not.toHaveBeenCalled();
    wait(1);
    expect(onFilterChange).toHaveBeenCalledTimes(1);
    expect(onFilterChange).toHaveBeenCalledWith("err");
    expect(screen.getByTestId("log-filter-input")).toHaveValue("err");
  });

  it("hands on clearing too, and nothing when the text comes back unchanged", () => {
    const onFilterChange = vi.fn();
    render(<Harness onFilterChange={onFilterChange} appliedText="error" />);
    expect(screen.getByTestId("log-filter-input")).toHaveValue("error");
    wait(FILTER_DELAY_MS);
    expect(onFilterChange).not.toHaveBeenCalled();
    type("");
    wait(FILTER_DELAY_MS);
    expect(onFilterChange).toHaveBeenLastCalledWith("");
    type("x");
    wait(100);
    type("");
    wait(FILTER_DELAY_MS);
    expect(onFilterChange).toHaveBeenCalledTimes(1);
  });

  it("does not submit the form with Enter and can be disabled", () => {
    const onSubmit = vi.fn((event: { preventDefault: () => void }) => event.preventDefault());
    const { rerender } = render(
      <form onSubmit={onSubmit}>
        <Harness onFilterChange={vi.fn()} />
      </form>,
    );
    const input = screen.getByTestId("log-filter-input");
    expect(input).toBeEnabled();
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onSubmit).not.toHaveBeenCalled();
    rerender(
      <form onSubmit={onSubmit}>
        <Harness onFilterChange={vi.fn()} disabled />
      </form>,
    );
    expect(screen.getByTestId("log-filter-input")).toBeDisabled();
  });

  it("has an English and a Japanese label and placeholder", () => {
    const { unmount } = render(<Harness onFilterChange={vi.fn()} />);
    expect(screen.getByRole("searchbox", { name: "Filter logs" })).toHaveAttribute(
      "placeholder",
      "Part of a message (case-insensitive)",
    );
    unmount();
    render(<Harness onFilterChange={vi.fn()} locale="ja" />);
    expect(screen.getByRole("searchbox", { name: "ログの絞り込み" })).toHaveAttribute(
      "placeholder",
      "メッセージの一部（大文字・小文字を区別しない）",
    );
  });

  it("hands the text on at once with Enter, and only once", () => {
    const onFilterChange = vi.fn();
    render(<Harness onFilterChange={onFilterChange} />);
    type("timeout");
    fireEvent.keyDown(screen.getByTestId("log-filter-input"), { key: "Enter" });
    expect(onFilterChange).toHaveBeenCalledTimes(1);
    expect(onFilterChange).toHaveBeenCalledWith("timeout");
    wait(FILTER_DELAY_MS);
    fireEvent.keyDown(screen.getByTestId("log-filter-input"), { key: "Enter" });
    expect(onFilterChange).toHaveBeenCalledTimes(1);
  });

  it("sends the text again after a refused hand-over", () => {
    const onFilterChange = vi.fn();
    const { rerender } = render(<Harness onFilterChange={onFilterChange} />);
    type("error");
    wait(FILTER_DELAY_MS);
    expect(onFilterChange).toHaveBeenCalledTimes(1);
    // The core refused it: it still holds "" while the field shows "error".
    rerender(<Harness onFilterChange={onFilterChange} failureCount={1} />);
    expect(onFilterChange).toHaveBeenCalledTimes(2);
    expect(onFilterChange).toHaveBeenLastCalledWith("error");
  });

  it("sends the text again when a dialog that disabled the field closes", () => {
    const onFilterChange = vi.fn();
    const { rerender } = render(<Harness onFilterChange={onFilterChange} appliedText="" />);
    type("warn");
    wait(FILTER_DELAY_MS);
    rerender(<Harness onFilterChange={onFilterChange} appliedText="" disabled />);
    expect(onFilterChange).toHaveBeenCalledTimes(1);
    rerender(<Harness onFilterChange={onFilterChange} appliedText="" />);
    expect(onFilterChange).toHaveBeenCalledTimes(2);
    expect(onFilterChange).toHaveBeenLastCalledWith("warn");
    // When the core already holds the text, nothing is sent.
    rerender(<Harness onFilterChange={onFilterChange} appliedText="warn" disabled />);
    rerender(<Harness onFilterChange={onFilterChange} appliedText="warn" />);
    expect(onFilterChange).toHaveBeenCalledTimes(2);
  });
});
