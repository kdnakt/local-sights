import { describe, expect, it } from "vitest";
import {
  initialExpansionState,
  moveSelection,
  rowKeyOf,
  toggleRow,
  toggleSelected,
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

  it("does nothing on Enter or Space while the selected row is not loaded", () => {
    const state = moveSelection(initialExpansionState(0), "End", 1_000, 5, () => undefined);
    expect(state).not.toBeNull();
    if (state === null) {
      return;
    }
    expect(toggleSelected(state, () => undefined)).toBe(state);
    const toggled = toggleSelected(state, (position) => (position === 999 ? c : undefined));
    expect([...toggled.expanded]).toEqual([[c, 999]]);
    expect(toggleSelected(toggled, () => c).expanded.size).toBe(0);
  });
});
