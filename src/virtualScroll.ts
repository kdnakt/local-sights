/**
 * Virtual scrolling of the log table (U3:BR6.4, BR6.5, Q1: own code).
 *
 * Rows have a fixed height. The scrolled area is `totalCount × rowHeight`
 * high, but browsers cap the height of an element, so above
 * `maxScrollHeight` the scroll position is mapped proportionally onto the
 * "virtual" position of the full list. Every function here is pure.
 */

/** Height of one row, in pixels (fixed, BR6.4). */
export const ROW_HEIGHT = 22;

/** Highest scrolled area used, below every browser's element height limit. */
export const MAX_SCROLL_HEIGHT = 10_000_000;

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

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), Math.max(low, high));
}

/** Height of the full list. */
function virtualHeight(g: Geometry): number {
  return Math.max(0, g.totalCount) * g.rowHeight;
}

function maxVirtualTop(g: Geometry): number {
  return Math.max(0, virtualHeight(g) - g.viewportHeight);
}

/** Height of the scrolled area: the list's height, capped. */
export function scrollHeight(g: Geometry): number {
  return Math.min(virtualHeight(g), g.maxScrollHeight);
}

/** Highest scroll position. */
export function maxScrollTop(g: Geometry): number {
  return Math.max(0, scrollHeight(g) - g.viewportHeight);
}

function toVirtual(scrollTop: number, g: Geometry): number {
  const maxTop = maxScrollTop(g);
  if (maxTop <= 0) {
    return 0;
  }
  return (clamp(scrollTop, 0, maxTop) * maxVirtualTop(g)) / maxTop;
}

function fromVirtual(virtualTop: number, g: Geometry): number {
  const maxVirtual = maxVirtualTop(g);
  if (maxVirtual <= 0) {
    return 0;
  }
  return (clamp(virtualTop, 0, maxVirtual) * maxScrollTop(g)) / maxVirtual;
}

/** The rows that cover the viewport at `scrollTop`, and where to draw them. */
export function visibleRows(scrollTop: number, g: Geometry): RowRange {
  if (g.totalCount <= 0) {
    return { firstRow: 0, rowCount: 0, offsetY: 0 };
  }
  const top = clamp(scrollTop, 0, maxScrollTop(g));
  const virtualTop = toVirtual(top, g);
  const firstRow = Math.min(Math.floor(virtualTop / g.rowHeight), g.totalCount - 1);
  const rowsInView = Math.ceil(g.viewportHeight / g.rowHeight) + 1;
  const rowCount = Math.min(g.totalCount - firstRow, rowsInView);
  const offsetY = top - (virtualTop - firstRow * g.rowHeight);
  return { firstRow, rowCount, offsetY };
}

/** Scroll position that shows `row` at the top, scrolled `pixelOffset` past. */
export function scrollTopForRow(row: number, pixelOffset: number, g: Geometry): number {
  return fromVirtual(row * g.rowHeight + pixelOffset, g);
}

/** The row at the top at `scrollTop`; `null` when scrolled to the very top. */
export function anchorAt(scrollTop: number, g: Geometry): Anchor | null {
  if (scrollTop <= 0 || g.totalCount <= 0) {
    return null;
  }
  const virtualTop = toVirtual(scrollTop, g);
  const row = Math.min(Math.floor(virtualTop / g.rowHeight), g.totalCount - 1);
  return { row, pixelOffset: virtualTop - row * g.rowHeight };
}

/**
 * BR6.5: after the list changed, the scroll position that keeps the anchored
 * row where it was. `newPosition` is the anchored row's new position, or
 * `null` when there was no anchor (scrolled to the top) or the row is gone:
 * then the list shows its top.
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
 * BR6.8: the scroll position after a key press (arrows: one row, Page Up and
 * Page Down: one viewport, Home and End: either end); `null` for other keys.
 */
export function scrollTopForKey(key: string, scrollTop: number, g: Geometry): number | null {
  if (!(SCROLL_KEYS as readonly string[]).includes(key)) {
    return null;
  }
  if (key === "Home") {
    return 0;
  }
  if (key === "End") {
    return maxScrollTop(g);
  }
  const step = key.startsWith("Page") ? g.viewportHeight : g.rowHeight;
  const direction = key === "ArrowUp" || key === "PageUp" ? -1 : 1;
  return fromVirtual(toVirtual(scrollTop, g) + direction * step, g);
}
