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
  /** True only while the connection change dialog is open (U5:BR3.4). */
  disabled: boolean;
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
 * fetch.
 */
export function LogFilterInput({
  value,
  appliedText,
  disabled,
  t,
  onChange,
  onFilterChange,
}: LogFilterInputProps) {
  const debounced = useDebouncedValue(value, FILTER_DELAY_MS);
  const lastSent = useRef(appliedText);

  useEffect(() => {
    if (debounced !== lastSent.current) {
      lastSent.current = debounced;
      onFilterChange(debounced);
    }
  }, [debounced, onFilterChange]);

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
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
