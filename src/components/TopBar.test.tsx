import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator, type Locale } from "../i18n/messages";
import { connectedView, sessionView, ssoLoginView, t } from "../test/fixtures";
import { TopBar } from "./TopBar";

function renderBar(view = connectedView(), locale: Locale = "en") {
  const handlers = {
    onSelectProfile: vi.fn(),
    onSelectRegion: vi.fn(),
    onSelectTimeZone: vi.fn(),
    onOpenSettings: vi.fn(),
  };
  render(
    <TopBar
      session={view}
      t={locale === "en" ? t : createTranslator("ja")}
      locale={locale}
      {...handlers}
    />,
  );
  return handlers;
}

/** The texts of the region options now listed. */
function regionOptions(): (string | null)[] {
  return screen.queryAllByRole("option").map((option) => option.textContent);
}

/** Waits until the region options are `expected`: the Select filters after a short delay. */
async function expectRegionOptions(expected: string[]): Promise<void> {
  await waitFor(() => expect(regionOptions()).toEqual(expected));
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
    await user.click(screen.getByRole("option", { name: /^us-east-1/ }));
    expect(onSelectRegion).toHaveBeenCalledWith("us-east-1");
  });

  // ---- 261020-carryover-fixes FR5: region names and filter ------------

  it("lists the regions in their order with the code and the Japanese name, the current one selected", async () => {
    const user = userEvent.setup();
    renderBar(connectedView(), "ja");
    await user.click(trigger("top-bar-region"));
    expect(regionOptions()).toEqual([
      "ap-northeast-1 アジアパシフィック (東京)",
      "eu-west-1 欧州 (アイルランド)",
      "us-east-1 米国東部 (バージニア北部)",
    ]);
    expect(screen.getByRole("option", { name: /^ap-northeast-1/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("option", { name: /^eu-west-1/ })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("writes the English names on the English screen", async () => {
    const user = userEvent.setup();
    renderBar();
    await user.click(trigger("top-bar-region"));
    expect(regionOptions()).toEqual([
      "ap-northeast-1 Asia Pacific (Tokyo)",
      "eu-west-1 Europe (Ireland)",
      "us-east-1 US East (N. Virginia)",
    ]);
  });

  it("filters the regions by what is typed, by either language's name", async () => {
    const user = userEvent.setup();
    renderBar();
    await user.click(trigger("top-bar-region"));
    const filter = screen.getByRole("combobox", { name: "Region" });
    await user.type(filter, "tokyo");
    await expectRegionOptions(["ap-northeast-1 Asia Pacific (Tokyo)"]);
    await user.clear(filter);
    await user.type(filter, "Europe");
    await expectRegionOptions(["eu-west-1 Europe (Ireland)"]);
    await user.clear(filter);
    await user.type(filter, "東京");
    await expectRegionOptions(["ap-northeast-1 Asia Pacific (Tokyo)"]);
  });

  it("says in the screen's language that no region matches", async () => {
    const user = userEvent.setup();
    renderBar(connectedView(), "ja");
    await user.click(trigger("top-bar-region"));
    await user.type(screen.getByRole("combobox", { name: "リージョン" }), "zzz");
    await expectRegionOptions([]);
    expect(screen.getByText("一致するリージョンはありません")).toBeInTheDocument();
  });

  it("shows the chosen region with its name while closed", () => {
    renderBar(connectedView(), "ja");
    expect(trigger("top-bar-region")).toHaveTextContent("ap-northeast-1 アジアパシフィック (東京)");
  });

  it("writes a region the table does not name by its code and finds it by its code", async () => {
    const user = userEvent.setup();
    renderBar(
      connectedView({
        regions: ["ap-northeast-1", "xx-test-1"],
        connection: { profile: { kind: "Named", profileName: "dev" }, region: "xx-test-1" },
      }),
    );
    expect(trigger("top-bar-region")).toHaveTextContent("xx-test-1");
    await user.click(trigger("top-bar-region"));
    expect(regionOptions()).toEqual(["ap-northeast-1 Asia Pacific (Tokyo)", "xx-test-1"]);
    await user.type(screen.getByRole("combobox", { name: "Region" }), "XX-");
    await expectRegionOptions(["xx-test-1"]);
  });

  it("does not open while the connection cannot change or a sign-in runs (FR5.8)", async () => {
    const user = userEvent.setup();
    for (const view of [
      connectedView({ canChangeConnection: false }),
      connectedView({ ssoLogin: ssoLoginView({ state: "Waiting" }) }),
    ]) {
      const { onSelectRegion } = renderBar(view);
      expect(trigger("top-bar-region")).toBeDisabled();
      await user.click(trigger("top-bar-region"));
      expect(regionOptions()).toEqual([]);
      expect(screen.queryByRole("combobox", { name: "Region" })).toBeNull();
      expect(onSelectRegion).not.toHaveBeenCalled();
      cleanup();
    }
  });

  it("is named Region for screen readers and opens with the keyboard only", async () => {
    const user = userEvent.setup();
    const { onSelectRegion } = renderBar();
    expect(trigger("top-bar-region")).toHaveAccessibleName(/^Region/);
    await user.tab();
    await user.tab();
    expect(trigger("top-bar-region")).toHaveFocus();
    // Cloudscape reads `keyCode`, which user-event's keys do not carry.
    const press = (key: string, keyCode: number) =>
      fireEvent.keyDown(document.activeElement as Element, { key, keyCode });
    press("ArrowDown", 40);
    await waitFor(() => expect(regionOptions()).toHaveLength(3));
    press("ArrowDown", 40);
    press("ArrowDown", 40);
    press("Enter", 13);
    expect(onSelectRegion).toHaveBeenCalledTimes(1);
    // From the selected ap-northeast-1, two steps down.
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
        locale="en"
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

  it("disables the profile, the region and the settings while signing in, not the time zone (261011-sso-login BR3.4)", () => {
    for (const state of ["Starting", "Waiting"] as const) {
      const { unmount } = render(
        <TopBar
          // Even if the flags lagged behind, a running login disables them.
          session={connectedView({ ssoLogin: ssoLoginView({ state }) })}
          t={t}
          locale="en"
          onSelectProfile={vi.fn()}
          onSelectRegion={vi.fn()}
          onSelectTimeZone={vi.fn()}
          onOpenSettings={vi.fn()}
        />,
      );
      expect(trigger("top-bar-profile")).toBeDisabled();
      expect(trigger("top-bar-region")).toBeDisabled();
      expect(screen.getByTestId("top-bar-settings-button")).toBeDisabled();
      expect(trigger("top-bar-time-zone")).toBeEnabled();
      unmount();
    }
    renderBar(connectedView({ ssoLogin: ssoLoginView({ state: "Failed", exitCode: 1 }) }));
    expect(trigger("top-bar-profile")).toBeEnabled();
    expect(screen.getByTestId("top-bar-settings-button")).toBeEnabled();
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
