import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type UIEvent,
} from "react";
import type { DisplayRow, RowWindow, TimeZoneChoice } from "../api";
import { toSingleLine } from "../format";
import {
  initialExpansionState,
  moveSelection,
  rowKeyOf,
  toggleRow,
  withDiscardGeneration,
  withPositions,
  withSelectedKey,
  type ExpansionState,
  type RowKey,
} from "../hooks/expansionState";
import { useRowLayout } from "../hooks/useRowLayout";
import { useRowPositions, type RowPositionsAnswer } from "../hooks/useRowPositions";
import { useRowWindow } from "../hooks/useRowWindow";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";
import {
  ROW_HEIGHT,
  anchorAt,
  rowHeightAt,
  rowOffset,
  scrollHeight,
  scrollTopForAnchor,
  scrollTopToReveal,
  visibleRange,
  type Layout,
} from "../rowLayout";
import { ExpandedMessage } from "./ExpandedMessage";

/** Viewport size used until the real one is known. */
const DEFAULT_VIEWPORT_HEIGHT = 400;
const DEFAULT_VIEWPORT_WIDTH = 800;

export interface LogTableProps {
  /**
   * Number of rows: the held events, or the matching events while a log
   * filter is in force (U5:BR3.1).
   */
  totalCount: number;
  /** Version of the core's timeline; a change means rows moved or went away. */
  timelineVersion: number;
  /** The chosen time zone: names the time column and re-reads the rows. */
  timeZone: TimeZoneChoice;
  /** The log filter in force, `null` without one; a change goes to the top (U5:BR3.2). */
  filterId?: number | null;
  /** The filter result version; a change re-reads the rows (U5:BR3.6). */
  resultVersion?: number;
  /** Advances when the held logs were discarded: every expansion closes (U7:BR1.6). */
  discardGeneration?: number;
  t: Translate;
  onError: (error: unknown) => void;
  /** Fixed viewport height in pixels; measured from the element when absent. */
  viewportHeight?: number;
  /** Fixed list width in pixels; measured from the element when absent. */
  viewportWidth?: number;
  /** Width of one monospace character; measured once when absent. */
  charWidth?: number;
}

/** The scroll position and the layout it was taken with. */
interface ScrollState {
  top: number;
  layout: Layout;
}

/** The row to keep at the top until its new position is known (BR1.8). */
interface PendingAnchor {
  key: RowKey;
  position: number;
  offsetPx: number;
}

/** The layout last drawn and the versions it belonged to. */
interface DrawnLayout {
  layout: Layout;
  versions: string;
}

function keyOf(event: DisplayRow): RowKey {
  return rowKeyOf(event.logStreamName, event.sequence);
}

/** The loaded row at `position`, if the last window contains it. */
function loadedRow(rowWindow: RowWindow | null, position: number): DisplayRow | undefined {
  if (rowWindow === null) {
    return undefined;
  }
  return rowWindow.rows[position - rowWindow.offset];
}

function rowElementId(position: number): string {
  return `log-table-row-${position}`;
}

/**
 * The log list (U3:BR6.4, U7:BR1.1-BR1.10): "time / stream / message", the
 * message on one line, with a grid role. Pressing a row's line (or Enter or
 * Space on the selected row) opens the whole message right below it, and
 * again closes it; several rows can be open (BR1.1, BR1.2). The arrow keys,
 * Page Up, Page Down, Home and End move the selected row and scroll it into
 * view (BR1.5, replacing U3's scrolling keys). Only the rows of the viewport
 * are fetched and drawn; the core holds the events. Row heights vary with
 * the expansions (`rowLayout`), and the row at the top stays where it was
 * when rows are opened, closed, measured or moved (BR1.8, U3:BR6.5). The
 * positions of the expanded, selected and top rows are asked from the core
 * in one call when the timeline, the filter or its result change (BR1.7);
 * expansions stay open across a filter, a time zone switch and added rows,
 * and all close when the held logs are discarded (BR1.6). A new filter (or
 * none) goes back to the top (U5:BR3.2).
 */
export function LogTable({
  totalCount,
  timelineVersion,
  timeZone,
  filterId = null,
  resultVersion = 0,
  discardGeneration = 0,
  t,
  onError,
  viewportHeight: fixedViewportHeight,
  viewportWidth: fixedViewportWidth,
  charWidth,
}: LogTableProps) {
  const gridRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const [measuredSize, setMeasuredSize] = useState<{ height: number; width: number } | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [expansion, setExpansion] = useState<ExpansionState>(() =>
    initialExpansionState(discardGeneration),
  );
  const [positionsSeen, setPositionsSeen] = useState(0);
  const viewportHeight = fixedViewportHeight ?? measuredSize?.height ?? DEFAULT_VIEWPORT_HEIGHT;
  const listWidth = fixedViewportWidth ?? measuredSize?.width ?? DEFAULT_VIEWPORT_WIDTH;
  const versions = `${timelineVersion}/${resultVersion}/${filterId}`;

  const rowLayout = useRowLayout({
    rowCount: totalCount,
    expanded: expansion.expanded,
    listWidth,
    charWidth,
  });
  const { layout, forget: forgetRows } = rowLayout;
  const range = visibleRange(layout, scrollTop, viewportHeight);
  const rowWindow = useRowWindow({
    firstRow: range.first,
    rowCount: range.count,
    timelineVersion,
    totalCount,
    timeZone,
    filterId,
    resultVersion,
    onError,
  });

  // Kept for the effects and handlers below; never read while rendering.
  const scrollRef = useRef<ScrollState>({ top: 0, layout });
  const drawnRef = useRef<DrawnLayout>({ layout, versions });
  const windowRef = useRef<RowWindow | null>(null);
  const expansionRef = useRef(expansion);
  const anchorRef = useRef<PendingAnchor | null>(null);
  const pendingScrollRef = useRef<{ position: number; offsetPx: number } | null>(null);
  const filterRef = useRef(filterId);
  const generationRef = useRef(discardGeneration);
  const currentRef = useRef({ totalCount, viewportHeight, timelineVersion, resultVersion });

  useEffect(() => {
    currentRef.current = { totalCount, viewportHeight, timelineVersion, resultVersion };
  }, [totalCount, viewportHeight, timelineVersion, resultVersion]);
  useEffect(() => {
    windowRef.current = rowWindow;
  }, [rowWindow]);
  useEffect(() => {
    expansionRef.current = expansion;
  }, [expansion]);

  useEffect(() => {
    const element = viewportRef.current;
    if (
      (fixedViewportHeight !== undefined && fixedViewportWidth !== undefined) ||
      element === null ||
      typeof ResizeObserver === "undefined"
    ) {
      return undefined;
    }
    const observer = new ResizeObserver(() =>
      setMeasuredSize({ height: element.clientHeight, width: element.clientWidth }),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [fixedViewportHeight, fixedViewportWidth]);

  const applyScrollTop = useCallback((top: number, scrolledLayout: Layout) => {
    scrollRef.current = { top, layout: scrolledLayout };
    if (viewportRef.current !== null) {
      viewportRef.current.scrollTop = top;
    }
    setScrollTop(top);
  }, []);

  /** BR1.6: the logs were discarded: nothing open, top row selected, at the top. */
  const startOver = useCallback(
    (generation: number) => {
      generationRef.current = generation;
      anchorRef.current = null;
      pendingScrollRef.current = null;
      forgetRows(null);
      setExpansion((state) => withDiscardGeneration(state, generation));
    },
    [forgetRows],
  );

  useEffect(() => {
    if (discardGeneration > generationRef.current) {
      startOver(discardGeneration);
    }
  }, [discardGeneration, startOver]);

  const keyAt = useCallback((position: number) => {
    const row = loadedRow(windowRef.current, position);
    return row === undefined ? undefined : keyOf(row);
  }, []);

  const handleAnswer = useCallback(
    (answer: RowPositionsAnswer) => {
      if (answer.discardGeneration > generationRef.current) {
        startOver(answer.discardGeneration);
        return;
      }
      setExpansion((state) =>
        withPositions(withSelectedKey(state, keyAt), answer.positions, answer.totalCount),
      );
      // An answer for older rows than the screen now shows leaves the top row
      // pending: the request already made for the newer rows will place it.
      const current = currentRef.current;
      const latest =
        answer.timelineVersion >= current.timelineVersion &&
        answer.resultVersion >= current.resultVersion;
      const anchor = anchorRef.current;
      if (anchor !== null && latest) {
        anchorRef.current = null;
        const found = answer.positions.get(anchor.key);
        // BR1.8: gone from the list: the row now at the same position.
        const position = Math.min(found ?? anchor.position, Math.max(0, answer.totalCount - 1));
        pendingScrollRef.current = { position, offsetPx: found == null ? 0 : anchor.offsetPx };
      }
      setPositionsSeen((count) => count + 1);
    },
    [keyAt, startOver],
  );

  const getKeys = useCallback((): RowKey[] => {
    const state = expansionRef.current;
    const keys = [...state.expanded.keys()];
    // BR1.5: the selected row's key, found from the loaded rows when not known yet.
    const selectedKey = state.selection.key ?? keyAt(state.selection.position);
    if (selectedKey !== undefined) {
      keys.push(selectedKey);
    }
    if (anchorRef.current !== null) {
      keys.push(anchorRef.current.key);
    }
    return keys;
  }, [keyAt]);

  const requestPositions = useRowPositions({ getKeys, onAnswer: handleAnswer, onError });

  // BR1.7, BR1.8 (U3:BR6.5): the rows moved. The top row is taken once,
  // from the scroll position the user left, and kept until an answer was
  // applied, so it never drifts to a row that moved in meanwhile. A new
  // filter, or none, starts again from the top (U5:BR3.2).
  useEffect(() => {
    const filterChanged = filterRef.current !== filterId;
    filterRef.current = filterId;
    const { totalCount: count, viewportHeight: height } = currentRef.current;
    if (filterChanged || count === 0) {
      anchorRef.current = null;
      pendingScrollRef.current = null;
      if (scrollRef.current.top !== 0) {
        applyScrollTop(0, scrollRef.current.layout);
      }
    } else if (anchorRef.current === null) {
      const { top, layout: scrolledLayout } = scrollRef.current;
      const anchor = anchorAt(scrolledLayout, top, height);
      const anchored = anchor === null ? undefined : loadedRow(windowRef.current, anchor.position);
      if (anchor !== null && anchored !== undefined) {
        anchorRef.current = {
          key: keyOf(anchored),
          position: anchor.position,
          offsetPx: anchor.offsetPx,
        };
      }
    }
    requestPositions();
  }, [timelineVersion, resultVersion, filterId, applyScrollTop, requestPositions]);

  // BR1.8: after the layout changed, keep the row at the top where it was:
  // at its new position from the core, or (same rows, only expansions and
  // heights changed) at its position in the new layout.
  useLayoutEffect(() => {
    const drawn = drawnRef.current;
    drawnRef.current = { layout, versions };
    const pending = pendingScrollRef.current;
    if (pending !== null) {
      pendingScrollRef.current = null;
      applyScrollTop(
        scrollTopForAnchor(layout, pending.position, pending.offsetPx, viewportHeight),
        layout,
      );
      return;
    }
    const { top } = scrollRef.current;
    const sameRows = drawn.versions === versions && drawn.layout.rowCount === layout.rowCount;
    if (!sameRows || drawn.layout === layout) {
      return;
    }
    const anchor = anchorAt(drawn.layout, top, viewportHeight);
    if (anchor === null) {
      scrollRef.current = { top, layout };
      return;
    }
    const offsetPx = Math.min(anchor.offsetPx, rowHeightAt(layout, anchor.position) - 1);
    applyScrollTop(
      scrollTopForAnchor(layout, anchor.position, Math.max(0, offsetPx), viewportHeight),
      layout,
    );
  }, [layout, versions, positionsSeen, viewportHeight, applyScrollTop]);

  /** The user moved the list: a pending anchor no longer applies. */
  const dropPendingAnchor = () => {
    anchorRef.current = null;
    pendingScrollRef.current = null;
  };

  const handleScroll = (event: UIEvent<HTMLDivElement>) => {
    const top = event.currentTarget.scrollTop;
    if (top === scrollRef.current.top) {
      // The echo of a scroll position set here; nothing changed.
      return;
    }
    dropPendingAnchor();
    scrollRef.current = { top, layout };
    setScrollTop(top);
  };

  /** BR1.1: opens or closes the row and selects it. */
  const toggle = (event: DisplayRow, position: number) => {
    const key = keyOf(event);
    if (expansion.expanded.has(key)) {
      forgetRows(key);
    } else {
      rowLayout.remember(key, event.message);
    }
    setExpansion((state) => toggleRow(state, key, position));
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // BR1.10: keys inside an expansion are its own (selection, scrolling).
    if (event.target !== event.currentTarget) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const position = expansion.selection.position;
      const row = loadedRow(rowWindow, position);
      if (row !== undefined && totalCount > 0) {
        toggle(row, position);
      }
      return;
    }
    const pageRows = Math.max(1, Math.floor(viewportHeight / ROW_HEIGHT));
    const moved = moveSelection(expansion, event.key, totalCount, pageRows, keyAt);
    if (moved === null) {
      return;
    }
    event.preventDefault();
    dropPendingAnchor();
    setExpansion(moved);
    applyScrollTop(
      scrollTopToReveal(layout, moved.selection.position, scrollTop, viewportHeight),
      layout,
    );
  };

  const focusGrid = () => gridRef.current?.focus({ preventScroll: true });

  const rows: Array<{ event: DisplayRow; position: number; top: number }> = [];
  if (rowWindow !== null && totalCount > 0) {
    const firstTop = rowOffset(layout, range.first);
    for (let position = range.first; position < range.first + range.count; position += 1) {
      const event = loadedRow(rowWindow, position);
      if (event !== undefined) {
        rows.push({ event, position, top: range.offsetY + rowOffset(layout, position) - firstTop });
      }
    }
  }
  const selected = expansion.selection;
  const selectedDrawn = rows.some(({ position }) => position === selected.position);

  const contentStyle = useMemo(() => ({ height: scrollHeight(layout) }), [layout]);

  return (
    <div
      ref={gridRef}
      className="log-table"
      role="grid"
      aria-label={t("table.label")}
      aria-rowcount={totalCount + 1}
      aria-colcount={3}
      aria-activedescendant={
        selectedDrawn && totalCount > 0 ? rowElementId(selected.position) : undefined
      }
      tabIndex={0}
      data-testid="log-table"
      onKeyDown={handleKeyDown}
    >
      <div className="log-table-header" role="rowgroup">
        <div className="log-table-row log-table-line" role="row" aria-rowindex={1}>
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
        style={
          fixedViewportHeight === undefined
            ? undefined
            : { height: fixedViewportHeight, width: fixedViewportWidth }
        }
        data-testid="log-table-viewport"
        onScroll={handleScroll}
      >
        <div className="log-table-content" role="rowgroup" style={contentStyle}>
          {rows.map(({ event, position, top }) => {
            const key = keyOf(event);
            const expanded = expansion.expanded.has(key);
            const isSelected = selected.position === position;
            return (
              <div
                key={key}
                id={rowElementId(position)}
                className="log-table-row"
                role="row"
                aria-rowindex={position + 2}
                aria-expanded={expanded}
                aria-selected={isSelected}
                style={{ top }}
                data-testid="log-table-row"
              >
                <div
                  className="log-table-line"
                  role="none"
                  data-testid="log-table-row-line"
                  onClick={() => {
                    toggle(event, position);
                    focusGrid();
                  }}
                >
                  <div className="log-table-time" role="gridcell">
                    {event.displayTime}
                  </div>
                  <div className="log-table-stream" role="gridcell" title={event.logStreamName}>
                    {event.logStreamName}
                  </div>
                  <div className="log-table-message" role="gridcell">
                    {toSingleLine(event.message)}
                  </div>
                </div>
                {expanded && (
                  <ExpandedMessage
                    message={event.message}
                    focusable={isSelected && rowLayout.scrolls(key)}
                    label={t("table.expanded")}
                    onMeasure={(height, scrolls) => rowLayout.measure(key, height, scrolls)}
                    onLeave={focusGrid}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
