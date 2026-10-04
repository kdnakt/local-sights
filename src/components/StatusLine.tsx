import type { SessionView } from "../api";
import type { Translate } from "../i18n/messages";

export interface StatusLineProps {
  session: SessionView | null;
  /** Events received so far in the running fetch. */
  liveCount: number;
  t: Translate;
}

/**
 * One status line: fetching (BR1.4), the event count including the explicit
 * zero case (BR5.4), and on failure the kind name and the safe detail as is
 * (BR4.4, provisional wording until U7).
 */
export function StatusLine({ session, liveCount, t }: StatusLineProps) {
  return (
    <div className="status-line" role="status" aria-live="polite" data-testid="status-line">
      {renderStatus(session, liveCount, t)}
    </div>
  );
}

function renderStatus(session: SessionView | null, liveCount: number, t: Translate) {
  if (session === null || session.phase === "Idle") {
    return <span data-testid="status-line-idle">{t("status.idle")}</span>;
  }
  if (session.phase === "Fetching") {
    return (
      <span data-testid="status-line-fetching">{t("status.fetching", { count: liveCount })}</span>
    );
  }
  const count = (
    <span data-testid="status-line-count">
      {session.eventCount === 0
        ? t("status.zero")
        : t("status.count", { count: session.eventCount })}
    </span>
  );
  const failure = session.phase === "Failed" ? session.lastJob?.failure : null;
  if (!failure) {
    return count;
  }
  return (
    <>
      <span className="status-line-error" data-testid="status-line-error">
        {t("status.failed", { kind: t(`failure.kind.${failure.kind}`) })}
      </span>{" "}
      {count}{" "}
      <span className="status-line-detail" data-testid="status-line-detail">
        {t("status.detail", { detail: failure.safeDetail })}
      </span>
    </>
  );
}
