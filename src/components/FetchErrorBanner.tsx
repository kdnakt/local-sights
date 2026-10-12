import { useState } from "react";
import Alert from "@cloudscape-design/components/alert";
import Box from "@cloudscape-design/components/box";
import type { SessionView } from "../api";
import { errorLines } from "../errorText";
import type { Translate } from "../i18n/messages";
import { SsoLoginButton, type SsoLoginButtonProps } from "./SsoLoginButton";

export interface FetchErrorBannerProps {
  session: SessionView | null;
  t: Translate;
  /**
   * [Sign in with SSO] in an auth error of the fetch (261011-sso-login
   * FR1.1); without it no such button is shown.
   */
  onSsoLogin?: SsoLoginButtonProps["onStart"];
}

/**
 * The error area above the log list (U7:BR2.4, FR3.2, mockups screen 5b):
 * shown only when the whole fetch failed (U3 Failed), as a console-style
 * error alert with what happened as its heading, what to do next and the
 * safe detail as its lines, announced to screen readers. Its close button
 * hides it until the next failed fetch (another job); Escape is not used
 * (R-06). Rows fetched before the failure stay listed below it. Since
 * 261011-sso-login an auth error carries [Sign in with SSO] below its lines
 * when the core says the selected profile can sign in (FR1.1, BR1.5).
 */
export function FetchErrorBanner({ session, t, onSsoLogin }: FetchErrorBannerProps) {
  const [dismissedJobId, setDismissedJobId] = useState<number | null>(null);
  const job = session?.phase === "Failed" ? session.lastJob : null;
  const failure = job?.failure ?? null;
  if (job === null || failure === null || job.jobId === dismissedJobId) {
    return null;
  }
  const lines = errorLines(failure.kind, "Fetch", failure.safeDetail, t);
  return (
    <div role="alert" data-testid="fetch-error-banner">
      <Alert
        type="error"
        dismissible
        dismissAriaLabel={t("error.dismiss")}
        statusIconAriaLabel={t("failure.kind.Other")}
        header={<span data-testid="fetch-error-what">{lines.what}</span>}
        onDismiss={() => setDismissedJobId(job.jobId)}
      >
        <div data-testid="fetch-error-next">{lines.next}</div>
        {lines.detail !== null && <div data-testid="fetch-error-detail">{lines.detail}</div>}
        {failure.kind === "AuthRequired" && onSsoLogin && session && (
          <Box margin={{ top: "xs" }}>
            <SsoLoginButton
              session={session}
              t={t}
              onStart={onSsoLogin}
              testId="fetch-error-sso-login-button"
            />
          </Box>
        )}
      </Alert>
    </div>
  );
}
