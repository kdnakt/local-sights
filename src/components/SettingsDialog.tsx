import { useEffect, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import type { SessionView } from "../api";
import { useEscapeKey } from "../hooks/useEscapeKey";
import type { Translate } from "../i18n/messages";

export interface SettingsDialogProps {
  session: SessionView;
  t: Translate;
  /** The [*] button; focus goes back to it when the dialog closes (U6:BR5.2). */
  returnFocusRef: RefObject<HTMLButtonElement | null>;
  onSave: (enabled: boolean) => void;
  onCancel: () => void;
  onClear: () => void;
}

/**
 * The settings dialog (screen 6, U6:BR5.1, BR5.2): whether to keep fetched
 * logs in the disk cache, where the cache is, the warning that logs may
 * contain confidential information (both always shown, FR7.2, FR7.3),
 * [Clear cache], [Cancel] and [Save]. Focus starts on the checkbox, Tab and
 * Shift+Tab cycle inside the dialog (nothing else is usable meanwhile),
 * Space toggles the checkbox, Enter presses the focused button, and Escape
 * is [Cancel]. [Clear cache] acts at once and [Cancel] does not undo it
 * (U6:BR1.4). The dialog closes when the core says so; until then the
 * notice of the last operation is shown in words.
 */
export function SettingsDialog({
  session,
  t,
  returnFocusRef,
  onSave,
  onCancel,
  onClear,
}: SettingsDialogProps) {
  const [enabled, setEnabled] = useState(session.cacheEnabled);
  const dialogRef = useRef<HTMLDivElement>(null);
  const checkboxRef = useRef<HTMLInputElement>(null);
  useEscapeKey(onCancel);
  useEffect(() => {
    checkboxRef.current?.focus();
    const returnTo = returnFocusRef.current;
    return () => returnTo?.focus();
  }, [returnFocusRef]);

  const trapFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab" || dialogRef.current === null) {
      return;
    }
    const focusable = Array.from(
      dialogRef.current.querySelectorAll<HTMLElement>("input, button:not(:disabled)"),
    );
    if (focusable.length === 0) {
      return;
    }
    event.preventDefault();
    const current = focusable.indexOf(document.activeElement as HTMLElement);
    const step = event.shiftKey ? -1 : 1;
    const next = (current + step + focusable.length) % focusable.length;
    focusable[next]?.focus();
  };

  return (
    <div className="confirm-backdrop">
      <div
        ref={dialogRef}
        className="confirm-dialog settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-dialog-title"
        data-testid="settings-dialog"
        onKeyDown={trapFocus}
      >
        <h2 id="settings-dialog-title">{t("settings.title")}</h2>
        <label className="settings-dialog-option">
          <input
            ref={checkboxRef}
            type="checkbox"
            checked={enabled}
            onChange={(event) => setEnabled(event.target.checked)}
            data-testid="settings-dialog-cache-checkbox"
          />{" "}
          {t("settings.cacheEnabled")}
        </label>
        <p data-testid="settings-dialog-location">
          {t("settings.location", {
            path: session.cacheDirectory ?? t("settings.locationUnknown"),
          })}
        </p>
        <p data-testid="settings-dialog-warning">{t("settings.warning")}</p>
        {session.settingsNotice && (
          <p role="status" data-testid="settings-dialog-notice">
            {t(`settings.notice.${session.settingsNotice}`)}
          </p>
        )}
        <div className="confirm-dialog-buttons">
          <button type="button" onClick={onClear} data-testid="settings-dialog-clear-button">
            {t("settings.clear")}
          </button>
          <button type="button" onClick={onCancel} data-testid="settings-dialog-cancel-button">
            {t("settings.cancel")}
          </button>
          <button
            type="button"
            onClick={() => onSave(enabled)}
            data-testid="settings-dialog-save-button"
          >
            {t("settings.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
