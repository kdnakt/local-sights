import { describe, expect, it } from "vitest";
import {
  MAX_EXPANDED_ROWS,
  initialExpansionState,
  moveSelection,
  rowKeyOf,
  toggleRow,
  type ExpansionState,
  withDiscardGeneration,
  withPositions,
  withSelectedKey,
} from "./expansionState";

const a = rowKeyOf("stream-a", 1);
const b = rowKeyOf("stream-b", 7);
const c = rowKeyOf("stream-a", 9);

describe("expansionState", () => {
  it("opens and closes rows independently and selects the toggled row", () => {
    let state = initialExpansionState(0);
    state = toggleRow(state, a, 1);
    state = toggleRow(state, b, 5);
    expect([...state.expanded]).toEqual([
      [a, 1],
      [b, 5],
    ]);
    expect(state.selection).toEqual({ key: b, position: 5 });
    state = toggleRow(state, a, 1);
    expect([...state.expanded.keys()]).toEqual([b]);
    expect(state.selection).toEqual({ key: a, position: 1 });
    expect(rowKeyOf("x", 3)).toBe("x\u00003");
  });

  it("empties the expansions and selects the top row when the logs were discarded", () => {
    let state = toggleRow(toggleRow(initialExpansionState(2), a, 1), b, 5);
    expect(withDiscardGeneration(state, 2)).toBe(state);
    expect(withDiscardGeneration(state, 1)).toBe(state);
    state = withDiscardGeneration(state, 3);
    expect(state.expanded.size).toBe(0);
    expect(state.selection).toEqual({ key: null, position: 0 });
    expect(state.discardGeneration).toBe(3);
  });

  it("keeps the expansions across a filter, a time zone switch and added rows", () => {
    let state = toggleRow(toggleRow(initialExpansionState(0), a, 1), b, 5);
    // Filtered: a is hidden, b moved; the set stays.
    state = withPositions(
      state,
      new Map([
        [a, null],
        [b, 2],
      ]),
      10,
    );
    expect([...state.expanded]).toEqual([
      [a, null],
      [b, 2],
    ]);
    // Filter cleared and rows added above: both shown again, open.
    state = withPositions(
      state,
      new Map([
        [a, 4],
        [b, 9],
      ]),
      50,
    );
    expect([...state.expanded]).toEqual([
      [a, 4],
      [b, 9],
    ]);
    // A key not asked about keeps its last position.
    state = withPositions(state, new Map([[b, 12]]), 50);
    expect(state.expanded.get(a)).toBe(4);
  });

  it("selects the row at the same position when the selected key went away", () => {
    let state = toggleRow(initialExpansionState(0), c, 6);
    state = withPositions(state, new Map([[c, 3]]), 10);
    expect(state.selection).toEqual({ key: c, position: 3 });
    state = withPositions(state, new Map([[c, null]]), 10);
    expect(state.selection).toEqual({ key: null, position: 3 });
    state = withSelectedKey(state, (position) => (position === 3 ? a : undefined));
    expect(state.selection).toEqual({ key: a, position: 3 });
    // Fewer rows than the position: the last row.
    state = withPositions(state, new Map([[a, null]]), 2);
    expect(state.selection).toEqual({ key: null, position: 1 });
    state = withPositions(state, new Map(), 0);
    expect(state.selection).toEqual({ key: null, position: 0 });
  });

  it("moves the selection by row, by page and to either end", () => {
    const keyAt = (position: number) => rowKeyOf("s", position);
    let state = initialExpansionState(0);
    state = moveSelection(state, "ArrowDown", 100, 5, keyAt) ?? state;
    expect(state.selection).toEqual({ key: keyAt(1), position: 1 });
    state = moveSelection(state, "PageDown", 100, 5, keyAt) ?? state;
    expect(state.selection.position).toBe(6);
    state = moveSelection(state, "End", 100, 5, () => undefined) ?? state;
    expect(state.selection).toEqual({ key: null, position: 99 });
    state = moveSelection(state, "ArrowDown", 100, 5, keyAt) ?? state;
    expect(state.selection.position).toBe(99);
    state = moveSelection(state, "PageUp", 100, 5, keyAt) ?? state;
    expect(state.selection.position).toBe(94);
    state = moveSelection(state, "Home", 100, 5, keyAt) ?? state;
    expect(state.selection).toEqual({ key: keyAt(0), position: 0 });
    state = moveSelection(state, "ArrowUp", 100, 5, keyAt) ?? state;
    expect(state.selection.position).toBe(0);
    expect(moveSelection(state, "Enter", 100, 5, keyAt)).toBeNull();
    expect(moveSelection(state, "ArrowDown", 0, 5, keyAt)?.selection).toEqual({
      key: null,
      position: 0,
    });
  });

  it("refuses to open more than MAX_EXPANDED_ROWS rows and says so (R-04)", () => {
    const full: ExpansionState = {
      ...initialExpansionState(0),
      expanded: new Map(
        Array.from({ length: MAX_EXPANDED_ROWS }, (_, i) => [rowKeyOf("s", i), i] as const),
      ),
    };
    expect(MAX_EXPANDED_ROWS).toBe(10_000);
    const refused = toggleRow(full, c, 20_000);
    expect(refused.expanded).toBe(full.expanded);
    expect(refused.expanded.has(c)).toBe(false);
    expect(refused.limitReached).toBe(true);
    expect(refused.selection).toEqual({ key: c, position: 20_000 });
    // Closing one is always allowed and clears the notice; then one more opens.
    const closed = toggleRow(refused, rowKeyOf("s", 0), 0);
    expect(closed.expanded.size).toBe(MAX_EXPANDED_ROWS - 1);
    expect(closed.limitReached).toBe(false);
    const reopened = toggleRow(closed, c, 20_000);
    expect(reopened.expanded.size).toBe(MAX_EXPANDED_ROWS);
    expect(reopened.limitReached).toBe(false);
  });

  it("starts with the notice off and clears it when the logs are discarded", () => {
    expect(initialExpansionState(0).limitReached).toBe(false);
    const state = { ...toggleRow(initialExpansionState(0), a, 1), limitReached: true };
    expect(withDiscardGeneration(state, 1).limitReached).toBe(false);
    expect(withPositions(state, new Map([[a, 2]]), 10).limitReached).toBe(true);
  });
});
