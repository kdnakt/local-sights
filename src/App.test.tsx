import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LogBatch, SessionView } from "./api";
import { App } from "./App";
import { connectedView, logEvent, sessionView } from "./test/fixtures";

const handlers: {
  session?: (view: SessionView) => void;
  batch?: (batch: LogBatch) => void;
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
    onSessionChanged: vi.fn(async (handler: (view: SessionView) => void) => {
      handlers.session = handler;
      return () => undefined;
    }),
    onLogBatch: vi.fn(async (handler: (batch: LogBatch) => void) => {
      handlers.batch = handler;
      return () => undefined;
    }),
  };
});

const api = await import("./api");

describe("App", () => {
  beforeEach(() => {
    vi.mocked(api.getSession).mockResolvedValue(sessionView());
    vi.mocked(api.startFetch).mockResolvedValue(undefined);
    vi.mocked(api.updateInput).mockResolvedValue(sessionView());
    for (const command of [
      api.selectProfile,
      api.selectRegion,
      api.confirmConnectionChange,
      api.cancelConnectionChange,
      api.reloadLogGroups,
      api.updateLogGroupFilter,
      api.selectLogGroup,
    ]) {
      vi.mocked(command).mockResolvedValue(undefined);
    }
  });

  it("appends batches of the current job and starts over for a new job", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    act(() => {
      handlers.session?.(sessionView({ phase: "Fetching", currentJobId: "job-1" }));
      handlers.batch?.({ jobId: "job-1", events: [logEvent(0), logEvent(1)], total: 2 });
      handlers.batch?.({ jobId: "job-1", events: [logEvent(2)], total: 3 });
    });
    expect(screen.getAllByTestId("log-table-row")).toHaveLength(3);

    act(() => {
      handlers.session?.(sessionView({ phase: "Fetching", currentJobId: "job-2" }));
    });
    expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0);
    act(() => {
      handlers.batch?.({ jobId: "job-2", events: [logEvent(0, "new")], total: 1 });
    });
    expect(screen.getAllByTestId("log-table-row")).toHaveLength(1);
    expect(screen.getByText("new")).toBeInTheDocument();
  });

  it("keeps the job's rows when events arrive before the start_fetch response", async () => {
    let resolveStart: () => void = () => undefined;
    vi.mocked(api.startFetch).mockReturnValue(
      new Promise<void>((resolve) => {
        resolveStart = resolve;
      }),
    );
    vi.mocked(api.getSession).mockResolvedValue(sessionView({ canFetch: true }));
    const user = userEvent.setup();
    render(<App locale="en" />);
    await user.click(await screen.findByTestId("fetch-form-submit-button"));

    act(() => {
      handlers.session?.(sessionView({ phase: "Fetching", canFetch: false, currentJobId: null }));
      handlers.session?.(
        sessionView({ phase: "Fetching", canFetch: false, currentJobId: "job-x" }),
      );
      handlers.batch?.({ jobId: "job-x", events: [logEvent(0), logEvent(1)], total: 2 });
      handlers.session?.(sessionView({ phase: "Done", currentJobId: "job-x", eventCount: 2 }));
    });
    await act(async () => {
      resolveStart();
    });

    expect(screen.getAllByTestId("log-table-row")).toHaveLength(2);
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("2 events");
  });

  it("does not discard rows on a view without a job ID", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    act(() => {
      handlers.batch?.({ jobId: "job-1", events: [logEvent(0)], total: 1 });
      handlers.session?.(sessionView({ phase: "Fetching", currentJobId: null }));
    });
    expect(screen.getAllByTestId("log-table-row")).toHaveLength(1);
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
    expect(screen.getByRole("columnheader", { name: "時刻（UTC）" })).toBeInTheDocument();
  });
  it("asks before a connection change and forwards the answer", async () => {
    const user = userEvent.setup();
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    act(() => {
      handlers.session?.(
        connectedView({
          pendingChange: { proposedProfile: null, proposedRegion: "eu-west-1" },
          canChangeConnection: false,
        }),
      );
    });
    expect(screen.getByTestId("confirm-dialog-cancel-button")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(api.cancelConnectionChange).toHaveBeenCalledTimes(1);
    await user.click(screen.getByTestId("confirm-dialog-change-button"));
    expect(api.confirmConnectionChange).toHaveBeenCalledTimes(1);
    act(() => {
      handlers.session?.(connectedView());
    });
    expect(screen.queryByTestId("confirm-dialog")).not.toBeInTheDocument();
  });

  it("drops the shown rows when a connection change discards the logs", async () => {
    render(<App locale="en" />);
    await screen.findByTestId("fetch-form");
    act(() => {
      handlers.session?.(connectedView({ phase: "Fetching", currentJobId: "job-1" }));
      handlers.batch?.({ jobId: "job-1", events: [logEvent(0), logEvent(1)], total: 2 });
      handlers.session?.(connectedView({ phase: "Done", currentJobId: "job-1", eventCount: 2 }));
    });
    expect(screen.getAllByTestId("log-table-row")).toHaveLength(2);
    act(() => {
      handlers.session?.(
        connectedView({ phase: "Idle", currentJobId: null, timelineGeneration: 1 }),
      );
    });
    expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0);
  });

  it("forwards profile, region, filter, reload and log group choices", async () => {
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
    await user.click(screen.getByTestId("log-group-pane-reload-button"));
    expect(api.reloadLogGroups).toHaveBeenCalledTimes(1);
    await user.click(screen.getAllByTestId("log-group-pane-option")[0]!);
    expect(api.selectLogGroup).toHaveBeenCalledWith("/aws/ecs/web");
  });
});
