import { useEffect, useState, type RefObject } from "react";
import Box from "@cloudscape-design/components/box";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import Checkbox from "@cloudscape-design/components/checkbox";
import FormField from "@cloudscape-design/components/form-field";
import Modal from "@cloudscape-design/components/modal";
import Select, { type SelectProps } from "@cloudscape-design/components/select";
import SpaceBetween from "@cloudscape-design/components/space-between";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import type { SessionView } from "../api";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { LanguagePreference, Translate } from "../i18n/messages";

export interface SettingsDialogProps {
  session: SessionView;
  t: Translate;
  /** The settings button; focus goes back to it when the dialog closes (U6:BR5.2). */
  returnFocusRef: RefObject<ButtonProps.Ref | null>;
  /** [Save]: both settings at once, in one file (261022-small-features U1:BR1.1). */
  onSave: (enabled: boolean, language: LanguagePreference) => void;
  onCancel: () => void;
  onClear: () => void;
}

/** The language choices in the order shown (261022-small-features U1:BR1.1). */
const LANGUAGE_CHOICES: readonly LanguagePreference[] = ["system", "ja", "en"];

/** Whether `value` is one of the language choices. */
function isLanguagePreference(value: string | undefined): value is LanguagePreference {
  return LANGUAGE_CHOICES.some((choice) => choice === value);
}

/**
 * The settings dialog (U6:BR5.1, BR5.2, mockups screen 6): a Cloudscape
 * modal with whether to keep fetched logs in the disk cache, where the cache
 * is, the warning that logs may contain confidential information (both
 * always shown, FR7.2, FR7.3), and the footer [Clear cache], [Cancel] and the
 * primary [Save]. Focus starts on the checkbox, Tab stays inside the dialog
 * (nothing else is usable meanwhile), Space toggles the checkbox, Enter
 * presses the focused button, and Escape or the close button is [Cancel].
 * [Clear cache] acts at once and [Cancel] does not undo it (U6:BR1.4). The
 * dialog closes when the core says so; until then the notice of the last
 * operation is shown in words.
 * Since 261022-small-features U1 a "Language" select under the checkbox
 * (U1:BR1.1): "Follow the OS setting" in the screen's language, then "日本語"
 * and "English", each fixed in its own language so the way back stays
 * readable. It starts on the saved preference, choosing alone changes
 * nothing, [Save] hands both settings to the core, and [Cancel] drops the
 * choice. It is reached with Tab after the checkbox and named by its label
 * (U1:BR1.7); the first focus stays on the checkbox.
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
  const [language, setLanguage] = useState<LanguagePreference>(session.language);
  const checkboxRef = useInitialFocus();
  useEffect(() => {
    const returnTo = returnFocusRef.current;
    return () => returnTo?.focus();
  }, [returnFocusRef]);

  const languageOption = (choice: LanguagePreference): SelectProps.Option => ({
    value: choice,
    label: t(`settings.language.${choice}`),
  });
  const languageOptions = LANGUAGE_CHOICES.map(languageOption);

  const notice = session.settingsNotice;
  return (
    <Modal
      visible
      size="medium"
      header={t("settings.title")}
      closeAriaLabel={t("settings.cancel")}
      onDismiss={onCancel}
      footer={
        <Box float="right">
          <SpaceBetween direction="horizontal" size="xs">
            <Button onClick={onClear} data-testid="settings-dialog-clear-button">
              {t("settings.clear")}
            </Button>
            <Button onClick={onCancel} data-testid="settings-dialog-cancel-button">
              {t("settings.cancel")}
            </Button>
            <Button
              variant="primary"
              onClick={() => onSave(enabled, language)}
              data-testid="settings-dialog-save-button"
            >
              {t("settings.save")}
            </Button>
          </SpaceBetween>
        </Box>
      }
      data-testid="settings-dialog"
    >
      <SpaceBetween size="s">
        <Checkbox
          ref={checkboxRef}
          checked={enabled}
          onChange={({ detail }) => setEnabled(detail.checked)}
          data-testid="settings-dialog-cache-checkbox"
        >
          {t("settings.cacheEnabled")}
        </Checkbox>
        <FormField label={t("settings.language")}>
          <Select
            options={languageOptions}
            selectedOption={languageOption(language)}
            onChange={({ detail }) => {
              const value = detail.selectedOption.value;
              if (isLanguagePreference(value)) {
                setLanguage(value);
              }
            }}
            data-testid="settings-dialog-language-select"
          />
        </FormField>
        <Box data-testid="settings-dialog-location">
          {t("settings.location", {
            path: session.cacheDirectory ?? t("settings.locationUnknown"),
          })}
        </Box>
        <Box color="text-body-secondary" data-testid="settings-dialog-warning">
          {t("settings.warning")}
        </Box>
        {notice && (
          <div role="status" data-testid="settings-dialog-notice">
            <StatusIndicator
              type={notice === "Cleared" ? "success" : "error"}
              iconAriaLabel={t("failure.kind.Other")}
            >
              {t(`settings.notice.${notice}`)}
            </StatusIndicator>
          </div>
        )}
      </SpaceBetween>
    </Modal>
  );
}
