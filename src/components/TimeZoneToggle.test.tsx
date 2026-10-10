import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { TimeZoneChoice } from "../api";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { TimeZoneToggle } from "./TimeZoneToggle";

function renderToggle(timeZone: TimeZoneChoice = "Local", translate = t) {
  const onSelect = vi.fn();
  render(<TimeZoneToggle timeZone={timeZone} t={translate} onSelect={onSelect} />);
  return { onSelect };
}

describe("TimeZoneToggle", () => {
  it("shows the core's choice, Local at startup", () => {
    renderToggle("Local");
    expect(screen.getByRole("group", { name: "Time zone" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Local" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "UTC" })).not.toBeChecked();
  });

  it("asks the core to switch when the other zone is clicked", async () => {
    const { onSelect } = renderToggle("Local");
    await userEvent.click(screen.getByTestId("time-zone-toggle-utc-radio"));
    expect(onSelect).toHaveBeenCalledWith("Utc");
  });

  it("is reached with Tab and switched with the arrow keys", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderToggle("Utc");
    await user.tab();
    expect(screen.getByTestId("time-zone-toggle-utc-radio")).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    expect(onSelect).toHaveBeenLastCalledWith("Local");
  });

  it("is switched with Space on the focused option", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderToggle("Local");
    screen.getByTestId("time-zone-toggle-utc-radio").focus();
    await user.keyboard(" ");
    expect(onSelect).toHaveBeenCalledWith("Utc");
  });

  it("is labelled in Japanese for the ja locale", () => {
    renderToggle("Local", createTranslator("ja"));
    expect(screen.getByRole("group", { name: "タイムゾーン" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "ローカル" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "UTC" })).toBeEnabled();
  });
});
