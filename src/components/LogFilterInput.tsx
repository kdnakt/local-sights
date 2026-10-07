import { useEffect, useRef, type KeyboardEvent } from "react";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import type { Translate } from "../i18n/messages";

/** How long typing must pause before the text goes to the core (U5:BR1.3). */
export const FILTER_DELAY_MS = 300;

export interface LogFilterInputProps {
  /** The text in the field; the parent keeps it so a remount loses nothing. */
  value: string;
  /** The text the core last received (`SessionView.logFilter`). */
  appliedText: string;
  /** True only while the connection change or settings dialog is open (U5:BR3.4). */
  disabled: boolean;
  /**
   * Counts the failed hand-overs (`set_log_filter` refused); each increase
   * makes the field compare its text with `appliedText` and send it again
   * (U5 review R-02).
   */
  failureCount?: number;
  t: Translate;
  /** Every keystroke. */
  onChange: (value: string) => void;
  /** The text once typing paused for {@link FILTER_DELAY_MS}; only the last one. */
  onFilterChange: (text: string) => void;
}

/**
 * The log filter field of the condition area (U5:BR3.4): a standard search
 * field with an English or Japanese label and placeholder, reachable with
 * Tab and usable while fetching, since filtering is not a fetch condition.
 * The text goes to the core about 0.3 s after typing stops (U5:BR1.3); the
 * core trims it and decides whether anything changed (U5:BR1.1, BR1.4).
 * Enter does not submit the fetch form, so typing a filter never starts a
 * fetch; since U6 it hands the text on at once instead of waiting (U6:BR5.4).
 * When a hand-over failed, or when a dialog that disabled the field closes,
 * the field compares its text with the one the core holds and sends it again
 * when they differ (U5 review R-02).
 */
export function LogFilterInput({
  value,
  appliedText,
  disabled,
  failureCount = 0,
  t,
  onChange,
  onFilterChange,
}: LogFilterInputProps) {
  const debounced = useDebouncedValue(value, FILTER_DELAY_MS);
  const lastSent = useRef(appliedText);
  const seen = useRef({ disabled, failureCount });

  useEffect(() => {
    if (debounced !== lastSent.current) {
      lastSent.current = debounced;
      onFilterChange(debounced);
    }
  }, [debounced, onFilterChange]);

  // U5 review R-02: after a refused hand-over, or once a dialog closed, the
  // core may hold another text than the field; send the field's text again.
  useEffect(() => {
    const previous = seen.current;
    seen.current = { disabled, failureCount };
    const reopened = previous.disabled && !disabled;
    const failed = failureCount !== previous.failureCount;
    if (!disabled && (reopened || failed) && value !== appliedText) {
      lastSent.current = value;
      onFilterChange(value);
    }
  }, [disabled, failureCount, value, appliedText, onFilterChange]);

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    // U6:BR5.4: hand the text on at once; the same text again does nothing.
    if (value !== lastSent.current) {
      lastSent.current = value;
      onFilterChange(value);
    }
  };

  return (
    <label className="fetch-form-field log-filter">
      <span>{t("logFilter.label")}</span>
      <input
        type="search"
        name="logFilter"
        value={value}
        placeholder={t("logFilter.placeholder")}
        disabled={disabled}
        autoComplete="off"
        spellCheck={false}
        data-testid="log-filter-input"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={handleKeyDown}
      />
    </label>
  );
}
