import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueryRow, QueryRowDetail, ResultSummary } from "../api";
import { createTranslator } from "../i18n/messages";
import {
  aggregateRow,
  aggregatesSummary,
  queryRow,
  queryRowWindow,
  resultSummary,
  t,
} from "../test/fixtures";
import { QueryResultTable } from "./QueryResultTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return {
    ...original,
    getQueryRows: vi.fn(),
    getQueryRowDetail: vi.fn(),
    findQueryRow: vi.fn(),
  };
});

const api = await import("../api");

const RESULT: ResultSummary = resultSummary({
  runId: 3,
  rowCount: 500,
  evaluatedCount: 9_000,
});

const rows: QueryRow[] = Array.from({ length: 500 }, (_, index) =>
  index === 1
    ? {
        ...queryRow(1, "ERROR first line\n  at second line"),
        logStreamName: "2024/01/02/[$LATEST]abc",
      }
    : queryRow(index),
);

/** The fields of an opened row as the core lists them (U2:BR7.3). */
function detailOf(runId: number, index: number, version = 1): QueryRowDetail {
  const row = rows[index]!;
  return {
    runId,
    version,
    index,
    fields: [
      { name: "@timestamp", value: row.displayTime ?? null },
      { name: "@log", value: row.logGroupName ?? null },
      { name: "@logStream", value: row.logStreamName ?? null },
      { name: "@message", value: row.message ?? null },
      { name: "`user.id`", value: "u1" },
      { name: "http.status", value: "500" },
      { name: "z", value: null },
    ],
  };
}

function renderTable(result: ResultSummary = RESULT, locale: "en" | "ja" = "en") {
  const onError = vi.fn();
  render(
    <QueryResultTable
      result={result}
      timeZone="Utc"
      t={locale === "en" ? t : createTranslator("ja")}
      onError={onError}
    />,
  );
  return onError;
}

describe("QueryResultTable (261017-insights-query U1:BR6.3, BR6.4, U2:BR7.1-BR7.4)", () => {
  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    // 261025-insights-stats U3:BR2.4: the rows are read with (run, version).
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId, version),
    );
    vi.mocked(api.getQueryRowDetail).mockReset();
    vi.mocked(api.getQueryRowDetail).mockImplementation(async (runId, version, index) =>
      detailOf(runId, index, version),
    );
  });

  it("reads only the rows in view, for the shown run", async () => {
    renderTable();
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row").length).toBeGreaterThan(0),
    );
    const [runId, , offset, limit] = vi.mocked(api.getQueryRows).mock.calls.at(-1)!;
    expect([runId, offset]).toEqual([3, 0]);
    expect(limit).toBeLessThan(RESULT.rowCount);
    expect(screen.getAllByTestId("query-result-row")).toHaveLength(limit);
    expect(screen.getByRole("grid")).toHaveAttribute("aria-rowcount", "500");
  });

  it("shows the time in the chosen zone and the first line of the message", async () => {
    renderTable();
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await waitFor(() => expect(second).toHaveTextContent("core time 1"));
    expect(second).toHaveTextContent("ERROR first line");
    expect(second).not.toHaveTextContent("second line");
    expect(screen.getByTestId("query-result-table")).toHaveTextContent("@timestamp (UTC)");
  });

  it("drops an answer for another run (261020-carryover-fixes FR2.3.3)", async () => {
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId - 1, version),
    );
    renderTable();
    await waitFor(() => expect(api.getQueryRows).toHaveBeenCalled());
    await Promise.resolve();
    const first = screen.getAllByTestId("query-result-row")[0]!;
    expect(first).not.toHaveTextContent("core time 0");
  });

  it("opens a row with every field and value, the fixed ones first, and closes it again", async () => {
    // U2:BR7.3, R-09: a key holding a dot keeps its backquotes.
    const user = userEvent.setup();
    renderTable();
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await waitFor(() => expect(second).toHaveTextContent("ERROR first line"));
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    const details = screen.getByTestId("query-result-details");
    await waitFor(() =>
      expect(within(details).getByTestId("query-result-field-@message").textContent).toBe(
        "ERROR first line\n  at second line",
      ),
    );
    expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 1, 1);
    const names = within(details)
      .getAllByRole("rowheader")
      .map((cell) => cell.textContent);
    expect(names).toEqual([
      "@timestamp",
      "@log",
      "@logStream",
      "@message",
      "`user.id`",
      "http.status",
      "z",
    ]);
    expect(details).toHaveTextContent("2024/01/02/[$LATEST]abc");
    expect(within(details).getByTestId("query-result-field-z").textContent).toBe("");
    expect(second).toHaveAttribute("aria-expanded", "true");
    await user.click(within(second).getByRole("button", { name: "Hide details" }));
    expect(screen.queryByTestId("query-result-details")).not.toBeInTheDocument();
  });

  it("moves the selection with the arrow keys and opens it with Enter", async () => {
    renderTable();
    const grid = screen.getByRole("grid");
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row")[0]).toHaveTextContent("ERROR 0"),
    );
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    const rowsShown = screen.getAllByTestId("query-result-row");
    expect(rowsShown[1]).toHaveAttribute("aria-selected", "true");
    fireEvent.keyDown(grid, { key: "ArrowUp" });
    expect(screen.getAllByTestId("query-result-row")[0]).toHaveAttribute("aria-selected", "true");
    fireEvent.keyDown(grid, { key: "Enter" });
    expect(screen.getByTestId("query-result-details")).toBeInTheDocument();
  });

  it("is shown in Japanese for the ja locale and reports a failed read", async () => {
    vi.mocked(api.getQueryRows).mockRejectedValue(new Error("ipc"));
    const onError = renderTable(RESULT, "ja");
    expect(screen.getByTestId("query-result-table")).toHaveTextContent("@timestamp（UTC）");
    await waitFor(() => expect(onError).toHaveBeenCalled());
  });

  // ---- 261017-insights-query U2 ---------------------------------------

  it("draws one column per result column, in order, with an empty cell for no value", async () => {
    // U2:BR7.1.
    const columns = ["@timestamp", "http.status", "@message", "level"];
    const shaped = [
      queryRow(0, "line one\nline two", ["core time 0", "500", "line one\nline two", null]),
    ];
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(shaped, offset, limit, runId, version),
    );
    renderTable(resultSummary({ runId: 3, rowCount: 1, columns }));
    const header = screen.getByTestId("query-result-header");
    expect(Array.from(header.children).map((cell) => cell.textContent)).toEqual([
      "",
      "@timestamp (UTC)",
      "http.status",
      "@message",
      "level",
    ]);
    expect(screen.getByRole("grid")).toHaveAttribute("aria-colcount", "5");
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-cell").map((cell) => cell.textContent)).toEqual([
        "core time 0",
        "500",
        "line one",
        "",
      ]),
    );
  });

  it("says the fields are being read until they come and reads them once per row", async () => {
    // U2:BR7.3, BR7.4.
    let answer: (detail: QueryRowDetail) => void = () => undefined;
    vi.mocked(api.getQueryRowDetail).mockImplementation(
      () =>
        new Promise((resolve) => {
          answer = resolve;
        }),
    );
    const user = userEvent.setup();
    renderTable();
    const first = (await screen.findAllByTestId("query-result-row"))[0]!;
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    expect(screen.getByTestId("query-result-details-loading")).toHaveTextContent(
      "Loading the fields",
    );
    answer(detailOf(3, 0));
    await waitFor(() =>
      expect(screen.queryByTestId("query-result-details-loading")).not.toBeInTheDocument(),
    );
    await user.click(within(first).getByRole("button", { name: "Hide details" }));
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    expect(api.getQueryRowDetail).toHaveBeenCalledTimes(1);
  });

  it("drops the fields of another run and reports a failed read of them", async () => {
    vi.mocked(api.getQueryRowDetail).mockImplementationOnce(async (runId, version, index) =>
      detailOf(runId + 1, index, version),
    );
    const user = userEvent.setup();
    const onError = renderTable();
    const first = (await screen.findAllByTestId("query-result-row"))[0]!;
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    await Promise.resolve();
    expect(screen.getByTestId("query-result-details-loading")).toBeInTheDocument();
    vi.mocked(api.getQueryRowDetail).mockRejectedValueOnce(new Error("ipc"));
    const second = screen.getAllByTestId("query-result-row")[1]!;
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    await waitFor(() => expect(onError).toHaveBeenCalled());
  });

  it("says the rows are the previous run's while the next run goes, in both languages (261017-insights-query U3:BR3.4)", async () => {
    const { unmount } = render(
      <QueryResultTable result={RESULT} timeZone="Utc" t={t} onError={vi.fn()} previous />,
    );
    expect(screen.getByTestId("query-result-previous")).toHaveTextContent(
      "Results of the previous run",
    );
    expect((await screen.findAllByTestId("query-result-row")).length).toBeGreaterThan(0);
    unmount();
    render(
      <QueryResultTable
        result={RESULT}
        timeZone="Utc"
        t={createTranslator("ja")}
        onError={vi.fn()}
        previous
      />,
    );
    expect(screen.getByTestId("query-result-previous")).toHaveTextContent("前回の実行の結果");
    renderTable();
    expect(screen.getAllByTestId("query-result-previous")).toHaveLength(1);
  });
});

describe("QueryResultTable across versions of a result (261025-insights-stats U3:BR2.4, BR2.8)", () => {
  /** Version 2: three new rows first, then version 1's rows, each 3 places down. */
  const newer: QueryRow[] = [queryRow(900), queryRow(901), queryRow(902), ...rows];

  function serveVersions() {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(version === 1 ? rows : newer, offset, limit, runId, version),
    );
    vi.mocked(api.getQueryRowDetail).mockReset();
    vi.mocked(api.getQueryRowDetail).mockImplementation(async (runId, version, index) => ({
      ...detailOf(runId, version === 1 ? index : Math.max(index - 3, 0), version),
      index,
    }));
    vi.mocked(api.findQueryRow).mockReset();
    vi.mocked(api.findQueryRow).mockImplementation(async (_runId, version, rowKey) => {
      const index = (version === 1 ? rows : newer).findIndex((row) => row.rowKey === rowKey);
      return index < 0 ? null : index;
    });
  }

  function renderVersions(first: ResultSummary) {
    const onError = vi.fn();
    const view = render(<QueryResultTable result={first} timeZone="Utc" t={t} onError={onError} />);
    const show = (next: ResultSummary) =>
      view.rerender(<QueryResultTable result={next} timeZone="Utc" t={t} onError={onError} />);
    return { show, onError };
  }

  const partial = (version: number, rowCount: number) =>
    resultSummary({ runId: 3, version, rowCount, stage: "partial" });

  beforeEach(serveVersions);

  it("keeps an opened row open by its key at its new place and reads its fields again", async () => {
    const user = userEvent.setup();
    const { show } = renderVersions(partial(1, 500));
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await waitFor(() => expect(second).toHaveTextContent("ERROR first line"));
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    await waitFor(() => expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 1, 1));
    show(partial(2, 503));
    await waitFor(() => expect(api.findQueryRow).toHaveBeenCalledWith(3, 2, rows[1]!.rowKey));
    await waitFor(() => expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 2, 4));
    const moved = screen
      .getAllByTestId("query-result-row")
      .find((row) => row.getAttribute("aria-rowindex") === "5")!;
    expect(moved).toHaveAttribute("aria-expanded", "true");
    await waitFor(() => expect(moved).toHaveTextContent("ERROR first line"));
    expect(screen.getAllByTestId("query-result-details")).toHaveLength(1);
  });

  it("closes an opened row the newer version does not have", async () => {
    vi.mocked(api.findQueryRow).mockResolvedValue(null);
    const user = userEvent.setup();
    const { show } = renderVersions(partial(1, 500));
    const first = (await screen.findAllByTestId("query-result-row"))[0]!;
    await waitFor(() => expect(first).toHaveTextContent("ERROR 0"));
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    expect(screen.getByTestId("query-result-details")).toBeInTheDocument();
    show(partial(2, 503));
    await waitFor(() =>
      expect(screen.queryByTestId("query-result-details")).not.toBeInTheDocument(),
    );
  });

  it("scrolls so that the row shown first is first again", async () => {
    const { show } = renderVersions(partial(1, 500));
    const list = screen.getByTestId("query-result-list");
    list.scrollTop = 40 * 22;
    fireEvent.scroll(list);
    await waitFor(() => {
      const top = screen.getAllByTestId("query-result-row")[0]!;
      expect(top).toHaveAttribute("aria-rowindex", "41");
      expect(top).toHaveTextContent("ERROR 40");
    });
    show(partial(2, 503));
    await waitFor(() => expect(api.findQueryRow).toHaveBeenCalledWith(3, 2, rows[40]!.rowKey));
    await waitFor(() => expect(list.scrollTop).toBe(43 * 22));
    await waitFor(() =>
      expect(
        screen
          .getAllByTestId("query-result-row")
          .find((row) => row.getAttribute("aria-rowindex") === "44"),
      ).toHaveTextContent("ERROR 40"),
    );
  });

  // 261025-insights-stats U3 code review R-02 (fixed in U4): an opened row's old place
  // names another row in the newer version, so its fields wait for find_query_row.
  it("does not read an opened row's fields at its old place before the newer version places it", async () => {
    // Every place asked for (the opened row and the row shown first) waits until
    // `place` answers them as version 2 would.
    const waiting: Array<() => void> = [];
    vi.mocked(api.findQueryRow).mockImplementation(
      (_runId, _version, rowKey) =>
        new Promise<number | null>((resolve) => {
          waiting.push(() => resolve(newer.findIndex((row) => row.rowKey === rowKey)));
        }),
    );
    const place = () => waiting.splice(0).forEach((answer) => answer());
    const user = userEvent.setup();
    const { show } = renderVersions(partial(1, 500));
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await waitFor(() => expect(second).toHaveTextContent("ERROR first line"));
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    await waitFor(() => expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 1, 1));
    show(partial(2, 503));
    await waitFor(() => expect(api.findQueryRow).toHaveBeenCalledWith(3, 2, rows[1]!.rowKey));
    // Waiting for the place: nothing is read for version 2, and the row says so.
    await Promise.resolve();
    expect(api.getQueryRowDetail).not.toHaveBeenCalledWith(3, 2, expect.any(Number));
    expect(screen.getByTestId("query-result-details-loading")).toBeInTheDocument();
    place();
    await waitFor(() => expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 2, 4));
    expect(api.getQueryRowDetail).not.toHaveBeenCalledWith(3, 2, 1);
    await waitFor(() =>
      expect(screen.getByTestId("query-result-details")).toHaveTextContent("ERROR first line"),
    );
  });

  it("reads no field of version 2 for an opened row the newer version does not have", async () => {
    vi.mocked(api.findQueryRow).mockResolvedValue(null);
    const user = userEvent.setup();
    const { show } = renderVersions(partial(1, 500));
    const first = (await screen.findAllByTestId("query-result-row"))[0]!;
    await waitFor(() => expect(first).toHaveTextContent("ERROR 0"));
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    await waitFor(() => expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 1, 0));
    show(partial(2, 503));
    await waitFor(() =>
      expect(screen.queryByTestId("query-result-details")).not.toBeInTheDocument(),
    );
    expect(api.getQueryRowDetail).not.toHaveBeenCalledWith(3, 2, expect.any(Number));
  });

  it("drops rows read for an older version and keeps nothing of another run", async () => {
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId, version - 1),
    );
    const { show } = renderVersions(partial(2, 500));
    await waitFor(() => expect(api.getQueryRows).toHaveBeenCalledWith(3, 2, 0, expect.any(Number)));
    await Promise.resolve();
    expect(screen.getAllByTestId("query-result-row")[0]).not.toHaveTextContent("ERROR 0");
    show(resultSummary({ runId: 4, version: 1, rowCount: 500 }));
    await waitFor(() => expect(api.getQueryRows).toHaveBeenCalledWith(4, 1, 0, expect.any(Number)));
    expect(api.findQueryRow).not.toHaveBeenCalled();
  });
});

describe("QueryResultTable with an aggregates result (261025-insights-stats BR4.5)", () => {
  const binned = [
    aggregateRow(["2024/01/02 03:00:00.000", "12", "200.3333"]),
    aggregateRow(["2024/01/02 03:05:00.000", "3", null]),
  ];

  function serve(answer: QueryRow[]) {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(answer, offset, limit, runId, version),
    );
  }

  const cellTexts = () =>
    screen.getAllByTestId("query-result-cell").map((cell) => cell.textContent);

  it("draws the key and aggregate columns with the cells the core wrote", async () => {
    serve(binned);
    renderTable(
      aggregatesSummary(["bin(5m)", "count(*)", "avg(latency)"], { runId: 3, rowCount: 2 }),
    );
    expect(screen.getByTestId("query-result-table")).toHaveAttribute("data-shape", "aggregates");
    expect(screen.getAllByTestId("query-result-heading").map((cell) => cell.textContent)).toEqual([
      "bin(5m)",
      "count(*)",
      "avg(latency)",
    ]);
    await waitFor(() =>
      expect(cellTexts()).toEqual([
        "2024/01/02 03:00:00.000",
        "12",
        "200.3333",
        "2024/01/02 03:05:00.000",
        "3",
        "",
      ]),
    );
    expect(screen.getByRole("grid")).toHaveAttribute("aria-rowcount", "2");
  });

  it("shows a by @message key in full, not by its first line", async () => {
    serve([aggregateRow(["ERROR first line\n  at second line", "4"])]);
    renderTable(aggregatesSummary(["@message", "count(*)"], { runId: 3, rowCount: 1 }));
    await waitFor(() => expect(cellTexts()).toEqual(["ERROR first line\n  at second line", "4"]));
    const cells = screen.getAllByTestId("query-result-cell");
    expect(cells[0]).toHaveClass("query-result-cell");
  });

  it("adds no zone to a by @timestamp heading", () => {
    serve([]);
    renderTable(aggregatesSummary(["@timestamp", "count(*)"], { runId: 3, rowCount: 0 }));
    expect(screen.getAllByTestId("query-result-heading").map((cell) => cell.textContent)).toEqual([
      "@timestamp",
      "count(*)",
    ]);
    expect(screen.getByTestId("query-result-table")).not.toHaveTextContent("(UTC)");
  });

  it("opens an aggregate row with its columns and cells", async () => {
    serve(binned);
    vi.mocked(api.getQueryRowDetail).mockReset();
    vi.mocked(api.getQueryRowDetail).mockImplementation(async (runId, version, index) => ({
      runId,
      version,
      index,
      fields: [
        { name: "bin(5m)", value: "2024/01/02 03:05:00.000" },
        { name: "count(*)", value: "3" },
        { name: "avg(latency)", value: null },
      ],
    }));
    const user = userEvent.setup();
    renderTable(
      aggregatesSummary(["bin(5m)", "count(*)", "avg(latency)"], { runId: 3, rowCount: 2 }),
    );
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    const details = screen.getByTestId("query-result-details");
    await waitFor(() =>
      expect(within(details).getByTestId("query-result-field-count(*)").textContent).toBe("3"),
    );
    expect(
      within(details)
        .getAllByRole("rowheader")
        .map((cell) => cell.textContent),
    ).toEqual(["bin(5m)", "count(*)", "avg(latency)"]);
    expect(within(details).getByTestId("query-result-field-avg(latency)").textContent).toBe("");
    expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 1, 1);
  });

  it("reads no row for a result of 0 rows", () => {
    serve([]);
    renderTable(aggregatesSummary(["level", "count(*)"], { runId: 3, rowCount: 0 }));
    expect(screen.queryAllByTestId("query-result-row")).toHaveLength(0);
    expect(api.getQueryRows).not.toHaveBeenCalled();
    expect(screen.getByRole("grid")).toHaveAttribute("aria-rowcount", "0");
  });

  it("lets many columns scroll sideways and leaves an events result as it was", () => {
    serve([]);
    const columns = Array.from({ length: 12 }, (_, index) => `c${index}`);
    const { unmount } = render(
      <QueryResultTable
        result={aggregatesSummary(columns, { runId: 3, rowCount: 0 })}
        timeZone="Utc"
        t={t}
        onError={vi.fn()}
      />,
    );
    expect(screen.getByTestId("query-result-table")).toHaveClass("is-aggregates");
    expect(screen.getByTestId("query-result-header").style.minWidth).toBe("calc(28px + 96rem)");
    expect(screen.getByRole("grid").style.minWidth).toBe("calc(28px + 96rem)");
    unmount();
    renderTable(resultSummary({ runId: 3, rowCount: 0 }));
    expect(screen.getByTestId("query-result-table")).toHaveAttribute("data-shape", "events");
    expect(screen.getByTestId("query-result-table")).not.toHaveClass("is-aggregates");
    expect(screen.getByTestId("query-result-header").style.minWidth).toBe("");
    expect(screen.getByTestId("query-result-table")).toHaveTextContent("@timestamp (UTC)");
  });
});
