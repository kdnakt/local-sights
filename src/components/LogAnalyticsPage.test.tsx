import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InsightsView, SessionView } from "../api";
import { createTranslator } from "../i18n/messages";
import { connectedView, insightsView, queryRow, queryRowWindow, t } from "../test/fixtures";
import { LogAnalyticsPage } from "./LogAnalyticsPage";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, getQueryRows: vi.fn() };
});
vi.mock("../aceLoader", () => ({ loadAce: () => Promise.resolve({}) }));
vi.mock("@cloudscape-design/components/code-editor", () => import("../test/codeEditorMock"));

const api = await import("../api");

function view(insights: Partial<InsightsView> = {}, session: Partial<SessionView> = {}) {
  return connectedView({ insights: insightsView(insights), ...session });
}

function renderPage(session: SessionView = view(), locale: "en" | "ja" = "en") {
  const handlers = { onChange: vi.fn(), onRun: vi.fn(), onCancel: vi.fn(), onError: vi.fn() };
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

const runButton = () => screen.getByTestId("log-analytics-run-button");
const editor = () => screen.getByRole("textbox", { name: "Query" });
const results = () => screen.getByTestId("log-analytics-results");

describe("LogAnalyticsPage (261017-insights-query U1, refined-mockups screens 2-7)", () => {
  beforeEach(() => {
    vi.mocked(api.getQueryRows).mockReset();
    vi.mocked(api.getQueryRows).mockImplementation(async (runId, epoch, offset, limit) =>
      queryRowWindow([queryRow(0), queryRow(1)], offset, limit, runId, epoch),
    );
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
        progress: {
          selectedStreamCount: 37,
          plannedStreamCount: 37,
          finishedStreamCount: 14,
          eventCount: 48_210,
          evaluatedCount: 0,
          totalCount: 0,
        },
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
      "Fetching logs: 14 / 37 streams, 48210 events",
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
        progress: {
          selectedStreamCount: 2,
          plannedStreamCount: 2,
          finishedStreamCount: 2,
          eventCount: 25_000,
          evaluatedCount: 10_000,
          totalCount: 25_000,
        },
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
        result: { runId: 1, holdingsEpoch: 2, rowCount: 0, evaluatedCount: 40 },
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

  it("shows the rows of a result with their count", async () => {
    renderPage(
      view({
        phase: "Done",
        result: { runId: 1, holdingsEpoch: 2, rowCount: 2, evaluatedCount: 40 },
      }),
    );
    expect(results()).toHaveTextContent("(2 matched)");
    await waitFor(() =>
      expect(screen.getAllByTestId("query-result-row")[1]).toHaveTextContent("ERROR 1"),
    );
    expect(api.getQueryRows).toHaveBeenCalledWith(1, 2, 0, 2);
  });

  it("warns that a partial result is based on part of the logs and lists the failures (U1:BR2.5)", async () => {
    const user = userEvent.setup();
    renderPage(
      view({
        phase: "Done",
        partial: true,
        failedStreams: [
          {
            logStreamName: "stream-x",
            failure: { kind: "AccessDenied", safeDetail: "", retryable: false },
          },
        ],
        result: { runId: 1, holdingsEpoch: 2, rowCount: 2, evaluatedCount: 40 },
      }),
    );
    expect(screen.getByTestId("log-analytics-partial")).toHaveTextContent(
      "Results are based on part of the logs. 1 streams could not be fetched.",
    );
    await user.click(screen.getByTestId("log-analytics-show-failures-button"));
    expect(screen.getByTestId("failure-list")).toHaveTextContent("stream-x");
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
        result: { runId: 1, holdingsEpoch: 2, rowCount: 2, evaluatedCount: 40 },
      }),
    );
    rerender(
      view({
        phase: "QueryError",
        runId: 2,
        result: { runId: 1, holdingsEpoch: 2, rowCount: 2, evaluatedCount: 40 },
        problem: {
          kind: "Unsupported",
          messageKey: "query.unsupported.command",
          word: "stats",
          span: { line: 1, column: 30, length: 5 },
        },
      }),
    );
    expect(screen.getByTestId("query-problem")).toHaveTextContent(
      'Line 1, column 30: "stats" is not supported in this version. Supported commands: filter.',
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
});
