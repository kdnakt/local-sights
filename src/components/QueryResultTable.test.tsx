import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueryRow, QueryRowDetail, ResultSummary } from "../api";
import { createTranslator } from "../i18n/messages";
import { queryRow, queryRowWindow, resultSummary, t } from "../test/fixtures";
import { QueryResultTable } from "./QueryResultTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getQueryRows: vi.fn(), getQueryRowDetail: vi.fn() };
});

const api = await import("../api");

const RESULT: ResultSummary = resultSummary({
  runId: 3,
  holdingsEpoch: 7,
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
function detailOf(runId: number, holdingsEpoch: number, index: number): QueryRowDetail {
  const row = rows[index]!;
  return {
    runId,
    holdingsEpoch,
    index,
    fields: [
      { name: "@timestamp", value: row.displayTime },
      { name: "@log", value: row.logGroupName },
      { name: "@logStream", value: row.logStreamName },
      { name: "@message", value: row.message },
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
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, epoch, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId, epoch),
    );
    vi.mocked(api.getQueryRowDetail).mockReset();
    vi.mocked(api.getQueryRowDetail).mockImplementation(async (runId, epoch, index) =>
      detailOf(runId, epoch, index),
    );
  });

  it("reads only the rows in view, for the shown run and holdings epoch", async () => {
    renderTable();
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row").length).toBeGreaterThan(0),
    );
    const [runId, epoch, offset, limit] = vi.mocked(api.getQueryRows).mock.calls.at(-1)!;
    expect([runId, epoch, offset]).toEqual([3, 7, 0]);
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

  it("drops an answer for another run or holdings epoch", async () => {
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, epoch, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId, epoch + 1),
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
    expect(api.getQueryRowDetail).toHaveBeenCalledWith(3, 7, 1);
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
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, epoch, offset, limit) =>
      queryRowWindow(shaped, offset, limit, runId, epoch),
    );
    renderTable(resultSummary({ runId: 3, holdingsEpoch: 7, rowCount: 1, columns }));
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
    answer(detailOf(3, 7, 0));
    await waitFor(() =>
      expect(screen.queryByTestId("query-result-details-loading")).not.toBeInTheDocument(),
    );
    await user.click(within(first).getByRole("button", { name: "Hide details" }));
    await user.click(within(first).getByRole("button", { name: "Show details" }));
    expect(api.getQueryRowDetail).toHaveBeenCalledTimes(1);
  });

  it("drops the fields of another run and reports a failed read of them", async () => {
    vi.mocked(api.getQueryRowDetail).mockImplementationOnce(async (runId, epoch, index) =>
      detailOf(runId + 1, epoch, index),
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
