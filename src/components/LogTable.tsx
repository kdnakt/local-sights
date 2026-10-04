import type { LogEvent } from "../api";
import { formatUtcMillis, toSingleLine } from "../format";
import type { Translate } from "../i18n/messages";

export interface LogTableProps {
  events: readonly LogEvent[];
  t: Translate;
}

/**
 * Every fetched event, oldest first as delivered by the core (BR5.1): time in
 * UTC with milliseconds (BR5.3) and the message on one line, cut with an
 * ellipsis when it does not fit (BR5.2). U1 renders all rows; virtual
 * scrolling arrives in a later unit.
 */
export function LogTable({ events, t }: LogTableProps) {
  return (
    <div className="log-table-container">
      <table className="log-table" aria-label={t("table.label")} data-testid="log-table">
        <colgroup>
          <col className="log-table-time-column" />
          <col />
        </colgroup>
        <thead>
          <tr>
            <th scope="col">{t("table.time")}</th>
            <th scope="col">{t("table.message")}</th>
          </tr>
        </thead>
        <tbody>
          {events.map((event) => (
            <tr key={`${event.logStreamName}:${event.sequence}`} data-testid="log-table-row">
              <td className="log-table-time">{formatUtcMillis(event.timestamp)}</td>
              <td className="log-table-message">{toSingleLine(event.message)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
