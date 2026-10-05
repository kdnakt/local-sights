import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type UIEvent,
} from "react";
import { findRowPosition, type DisplayRow, type RowWindow, type TimeZoneChoice } from "../api";
import { toSingleLine } from "../format";
import { useRowWindow } from "../hooks/useRowWindow";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";
import {
  MAX_SCROLL_HEIGHT,
  ROW_HEIGHT,
  anchorAt,
  preservedScrollTop,
  scrollHeight,
  scrollTopForKey,
  visibleRows,
  type Geometry,
} from "../virtualScroll";

/** Viewport height used until the real one is known. */
const DEFAULT_VIEWPORT_HEIGHT = 400;

export interface LogTableProps {
  /** Number of held events (the core's count). */
  totalCount: number;
  /** Version of the core's timeline; a change means rows moved or went away. */
  timelineVersion: number;
  /** The chosen time zone: names the time column and re-reads the rows. */
  timeZone: TimeZoneChoice;
  t: Translate;
  onError: (error: unknown) => void;
  /** Fixed viewport height in pixels; measured from the element when absent. */
  viewportHeight?: number;
}

/** The scroll position and the list size it was taken with. */
interface ScrollState {
  top: number;
  geometry: Geometry;
}

/** The row to keep in place, identified until the next fetch (BR4.4). */
interface PendingAnchor {
  logStreamName: string;
  sequence: number;
  pixelOffset: number;
}

function rowKey(event: DisplayRow): string {
  return `${event.logStreamName}\u0000${event.sequence}`;
}

/** The held row at `row`, if the last window contains it. */
function rowAt(rowWindow: RowWindow | null, row: number): DisplayRow | undefined {
  if (rowWindow === null) {
    return undefined;
  }
  return rowWindow.rows[row - rowWindow.offset];
}

/**
 * The log list: "time / stream / message" with fixed row height and column
 * widths, the message on one line cut with an ellipsis (U3:BR6.4). Only the
 * rows of the viewport are fetched and drawn; the core holds the events.
 * When the timeline changes while scrolled down, the row at the top stays
 * where it was (BR6.5); at the very top the list stays at the top. The
 * arrow keys, Page Up, Page Down, Home and End scroll it (BR6.8). The time
 * column shows the core's `displayTime` as is and its header names the chosen
 * zone; switching the zone re-reads the rows without moving them (U4:BR3.1,
 * BR3.2, BR3.4).
 */
export function LogTable({
  totalCount,
  timelineVersion,
  timeZone,
  t,
  onError,
  viewportHeight: fixedViewportHeight,
}: LogTableProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const viewportHeight = fixedViewportHeight ?? measuredHeight ?? DEFAULT_VIEWPORT_HEIGHT;

  const geometry = useMemo<Geometry>(
    () => ({
      totalCount,
      rowHeight: ROW_HEIGHT,
      viewportHeight,
      maxScrollHeight: MAX_SCROLL_HEIGHT,
    }),
    [totalCount, viewportHeight],
  );
  const range = visibleRows(scrollTop, geometry);
  const rowWindow = useRowWindow(
    range.firstRow,
    range.rowCount,
    timelineVersion,
    totalCount,
    timeZone,
    onError,
  );

  // Kept for the effects and handlers below; never read while rendering.
  const scrollRef = useRef<ScrollState>({ top: 0, geometry });
  const windowRef = useRef<RowWindow | null>(null);
  const geometryRef = useRef(geometry);
  const positionRequest = useRef(0);
  const anchorRef = useRef<PendingAnchor | null>(null);

  useEffect(() => {
    windowRef.current = rowWindow;
  }, [rowWindow]);
  useEffect(() => {
    geometryRef.current = geometry;
  }, [geometry]);

  useEffect(() => {
    const element = viewportRef.current;
    if (fixedViewportHeight !== undefined || element === null) {
      return undefined;
    }
    if (typeof ResizeObserver === "undefined") {
      return undefined;
    }
    const observer = new ResizeObserver(() => setMeasuredHeight(element.clientHeight));
    observer.observe(element);
    return () => observer.disconnect();
  }, [fixedViewportHeight]);

  const applyScrollTop = useCallback((top: number, scrolledGeometry: Geometry) => {
    scrollRef.current = { top, geometry: scrolledGeometry };
    if (viewportRef.current !== null) {
      viewportRef.current.scrollTop = top;
    }
    setScrollTop(top);
  }, []);

  // BR6.5: keep the top row in place when the timeline changes. The anchor
  // row is taken once, from the scroll position the user left, and kept in
  // `anchorRef` until a reply to `find_row_position` has been applied. When
  // another version arrives before that reply, the same anchor is asked for
  // again, so the anchor never drifts to a row that moved in meanwhile
  // (review R-01).
  useEffect(() => {
    positionRequest.current += 1;
    if (geometryRef.current.totalCount === 0) {
      // BR4.5: the logs were discarded; start again from the top.
      anchorRef.current = null;
      if (scrollRef.current.top !== 0) {
        applyScrollTop(0, geometryRef.current);
      }
      return;
    }
    if (anchorRef.current === null) {
      const { top, geometry: scrolledGeometry } = scrollRef.current;
      const anchor = anchorAt(top, scrolledGeometry);
      const anchored = anchor === null ? undefined : rowAt(windowRef.current, anchor.row);
      if (anchor === null || anchored === undefined) {
        return;
      }
      anchorRef.current = {
        logStreamName: anchored.logStreamName,
        sequence: anchored.sequence,
        pixelOffset: anchor.pixelOffset,
      };
    }
    const anchor = anchorRef.current;
    const request = positionRequest.current;
    findRowPosition(anchor.logStreamName, anchor.sequence).then(
      (answer) => {
        if (request !== positionRequest.current) {
          return;
        }
        anchorRef.current = null;
        const current = geometryRef.current;
        applyScrollTop(preservedScrollTop(answer.position, anchor.pixelOffset, current), current);
      },
      (error: unknown) => {
        if (request === positionRequest.current) {
          anchorRef.current = null;
        }
        onError(error);
      },
    );
  }, [timelineVersion, applyScrollTop, onError]);

  /** The user moved the list: a pending anchor no longer applies. */
  const dropPendingAnchor = () => {
    anchorRef.current = null;
    positionRequest.current += 1;
  };

  const handleScroll = (event: UIEvent<HTMLDivElement>) => {
    const top = event.currentTarget.scrollTop;
    if (top === scrollRef.current.top) {
      // The echo of a scroll position set here; nothing changed.
      return;
    }
    dropPendingAnchor();
    scrollRef.current = { top, geometry };
    setScrollTop(top);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const top = scrollTopForKey(event.key, scrollTop, geometry);
    if (top === null) {
      return;
    }
    event.preventDefault();
    dropPendingAnchor();
    applyScrollTop(top, geometry);
  };

  const rows: Array<{ event: DisplayRow; top: number }> = [];
  if (rowWindow !== null) {
    rowWindow.rows.forEach((event, index) => {
      const row = rowWindow.offset + index;
      if (row >= range.firstRow && row < range.firstRow + range.rowCount) {
        rows.push({ event, top: range.offsetY + (row - range.firstRow) * ROW_HEIGHT });
      }
    });
  }

  return (
    <div
      className="log-table"
      role="table"
      aria-label={t("table.label")}
      aria-rowcount={totalCount}
      data-testid="log-table"
    >
      <div className="log-table-header" role="rowgroup">
        <div className="log-table-row" role="row">
          <div className="log-table-time" role="columnheader">
            {t("table.time", { zone: t(timeZoneLabelKey(timeZone)) })}
          </div>
          <div className="log-table-stream" role="columnheader">
            {t("table.stream")}
          </div>
          <div className="log-table-message" role="columnheader">
            {t("table.message")}
          </div>
        </div>
      </div>
      <div
        ref={viewportRef}
        className="log-table-viewport"
        style={fixedViewportHeight === undefined ? undefined : { height: fixedViewportHeight }}
        tabIndex={0}
        aria-label={t("table.scroll")}
        data-testid="log-table-viewport"
        onScroll={handleScroll}
        onKeyDown={handleKeyDown}
      >
        <div
          className="log-table-content"
          role="rowgroup"
          style={{ height: scrollHeight(geometry) }}
        >
          {rows.map(({ event, top }) => (
            <div
              key={rowKey(event)}
              className="log-table-row"
              role="row"
              style={{ top, height: ROW_HEIGHT }}
              data-testid="log-table-row"
            >
              <div className="log-table-time" role="cell">
                {event.displayTime}
              </div>
              <div className="log-table-stream" role="cell" title={event.logStreamName}>
                {event.logStreamName}
              </div>
              <div className="log-table-message" role="cell">
                {toSingleLine(event.message)}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
