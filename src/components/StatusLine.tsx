import Button from "@cloudscape-design/components/button";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import type { CacheNotice, FilterSummary, SessionView } from "../api";
import { errorLines } from "../errorText";
import type { Translate } from "../i18n/messages";

export interface StatusLineProps {
  session: SessionView | null;
  t: Translate;
  /** Opens the list of failed streams (U3:BR6.3). */
  onOpenFailures: () => void;
}

/** Message key of each cache notice shown next to the count (U6:BR5.3). */
const CACHE_NOTICE_KEYS: Record<CacheNotice, string> = {
  Hit: "cache.hit",
  ReadFailed: "cache.readFailed",
  SaveFailed: "cache.saveFailed",
};

/**
 * One status line below the list (U3:BR6.2, FR3.3, interaction-spec §7),
 * drawn with Cloudscape's status indicators. While listing: the streams
 * selected so far. While fetching: finished / planned streams and the
 * events so far (loading). After a fetch: the event count including the
 * explicit zero case (U1:BR5.4), a link button with the number of failed
 * streams, the notice that the stream listing is incomplete, and on failure
 * what happened in words (U7:BR2.4: only that sentence; what to do and the
 * detail are in the error area above the list). Since U5, while a log
 * filter is in force: "filtered N of M events" (with the zero case in words)
 * and, while the scan runs, "filtering" (U5:BR3.3). Since U6: "saving to
 * the cache" while a fetch writes the cache, and after a fetch its cache
 * notices next to the count, in words (U6:BR5.3); a fetch shown from the
 * cache has no stream counts (U6 review R-12).
 */
export function StatusLine({ session, t, onOpenFailures }: StatusLineProps) {
  const filter = session?.filterSummary ?? null;
  return (
    <div className="status-line" role="status" aria-live="polite" data-testid="status-line">
      {renderStatus(session, t, onOpenFailures)}
      {session && renderCache(session, t)}
      {filter && renderFilter(filter, t)}
    </div>
  );
}

function renderFilter(filter: FilterSummary, t: Translate) {
  return (
    <>
      <span className="status-line-filter" data-testid="status-line-filter">
        {filter.matchedCount === 0
          ? t("status.filter.zero", { all: filter.allCount })
          : t("status.filter.count", { matched: filter.matchedCount, all: filter.allCount })}
      </span>
      {filter.status === "Filtering" && (
        <span data-testid="status-line-filtering">
          <StatusIndicator type="loading">{t("status.filter.filtering")}</StatusIndicator>
        </span>
      )}
    </>
  );
}

function renderCache(session: SessionView, t: Translate) {
  if (session.phase === "Fetching") {
    return (
      session.cacheSaving && (
        <span data-testid="status-line-cache-saving">
          <StatusIndicator type="in-progress">{t("cache.saving")}</StatusIndicator>
        </span>
      )
    );
  }
  return session.cacheNotices.map((notice) => (
    <span key={notice} className="status-line-cache" data-testid={`status-line-cache-${notice}`}>
      {t(CACHE_NOTICE_KEYS[notice])}
    </span>
  ));
}

function renderCount(session: SessionView, t: Translate) {
  return (
    <span data-testid="status-line-count">
      <StatusIndicator type={session.eventCount === 0 ? "stopped" : "success"}>
        {session.eventCount === 0
          ? t("status.zero")
          : t("status.count", { count: session.eventCount })}
      </StatusIndicator>
    </span>
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
          <StatusIndicator type="loading">
            {t("status.listing", { count: progress?.selectedStreamCount ?? 0 })}
          </StatusIndicator>
        </span>
      );
    }
    return (
      <span data-testid="status-line-fetching">
        <StatusIndicator type="loading">
          {t("status.fetching", {
            finished: progress.finishedStreamCount,
            planned: progress.plannedStreamCount,
            count: progress.eventCount,
          })}
        </StatusIndicator>
      </span>
    );
  }
  if (session.cacheNotices.includes("Hit")) {
    return renderCount(session, t);
  }
  const failure = session.phase === "Failed" ? session.lastJob?.failure : null;
  const failedCount = session.failedStreams.length;
  const hasFailures = failedCount > 0 || session.listingFailure !== null;
  return (
    <>
      {failure && (
        <span className="status-line-error" data-testid="status-line-error">
          <StatusIndicator type="error" iconAriaLabel={t("failure.kind.Other")}>
            <span data-testid="status-line-what">
              {errorLines(failure.kind, "Fetch", failure.safeDetail, t).what}
            </span>
          </StatusIndicator>
        </span>
      )}
      {renderCount(session, t)}
      {session.phase === "Done" && session.listingStatus === "Partial" && (
        <span className="status-line-error" data-testid="status-line-listing-partial">
          <StatusIndicator type="warning" iconAriaLabel={t("failure.kind.Other")}>
            {t("status.listingPartial")}
          </StatusIndicator>
        </span>
      )}
      {session.phase === "Done" && hasFailures && (
        <Button
          variant="inline-link"
          onClick={onOpenFailures}
          data-testid="status-line-failures-button"
        >
          {failedCount > 0
            ? t("status.failedStreams", { count: failedCount })
            : t("status.showFailures")}
        </Button>
      )}
    </>
  );
}
