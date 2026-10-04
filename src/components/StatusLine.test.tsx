import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusLine } from "./StatusLine";
import { sessionView, t } from "../test/fixtures";

describe("StatusLine", () => {
  it("prompts for input before the first fetch", () => {
    render(<StatusLine session={sessionView()} liveCount={0} t={t} />);
    expect(screen.getByTestId("status-line-idle")).toHaveTextContent(
      "Enter the conditions and press Fetch.",
    );
  });

  it("shows that a fetch is running with the live count", () => {
    render(<StatusLine session={sessionView({ phase: "Fetching" })} liveCount={250} t={t} />);
    expect(screen.getByTestId("status-line-fetching")).toHaveTextContent(
      "Fetching… 250 events so far",
    );
  });

  it("shows the event count when done", () => {
    render(
      <StatusLine session={sessionView({ phase: "Done", eventCount: 42 })} liveCount={42} t={t} />,
    );
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("42 events");
    expect(screen.queryByTestId("status-line-error")).not.toBeInTheDocument();
  });

  it("says explicitly when nothing was found", () => {
    render(
      <StatusLine session={sessionView({ phase: "Done", eventCount: 0 })} liveCount={0} t={t} />,
    );
    expect(screen.getByTestId("status-line-count")).toHaveTextContent(
      "0 events: no logs in this range.",
    );
  });

  it("shows the failure kind name, the count so far and the safe detail", () => {
    const session = sessionView({
      phase: "Failed",
      eventCount: 3,
      lastJob: {
        jobId: "job-1",
        status: "Failed",
        eventCount: 3,
        pageCount: 1,
        failure: {
          kind: "AccessDenied",
          safeDetail: "kind=AccessDenied; api=GetLogEvents; profile=dev",
          retryable: false,
        },
      },
    });
    render(<StatusLine session={session} liveCount={3} t={t} />);
    expect(screen.getByTestId("status-line-error")).toHaveTextContent("Failed: Access denied");
    expect(screen.getByTestId("status-line-count")).toHaveTextContent("3 events");
    expect(screen.getByTestId("status-line-detail")).toHaveTextContent(
      "Details: kind=AccessDenied; api=GetLogEvents; profile=dev",
    );
  });
});
