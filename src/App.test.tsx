import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LogBatch, SessionView } from "./api";
import { App } from "./App";
import { logEvent, sessionView } from "./test/fixtures";

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
    vi.mocked(api.startFetch).mockResolvedValue(
      sessionView({ phase: "Fetching", canFetch: false }),
    );
    vi.mocked(api.updateInput).mockResolvedValue(sessionView());
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
      handlers.session?.(sessionView({ phase: "Fetching", currentJobId: null }));
    });
    expect(screen.queryAllByTestId("log-table-row")).toHaveLength(0);
    act(() => {
      handlers.batch?.({ jobId: "job-2", events: [logEvent(0, "new")], total: 1 });
    });
    expect(screen.getAllByTestId("log-table-row")).toHaveLength(1);
    expect(screen.getByText("new")).toBeInTheDocument();
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
});
