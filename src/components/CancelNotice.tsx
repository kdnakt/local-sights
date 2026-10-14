import Alert from "@cloudscape-design/components/alert";
import type { CancelNoticeView } from "../api";
import type { Translate } from "../i18n/messages";

export interface CancelNoticeProps {
  notice: CancelNoticeView;
  t: Translate;
}

/**
 * The notice of a cancelled fetch (261012-parallel-fetch U5:BR5.11, FR5.11,
 * refined mockups CancelNotice): a warning above the log list, in the place
 * of the error area, saying how many streams were not read to their end, or
 * that the fetch was cancelled before the stream listing ended. The icon and
 * the words tell it, not the colour alone (NFR9.4). It has no close button;
 * the core drops it with the next fetch, a connection change or another log
 * group (BR5.9). The `role="status"` wrapper reads it out once when it
 * appears, and the focus does not move.
 */
export function CancelNotice({ notice, t }: CancelNoticeProps) {
  const text =
    notice.unfinishedStreams === null
      ? t("cancel.notice.beforeListing")
      : t("cancel.notice.afterListing", { n: notice.unfinishedStreams });
  return (
    <div role="status" data-testid="cancel-notice">
      <Alert type="warning" statusIconAriaLabel={t("status.cancelled")}>
        <span data-testid="cancel-notice-text">{text}</span>
      </Alert>
    </div>
  );
}
