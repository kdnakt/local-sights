import { describe, expect, it } from "vitest";
import {
  anchorAt,
  maxScrollTop,
  preservedScrollTop,
  scrollHeight,
  scrollTopForKey,
  scrollTopForRow,
  visibleRows,
  type Geometry,
} from "./virtualScroll";

function geometry(totalCount: number, overrides: Partial<Geometry> = {}): Geometry {
  return {
    totalCount,
    rowHeight: 20,
    viewportHeight: 100,
    maxScrollHeight: 10_000_000,
    ...overrides,
  };
}

describe("virtual scroll", () => {
  it("shows nothing for zero rows", () => {
    const g = geometry(0);
    expect(scrollHeight(g)).toBe(0);
    expect(maxScrollTop(g)).toBe(0);
    expect(visibleRows(0, g)).toEqual({ firstRow: 0, rowCount: 0, offsetY: 0 });
    expect(anchorAt(0, g)).toBeNull();
  });

  it("finds the first row, the row count and where to draw them", () => {
    const g = geometry(1_000);
    expect(scrollHeight(g)).toBe(20_000);
    expect(visibleRows(0, g)).toEqual({ firstRow: 0, rowCount: 6, offsetY: 0 });
    expect(visibleRows(205, g)).toEqual({ firstRow: 10, rowCount: 6, offsetY: 200 });
  });

  it("reaches the last row when scrolled to the end", () => {
    const g = geometry(1_000);
    expect(maxScrollTop(g)).toBe(19_900);
    const range = visibleRows(maxScrollTop(g), g);
    expect(range.firstRow).toBe(995);
    expect(range.firstRow + range.rowCount).toBe(1_000);
    // A scroll position beyond the end is treated as the end.
    expect(visibleRows(50_000, g)).toEqual(range);
  });

  it("shrinks the scroll height above the limit and still reaches the first and last rows", () => {
    const g = geometry(1_000_000);
    expect(scrollHeight(g)).toBe(10_000_000);
    expect(visibleRows(0, g)).toEqual({ firstRow: 0, rowCount: 6, offsetY: 0 });
    const end = visibleRows(maxScrollTop(g), g);
    expect(end.firstRow + end.rowCount).toBe(1_000_000);
    // The drawn rows stay inside the scrolled area at the end.
    expect(end.offsetY + end.rowCount * 20).toBeGreaterThanOrEqual(maxScrollTop(g) + 100);
    expect(end.offsetY + end.rowCount * 20).toBeLessThanOrEqual(scrollHeight(g) + 20);
    const middle = visibleRows(maxScrollTop(g) / 2, g);
    expect(Math.abs(middle.firstRow - 499_997)).toBeLessThanOrEqual(1);
  });

  it("converts between a row and a scroll position both ways", () => {
    for (const g of [geometry(1_000), geometry(1_000_000)]) {
      const top = scrollTopForRow(4_321 % g.totalCount, 7, g);
      const anchor = anchorAt(top, g);
      expect(anchor?.row).toBe(4_321 % g.totalCount);
      expect(anchor?.pixelOffset ?? -1).toBeCloseTo(7, 3);
    }
    const g = geometry(1_000);
    expect(scrollTopForRow(0, 0, g)).toBe(0);
    expect(scrollTopForRow(999, 0, g)).toBe(19_900);
  });

  it("keeps the top at the top and otherwise follows the anchored row", () => {
    const before = geometry(1_000);
    expect(anchorAt(0, before)).toBeNull();
    const anchor = anchorAt(205, before);
    expect(anchor).toEqual({ row: 10, pixelOffset: 5 });
    // 50 rows were inserted above: the anchored row is now at 60.
    const after = geometry(1_050);
    expect(preservedScrollTop(60, anchor?.pixelOffset ?? 0, after)).toBe(1_205);
    // The anchored row is gone (the logs were discarded): back to the top.
    expect(preservedScrollTop(null, 5, after)).toBe(0);
  });

  it("scrolls by row, by page and to either end with the keyboard", () => {
    const g = geometry(1_000);
    expect(scrollTopForKey("ArrowDown", 0, g)).toBe(20);
    expect(scrollTopForKey("ArrowUp", 0, g)).toBe(0);
    expect(scrollTopForKey("PageDown", 40, g)).toBe(140);
    expect(scrollTopForKey("PageUp", 40, g)).toBe(0);
    expect(scrollTopForKey("End", 0, g)).toBe(19_900);
    expect(scrollTopForKey("Home", 5_000, g)).toBe(0);
    expect(scrollTopForKey("ArrowDown", 19_900, g)).toBe(19_900);
    expect(scrollTopForKey("Enter", 0, g)).toBeNull();
  });
});
