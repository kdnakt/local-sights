import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import type { SessionView } from "../api";
import { createTranslator } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";
import { SettingsDialog } from "./SettingsDialog";

function renderDialog(view: SessionView = sessionView({ settingsDialog: "Open" })) {
  const returnFocusRef = createRef<HTMLButtonElement>();
  const handlers = { onSave: vi.fn(), onCancel: vi.fn(), onClear: vi.fn() };
  const result = render(
    <>
      <button type="button" ref={returnFocusRef} data-testid="settings-button">
        *
      </button>
      <SettingsDialog session={view} t={t} returnFocusRef={returnFocusRef} {...handlers} />
    </>,
  );
  return { ...handlers, returnFocusRef, ...result };
}

describe("SettingsDialog", () => {
  it("starts on the checkbox with the saved setting", () => {
    renderDialog(sessionView({ settingsDialog: "Open", cacheEnabled: true }));
    const checkbox = screen.getByTestId("settings-dialog-cache-checkbox");
    expect(checkbox).toHaveFocus();
    expect(checkbox).toBeChecked();
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
  });

  it("always shows the cache location and the warning, checked or not", async () => {
    const user = userEvent.setup();
    renderDialog();
    expect(screen.getByTestId("settings-dialog-location")).toHaveTextContent(
      "Cache location: /Users/me/Library/Caches/dev.local-sights.app/log-cache",
    );
    expect(screen.getByTestId("settings-dialog-warning")).toHaveTextContent(
      "Logs may contain confidential information.",
    );
    await user.click(screen.getByTestId("settings-dialog-cache-checkbox"));
    expect(screen.getByTestId("settings-dialog-location")).toBeInTheDocument();
    expect(screen.getByTestId("settings-dialog-warning")).toBeInTheDocument();
  });

  it("saves the choice made with Space and Enter only", async () => {
    const user = userEvent.setup();
    const { onSave, onCancel } = renderDialog();
    await user.keyboard(" ");
    expect(screen.getByTestId("settings-dialog-cache-checkbox")).toBeChecked();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-clear-button")).toHaveFocus();
    await user.tab();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-save-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-cache-checkbox")).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByTestId("settings-dialog-save-button")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onSave).toHaveBeenCalledWith(true);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("closes without saving on Escape and on [Cancel]", async () => {
    const user = userEvent.setup();
    const { onSave, onCancel } = renderDialog();
    await user.click(screen.getByTestId("settings-dialog-cache-checkbox"));
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    await user.click(screen.getByTestId("settings-dialog-cancel-button"));
    expect(onCancel).toHaveBeenCalledTimes(2);
    expect(onSave).not.toHaveBeenCalled();
  });

  it("clears the cache at once and shows the notices in words", async () => {
    const user = userEvent.setup();
    const { onClear, rerender, returnFocusRef } = renderDialog();
    await user.click(screen.getByTestId("settings-dialog-clear-button"));
    expect(onClear).toHaveBeenCalledTimes(1);
    for (const [notice, text] of [
      ["Cleared", "The cache was cleared."],
      ["ClearFailed", "The cache could not be cleared."],
      ["SaveFailed", "The setting could not be saved."],
    ] as const) {
      rerender(
        <>
          <button type="button" ref={returnFocusRef} data-testid="settings-button">
            *
          </button>
          <SettingsDialog
            session={sessionView({ settingsDialog: "Open", settingsNotice: notice })}
            t={t}
            returnFocusRef={returnFocusRef}
            onSave={vi.fn()}
            onCancel={vi.fn()}
            onClear={onClear}
          />
        </>,
      );
      expect(screen.getByTestId("settings-dialog-notice")).toHaveTextContent(text);
    }
  });

  it("gives the focus back to [*] when it closes, and has Japanese texts", () => {
    const { rerender, returnFocusRef } = renderDialog();
    rerender(
      <button type="button" ref={returnFocusRef} data-testid="settings-button">
        *
      </button>,
    );
    expect(screen.getByTestId("settings-button")).toHaveFocus();

    rerender(
      <SettingsDialog
        session={sessionView({ settingsDialog: "Open", cacheDirectory: "/c" })}
        t={createTranslator("ja")}
        returnFocusRef={returnFocusRef}
        onSave={vi.fn()}
        onCancel={vi.fn()}
        onClear={vi.fn()}
      />,
    );
    expect(screen.getByRole("dialog", { name: "設定" })).toBeInTheDocument();
    expect(screen.getByTestId("settings-dialog-location")).toHaveTextContent("保存場所：/c");
    expect(screen.getByRole("button", { name: "キャッシュを消す" })).toBeInTheDocument();
  });
});
