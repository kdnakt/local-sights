import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { JobSummary } from "../api";
import { createTranslator } from "../i18n/messages";
import { sessionView, ssoLoginView, t } from "../test/fixtures";
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

describe("FetchErrorBanner (FR3.2, mockups screen 5b)", () => {
  it("shows the failure of the whole fetch as an error alert: what happened, what to do, the detail", () => {
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
    // Not by color alone (FR3.5): the error icon carries a name.
    expect(screen.getByRole("img", { name: "Other error" })).toBeInTheDocument();
  });

  it("leaves out the detail line when there is none, in Japanese too", () => {
    render(
      <FetchErrorBanner
        session={sessionView({
          phase: "Failed",
          lastJob: { ...failedJob, failure: { kind: "Network", safeDetail: "", retryable: true } },
        })}
        t={createTranslator("ja")}
      />,
    );
    expect(screen.getByTestId("fetch-error-what")).toHaveTextContent(
      "AWS に接続できませんでした。",
    );
    expect(screen.getByTestId("fetch-error-next")).toHaveTextContent(
      "接続を確認してから、[取得] でもう一度取得してください。",
    );
    expect(screen.queryByTestId("fetch-error-detail")).not.toBeInTheDocument();
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

  it("regression (FR3.2): the three-line alert closes with its button and comes back for the next failure", async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <FetchErrorBanner session={sessionView({ phase: "Failed", lastJob: failedJob })} t={t} />,
    );
    const banner = screen.getByTestId("fetch-error-banner");
    expect(banner).toHaveTextContent("This profile has no default region.");
    expect(banner).toHaveTextContent("Choose a region in the top bar.");
    expect(banner).toHaveTextContent("Details: profile=prod");
    await user.click(screen.getByRole("button", { name: "Dismiss (Esc)" }));
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
    // The same failed job stays closed; another failed fetch is shown again.
    rerender(
      <FetchErrorBanner session={sessionView({ phase: "Failed", lastJob: failedJob })} t={t} />,
    );
    expect(screen.queryByTestId("fetch-error-banner")).not.toBeInTheDocument();
    rerender(
      <FetchErrorBanner
        session={sessionView({ phase: "Failed", lastJob: { ...failedJob, jobId: 2 } })}
        t={t}
      />,
    );
    expect(screen.getByTestId("fetch-error-banner")).toBeInTheDocument();
  });

  it("carries [Sign in with SSO] in an auth error when the login is available (261011-sso-login FR1.1)", async () => {
    const user = userEvent.setup();
    const onSsoLogin = vi.fn();
    const authJob: JobSummary = {
      ...failedJob,
      failure: { kind: "AuthRequired", safeDetail: "profile=dev", retryable: false },
    };
    const { rerender } = render(
      <FetchErrorBanner
        session={sessionView({
          phase: "Failed",
          lastJob: authJob,
          ssoLogin: ssoLoginView({ available: true, canStart: true }),
        })}
        t={t}
        onSsoLogin={onSsoLogin}
      />,
    );
    await user.click(screen.getByTestId("fetch-error-sso-login-button"));
    expect(onSsoLogin).toHaveBeenCalledTimes(1);
    // Another kind of failure, or a profile that cannot sign in: no button.
    rerender(
      <FetchErrorBanner
        session={sessionView({
          phase: "Failed",
          lastJob: { ...failedJob, jobId: 2 },
          ssoLogin: ssoLoginView({ available: true, canStart: true }),
        })}
        t={t}
        onSsoLogin={onSsoLogin}
      />,
    );
    expect(screen.queryByTestId("fetch-error-sso-login-button")).not.toBeInTheDocument();
    rerender(
      <FetchErrorBanner
        session={sessionView({ phase: "Failed", lastJob: { ...authJob, jobId: 3 } })}
        t={t}
        onSsoLogin={onSsoLogin}
      />,
    );
    expect(screen.getByTestId("fetch-error-banner")).toBeInTheDocument();
    expect(screen.queryByTestId("fetch-error-sso-login-button")).not.toBeInTheDocument();
  });

  it("does not close with Escape (R-06: the close button only)", async () => {
    const user = userEvent.setup();
    render(
      <FetchErrorBanner session={sessionView({ phase: "Failed", lastJob: failedJob })} t={t} />,
    );
    await user.keyboard("{Escape}");
    expect(screen.getByTestId("fetch-error-banner")).toBeInTheDocument();
  });
});
