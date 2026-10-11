import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { connectedView, sessionView, t } from "../test/fixtures";
import { TopBar } from "./TopBar";

function renderBar(view = connectedView()) {
  const handlers = {
    onSelectProfile: vi.fn(),
    onSelectRegion: vi.fn(),
    onSelectTimeZone: vi.fn(),
    onOpenSettings: vi.fn(),
  };
  render(<TopBar session={view} t={t} {...handlers} />);
  return handlers;
}

/** The trigger button of the dropdown with `testId`. */
function trigger(testId: string): HTMLElement {
  return within(screen.getByTestId(testId)).getByRole("button");
}

describe("TopBar (mockups screen 1, R-05)", () => {
  it("shows the app name as the h1 and the three dropdowns with the current choices", () => {
    renderBar(connectedView({ timeZone: "Utc" }));
    // Gate fix 14h: the app is called "Local Sights" (the package stays local-sights).
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Local Sights");
    expect(trigger("top-bar-profile")).toHaveTextContent("Profile: dev");
    expect(trigger("top-bar-region")).toHaveTextContent("ap-northeast-1");
    expect(trigger("top-bar-time-zone")).toHaveTextContent("Time zone: UTC");
    expect(screen.getByTestId("top-bar-settings-button")).toHaveAccessibleName("Settings");
  });

  it("prompts for a profile and a region while nothing is chosen (U2:BR2.1)", () => {
    renderBar(sessionView());
    expect(trigger("top-bar-profile")).toHaveTextContent("Choose a profile");
    expect(trigger("top-bar-region")).toHaveTextContent("Choose a region");
  });

  it("selects a profile from the dropdown, the SDK default first", async () => {
    const user = userEvent.setup();
    const { onSelectProfile } = renderBar();
    await user.click(trigger("top-bar-profile"));
    const items = screen.getAllByRole("menuitem");
    expect(items.map((item) => item.textContent)).toEqual(["Default", "dev", "prod"]);
    await user.click(screen.getByRole("menuitem", { name: "prod" }));
    expect(onSelectProfile).toHaveBeenCalledWith({ kind: "Named", profileName: "prod" });
    await user.click(trigger("top-bar-profile"));
    await user.click(screen.getByRole("menuitem", { name: "Default" }));
    expect(onSelectProfile).toHaveBeenLastCalledWith({ kind: "SdkDefault" });
  });

  it("names the SDK default profile Default in the trigger (gate fixes 14b, 15a)", () => {
    renderBar(
      connectedView({ connection: { profile: { kind: "SdkDefault" }, region: "us-east-1" } }),
    );
    expect(trigger("top-bar-profile")).toHaveTextContent("Profile: Default");
    expect(trigger("top-bar-profile")).not.toHaveTextContent("SDK");
  });

  it("selects a region from the dropdown", async () => {
    const user = userEvent.setup();
    const { onSelectRegion } = renderBar();
    await user.click(trigger("top-bar-region"));
    await user.click(screen.getByRole("menuitem", { name: "us-east-1" }));
    expect(onSelectRegion).toHaveBeenCalledWith("us-east-1");
  });

  it("switches the time zone from the dropdown (U4:BR3.3)", async () => {
    const user = userEvent.setup();
    const { onSelectTimeZone } = renderBar();
    await user.click(trigger("top-bar-time-zone"));
    await user.click(screen.getByRole("menuitem", { name: "UTC" }));
    expect(onSelectTimeZone).toHaveBeenCalledWith("Utc");
  });

  it("opens the settings from the gear button", async () => {
    const { onOpenSettings } = renderBar();
    await userEvent.click(screen.getByTestId("top-bar-settings-button"));
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  it("disables the connection and settings while fetching but keeps the time zone usable", () => {
    renderBar(
      connectedView({ phase: "Fetching", canChangeConnection: false, canOpenSettings: false }),
    );
    expect(trigger("top-bar-profile")).toBeDisabled();
    expect(trigger("top-bar-region")).toBeDisabled();
    expect(trigger("top-bar-time-zone")).toBeEnabled();
    expect(screen.getByTestId("top-bar-settings-button")).toBeDisabled();
  });

  it("disables everything while a change awaits confirmation or the settings are open", () => {
    const { unmount } = render(
      <TopBar
        session={connectedView({
          pendingChange: { proposedProfile: null, proposedRegion: "eu-west-1" },
          canChangeConnection: false,
          canOpenSettings: false,
        })}
        t={t}
        onSelectProfile={vi.fn()}
        onSelectRegion={vi.fn()}
        onSelectTimeZone={vi.fn()}
        onOpenSettings={vi.fn()}
      />,
    );
    for (const id of ["top-bar-profile", "top-bar-region", "top-bar-time-zone"]) {
      expect(trigger(id)).toBeDisabled();
    }
    expect(screen.getByTestId("top-bar-settings-button")).toBeDisabled();
    unmount();
    renderBar(connectedView({ settingsDialog: "Open", canOpenSettings: true }));
    expect(trigger("top-bar-time-zone")).toBeDisabled();
    expect(screen.getByTestId("top-bar-settings-button")).toBeDisabled();
  });

  it("shows a warning alert for each config file that could not be read (U2:BR1.3)", () => {
    renderBar(
      sessionView({
        catalogNotices: ["catalog.unreadable.config", "catalog.unreadable.credentials"],
      }),
    );
    const notices = screen.getAllByTestId("top-bar-notice");
    expect(notices).toHaveLength(2);
    expect(notices[0]).toHaveTextContent(
      "The AWS config file could not be read; its profiles are not listed.",
    );
    expect(notices[1]).toHaveTextContent("credentials file");
    expect(screen.getByTestId("top-bar").textContent).not.toMatch(/\/Users|\.aws/);
  });
});
