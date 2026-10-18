import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { QueryRow, ResultSummary } from "../api";
import { createTranslator } from "../i18n/messages";
import { queryRow, queryRowWindow, t } from "../test/fixtures";
import { QueryResultTable } from "./QueryResultTable";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getQueryRows: vi.fn() };
});

const api = await import("../api");

const RESULT: ResultSummary = { runId: 3, holdingsEpoch: 7, rowCount: 500, evaluatedCount: 9_000 };

const rows: QueryRow[] = Array.from({ length: 500 }, (_, index) =>
  index === 1
    ? {
        ...queryRow(1, "ERROR first line\n  at second line"),
        logStreamName: "2024/01/02/[$LATEST]abc",
      }
    : queryRow(index),
);

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

describe("QueryResultTable (261017-insights-query U1:BR6.3, BR6.4)", () => {
  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, epoch, offset, limit) =>
      queryRowWindow(rows, offset, limit, runId, epoch),
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

  it("opens a row with the whole message, @logStream and @log, and closes it again", async () => {
    const user = userEvent.setup();
    renderTable();
    const second = (await screen.findAllByTestId("query-result-row"))[1]!;
    await waitFor(() => expect(second).toHaveTextContent("ERROR first line"));
    await user.click(within(second).getByRole("button", { name: "Show details" }));
    const details = screen.getByTestId("query-result-details");
    expect(within(details).getByTestId("query-result-field-@message").textContent).toBe(
      "ERROR first line\n  at second line",
    );
    expect(details).toHaveTextContent("2024/01/02/[$LATEST]abc");
    expect(details).toHaveTextContent("/aws/lambda/MyFunction");
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
});
