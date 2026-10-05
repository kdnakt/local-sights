import { useEffect, useRef, useState } from "react";
import { getRows, type RowWindow, type TimeZoneChoice } from "../api";

/**
 * Fetches the rows `[firstRow, firstRow + rowCount)` from the core with
 * `get_rows` whenever the range, the timeline version, the count or the time
 * zone changes (U3:BR4.3, BR6.4). A time zone switch keeps the timeline
 * version, so the zone itself triggers the re-read that brings the rows'
 * new display times (U4:BR3.1, review R-08). Each request gets a number and only the answer to the
 * latest request is kept, so a slow, older answer never replaces a newer one.
 * Until the first answer arrives the previous window is returned, so the
 * table does not flash empty while scrolling.
 */
export function useRowWindow(
  firstRow: number,
  rowCount: number,
  timelineVersion: number,
  totalCount: number,
  timeZone: TimeZoneChoice,
  onError: (error: unknown) => void,
): RowWindow | null {
  const [rowWindow, setRowWindow] = useState<RowWindow | null>(null);
  const latestRequest = useRef(0);

  useEffect(() => {
    if (rowCount <= 0 || totalCount <= 0) {
      latestRequest.current += 1;
      return;
    }
    latestRequest.current += 1;
    const request = latestRequest.current;
    getRows(firstRow, rowCount).then(
      (answer) => {
        if (request === latestRequest.current) {
          setRowWindow(answer);
        }
      },
      (error: unknown) => {
        if (request === latestRequest.current) {
          onError(error);
        }
      },
    );
  }, [firstRow, rowCount, timelineVersion, totalCount, timeZone, onError]);

  return rowCount <= 0 || totalCount <= 0 ? null : rowWindow;
}
