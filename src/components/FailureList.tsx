import { useEffect, useRef } from "react";
import type { ApiFailure, FailedStream } from "../api";
import { useEscapeKey } from "../hooks/useEscapeKey";
import type { Translate } from "../i18n/messages";
import { ErrorMessage } from "./ErrorMessage";

export interface FailureListProps {
  failedStreams: readonly FailedStream[];
  /** The stream listing failure, shown first when present. */
  listingFailure: ApiFailure | null;
  t: Translate;
  onClose: () => void;
}

/**
 * The failures of the last fetch (U3:BR6.3): the stream listing failure
 * first, then each failed stream, each in words: what happened, what to do
 * next and the safe detail only (U7:BR2.2-BR2.4, U1:BR4.3). Focus starts
 * on Close (BR6.8); Close or Escape closes it.
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
            <span className="failure-list-name">{t("failures.listing")}</span>
            <ErrorMessage
              failure={listingFailure}
              scene="Stream"
              t={t}
              testId="failure-list-listing-error"
            />
          </li>
        )}
        {failedStreams.map((failed) => (
          <li key={failed.logStreamName} data-testid="failure-list-item">
            <span className="failure-list-name">{failed.logStreamName}</span>
            <ErrorMessage
              failure={failed.failure}
              scene="Stream"
              t={t}
              testId="failure-list-item-error"
            />
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
