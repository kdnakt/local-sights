import { render, screen } from "@testing-library/react";
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

describe("CloseConfirmDialog", () => {
  it("says in words what is lost and starts on [Keep fetching]", () => {
    renderDialog();
    const dialog = screen.getByRole("alertdialog", { name: "Fetching is in progress." });
    expect(dialog).toHaveAttribute("aria-modal", "true");
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
    await user.keyboard("{Escape}");
    expect(onKeep).toHaveBeenCalledTimes(2);
    expect(onClose).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("close-confirm-close-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("keeps Tab on its two buttons and gives the focus back when it closes", async () => {
    const user = userEvent.setup();
    const before = document.createElement("button");
    document.body.appendChild(before);
    before.focus();
    const { unmount } = renderDialog(createTranslator("ja"));
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveTextContent("取得を続ける");
    await user.tab();
    expect(screen.getByTestId("close-confirm-close-button")).toHaveFocus();
    expect(screen.getByTestId("close-confirm-close-button")).toHaveTextContent("閉じる");
    await user.tab();
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByTestId("close-confirm-close-button")).toHaveFocus();
    unmount();
    expect(before).toHaveFocus();
    before.remove();
  });
});
