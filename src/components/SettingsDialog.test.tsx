import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ButtonProps } from "@cloudscape-design/components/button";
import type { SessionView } from "../api";
import { createTranslator, type Translate } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";
import { SettingsDialog } from "./SettingsDialog";

function renderDialog(
  view: SessionView = sessionView({ settingsDialog: "Open" }),
  translate: Translate = t,
) {
  const returnFocusRef = { current: { focus: vi.fn() } as ButtonProps.Ref };
  const handlers = { onSave: vi.fn(), onCancel: vi.fn(), onClear: vi.fn() };
  const result = render(
    <SettingsDialog session={view} t={translate} returnFocusRef={returnFocusRef} {...handlers} />,
  );
  return { ...handlers, returnFocusRef, ...result };
}

const checkbox = () => screen.getByRole("checkbox");
/** The trigger button of the language select (261022-small-features U1). */
const languageTrigger = () =>
  within(screen.getByTestId("settings-dialog-language-select")).getByRole("button");
/** Asserts the language options now listed, in order (the selected one repeats its name for readers). */
function expectLanguageOptions(expected: readonly string[]) {
  const options = screen.getAllByRole("option");
  expect(options).toHaveLength(expected.length);
  expected.forEach((text, index) => expect(options[index]).toHaveTextContent(text));
}

/** Cloudscape reads the key code; user-event sends none, so these keys are fired by hand. */
const pressKey = (key: string, keyCode: number) =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key, keyCode });
const pressEscape = () => pressKey("Escape", 27);

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
    expect(languageTrigger()).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-clear-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-cancel-button")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("settings-dialog-save-button")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onSave).toHaveBeenCalledWith(true, "system");
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("keeps Tab inside the dialog", async () => {
    const user = userEvent.setup();
    renderDialog();
    const dialog = screen.getByRole("dialog", { name: "Settings" });
    for (let step = 0; step < 8; step += 1) {
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

  // ---- 261022-small-features U1: the language choice (BR1.1, BR1.7) ----

  describe("the language choice (261022-small-features U1:BR1.1, BR1.7)", () => {
    it("starts on the saved preference, shown by its name", () => {
      renderDialog(sessionView({ settingsDialog: "Open", language: "ja" }));
      expect(languageTrigger()).toHaveTextContent("日本語");
      expect(checkbox()).toHaveFocus();
    });

    it("lists the three choices, the OS one translated and the two languages fixed", async () => {
      const user = userEvent.setup();
      const { unmount } = renderDialog();
      await user.click(languageTrigger());
      expectLanguageOptions(["Follow the OS setting", "日本語", "English"]);
      pressEscape();
      unmount();

      renderDialog(sessionView({ settingsDialog: "Open", language: "en" }), createTranslator("ja"));
      expect(languageTrigger()).toHaveTextContent("English");
      await user.click(languageTrigger());
      expectLanguageOptions(["OS の設定に合わせる", "日本語", "English"]);
    });

    it("changes nothing when a choice is made, and hands both settings over on [Save]", async () => {
      const user = userEvent.setup();
      const { onSave } = renderDialog(sessionView({ settingsDialog: "Open", cacheEnabled: true }));
      await user.click(languageTrigger());
      await user.click(screen.getByRole("option", { name: "日本語" }));
      expect(languageTrigger()).toHaveTextContent("日本語");
      expect(onSave).not.toHaveBeenCalled();
      await user.click(screen.getByTestId("settings-dialog-save-button"));
      expect(onSave).toHaveBeenCalledTimes(1);
      expect(onSave).toHaveBeenCalledWith(true, "ja");
    });

    it("drops the choice on [Cancel] and on Escape", async () => {
      const user = userEvent.setup();
      const { onSave, onCancel } = renderDialog();
      await user.click(languageTrigger());
      await user.click(screen.getByRole("option", { name: "English" }));
      expect(languageTrigger()).toHaveTextContent("English");
      await user.click(screen.getByTestId("settings-dialog-cancel-button"));
      expect(onCancel).toHaveBeenCalledTimes(1);
      pressEscape();
      expect(onCancel).toHaveBeenCalledTimes(2);
      expect(onSave).not.toHaveBeenCalled();
    });

    it("is named by its label and reached with the keyboard", async () => {
      const user = userEvent.setup();
      const { onSave } = renderDialog(
        sessionView({ settingsDialog: "Open" }),
        createTranslator("ja"),
      );
      expect(screen.getByRole("button", { name: /言語/ })).toBe(languageTrigger());
      expect(checkbox()).toHaveFocus();
      await user.tab();
      expect(languageTrigger()).toHaveFocus();
      // Enter opens the list on the current choice; the arrows move, Enter takes.
      pressKey("Enter", 13);
      expectLanguageOptions(["OS の設定に合わせる", "日本語", "English"]);
      pressKey("ArrowDown", 40);
      pressKey("ArrowDown", 40);
      pressKey("Enter", 13);
      expect(screen.queryAllByRole("option")).toHaveLength(0);
      expect(languageTrigger()).toHaveTextContent("English");
      await user.click(screen.getByTestId("settings-dialog-save-button"));
      expect(onSave).toHaveBeenCalledWith(false, "en");
    });
  });
});
