import { describe, expect, it } from "vitest";
import {
  EXPANDED_MAX_HEIGHT,
  anchorAt,
  buildLayout,
  estimateExpandedHeight,
  lineUnits,
  maxScrollTop,
  rowAt,
  rowHeightAt,
  rowOffset,
  scrollHeight,
  scrollTopForAnchor,
  scrollTopToReveal,
  toScrollTop,
  toVirtualTop,
  virtualHeight,
  visibleRange,
} from "./rowLayout";
import { anchorAt as u3AnchorAt, visibleRows, type Geometry } from "./virtualScroll";

/** Rows of 22 px; rows 3 and 10 are expanded, row 200 does not exist. */
function expandedLayout() {
  return buildLayout(100, [
    { position: 10, height: 100 },
    { position: 3, height: 50 },
    { position: 200, height: 9 },
  ]);
}

describe("rowLayout", () => {
  it("adds the heights of the expansions above a row to its offset", () => {
    const layout = expandedLayout();
    expect(rowOffset(layout, 0)).toBe(0);
    expect(rowOffset(layout, 3)).toBe(66);
    expect(rowOffset(layout, 4)).toBe(4 * 22 + 50);
    expect(rowOffset(layout, 10)).toBe(10 * 22 + 50);
    expect(rowOffset(layout, 11)).toBe(11 * 22 + 150);
    expect(rowHeightAt(layout, 3)).toBe(72);
    expect(rowHeightAt(layout, 4)).toBe(22);
    expect(rowHeightAt(layout, 10)).toBe(122);
    // A position outside the list (200) is not counted.
    expect(virtualHeight(layout)).toBe(100 * 22 + 150);
  });

  it("finds the row of a y both ways, and a y inside an expansion belongs to its row", () => {
    const layout = expandedLayout();
    for (const position of [0, 2, 3, 4, 9, 10, 11, 99]) {
      expect(rowAt(layout, rowOffset(layout, position) + 5)).toEqual({ position, offsetPx: 5 });
    }
    // 30 px into the expansion of row 3 (below its 22 px line).
    expect(rowAt(layout, rowOffset(layout, 3) + 22 + 30)).toEqual({ position: 3, offsetPx: 52 });
    expect(rowAt(layout, rowOffset(layout, 10) + 121)).toEqual({ position: 10, offsetPx: 121 });
    expect(rowAt(layout, rowOffset(layout, 10) + 122)).toEqual({ position: 11, offsetPx: 0 });
    expect(rowAt(layout, -5)).toEqual({ position: 0, offsetPx: 0 });
    expect(rowAt(layout, 1_000_000).position).toBe(99);
  });

  it("shrinks above the limit with U3's normalization (review R-12)", () => {
    const layout = buildLayout(1_000_000, [{ position: 5, height: 300 }]);
    const viewport = 110;
    const height = 22_000_000 + 300;
    expect(virtualHeight(layout)).toBe(height);
    expect(scrollHeight(layout)).toBe(10_000_000);
    expect(maxScrollTop(layout, viewport)).toBe(10_000_000 - viewport);
    expect(toScrollTop(layout, height - viewport, viewport)).toBe(10_000_000 - viewport);
    expect(toVirtualTop(layout, 10_000_000 - viewport, viewport)).toBe(height - viewport);
    expect(toVirtualTop(layout, 0, viewport)).toBe(0);
    const middle = toScrollTop(layout, (height - viewport) / 2, viewport);
    expect(middle).toBeCloseTo((10_000_000 - viewport) / 2, 6);
  });

  it("reaches the last row of a million with End, with and without expansions", () => {
    const viewport = 110;
    for (const expansions of [
      [],
      [
        { position: 0, height: 100 },
        { position: 500_000, height: 200 },
        { position: 999_999, height: 372 },
      ],
    ]) {
      const layout = buildLayout(1_000_000, expansions);
      const end = visibleRange(layout, maxScrollTop(layout, viewport), viewport);
      expect(end.first + end.count).toBe(1_000_000);
      const top = scrollTopToReveal(layout, 999_999, 0, viewport);
      const shown = visibleRange(layout, top, viewport);
      expect(shown.first).toBeLessThanOrEqual(999_999);
      expect(shown.first + shown.count).toBe(1_000_000);
      expect(top).toBeLessThanOrEqual(maxScrollTop(layout, viewport));
    }
  });

  it("does not move the rows above an expanded or collapsed row", () => {
    const before = buildLayout(1_000, []);
    const after = buildLayout(1_000, [{ position: 60, height: 200 }]);
    for (const position of [0, 30, 59, 60]) {
      expect(rowOffset(after, position)).toBe(rowOffset(before, position));
    }
    expect(rowOffset(after, 61)).toBe(rowOffset(before, 61) + 200);
    // The row at the top (50, scrolled 3 px past) stays where it was.
    expect(scrollTopForAnchor(after, 50, 3, 110)).toBe(scrollTopForAnchor(before, 50, 3, 110));
    expect(scrollTopForAnchor(after, 50, 3, 110)).toBe(50 * 22 + 3);
  });

  it("scrolls to an anchor row with its offset, also when shrunk", () => {
    const layout = expandedLayout();
    expect(scrollTopForAnchor(layout, 11, 4, 110)).toBe(11 * 22 + 150 + 4);
    expect(anchorAt(layout, 0, 110)).toBeNull();
    expect(anchorAt(layout, rowOffset(layout, 10) + 30, 110)).toEqual({
      position: 10,
      offsetPx: 30,
    });
    const big = buildLayout(1_000_000, [{ position: 10, height: 372 }]);
    const viewport = 110;
    const top = scrollTopForAnchor(big, 500_000, 5, viewport);
    const ratio = (10_000_000 - viewport) / (22_000_000 + 372 - viewport);
    expect(top).toBeCloseTo((500_000 * 22 + 372 + 5) * ratio, 6);
    const anchor = anchorAt(big, top, viewport);
    expect(anchor?.position).toBe(500_000);
    expect(anchor?.offsetPx ?? -1).toBeCloseTo(5, 3);
  });

  it("reveals a selected row only when its line is out of view", () => {
    const layout = expandedLayout();
    // Row 2 is in view at the top: nothing moves.
    expect(scrollTopToReveal(layout, 2, 0, 110)).toBe(0);
    // Row 20 is below: its line ends at the bottom of the view.
    expect(scrollTopToReveal(layout, 20, 0, 110)).toBe(rowOffset(layout, 20) + 22 - 110);
    // Row 3 is above: its line goes to the top.
    expect(scrollTopToReveal(layout, 3, 500, 110)).toBe(rowOffset(layout, 3));
  });

  it("estimates an expansion: 18 px lines, 12 px padding, at most 372 px, full width counts twice (R-14)", () => {
    // 7 px characters in 700 px: 100 columns.
    expect(estimateExpandedHeight("short", 7, 700)).toBe(30);
    expect(estimateExpandedHeight("", 7, 700)).toBe(30);
    expect(estimateExpandedHeight("a\r\nb\rc\nd", 7, 700)).toBe(4 * 18 + 12);
    // One long word without spaces wraps anywhere: 250 characters, 3 lines.
    expect(estimateExpandedHeight("A".repeat(250), 7, 700)).toBe(3 * 18 + 12);
    // 60 full-width characters take 120 columns: 2 lines; 60 ASCII take 1.
    expect(estimateExpandedHeight("あ".repeat(60), 7, 700)).toBe(2 * 18 + 12);
    expect(estimateExpandedHeight("a".repeat(60), 7, 700)).toBe(30);
    const many = Array.from({ length: 25 }, (_, i) => `line ${i}`).join("\n");
    expect(estimateExpandedHeight(many, 7, 700)).toBe(EXPANDED_MAX_HEIGHT);
    expect(EXPANDED_MAX_HEIGHT).toBe(372);
    expect(lineUnits("a日本\n😀ｱ")).toEqual([5, 3]);
  });

  it("gives U3's answers when nothing is expanded", () => {
    const fixed = (count: number) => buildLayout(count, [], { rowHeight: 20 });
    const viewport = 100;
    expect(visibleRange(fixed(0), 0, viewport)).toEqual({ first: 0, count: 0, offsetY: 0 });
    expect(visibleRange(fixed(1_000), 0, viewport)).toEqual({ first: 0, count: 6, offsetY: 0 });
    expect(visibleRange(fixed(1_000), 205, viewport)).toEqual({
      first: 10,
      count: 6,
      offsetY: 200,
    });
    expect(maxScrollTop(fixed(1_000), viewport)).toBe(19_900);
    expect(visibleRange(fixed(1_000), 19_900, viewport)).toEqual({
      first: 995,
      count: 5,
      offsetY: 19_900,
    });
    expect(visibleRange(fixed(1_000), 50_000, viewport)).toEqual(
      visibleRange(fixed(1_000), 19_900, viewport),
    );
    expect(anchorAt(fixed(1_000), 205, viewport)).toEqual({ position: 10, offsetPx: 5 });
    expect(scrollTopForAnchor(fixed(1_050), 60, 5, viewport)).toBe(1_205);
    const big = fixed(1_000_000);
    expect(scrollHeight(big)).toBe(10_000_000);
    const middle = visibleRange(big, maxScrollTop(big, viewport) / 2, viewport);
    expect(Math.abs(middle.first - 499_997)).toBeLessThanOrEqual(1);

    // The same answers as U3's virtualScroll for a sweep of lists and positions.
    for (const totalCount of [0, 1, 7, 1_000, 1_000_000]) {
      const g: Geometry = {
        totalCount,
        rowHeight: 20,
        viewportHeight: viewport,
        maxScrollHeight: 10_000_000,
      };
      for (const top of [0, 1, 205, 9_999, 19_900, 50_000, 5_000_000]) {
        const u3 = visibleRows(top, g);
        const range = visibleRange(fixed(totalCount), top, viewport);
        expect([range.first, range.count]).toEqual([u3.firstRow, u3.rowCount]);
        expect(range.offsetY).toBeCloseTo(u3.offsetY, 6);
        const anchor = anchorAt(fixed(totalCount), top, viewport);
        const u3Anchor = u3AnchorAt(top, g);
        expect(anchor?.position ?? null).toBe(u3Anchor?.row ?? null);
      }
    }
  });
});
