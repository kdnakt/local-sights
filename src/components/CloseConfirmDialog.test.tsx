import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { CloseConfirmDialog } from "./CloseConfirmDialog";

function renderDialog(translate = t) {
  const onKeep = vi.fn();
  const onClose = vi.fn();
  const view = render(<CloseConfirmDialog t={translate} onKeep={onKeep} onClose={onClose} />);
  return { ...view, onKeep, onClose };
}

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

describe("CloseConfirmDialog (U7:BR3.3, mockups screen 6)", () => {
  it("says in words what is lost and starts on [Keep fetching]", () => {
    renderDialog();
    const dialog = screen.getByRole("dialog", { name: "Fetching is in progress." });
    expect(dialog).toHaveTextContent(
      "If you close the window, the logs fetched so far will be lost.",
    );
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveFocus();
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveTextContent("Keep fetching");
    expect(screen.getByTestId("close-confirm-close-button")).toHaveTextContent("Close");
  });

  it("answers Keep fetching with Enter or Escape and Close with its button", async () => {
    const user = userEvent.setup();
    const { onKeep, onClose } = renderDialog();
    await user.keyboard("{Enter}");
    expect(onKeep).toHaveBeenCalledTimes(1);
    pressEscape();
    expect(onKeep).toHaveBeenCalledTimes(2);
    expect(onClose).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("close-confirm-close-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("puts the primary Close at the right end and is in Japanese for ja", () => {
    renderDialog(createTranslator("ja"));
    const keep = screen.getByTestId("close-confirm-keep-button");
    const close = screen.getByTestId("close-confirm-close-button");
    expect(keep).toHaveTextContent("取得を続ける");
    expect(close).toHaveTextContent("閉じる");
    expect(keep.compareDocumentPosition(close) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("keeps Tab inside the dialog and gives the focus back when it closes", async () => {
    const user = userEvent.setup();
    const before = document.createElement("button");
    document.body.appendChild(before);
    before.focus();
    const { unmount } = renderDialog();
    const dialog = screen.getByRole("dialog");
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("close-confirm-close-button")).toHaveFocus();
    for (let step = 0; step < 3; step += 1) {
      await user.tab();
      expect(dialog).toContainElement(document.activeElement as HTMLElement);
    }
    await user.tab({ shift: true });
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
    unmount();
    expect(before).toHaveFocus();
    before.remove();
  });
});
