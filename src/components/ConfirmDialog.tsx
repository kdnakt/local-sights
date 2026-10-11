import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Modal from "@cloudscape-design/components/modal";
import SpaceBetween from "@cloudscape-design/components/space-between";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { Translate } from "../i18n/messages";

export interface ConfirmDialogProps {
  t: Translate;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Asks before a connection change discards the shown logs (U2:BR2.5,
 * mockups screen 6): a Cloudscape modal whose footer has [Cancel] and the
 * primary [Change] at the right end. Focus starts on [Cancel], the modal
 * keeps Tab inside it (U2:BR2.6: nothing else is usable meanwhile), and
 * Escape or the close button cancels (U2:BR5.2).
 */
export function ConfirmDialog({ t, onConfirm, onCancel }: ConfirmDialogProps) {
  const cancelRef = useInitialFocus();

  return (
    <Modal
      visible
      size="small"
      header={t("confirm.title")}
      closeAriaLabel={t("confirm.cancel")}
      onDismiss={onCancel}
      footer={
        <Box float="right">
          <SpaceBetween direction="horizontal" size="xs">
            <Button ref={cancelRef} onClick={onCancel} data-testid="confirm-dialog-cancel-button">
              {t("confirm.cancel")}
            </Button>
            <Button
              variant="primary"
              onClick={onConfirm}
              data-testid="confirm-dialog-change-button"
            >
              {t("confirm.change")}
            </Button>
          </SpaceBetween>
        </Box>
      }
      data-testid="confirm-dialog"
    >
      <p data-testid="confirm-dialog-message">{t("confirm.message")}</p>
    </Modal>
  );
}
