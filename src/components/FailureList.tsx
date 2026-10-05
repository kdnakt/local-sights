import { useEffect, useRef } from "react";
import type { ApiFailure, FailedStream } from "../api";
import { useEscapeKey } from "../hooks/useEscapeKey";
import type { Translate } from "../i18n/messages";

export interface FailureListProps {
  failedStreams: readonly FailedStream[];
  /** The stream listing failure, shown first when present. */
  listingFailure: ApiFailure | null;
  t: Translate;
  onClose: () => void;
}

/**
 * The failures of the last fetch (U3:BR6.3): the stream listing failure
 * first, then each failed stream with the kind name and the safe detail
 * only (U1:BR4.3). Focus starts on Close (BR6.8); Close or Escape closes it.
 */
export function FailureList({ failedStreams, listingFailure, t, onClose }: FailureListProps) {
  const closeRef = useRef<HTMLButtonElement>(null);
  useEscapeKey(onClose);
  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  return (
    <section
      className="failure-list"
      role="dialog"
      aria-labelledby="failure-list-title"
      data-testid="failure-list"
    >
      <h2 id="failure-list-title">{t("failures.title")}</h2>
      <ul className="failure-list-items">
        {listingFailure && (
          <li data-testid="failure-list-listing">
            <span className="failure-list-name">{t("failures.listing")}</span>{" "}
            <span>{t(`failure.kind.${listingFailure.kind}`)}</span>{" "}
            <span className="failure-list-detail">
              {t("status.detail", { detail: listingFailure.safeDetail })}
            </span>
          </li>
        )}
        {failedStreams.map((failed) => (
          <li key={failed.logStreamName} data-testid="failure-list-item">
            <span className="failure-list-name">{failed.logStreamName}</span>{" "}
            <span>{t(`failure.kind.${failed.failure.kind}`)}</span>{" "}
            <span className="failure-list-detail">
              {t("status.detail", { detail: failed.failure.safeDetail })}
            </span>
          </li>
        ))}
      </ul>
      <button
        type="button"
        ref={closeRef}
        onClick={onClose}
        data-testid="failure-list-close-button"
      >
        {t("failures.close")}
      </button>
    </section>
  );
}
