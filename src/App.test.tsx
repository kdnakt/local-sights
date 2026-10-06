import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  FetchProgressUpdate,
  FilterProgressUpdate,
  FilterSummary,
  RowWindow,
  SessionView,
} from "./api";
import { App } from "./App";
import { connectedView, logEvent, rowWindow, sessionView } from "./test/fixtures";

const handlers: {
  session?: (view: SessionView) => void;
  progress?: (update: FetchProgressUpdate) => void;
  filter?: (update: FilterProgressUpdate) => void;
} = {};

vi.mock("./api", async (importOriginal) => {
  const original = await importOriginal<typeof import("./api")>();
  return {
    ...original,
    getSession: vi.fn(),
    updateInput: vi.fn(),
    startFetch: vi.fn(),
    selectProfile: vi.fn(),
    selectRegion: vi.fn(),
    confirmConnectionChange: vi.fn(),
    cancelConnectionChange: vi.fn(),
    reloadLogGroups: vi.fn(),
    updateLogGroupFilter: vi.fn(),
    selectLogGroup: vi.fn(),
    setFailureListOpen: vi.fn(),
    selectTimeZone: vi.fn(),
    setLogFilter: vi.fn(),
    getRows: vi.fn(),
    findRowPosition: vi.fn(),
    onSessionChanged: vi.fn(async (handler: (view: SessionView) => void) => {
      handlers.session = handler;
      return () => undefined;
    }),
    onFetchProgress: vi.fn(async (handler: (update: FetchProgressUpdate) => void) => {
      handlers.progress = handler;
      return () => undefined;
    }),
    onFilterProgress: vi.fn(async (handler: (update: FilterProgressUpdate) => void) => {
      handlers.filter = handler;
      return () => undefined;
    }),
  };
});

const api = await import("./api");

const events = [logEvent(0), logEvent(1, "middle", "stream-b"), logEvent(2)];

function push(view: SessionView) {
  act(() => {
    handlers.session?.(view);
  });
}

describe("App", () => {
  beforeEach(() => {
    vi.mocked(api.getSession).mockResolvedValue(sessionView());
    vi.mocked(api.startFetch).mockResolvedValue(undefined);
    vi.mocked(api.updateInput).mockResolvedValue(sessionView());
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) =>
      rowWindow(events, offset, limit),
    );
    vi.mocked(api.findRowPosition).mockResolvedValue({ position: null, timelineVersion: 0 });
    for (const command of [
      api.selectProfile,
      api.selectRegion,
      api.confirmConnectionChange,
      api.cancelConnectionChange,
      api.reloadLogGroups,
      api.updateLogGroupFilter,
      api.selectLogGroup,
      api.setFailureListOpen,
      api.selectTimeZone,
      api.setLogFilter,
    ]) {
      vi.mocked(command).mockReset();
      vi.mocked(command).mockResolvedValue(undefined);
    }
  });

  it("shows the rows the core holds and follows the timeline version", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", eventCount: 3, timelineVersion: 1 }));
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(screen.getByText("middle")).toBeInTheDocument();
    expect(screen.getByText("stream-b")).toBeInTheDocument();

    // A connection change discards the logs: count 0, new version.
    push(connectedView({ phase: "Idle", eventCount: 0, timelineVersion: 2 }));
    await waitFor(() => expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0));
  });

  it("applies the light progress messages of the running job only", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", currentJobId: 7, timelineVersion: 1 }));
    const update = (jobId: number, eventCount: number, timelineVersion: number) =>
      act(() => {
        handlers.progress?.({
          jobId,
          progress: {
            seenStreamCount: 2,
            selectedStreamCount: 2,
            plannedStreamCount: 2,
            finishedStreamCount: 1,
            eventCount,
          },
          eventCount,
          timelineVersion,
          filterSummary: null,
          filterResultVersion: 0,
        });
      });
    update(7, 3, 2);
    expect(screen.getByTestId("status-line-fetching")).toHaveTextContent(
      "Fetching… 1/2 streams, 3 events so far",
    );
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));

    update(8, 99, 9);
    expect(screen.getByTestId("status-line-fetching")).toHaveTextContent("3 events so far");

    push(sessionView({ phase: "Done", currentJobId: 7, eventCount: 3, timelineVersion: 2 }));
    update(7, 50, 5);
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("3 events");
  });

  it("sends the connection generation with fetch, reload and log group selection", async () => {
    vi.mocked(api.getSession).mockResolvedValue(
      connectedView({ connectionGeneration: 4, canFetch: true }),
    );
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(await screen.findByTestId("fetch-form-submit-button"));
    expect(api.startFetch).toHaveBeenCalledWith(4);
    await user.click(screen.getByTestId("log-group-pane-reload-button"));
    expect(api.reloadLogGroups).toHaveBeenCalledWith(4);
    await user.click(screen.getAllByTestId("log-group-pane-option")[0]!);
    expect(api.selectLogGroup).toHaveBeenCalledWith("/aws/ecs/web", 4);

    push(connectedView({ connectionGeneration: 5 }));
    await user.click(screen.getByTestId("log-group-pane-reload-button"));
    expect(api.reloadLogGroups).toHaveBeenLastCalledWith(5);
  });

  it("opens and closes the list of failed streams through the core", async () => {
    const failure = { kind: "Throttled" as const, safeDetail: "kind=Throttled", retryable: true };
    const done = sessionView({
      phase: "Done",
      eventCount: 3,
      timelineVersion: 1,
      failedStreams: [{ logStreamName: "s1", failure }],
    });
    vi.mocked(api.getSession).mockResolvedValue(done);
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(await screen.findByTestId("status-line-failures-button"));
    expect(api.setFailureListOpen).toHaveBeenCalledWith(true);

    push({ ...done, failureListOpen: true });
    expect(screen.getByTestId("failure-list-close-button")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(api.setFailureListOpen).toHaveBeenLastCalledWith(false);
    push({ ...done, failureListOpen: false });
    expect(screen.queryByTestId("failure-list")).not.toBeInTheDocument();
  });

  it("shows a refused command as a message and dismisses it with Escape", async () => {
    vi.mocked(api.startFetch).mockRejectedValue({ key: "session.busy" });
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(await screen.findByTestId("fetch-form-submit-button"));
    expect(await screen.findByTestId("command-error")).toHaveTextContent("A fetch is in progress.");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("command-error")).not.toBeInTheDocument());
  });

  it("renders the screen in Japanese for the ja locale", async () => {
    render(<App locale="ja" />);
    expect(await screen.findByTestId("fetch-form-submit-button")).toHaveTextContent("取得");
    expect(screen.getByRole("columnheader", { name: "時刻（ローカル）" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "ストリーム名" })).toBeInTheDocument();
  });

  it("asks before a connection change and forwards the answer", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(
      connectedView({
        pendingChange: { proposedProfile: null, proposedRegion: "eu-west-1" },
        canChangeConnection: false,
      }),
    );
    expect(screen.getByTestId("confirm-dialog-cancel-button")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(api.cancelConnectionChange).toHaveBeenCalledTimes(1);
    await user.click(screen.getByTestId("confirm-dialog-change-button"));
    expect(api.confirmConnectionChange).toHaveBeenCalledTimes(1);
    push(connectedView());
    expect(screen.queryByTestId("confirm-dialog")).not.toBeInTheDocument();
  });

  it("forwards profile, region and filter choices", async () => {
    vi.mocked(api.getSession).mockResolvedValue(connectedView());
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.selectOptions(
      await screen.findByTestId("connection-bar-profile-select"),
      "named:prod",
    );
    expect(api.selectProfile).toHaveBeenCalledWith({ kind: "Named", profileName: "prod" });
    await user.selectOptions(screen.getByTestId("connection-bar-region-select"), "us-east-1");
    expect(api.selectRegion).toHaveBeenCalledWith("us-east-1");
    await user.type(screen.getByTestId("log-group-pane-filter-input"), "w");
    expect(api.updateLogGroupFilter).toHaveBeenCalledWith("w");
  });

  it("switches the time zone through the core and re-reads the rows without fetching", async () => {
    const user = userEvent.setup();
    const startInput = { text: "2024-03-01 10:00:00", instant: 1_709_254_800_000, error: null };
    vi.mocked(api.getSession).mockResolvedValue(
      connectedView({ phase: "Done", eventCount: 3, timelineVersion: 1, startInput }),
    );
    render(<App locale="en" />);
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(screen.getAllByRole("cell")[0]).toHaveTextContent("core time 0");
    expect(screen.getByTestId("fetch-form-start-input")).toHaveValue("2024-03-01 10:00:00");
    const readsBefore = vi.mocked(api.getRows).mock.calls.length;

    await user.click(screen.getByTestId("time-zone-toggle-utc-radio"));
    expect(api.selectTimeZone).toHaveBeenCalledWith("Utc");

    // The core answers with the new zone, the rewritten input and the same
    // timeline version; the rows come back with times in UTC.
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) =>
      rowWindow(
        events.map((event) => ({ ...event, displayTime: `utc ${event.sequence}` })),
        offset,
        limit,
      ),
    );
    push(
      connectedView({
        phase: "Done",
        eventCount: 3,
        timelineVersion: 1,
        timeZone: "Utc",
        startInput: { ...startInput, text: "2024-03-01 01:00:00" },
      }),
    );
    await waitFor(() => expect(screen.getAllByRole("cell")[0]).toHaveTextContent("utc 0"));
    expect(vi.mocked(api.getRows).mock.calls.length).toBeGreaterThan(readsBefore);
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
    expect(screen.getByTestId("time-zone-toggle-utc-radio")).toBeChecked();
    expect(screen.getByTestId("fetch-form-start-input")).toHaveValue("2024-03-01 01:00:00");
    expect(screen.getByText("Start (UTC)")).toBeInTheDocument();
    expect(api.startFetch).not.toHaveBeenCalled();
  });

  // ---- U5: the log filter (BR1.3, BR3.1, BR3.3, BR3.6) ----

  function filterSummary(overrides: Partial<FilterSummary> = {}): FilterSummary {
    return {
      filterId: 1,
      matchedCount: 2,
      allCount: 3,
      status: "Ready",
      resultVersion: 5,
      ...overrides,
    };
  }

  /** A filtered `get_rows` answer from result version `version`. */
  function filteredRows(messages: string[], version: number): RowWindow {
    return {
      offset: 0,
      rows: messages.map((message, index) => logEvent(index, message)),
      totalCount: messages.length,
      timelineVersion: 1,
      filtered: true,
      allCount: 3,
      resultVersion: version,
    };
  }

  function sendFilter(summary: FilterSummary | null, version: number) {
    act(() => {
      handlers.filter?.({ filterSummary: summary, filterResultVersion: version });
    });
  }

  it("re-reads the rows when the result version changes, even with the same count", async () => {
    vi.mocked(api.getRows).mockResolvedValue(filteredRows(["first error", "second error"], 5));
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        phase: "Done",
        eventCount: 3,
        timelineVersion: 1,
        logFilter: "error",
        filterSummary: filterSummary(),
        filterResultVersion: 5,
      }),
    );
    render(<App locale="en" />);
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(2));
    expect(screen.getByTestId("status-line-filter")).toHaveTextContent("Filtered: 2 of 3 events");
    expect(screen.getByTestId("log-filter-input")).toHaveValue("error");
    const reads = vi.mocked(api.getRows).mock.calls.length;

    vi.mocked(api.getRows).mockResolvedValue(filteredRows(["other error", "second error"], 6));
    sendFilter(filterSummary({ resultVersion: 6 }), 6);
    await waitFor(() => expect(screen.getByText("other error")).toBeInTheDocument());
    expect(vi.mocked(api.getRows).mock.calls.length).toBeGreaterThan(reads);
  });

  it("drops rows of an older result version and filter messages older than the view", async () => {
    vi.mocked(api.getRows).mockResolvedValue(filteredRows(["stale row"], 6));
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        phase: "Done",
        eventCount: 3,
        timelineVersion: 1,
        filterSummary: filterSummary({ matchedCount: 1, resultVersion: 7 }),
        filterResultVersion: 7,
      }),
    );
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    await waitFor(() => expect(api.getRows).toHaveBeenCalled());
    await act(async () => undefined);
    expect(screen.queryByText("stale row")).not.toBeInTheDocument();

    sendFilter(filterSummary({ matchedCount: 9, status: "Filtering", resultVersion: 3 }), 3);
    expect(screen.getByTestId("status-line-filter")).toHaveTextContent("Filtered: 1 of 3 events");
    expect(screen.queryByTestId("status-line-filtering")).not.toBeInTheDocument();

    vi.mocked(api.getRows).mockResolvedValue(filteredRows(["fresh row"], 8));
    sendFilter(filterSummary({ matchedCount: 1, resultVersion: 8 }), 8);
    await waitFor(() => expect(screen.getByText("fresh row")).toBeInTheDocument());
  });

  it("hands the log filter text to the core after the typing pause", async () => {
    vi.mocked(api.getSession).mockResolvedValue(sessionView({ phase: "Fetching" }));
    render(<App locale="en" />);
    const input = await screen.findByTestId("log-filter-input");
    expect(input).toBeEnabled();
    vi.useFakeTimers();
    try {
      fireEvent.change(input, { target: { value: "time" } });
      fireEvent.change(input, { target: { value: "timeout" } });
      act(() => {
        vi.advanceTimersByTime(299);
      });
      expect(api.setLogFilter).not.toHaveBeenCalled();
      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(api.setLogFilter).toHaveBeenCalledTimes(1);
      expect(api.setLogFilter).toHaveBeenCalledWith("timeout");
    } finally {
      vi.useRealTimers();
    }
    expect(api.startFetch).not.toHaveBeenCalled();
  });
});
