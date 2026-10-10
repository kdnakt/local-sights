import { useEffect, useRef, type KeyboardEvent } from "react";
import { useEscapeKey } from "../hooks/useEscapeKey";
import type { Translate } from "../i18n/messages";

export interface CloseConfirmDialogProps {
  t: Translate;
  /** [Keep fetching] or Escape (U7:BR3.2). */
  onKeep: () => void;
  /** [Close]: stop fetching and end the app (U7:BR3.2). */
  onClose: () => void;
}

/**
 * Screen 7 (U7:BR3.3): asks before closing while fetching, saying in words
 * that the logs fetched so far will be lost. Shown while the session's
 * closeConfirmation is Pending, also when the fetch ends meanwhile (BR3.4).
 * Focus starts on [Keep fetching], so an Enter pressed by mistake loses
 * nothing; Tab and Shift+Tab stay on the two buttons; Escape is [Keep
 * fetching]; the focus goes back where it was when the dialog closes.
 */
export function CloseConfirmDialog({ t, onKeep, onClose }: CloseConfirmDialogProps) {
  const keepRef = useRef<HTMLButtonElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  useEscapeKey(onKeep);
  useEffect(() => {
    const previous = document.activeElement;
    keepRef.current?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) {
        previous.focus();
      }
    };
  }, []);

  const trapFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") {
      return;
    }
    event.preventDefault();
    const onKeepButton = document.activeElement === keepRef.current;
    (onKeepButton ? closeRef : keepRef).current?.focus();
  };

  return (
    <div className="confirm-backdrop">
      <div
        className="confirm-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="close-confirm-title"
        aria-describedby="close-confirm-message"
        data-testid="close-confirm-dialog"
        onKeyDown={trapFocus}
      >
        <h2 id="close-confirm-title">{t("close.title")}</h2>
        <p id="close-confirm-message">{t("close.message")}</p>
        <div className="confirm-dialog-buttons">
          <button
            type="button"
            ref={keepRef}
            onClick={onKeep}
            data-testid="close-confirm-keep-button"
          >
            {t("close.keep")}
          </button>
          <button
            type="button"
            ref={closeRef}
            onClick={onClose}
            data-testid="close-confirm-close-button"
          >
            {t("close.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
