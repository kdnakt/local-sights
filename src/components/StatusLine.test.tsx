import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ApiFailure, SessionView } from "../api";
import { StatusLine } from "./StatusLine";
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

  it("shows the failure kind name, the count and the safe detail when failed", () => {
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
    expect(screen.getByTestId("status-line-error")).toHaveTextContent("Failed: Access denied");
    expect(screen.getByTestId("status-line-detail")).toHaveTextContent(
      "Details: kind=AccessDenied; api=DescribeLogStreams; profile=dev",
    );
  });
});
