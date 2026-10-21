import Alert from "@cloudscape-design/components/alert";
import type { CancelNoticeView } from "../api";
import type { Translate } from "../i18n/messages";

export interface CancelNoticeProps {
  /** The notice to show; `null` leaves the status region empty. */
  notice: CancelNoticeView | null;
  t: Translate;
}

/**
 * The notice of a cancelled fetch (261012-parallel-fetch U5:BR5.11, FR5.11,
 * refined mockups CancelNotice): a warning above the log list, in the place
 * of the error area, saying how many streams were not read to their end, or
 * that the fetch was cancelled before the stream listing ended. The icon and
 * the words tell it, not the colour alone (NFR9.4). It has no close button;
 * the core drops it with the next fetch, a connection change or another log
 * group (BR5.9), and the focus does not move.
 *
 * The `role="status"` region is always on the screen, empty while there is
 * no notice, and the notice is put into it when it comes
 * (261020-carryover-fixes FR4.2.1): a live region that appears together with
 * its words is not read out by every screen reader, one that is already
 * there is. While empty it takes no room in the column (`.cancel-notice:empty`
 * in styles.css); the warning looks as before (FR4.2.2).
 */
export function CancelNotice({ notice, t }: CancelNoticeProps) {
  return (
    <div role="status" className="cancel-notice" data-testid="cancel-notice">
      {notice !== null && <CancelWarning notice={notice} t={t} />}
    </div>
  );
}

interface CancelWarningProps {
  notice: CancelNoticeView;
  t: Translate;
}

/** The warning itself: how many streams were left, or that the listing had not ended. */
function CancelWarning({ notice, t }: CancelWarningProps) {
  const text =
    notice.unfinishedStreams === null
      ? t("cancel.notice.beforeListing")
      : t("cancel.notice.afterListing", { n: notice.unfinishedStreams });
  return (
    <Alert type="warning" statusIconAriaLabel={t("status.cancelled")}>
      <span data-testid="cancel-notice-text">{text}</span>
    </Alert>
  );
}
