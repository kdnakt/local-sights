import { useEffect, useState, type RefObject } from "react";
import Box from "@cloudscape-design/components/box";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import Checkbox from "@cloudscape-design/components/checkbox";
import Modal from "@cloudscape-design/components/modal";
import SpaceBetween from "@cloudscape-design/components/space-between";
import StatusIndicator from "@cloudscape-design/components/status-indicator";
import type { SessionView } from "../api";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { Translate } from "../i18n/messages";

export interface SettingsDialogProps {
  session: SessionView;
  t: Translate;
  /** The settings button; focus goes back to it when the dialog closes (U6:BR5.2). */
  returnFocusRef: RefObject<ButtonProps.Ref | null>;
  onSave: (enabled: boolean) => void;
  onCancel: () => void;
  onClear: () => void;
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
  const checkboxRef = useInitialFocus();
  useEffect(() => {
    const returnTo = returnFocusRef.current;
    return () => returnTo?.focus();
  }, [returnFocusRef]);

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
              onClick={() => onSave(enabled)}
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
