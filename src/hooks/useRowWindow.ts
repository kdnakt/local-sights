import { useEffect, useRef, useState } from "react";
import { getRows, type RowWindow, type TimeZoneChoice } from "../api";

export interface RowWindowRequest {
  firstRow: number;
  rowCount: number;
  timelineVersion: number;
  totalCount: number;
  timeZone: TimeZoneChoice;
  /** The log filter in force, `null` without one (U5:BR3.2). */
  filterId: number | null;
  /** The filter result version the screen knows (U5:BR3.6). */
  resultVersion: number;
  onError: (error: unknown) => void;
}

/**
 * Fetches the rows `[firstRow, firstRow + rowCount)` from the core with
 * `get_rows` whenever the range, the timeline version, the count or the time
 * zone changes (U3:BR4.3, BR6.4). A time zone switch keeps the timeline
 * version, so the zone itself triggers the re-read that brings the rows'
 * new display times (U4:BR3.1, review R-08). Since U5 a change of the log
 * filter or of its result version also triggers a re-read, even when the
 * count stays the same, and an answer taken from an older result version
 * than the one the screen knows is dropped (U5:BR3.6). Each request gets a
 * number and only the answer to the latest request is kept, so a slow, older
 * answer never replaces a newer one. Until the first answer arrives the
 * previous window is returned, so the table does not flash empty while
 * scrolling.
 */
export function useRowWindow({
  firstRow,
  rowCount,
  timelineVersion,
  totalCount,
  timeZone,
  filterId,
  resultVersion,
  onError,
}: RowWindowRequest): RowWindow | null {
  const [rowWindow, setRowWindow] = useState<RowWindow | null>(null);
  const latestRequest = useRef(0);
  const knownResultVersion = useRef(resultVersion);

  useEffect(() => {
    knownResultVersion.current = Math.max(knownResultVersion.current, resultVersion);
    latestRequest.current += 1;
    if (rowCount <= 0 || totalCount <= 0) {
      return;
    }
    const request = latestRequest.current;
    getRows(firstRow, rowCount).then(
      (answer) => {
        const outdated =
          answer.resultVersion !== null && answer.resultVersion < knownResultVersion.current;
        if (request === latestRequest.current && !outdated) {
          setRowWindow(answer);
        }
      },
      (error: unknown) => {
        if (request === latestRequest.current) {
          onError(error);
        }
      },
    );
  }, [firstRow, rowCount, timelineVersion, totalCount, timeZone, filterId, resultVersion, onError]);

  return rowCount <= 0 || totalCount <= 0 ? null : rowWindow;
}
