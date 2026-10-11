import Box from "@cloudscape-design/components/box";
import Button from "@cloudscape-design/components/button";
import Modal from "@cloudscape-design/components/modal";
import Table from "@cloudscape-design/components/table";
import type { ApiFailure, FailedStream } from "../api";
import { useInitialFocus } from "../hooks/useInitialFocus";
import type { Translate } from "../i18n/messages";
import { ErrorMessage } from "./ErrorMessage";

export interface FailureListProps {
  failedStreams: readonly FailedStream[];
  /** The stream listing failure, shown first when present. */
  listingFailure: ApiFailure | null;
  t: Translate;
  onClose: () => void;
}

/** One row of the table: the listing failure or one failed stream. */
interface FailureRow {
  key: string;
  name: string;
  failure: ApiFailure;
  testId: string;
}

/**
 * The failures of the last fetch (U3:BR6.3, FR3.3, mockups screen 5d): a
 * modal dialog with a table of the stream listing failure first, then each
 * failed stream, each in words: what happened, what to do next and the safe
 * detail only (U7:BR2.2-BR2.4, U1:BR4.3). Focus starts on [Close] (BR6.8);
 * [Close], the dialog's close button or Escape closes it, and the focus goes
 * back to the button that opened it.
 */
export function FailureList({ failedStreams, listingFailure, t, onClose }: FailureListProps) {
  const closeRef = useInitialFocus();

  const rows: FailureRow[] = [];
  if (listingFailure) {
    rows.push({
      key: "listing",
      name: t("failures.listing"),
      failure: listingFailure,
      testId: "failure-list-listing",
    });
  }
  for (const failed of failedStreams) {
    rows.push({
      key: `stream:${failed.logStreamName}`,
      name: failed.logStreamName,
      failure: failed.failure,
      testId: "failure-list-item",
    });
  }

  return (
    <Modal
      visible
      size="large"
      header={t("failures.title")}
      closeAriaLabel={t("failures.close")}
      onDismiss={onClose}
      footer={
        <Box float="right">
          <Button ref={closeRef} onClick={onClose} data-testid="failure-list-close-button">
            {t("failures.close")}
          </Button>
        </Box>
      }
      data-testid="failure-list"
    >
      <Table<FailureRow>
        variant="embedded"
        items={rows}
        trackBy="key"
        wrapLines
        ariaLabels={{ tableLabel: t("failures.title") }}
        columnDefinitions={[
          {
            id: "stream",
            header: t("failures.column.stream"),
            cell: (row) => <span data-testid={row.testId}>{row.name}</span>,
          },
          {
            id: "reason",
            header: t("failures.column.reason"),
            cell: (row) => (
              <ErrorMessage
                failure={row.failure}
                scene="Stream"
                t={t}
                testId={`${row.testId}-error`}
              />
            ),
          },
        ]}
      />
    </Modal>
  );
}
