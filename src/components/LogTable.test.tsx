import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DisplayRow, RowKeyParts, RowPositions, RowWindow, TimeZoneChoice } from "../api";
import { logEvent, t } from "../test/fixtures";
import { LogTable, type LogTableProps } from "./LogTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getRows: vi.fn(), rowPositions: vi.fn(), findRowPosition: vi.fn() };
});

const api = await import("../api");

/** jsdom has no ResizeObserver: a fake whose callbacks the tests fire. */
class FakeResizeObserver {
  static all = new Set<FakeResizeObserver>();
  targets = new Set<Element>();
  constructor(private readonly callback: ResizeObserverCallback) {
    FakeResizeObserver.all.add(this);
  }
  observe(target: Element) {
    this.targets.add(target);
  }
  unobserve(target: Element) {
    this.targets.delete(target);
  }
  disconnect() {
    this.targets.clear();
    FakeResizeObserver.all.delete(this);
  }
  static resize(target: Element, blockSize: number) {
    for (const observer of FakeResizeObserver.all) {
      if (observer.targets.has(target)) {
        const entry = { target, borderBoxSize: [{ blockSize, inlineSize: 700 }] };
        observer.callback(
          [entry as unknown as ResizeObserverEntry],
          observer as unknown as ResizeObserver,
        );
      }
    }
  }
}

/** Rows of a timeline of `total` events, made on demand. */
function answer(
  total: number,
  offset: number,
  limit: number,
  make: (row: number) => DisplayRow = (row) => logEvent(row),
  version = 1,
): RowWindow {
  const rows: DisplayRow[] = [];
  for (let row = offset; row < Math.min(total, offset + limit); row += 1) {
    rows.push(make(row));
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

function serveRows(total: number, make?: (row: number) => DisplayRow) {
  vi.mocked(api.getRows).mockImplementation(async (offset, limit) =>
    answer(total, offset, limit, make),
  );
}

/** `row_positions` answers from a table `stream/sequence → position`. */
function servePositions(
  table: Record<string, number | null>,
  versions: Partial<RowPositions> = {},
) {
  vi.mocked(api.rowPositions).mockImplementation(async (keys: RowKeyParts[]) => ({
    positions: keys.map((key) => table[`${key.logStreamName}/${key.sequence}`] ?? null),
    timelineVersion: 1,
    resultVersion: 0,
    discardGeneration: 0,
    totalCount: 1_000,
    ...versions,
  }));
}

/** Viewport of 110 px (five 22 px rows), 712 px wide: 100 columns of 7 px. */
const VIEWPORT = 110;
const WIDTH = 712;

function renderTable(props: Partial<LogTableProps> & { totalCount: number }) {
  const onError = vi.fn();
  const all = (next: Partial<LogTableProps> & { totalCount: number }): LogTableProps => ({
    timelineVersion: 1,
    timeZone: "Local" as TimeZoneChoice,
    t,
    onError,
    viewportHeight: VIEWPORT,
    viewportWidth: WIDTH,
    charWidth: 7,
    ...next,
  });
  const view = render(<LogTable {...all(props)} />);
  const rerender = (next: Partial<LogTableProps> & { totalCount: number }) =>
    view.rerender(<LogTable {...all(next)} />);
  return { ...view, onError, rerender };
}

function rows(): HTMLElement[] {
  return screen.queryAllByTestId("log-table-row");
}

function messages(): string[] {
  return rows().map((row) => within(row).getAllByRole("gridcell")[2]?.textContent ?? "");
}

function rowOf(message: string): HTMLElement {
  const row = rows().find(
    (element) => within(element).getAllByRole("gridcell")[2]?.textContent === message,
  );
  if (row === undefined) {
    throw new Error(`no row ${message}`);
  }
  return row;
}

function viewport(): HTMLElement {
  return screen.getByTestId("log-table-viewport");
}

async function scrollTo(top: number) {
  viewport().scrollTop = top;
  fireEvent.scroll(viewport());
  await waitFor(() => expect(messages()[0]).toBe(`message ${Math.floor(top / 22)}`));
}

describe("LogTable", () => {
  beforeEach(() => {
    vi.mocked(api.getRows).mockReset();
    vi.mocked(api.rowPositions).mockReset();
    servePositions({});
    vi.stubGlobal("ResizeObserver", FakeResizeObserver);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    FakeResizeObserver.all.clear();
  });

  it("draws only the rows of the viewport out of a million in a grid with a header row", async () => {
    serveRows(1_000_000);
    renderTable({ totalCount: 1_000_000 });
    await waitFor(() => expect(rows()).toHaveLength(6));
    expect(api.getRows).toHaveBeenCalledWith(0, 6);
    expect(messages()).toEqual([0, 1, 2, 3, 4, 5].map((n) => `message ${n}`));
    const grid = screen.getByRole("grid", { name: "Log events" });
    expect(grid).toHaveAttribute("aria-rowcount", "1000001");
    expect(grid).toHaveAttribute("tabindex", "0");
    const header = within(grid).getAllByRole("row")[0];
    expect(header).toHaveAttribute("aria-rowindex", "1");
    expect(within(header!).getAllByRole("columnheader")).toHaveLength(3);
    expect(rows()[2]).toHaveAttribute("aria-rowindex", "4");
    expect(rows()[0]).toHaveAttribute("aria-expanded", "false");
    expect(viewport()).not.toHaveAttribute("tabindex");
  });

  it("shows the core's time, stream and the message on one line in three columns", async () => {
    // The time is the core's text: the screen converts nothing (U4:BR3.4).
    const event = logEvent(
      7,
      "first line\nsecond line\r\nthird",
      "stream-b",
      "2030-12-31 23:59:59.999",
    );
    serveRows(1, () => event);
    renderTable({ totalCount: 1 });
    const row = await screen.findByTestId("log-table-row");
    expect(
      within(row)
        .getAllByRole("gridcell")
        .map((cell) => cell.textContent),
    ).toEqual(["2030-12-31 23:59:59.999", "stream-b", "first line second line third"]);
    for (const name of ["Time (Local)", "Stream", "Message"]) {
      expect(screen.getByRole("columnheader", { name })).toBeInTheDocument();
    }
  });

  it("re-reads the rows in place when the time zone is switched", async () => {
    serveRows(3, (row) => logEvent(row, `message ${row}`, "stream-a", `local ${row}`));
    const view = renderTable({ totalCount: 3, timeZone: "Local" });
    await waitFor(() => expect(rows()).toHaveLength(3));
    expect(within(rows()[0]!).getAllByRole("gridcell")[0]).toHaveTextContent("local 0");
    serveRows(3, (row) => logEvent(row, `message ${row}`, "stream-a", `utc ${row}`));
    view.rerender({ totalCount: 3, timeZone: "Utc" });
    await waitFor(() =>
      expect(within(rows()[0]!).getAllByRole("gridcell")[0]).toHaveTextContent("utc 0"),
    );
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
    expect(vi.mocked(api.getRows).mock.calls).toEqual([
      [0, 3],
      [0, 3],
    ]);
    expect(api.rowPositions).not.toHaveBeenCalled();
  });

  it("opens the whole message as text below the row, several at once, and closes it again", async () => {
    const html = '<b>bold</b> {"a":1,"b":[2]}\nsecond line';
    serveRows(10, (row) => logEvent(row, row === 1 ? html : `message ${row}`));
    const user = userEvent.setup();
    renderTable({ totalCount: 10 });
    await waitFor(() => expect(rows().length).toBeGreaterThan(2));
    const line = within(rows()[1]!).getByTestId("log-table-row-line");
    await user.click(line);
    const expanded = within(rows()[1]!).getByTestId("log-table-expanded");
    // A text node only: not parsed as HTML, JSON as returned, line break kept.
    expect(expanded.textContent).toBe(html);
    expect(expanded.querySelector("b")).toBeNull();
    expect(expanded.parentElement).toHaveAttribute("role", "gridcell");
    expect(expanded.parentElement).toHaveAttribute("aria-colspan", "3");
    expect(rows()[1]).toHaveAttribute("aria-expanded", "true");
    // The next row sits below the expansion (2 lines: 2 × 18 + 12 = 48 px).
    expect(rows()[2]!.style.top).toBe(`${2 * 22 + 48}px`);

    await user.click(within(rowOf("message 3")).getByTestId("log-table-row-line"));
    expect(screen.getAllByTestId("log-table-expanded")).toHaveLength(2);
    // Pressing inside an expansion selects text; it never closes the row.
    await user.click(expanded);
    expect(screen.getAllByTestId("log-table-expanded")).toHaveLength(2);
    await user.click(within(rows()[1]!).getByTestId("log-table-row-line"));
    expect(screen.getAllByTestId("log-table-expanded")).toHaveLength(1);
    expect(rows()[1]).toHaveAttribute("aria-expanded", "false");
  });

  it("stops an expansion at 20 lines (372 px) and uses its measured height", async () => {
    const long = Array.from({ length: 30 }, (_, i) => `line ${i}`).join("\n");
    serveRows(10, (row) => logEvent(row, row === 0 ? long : `message ${row}`));
    const user = userEvent.setup();
    renderTable({ totalCount: 10, viewportHeight: 600 });
    await waitFor(() => expect(rows().length).toBeGreaterThan(1));
    await user.click(within(rows()[0]!).getByTestId("log-table-row-line"));
    await waitFor(() => expect(rows()[1]!.style.top).toBe(`${22 + 372}px`));
    // The selected row's expansion scrolls: Tab can enter it (review R-13).
    expect(screen.getByTestId("log-table-expanded")).toHaveAttribute("tabindex", "0");
    act(() => FakeResizeObserver.resize(screen.getByTestId("log-table-expanded"), 300));
    await waitFor(() => expect(rows()[1]!.style.top).toBe(`${22 + 300}px`));
  });

  it("selects rows with the arrow keys, Page Up, Page Down, Home and End and opens them with Enter and Space", async () => {
    serveRows(1_000_000);
    const user = userEvent.setup();
    renderTable({ totalCount: 1_000_000 });
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    const grid = screen.getByTestId("log-table");
    grid.focus();
    expect(grid).toHaveAttribute("aria-activedescendant", "log-table-row-0");

    await user.keyboard("{ArrowDown}");
    expect(rowOf("message 1")).toHaveAttribute("aria-selected", "true");
    expect(grid).toHaveAttribute("aria-activedescendant", "log-table-row-1");
    await user.keyboard("{Enter}");
    expect(rowOf("message 1")).toHaveAttribute("aria-expanded", "true");
    // Down goes to the next row, not into the expansion.
    await user.keyboard("{ArrowDown}");
    expect(rowOf("message 2")).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{ArrowUp}");
    await user.keyboard(" ");
    expect(rowOf("message 1")).toHaveAttribute("aria-expanded", "false");

    await user.keyboard("{PageDown}");
    await waitFor(() => expect(rowOf("message 6")).toHaveAttribute("aria-selected", "true"));
    await user.keyboard("{End}");
    await waitFor(() => expect(messages().at(-1)).toBe("message 999999"));
    await waitFor(() =>
      expect(grid).toHaveAttribute("aria-activedescendant", "log-table-row-999999"),
    );
    await user.keyboard("{Home}");
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    expect(grid).toHaveAttribute("aria-activedescendant", "log-table-row-0");
    expect(viewport().scrollTop).toBe(0);
  });

  it("leaves the keys to a focused expansion and goes back to the list with Escape", async () => {
    const long = Array.from({ length: 30 }, (_, i) => `line ${i}`).join("\n");
    serveRows(10, (row) => logEvent(row, row === 0 ? long : `message ${row}`));
    const user = userEvent.setup();
    renderTable({ totalCount: 10 });
    await waitFor(() => expect(rows().length).toBeGreaterThan(1));
    await user.click(within(rows()[0]!).getByTestId("log-table-row-line"));
    const grid = screen.getByTestId("log-table");
    expect(grid).toHaveFocus();
    await user.tab();
    const expanded = screen.getByTestId("log-table-expanded");
    expect(expanded).toHaveFocus();
    await user.keyboard("{ArrowDown}{Enter} ");
    expect(rows()[0]).toHaveAttribute("aria-selected", "true");
    expect(rows()[0]).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{Escape}");
    expect(grid).toHaveFocus();
    await user.tab();
    expect(expanded).toHaveFocus();
    await user.tab({ shift: true });
    expect(grid).toHaveFocus();
    // Another row selected: its expansion no longer takes the focus.
    await user.keyboard("{ArrowDown}");
    expect(expanded).toHaveAttribute("tabindex", "-1");
  });

  it("does not move the row at the top when a row below it is opened", async () => {
    serveRows(1_000);
    const user = userEvent.setup();
    renderTable({ totalCount: 1_000 });
    await scrollTo(225);
    await user.click(within(rowOf("message 11")).getByTestId("log-table-row-line"));
    expect(screen.getAllByTestId("log-table-expanded")).toHaveLength(1);
    expect(viewport().scrollTop).toBe(225);
    expect(messages()[0]).toBe("message 10");
  });

  it("keeps the top row in place when rows are added above it", async () => {
    serveRows(1_000);
    const view = renderTable({ totalCount: 1_000 });
    await scrollTo(225);
    servePositions({ "stream-a/10": 60 }, { timelineVersion: 2, totalCount: 1_050 });
    serveRows(1_050, (row) =>
      row < 50 ? logEvent(row, `other ${row}`, "stream-x") : logEvent(row - 50),
    );
    view.rerender({ totalCount: 1_050, timelineVersion: 2 });
    await waitFor(() => expect(viewport().scrollTop).toBe(60 * 22 + 5));
    expect(api.rowPositions).toHaveBeenCalledWith(
      expect.arrayContaining([{ logStreamName: "stream-a", sequence: 10 }]),
    );
  });

  it("asks once at a time and places the same top row after a second version", async () => {
    serveRows(1_000);
    const view = renderTable({ totalCount: 1_000 });
    await scrollTo(225);
    const replies: Array<(positions: RowPositions) => void> = [];
    vi.mocked(api.rowPositions).mockImplementation(
      () =>
        new Promise<RowPositions>((resolve) => {
          replies.push(resolve);
        }),
    );
    // 50 rows of another stream were inserted at the top.
    serveRows(1_050, (row) =>
      row < 50 ? logEvent(row, `other ${row}`, "stream-x") : logEvent(row - 50),
    );
    view.rerender({ totalCount: 1_050, timelineVersion: 2 });
    await waitFor(() => expect(api.rowPositions).toHaveBeenCalledTimes(1));
    view.rerender({ totalCount: 1_100, timelineVersion: 3 });
    // Still one call in flight.
    expect(api.rowPositions).toHaveBeenCalledTimes(1);
    await act(async () => {
      replies[0]!({
        positions: [60],
        timelineVersion: 2,
        resultVersion: 0,
        discardGeneration: 0,
        totalCount: 1_050,
      });
    });
    await waitFor(() => expect(api.rowPositions).toHaveBeenCalledTimes(2));
    for (const [keys] of vi.mocked(api.rowPositions).mock.calls) {
      expect(keys).toContainEqual({ logStreamName: "stream-a", sequence: 10 });
    }
    await act(async () => {
      replies[1]!({
        positions: (vi.mocked(api.rowPositions).mock.calls[1]![0] as RowKeyParts[]).map(() => 110),
        timelineVersion: 3,
        resultVersion: 0,
        discardGeneration: 0,
        totalCount: 1_100,
      });
    });
    await waitFor(() => expect(viewport().scrollTop).toBe(110 * 22 + 5));
  });

  it("keeps rows open across a filter and added rows, and closes them all when the logs are discarded", async () => {
    serveRows(10);
    const user = userEvent.setup();
    const view = renderTable({ totalCount: 10 });
    await waitFor(() => expect(rows().length).toBeGreaterThan(3));
    await user.click(within(rowOf("message 3")).getByTestId("log-table-row-line"));
    // A filter: row 3 is now the second row of the result.
    serveRows(4, (row) => logEvent([1, 3, 5, 7][row]!));
    servePositions({ "stream-a/3": 1 }, { resultVersion: 2, totalCount: 4 });
    view.rerender({ totalCount: 4, filterId: 7, resultVersion: 2 });
    await waitFor(() =>
      expect(messages()).toEqual(["message 1", "message 3", "message 5", "message 7"]),
    );
    await waitFor(() => expect(rowOf("message 3")).toHaveAttribute("aria-expanded", "true"));
    expect(rows()[2]!.style.top).toBe(`${2 * 22 + 30}px`);
    // Rows added (timeline version 2), filter cleared: still open.
    serveRows(20);
    servePositions({ "stream-a/3": 3 }, { timelineVersion: 2, resultVersion: 3, totalCount: 20 });
    view.rerender({ totalCount: 20, timelineVersion: 2, filterId: null, resultVersion: 3 });
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    await waitFor(() => expect(rowOf("message 3")).toHaveAttribute("aria-expanded", "true"));
    // A new fetch discarded the held logs: everything closes, top row selected.
    view.rerender({
      totalCount: 0,
      timelineVersion: 3,
      resultVersion: 3,
      discardGeneration: 1,
    });
    await waitFor(() => expect(rows()).toHaveLength(0));
    serveRows(20);
    view.rerender({ totalCount: 20, timelineVersion: 4, resultVersion: 3, discardGeneration: 1 });
    await waitFor(() => expect(messages()[0]).toBe("message 0"));
    expect(screen.queryAllByTestId("log-table-expanded")).toHaveLength(0);
    expect(rows()[0]).toHaveAttribute("aria-selected", "true");
    expect(viewport().scrollTop).toBe(0);
  });

  it("selects the row at the same position when the selected row is hidden", async () => {
    serveRows(10);
    const user = userEvent.setup();
    const view = renderTable({ totalCount: 10 });
    await waitFor(() => expect(rows().length).toBeGreaterThan(3));
    await user.click(within(rowOf("message 2")).getByTestId("log-table-row-line"));
    await user.click(within(rowOf("message 2")).getByTestId("log-table-row-line"));
    serveRows(5, (row) => logEvent(row * 2 + 1));
    servePositions({ "stream-a/2": null }, { resultVersion: 2, totalCount: 5 });
    view.rerender({ totalCount: 5, filterId: 9, resultVersion: 2 });
    await waitFor(() => expect(messages()[0]).toBe("message 1"));
    await waitFor(() => expect(rowOf("message 5")).toHaveAttribute("aria-selected", "true"));
  });

  it("ignores a late answer to an older row request", async () => {
    let answerFirst: (window: RowWindow) => void = () => undefined;
    vi.mocked(api.getRows)
      .mockImplementationOnce(
        () =>
          new Promise<RowWindow>((resolve) => {
            answerFirst = resolve;
          }),
      )
      .mockImplementation(async (offset, limit) => answer(1_000, offset, limit));
    renderTable({ totalCount: 1_000 });
    await scrollTo(220);
    await act(async () => {
      answerFirst(answer(1_000, 0, 6));
    });
    expect(messages()[0]).toBe("message 10");
  });

  it("keeps the top row for a new filter result and goes to the top for a new filter", async () => {
    serveRows(1_000);
    const view = renderTable({ totalCount: 1_000 });
    await scrollTo(440);
    servePositions({ "stream-a/20": 30 }, { resultVersion: 4 });
    view.rerender({ totalCount: 1_000, resultVersion: 4 });
    await waitFor(() => expect(viewport().scrollTop).toBe(30 * 22));
    view.rerender({ totalCount: 1_000, filterId: 7, resultVersion: 5 });
    await waitFor(() => expect(viewport().scrollTop).toBe(0));
  });
});
