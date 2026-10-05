import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LogEvent, RowWindow } from "../api";
import { formatUtcMillis } from "../format";
import { logEvent, t } from "../test/fixtures";
import { LogTable } from "./LogTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getRows: vi.fn(), findRowPosition: vi.fn() };
});

const api = await import("../api");

/** Rows of a timeline of `total` events, made on demand. */
function answer(total: number, offset: number, limit: number, version = 1): RowWindow {
  const rows: LogEvent[] = [];
  for (let row = offset; row < Math.min(total, offset + limit); row += 1) {
    rows.push(logEvent(row));
  }
  return { offset, rows, totalCount: total, timelineVersion: version };
}

function serveRows(total: number) {
  vi.mocked(api.getRows).mockImplementation(async (offset, limit) => answer(total, offset, limit));
}

/** Viewport of 110 px: five 22 px rows, six drawn. */
const VIEWPORT = 110;

function renderTable(totalCount: number, timelineVersion = 1) {
  const onError = vi.fn();
  const view = render(
    <LogTable
      totalCount={totalCount}
      timelineVersion={timelineVersion}
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

  it("shows time, stream and the message on one line in three columns", async () => {
    const event = logEvent(7, "first line\nsecond line\r\nthird", "stream-b");
    vi.mocked(api.getRows).mockResolvedValue({
      offset: 0,
      rows: [event],
      totalCount: 1,
      timelineVersion: 1,
    });
    renderTable(1);
    const row = await screen.findByTestId("log-table-row");
    const cells = within(row).getAllByRole("cell");
    expect(cells.map((cell) => cell.textContent)).toEqual([
      "2024-01-02 03:04:05.007",
      "stream-b",
      "first line second line third",
    ]);
    expect(event.message).toBe("first line\nsecond line\r\nthird");
    for (const name of ["Time (UTC)", "Stream", "Message"]) {
      expect(screen.getByRole("columnheader", { name })).toBeInTheDocument();
    }
    expect(formatUtcMillis(0)).toBe("1970-01-01 00:00:00.000");
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
        t={t}
        onError={view.onError}
        viewportHeight={VIEWPORT}
      />,
    );

    await waitFor(() => expect(screen.getByTestId("log-table-viewport").scrollTop).toBe(1_325));
    expect(api.findRowPosition).toHaveBeenCalledWith("stream-a", 10);
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
});
