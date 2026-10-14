import type { ElapsedView } from "../api";
import { formatElapsed } from "../elapsed";
import { useElapsed } from "../hooks/useElapsed";
import type { Translate } from "../i18n/messages";

export interface ElapsedTimeProps {
  /** `SessionView.elapsed`; nothing is shown when it is empty. */
  elapsed: ElapsedView | null | undefined;
  t: Translate;
  /** The monotonic clock in milliseconds; tests may pass their own. */
  now?: () => number;
}

/**
 * "Elapsed mm:ss" on the status line (261012-parallel-fetch U4:BR4.6-BR4.9).
 * While the fetch runs the time advances every second here only, so the rest
 * of the status line is not drawn again (NFR5.3), and it is hidden from
 * screen readers so the seconds are not read out (BR4.9, AC2.1.7). When the
 * fetch ends the part is read again, with the final value, in the same
 * update as the end of the fetch.
 */
export function ElapsedTime({ elapsed, t, now }: ElapsedTimeProps) {
  const shown = useElapsed(elapsed, now);
  if (shown === null) {
    return null;
  }
  return (
    <span
      className="status-line-elapsed"
      data-testid="status-line-elapsed"
      aria-hidden={shown.running ? "true" : undefined}
    >
      {t("status.elapsed", { time: formatElapsed(shown.ms) })}
    </span>
  );
}
