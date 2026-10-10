import type { TimeZoneChoice } from "../api";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";

const CHOICES: readonly TimeZoneChoice[] = ["Local", "Utc"];

const TEST_IDS: Record<TimeZoneChoice, string> = {
  Local: "time-zone-toggle-local-radio",
  Utc: "time-zone-toggle-utc-radio",
};

export interface TimeZoneToggleProps {
  timeZone: TimeZoneChoice;
  t: Translate;
  onSelect: (timeZone: TimeZoneChoice) => void;
}

/**
 * The time zone switch of the top bar (U4:BR3.3): two standard radio buttons,
 * Local and UTC. Tab reaches the group; the arrow keys or Space choose. It is
 * never disabled, not even while fetching or while a connection change awaits
 * confirmation, because switching changes no fetch condition (U4:BR1.4). The
 * screen only shows the choice the core holds; it converts nothing.
 */
export function TimeZoneToggle({ timeZone, t, onSelect }: TimeZoneToggleProps) {
  return (
    <fieldset className="time-zone-toggle" data-testid="time-zone-toggle">
      <legend>{t("timeZone.label")}</legend>
      {CHOICES.map((choice) => (
        <label key={choice} className="time-zone-toggle-option">
          <input
            type="radio"
            name="time-zone"
            value={choice}
            checked={timeZone === choice}
            data-testid={TEST_IDS[choice]}
            onChange={() => onSelect(choice)}
          />
          {t(timeZoneLabelKey(choice))}
        </label>
      ))}
    </fieldset>
  );
}
