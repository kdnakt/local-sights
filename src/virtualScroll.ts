/**
 * U3's fixed-height view of the log list (U3:BR6.4, BR6.5, BR6.8), now
 * computed by `rowLayout.ts` with no row expanded (U7:BR1.4, review R-12).
 *
 * The log table itself uses `rowLayout.ts` directly since U7; this module
 * keeps U3's functions and answers so that U3's tests keep checking the
 * new calculation without expansions. U3's `preservedScrollTop` and
 * `scrollTopForKey` were removed at the code generation review (R-05): the
 * list keeps its top row with `scrollTopForAnchor` (BR1.8) and the keys
 * move the selection (BR1.5, `expansionState.moveSelection`). Every
 * function here is pure.
 */

import {
  anchorAt as layoutAnchorAt,
  buildLayout,
  maxScrollTop as layoutMaxScrollTop,
  scrollHeight as layoutScrollHeight,
  scrollTopForAnchor,
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
