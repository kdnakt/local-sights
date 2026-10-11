import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";
import { t } from "../test/fixtures";
import { createTranslator } from "../i18n/messages";

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

describe("ConfirmDialog (U2:BR2.5, mockups screen 6)", () => {
  it("asks with the catalog text in a modal and focuses Cancel", () => {
    render(<ConfirmDialog t={t} onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByRole("dialog", { name: "Change the connection?" })).toHaveTextContent(
      "The logs shown will be cleared. Change the connection?",
    );
    expect(screen.getByTestId("confirm-dialog-cancel-button")).toHaveFocus();
  });

  it("confirms with the primary Change at the right end", async () => {
    const onConfirm = vi.fn();
    render(<ConfirmDialog t={t} onConfirm={onConfirm} onCancel={vi.fn()} />);
    const change = screen.getByTestId("confirm-dialog-change-button");
    const cancel = screen.getByTestId("confirm-dialog-cancel-button");
    expect(cancel.compareDocumentPosition(change) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await userEvent.click(change);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("cancels with Cancel, Enter on the focused Cancel, Escape, or the close button", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<ConfirmDialog t={t} onConfirm={vi.fn()} onCancel={onCancel} />);
    pressEscape();
    await user.keyboard("{Enter}");
    await user.click(screen.getByTestId("confirm-dialog-cancel-button"));
    expect(onCancel).toHaveBeenCalledTimes(3);
  });

  it("is shown in Japanese for the ja locale", () => {
    render(<ConfirmDialog t={createTranslator("ja")} onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByRole("dialog")).toHaveTextContent(
      "表示中のログが消えます。変えてよいですか？",
    );
    expect(screen.getByTestId("confirm-dialog-change-button")).toHaveTextContent("変える");
  });

  it("keeps focus inside the dialog with Tab and Shift+Tab", async () => {
    const user = userEvent.setup();
    render(
      <>
        <button type="button" data-testid="outside-button">
          outside
        </button>
        <ConfirmDialog t={t} onConfirm={vi.fn()} onCancel={vi.fn()} />
      </>,
    );
    const dialog = screen.getByRole("dialog");
    const change = screen.getByTestId("confirm-dialog-change-button");
    const cancel = screen.getByTestId("confirm-dialog-cancel-button");
    expect(cancel).toHaveFocus();
    await user.tab();
    expect(change).toHaveFocus();
    const visited = new Set<Element | null>();
    for (let step = 0; step < 4; step += 1) {
      await user.tab();
      visited.add(document.activeElement);
      expect(dialog).toContainElement(document.activeElement as HTMLElement);
    }
    expect(visited.has(cancel)).toBe(true);
    for (let step = 0; step < 4; step += 1) {
      await user.tab({ shift: true });
      expect(dialog).toContainElement(document.activeElement as HTMLElement);
    }
    expect(screen.getByTestId("outside-button")).not.toHaveFocus();
  });
});
