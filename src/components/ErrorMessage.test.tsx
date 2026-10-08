import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ApiFailure } from "../api";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { ErrorMessage } from "./ErrorMessage";

const auth: ApiFailure = {
  kind: "AuthRequired",
  safeDetail: "kind=AuthRequired; profile=dev",
  retryable: false,
};

describe("ErrorMessage", () => {
  it("shows what happened, what to do next and the safe detail on three lines", () => {
    render(<ErrorMessage failure={auth} scene="Fetch" t={t} />);
    expect(screen.getByTestId("error-message-what")).toHaveTextContent(
      "Credentials could not be used (for example, the SSO session expired).",
    );
    expect(screen.getByTestId("error-message-next")).toHaveTextContent(
      "If you use SSO, run aws sso login, then press [Fetch] to fetch again. Otherwise, check the profile settings or choose another profile.",
    );
    expect(screen.getByTestId("error-message-detail")).toHaveTextContent(
      "Details: kind=AuthRequired; profile=dev",
    );
    expect(screen.getByTestId("error-message").querySelectorAll("p")).toHaveLength(3);
  });

  it("leaves out the detail line when there is no safe detail", () => {
    render(<ErrorMessage failure={{ ...auth, safeDetail: "" }} scene="Listing" t={t} />);
    expect(screen.queryByTestId("error-message-detail")).not.toBeInTheDocument();
    expect(screen.getByTestId("error-message-next")).toHaveTextContent(
      "then press [Reload] to load the list again.",
    );
  });

  it("keeps only the first sentence when compact, in Japanese too", () => {
    render(
      <ErrorMessage
        failure={{ kind: "Throttled", safeDetail: "x", retryable: true }}
        scene="Stream"
        t={createTranslator("ja")}
        compact
        testId="compact"
      />,
    );
    expect(screen.getByTestId("compact-what")).toHaveTextContent(
      "AWS の呼び出しが多すぎるため、制限されました。",
    );
    expect(screen.queryByTestId("compact-next")).not.toBeInTheDocument();
    expect(screen.queryByText(/詳細/)).not.toBeInTheDocument();
  });

  it("shows words for a kind that cannot happen in its scene, never a bare kind name", () => {
    render(
      <ErrorMessage
        failure={{ kind: "RegionMissing", safeDetail: "", retryable: false }}
        scene="Stream"
        t={t}
      />,
    );
    expect(screen.getByTestId("error-message-what")).toHaveTextContent(
      "Something went wrong while calling AWS.",
    );
    expect(screen.getByTestId("error-message")).not.toHaveTextContent("RegionMissing");
  });
});
