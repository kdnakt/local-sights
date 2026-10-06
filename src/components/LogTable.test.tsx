import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DisplayRow, RowPosition, RowWindow, TimeZoneChoice } from "../api";
import { logEvent, t } from "../test/fixtures";
import { LogTable } from "./LogTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getRows: vi.fn(), findRowPosition: vi.fn() };
});

const api = await import("../api");

/** Rows of a timeline of `total` events, made on demand. */
function answer(total: number, offset: number, limit: number, version = 1): RowWindow {
  const rows: DisplayRow[] = [];
  for (let row = offset; row < Math.min(total, offset + limit); row += 1) {
    rows.push(logEvent(row));
  }
  return {
    offset,
    rows,
    totalCount: total,
    timelineVersion: version,
    filtered: false,
    allCount: total,
    resultVersion: null,
  };
}

function serveRows(total: number) {
  vi.mocked(api.getRows).mockImplementation(async (offset, limit) => answer(total, offset, limit));
}

/** Viewport of 110 px: five 22 px rows, six drawn. */
const VIEWPORT = 110;

function renderTable(totalCount: number, timelineVersion = 1, timeZone: TimeZoneChoice = "Local") {
  const onError = vi.fn();
  const view = render(
    <LogTable
      totalCount={totalCount}
      timelineVersion={timelineVersion}
      timeZone={timeZone}
      t={t}
      onError={onError}
      viewportHeight={VIEWPORT}
    />,
  );
  return { ...view, onError };
}

function messages(): string[] {
  return screen
    .queryAllByTestId("log-table-row")
    .map((row) => row.lastElementChild?.textContent ?? "");
}

async function scrollTo(top: number) {
  const viewport = screen.getByTestId("log-table-viewport");
  viewport.scrollTop = top;
  fireEvent.scroll(viewport);
  await waitFor(() => expect(api.getRows).toHaveBeenLastCalledWith(Math.floor(top / 22), 6));
}

describe("LogTable", () => {
  beforeEach(() => {
    vi.mocked(api.getRows).mockReset();
    vi.mocked(api.findRowPosition).mockReset();
  });

  it("draws only the rows of the viewport out of a million", async () => {
    serveRows(1_000_000);
    renderTable(1_000_000);
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(6));
    expect(api.getRows).toHaveBeenCalledWith(0, 6);
    expect(messages()).toEqual([0, 1, 2, 3, 4, 5].map((n) => `message ${n}`));
    expect(screen.getByTestId("log-table")).toHaveAttribute("aria-rowcount", "1000000");
  });

  it("shows the core's time, stream and the message on one line in three columns", async () => {
    // The time is the core's text, unrelated to what the timestamp would give
    // in any zone: the screen converts nothing (U4:BR3.4).
    const event = logEvent(
      7,
      "first line\nsecond line\r\nthird",
      "stream-b",
      "2030-12-31 23:59:59.999",
    );
    vi.mocked(api.getRows).mockResolvedValue({
      offset: 0,
      rows: [event],
      totalCount: 1,
      timelineVersion: 1,
      filtered: false,
      allCount: 1,
      resultVersion: null,
    });
    renderTable(1);
    const row = await screen.findByTestId("log-table-row");
    const cells = within(row).getAllByRole("cell");
    expect(cells.map((cell) => cell.textContent)).toEqual([
      "2030-12-31 23:59:59.999",
      "stream-b",
      "first line second line third",
    ]);
    expect(event.message).toBe("first line\nsecond line\r\nthird");
    for (const name of ["Time (Local)", "Stream", "Message"]) {
      expect(screen.getByRole("columnheader", { name })).toBeInTheDocument();
    }
  });

  it("shows a raw number when the core could not write the time", async () => {
    vi.mocked(api.getRows).mockResolvedValue({
      offset: 0,
      rows: [logEvent(0, "far future", "stream-a", "253402300800000")],
      totalCount: 1,
      timelineVersion: 1,
      filtered: false,
      allCount: 1,
      resultVersion: null,
    });
    renderTable(1, 1, "Utc");
    const row = await screen.findByTestId("log-table-row");
    expect(within(row).getAllByRole("cell")[0]).toHaveTextContent("253402300800000");
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
  });

  it("re-reads the rows in place when the time zone is switched", async () => {
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) => {
      const rows: DisplayRow[] = [];
      for (let row = offset; row < Math.min(3, offset + limit); row += 1) {
        rows.push(logEvent(row, `message ${row}`, "stream-a", `local ${row}`));
      }
      return {
        offset,
        rows,
        totalCount: 3,
        timelineVersion: 1,
        filtered: false,
        allCount: 3,
        resultVersion: null,
      };
    });
    const view = renderTable(3, 1, "Local");
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(screen.getAllByRole("cell")[0]).toHaveTextContent("local 0");

    vi.mocked(api.getRows).mockImplementation(async (offset, limit) => {
      const rows: DisplayRow[] = [];
      for (let row = offset; row < Math.min(3, offset + limit); row += 1) {
        rows.push(logEvent(row, `message ${row}`, "stream-a", `utc ${row}`));
      }
      return {
        offset,
        rows,
        totalCount: 3,
        timelineVersion: 1,
        filtered: false,
        allCount: 3,
        resultVersion: null,
      };
    });
    view.rerender(
      <LogTable
        totalCount={3}
        timelineVersion={1}
        timeZone="Utc"
        t={t}
        onError={view.onError}
        viewportHeight={VIEWPORT}
      />,
    );
    await waitFor(() => expect(screen.getAllByRole("cell")[0]).toHaveTextContent("utc 0"));
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
    expect(vi.mocked(api.getRows).mock.calls).toEqual([
      [0, 3],
      [0, 3],
    ]);
    expect(messages()).toEqual(["message 0", "message 1", "message 2"]);
    expect(api.findRowPosition).not.toHaveBeenCalled();
  });

  it("scrolls by row, by page and to either end with the keyboard", async () => {
    serveRows(100_000);
    const user = userEvent.setup();
    renderTable(100_000);
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    screen.getByTestId("log-table-viewport").focus();

    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(messages()[0]).toBe("message 1"));
    await user.keyboard("{PageDown}");
    await waitFor(() => expect(messages()[0]).toBe("message 6"));
    await user.keyboard("{End}");
    await waitFor(() => expect(messages().at(-1)).toBe("message 99999"));
    await user.keyboard("{PageUp}");
    await waitFor(() => expect(messages()[0]).toBe("message 99990"));
    await user.keyboard("{Home}");
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    await user.keyboard("{ArrowUp}");
    expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(0);
  });

  it("keeps the top row in place when rows are added above it", async () => {
    serveRows(1_000);
    const view = renderTable(1_000, 1);
    await scrollTo(225);
    await waitFor(() => expect(messages()[0]).toBe("message 10"));
    vi.mocked(api.findRowPosition).mockResolvedValue({ position: 60, timelineVersion: 2 });
    serveRows(1_050);

    view.rerender(
      <LogTable
        totalCount={1_050}
        timelineVersion={2}
        timeZone="Local"
        t={t}
        onError={view.onError}
        viewportHeight={VIEWPORT}
      />,
    );

    await waitFor(() => expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(1_325));
    expect(api.findRowPosition).toHaveBeenCalledWith("stream-a", 10);
  });

  it("keeps the same anchor row when a second version arrives before the reply", async () => {
    serveRows(1_000);
    const view = renderTable(1_000, 1);
    await scrollTo(225);
    await waitFor(() => expect(messages()[0]).toBe("message 10"));

    const replies: Array<(position: RowPosition) => void> = [];
    vi.mocked(api.findRowPosition).mockImplementation(
      () =>
        new Promise<RowPosition>((resolve) => {
          replies.push(resolve);
        }),
    );
    // 50 events of another stream were inserted at the top: the row now at
    // position 10 is a different event, so taking the anchor again from the
    // shown rows would pick the wrong one.
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) => {
      const rows: DisplayRow[] = [];
      for (let row = offset; row < offset + limit; row += 1) {
        rows.push(row < 50 ? logEvent(row, `other ${row}`, "stream-x") : logEvent(row - 50));
      }
      return {
        offset,
        rows,
        totalCount: 1_050,
        timelineVersion: 2,
        filtered: false,
        allCount: 1_050,
        resultVersion: null,
      };
    });
    const rerender = (totalCount: number, timelineVersion: number) =>
      view.rerender(
        <LogTable
          totalCount={totalCount}
          timelineVersion={timelineVersion}
          timeZone="Local"
          t={t}
          onError={view.onError}
          viewportHeight={VIEWPORT}
        />,
      );
    rerender(1_050, 2);
    await waitFor(() => expect(messages()[0]).toBe("other 10"));
    rerender(1_100, 3);
    await waitFor(() => expect(api.findRowPosition).toHaveBeenCalledTimes(2));
    expect(vi.mocked(api.findRowPosition).mock.calls).toEqual([
      ["stream-a", 10],
      ["stream-a", 10],
    ]);

    await act(async () => {
      replies[1]!({ position: 110, timelineVersion: 3 });
    });
    expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(2_425);
    await act(async () => {
      replies[0]!({ position: 60, timelineVersion: 2 });
    });
    expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(2_425);
  });

  it("stays at the top when scrolled to the top", async () => {
    serveRows(1_000);
    const view = renderTable(1_000, 1);
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    serveRows(1_200);
    view.rerender(
      <LogTable
        totalCount={1_200}
        timelineVersion={2}
        timeZone="Local"
        t={t}
        onError={view.onError}
        viewportHeight={VIEWPORT}
      />,
    );
    await waitFor(() => expect(api.getRows).toHaveBeenCalledTimes(2));
    expect(api.findRowPosition).not.toHaveBeenCalled();
    expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(0);
  });

  it("returns to the top with no rows when the logs are discarded", async () => {
    serveRows(1_000);
    const view = renderTable(1_000, 1);
    await scrollTo(440);
    await waitFor(() => expect(messages()[0]).toBe("message 20"));
    view.rerender(
      <LogTable
        totalCount={0}
        timelineVersion={2}
        timeZone="Local"
        t={t}
        onError={view.onError}
        viewportHeight={VIEWPORT}
      />,
    );
    await waitFor(() => expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(0));
    expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0);
  });

  it("ignores a late answer to an older request", async () => {
    let answerFirst: (window: RowWindow) => void = () => undefined;
    vi.mocked(api.getRows)
      .mockImplementationOnce(
        () =>
          new Promise<RowWindow>((resolve) => {
            answerFirst = resolve;
          }),
      )
      .mockImplementation(async (offset, limit) => answer(1_000, offset, limit));
    renderTable(1_000);
    await scrollTo(220);
    await waitFor(() => expect(messages()[0]).toBe("message 10"));
    await act(async () => {
      answerFirst(answer(1_000, 0, 6));
    });
    expect(messages()[0]).toBe("message 10");
  });

  it("keeps the top row for a new filter result and goes to the top for a new filter", async () => {
    serveRows(1_000);
    const view = renderTable(1_000, 1);
    await scrollTo(440);
    await waitFor(() => expect(messages()[0]).toBe("message 20"));
    vi.mocked(api.findRowPosition).mockResolvedValue({ position: 30, timelineVersion: 1 });
    const rerender = (filterId: number | null, resultVersion: number) =>
      view.rerender(
        <LogTable
          totalCount={1_000}
          timelineVersion={1}
          timeZone="Local"
          filterId={filterId}
          resultVersion={resultVersion}
          t={t}
          onError={view.onError}
          viewportHeight={VIEWPORT}
        />,
      );
    // Same filter, a new result version: the anchor row is looked up again.
    rerender(null, 4);
    await waitFor(() => expect(api.findRowPosition).toHaveBeenCalledWith("stream-a", 20));
    await waitFor(() => expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(30 * 22));
    const reads = vi.mocked(api.getRows).mock.calls.length;
    const lookups = vi.mocked(api.findRowPosition).mock.calls.length;
    // A new filter: back to the top, no anchor, the rows are read again.
    rerender(7, 5);
    await waitFor(() => expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(0));
    expect(api.findRowPosition).toHaveBeenCalledTimes(lookups);
    await waitFor(() => expect(vi.mocked(api.getRows).mock.calls.length).toBeGreaterThan(reads));
  });
});
