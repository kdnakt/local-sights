import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LogGroupPane } from "./LogGroupPane";
import { connectedView, sessionView, ssoLoginView, t } from "../test/fixtures";

function renderPane(view = connectedView()) {
  const handlers = { onFilterChange: vi.fn(), onReload: vi.fn(), onSelect: vi.fn() };
  const result = render(<LogGroupPane session={view} t={t} {...handlers} />);
  return { ...handlers, ...result };
}

const searchBox = () => screen.getByRole("searchbox", { name: "Find log groups" });

describe("LogGroupPane (mockups screen 1 and 5c, Q6)", () => {
  it("is a container headed Log groups with the search field, the table and Reload", () => {
    renderPane();
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Log groups");
    expect(searchBox()).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "Log groups" })).toBeInTheDocument();
    expect(screen.getByTestId("log-group-pane-reload-button")).toHaveTextContent("Reload");
  });

  it("prompts for a profile, then for a region, in the table's empty state", () => {
    const { unmount } = renderPane(sessionView());
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Choose a profile.");
    unmount();
    renderPane(sessionView({ connection: { profile: { kind: "SdkDefault" }, region: null } }));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Choose a region.");
    expect(screen.getByTestId("log-group-pane-reload-button")).toBeDisabled();
  });

  it("lists the groups with a radio each, marks the selection and selects on click", async () => {
    const { onSelect } = renderPane(connectedView({ selectedLogGroupName: "/aws/ecs/web" }));
    const options = screen.getAllByTestId("log-group-pane-option");
    expect(options.map((o) => o.textContent)).toEqual(["/aws/ecs/web", "/aws/lambda/MyFunction"]);
    expect(screen.getByRole("radio", { name: "/aws/ecs/web" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "/aws/lambda/MyFunction" })).not.toBeChecked();
    await userEvent.click(screen.getByRole("radio", { name: "/aws/lambda/MyFunction" }));
    expect(onSelect).toHaveBeenCalledWith("/aws/lambda/MyFunction");
    await userEvent.click(options[1]!);
    expect(onSelect).toHaveBeenLastCalledWith("/aws/lambda/MyFunction");
  });

  it("selects with the arrow keys between the radios (K4)", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderPane(connectedView({ selectedLogGroupName: "/aws/ecs/web" }));
    screen.getByRole("radio", { name: "/aws/ecs/web" }).focus();
    await user.keyboard("{ArrowDown}");
    expect(onSelect).toHaveBeenLastCalledWith("/aws/lambda/MyFunction");
  });

  it("forwards the search text and Reload", async () => {
    const user = userEvent.setup();
    const { onFilterChange, onReload } = renderPane();
    await user.type(searchBox(), "ecs");
    expect(onFilterChange).toHaveBeenLastCalledWith("ecs");
    expect(searchBox()).toHaveValue("ecs");
    await user.click(screen.getByTestId("log-group-pane-reload-button"));
    expect(onReload).toHaveBeenCalledTimes(1);
  });

  it("shows loading in the table while empty, and above it while groups stay selectable", async () => {
    const loading = (groups: string[]) =>
      connectedView({
        logGroups: {
          status: "Loading",
          visibleGroups: groups,
          totalCount: groups.length,
          emptyState: null,
          failure: null,
        },
      });
    const { unmount } = renderPane(loading([]));
    expect(screen.getByRole("table")).toHaveTextContent("Loading…");
    unmount();
    const { onSelect } = renderPane(loading(["/a"]));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Loading…");
    await userEvent.click(screen.getByTestId("log-group-pane-option"));
    expect(onSelect).toHaveBeenCalledWith("/a");
  });

  it("distinguishes no groups from no matches", () => {
    const empty = (emptyState: "NoGroups" | "NoMatches") =>
      connectedView({
        logGroups: {
          status: "Complete",
          visibleGroups: [],
          totalCount: 0,
          emptyState,
          failure: null,
        },
      });
    const { unmount } = renderPane(empty("NoGroups"));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent(
      "There are no log groups.",
    );
    unmount();
    renderPane(empty("NoMatches"));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("No log groups match.");
  });

  it("shows a partial list as a warning alert with the reason, what to do and the safe detail", () => {
    renderPane(
      connectedView({
        logGroups: {
          status: "Partial",
          visibleGroups: ["/aws/ecs/web"],
          totalCount: 1,
          emptyState: null,
          failure: {
            kind: "AccessDenied",
            safeDetail: "kind=AccessDenied; api=DescribeLogGroups; profile=dev",
            retryable: false,
          },
        },
      }),
    );
    const alert = screen.getByTestId("log-group-pane-error");
    expect(within(alert).getByTestId("log-group-pane-partial")).toHaveTextContent(
      "The list is incomplete.",
    );
    expect(screen.getByTestId("log-group-pane-error-what")).toHaveTextContent("Access was denied.");
    expect(screen.getByTestId("log-group-pane-error-next")).toHaveTextContent(
      "Check the IAM permissions or choose another profile.",
    );
    expect(screen.getByTestId("log-group-pane-error-detail")).toHaveTextContent(
      "Details: kind=AccessDenied; api=DescribeLogGroups; profile=dev",
    );
    expect(alert).not.toHaveTextContent("Access denied");
    // The rows that did arrive are still listed.
    expect(screen.getByTestId("log-group-pane-option")).toHaveTextContent("/aws/ecs/web");
  });

  it("carries [Sign in with SSO] in an auth error of the list when the login is available (261011-sso-login FR1.1)", async () => {
    const authList = (available: boolean, kind: "AuthRequired" | "Network" = "AuthRequired") =>
      connectedView({
        logGroups: {
          status: "Partial",
          visibleGroups: [],
          totalCount: 0,
          emptyState: null,
          failure: { kind, safeDetail: "", retryable: false },
        },
        ssoLogin: ssoLoginView({ available, canStart: available }),
      });
    const onSsoLogin = vi.fn();
    const { rerender } = render(
      <LogGroupPane
        session={authList(true)}
        t={t}
        onFilterChange={vi.fn()}
        onReload={vi.fn()}
        onSelect={vi.fn()}
        onSsoLogin={onSsoLogin}
      />,
    );
    const button = within(screen.getByTestId("log-group-pane-error")).getByTestId(
      "log-group-pane-sso-login-button",
    );
    await userEvent.click(button);
    expect(onSsoLogin).toHaveBeenCalledTimes(1);
    for (const view of [authList(false), authList(true, "Network")]) {
      rerender(
        <LogGroupPane
          session={view}
          t={t}
          onFilterChange={vi.fn()}
          onReload={vi.fn()}
          onSelect={vi.fn()}
          onSsoLogin={onSsoLogin}
        />,
      );
      expect(screen.queryByTestId("log-group-pane-sso-login-button")).not.toBeInTheDocument();
    }
  });

  it("disables the rows and [Reload] while the core says a login runs (BR3.4)", () => {
    renderPane(
      connectedView({
        canChangeConnection: false,
        canReload: false,
        ssoLogin: ssoLoginView({ state: "Waiting" }),
      }),
    );
    expect(screen.getByTestId("log-group-pane-reload-button")).toBeDisabled();
    for (const radio of screen.getAllByRole("radio")) {
      expect(radio).toBeDisabled();
    }
  });

  it("points a failed listing to [Reload] and still says when nothing matches", () => {
    renderPane(
      connectedView({
        logGroupFilter: "zzz",
        logGroups: {
          status: "Partial",
          visibleGroups: [],
          totalCount: 3,
          emptyState: "NoMatches",
          failure: { kind: "Throttled", safeDetail: "kind=Throttled", retryable: true },
        },
      }),
    );
    expect(screen.getByTestId("log-group-pane-error-next")).toHaveTextContent(
      "Wait a while, then press [Reload] to load the list again.",
    );
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("No log groups match.");
  });

  it("cannot select while locked and disables Reload, but keeps the search usable while fetching", async () => {
    const { onSelect, unmount } = renderPane(
      connectedView({ canChangeConnection: false, canReload: false, phase: "Fetching" }),
    );
    expect(screen.getByTestId("log-group-pane-reload-button")).toBeDisabled();
    expect(screen.getByRole("radio", { name: "/aws/ecs/web" })).toBeDisabled();
    await userEvent.click(screen.getAllByTestId("log-group-pane-option")[0]!);
    expect(onSelect).not.toHaveBeenCalled();
    expect(searchBox()).toBeEnabled();
    unmount();
    const { onFilterChange } = renderPane(
      connectedView({
        pendingChange: { proposedProfile: { kind: "SdkDefault" }, proposedRegion: null },
        canChangeConnection: false,
        canReload: false,
      }),
    );
    expect(searchBox()).toBeDisabled();
    await userEvent.type(searchBox(), "x");
    expect(onFilterChange).not.toHaveBeenCalled();
  });

  it("shows the close button in the heading only when the pane can be closed, and calls back (gate fix 15b)", async () => {
    const { unmount } = renderPane();
    expect(screen.queryByTestId("log-group-pane-collapse-button")).not.toBeInTheDocument();
    unmount();
    const onCollapse = vi.fn();
    render(
      <LogGroupPane
        session={connectedView()}
        t={t}
        onFilterChange={vi.fn()}
        onReload={vi.fn()}
        onSelect={vi.fn()}
        onCollapse={onCollapse}
      />,
    );
    const button = screen.getByRole("button", { name: "Close the log group list" });
    // In the heading row: after the heading text, before the search field.
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.compareDocumentPosition(button) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(
      button.compareDocumentPosition(searchBox()) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    await userEvent.click(button);
    expect(onCollapse).toHaveBeenCalledTimes(1);
    // The names keep their full text as the title, for a narrow pane.
    expect(screen.getAllByTestId("log-group-pane-option")[0]).toHaveAttribute(
      "title",
      "/aws/ecs/web",
    );
  });

  it("orders Tab as search, table, then Reload (K7)", async () => {
    const user = userEvent.setup();
    renderPane(connectedView({ selectedLogGroupName: "/aws/ecs/web" }));
    await user.tab();
    expect(searchBox()).toHaveFocus();
    await user.tab();
    expect(screen.getByRole("radio", { name: "/aws/ecs/web" })).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("log-group-pane-reload-button")).toHaveFocus();
  });
});
