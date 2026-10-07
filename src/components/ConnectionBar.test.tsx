import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConnectionBar } from "./ConnectionBar";
import { connectedView, sessionView, t } from "../test/fixtures";

function renderBar(view = sessionView()) {
  const onSelectProfile = vi.fn();
  const onSelectRegion = vi.fn();
  const onSelectTimeZone = vi.fn();
  render(
    <ConnectionBar
      session={view}
      t={t}
      onSelectProfile={onSelectProfile}
      onSelectRegion={onSelectRegion}
      onSelectTimeZone={onSelectTimeZone}
    />,
  );
  return { onSelectProfile, onSelectRegion, onSelectTimeZone };
}

describe("ConnectionBar", () => {
  it("starts unselected with the SDK default first", () => {
    renderBar();
    const profile = screen.getByTestId("connection-bar-profile-select");
    expect(profile).toHaveValue("");
    const options = within(profile).getAllByRole("option");
    expect(options[1]).toHaveTextContent("Default settings (left to the SDK)");
    expect(options.slice(2).map((o) => o.textContent)).toEqual(["dev", "prod"]);
    expect(screen.getByTestId("connection-bar-region-select")).toHaveValue("");
  });

  it("forwards the chosen profile and region", async () => {
    const user = userEvent.setup();
    const { onSelectProfile, onSelectRegion } = renderBar();
    await user.selectOptions(screen.getByTestId("connection-bar-profile-select"), "named:dev");
    expect(onSelectProfile).toHaveBeenCalledWith({ kind: "Named", profileName: "dev" });
    await user.selectOptions(screen.getByTestId("connection-bar-profile-select"), "sdk-default");
    expect(onSelectProfile).toHaveBeenLastCalledWith({ kind: "SdkDefault" });
    await user.selectOptions(screen.getByTestId("connection-bar-region-select"), "eu-west-1");
    expect(onSelectRegion).toHaveBeenCalledWith("eu-west-1");
  });

  it("shows the current connection", () => {
    renderBar(connectedView());
    expect(screen.getByTestId("connection-bar-profile-select")).toHaveValue("named:dev");
    expect(screen.getByTestId("connection-bar-region-select")).toHaveValue("ap-northeast-1");
  });

  it("is disabled while fetching or confirming", () => {
    renderBar(connectedView({ canChangeConnection: false }));
    expect(screen.getByTestId("connection-bar-profile-select")).toBeDisabled();
    expect(screen.getByTestId("connection-bar-region-select")).toBeDisabled();
  });

  it("names the unreadable file kind only", () => {
    renderBar(sessionView({ catalogNotices: ["catalog.unreadable.credentials"] }));
    expect(screen.getByTestId("connection-bar-notice")).toHaveTextContent(
      "The AWS credentials file could not be read; its profiles are not listed.",
    );
  });

  it("holds the time zone switch, usable while the connection is locked", async () => {
    const { onSelectTimeZone } = renderBar(
      connectedView({ phase: "Fetching", canChangeConnection: false }),
    );
    expect(screen.getByTestId("connection-bar-profile-select")).toBeDisabled();
    const utc = screen.getByTestId("time-zone-toggle-utc-radio");
    expect(utc).toBeEnabled();
    await userEvent.click(utc);
    expect(onSelectTimeZone).toHaveBeenCalledWith("Utc");
  });

  it("opens the settings with [*] unless fetching or confirming", async () => {
    const user = userEvent.setup();
    const onOpenSettings = vi.fn();
    const { rerender } = render(
      <ConnectionBar
        session={sessionView()}
        t={t}
        onSelectProfile={vi.fn()}
        onSelectRegion={vi.fn()}
        onSelectTimeZone={vi.fn()}
        onOpenSettings={onOpenSettings}
      />,
    );
    const button = screen.getByRole("button", { name: "Settings" });
    expect(button).toHaveTextContent("*");
    await user.click(button);
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
    rerender(
      <ConnectionBar
        session={sessionView({ phase: "Fetching", canOpenSettings: false })}
        t={t}
        onSelectProfile={vi.fn()}
        onSelectRegion={vi.fn()}
        onSelectTimeZone={vi.fn()}
        onOpenSettings={onOpenSettings}
      />,
    );
    expect(screen.getByTestId("connection-bar-settings-button")).toBeDisabled();
  });
});
