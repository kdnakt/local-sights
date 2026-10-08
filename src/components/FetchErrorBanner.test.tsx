import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { JobSummary } from "../api";
import { sessionView, t } from "../test/fixtures";
import { FetchErrorBanner } from "./FetchErrorBanner";

const failedJob: JobSummary = {
  jobId: 1,
  status: "Failed",
  eventCount: 0,
  plannedStreamCount: 0,
  finishedStreamCount: 0,
  failedStreamCount: 0,
  failure: { kind: "RegionMissing", safeDetail: "profile=prod", retryable: false },
};

describe("FetchErrorBanner", () => {
  it("shows the failure of the whole fetch in words as an alert", () => {
    render(
      <FetchErrorBanner session={sessionView({ phase: "Failed", lastJob: failedJob })} t={t} />,
    );
    const banner = screen.getByTestId("fetch-error-banner");
    expect(banner).toHaveAttribute("role", "alert");
    expect(screen.getByTestId("fetch-error-what")).toHaveTextContent(
      "This profile has no default region.",
    );
    expect(screen.getByTestId("fetch-error-next")).toHaveTextContent(
      "Choose a region in the top bar.",
    );
    expect(screen.getByTestId("fetch-error-detail")).toHaveTextContent("Details: profile=prod");
  });

  it("shows nothing unless the fetch failed as a whole", () => {
    const { rerender } = render(<FetchErrorBanner session={null} t={t} />);
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
    rerender(
      <FetchErrorBanner session={sessionView({ phase: "Done", lastJob: failedJob })} t={t} />,
    );
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
    rerender(
      <FetchErrorBanner
        session={sessionView({ phase: "Failed", lastJob: { ...failedJob, failure: null } })}
        t={t}
      />,
    );
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
  });
});
