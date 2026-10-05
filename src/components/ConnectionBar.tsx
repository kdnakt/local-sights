import type { ChangeEvent } from "react";
import { selectorOf, type ProfileSelector, type SessionView, type TimeZoneChoice } from "../api";
import type { Translate } from "../i18n/messages";
import { TimeZoneToggle } from "./TimeZoneToggle";

const SDK_DEFAULT_VALUE = "sdk-default";
const NAMED_PREFIX = "named:";

function encodeSelector(selector: ProfileSelector): string {
  return selector.kind === "SdkDefault"
    ? SDK_DEFAULT_VALUE
    : `${NAMED_PREFIX}${selector.profileName}`;
}

function decodeSelector(value: string): ProfileSelector | null {
  if (value === SDK_DEFAULT_VALUE) {
    return { kind: "SdkDefault" };
  }
  return value.startsWith(NAMED_PREFIX)
    ? { kind: "Named", profileName: value.slice(NAMED_PREFIX.length) }
    : null;
}

export interface ConnectionBarProps {
  session: SessionView;
  t: Translate;
  onSelectProfile: (profile: ProfileSelector) => void;
  onSelectRegion: (region: string) => void;
  onSelectTimeZone: (timeZone: TimeZoneChoice) => void;
}

/**
 * Top bar: profile (the SDK default first, U2:BR1.1) and region (U2:BR1.4)
 * selectors, both unselected at startup (U2:BR2.1), and the notices for
 * config files that could not be read (file kind only, U2:BR1.3). Disabled
 * while fetching or while a change awaits confirmation (U2:BR2.6). Since U4
 * it also holds the time zone switch, which stays usable then (U4:BR3.3).
 */
export function ConnectionBar({
  session,
  t,
  onSelectProfile,
  onSelectRegion,
  onSelectTimeZone,
}: ConnectionBarProps) {
  const disabled = !session.canChangeConnection;
  const profileValue = session.connection.profile ? encodeSelector(session.connection.profile) : "";

  const handleProfile = (event: ChangeEvent<HTMLSelectElement>) => {
    const selector = decodeSelector(event.target.value);
    if (selector) {
      onSelectProfile(selector);
    }
  };

  const handleRegion = (event: ChangeEvent<HTMLSelectElement>) => {
    if (event.target.value !== "") {
      onSelectRegion(event.target.value);
    }
  };

  return (
    <header className="connection-bar" aria-label={t("connection.label")}>
      <label className="connection-bar-field">
        <span>{t("connection.profile.label")}</span>
        <select
          value={profileValue}
          disabled={disabled}
          onChange={handleProfile}
          data-testid="connection-bar-profile-select"
        >
          <option value="" disabled>
            {t("connection.profile.placeholder")}
          </option>
          {session.profiles.map((profile) => {
            const value = encodeSelector(selectorOf(profile));
            return (
              <option key={value} value={value}>
                {profile.kind === "SdkDefault"
                  ? t("connection.profile.sdkDefault")
                  : profile.profileName}
              </option>
            );
          })}
        </select>
      </label>
      <label className="connection-bar-field">
        <span>{t("connection.region.label")}</span>
        <select
          value={session.connection.region ?? ""}
          disabled={disabled}
          onChange={handleRegion}
          data-testid="connection-bar-region-select"
        >
          <option value="" disabled>
            {t("connection.region.placeholder")}
          </option>
          {session.regions.map((region) => (
            <option key={region} value={region}>
              {region}
            </option>
          ))}
        </select>
      </label>
      <TimeZoneToggle timeZone={session.timeZone} t={t} onSelect={onSelectTimeZone} />
      {session.catalogNotices.map((key) => (
        <p
          key={key}
          className="connection-bar-notice"
          role="status"
          data-testid="connection-bar-notice"
        >
          {t(key)}
        </p>
      ))}
    </header>
  );
}
