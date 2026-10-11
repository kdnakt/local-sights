import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { logEvent, rowWindow, t } from "../test/fixtures";
import { LogTableCloudscape } from "./LogTableCloudscape";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getRows: vi.fn() };
});

const api = await import("../api");

const events = [logEvent(0, "first"), logEvent(1, "second\nline two", "stream-b"), logEvent(2)];

describe("LogTableCloudscape (plan Step 10 prototype)", () => {
  beforeEach(() => {
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) =>
      rowWindow(events, offset, limit),
    );
  });

  it("reads the whole list from the core and shows time, stream and message", async () => {
    render(
      <LogTableCloudscape
        totalCount={3}
        timelineVersion={1}
        timeZone="Local"
        t={t}
        onError={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getAllByText("second line two").length).toBeGreaterThan(0));
    expect(api.getRows).toHaveBeenCalledWith(0, 3);
    // Expandable rows make the table a tree grid.
    expect(screen.getByRole("treegrid", { name: "Log events" })).toBeInTheDocument();
    expect(screen.getAllByText("core time 1").length).toBeGreaterThan(0);
    expect(screen.getAllByText("stream-b").length).toBeGreaterThan(0);
    expect(screen.getAllByRole("columnheader", { name: "Time (Local)" }).length).toBeGreaterThan(0);
  });

  it("opens the whole message below a row when the row is clicked, and closes it again", async () => {
    const user = userEvent.setup();
    render(
      <LogTableCloudscape
        totalCount={3}
        timelineVersion={1}
        timeZone="Local"
        t={t}
        onError={vi.fn()}
      />,
    );
    const [cell] = await screen.findAllByText("second line two");
    await user.click(cell!);
    expect(screen.getByTestId("log-table-expanded")).toHaveTextContent("second line two");
    expect(screen.getByTestId("log-table-expanded").textContent).toBe("second\nline two");
    await user.click(screen.getAllByText("second line two")[0]!);
    expect(screen.queryByTestId("log-table-expanded")).not.toBeInTheDocument();
  });

  it("shows the empty text and reads nothing when there are no rows", () => {
    render(
      <LogTableCloudscape
        totalCount={0}
        timelineVersion={1}
        timeZone="Local"
        t={t}
        onError={vi.fn()}
        emptyText="No events."
      />,
    );
    expect(screen.getByTestId("log-table-empty")).toHaveTextContent("No events.");
    expect(api.getRows).not.toHaveBeenCalled();
  });
});
