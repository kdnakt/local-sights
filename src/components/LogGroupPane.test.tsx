import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LogGroupPane } from "./LogGroupPane";
import { connectedView, sessionView, t } from "../test/fixtures";

function renderPane(view = connectedView()) {
  const handlers = { onFilterChange: vi.fn(), onReload: vi.fn(), onSelect: vi.fn() };
  render(<LogGroupPane session={view} t={t} {...handlers} />);
  return handlers;
}

describe("LogGroupPane", () => {
  it("prompts for a profile, then for a region", () => {
    const { unmount } = render(
      <LogGroupPane
        session={sessionView()}
        t={t}
        onFilterChange={vi.fn()}
        onReload={vi.fn()}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Choose a profile.");
    unmount();
    renderPane(sessionView({ connection: { profile: { kind: "SdkDefault" }, region: null } }));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Choose a region.");
    expect(screen.getByTestId("log-group-pane-reload-button")).toBeDisabled();
  });

  it("lists the groups, marks the selection and selects on click", async () => {
    const { onSelect } = renderPane(connectedView({ selectedLogGroupName: "/aws/ecs/web" }));
    const options = screen.getAllByTestId("log-group-pane-option");
    expect(options.map((o) => o.textContent)).toEqual(["/aws/ecs/web", "/aws/lambda/MyFunction"]);
    expect(options[0]).toHaveAttribute("aria-selected", "true");
    await userEvent.click(options[1]!);
    expect(onSelect).toHaveBeenCalledWith("/aws/lambda/MyFunction");
  });

  it("selects with the arrow keys and Enter or Space", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderPane();
    screen.getByTestId("log-group-pane-list").focus();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(onSelect).toHaveBeenLastCalledWith("/aws/lambda/MyFunction");
    await user.keyboard("{ArrowUp} ");
    expect(onSelect).toHaveBeenLastCalledWith("/aws/ecs/web");
  });

  it("forwards filter text and reload", async () => {
    const user = userEvent.setup();
    const { onFilterChange, onReload } = renderPane();
    await user.type(screen.getByTestId("log-group-pane-filter-input"), "ecs");
    expect(onFilterChange).toHaveBeenLastCalledWith("ecs");
    await user.click(screen.getByTestId("log-group-pane-reload-button"));
    expect(onReload).toHaveBeenCalledTimes(1);
  });

  it("shows loading while groups stay selectable", async () => {
    const { onSelect } = renderPane(
      connectedView({
        logGroups: {
          status: "Loading",
          visibleGroups: ["/a"],
          totalCount: 1,
          emptyState: null,
          failure: null,
        },
      }),
    );
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("Loading…");
    await userEvent.click(screen.getByTestId("log-group-pane-option"));
    expect(onSelect).toHaveBeenCalledWith("/a");
  });

  it("shows a partial list with the kind and safe detail", () => {
    renderPane(
      connectedView({
        logGroups: {
          status: "Partial",
          visibleGroups: [],
          totalCount: 0,
          emptyState: null,
          failure: {
            kind: "AccessDenied",
            safeDetail: "kind=AccessDenied; api=DescribeLogGroups; profile=dev",
            retryable: false,
          },
        },
      }),
    );
    expect(screen.getByTestId("log-group-pane-partial")).toHaveTextContent(
      "The list is incomplete: Access denied",
    );
    expect(screen.getByTestId("log-group-pane-detail")).toHaveTextContent(
      "kind=AccessDenied; api=DescribeLogGroups; profile=dev",
    );
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
    const { unmount } = render(
      <LogGroupPane
        session={empty("NoGroups")}
        t={t}
        onFilterChange={vi.fn()}
        onReload={vi.fn()}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent(
      "There are no log groups.",
    );
    unmount();
    renderPane(empty("NoMatches"));
    expect(screen.getByTestId("log-group-pane-status")).toHaveTextContent("No log groups match.");
  });

  it("ignores selection and disables reload while locked", async () => {
    const { onSelect } = renderPane(
      connectedView({ canChangeConnection: false, canReload: false }),
    );
    expect(screen.getByTestId("log-group-pane-reload-button")).toBeDisabled();
    await userEvent.click(screen.getAllByTestId("log-group-pane-option")[0]!);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("orders Tab as filter, list, then reload", async () => {
    const user = userEvent.setup();
    renderPane();
    for (const testId of [
      "log-group-pane-filter-input",
      "log-group-pane-list",
      "log-group-pane-reload-button",
    ]) {
      await user.tab();
      expect(screen.getByTestId(testId)).toHaveFocus();
    }
  });
});
