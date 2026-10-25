import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { t } from "../test/fixtures";
import { ExpandedMessage, type ExpandedMessageProps } from "./ExpandedMessage";

const JSON_MESSAGE = 'INFO {"a":1,"b":[true,null]} done';
const PRETTY = 'INFO {\n  "a": 1,\n  "b": [\n    true,\n    null\n  ]\n} done';

function renderExpanded(props: Partial<ExpandedMessageProps> = {}) {
  const onLeave = vi.fn();
  const onMeasure = vi.fn();
  const onGridKeyDown = vi.fn();
  const onGridClick = vi.fn();
  render(
    // The row and the grid around the expansion, listening like LogTable does.
    <div role="grid" onKeyDown={onGridKeyDown} onClick={onGridClick}>
      <div role="row">
        <ExpandedMessage
          message={JSON_MESSAGE}
          focusable
          onMeasure={onMeasure}
          onLeave={onLeave}
          label="Full message"
          t={t}
          {...props}
        />
      </div>
    </div>,
  );
  return { onLeave, onMeasure, onGridKeyDown, onGridClick };
}

const text = () => screen.getByTestId("log-table-expanded");
const toggle = () => screen.getByTestId("log-table-expanded-mode");

/** Whether the text is one text node only (never parsed as HTML). */
function isOneTextNode(element: HTMLElement): boolean {
  return element.childNodes.length === 1 && element.childNodes[0]?.nodeType === Node.TEXT_NODE;
}

describe("ExpandedMessage (261022-small-features U2:BR5.3, BR5.4, BR5.5)", () => {
  it("shows a message without JSON as it is, with no switch (FR5.6)", () => {
    renderExpanded({ message: "<b>plain</b> text\nsecond line" });
    expect(text().textContent).toBe("<b>plain</b> text\nsecond line");
    expect(text().querySelector("b")).toBeNull();
    expect(screen.queryByTestId("log-table-expanded-mode")).toBeNull();
    expect(screen.getByRole("region", { name: "Full message" })).toBe(text());
  });

  it("starts on Formatted and shows the prefix, the formatted JSON and the suffix", () => {
    renderExpanded();
    expect(text().textContent).toBe(PRETTY);
    const group = within(toggle()).getByRole("toolbar", { name: "Display" });
    expect(within(group).getByRole("button", { name: "Formatted" })).toBeInTheDocument();
    expect(within(group).getByRole("button", { name: "Original" })).toBeInTheDocument();
  });

  it("shows the message as returned on Original and comes back on Formatted", async () => {
    const user = userEvent.setup();
    renderExpanded();
    await user.click(within(toggle()).getByRole("button", { name: "Original" }));
    expect(text().textContent).toBe(JSON_MESSAGE);
    await user.click(within(toggle()).getByRole("button", { name: "Formatted" }));
    expect(text().textContent).toBe(PRETTY);
  });

  it("draws the shown text as one text node, so what is copied is what is shown (FR5.8)", async () => {
    const user = userEvent.setup();
    renderExpanded({ message: '<b>x</b> {"html":"<i>y</i>"}' });
    expect(isOneTextNode(text())).toBe(true);
    expect(text().querySelector("b, i")).toBeNull();
    await user.click(within(toggle()).getByRole("button", { name: "Original" }));
    expect(isOneTextNode(text())).toBe(true);
    expect(text().textContent).toBe('<b>x</b> {"html":"<i>y</i>"}');
  });

  it("keeps the switch's keys and clicks from the row and the grid (review R-05)", async () => {
    const user = userEvent.setup();
    const { onGridKeyDown, onGridClick, onLeave } = renderExpanded();
    const original = within(toggle()).getByRole("button", { name: "Original" });
    await user.click(original);
    expect(onGridClick).not.toHaveBeenCalled();
    original.focus();
    fireEvent.keyDown(original, { key: "ArrowLeft", keyCode: 37 });
    fireEvent.keyDown(original, { key: "Enter", keyCode: 13 });
    fireEvent.keyDown(original, { key: "Escape", keyCode: 27 });
    expect(onGridKeyDown).not.toHaveBeenCalled();
    expect(onLeave).not.toHaveBeenCalled();
    // The switch is reachable with Tab (its chosen segment) whether or not the text is.
    expect(original).toHaveAttribute("tabindex", "0");
    expect(within(toggle()).getByRole("button", { name: "Formatted" })).toHaveAttribute(
      "tabindex",
      "-1",
    );
  });

  it("still leaves on Escape and Shift+Tab from the text, and only the text follows focusable", () => {
    const { onLeave, onGridKeyDown } = renderExpanded();
    expect(text()).toHaveAttribute("tabindex", "0");
    fireEvent.keyDown(text(), { key: "Escape", keyCode: 27 });
    expect(onLeave).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(text(), { key: "Tab", shiftKey: true, keyCode: 9 });
    expect(onLeave).toHaveBeenCalledTimes(2);
    expect(onGridKeyDown).not.toHaveBeenCalled();
    // Other keys in the text still reach the grid, as before.
    fireEvent.keyDown(text(), { key: "ArrowDown", keyCode: 40 });
    expect(onGridKeyDown).toHaveBeenCalledTimes(1);
    const { unmount } = render(
      <ExpandedMessage
        message="x"
        focusable={false}
        onMeasure={vi.fn()}
        onLeave={vi.fn()}
        label="Full message"
        t={t}
      />,
    );
    expect(screen.getAllByTestId("log-table-expanded")[1]).toHaveAttribute("tabindex", "-1");
    unmount();
  });
});
