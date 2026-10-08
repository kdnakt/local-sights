import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ApiFailure, SessionView } from "../api";
import { StatusLine } from "./StatusLine";
import { createTranslator } from "../i18n/messages";
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
    expect(screen.getByTestId("status-line")).toHaveTextContent(
      "12 events Shown from cache (AWS was not called)",
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
