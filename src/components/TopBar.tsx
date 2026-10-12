import type { Ref } from "react";
import Alert from "@cloudscape-design/components/alert";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import ButtonDropdown, {
  type ButtonDropdownProps,
} from "@cloudscape-design/components/button-dropdown";
import TopNavigation from "@cloudscape-design/components/top-navigation";
import {
  isSsoLoginRunning,
  selectorOf,
  type ProfileSelector,
  type SessionView,
  type TimeZoneChoice,
} from "../api";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";

const SDK_DEFAULT_VALUE = "sdk-default";
const NAMED_PREFIX = "named:";
const TIME_ZONES: readonly TimeZoneChoice[] = ["Local", "Utc"];

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

export interface TopBarProps {
  session: SessionView;
  t: Translate;
  onSelectProfile: (profile: ProfileSelector) => void;
  onSelectRegion: (region: string) => void;
  onSelectTimeZone: (timeZone: TimeZoneChoice) => void;
  /** Opens the settings dialog (U6:BR5.1). */
  onOpenSettings?: () => void;
  /** The settings button, to which focus returns when the dialog closes (U6:BR5.2). */
  settingsButtonRef?: Ref<ButtonProps.Ref>;
}

/**
 * The black top navigation of the console (mockups screen 1, FR4.1, FR4.2):
 * the app name on the left and, on the right, the profile (the SDK default
 * first, U2:BR1.1), the region (U2:BR1.4) and the time zone (U4:BR3.3) as
 * dropdowns, then the gear button of the settings dialog (U6:BR5.1). The
 * core selects the SDK default profile at startup, with its default region
 * when the SDK has one (gate fix 14c, replacing U2:BR2.1's "nothing
 * selected"); the bar only shows what the session holds. Enabled and disabled per
 * the plan's table (R-05): while fetching the connection dropdowns and the
 * settings button are disabled and the time zone stays usable (FR1.5); while
 * a connection change awaits confirmation or the settings dialog is open,
 * everything is disabled (U2:BR2.6, U6:BR5.1); while an SSO login runs the
 * profile, the region and the settings button are disabled and the time
 * zone stays usable (261011-sso-login BR3.4). The notices for config files
 * that could not be read (file kind only, U2:BR1.3) are warning alerts right
 * below the bar. The dropdowns are Cloudscape ButtonDropdowns inside the
 * TopNavigation's custom content, because its structured utilities cannot be
 * disabled (recorded in code-summary.md).
 */
export function TopBar({
  session,
  t,
  onSelectProfile,
  onSelectRegion,
  onSelectTimeZone,
  onOpenSettings,
  settingsButtonRef,
}: TopBarProps) {
  // 261011-sso-login BR3.4: while signing in, the profile, the region and
  // the settings are disabled (the core refuses them too).
  const loggingIn = isSsoLoginRunning(session.ssoLogin);
  const connectionDisabled = !session.canChangeConnection || loggingIn;
  const dialogOpen = session.pendingChange !== null || session.settingsDialog === "Open";
  const currentProfile = session.connection.profile
    ? encodeSelector(session.connection.profile)
    : null;
  const profileName = (profile: SessionView["profiles"][number]) =>
    profile.kind === "SdkDefault"
      ? t("connection.profile.sdkDefault")
      : (profile.profileName ?? "");

  const profileItems: ButtonDropdownProps.Items = session.profiles.map((profile) => {
    const id = encodeSelector(selectorOf(profile));
    return {
      id,
      text: profileName(profile),
      iconName: id === currentProfile ? "check" : undefined,
    };
  });
  const regionItems: ButtonDropdownProps.Items = session.regions.map((region) => ({
    id: region,
    text: region,
    iconName: region === session.connection.region ? "check" : undefined,
  }));
  const timeZoneItems: ButtonDropdownProps.Items = TIME_ZONES.map((zone) => ({
    id: zone,
    text: t(timeZoneLabelKey(zone)),
    iconName: zone === session.timeZone ? "check" : undefined,
  }));

  const selectedProfile = session.profiles.find(
    (profile) => encodeSelector(selectorOf(profile)) === currentProfile,
  );
  const profileText = selectedProfile
    ? `${t("connection.profile.label")}: ${profileName(selectedProfile)}`
    : t("connection.profile.placeholder");
  const regionText = session.connection.region ?? t("connection.region.placeholder");
  const timeZoneText = `${t("timeZone.label")}: ${t(timeZoneLabelKey(session.timeZone))}`;

  return (
    <>
      <TopNavigation visualContext="top-navigation" data-testid="top-bar">
        <div className="top-bar">
          <h1 className="top-bar-title">{t("app.title")}</h1>
          <div className="top-bar-utilities">
            <ButtonDropdown
              items={profileItems}
              disabled={connectionDisabled}
              ariaLabel={t("connection.profile.label")}
              onItemClick={({ detail }) => {
                const selector = decodeSelector(detail.id);
                if (selector) {
                  onSelectProfile(selector);
                }
              }}
              data-testid="top-bar-profile"
            >
              {profileText}
            </ButtonDropdown>
            <ButtonDropdown
              items={regionItems}
              disabled={connectionDisabled}
              ariaLabel={t("connection.region.label")}
              onItemClick={({ detail }) => onSelectRegion(detail.id)}
              data-testid="top-bar-region"
            >
              {regionText}
            </ButtonDropdown>
            <ButtonDropdown
              items={timeZoneItems}
              disabled={dialogOpen}
              ariaLabel={t("timeZone.label")}
              onItemClick={({ detail }) => {
                if (detail.id === "Local" || detail.id === "Utc") {
                  onSelectTimeZone(detail.id);
                }
              }}
              data-testid="top-bar-time-zone"
            >
              {timeZoneText}
            </ButtonDropdown>
            <Button
              ref={settingsButtonRef}
              variant="icon"
              iconName="settings"
              ariaLabel={t("settings.open")}
              disabled={
                !session.canOpenSettings || dialogOpen || loggingIn || onOpenSettings === undefined
              }
              onClick={onOpenSettings}
              data-testid="top-bar-settings-button"
            />
          </div>
        </div>
      </TopNavigation>
      {session.catalogNotices.length > 0 && (
        <div className="app-notices">
          {session.catalogNotices.map((key) => (
            <Alert
              key={key}
              type="warning"
              statusIconAriaLabel={t("failure.kind.Other")}
              data-testid="top-bar-notice"
            >
              {t(key)}
            </Alert>
          ))}
        </div>
      )}
    </>
  );
}
