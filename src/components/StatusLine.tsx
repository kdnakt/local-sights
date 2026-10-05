import type { SessionView } from "../api";
import type { Translate } from "../i18n/messages";

export interface StatusLineProps {
  session: SessionView | null;
  t: Translate;
  /** Opens the list of failed streams (U3:BR6.3). */
  onOpenFailures: () => void;
}

/**
 * One status line (U3:BR6.2). While listing: the streams selected so far.
 * While fetching: finished / planned streams and the events so far. After a
 * fetch: the event count including the explicit zero case (U1:BR5.4), a
 * button with the number of failed streams, the notice that the stream
 * listing is incomplete, and on failure the kind name and the safe detail
 * (U1:BR4.4, provisional wording until U7).
 */
export function StatusLine({ session, t, onOpenFailures }: StatusLineProps) {
  return (
    <div className="status-line" role="status" aria-live="polite" data-testid="status-line">
      {renderStatus(session, t, onOpenFailures)}
    </div>
  );
}

function renderStatus(session: SessionView | null, t: Translate, onOpenFailures: () => void) {
  if (session === null || session.phase === "Idle") {
    return <span data-testid="status-line-idle">{t("status.idle")}</span>;
  }
  if (session.phase === "Fetching") {
    const progress = session.progress;
    if (progress === null || progress.plannedStreamCount === null) {
      return (
        <span data-testid="status-line-listing">
          {t("status.listing", { count: progress?.selectedStreamCount ?? 0 })}
        </span>
      );
    }
    return (
      <span data-testid="status-line-fetching">
        {t("status.fetching", {
          finished: progress.finishedStreamCount,
          planned: progress.plannedStreamCount,
          count: progress.eventCount,
        })}
      </span>
    );
  }
  const failure = session.phase === "Failed" ? session.lastJob?.failure : null;
  const failedCount = session.failedStreams.length;
  const hasFailures = failedCount > 0 || session.listingFailure !== null;
  return (
    <>
      {failure && (
        <>
          <span className="status-line-error" data-testid="status-line-error">
            {t("status.failed", { kind: t(`failure.kind.${failure.kind}`) })}
          </span>{" "}
        </>
      )}
      <span data-testid="status-line-count">
        {session.eventCount === 0
          ? t("status.zero")
          : t("status.count", { count: session.eventCount })}
      </span>
      {failure && (
        <>
          {" "}
          <span className="status-line-detail" data-testid="status-line-detail">
            {t("status.detail", { detail: failure.safeDetail })}
          </span>
        </>
      )}
      {session.phase === "Done" && session.listingStatus === "Partial" && (
        <>
          {" "}
          <span className="status-line-error" data-testid="status-line-listing-partial">
            {t("status.listingPartial")}
          </span>
        </>
      )}
      {session.phase === "Done" && hasFailures && (
        <>
          {" "}
          <button type="button" data-testid="status-line-failures-button" onClick={onOpenFailures}>
            {failedCount > 0
              ? t("status.failedStreams", { count: failedCount })
              : t("status.showFailures")}
          </button>
        </>
      )}
    </>
  );
}
