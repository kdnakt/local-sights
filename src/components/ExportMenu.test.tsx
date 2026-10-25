import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { ExportMenu, type ExportMenuProps } from "./ExportMenu";

function renderMenu(props: Partial<ExportMenuProps> = {}) {
  const onSelect = vi.fn();
  render(<ExportMenu disabled={false} loading={false} onSelect={onSelect} t={t} {...props} />);
  return { onSelect };
}

/** The trigger button of the menu. */
const trigger = () => within(screen.getByTestId("export-menu")).getByRole("button");

/** Whether a Cloudscape button cannot be pressed (disabled, or busy). */
const isInert = (button: HTMLElement) =>
  button.hasAttribute("disabled") || button.getAttribute("aria-disabled") === "true";

describe("ExportMenu (261022-small-features U2:BR4.1, BR5.5, ADR-006)", () => {
  it("offers the four exports in the order of BR4.1", async () => {
    const user = userEvent.setup();
    renderMenu();
    expect(trigger()).toHaveAccessibleName("Export");
    await user.click(trigger());
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
      "Save as CSV",
      "Save as Markdown",
      "Copy as CSV",
      "Copy as Markdown",
    ]);
  });

  it("hands the chosen format and target to onSelect", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderMenu();
    await user.click(trigger());
    await user.click(screen.getByRole("menuitem", { name: "Copy as Markdown" }));
    expect(onSelect).toHaveBeenLastCalledWith("markdown", "clipboard");
    await user.click(trigger());
    await user.click(screen.getByRole("menuitem", { name: "Save as CSV" }));
    expect(onSelect).toHaveBeenLastCalledWith("csv", "file");
    await user.click(trigger());
    await user.click(screen.getByRole("menuitem", { name: "Save as Markdown" }));
    expect(onSelect).toHaveBeenLastCalledWith("markdown", "file");
    await user.click(trigger());
    await user.click(screen.getByRole("menuitem", { name: "Copy as CSV" }));
    expect(onSelect).toHaveBeenLastCalledWith("csv", "clipboard");
    expect(onSelect).toHaveBeenCalledTimes(4);
  });

  it("cannot be opened while disabled (BR2.3, BR3.2)", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderMenu({ disabled: true });
    expect(isInert(trigger())).toBe(true);
    await user.click(trigger());
    expect(screen.queryAllByRole("menuitem")).toHaveLength(0);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("is busy and cannot be opened while an export runs (BR4.8)", async () => {
    const user = userEvent.setup();
    renderMenu({ loading: true });
    expect(isInert(trigger())).toBe(true);
    await user.click(trigger());
    expect(screen.queryAllByRole("menuitem")).toHaveLength(0);
    expect(screen.getByText("Exporting…")).toBeInTheDocument();
  });

  it("opens and chooses with the keyboard (BR5.5)", async () => {
    const { onSelect } = renderMenu();
    trigger().focus();
    // Cloudscape reads the key code; user-event sends none. Arrow Down opens
    // the menu with the first item highlighted, the next one moves on.
    fireEvent.keyDown(trigger(), { key: "ArrowDown", keyCode: 40 });
    const items = await screen.findAllByRole("menuitem");
    expect(items).toHaveLength(4);
    fireEvent.keyDown(trigger(), { key: "ArrowDown", keyCode: 40 });
    fireEvent.keyDown(trigger(), { key: "Enter", keyCode: 13 });
    expect(onSelect).toHaveBeenCalledWith("markdown", "file");
  });

  it("reads エクスポート and its items in Japanese", async () => {
    const user = userEvent.setup();
    renderMenu({ t: createTranslator("ja") });
    expect(trigger()).toHaveAccessibleName("エクスポート");
    expect(trigger()).toHaveTextContent("エクスポート");
    await user.click(trigger());
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
      "CSV で保存",
      "Markdown で保存",
      "CSV をコピー",
      "Markdown をコピー",
    ]);
  });
});
