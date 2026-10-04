import { useEffect, useRef } from "react";
import { useEscapeKey } from "../hooks/useEscapeKey";
import type { Translate } from "../i18n/messages";

export interface ConfirmDialogProps {
  t: Translate;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Asks before a connection change discards the shown logs (U2:BR2.5). Focus
 * starts on Cancel and Escape cancels (U2:BR5.2).
 */
export function ConfirmDialog({ t, onConfirm, onCancel }: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEscapeKey(onCancel);
  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  return (
    <div className="confirm-backdrop">
      <div
        className="confirm-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
        aria-describedby="confirm-dialog-message"
        data-testid="confirm-dialog"
      >
        <h2 id="confirm-dialog-title">{t("confirm.title")}</h2>
        <p id="confirm-dialog-message">{t("confirm.message")}</p>
        <div className="confirm-dialog-buttons">
          <button type="button" onClick={onConfirm} data-testid="confirm-dialog-change-button">
            {t("confirm.change")}
          </button>
          <button
            type="button"
            ref={cancelRef}
            onClick={onCancel}
            data-testid="confirm-dialog-cancel-button"
          >
            {t("confirm.cancel")}
          </button>
        </div>
      </div>
    </div>
  );
}
