/**
 * U3's fixed-height view of the log list (U3:BR6.4, BR6.5, BR6.8), now
 * computed by `rowLayout.ts` with no row expanded (U7:BR1.4, review R-12).
 *
 * The log table itself uses `rowLayout.ts` directly since U7; this module
 * keeps U3's functions and answers so that U3's tests keep checking the
 * new calculation without expansions. Every function here is pure.
 */

import {
  anchorAt as layoutAnchorAt,
  buildLayout,
  maxScrollTop as layoutMaxScrollTop,
  scrollHeight as layoutScrollHeight,
  scrollTopForAnchor,
  toScrollTop,
  toVirtualTop,
  visibleRange,
  type Layout,
} from "./rowLayout";

export { MAX_SCROLL_HEIGHT, ROW_HEIGHT } from "./rowLayout";

export interface Geometry {
  totalCount: number;
  rowHeight: number;
  viewportHeight: number;
  maxScrollHeight: number;
}

/** Rows to draw for one scroll position. */
export interface RowRange {
  /** Position of the first row to draw. */
  firstRow: number;
  /** Number of rows to draw from `firstRow`. */
  rowCount: number;
  /** Top of the first row inside the scrolled area, in pixels. */
  offsetY: number;
}

/** The row at the top of the viewport and how far it is scrolled past. */
export interface Anchor {
  row: number;
  pixelOffset: number;
}

const SCROLL_KEYS = ["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End"] as const;

function layoutOf(g: Geometry): Layout {
  return buildLayout(g.totalCount, [], {
    rowHeight: g.rowHeight,
    maxScrollHeight: g.maxScrollHeight,
  });
}

/** Height of the scrolled area: the list's height, capped. */
export function scrollHeight(g: Geometry): number {
  return layoutScrollHeight(layoutOf(g));
}

/** Highest scroll position. */
export function maxScrollTop(g: Geometry): number {
  return layoutMaxScrollTop(layoutOf(g), g.viewportHeight);
}

/** The rows that cover the viewport at `scrollTop`, and where to draw them. */
export function visibleRows(scrollTop: number, g: Geometry): RowRange {
  const range = visibleRange(layoutOf(g), scrollTop, g.viewportHeight);
  return { firstRow: range.first, rowCount: range.count, offsetY: range.offsetY };
}

/** Scroll position that shows `row` at the top, scrolled `pixelOffset` past. */
export function scrollTopForRow(row: number, pixelOffset: number, g: Geometry): number {
  return scrollTopForAnchor(layoutOf(g), row, pixelOffset, g.viewportHeight);
}

/** The row at the top at `scrollTop`; `null` when scrolled to the very top. */
export function anchorAt(scrollTop: number, g: Geometry): Anchor | null {
  const anchor = layoutAnchorAt(layoutOf(g), scrollTop, g.viewportHeight);
  return anchor === null ? null : { row: anchor.position, pixelOffset: anchor.offsetPx };
}

/**
 * U3:BR6.5: after the list changed, the scroll position that keeps the
 * anchored row where it was; the top when there was no anchor or the row is
 * gone.
 */
export function preservedScrollTop(
  newPosition: number | null,
  pixelOffset: number,
  g: Geometry,
): number {
  if (newPosition === null) {
    return 0;
  }
  return scrollTopForRow(newPosition, pixelOffset, g);
}

/**
 * U3:BR6.8: the scroll position after a key press (arrows: one row, Page Up
 * and Page Down: one viewport, Home and End: either end); `null` for other
 * keys. U7:BR1.5 replaced these keys in the log table with row selection.
 */
export function scrollTopForKey(key: string, scrollTop: number, g: Geometry): number | null {
  if (!(SCROLL_KEYS as readonly string[]).includes(key)) {
    return null;
  }
  const layout = layoutOf(g);
  if (key === "Home") {
    return 0;
  }
  if (key === "End") {
    return layoutMaxScrollTop(layout, g.viewportHeight);
  }
  const step = key.startsWith("Page") ? g.viewportHeight : g.rowHeight;
  const direction = key === "ArrowUp" || key === "PageUp" ? -1 : 1;
  const virtualTop = toVirtualTop(layout, scrollTop, g.viewportHeight);
  return toScrollTop(layout, virtualTop + direction * step, g.viewportHeight);
}
