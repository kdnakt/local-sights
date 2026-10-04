import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";
import { t } from "../test/fixtures";
import { createTranslator } from "../i18n/messages";

describe("ConfirmDialog", () => {
  it("asks with the catalog text and focuses Cancel", () => {
    render(<ConfirmDialog t={t} onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByRole("dialog")).toHaveTextContent(
      "The logs shown will be cleared. Change the connection?",
    );
    expect(screen.getByTestId("confirm-dialog-cancel-button")).toHaveFocus();
  });

  it("confirms with Change", async () => {
    const onConfirm = vi.fn();
    render(<ConfirmDialog t={t} onConfirm={onConfirm} onCancel={vi.fn()} />);
    await userEvent.click(screen.getByTestId("confirm-dialog-change-button"));
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("cancels with Cancel, Enter on the focused Cancel, or Escape", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<ConfirmDialog t={t} onConfirm={vi.fn()} onCancel={onCancel} />);
    await user.keyboard("{Escape}");
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
    const change = screen.getByTestId("confirm-dialog-change-button");
    const cancel = screen.getByTestId("confirm-dialog-cancel-button");
    expect(cancel).toHaveFocus();
    await user.tab();
    expect(change).toHaveFocus();
    await user.tab();
    expect(cancel).toHaveFocus();
    await user.tab({ shift: true });
    expect(change).toHaveFocus();
    await user.tab({ shift: true });
    expect(cancel).toHaveFocus();
    expect(screen.getByTestId("outside-button")).not.toHaveFocus();
  });
});
