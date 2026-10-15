import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ApiFailure, ElapsedView, SessionView } from "../api";
import { StatusLine } from "./StatusLine";
import { createTranslator, type Translate } from "../i18n/messages";
import { sessionView, t } from "../test/fixtures";

function renderStatus(session: SessionView | null) {
  const onOpenFailures = vi.fn();
  render(<StatusLine session={session} t={t} onOpenFailures={onOpenFailures} />);
  return onOpenFailures;
}

const throttled: ApiFailure = {
  kind: "Throttled",
  safeDetail: "kind=Throttled; api=GetLogEvents; logStream=s1",
  retryable: true,
};

describe("StatusLine", () => {
  it("prompts for input before the first fetch", () => {
    renderStatus(sessionView());
    expect(screen.getByTestId("status-line-idle")).toHaveTextContent(
      "Enter the conditions and press Fetch.",
    );
  });

  it("shows the streams selected so far while listing", () => {
    renderStatus(
      sessionView({
        phase: "Fetching",
        progress: {
          seenStreamCount: 40,
          selectedStreamCount: 12,
          plannedStreamCount: null,
          finishedStreamCount: 0,
          eventCount: 0,
        },
      }),
    );
    expect(screen.getByTestId("status-line-listing")).toHaveTextContent(
      "Listing streams… 12 streams selected so far",
    );
  });

  it("shows finished and planned streams and the events so far while fetching", () => {
    renderStatus(
      sessionView({
        phase: "Fetching",
        progress: {
          seenStreamCount: 40,
          selectedStreamCount: 12,
          plannedStreamCount: 12,
          finishedStreamCount: 3,
          eventCount: 250,
        },
      }),
    );
    expect(screen.getByTestId("status-line-fetching")).toHaveTextContent(
      "Fetching… 3/12 streams, 250 events so far",
    );
  });

  it("shows the count when done and says explicitly when nothing was found", () => {
    renderStatus(sessionView({ phase: "Done", eventCount: 42 }));
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("42 events");
    expect(screen.queryByTestId("status-line-failures-button")).not.toBeInTheDocument();
    expect(screen.queryByTestId("status-line-error")).not.toBeInTheDocument();
  });

  it("says 0 events when the range has no logs", () => {
    renderStatus(sessionView({ phase: "Done", eventCount: 0 }));
    expect(screen.getByTestId("status-line-count")).toHaveTextContent(
      "0 events: no logs in this range.",
    );
  });

  it("offers the number of failed streams as a button that opens the list", async () => {
    const onOpen = renderStatus(
      sessionView({
        phase: "Done",
        eventCount: 9,
        failedStreams: [
          { logStreamName: "s1", failure: throttled },
          { logStreamName: "s2", failure: throttled },
        ],
      }),
    );
    const button = screen.getByTestId("status-line-failures-button");
    expect(button).toHaveTextContent("Failed in 2 streams");
    await userEvent.click(button);
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("says when the stream listing is incomplete", () => {
    renderStatus(
      sessionView({
        phase: "Done",
        eventCount: 3,
        listingStatus: "Partial",
        listingFailure: { ...throttled, safeDetail: "kind=Throttled; api=DescribeLogStreams" },
      }),
    );
    expect(screen.getByTestId("status-line-listing-partial")).toHaveTextContent(
      "The stream listing is incomplete.",
    );
    expect(screen.getByTestId("status-line-failures-button")).toHaveTextContent(
      "Show failure details",
    );
  });

  it("shows only what happened in words and the count when failed, never the detail", () => {
    renderStatus(
      sessionView({
        phase: "Failed",
        eventCount: 0,
        lastJob: {
          jobId: 1,
          status: "Failed",
          eventCount: 0,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: {
            kind: "AccessDenied",
            safeDetail: "kind=AccessDenied; api=DescribeLogStreams; profile=dev",
            retryable: false,
          },
        },
      }),
    );
    expect(screen.getByTestId("status-line-error")).toHaveTextContent("Access was denied.");
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("0 events");
    // U7:BR2.4: the next action and the detail are in the error area, not here.
    expect(screen.queryByTestId("status-line-detail")).not.toBeInTheDocument();
    const line = screen.getByTestId("status-line");
    expect(line).not.toHaveTextContent("Details:");
    expect(line).not.toHaveTextContent("IAM");
    expect(line).not.toHaveTextContent("Failed:");
  });

  it("shows filtered / all and the filtering notice next to the fetch progress", () => {
    renderStatus(
      sessionView({
        phase: "Fetching",
        progress: {
          seenStreamCount: 2,
          selectedStreamCount: 2,
          plannedStreamCount: 2,
          finishedStreamCount: 1,
          eventCount: 120,
        },
        filterSummary: {
          filterId: 3,
          matchedCount: 7,
          allCount: 120,
          status: "Filtering",
          resultVersion: 9,
        },
      }),
    );
    expect(screen.getByTestId("status-line-fetching")).toBeInTheDocument();
    expect(screen.getByTestId("status-line-filter")).toHaveTextContent("Filtered: 7 of 120 events");
    expect(screen.getByTestId("status-line-filtering")).toHaveTextContent("Filtering…");
  });

  it("says in words when nothing matches and drops the notice when ready", () => {
    renderStatus(
      sessionView({
        phase: "Done",
        eventCount: 50,
        filterSummary: {
          filterId: 1,
          matchedCount: 0,
          allCount: 50,
          status: "Ready",
          resultVersion: 4,
        },
      }),
    );
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("50 events");
    expect(screen.getByTestId("status-line-filter")).toHaveTextContent(
      "Filtered: 0 of 50 events, no logs match.",
    );
    expect(screen.queryByTestId("status-line-filtering")).not.toBeInTheDocument();
  });

  it("shows nothing about the filter without one", () => {
    renderStatus(sessionView({ phase: "Done", eventCount: 5, filterSummary: null }));
    expect(screen.queryByTestId("status-line-filter")).not.toBeInTheDocument();
  });

  it("is a live status region drawn with status indicators, the failures as a link button", async () => {
    const onOpen = renderStatus(
      sessionView({
        phase: "Done",
        eventCount: 9,
        failedStreams: [{ logStreamName: "s1", failure: throttled }],
        progress: null,
      }),
    );
    const line = screen.getByRole("status");
    expect(line).toHaveAttribute("aria-live", "polite");
    const button = screen.getByRole("button", { name: "Failed in 1 streams" });
    await userEvent.keyboard("{Tab}");
    expect(button).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onOpen).toHaveBeenCalledTimes(1);
    // The error of a failed fetch carries an icon with a name, not a color alone (FR3.5).
    renderStatus(
      sessionView({
        phase: "Failed",
        lastJob: {
          jobId: 2,
          status: "Failed",
          eventCount: 0,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: throttled,
        },
      }),
    );
    expect(screen.getAllByRole("img", { name: "Other error" }).length).toBeGreaterThan(0);
  });

  it("says the logs are being saved to the cache while a fetch writes them", () => {
    renderStatus(
      sessionView({
        phase: "Fetching",
        cacheSaving: true,
        progress: {
          seenStreamCount: 2,
          selectedStreamCount: 2,
          plannedStreamCount: 2,
          finishedStreamCount: 2,
          eventCount: 9,
        },
      }),
    );
    expect(screen.getByTestId("status-line-fetching")).toBeInTheDocument();
    expect(screen.getByTestId("status-line-cache-saving")).toHaveTextContent("Saving to cache");
  });

  it("shows a fetch from the cache with its count and no stream numbers", () => {
    renderStatus(
      sessionView({
        phase: "Done",
        eventCount: 12,
        cacheNotices: ["Hit"],
        lastJob: {
          jobId: 1,
          status: "Completed",
          eventCount: 12,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: null,
        },
      }),
    );
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("12 events");
    expect(screen.getByTestId("status-line-cache-Hit")).toHaveTextContent(
      "Shown from cache (AWS was not called)",
    );
    expect(screen.getByTestId("status-line").textContent).not.toMatch(/stream/i);
  });

  it("shows both the read failure and the save failure in words", () => {
    renderStatus(
      sessionView({ phase: "Done", eventCount: 3, cacheNotices: ["ReadFailed", "SaveFailed"] }),
    );
    expect(screen.getByTestId("status-line-cache-ReadFailed")).toHaveTextContent(
      "The cache could not be read, so the logs were fetched again",
    );
    expect(screen.getByTestId("status-line-cache-SaveFailed")).toHaveTextContent(
      "Could not save to the cache",
    );
    expect(screen.queryByTestId("status-line-cache-saving")).not.toBeInTheDocument();
  });

  it("shows no cache notice when there is none, also in Japanese", () => {
    const { unmount } = render(
      <StatusLine
        session={sessionView({ phase: "Done", eventCount: 3, cacheNotices: [] })}
        t={t}
        onOpenFailures={vi.fn()}
      />,
    );
    expect(screen.getByTestId("status-line")).toHaveTextContent("3 events");
    expect(screen.getByTestId("status-line").textContent).not.toMatch(/cache/i);
    unmount();
    render(
      <StatusLine
        session={sessionView({ phase: "Done", eventCount: 3, cacheNotices: ["Hit"] })}
        t={createTranslator("ja")}
        onOpenFailures={vi.fn()}
      />,
    );
    expect(screen.getByTestId("status-line-cache-Hit")).toHaveTextContent(
      "キャッシュから表示（AWS は呼んでいない）",
    );
  });
});

describe("StatusLine elapsed time (261012-parallel-fetch U4)", () => {
  const fetchingProgress = {
    seenStreamCount: 12,
    selectedStreamCount: 12,
    plannedStreamCount: 12,
    finishedStreamCount: 3,
    eventCount: 250,
  };
  const running = (elapsedMs: number): ElapsedView => ({
    running: true,
    elapsedMs,
    fetchNumber: 1,
  });
  const stopped = (elapsedMs: number): ElapsedView => ({
    running: false,
    elapsedMs,
    fetchNumber: 1,
  });

  /** The status-line parts in the order they are drawn. */
  function partOrder() {
    return [...screen.getByTestId("status-line").querySelectorAll("[data-testid]")]
      .map((element) => element.getAttribute("data-testid"))
      .filter((id) => id !== null && /^status-line-(?!what$)/.test(id));
  }

  afterEach(() => {
    vi.useRealTimers();
  });

  it.each<[string, Partial<SessionView>]>([
    ["listing", { phase: "Fetching", elapsed: running(3_000) }],
    ["fetching", { phase: "Fetching", progress: fetchingProgress, elapsed: running(7_000) }],
    [
      "saving to the cache",
      {
        phase: "Fetching",
        progress: fetchingProgress,
        cacheSaving: true,
        elapsed: running(30_000),
      },
    ],
    ["done", { phase: "Done", eventCount: 42, elapsed: stopped(31_400) }],
    ["zero events", { phase: "Done", eventCount: 0, elapsed: stopped(2_000) }],
    [
      "some streams failed",
      {
        phase: "Done",
        eventCount: 9,
        failedStreams: [{ logStreamName: "s1", failure: throttled }],
        elapsed: stopped(31_000),
      },
    ],
    [
      "failed",
      {
        phase: "Failed",
        lastJob: {
          jobId: 1,
          status: "Failed",
          eventCount: 0,
          plannedStreamCount: 0,
          finishedStreamCount: 0,
          failedStreamCount: 0,
          failure: throttled,
        },
        elapsed: stopped(5_000),
      },
    ],
    [
      "shown from the cache",
      { phase: "Done", eventCount: 12, cacheNotices: ["Hit"], elapsed: stopped(0) },
    ],
  ])("puts the elapsed time after the fetch parts while %s (BR4.8)", (_, overrides) => {
    renderStatus(sessionView(overrides));
    const order = partOrder();
    expect(order.at(-1)).toBe("status-line-elapsed");
    expect(order.length).toBeGreaterThan(1);
  });

  it("writes the elapsed time in English and Japanese (NFR9.1, AC2.1.6)", () => {
    const { unmount } = render(
      <StatusLine
        session={sessionView({ phase: "Done", eventCount: 3, elapsed: stopped(75 * 60_000) })}
        t={t}
        onOpenFailures={vi.fn()}
      />,
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 1:15:00");
    unmount();
    render(
      <StatusLine
        session={sessionView({ phase: "Done", eventCount: 3, elapsed: stopped(31_400) })}
        t={createTranslator("ja")}
        onOpenFailures={vi.fn()}
      />,
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("経過 00:31");
  });

  it("shows no elapsed time while Idle, without a session and without a value", () => {
    renderStatus(sessionView({ phase: "Idle", elapsed: stopped(4_000) }));
    expect(screen.queryByTestId("status-line-elapsed")).not.toBeInTheDocument();
    cleanup();
    renderStatus(null);
    expect(screen.queryByTestId("status-line-elapsed")).not.toBeInTheDocument();
    cleanup();
    renderStatus(sessionView({ phase: "Done", eventCount: 1, elapsed: null }));
    expect(screen.queryByTestId("status-line-elapsed")).not.toBeInTheDocument();
  });

  it("puts the elapsed time before the log filter", () => {
    renderStatus(
      sessionView({
        phase: "Fetching",
        progress: fetchingProgress,
        cacheSaving: true,
        elapsed: running(1_000),
        filterSummary: {
          filterId: 1,
          matchedCount: 2,
          allCount: 250,
          status: "Filtering",
          resultVersion: 3,
        },
      }),
    );
    expect(partOrder()).toEqual([
      "status-line-fetching",
      "status-line-cache-saving",
      "status-line-elapsed",
      "status-line-filter",
      "status-line-filtering",
    ]);
  });

  it("is read out with the end of the fetch, in the same update (NFR9.2, BR4.9)", () => {
    const onOpenFailures = vi.fn();
    const { rerender } = render(
      <StatusLine
        session={sessionView({
          phase: "Fetching",
          progress: fetchingProgress,
          elapsed: running(30_000),
        })}
        t={t}
        onOpenFailures={onOpenFailures}
      />,
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveAttribute("aria-hidden", "true");
    rerender(
      <StatusLine
        session={sessionView({ phase: "Done", eventCount: 42, elapsed: stopped(31_400) })}
        t={t}
        onOpenFailures={onOpenFailures}
      />,
    );
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("42 events");
    const elapsed = screen.getByTestId("status-line-elapsed");
    expect(elapsed).not.toHaveAttribute("aria-hidden");
    expect(elapsed).toHaveTextContent("Elapsed 00:31");
    expect(screen.getByRole("status")).toHaveTextContent("42 events");
  });

  it("does not go back when a progress message is composed into the view (NFR6.9)", () => {
    vi.useFakeTimers();
    const view = sessionView({
      phase: "Fetching",
      currentJobId: 7,
      progress: fetchingProgress,
      elapsed: running(10_000),
    });
    const onOpenFailures = vi.fn();
    const { rerender } = render(
      <StatusLine session={view} t={t} onOpenFailures={onOpenFailures} />,
    );
    for (let step = 0; step < 12; step += 1) {
      act(() => {
        vi.advanceTimersByTime(250);
      });
    }
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 00:13");
    // As `withProgress` does: a new view object keeping the same `elapsed`.
    rerender(
      <StatusLine
        session={{ ...view, progress: { ...fetchingProgress, finishedStreamCount: 4 } }}
        t={t}
        onOpenFailures={onOpenFailures}
      />,
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 00:13");
    expect(screen.getByTestId("status-line-fetching")).toHaveTextContent("4/12 streams");
  });

  it("redraws only the elapsed part every second (NFR5.3)", () => {
    vi.useFakeTimers();
    const translate = vi.fn<Translate>((key, params) => t(key, params));
    render(
      <StatusLine
        session={sessionView({
          phase: "Fetching",
          progress: fetchingProgress,
          elapsed: running(0),
        })}
        t={translate}
        onOpenFailures={vi.fn()}
      />,
    );
    const drawsOf = (key: string) => translate.mock.calls.filter(([k]) => k === key).length;
    const lineDraws = drawsOf("status.fetching");
    const elapsedDraws = drawsOf("status.elapsed");
    for (let step = 0; step < 12; step += 1) {
      act(() => {
        vi.advanceTimersByTime(250);
      });
    }
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 00:03");
    expect(drawsOf("status.fetching")).toBe(lineDraws);
    expect(drawsOf("status.elapsed")).toBeGreaterThanOrEqual(elapsedDraws + 3);
  });
});

describe("StatusLine after a cancelled fetch (261012-parallel-fetch U5:BR5.12)", () => {
  const cancelled = (eventCount: number, more: Partial<SessionView> = {}) =>
    sessionView({
      phase: "Done",
      eventCount,
      lastJob: {
        jobId: 4,
        status: "Cancelled",
        eventCount,
        plannedStreamCount: 20,
        finishedStreamCount: 7,
        failedStreamCount: 0,
        failure: null,
      },
      cancelNotice: { unfinishedStreams: 13 },
      elapsed: { running: false, elapsedMs: 12_000, fetchNumber: 1 },
      ...more,
    });

  it("says Cancelled with the count, the failed streams and the elapsed time", () => {
    renderStatus(
      cancelled(120, {
        failedStreams: [{ logStreamName: "s1", failure: throttled }],
      }),
    );
    expect(screen.getByTestId("status-line-cancelled")).toHaveTextContent("Cancelled");
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("120 events");
    expect(screen.getByTestId("status-line-failures-button")).toHaveTextContent(
      "Failed in 1 streams",
    );
    expect(screen.getByTestId("status-line-elapsed")).toHaveTextContent("Elapsed 00:12");
    const parts = [...screen.getByTestId("status-line").querySelectorAll("[data-testid]")].map(
      (element) => element.getAttribute("data-testid"),
    );
    expect(parts.indexOf("status-line-cancelled")).toBeLessThan(parts.indexOf("status-line-count"));
    expect(parts.indexOf("status-line-count")).toBeLessThan(parts.indexOf("status-line-elapsed"));
  });

  it("says just 0 events, never that the range has no logs", () => {
    renderStatus(cancelled(0));
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("0 events");
    expect(screen.getByTestId("status-line")).not.toHaveTextContent("no logs in this range");
  });

  it("shows no cache notice", () => {
    renderStatus(cancelled(5, { cacheNotices: ["ReadFailed"] }));
    expect(screen.queryByTestId("status-line-cache-ReadFailed")).not.toBeInTheDocument();
  });

  it("says 中断 and 0 件 in Japanese", () => {
    render(
      <StatusLine session={cancelled(0)} t={createTranslator("ja")} onOpenFailures={vi.fn()} />,
    );
    expect(screen.getByTestId("status-line-cancelled")).toHaveTextContent("中断");
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("0 件");
    expect(screen.getByTestId("status-line")).not.toHaveTextContent("この範囲にログはありません");
  });
});
