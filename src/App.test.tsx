import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
import { connectedView, logEvent, rowWindow, sessionView, ssoLoginView } from "./test/fixtures";

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
    cancelFetch: vi.fn(),
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
    openSettings: vi.fn(),
    cancelSettings: vi.fn(),
    saveSettings: vi.fn(),
    clearCache: vi.fn(),
    getRows: vi.fn(),
    rowPositions: vi.fn(),
    cancelClose: vi.fn(),
    confirmClose: vi.fn(),
    startSsoLogin: vi.fn(),
    cancelSsoLogin: vi.fn(),
    dismissSsoLogin: vi.fn(),
    openSsoUrl: vi.fn(),
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

/** The trigger button of the top bar dropdown with `testId`. */
const dropdown = (testId: string) => within(screen.getByTestId(testId)).getByRole("button");
const logFilter = () => screen.getByRole("searchbox", { name: "Filter messages" });
const dateInput = (side: "start" | "end") =>
  within(screen.getByTestId(`fetch-form-${side}-date`)).getByRole("textbox");
const timeInput = (side: "start" | "end") =>
  within(screen.getByTestId(`fetch-form-${side}-time`)).getByRole("textbox");

/** Cloudscape's modal reads the key code; user-event sends none, so Escape is fired by hand. */
const pressEscape = () =>
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape", keyCode: 27 });

describe("App", () => {
  beforeEach(() => {
    // The pane's width and collapsed state are remembered per browser.
    window.localStorage.clear();
    vi.mocked(api.getSession).mockResolvedValue(sessionView());
    vi.mocked(api.startFetch).mockResolvedValue(undefined);
    vi.mocked(api.updateInput).mockResolvedValue(sessionView());
    vi.mocked(api.getRows).mockImplementation(async (offset, limit) =>
      rowWindow(events, offset, limit),
    );
    vi.mocked(api.rowPositions).mockImplementation(async (keys) => ({
      positions: keys.map(() => null),
      timelineVersion: 0,
      resultVersion: 0,
      discardGeneration: 0,
      totalCount: 0,
    }));
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
      api.openSettings,
      api.cancelSettings,
      api.saveSettings,
      api.clearCache,
      api.cancelClose,
      api.confirmClose,
      api.startSsoLogin,
      api.cancelSsoLogin,
      api.dismissSsoLogin,
      api.openSsoUrl,
      api.cancelFetch,
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

    // A connection change discards the logs: count 0, new version, empty state.
    push(connectedView({ phase: "Idle", eventCount: 0, timelineVersion: 2 }));
    await waitFor(() => expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0));
    expect(screen.getByTestId("log-table-empty")).toHaveTextContent(
      "No events. Select a log group, set a time range, then choose Fetch.",
    );
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
          cacheSaving: false,
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

  it("starts the elapsed clock from getSession and keeps it across progress messages (261012-parallel-fetch U4, NFR6.9)", async () => {
    const elapsed = { running: true, elapsedMs: 5_999, fetchNumber: 3 };
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({ phase: "Fetching", currentJobId: 7, elapsed }),
    );
    render(<App locale="en" />);
    const part = await screen.findByTestId("status-line-elapsed");
    expect(part).toHaveAttribute("aria-hidden", "true");
    // The screen's own clock moves it on without any message from the core.
    const shownSeconds = () =>
      Number(
        /Elapsed 00:(\d\d)/.exec(screen.getByTestId("status-line-elapsed").textContent ?? "")?.[1],
      );
    await waitFor(() => expect(shownSeconds()).toBeGreaterThanOrEqual(6));
    const beforeProgress = shownSeconds();

    act(() => {
      handlers.progress?.({
        jobId: 7,
        progress: {
          seenStreamCount: 2,
          selectedStreamCount: 2,
          plannedStreamCount: 2,
          finishedStreamCount: 1,
          eventCount: 3,
        },
        eventCount: 3,
        timelineVersion: 1,
        filterSummary: null,
        filterResultVersion: 0,
        cacheSaving: true,
      });
    });
    expect(screen.getByTestId("status-line-cache-saving")).toBeInTheDocument();
    expect(shownSeconds()).toBeGreaterThanOrEqual(beforeProgress);

    push(
      sessionView({
        phase: "Done",
        currentJobId: 7,
        eventCount: 3,
        elapsed: { running: false, elapsedMs: 31_400, fetchNumber: 3 },
      }),
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 00:31");
    expect(screen.getByTestId("status-line-elapsed")).not.toHaveAttribute("aria-hidden");

    // A connection change clears the result and the elapsed time with it.
    push(connectedView({ phase: "Idle", elapsed: null }));
    expect(screen.queryByTestId("status-line-elapsed")).not.toBeInTheDocument();
  });

  it("keeps the same elapsed object when progress and filter messages are composed (NFR6.9)", () => {
    const view = sessionView({
      phase: "Fetching",
      currentJobId: 7,
      filterResultVersion: 2,
      elapsed: { running: true, elapsedMs: 1_000, fetchNumber: 1 },
    });
    const progress = {
      jobId: 7,
      progress: {
        seenStreamCount: 1,
        selectedStreamCount: 1,
        plannedStreamCount: 1,
        finishedStreamCount: 0,
        eventCount: 1,
      },
      eventCount: 1,
      timelineVersion: 1,
      filterSummary: null,
      filterResultVersion: 3,
      cacheSaving: false,
    };
    expect(api.withProgress(view, progress)?.elapsed).toBe(view.elapsed);
    expect(api.withFilter(view, { filterSummary: null, filterResultVersion: 4 })?.elapsed).toBe(
      view.elapsed,
    );
    const newer = { ...view, filterResultVersion: 1, elapsed: { ...view.elapsed! } };
    expect(api.mergeSessionView(view, newer).elapsed).toBe(newer.elapsed);
  });

  it("sends the connection generation with fetch, reload and log group selection", async () => {
    vi.mocked(api.getSession).mockResolvedValue(
      connectedView({
        connectionGeneration: 4,
        canFetch: true,
        startInput: { text: "2024/03/01 10:00:00", instant: 1, error: null },
        endInput: { text: "2024/03/01 12:00:00", instant: 2, error: null },
      }),
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
    pressEscape();
    expect(api.setFailureListOpen).toHaveBeenLastCalledWith(false);
    push({ ...done, failureListOpen: false });
    expect(screen.queryByTestId("failure-list")).not.toBeInTheDocument();
    // The focus goes back to the button that opened the list.
    expect(screen.getByTestId("status-line-failures-button")).toHaveFocus();
  });

  it("shows a refused command as an error alert and dismisses it with Escape or its button", async () => {
    vi.mocked(api.startFetch).mockRejectedValue({ key: "session.busy" });
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        canFetch: true,
        startInput: { text: "2024/03/01 10:00:00", instant: 1, error: null },
        endInput: { text: "2024/03/01 12:00:00", instant: 2, error: null },
      }),
    );
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(await screen.findByTestId("fetch-form-submit-button"));
    expect(await screen.findByTestId("command-error")).toHaveTextContent("A fetch is in progress.");
    expect(screen.getByTestId("command-error")).toHaveAttribute("role", "alert");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("command-error")).not.toBeInTheDocument());
    await user.click(screen.getByTestId("fetch-form-submit-button"));
    await user.click(
      within(await screen.findByTestId("command-error")).getByRole("button", {
        name: "Dismiss (Esc)",
      }),
    );
    await waitFor(() => expect(screen.queryByTestId("command-error")).not.toBeInTheDocument());
  });

  it("renders the screen in Japanese for the ja locale", async () => {
    render(<App locale="ja" />);
    expect(await screen.findByTestId("fetch-form-submit-button")).toHaveTextContent("取得");
    expect(screen.getByRole("columnheader", { name: "時刻（ローカル）" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "ストリーム名" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "ロググループ" })).toBeInTheDocument();
    expect(screen.getByRole("searchbox", { name: "メッセージを絞り込み" })).toBeInTheDocument();
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
    pressEscape();
    expect(api.cancelConnectionChange).toHaveBeenCalledTimes(1);
    await user.click(screen.getByTestId("confirm-dialog-change-button"));
    expect(api.confirmConnectionChange).toHaveBeenCalledTimes(1);
    push(connectedView());
    expect(screen.queryByTestId("confirm-dialog")).not.toBeInTheDocument();
  });

  it("forwards profile, region and log group search choices from the top bar and the pane", async () => {
    vi.mocked(api.getSession).mockResolvedValue(connectedView());
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(
      await screen.findByTestId("top-bar-profile").then(() => dropdown("top-bar-profile")),
    );
    await user.click(screen.getByRole("menuitem", { name: "prod" }));
    expect(api.selectProfile).toHaveBeenCalledWith({ kind: "Named", profileName: "prod" });
    await user.click(dropdown("top-bar-region"));
    await user.click(screen.getByRole("menuitem", { name: "us-east-1" }));
    expect(api.selectRegion).toHaveBeenCalledWith("us-east-1");
    await user.type(screen.getByRole("searchbox", { name: "Find log groups" }), "w");
    expect(api.updateLogGroupFilter).toHaveBeenCalledWith("w");
  });

  it("shows at startup the SDK default profile, its region and the loading list the core selected, without any click (gate fix 14c)", async () => {
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        connection: { profile: { kind: "SdkDefault" }, region: "us-east-1" },
        canReload: true,
        logGroups: {
          status: "Loading",
          visibleGroups: [],
          totalCount: 0,
          emptyState: null,
          failure: null,
        },
      }),
    );
    render(<App locale="en" />);
    expect(await screen.findByTestId("top-bar-profile")).toHaveTextContent("Profile: Default");
    expect(screen.getByTestId("top-bar-region")).toHaveTextContent("us-east-1");
    expect(screen.getByRole("table", { name: "Log groups" })).toHaveTextContent("Loading…");
    // The selection is the core's: the screen sends nothing at startup.
    expect(api.selectProfile).not.toHaveBeenCalled();
    expect(api.selectRegion).not.toHaveBeenCalled();
    expect(api.reloadLogGroups).not.toHaveBeenCalled();
  });

  it("shows at startup the default range the core filled in, yesterday to today, split into the date and time fields (gate fix 16a)", async () => {
    // The core fills yesterday 00:00:00 to today 23:59:59 in the local zone;
    // the screen only splits each text into its date and time field.
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        startInput: { text: "2024/02/29 00:00:00", instant: 1_709_132_400_000, error: null },
        endInput: { text: "2024/03/01 23:59:59", instant: 1_709_305_199_000, error: null },
        validationErrors: ["selection.regionRequired", "selection.logGroupRequired"],
        canFetch: false,
      }),
    );
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    expect(dateInput("start")).toHaveValue("2024/02/29");
    expect(timeInput("start")).toHaveValue("00:00:00");
    expect(dateInput("end")).toHaveValue("2024/03/01");
    expect(timeInput("end")).toHaveValue("23:59:59");
    // No date or time reason: the filled values are valid input.
    expect(screen.queryByText(/yyyy\/mm\/dd/)).not.toBeInTheDocument();
    // The values are the core's: the screen sends nothing back at startup.
    expect(api.updateInput).not.toHaveBeenCalled();
    expect(api.startFetch).not.toHaveBeenCalled();
  });

  it("switches the time zone through the core, rewrites the date and time fields and re-reads the rows without fetching", async () => {
    const user = userEvent.setup();
    const startInput = { text: "2024/03/01 10:00:00", instant: 1_709_254_800_000, error: null };
    vi.mocked(api.getSession).mockResolvedValue(
      connectedView({ phase: "Done", eventCount: 3, timelineVersion: 1, startInput }),
    );
    render(<App locale="en" />);
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(screen.getAllByRole("gridcell")[0]).toHaveTextContent("core time 0");
    expect(dateInput("start")).toHaveValue("2024/03/01");
    expect(timeInput("start")).toHaveValue("10:00:00");
    const readsBefore = vi.mocked(api.getRows).mock.calls.length;

    await user.click(dropdown("top-bar-time-zone"));
    await user.click(screen.getByRole("menuitem", { name: "UTC" }));
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
        startInput: { ...startInput, text: "2024/03/01 01:00:00" },
      }),
    );
    await waitFor(() => expect(screen.getAllByRole("gridcell")[0]).toHaveTextContent("utc 0"));
    expect(vi.mocked(api.getRows).mock.calls.length).toBeGreaterThan(readsBefore);
    expect(screen.getByRole("columnheader", { name: "Time (UTC)" })).toBeInTheDocument();
    expect(dropdown("top-bar-time-zone")).toHaveTextContent("Time zone: UTC");
    expect(dateInput("start")).toHaveValue("2024/03/01");
    expect(timeInput("start")).toHaveValue("01:00:00");
    expect(screen.getByRole("group", { name: "Start (UTC)" })).toBeInTheDocument();
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
    expect(logFilter()).toHaveValue("error");
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

  it("says in the list when the filter matches nothing (R-06)", async () => {
    vi.mocked(api.getSession).mockResolvedValue(
      sessionView({
        phase: "Done",
        eventCount: 3,
        timelineVersion: 1,
        logFilter: "zzz",
        filterSummary: filterSummary({ matchedCount: 0, resultVersion: 2 }),
        filterResultVersion: 2,
      }),
    );
    render(<App locale="en" />);
    expect(await screen.findByTestId("log-table-empty")).toHaveTextContent(
      "No events match the filter.",
    );
  });

  it("hands the log filter text to the core after the typing pause", async () => {
    vi.mocked(api.getSession).mockResolvedValue(sessionView({ phase: "Fetching" }));
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    const input = logFilter();
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

  it("opens the settings dialog from the gear button, saves through the core and returns the focus", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    const settingsButton = screen.getByTestId("top-bar-settings-button");
    await user.click(settingsButton);
    expect(api.openSettings).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("settings-dialog")).not.toBeInTheDocument();

    push(sessionView({ settingsDialog: "Open" }));
    expect(screen.getByTestId("settings-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("fetch-form-submit-button")).toBeDisabled();
    expect(logFilter()).toBeDisabled();
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByTestId("settings-dialog-save-button"));
    expect(api.saveSettings).toHaveBeenCalledWith(true);

    push(sessionView({ settingsDialog: "Closed", cacheEnabled: true }));
    expect(screen.queryByTestId("settings-dialog")).not.toBeInTheDocument();
    expect(settingsButton).toHaveFocus();
  });

  it("forwards Escape and [Clear cache] of the settings dialog", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ settingsDialog: "Open" }));
    await user.click(screen.getByTestId("settings-dialog-clear-button"));
    expect(api.clearCache).toHaveBeenCalledTimes(1);
    pressEscape();
    expect(api.cancelSettings).toHaveBeenCalledTimes(1);
    expect(api.saveSettings).not.toHaveBeenCalled();
  });

  it("sends the log filter text again after the core refused it", async () => {
    vi.mocked(api.setLogFilter)
      .mockRejectedValueOnce({ key: "session.confirmationPending" })
      .mockResolvedValue(undefined);
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    const input = logFilter();
    fireEvent.change(input, { target: { value: "error" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(api.setLogFilter).toHaveBeenCalledWith("error");
    await waitFor(() => expect(api.setLogFilter).toHaveBeenCalledTimes(2));
    expect(api.setLogFilter).toHaveBeenLastCalledWith("error");
  });

  it("signs in with SSO from the list error: dialog, URL, [Cancel], focus back (261011-sso-login)", async () => {
    const user = userEvent.setup();
    const authError = connectedView({
      logGroups: {
        status: "Partial",
        visibleGroups: [],
        totalCount: 0,
        emptyState: null,
        failure: { kind: "AuthRequired", safeDetail: "", retryable: false },
      },
      ssoLogin: ssoLoginView({ available: true, canStart: true, profileOption: "--profile dev" }),
    });
    vi.mocked(api.getSession).mockResolvedValue(authError);
    render(<App locale="en" />);
    const button = await screen.findByTestId("log-group-pane-sso-login-button");
    await user.click(button);
    expect(api.startSsoLogin).toHaveBeenCalledTimes(1);

    const url = "https://device.sso.us-east-1.amazonaws.com/";
    push({
      ...authError,
      canChangeConnection: false,
      canReload: false,
      ssoLogin: ssoLoginView({
        available: true,
        state: "Waiting",
        attemptId: 1,
        verificationUrl: url,
        userCode: "ABCD-EFGH",
      }),
    });
    expect(screen.getByTestId("sso-login-modal")).toHaveTextContent("ABCD-EFGH");
    await user.click(screen.getByTestId("sso-login-modal-url-link"));
    expect(api.openSsoUrl).toHaveBeenCalledWith(url);
    await user.click(screen.getByTestId("sso-login-modal-cancel-button"));
    expect(api.cancelSsoLogin).toHaveBeenCalledTimes(1);

    push({
      ...authError,
      ssoLogin: ssoLoginView({ available: true, canStart: true, state: "TimedOut", attemptId: 1 }),
    });
    await user.click(screen.getByTestId("sso-login-modal-retry-button"));
    expect(api.startSsoLogin).toHaveBeenCalledTimes(2);

    push({
      ...authError,
      ssoLogin: ssoLoginView({ available: true, canStart: true, state: "Cancelled", attemptId: 1 }),
    });
    expect(screen.queryByTestId("sso-login-modal")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("log-group-pane-sso-login-button")).toHaveFocus(),
    );
  });

  it("shows a success notice at the top of the right pane and puts it away (261011-sso-login FR3.1)", async () => {
    const user = userEvent.setup();
    render(<App locale="ja" />);
    await screen.findByTestId("fetch-form");
    push(
      connectedView({
        ssoLogin: ssoLoginView({ state: "Failed", exitCode: 2, attemptId: 1 }),
      }),
    );
    expect(screen.getByTestId("sso-login-modal-result")).toHaveTextContent(
      "ログインできませんでした（終了コード 2）。",
    );
    await user.click(screen.getByTestId("sso-login-modal-close-button"));
    expect(api.dismissSsoLogin).toHaveBeenCalledTimes(1);

    push(connectedView({ ssoLogin: ssoLoginView({ state: "Succeeded", attemptId: 2 }) }));
    expect(screen.queryByTestId("sso-login-modal")).not.toBeInTheDocument();
    const notice = screen.getByTestId("sso-login-success");
    expect(notice).toHaveTextContent(
      "ログインしました。ロググループの一覧を読み込み直しています。",
    );
    await user.click(within(notice).getByRole("button", { name: "閉じる" }));
    expect(api.dismissSsoLogin).toHaveBeenCalledTimes(2);
    // The fetch is not started again by the screen (FR3.2).
    expect(api.startFetch).not.toHaveBeenCalled();
  });

  it("asks before closing while fetching and forwards [Keep fetching], Escape and [Close]", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", currentJobId: 3, closeConfirmation: "Pending" }));
    const dialog = screen.getByTestId("close-confirm-dialog");
    expect(dialog).toHaveTextContent("Fetching is in progress.");
    expect(dialog).toHaveTextContent(
      "If you close the window, the logs fetched so far will be lost.",
    );
    // Focus on [Keep fetching]: an Enter pressed by mistake loses nothing.
    expect(screen.getByTestId("close-confirm-keep-button")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(api.cancelClose).toHaveBeenCalledTimes(1);
    pressEscape();
    expect(api.cancelClose).toHaveBeenCalledTimes(2);
    expect(api.confirmClose).not.toHaveBeenCalled();
    // The fetch ended meanwhile: the dialog stays while the session asks (BR3.4).
    push(sessionView({ phase: "Done", eventCount: 4, closeConfirmation: "Pending" }));
    await user.click(screen.getByTestId("close-confirm-close-button"));
    expect(api.confirmClose).toHaveBeenCalledTimes(1);
    push(sessionView({ phase: "Done", eventCount: 4, closeConfirmation: "None" }));
    expect(screen.queryByTestId("close-confirm-dialog")).not.toBeInTheDocument();
  });

  it("shows a failed fetch as an error alert above the list and only what happened in the status line", async () => {
    render(<App locale="ja" />);
    await screen.findByTestId("fetch-form");
    push(
      sessionView({
        phase: "Failed",
        eventCount: 3,
        timelineVersion: 2,
        lastJob: {
          jobId: 1,
          status: "Failed",
          eventCount: 3,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: { kind: "Network", safeDetail: "kind=Network", retryable: true },
        },
      }),
    );
    const banner = screen.getByTestId("fetch-error-banner");
    expect(banner).toHaveAttribute("role", "alert");
    expect(screen.getByTestId("fetch-error-what")).toHaveTextContent(
      "AWS に接続できませんでした。",
    );
    expect(screen.getByTestId("fetch-error-next")).toHaveTextContent(
      "接続を確認してから、[取得] でもう一度取得してください。",
    );
    expect(screen.getByTestId("fetch-error-detail")).toHaveTextContent("詳細：kind=Network");
    expect(screen.getByTestId("status-line-error")).toHaveTextContent(
      "AWS に接続できませんでした。",
    );
    expect(screen.getByTestId("status-line")).not.toHaveTextContent("詳細");
    // The rows fetched before the failure stay listed below, and the alert sits above the list.
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(
      banner.compareDocumentPosition(screen.getByTestId("log-table")) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("keeps the status line the last part of the list's column, with or without the error alert (gate fix 18a)", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", eventCount: 3, timelineVersion: 1 }));
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    const column = screen.getByTestId("condition-area");
    const checkColumn = () => {
      // The list and the status line are siblings in the one flex column,
      // the status line last, so nothing below the list can push it away.
      expect(screen.getByTestId("log-table").parentElement).toBe(column);
      expect(screen.getByTestId("status-line").parentElement).toBe(column);
      expect(column.lastElementChild).toBe(screen.getByTestId("status-line"));
      expect(screen.getByTestId("log-table")).toHaveClass("log-table");
      // The column sits in the frame inside the Cloudscape container.
      expect(column.parentElement).toBe(screen.getByTestId("condition-area-frame"));
      expect(screen.getByTestId("fetch-form")).toContainElement(column);
    };
    checkColumn();
    // A failed fetch adds the alert above the list, in the same column.
    push(
      sessionView({
        phase: "Failed",
        eventCount: 3,
        timelineVersion: 1,
        lastJob: {
          jobId: 1,
          status: "Failed",
          eventCount: 3,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: { kind: "Network", safeDetail: "kind=Network", retryable: true },
        },
      }),
    );
    expect(screen.getByTestId("fetch-error-banner").parentElement).toBe(column);
    checkColumn();
  });

  it("closes every expanded row when the session says the logs were discarded", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Done", eventCount: 3, timelineVersion: 1 }));
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    await user.click(screen.getAllByTestId("log-table-row-line")[1]!);
    expect(screen.getByTestId("log-table-expanded")).toHaveTextContent("middle");
    push(sessionView({ phase: "Done", eventCount: 3, timelineVersion: 3, discardGeneration: 1 }));
    await waitFor(() => expect(screen.queryByTestId("log-table-expanded")).not.toBeInTheDocument());
  });

  it("moves through the screen with Tab in the order of mockups screen 1, the pane handle after Reload (14d)", async () => {
    vi.mocked(api.getSession).mockResolvedValue(
      connectedView({
        selectedLogGroupName: "/aws/ecs/web",
        eventCount: 3,
        phase: "Done",
        startInput: { text: "2024/03/01 10:00:00", instant: 1, error: null },
        endInput: { text: "2024/03/01 12:00:00", instant: 2, error: null },
      }),
    );
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("log-group-pane-table");
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    // Tab until the start date (the masked date and time inputs swallow
    // user-event's keyCode-less Tab, so the rest is checked on the DOM order).
    const expected = [
      "top-bar-profile",
      "top-bar-region",
      "top-bar-time-zone",
      "top-bar-settings-button",
      // Gate fix 15b: the pane's close button sits in its heading.
      "log-group-pane-collapse-button",
      "log-group-pane-filter",
      "log-group-pane-table",
      "log-group-pane-reload-button",
      // Gate fix 14d: the handle between the panes is keyboard-resizable.
      "pane-resizer",
      "fetch-form-submit-button",
      "log-filter",
      "fetch-form-start-date",
    ];
    for (const testId of expected) {
      await user.tab();
      const active = document.activeElement as HTMLElement;
      expect(active.closest("[data-testid]")?.getAttribute("data-testid"), testId).toBe(testId);
    }
    const after = (first: HTMLElement, second: HTMLElement) =>
      (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(after(dateInput("start"), timeInput("start"))).toBe(true);
    expect(after(timeInput("start"), dateInput("end"))).toBe(true);
    expect(after(dateInput("end"), timeInput("end"))).toBe(true);
    expect(after(timeInput("end"), screen.getByTestId("log-table"))).toBe(true);
    expect(screen.getByTestId("log-table")).toHaveAttribute("tabindex", "0");
  });

  // ---- gate fix 15b: the log group pane closes to a strip and opens again ----

  it("closes the log group pane to a strip, then opens it again at the same width (15b)", async () => {
    vi.mocked(api.getSession).mockResolvedValue(connectedView());
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("log-group-pane");
    const body = screen.getByTestId("app").querySelector<HTMLElement>(".app-body")!;
    const widthVar = () => body.style.getPropertyValue("--ls-log-group-pane-width");
    expect(widthVar()).toBe("25%");
    // Narrow the pane by three steps, then close it.
    screen.getByRole("separator", { name: "Log group pane width" }).focus();
    await user.keyboard("{ArrowLeft}{ArrowLeft}{ArrowLeft}");
    expect(widthVar()).toBe("22%");
    await user.click(screen.getByRole("button", { name: "Close the log group list" }));
    expect(screen.queryByTestId("log-group-pane")).not.toBeInTheDocument();
    expect(screen.queryByRole("separator")).not.toBeInTheDocument();
    const strip = screen.getByTestId("log-group-pane-collapsed");
    const openButton = screen.getByRole("button", { name: "Open the log group list" });
    expect(strip).toContainElement(openButton);
    // Only the open button is in the strip, and it has the focus.
    expect(strip.querySelectorAll("button, input")).toHaveLength(1);
    expect(openButton).toHaveFocus();
    expect(window.localStorage.getItem("local-sights.logGroupPaneCollapsed")).toBe("true");
    // The rest of the screen is still there.
    expect(screen.getByTestId("fetch-form")).toBeInTheDocument();
    // Open it again: the same width, the focus on the close button.
    await user.click(openButton);
    expect(screen.getByTestId("log-group-pane")).toBeInTheDocument();
    expect(screen.queryByTestId("log-group-pane-collapsed")).not.toBeInTheDocument();
    expect(widthVar()).toBe("22%");
    expect(screen.getByRole("separator", { name: "Log group pane width" })).toHaveAttribute(
      "aria-valuenow",
      "22",
    );
    expect(screen.getByRole("button", { name: "Close the log group list" })).toHaveFocus();
    expect(window.localStorage.getItem("local-sights.logGroupPaneCollapsed")).toBe("false");
  });

  it("starts with the pane closed when it was closed last time (15b)", async () => {
    window.localStorage.setItem("local-sights.logGroupPaneCollapsed", "true");
    window.localStorage.setItem("local-sights.logGroupPaneWidthPercent", "40");
    vi.mocked(api.getSession).mockResolvedValue(connectedView());
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("log-group-pane-collapsed");
    expect(screen.queryByTestId("log-group-pane")).not.toBeInTheDocument();
    expect(screen.queryByRole("separator")).not.toBeInTheDocument();
    // Tab reaches the open button right after the top bar.
    for (const testId of [
      "top-bar-profile",
      "top-bar-region",
      "top-bar-time-zone",
      "top-bar-settings-button",
      "log-group-pane-expand-button",
      "fetch-form-submit-button",
    ]) {
      await user.tab();
      const active = document.activeElement as HTMLElement;
      expect(active.closest("[data-testid]")?.getAttribute("data-testid"), testId).toBe(testId);
    }
    await user.click(screen.getByTestId("log-group-pane-expand-button"));
    expect(screen.getByTestId("log-group-pane")).toBeInTheDocument();
    expect(screen.getByRole("separator", { name: "Log group pane width" })).toHaveAttribute(
      "aria-valuenow",
      "40",
    );
  });

  // ---- 261012-parallel-fetch U5: [Cancel] ----------------------------------

  const cancelledView = (more: Partial<SessionView> = {}) =>
    sessionView({
      phase: "Done",
      currentJobId: 7,
      eventCount: 3,
      timelineVersion: 2,
      lastJob: {
        jobId: 7,
        status: "Cancelled",
        eventCount: 3,
        plannedStreamCount: 20,
        finishedStreamCount: 7,
        failedStreamCount: 0,
        failure: null,
      },
      cancelNotice: { unfinishedStreams: 13 },
      ...more,
    });

  it("sends [Cancel] to the core and shows [Cancelling…] once the core says so (BR5.1)", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", currentJobId: 7, fetchCancelAvailable: true }));
    await user.click(screen.getByTestId("fetch-form-submit-button"));
    expect(api.cancelFetch).toHaveBeenCalledTimes(1);
    expect(api.startFetch).not.toHaveBeenCalled();
    push(sessionView({ phase: "Fetching", currentJobId: 7, cancelling: true }));
    expect(screen.getByTestId("fetch-form-submit-button")).toHaveTextContent("Cancelling…");
  });

  it("makes [Cancel] unavailable when the progress message says the cache is written (BR5.2)", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    push(sessionView({ phase: "Fetching", currentJobId: 7, fetchCancelAvailable: true }));
    expect(screen.getByTestId("fetch-form-submit-button")).toBeEnabled();
    act(() => {
      handlers.progress?.({
        jobId: 7,
        progress: {
          seenStreamCount: 1,
          selectedStreamCount: 1,
          plannedStreamCount: 1,
          finishedStreamCount: 1,
          eventCount: 3,
        },
        eventCount: 3,
        timelineVersion: 2,
        filterSummary: null,
        filterResultVersion: 0,
        cacheSaving: true,
      });
    });
    expect(screen.getByTestId("fetch-form-submit-button")).toBeDisabled();
    expect(screen.getByTestId("fetch-form-submit-button")).toHaveTextContent("Cancel");
  });

  it("shows the cancel notice above the list, in the error area's place, never with the error (BR5.11)", async () => {
    render(<App locale="ja" />);
    await screen.findByTestId("fetch-form");
    push(cancelledView());
    const notice = screen.getByTestId("cancel-notice");
    expect(notice).toHaveTextContent(
      "取得を中断しました。13 本のストリームは読み終えていないため、一部のログが含まれていません。",
    );
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
    expect(screen.getByTestId("status-line-cancelled")).toHaveTextContent("中断");
    await waitFor(() => expect(screen.getAllByTestId("log-table-row")).toHaveLength(3));
    expect(
      notice.compareDocumentPosition(screen.getByTestId("log-table")) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    push(cancelledView({ cancelNotice: null, phase: "Idle", lastJob: null }));
    expect(screen.queryByTestId("cancel-notice")).not.toBeInTheDocument();
  });

  it("keeps the fetched rows filterable after a cancel (NFR5.5)", async () => {
    vi.mocked(api.getSession).mockResolvedValue(cancelledView());
    render(<App locale="en" />);
    await screen.findByTestId("cancel-notice");
    const input = logFilter();
    expect(input).toBeEnabled();
    vi.useFakeTimers();
    try {
      fireEvent.change(input, { target: { value: "middle" } });
      act(() => {
        vi.advanceTimersByTime(300);
      });
      expect(api.setLogFilter).toHaveBeenCalledWith("middle");
    } finally {
      vi.useRealTimers();
    }
  });
});
