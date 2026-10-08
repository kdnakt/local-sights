import type { SessionView } from "../api";
import type { Translate } from "../i18n/messages";
import { ErrorMessage } from "./ErrorMessage";

export interface FetchErrorBannerProps {
  session: SessionView | null;
  t: Translate;
}

/**
 * The error area above the log list (U7:BR2.4): shown only when the whole
 * fetch failed (U3 Failed), with what happened, what to do next and the
 * safe detail, announced to screen readers. Rows fetched before the failure
 * stay listed below it.
 */
export function FetchErrorBanner({ session, t }: FetchErrorBannerProps) {
  const failure = session?.phase === "Failed" ? session.lastJob?.failure : null;
  if (!failure) {
    return null;
  }
  return (
    <div className="fetch-error-banner" role="alert" data-testid="fetch-error-banner">
      <ErrorMessage failure={failure} scene="Fetch" t={t} testId="fetch-error" />
    </div>
  );
}
