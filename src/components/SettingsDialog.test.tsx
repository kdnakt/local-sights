import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ButtonProps } from "@cloudscape-design/components/button";
import type { SessionView } from "../api";
import { createTranslator } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";
import { SettingsDialog } from "./SettingsDialog";

function renderDialog(view: SessionView = sessionView({ settingsDialog: "Open" })) {
  const returnFocusRef = { current: { focus: vi.fn() } as ButtonProps.Ref };
  const handlers = { onSave: vi.fn(), onCancel: vi.fn(), onClear: vi.fn() };
  const result = render(
    <SettingsDialog session={view} t={t} returnFocusRef={returnFocusRef} {...handlers} />,
  );
  return { ...handlers, returnFocusRef, ...result };
}

const checkbox = () => screen.getByRole("checkbox");

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

describe("SettingsDialog (U6:BR5.1, mockups screen 6)", () => {
  it("is a modal that starts on the checkbox with the saved setting", () => {
    renderDialog(sessionView({ settingsDialog: "Open", cacheEnabled: true }));
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    expect(checkbox()).toHaveFocus();
    expect(checkbox()).toBeChecked();
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
    await user.click(checkbox());
    expect(checkbox()).toBeChecked();
    expect(screen.getByTestId("settings-dialog-location")).toBeInTheDocument();
    expect(screen.getByTestId("settings-dialog-warning")).toBeInTheDocument();
  });

  it("saves the choice made with Space and Enter, with the primary Save at the right end", async () => {
    const user = userEvent.setup();
    const { onSave, onCancel } = renderDialog();
    await user.keyboard(" ");
    expect(checkbox()).toBeChecked();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-clear-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-cancel-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-save-button")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onSave).toHaveBeenCalledWith(true);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("keeps Tab inside the dialog", async () => {
    const user = userEvent.setup();
    renderDialog();
    const dialog = screen.getByRole("dialog", { name: "Settings" });
    for (let step = 0; step < 7; step += 1) {
      await user.tab();
      expect(dialog).toContainElement(document.activeElement as HTMLElement);
    }
    await user.tab({ shift: true });
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
  });

  it("closes without saving on Escape and on [Cancel]", async () => {
    const user = userEvent.setup();
    const { onSave, onCancel } = renderDialog();
    await user.click(checkbox());
    pressEscape();
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
        <SettingsDialog
          session={sessionView({ settingsDialog: "Open", settingsNotice: notice })}
          t={t}
          returnFocusRef={returnFocusRef}
          onSave={vi.fn()}
          onCancel={vi.fn()}
          onClear={onClear}
        />,
      );
      expect(screen.getByTestId("settings-dialog-notice")).toHaveTextContent(text);
    }
  });

  it("gives the focus back to the settings button when it closes, and has Japanese texts", () => {
    const { unmount, returnFocusRef } = renderDialog();
    unmount();
    expect(returnFocusRef.current.focus).toHaveBeenCalledTimes(1);

    render(
      <SettingsDialog
        session={sessionView({ settingsDialog: "Open", cacheDirectory: "/c" })}
        t={createTranslator("ja")}
        returnFocusRef={{ current: null }}
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
