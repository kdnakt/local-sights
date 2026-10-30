import Alert from "@cloudscape-design/components/alert";
import type { ExportNotice } from "../api";
import type { Translate } from "../i18n/messages";

export interface ExportNoticeAlertProps {
  /** The core's notice of the last export; nothing is drawn without one. */
  notice: ExportNotice | null;
  t: Translate;
  /** The user closed the notice (U2:BR4.9). */
  onDismiss: () => void;
  /**
   * The export copied the cancelled result of a query, made from part of the logs
   * (261025-insights-stats U3:BR3.5, FR5.2): a saved or copied notice says so. The file
   * itself carries no mark.
   */
  partialResult?: boolean;
}

/**
 * The text of an export notice (U2:BR4.9, review R-02): built here from the
 * notice's kind and arguments, so a language switch rewrites it. A failure
 * names its kind through `export.failure.<kind>` with the chosen path; the
 * OS's own words never reach the screen (NFR5).
 */
function exportNoticeText(notice: ExportNotice, t: Translate): string {
  switch (notice.kind) {
    case "Saved":
      return t("export.notice.saved", { count: notice.rowCount, path: notice.path });
    case "Copied":
      return t("export.notice.copied", { count: notice.rowCount });
    case "SaveFailed":
      return t("export.notice.saveFailed", {
        detail: t(`export.failure.${notice.failure}`, { path: notice.path ?? "" }),
      });
    case "CopyFailed":
      return t("export.notice.copyFailed", { detail: t(`export.failure.${notice.failure}`) });
  }
}

/**
 * The notice of the last export in the alert area of a screen
 * (261022-small-features U2:BR4.9, BR5.5): a dismissible success Alert in a
 * status region for a saved or copied export, a dismissible error Alert in
 * an alert region for a failure. Closing it tells the core (`onDismiss`).
 */
export function ExportNoticeAlert({
  notice,
  t,
  onDismiss,
  partialResult = false,
}: ExportNoticeAlertProps) {
  if (notice === null) {
    return null;
  }
  const succeeded = notice.kind === "Saved" || notice.kind === "Copied";
  const text = exportNoticeText(notice, t);
  return (
    <div role={succeeded ? "status" : "alert"} data-testid="export-notice">
      <Alert
        type={succeeded ? "success" : "error"}
        dismissible
        dismissAriaLabel={t("export.notice.dismiss")}
        statusIconAriaLabel={t(succeeded ? "export.notice.success" : "export.notice.failure")}
        onDismiss={onDismiss}
      >
        {succeeded && partialResult ? `${text} ${t("export.notice.partialResult")}` : text}
      </Alert>
    </div>
  );
}
