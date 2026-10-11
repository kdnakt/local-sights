import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Modal from "@cloudscape-design/components/modal";
import SpaceBetween from "@cloudscape-design/components/space-between";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { Translate } from "../i18n/messages";

export interface CloseConfirmDialogProps {
  t: Translate;
  /** [Keep fetching], the close button or Escape (U7:BR3.2). */
  onKeep: () => void;
  /** [Close]: stop fetching and end the app (U7:BR3.2). */
  onClose: () => void;
}

/**
 * Screen 7 (U7:BR3.3, mockups screen 6): asks before closing while fetching,
 * saying in words that the logs fetched so far will be lost. Shown while the
 * session's closeConfirmation is Pending, also when the fetch ends meanwhile
 * (BR3.4). A Cloudscape modal: focus starts on [Keep fetching], so an Enter
 * pressed by mistake loses nothing; Tab stays inside the dialog; Escape is
 * [Keep fetching]; the focus goes back where it was when the dialog closes.
 */
export function CloseConfirmDialog({ t, onKeep, onClose }: CloseConfirmDialogProps) {
  const keepRef = useInitialFocus();

  return (
    <Modal
      visible
      size="small"
      header={t("close.title")}
      closeAriaLabel={t("close.keep")}
      onDismiss={onKeep}
      footer={
        <Box float="right">
          <SpaceBetween direction="horizontal" size="xs">
            <Button ref={keepRef} onClick={onKeep} data-testid="close-confirm-keep-button">
              {t("close.keep")}
            </Button>
            <Button variant="primary" onClick={onClose} data-testid="close-confirm-close-button">
              {t("close.close")}
            </Button>
          </SpaceBetween>
        </Box>
      }
      data-testid="close-confirm-dialog"
    >
      <p data-testid="close-confirm-message">{t("close.message")}</p>
    </Modal>
  );
}
