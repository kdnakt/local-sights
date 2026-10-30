import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InsightsProgress, InsightsView, SessionView } from "../api";
import { createTranslator } from "../i18n/messages";
import {
  QUERY_CATALOG,
  aggregatesSummary,
  cancelledSummary,
  connectedView,
  insightsView,
  partialSummary,
  queryRow,
  queryRowWindow,
  resultSummary,
  t,
} from "../test/fixtures";
import { LogAnalyticsPage } from "./LogAnalyticsPage";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return {
    ...original,
    getQueryRows: vi.fn(),
    getQueryRowDetail: vi.fn(),
    findQueryRow: vi.fn(async () => null),
    getCompletionCandidates: vi.fn(),
    getQueryCatalog: vi.fn(),
  };
});
vi.mock("../aceLoader", () => ({ loadAce: () => Promise.resolve({}) }));
vi.mock("@cloudscape-design/components/code-editor", () => import("../test/codeEditorMock"));

const api = await import("../api");

function view(insights: Partial<InsightsView> = {}, session: Partial<SessionView> = {}) {
  return connectedView({ insights: insightsView(insights), ...session });
}

function renderPage(session: SessionView = view(), locale: "en" | "ja" = "en") {
  const handlers = {
    onChange: vi.fn(),
    onRun: vi.fn(),
    onCancel: vi.fn(),
    onError: vi.fn(),
    onExport: vi.fn(),
  };
  const translate = locale === "en" ? t : createTranslator("ja");
  const rendered = render(
    <LogAnalyticsPage session={session} t={translate} locale={locale} mac={false} {...handlers} />,
  );
  const rerender = (next: SessionView) =>
    rendered.rerender(
      <LogAnalyticsPage session={next} t={translate} locale={locale} mac={false} {...handlers} />,
    );
  return { ...handlers, rerender };
}

/** A run's progress: nothing fetched yet, log group 1 of 1 (U3:BR2.7). */
function progress(overrides: Partial<InsightsProgress> = {}): InsightsProgress {
  return {
    readingCache: false,
    groupNumber: 1,
    groupCount: 1,
    failedGroupCount: 0,
    selectedStreamCount: 0,
    plannedStreamCount: null,
    finishedStreamCount: 0,
    eventCount: 0,
    evaluatedCount: 0,
    totalCount: 0,
    ...overrides,
  };
}

const runButton = () => screen.getByTestId("log-analytics-run-button");
const editor = () => screen.getByRole("textbox", { name: "Query" });
const results = () => screen.getByTestId("log-analytics-results");

describe("LogAnalyticsPage (261017-insights-query U1, refined-mockups screens 2-7)", () => {
  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow([queryRow(0), queryRow(1)], offset, limit, runId, version),
    );
    vi.mocked(api.getCompletionCandidates).mockReset();
    vi.mocked(api.getCompletionCandidates).mockResolvedValue([]);
    vi.mocked(api.getQueryCatalog).mockReset();
    vi.mocked(api.getQueryCatalog).mockResolvedValue([...QUERY_CATALOG]);
  });

  it("lays out the log group, the range, the query and the run row, with no results yet", () => {
    renderPage();
    const form = screen.getByTestId("log-analytics-form");
    const order = [
      "log-analytics-log-group",
      "log-analytics-start",
      "query-editor",
      "log-analytics-run",
    ];
    const elements = order.map((testId) => within(form).getByTestId(testId));
    for (let index = 1; index < elements.length; index += 1) {
      expect(
        elements[index - 1]!.compareDocumentPosition(elements[index]!) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
    expect(editor()).toHaveValue("filter @message like /ERROR/");
    expect(screen.getByTestId("log-analytics-run")).toHaveTextContent("Ctrl+Enter");
    expect(results()).toHaveTextContent("No results yet.");
  });

  it("cannot run while a reason holds and says the first one (U1:BR1.5)", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPage(
      view({ canRun: false, runBlocker: "insights.blocked.logManagementFetching" }),
    );
    expect(runButton()).toBeDisabled();
    expect(screen.getByTestId("log-analytics-run-reason")).toHaveTextContent(
      "Log Management is fetching logs.",
    );
    fireEvent.keyDown(editor(), { key: "Enter", ctrlKey: true });
    await user.click(runButton());
    expect(onRun).not.toHaveBeenCalled();
  });

  it("runs with the button and with Ctrl+Enter when it can (U1:BR1.6)", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPage();
    await user.click(runButton());
    fireEvent.keyDown(editor(), { key: "Enter", ctrlKey: true });
    expect(onRun).toHaveBeenCalledTimes(2);
  });

  it("hands every change to the core and never runs on a change", () => {
    const { onChange, onRun } = renderPage();
    fireEvent.change(editor(), { target: { value: "filter @log = 'g'" } });
    expect(onChange).toHaveBeenCalledWith("queryText", "filter @log = 'g'");
    expect(onRun).not.toHaveBeenCalled();
  });

  it("locks the inputs while running and shows [Cancel] and the progress (U1:BR2.4)", async () => {
    const user = userEvent.setup();
    const { onCancel, onChange } = renderPage(
      view({
        phase: "Fetching",
        canRun: false,
        runBlocker: "insights.blocked.queryRunning",
        cancelAvailable: true,
        progress: progress({
          groupNumber: 1,
          groupCount: 2,
          selectedStreamCount: 37,
          plannedStreamCount: 37,
          finishedStreamCount: 14,
          eventCount: 48_210,
        }),
      }),
    );
    expect(runButton()).toBeDisabled();
    expect(screen.queryByTestId("log-analytics-run-reason")).not.toBeInTheDocument();
    expect(
      within(screen.getByTestId("log-group-multiselect")).getAllByRole("button")[0],
    ).toBeDisabled();
    fireEvent.change(editor(), { target: { value: "changed" } });
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Fetching logs (log group 1 / 2): 14 / 37 streams, 48210 events",
    );
    await user.click(screen.getByTestId("log-analytics-cancel-button"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("shows the evaluation's progress and the cancelling state", () => {
    renderPage(
      view({
        phase: "Evaluating",
        canRun: false,
        cancelling: true,
        progress: progress({
          selectedStreamCount: 2,
          plannedStreamCount: 2,
          finishedStreamCount: 2,
          eventCount: 25_000,
          evaluatedCount: 10_000,
          totalCount: 25_000,
        }),
      }),
    );
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Running query: 10000 / 25000 events",
    );
    expect(screen.getByTestId("log-analytics-cancel-button")).toHaveTextContent("Cancelling…");
  });

  it("tells no matching events apart from a failed fetch (U1:BR6.1, BR2.6)", () => {
    const { rerender } = renderPage(
      view({
        phase: "Done",
        result: resultSummary({ rowCount: 0 }),
      }),
    );
    expect(screen.getByTestId("log-analytics-no-match")).toHaveTextContent("No matching events.");
    expect(results()).toHaveTextContent("(0 matched)");
    expect(screen.queryByTestId("log-analytics-fetch-error")).not.toBeInTheDocument();

    rerender(
      view({
        phase: "FetchFailed",
        failure: { kind: "AccessDenied", safeDetail: "kind=AccessDenied", retryable: false },
      }),
    );
    const error = screen.getByTestId("log-analytics-fetch-error");
    expect(error).toHaveTextContent("Access was denied.");
    expect(error).toHaveTextContent("The query was not run.");
    expect(screen.queryByTestId("log-analytics-no-match")).not.toBeInTheDocument();
    expect(screen.queryByTestId("log-analytics-initial")).not.toBeInTheDocument();
  });

  it("counts an aggregates result in rows and says when it has none (261025-insights-stats BR3.4, BR4.5)", () => {
    const { rerender } = renderPage(
      view({
        phase: "Done",
        result: aggregatesSummary(["level", "count(*)"], { rowCount: 0 }),
      }),
    );
    expect(screen.getByTestId("log-analytics-no-rows")).toHaveTextContent(
      "The query returned 0 rows.",
    );
    expect(screen.queryByTestId("log-analytics-no-match")).not.toBeInTheDocument();
    expect(results()).toHaveTextContent("(0 rows)");
    rerender(
      view({
        phase: "Done",
        result: aggregatesSummary(["bin(5m)", "count(*)"], { rowCount: 3 }),
      }),
    );
    expect(results()).toHaveTextContent("(3 rows)");
    expect(screen.getByTestId("query-result-table")).toHaveAttribute("data-shape", "aggregates");
  });

  it("shows the rows of a result with their count", async () => {
    renderPage(
      view({
        phase: "Done",
        result: resultSummary(),
      }),
    );
    expect(results()).toHaveTextContent("(2 matched)");
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row")[1]).toHaveTextContent("ERROR 1"),
    );
    expect(api.getQueryRows).toHaveBeenCalledWith(1, 1, 0, 2);
  });

  it("warns that a partial result is based on part of the logs and lists the failures (U1:BR2.5)", async () => {
    const user = userEvent.setup();
    renderPage(
      view({
        phase: "Done",
        partial: true,
        failedStreams: [
          {
            logGroupName: "/aws/lambda/orders",
            logStreamName: "stream-x",
            failure: { kind: "AccessDenied", safeDetail: "", retryable: false },
          },
        ],
        result: resultSummary(),
      }),
    );
    expect(screen.getByTestId("log-analytics-partial")).toHaveTextContent(
      "Results are based on part of the logs. 1 stream could not be fetched.",
    );
    await user.click(screen.getByTestId("log-analytics-show-failures-button"));
    // U3:BR2.6: "log group / stream".
    const list = screen.getByTestId("failure-list");
    expect(within(list).getByTestId("failure-list-item")).toHaveTextContent(
      "/aws/lambda/orders / stream-x",
    );
    expect(list).toHaveTextContent("Log group / stream");
  });

  it("says a cancelled query was cancelled (U1:BR2.7)", () => {
    renderPage(view({ phase: "Cancelled" }));
    expect(screen.getByTestId("log-analytics-cancelled")).toHaveTextContent("Query cancelled.");
    expect(screen.queryByTestId("log-analytics-initial")).not.toBeInTheDocument();
  });

  it("shows a reading problem with its place, keeps the text and the previous result, and focuses the field (U1:BR3.9)", async () => {
    const { rerender } = renderPage(
      view({
        phase: "Done",
        runId: 1,
        result: resultSummary(),
      }),
    );
    rerender(
      view({
        phase: "QueryError",
        runId: 2,
        result: resultSummary(),
        problem: {
          kind: "Unsupported",
          messageKey: "query.unsupported.command",
          word: "stats",
          span: { line: 1, column: 30, length: 5 },
        },
      }),
    );
    // 261025-insights-stats U2:BR4.2: the list is the catalog's, stats included.
    await waitFor(() =>
      expect(screen.getByTestId("query-problem")).toHaveTextContent(
        'Line 1, column 30: "stats" is not supported in this version. Supported commands: fields, filter, sort, limit, stats.',
      ),
    );
    expect(editor()).toHaveValue("filter @message like /ERROR/");
    await act(async () => undefined);
    expect(editor()).toHaveFocus();
    expect(results()).toHaveTextContent("(2 matched)");
  });

  it("is shown in Japanese for the ja locale", () => {
    renderPage(
      view({
        phase: "QueryError",
        problem: {
          kind: "Syntax",
          messageKey: "query.syntax.unclosedParen",
          word: null,
          span: { line: 2, column: 8, length: 1 },
        },
      }),
      "ja",
    );
    expect(screen.getByTestId("log-analytics-run-button")).toHaveTextContent("クエリを実行");
    expect(screen.getByTestId("query-problem")).toHaveTextContent(
      "2 行 8 列：括弧が閉じていません。",
    );
    expect(results()).toHaveTextContent("まだ結果はありません。");
  });

  // ---- 261017-insights-query U2 ---------------------------------------

  it("says how many matched and Showing N when the result was cut (U2:BR5.3, BR7.2)", () => {
    const { rerender } = renderPage(
      view({
        phase: "Done",
        result: resultSummary({ rowCount: 20, matchedCount: 1284, truncatedAt: 20 }),
      }),
    );
    expect(results()).toHaveTextContent("(20 of 1284 matched)");
    expect(screen.getByTestId("log-analytics-truncated")).toHaveTextContent("Showing 20");
    rerender(view({ phase: "Done", result: resultSummary({ rowCount: 7, matchedCount: 7 }) }));
    expect(results()).toHaveTextContent("(7 matched)");
    expect(screen.queryByTestId("log-analytics-truncated")).not.toBeInTheDocument();
  });

  it("takes a reading problem's words away once the text is changed (U2:BR6.1)", () => {
    const { onChange } = renderPage(
      view({
        phase: "QueryError",
        runId: 3,
        problem: {
          kind: "Syntax",
          messageKey: "query.syntax.notNeedsParen",
          word: "not",
          span: { line: 1, column: 8, length: 3 },
        },
      }),
    );
    expect(screen.getByTestId("query-problem")).toHaveTextContent(
      "Line 1, column 8: Put the condition after not in parentheses",
    );
    fireEvent.change(editor(), { target: { value: "filter not (@message like /x/)" } });
    expect(onChange).toHaveBeenCalledWith("queryText", "filter not (@message like /x/)");
    expect(screen.queryByTestId("query-problem")).not.toBeInTheDocument();
  });

  it("says an evaluation failure in its own words, without a result or a fetch error (U2:BR10.2)", () => {
    renderPage(view({ phase: "EvaluationFailed", runId: 4, result: null, canRun: true }));
    const alert = screen.getByTestId("log-analytics-evaluation-failed");
    expect(alert).toHaveTextContent("The query could not be evaluated.");
    expect(alert).toHaveTextContent("Run the query again.");
    expect(screen.queryByTestId("log-analytics-fetch-error")).not.toBeInTheDocument();
    expect(screen.queryByTestId("query-result-table")).not.toBeInTheDocument();
    expect(screen.queryByTestId("log-analytics-initial")).not.toBeInTheDocument();
    expect(runButton()).toBeEnabled();
  });

  /** Chooses the example named `name` from [Examples]. */
  async function chooseExample(user: ReturnType<typeof userEvent.setup>, name: string) {
    await user.click(screen.getByRole("button", { name: "Examples" }));
    await user.click(await screen.findByRole("menuitem", { name }));
  }

  it("puts an example in an empty field at once (U2:BR9.3)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderPage(view({ queryText: "" }));
    await chooseExample(user, "HTTP status 500 and above, largest first");
    expect(editor()).toHaveValue(
      "fields @timestamp, http.status, http.path\n| filter http.status >= 500\n| sort http.status desc, @timestamp desc\n| limit 50",
    );
    expect(onChange).toHaveBeenLastCalledWith("queryText", expect.stringContaining("http.status"));
    expect(screen.queryByTestId("replace-query-dialog")).not.toBeInTheDocument();
  });

  it("asks before an example replaces a written query, and can keep it (U2:BR9.3)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderPage();
    await chooseExample(user, "Errors that are not timeouts");
    expect(screen.getByTestId("replace-query-dialog")).toHaveTextContent(
      "The query you wrote is replaced by the example.",
    );
    await user.click(screen.getByTestId("replace-query-cancel-button"));
    expect(editor()).toHaveValue("filter @message like /ERROR/");
    expect(onChange).not.toHaveBeenCalled();
    await chooseExample(user, "Errors that are not timeouts");
    await user.click(screen.getByTestId("replace-query-button"));
    expect((editor() as HTMLTextAreaElement).value).toContain("not like /timeout/");
    expect(screen.queryByTestId("replace-query-dialog")).not.toBeInTheDocument();
  });

  it("offers no example while the query runs (U2:BR9.3, U1:BR2.4)", () => {
    renderPage(view({ phase: "Fetching", canRun: false, cancelAvailable: true }));
    expect(screen.getByRole("button", { name: "Examples" })).toBeDisabled();
  });

  it("asks the core for the completion candidates whenever the run changes (U2:BR9.1)", async () => {
    vi.mocked(api.getCompletionCandidates).mockResolvedValue([
      { text: "filter", kind: "Command" },
      { text: "http.status", kind: "JsonField" },
    ]);
    const { rerender } = renderPage();
    await waitFor(() => expect(api.getCompletionCandidates).toHaveBeenCalledTimes(1));
    rerender(view({ phase: "Fetching", runId: 1, canRun: false }));
    rerender(view({ phase: "Done", runId: 1, result: resultSummary() }));
    await waitFor(() => expect(api.getCompletionCandidates).toHaveBeenCalledTimes(3));
  });

  // ---- 261025-insights-stats U2:BR3.2, BR4.2 ----------------------------

  it("asks the core for the catalog once, however the run changes (BR4.2)", async () => {
    const { rerender } = renderPage();
    await waitFor(() => expect(api.getQueryCatalog).toHaveBeenCalledTimes(1));
    rerender(view({ phase: "Fetching", runId: 1, canRun: false }));
    rerender(view({ phase: "Done", runId: 1, result: resultSummary() }));
    await act(async () => undefined);
    expect(api.getQueryCatalog).toHaveBeenCalledTimes(1);
  });

  it("names no command list and shows no error when the catalog cannot be had (BR4.2)", async () => {
    vi.mocked(api.getQueryCatalog).mockRejectedValue(new Error("denied"));
    const { onError } = renderPage(
      view({
        phase: "QueryError",
        runId: 2,
        problem: {
          kind: "Unsupported",
          messageKey: "query.unsupported.command",
          word: "parse",
          span: { line: 1, column: 1, length: 5 },
        },
      }),
    );
    await act(async () => undefined);
    expect(screen.getByTestId("query-problem")).toHaveTextContent(
      'Line 1, column 1: "parse" is not supported in this version.',
    );
    expect(screen.getByTestId("query-problem")).not.toHaveTextContent("Supported commands");
    expect(onError).not.toHaveBeenCalled();
    expect(editor()).toHaveAttribute("data-language", "text");
  });

  it("says in rows how an aggregates result was cut (BR3.2)", () => {
    const { rerender } = renderPage(
      view({
        phase: "Done",
        result: aggregatesSummary(["level", "count(*)"], {
          rowCount: 10_000,
          matchedCount: 10_003,
          truncatedAt: 10_000,
        }),
      }),
    );
    expect(results()).toHaveTextContent("(10000 of 10003 rows)");
    expect(screen.getByTestId("log-analytics-truncated")).toHaveTextContent("Showing 10000 rows");
    rerender(
      view({
        phase: "Done",
        result: aggregatesSummary(["level", "count(*)"], { rowCount: 2, matchedCount: 2 }),
      }),
    );
    expect(results()).toHaveTextContent("(2 rows)");
    expect(screen.queryByTestId("log-analytics-truncated")).not.toBeInTheDocument();
  });

  it("says the aggregates cut in Japanese too (BR3.2, NFR9)", () => {
    renderPage(
      view({
        phase: "Done",
        result: aggregatesSummary(["level"], { rowCount: 5, matchedCount: 9, truncatedAt: 5 }),
      }),
      "ja",
    );
    expect(results()).toHaveTextContent("(9 行中 5 行)");
    expect(screen.getByTestId("log-analytics-truncated")).toHaveTextContent("5 行まで表示");
  });

  it("shows the columns of fields with their cells (U2:BR7.1)", async () => {
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow(
        [queryRow(0, "ERROR 0", ["core time 0", "500", null])],
        offset,
        limit,
        runId,
        version,
      ),
    );
    renderPage(
      view({
        phase: "Done",
        result: resultSummary({ rowCount: 1, columns: ["@timestamp", "http.status", "level"] }),
      }),
    );
    const header = screen.getByTestId("query-result-header");
    expect(header).toHaveTextContent("@timestamp (Local)");
    expect(header).toHaveTextContent("http.status");
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-cell").map((cell) => cell.textContent)).toEqual([
        "core time 0",
        "500",
        "",
      ]),
    );
  });

  // ---- 261017-insights-query U3 ---------------------------------------

  it("hands the core every chosen log group as a list (U3:BR4.1)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderPage(view({ selectedLogGroupNames: ["/aws/lambda/MyFunction"] }));
    expect(screen.getByTestId("log-analytics-log-group")).toHaveTextContent("Log groups");
    await user.click(
      within(screen.getByTestId("log-group-multiselect")).getAllByRole("button")[0]!,
    );
    await user.click(screen.getByRole("option", { name: "/aws/ecs/web" }));
    expect(onChange).toHaveBeenCalledWith("logGroup", ["/aws/lambda/MyFunction", "/aws/ecs/web"]);
  });

  it("says no log group is chosen as at least one (U3:BR4.2)", () => {
    renderPage(
      view({
        selectedLogGroupNames: [],
        canRun: false,
        runBlocker: "insights.blocked.logGroupRequired",
      }),
    );
    expect(runButton()).toBeDisabled();
    expect(screen.getByTestId("log-analytics-run-reason")).toHaveTextContent(
      "Choose at least one log group.",
    );
  });

  it("says when the cache is read and before a log group is fetched (U3:BR2.7)", () => {
    const running = { phase: "Fetching" as const, canRun: false, cancelAvailable: true };
    const { rerender } = renderPage(
      view({ ...running, progress: progress({ groupNumber: null }) }),
    );
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Fetching logs: finding what is missing",
    );
    rerender(view({ ...running, progress: progress({ groupNumber: null, readingCache: true }) }));
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Fetching logs: reading the cache",
    );
    rerender(
      view({
        ...running,
        progress: progress({ groupNumber: 3, groupCount: 5, selectedStreamCount: 4 }),
      }),
    );
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Fetching logs (log group 3 / 5): listing streams, 4 selected so far",
    );
  });

  it("counts the log groups that failed while the fetch goes on (U3:R-04)", () => {
    renderPage(
      view({
        phase: "Fetching",
        canRun: false,
        progress: progress({ groupNumber: 3, groupCount: 3, failedGroupCount: 2 }),
      }),
    );
    expect(screen.getByTestId("log-analytics-failed-groups")).toHaveTextContent(
      "2 log groups could not be fetched.",
    );
  });

  it("keeps the previous result shown while the next run goes (U3:BR3.4)", async () => {
    const { rerender } = renderPage(
      view({
        phase: "Fetching",
        runId: 2,
        canRun: false,
        progress: progress(),
        result: resultSummary({ runId: 1 }),
      }),
    );
    expect(screen.getByTestId("log-analytics-progress")).toBeInTheDocument();
    expect(screen.getByTestId("query-result-previous")).toHaveTextContent(
      "Results of the previous run",
    );
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row")[1]).toHaveTextContent("ERROR 1"),
    );
    expect(api.getQueryRows).toHaveBeenCalledWith(1, 1, 0, 2);
    rerender(
      view({
        phase: "Evaluating",
        runId: 2,
        canRun: false,
        progress: progress({ evaluatedCount: 1, totalCount: 9 }),
        result: resultSummary({ runId: 1 }),
      }),
    );
    expect(screen.getByTestId("query-result-previous")).toBeInTheDocument();
    rerender(view({ phase: "Done", runId: 2, result: resultSummary({ runId: 2 }) }));
    expect(screen.queryByTestId("query-result-previous")).not.toBeInTheDocument();
    expect(screen.queryByTestId("log-analytics-progress")).not.toBeInTheDocument();
  });

  it("names every log group a failed fetch could not fetch, with no result (U3:BR2.6, BR3.4)", () => {
    const denied = { kind: "AccessDenied" as const, safeDetail: "", retryable: false };
    const expired = { kind: "AuthRequired" as const, safeDetail: "", retryable: false };
    renderPage(
      view({
        phase: "FetchFailed",
        failure: denied,
        failedGroups: [
          { logGroupName: "/aws/lambda/orders", failure: denied },
          { logGroupName: "/aws/lambda/payments", failure: expired },
        ],
        result: null,
      }),
    );
    const error = screen.getByTestId("log-analytics-fetch-error");
    expect(error).toHaveTextContent("Log groups that could not be fetched:");
    const groups = within(error).getAllByTestId("log-analytics-failed-group");
    expect(groups.map((group) => group.textContent)).toEqual([
      expect.stringContaining("/aws/lambda/orders: Access was denied."),
      expect.stringContaining("/aws/lambda/payments:"),
    ]);
    expect(error).toHaveTextContent("The query was not run.");
    expect(screen.queryByTestId("query-result-table")).not.toBeInTheDocument();
  });

  it("says in Japanese which log group of how many is fetched (U3:BR2.7, NFR9)", () => {
    renderPage(
      view({
        phase: "Fetching",
        canRun: false,
        progress: progress({
          groupNumber: 2,
          groupCount: 4,
          plannedStreamCount: 8,
          finishedStreamCount: 3,
          eventCount: 120,
        }),
      }),
      "ja",
    );
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "ログを取得中（ロググループ 2 / 4）：ストリーム 3 / 8、120 件",
    );
  });
});

describe("LogAnalyticsPage progress placement (261020-carryover-fixes FR4.4)", () => {
  const form = () => screen.getByTestId("log-analytics-form");
  const progressStatus = () => screen.getByTestId("log-analytics-progress");

  /** The progress is in the query container, after the run row, and not in the results. */
  function expectProgressBelowRunBar() {
    const runRow = within(form()).getByTestId("log-analytics-run");
    const shown = within(form()).getByTestId("log-analytics-progress");
    expect(runRow.compareDocumentPosition(shown) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(within(results()).queryByTestId("log-analytics-progress")).not.toBeInTheDocument();
  }

  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow([queryRow(0), queryRow(1)], offset, limit, runId, version),
    );
    vi.mocked(api.getCompletionCandidates).mockReset();
    vi.mocked(api.getCompletionCandidates).mockResolvedValue([]);
  });

  it("shows a fetch's progress in the query container below the run row, not in the results (FR4.4.1, FR4.4.3)", () => {
    renderPage(
      view({
        phase: "Fetching",
        canRun: false,
        cancelAvailable: true,
        progress: progress({
          plannedStreamCount: 744,
          selectedStreamCount: 744,
          finishedStreamCount: 37,
          eventCount: 719,
          failedGroupCount: 1,
          groupCount: 2,
        }),
      }),
    );
    expectProgressBelowRunBar();
    expect(progressStatus()).toHaveAttribute("role", "status");
    expect(progressStatus()).toHaveTextContent(
      "Fetching logs (log group 1 / 2): 37 / 744 streams, 719 events",
    );
    expect(within(form()).getByTestId("log-analytics-failed-groups")).toBeInTheDocument();
  });

  it("keeps only the previous result in the results while the next run goes (FR4.4.2)", async () => {
    renderPage(
      view({
        phase: "Evaluating",
        runId: 2,
        canRun: false,
        progress: progress({ evaluatedCount: 3, totalCount: 9 }),
        result: resultSummary({ runId: 1 }),
      }),
    );
    expectProgressBelowRunBar();
    expect(progressStatus()).toHaveTextContent("Running query: 3 / 9 events");
    expect(within(results()).getByTestId("query-result-previous")).toHaveTextContent(
      "Results of the previous run",
    );
    await waitFor(() =>
      expect(within(results()).getAllByTestId("query-result-row")[1]).toHaveTextContent("ERROR 1"),
    );
    expect(within(results()).queryByTestId("log-analytics-initial")).not.toBeInTheDocument();
  });

  it("shows no progress anywhere once the run is done (FR4.4.4)", async () => {
    const { rerender } = renderPage(
      view({ phase: "Fetching", runId: 2, canRun: false, progress: progress() }),
    );
    expect(progressStatus()).toBeInTheDocument();
    rerender(view({ phase: "Done", runId: 2, result: resultSummary({ runId: 2 }) }));
    expect(screen.queryByTestId("log-analytics-progress")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row")[1]).toHaveTextContent("ERROR 1"),
    );
    rerender(view({ phase: "Cancelled", runId: 3 }));
    expect(screen.queryByTestId("log-analytics-progress")).not.toBeInTheDocument();
  });

  it("leaves the results container with its heading only while a first run goes (FR4.4.2, A9)", () => {
    renderPage(
      view({
        phase: "Fetching",
        canRun: false,
        progress: progress({ groupNumber: null }),
        result: null,
      }),
    );
    expectProgressBelowRunBar();
    expect(progressStatus()).toHaveTextContent("Fetching logs: finding what is missing");
    expect(within(results()).getByRole("heading", { name: "Results" })).toBeInTheDocument();
    expect(within(results()).queryByTestId("query-result-table")).not.toBeInTheDocument();
    expect(within(results()).queryByTestId("log-analytics-initial")).not.toBeInTheDocument();
    expect(within(results()).queryByTestId("log-analytics-no-match")).not.toBeInTheDocument();
    expect(results()).not.toHaveTextContent("No results yet.");
  });
});

describe("LogAnalyticsPage export menu (261022-small-features U2:BR3.2, BR4.1)", () => {
  const exportMenu = () => within(within(results()).getByTestId("export-menu")).getByRole("button");
  const isInert = (button: HTMLElement) =>
    button.hasAttribute("disabled") || button.getAttribute("aria-disabled") === "true";

  it("sits in the results heading and mirrors canExport and the running job", () => {
    const { rerender } = renderPage(view({ phase: "Idle", result: null, canExport: false }));
    expect(exportMenu()).toHaveAccessibleName("Export");
    expect(isInert(exportMenu())).toBe(true);
    rerender(view({ phase: "Done", result: resultSummary({ rowCount: 2 }), canExport: true }));
    expect(isInert(exportMenu())).toBe(false);
    // The cut and the menu share the heading's actions.
    rerender(
      view({
        phase: "Done",
        result: resultSummary({ rowCount: 20, matchedCount: 1284, truncatedAt: 20 }),
        canExport: true,
      }),
    );
    expect(within(results()).getByTestId("log-analytics-truncated")).toHaveTextContent(
      "Showing 20",
    );
    expect(isInert(exportMenu())).toBe(false);
    rerender(
      view(
        { phase: "Done", result: resultSummary({ rowCount: 2 }), canExport: false },
        {
          export: {
            exportNumber: 1,
            status: "Running",
            request: { screen: "logAnalytics", format: "csv", target: "file" },
            rowCount: 2,
          },
        },
      ),
    );
    expect(isInert(exportMenu())).toBe(true);
  });

  it("hands the chosen export to onExport", async () => {
    const user = userEvent.setup();
    const { onExport } = renderPage(
      view({ phase: "Done", result: resultSummary({ rowCount: 2 }), canExport: true }),
    );
    await user.click(exportMenu());
    await user.click(screen.getByRole("menuitem", { name: "Copy as CSV" }));
    expect(onExport).toHaveBeenCalledWith("csv", "clipboard");
  });
});

describe("LogAnalyticsPage partial and cancelled results (261025-insights-stats U3)", () => {
  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, version, offset, limit) =>
      queryRowWindow([queryRow(0), queryRow(1)], offset, limit, runId, version),
    );
    vi.mocked(api.getCompletionCandidates).mockReset();
    vi.mocked(api.getCompletionCandidates).mockResolvedValue([]);
    vi.mocked(api.getQueryCatalog).mockReset();
    vi.mocked(api.getQueryCatalog).mockResolvedValue([...QUERY_CATALOG]);
  });

  const fetching = (result: InsightsView["result"], overrides: Partial<InsightsView> = {}) =>
    view({
      runId: 1,
      phase: "Fetching",
      canRun: false,
      cancelAvailable: true,
      progress: progress(),
      result,
      ...overrides,
    });
  const announcement = () => screen.getByTestId("log-analytics-result-announcement");

  it("marks a partial result with the log group and the events it holds, in both languages (BR2.6)", () => {
    const { rerender } = renderPage(
      fetching(partialSummary({ groupNumber: 2, groupCount: 5, included: 48_210 })),
    );
    expect(within(results()).getByTestId("log-analytics-partial-mark")).toHaveTextContent(
      "Partial results (fetching: log group 2 / 5, up to 48210 events)",
    );
    rerender(fetching(partialSummary({ groupCount: 1, included: 300 })));
    expect(screen.getByTestId("log-analytics-partial-mark")).toHaveTextContent(
      "Partial results (fetching: up to 300 events)",
    );
    expect(screen.getByTestId("log-analytics-partial-mark")).not.toHaveTextContent("log group");
    expect(screen.queryByTestId("query-result-previous")).not.toBeInTheDocument();
  });

  it("says the partial mark in Japanese too (NFR9)", () => {
    renderPage(fetching(partialSummary({ groupNumber: 1, groupCount: 3, included: 12 })), "ja");
    expect(screen.getByTestId("log-analytics-partial-mark")).toHaveTextContent(
      "途中の結果（取得中：ロググループ 1 / 3、12 件まで）",
    );
  });

  it("drops the mark with the final result and shows the table all along (BR2.6, BR2.9)", async () => {
    const { rerender } = renderPage(fetching(partialSummary()));
    expect(await screen.findAllByTestId("query-result-row")).not.toHaveLength(0);
    expect(within(results()).getByTestId("export-menu")).toBeInTheDocument();
    rerender(view({ runId: 1, phase: "Done", result: resultSummary({ version: 2 }) }));
    expect(screen.queryByTestId("log-analytics-partial-mark")).not.toBeInTheDocument();
    expect(screen.queryByTestId("log-analytics-cancelled-mark")).not.toBeInTheDocument();
  });

  it("reads out only the changes of state, not every new version (BR2.7)", () => {
    const { rerender } = renderPage(fetching(null));
    expect(announcement()).toHaveTextContent("");
    expect(announcement()).toHaveAttribute("aria-live", "polite");
    rerender(fetching(partialSummary({}, { version: 1 })));
    expect(announcement()).toHaveTextContent("Partial results are shown.");
    const before = announcement().textContent;
    const node = announcement().firstChild;
    rerender(fetching(partialSummary({ included: 90 }, { version: 2, rowCount: 9 })));
    expect(announcement().textContent).toBe(before);
    expect(announcement().firstChild).toBe(node);
    rerender(view({ runId: 1, phase: "Done", result: resultSummary({ version: 3 }) }));
    expect(announcement()).toHaveTextContent("The final results are shown.");
    rerender(view({ runId: 1, phase: "Cancelled", result: cancelledSummary({ version: 3 }) }));
    expect(announcement()).toHaveTextContent("Results of part of the logs are shown (cancelled).");
  });

  it("says nothing of the previous run's result while the next run fetches", () => {
    renderPage(fetching(resultSummary({ runId: 1 }), { runId: 2 }));
    expect(announcement()).toHaveTextContent("");
    expect(screen.getByTestId("query-result-previous")).toBeInTheDocument();
  });

  it("marks a cancelled result and lists its failed streams too (BR3.4, FR5.4)", async () => {
    const user = userEvent.setup();
    renderPage(
      view({
        runId: 1,
        phase: "Cancelled",
        partial: true,
        failedStreams: [
          {
            logGroupName: "/aws/lambda/orders",
            logStreamName: "stream-x",
            failure: { kind: "AccessDenied", safeDetail: "", retryable: false },
          },
        ],
        result: cancelledSummary(),
        canExport: true,
      }),
    );
    expect(within(results()).getByTestId("log-analytics-cancelled-mark")).toHaveTextContent(
      "Results of part of the logs (cancelled)",
    );
    expect(screen.getByTestId("log-analytics-cancelled")).toHaveTextContent(
      "Query cancelled. Results of part of the logs are shown.",
    );
    expect(screen.getByTestId("log-analytics-partial")).toHaveTextContent("1 stream");
    await user.click(screen.getByTestId("log-analytics-show-failures-button"));
    expect(screen.getByTestId("failure-list")).toHaveTextContent("stream-x");
  });

  it("says just cancelled when no result of the run is left (BR3.2 ELSE)", () => {
    renderPage(view({ runId: 2, phase: "Cancelled", result: resultSummary({ runId: 1 }) }));
    expect(screen.getByTestId("log-analytics-cancelled")).toHaveTextContent("Query cancelled.");
    expect(screen.getByTestId("log-analytics-cancelled")).not.toHaveTextContent("part of the logs");
    expect(screen.queryByTestId("log-analytics-cancelled-mark")).not.toBeInTheDocument();
  });

  it("says the cancelled mark and words in Japanese too (NFR9)", () => {
    renderPage(view({ runId: 1, phase: "Cancelled", result: cancelledSummary() }), "ja");
    expect(screen.getByTestId("log-analytics-cancelled-mark")).toHaveTextContent(
      "一部のログだけの結果（中断）",
    );
    expect(screen.getByTestId("log-analytics-cancelled")).toHaveTextContent(
      "一部のログだけの結果を出しました。",
    );
  });

  it("keeps the inputs and [Export] locked while cancelling and lets [Cancel] be pressed once more (BR3.2, BR3.7)", async () => {
    const user = userEvent.setup();
    const cancelling = fetching(partialSummary(), {
      phase: "Cancelling",
      cancelling: true,
      cancelAvailable: true,
      canExport: false,
      progress: progress({ evaluatedCount: 10_000, totalCount: 25_000 }),
    });
    const { onCancel, onChange, rerender } = renderPage(cancelling);
    expect(runButton()).toBeDisabled();
    fireEvent.change(editor(), { target: { value: "changed" } });
    expect(onChange).not.toHaveBeenCalled();
    const exportButton = within(within(results()).getByTestId("export-menu")).getByRole("button");
    expect(
      exportButton.hasAttribute("disabled") ||
        exportButton.getAttribute("aria-disabled") === "true",
    ).toBe(true);
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent(
      "Making the results of what was fetched: 10000 / 25000 events",
    );
    const cancel = screen.getByTestId("log-analytics-cancel-button");
    expect(cancel).toHaveTextContent("Cancel");
    await user.click(cancel);
    expect(onCancel).toHaveBeenCalledTimes(1);
    rerender({
      ...cancelling,
      insights: { ...cancelling.insights, cancelAvailable: false },
    });
    expect(screen.getByTestId("log-analytics-cancel-button")).toHaveTextContent("Cancelling…");
    await user.click(screen.getByTestId("log-analytics-cancel-button"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("says the fetch is being stopped before the results are made (BR3.7)", () => {
    renderPage(fetching(null, { phase: "Cancelling", cancelling: true }), "ja");
    expect(screen.getByTestId("log-analytics-progress")).toHaveTextContent("取得を止めています…");
  });

  it("keeps the aggregates cut note on a partial result (U2 review R-03)", () => {
    renderPage(
      fetching(
        aggregatesSummary(["n"], {
          stage: "partial",
          coverage: { groupNumber: 1, groupCount: 1, includedEventCount: 20_000 },
          rowCount: 10_000,
          matchedCount: 10_003,
          truncatedAt: 10_000,
        }),
      ),
    );
    expect(within(results()).getByTestId("log-analytics-truncated")).toBeInTheDocument();
    expect(screen.getByTestId("log-analytics-partial-mark")).toBeInTheDocument();
  });
});
