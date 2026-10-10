/**
 * Row positions of the log list with expanded rows (U7:BR1.4, BR1.8, BR1.9).
 *
 * Every row has a 22 px line; an expanded row also has its expansion below
 * the line. The virtual top of row `i` is `i × 22` plus the heights of the
 * expansions above it, found by binary search over their cumulative
 * heights. Browsers cap the height of an element, so above
 * `MAX_SCROLL_HEIGHT` the scroll position is mapped onto the virtual
 * position with U3's normalization (review R-12): the highest virtual top
 * (`H − viewport`) maps to the highest scroll position
 * (`min(H, MAX_SCROLL_HEIGHT) − viewport`). Without expansions every answer is
 * the one of U3's fixed-height list (`virtualScroll.ts`). Every function here
 * is pure.
 */

/** Height of one row's line, in pixels (U3:BR6.4). */
export const ROW_HEIGHT = 22;
/** Line height of an expansion's text (BR1.2). */
export const EXPANDED_LINE_HEIGHT = 18;
/** Top and bottom padding of an expansion together (6 px each, BR1.2). */
export const EXPANDED_PADDING = 12;
/** Lines of an expansion shown before it scrolls inside (BR1.2, Q1). */
export const EXPANDED_MAX_LINES = 20;
/** Highest expansion: 20 lines and the padding, 372 px (BR1.2). */
export const EXPANDED_MAX_HEIGHT = EXPANDED_LINE_HEIGHT * EXPANDED_MAX_LINES + EXPANDED_PADDING;
/** Highest scrolled area used, below every browser's element height limit. */
export const MAX_SCROLL_HEIGHT = 10_000_000;

/** One expanded row that is in the list, and the height of its expansion. */
export interface Expansion {
  position: number;
  height: number;
}

/** The list's rows and expansions, ready for the calculations below. */
export interface Layout {
  rowCount: number;
  rowHeight: number;
  maxScrollHeight: number;
  /** Expansions inside the list, by ascending position, one per position. */
  expansions: readonly Expansion[];
  /** `cumulative[k]`: the heights of `expansions[0..k)` added up. */
  cumulative: readonly number[];
}

export interface LayoutOptions {
  rowHeight?: number;
  maxScrollHeight?: number;
}

/** The rows to draw for one scroll position. */
export interface RowRange {
  /** Position of the first row to draw. */
  first: number;
  /** Number of rows to draw from `first`. */
  count: number;
  /** Top of the first row inside the scrolled area, in pixels. */
  offsetY: number;
}

/** A row and how far into it (line and expansion) a y lies. */
export interface RowPoint {
  position: number;
  offsetPx: number;
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), Math.max(low, high));
}

/**
 * The layout of `rowCount` rows with `expansions`. Expansions outside the
 * list are left out; for a position given twice the last one counts.
 */
export function buildLayout(
  rowCount: number,
  expansions: readonly Expansion[],
  options: LayoutOptions = {},
): Layout {
  const count = Math.max(0, rowCount);
  const byPosition = new Map<number, number>();
  for (const { position, height } of expansions) {
    if (Number.isInteger(position) && position >= 0 && position < count) {
      byPosition.set(position, Math.max(0, height));
    }
  }
  const sorted = [...byPosition]
    .map(([position, height]) => ({ position, height }))
    .sort((a, b) => a.position - b.position);
  const cumulative = [0];
  for (const { height } of sorted) {
    cumulative.push((cumulative.at(-1) ?? 0) + height);
  }
  return {
    rowCount: count,
    rowHeight: options.rowHeight ?? ROW_HEIGHT,
    maxScrollHeight: options.maxScrollHeight ?? MAX_SCROLL_HEIGHT,
    expansions: sorted,
    cumulative,
  };
}

/** Number of expansions whose position is below `position`. */
function expansionsBefore(layout: Layout, position: number): number {
  let low = 0;
  let high = layout.expansions.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if ((layout.expansions[middle]?.position ?? Infinity) < position) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low;
}

/** Height of the full list: every line and every expansion. */
export function virtualHeight(layout: Layout): number {
  return layout.rowCount * layout.rowHeight + (layout.cumulative.at(-1) ?? 0);
}

/** Height of the scrolled area: the list's height, capped. */
export function scrollHeight(layout: Layout): number {
  return Math.min(virtualHeight(layout), layout.maxScrollHeight);
}

/** Highest scroll position. */
export function maxScrollTop(layout: Layout, viewport: number): number {
  return Math.max(0, scrollHeight(layout) - viewport);
}

function maxVirtualTop(layout: Layout, viewport: number): number {
  return Math.max(0, virtualHeight(layout) - viewport);
}

/** Virtual top of `position`: its lines above plus the expansions above. */
export function rowOffset(layout: Layout, position: number): number {
  return position * layout.rowHeight + (layout.cumulative[expansionsBefore(layout, position)] ?? 0);
}

/** Height of the row at `position`: its line, and its expansion if any. */
export function rowHeightAt(layout: Layout, position: number): number {
  const index = expansionsBefore(layout, position);
  const expansion = layout.expansions[index];
  return layout.rowHeight + (expansion?.position === position ? expansion.height : 0);
}

/**
 * The row at virtual `y` and how far into it `y` lies; a `y` inside an
 * expansion belongs to the expanded row (BR1.4). Clamped to the list.
 */
export function rowAt(layout: Layout, virtualY: number): RowPoint {
  if (layout.rowCount <= 0) {
    return { position: 0, offsetPx: 0 };
  }
  const y = Math.max(0, virtualY);
  // The last expanded row whose top is not below y.
  let low = 0;
  let high = layout.expansions.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    const expansion = layout.expansions[middle];
    const top = (expansion?.position ?? 0) * layout.rowHeight + (layout.cumulative[middle] ?? 0);
    if (top <= y) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  const index = low - 1;
  const expansion = layout.expansions[index];
  if (expansion !== undefined) {
    const top = expansion.position * layout.rowHeight + (layout.cumulative[index] ?? 0);
    if (y < top + layout.rowHeight + expansion.height) {
      return { position: expansion.position, offsetPx: y - top };
    }
  }
  const above = layout.cumulative[low] ?? 0;
  const position = Math.min(Math.floor((y - above) / layout.rowHeight), layout.rowCount - 1);
  return { position, offsetPx: y - rowOffset(layout, position) };
}

/** The virtual top shown at scroll position `scrollTop` (R-12). */
export function toVirtualTop(layout: Layout, scrollTop: number, viewport: number): number {
  const maxTop = maxScrollTop(layout, viewport);
  if (maxTop <= 0) {
    return 0;
  }
  return (clamp(scrollTop, 0, maxTop) * maxVirtualTop(layout, viewport)) / maxTop;
}

/** The scroll position that shows virtual top `virtualTop` (R-12). */
export function toScrollTop(layout: Layout, virtualTop: number, viewport: number): number {
  const maxVirtual = maxVirtualTop(layout, viewport);
  if (maxVirtual <= 0) {
    return 0;
  }
  return (clamp(virtualTop, 0, maxVirtual) * maxScrollTop(layout, viewport)) / maxVirtual;
}

/**
 * The rows that cover the viewport at `scrollTop` and where to draw the
 * first one: from the row at the top, rows until the viewport is covered
 * with one row's height to spare (U3 drew `ceil(viewport / rowHeight) + 1`
 * rows).
 */
export function visibleRange(layout: Layout, scrollTop: number, viewport: number): RowRange {
  if (layout.rowCount <= 0) {
    return { first: 0, count: 0, offsetY: 0 };
  }
  const top = clamp(scrollTop, 0, maxScrollTop(layout, viewport));
  const virtualTop = toVirtualTop(layout, top, viewport);
  const { position: first, offsetPx } = rowAt(layout, virtualTop);
  // U3 drew one row's height past the viewport; an expanded top row also
  // needs the part of it scrolled past.
  const needed = Math.max(viewport + layout.rowHeight, offsetPx + viewport);
  let covered = 0;
  let count = 0;
  while (first + count < layout.rowCount && covered < needed) {
    covered += rowHeightAt(layout, first + count);
    count += 1;
  }
  return { first, count, offsetY: top - offsetPx };
}

/** The row at the top at `scrollTop`; `null` when scrolled to the very top. */
export function anchorAt(layout: Layout, scrollTop: number, viewport: number): RowPoint | null {
  if (scrollTop <= 0 || layout.rowCount <= 0) {
    return null;
  }
  return rowAt(layout, toVirtualTop(layout, scrollTop, viewport));
}

/**
 * BR1.8: the scroll position that shows `position` at the top, scrolled
 * `offsetPx` past its top, shrunk like every scroll position.
 */
export function scrollTopForAnchor(
  layout: Layout,
  position: number,
  offsetPx: number,
  viewport: number,
): number {
  return toScrollTop(layout, rowOffset(layout, position) + offsetPx, viewport);
}

/**
 * BR1.5: the scroll position that shows the line of the selected row: the
 * same position when the line is in view, else the least move that brings
 * it in (its top at the top, or its bottom at the bottom).
 */
export function scrollTopToReveal(
  layout: Layout,
  position: number,
  scrollTop: number,
  viewport: number,
): number {
  const virtualTop = toVirtualTop(layout, scrollTop, viewport);
  const lineTop = rowOffset(layout, position);
  const lineBottom = lineTop + layout.rowHeight;
  if (lineTop < virtualTop) {
    return toScrollTop(layout, lineTop, viewport);
  }
  if (lineBottom > virtualTop + viewport) {
    return toScrollTop(layout, lineBottom - viewport, viewport);
  }
  return clamp(scrollTop, 0, maxScrollTop(layout, viewport));
}

/** Whether a code point is East Asian Wide or Fullwidth (R-14: two columns). */
export function isWideCodePoint(codePoint: number): boolean {
  return (
    (codePoint >= 0x1100 && codePoint <= 0x115f) ||
    (codePoint >= 0x2e80 && codePoint <= 0x303e) ||
    (codePoint >= 0x3041 && codePoint <= 0x33ff) ||
    (codePoint >= 0x3400 && codePoint <= 0x4dbf) ||
    (codePoint >= 0x4e00 && codePoint <= 0x9fff) ||
    (codePoint >= 0xa000 && codePoint <= 0xa4cf) ||
    (codePoint >= 0xac00 && codePoint <= 0xd7a3) ||
    (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
    (codePoint >= 0xfe30 && codePoint <= 0xfe4f) ||
    (codePoint >= 0xff00 && codePoint <= 0xff60) ||
    (codePoint >= 0xffe0 && codePoint <= 0xffe6) ||
    (codePoint >= 0x1f300 && codePoint <= 0x1f64f) ||
    (codePoint >= 0x1f900 && codePoint <= 0x1f9ff) ||
    (codePoint >= 0x20000 && codePoint <= 0x2fffd) ||
    (codePoint >= 0x30000 && codePoint <= 0x3fffd)
  );
}

/**
 * The width of each line of `message` in columns of the monospace font: one
 * per character, two for a full-width one (R-14). Lines are split at
 * `\r\n`, `\r` and `\n`, as the message is shown.
 */
export function lineUnits(message: string): number[] {
  return message.split(/\r\n|\r|\n/).map((line) => {
    let units = 0;
    for (const character of line) {
      units += isWideCodePoint(character.codePointAt(0) ?? 0) ? 2 : 1;
    }
    return units;
  });
}

/** Number of wrapped lines of `units` (see `lineUnits`) in `contentWidth`. */
export function wrappedLineCount(
  units: readonly number[],
  charWidth: number,
  contentWidth: number,
): number {
  const width = Math.max(contentWidth, charWidth, 1);
  return units.reduce(
    (lines, lineWidth) => lines + Math.max(1, Math.ceil((lineWidth * charWidth) / width)),
    0,
  );
}

/** BR1.9: `min(lines, 20) × 18 + 12` px for an expansion of `lines` lines. */
export function expandedHeightForLines(lines: number): number {
  return Math.min(Math.max(1, lines), EXPANDED_MAX_LINES) * EXPANDED_LINE_HEIGHT + EXPANDED_PADDING;
}

/**
 * BR1.9: the estimated height of an expansion before it is measured: the
 * wrapped lines of the message (full-width characters count twice, R-14),
 * at most 20 lines, plus the padding.
 */
export function estimateExpandedHeight(
  message: string,
  charWidth: number,
  contentWidth: number,
): number {
  return expandedHeightForLines(wrappedLineCount(lineUnits(message), charWidth, contentWidth));
}
