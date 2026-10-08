/**
 * The expanded rows and the selected row of the log list (U7:BR1.1, BR1.5,
 * BR1.6). Both are screen-only state: the core is never told which rows are
 * open (functional-spec §5; the move away from SessionState.expandedRows of
 * components.md is recorded in code-summary.md, review R-14). Rows are
 * identified by `(logStreamName, sequence)`, which names the same event
 * until the held logs are discarded (U3:BR4.4); their positions are asked
 * from the core (BR1.7) and applied here. Every function here is pure and
 * returns a new state (or the same one when nothing changes).
 */

/** `${logStreamName}\u0000${sequence}`: one held event. */
export type RowKey = string;

/** The selected row: its key when known, and its position in the list. */
export interface Selection {
  key: RowKey | null;
  position: number;
}

export interface ExpansionState {
  /** The discard generation the keys belong to (BR1.6, review R-11). */
  discardGeneration: number;
  /** Expanded rows in the order they were opened, with their positions (`null`: hidden). */
  expanded: ReadonlyMap<RowKey, number | null>;
  selection: Selection;
  /**
   * The last attempt to open a row was refused because `MAX_EXPANDED_ROWS`
   * rows are open (code generation review R-04); the list says so.
   */
  limitReached: boolean;
}

/**
 * Most rows open at once (review R-04). The positions of the open rows,
 * the selected row and the top row are asked in one `row_positions` call,
 * which accepts at most this many keys plus those two.
 */
export const MAX_EXPANDED_ROWS = 10_000;

/** Finds the key of the row at a position, when that row is loaded. */
export type KeyAt = (position: number) => RowKey | undefined;

const MOVE_KEYS = ["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End"] as const;

/** The key of one held event. */
export function rowKeyOf(logStreamName: string, sequence: number): RowKey {
  return `${logStreamName}\u0000${sequence}`;
}

/** The stream name and sequence of a key; `null` when it is not a row key. */
export function parseRowKey(key: RowKey): { logStreamName: string; sequence: number } | null {
  const separator = key.lastIndexOf("\u0000");
  const sequence = Number(key.slice(separator + 1));
  if (separator < 0 || !Number.isSafeInteger(sequence)) {
    return null;
  }
  return { logStreamName: key.slice(0, separator), sequence };
}

/** Nothing expanded, the top row selected. */
export function initialExpansionState(discardGeneration = 0): ExpansionState {
  return {
    discardGeneration,
    expanded: new Map(),
    selection: { key: null, position: 0 },
    limitReached: false,
  };
}

/**
 * BR1.1: opens the row when it is closed and closes it when it is open; the
 * other rows stay as they are. The row also becomes the selected row. While
 * `maxExpanded` (`MAX_EXPANDED_ROWS`) rows are open, opening one more is refused and
 * `limitReached` is set, so the refusal is shown, never silent (review
 * R-04); any successful toggle clears it.
 */
export function toggleRow(
  state: ExpansionState,
  key: RowKey,
  position: number,
  maxExpanded: number = MAX_EXPANDED_ROWS,
): ExpansionState {
  const selection = { key, position };
  if (!state.expanded.has(key) && state.expanded.size >= maxExpanded) {
    return { ...state, selection, limitReached: true };
  }
  const expanded = new Map(state.expanded);
  if (expanded.has(key)) {
    expanded.delete(key);
  } else {
    expanded.set(key, position);
  }
  return { ...state, expanded, selection, limitReached: false };
}

/**
 * BR1.6: a newer discard generation means the held logs were discarded, so
 * every key is stale: nothing is expanded and the top row is selected. The
 * same or an older generation changes nothing.
 */
export function withDiscardGeneration(state: ExpansionState, generation: number): ExpansionState {
  if (generation <= state.discardGeneration) {
    return state;
  }
  return initialExpansionState(generation);
}

/**
 * BR1.6, BR1.7: applies positions asked from the core. Expanded rows stay
 * expanded whatever happens to their positions (a hidden row has `null`
 * and is open again when shown). When the selected key is gone, the row at
 * the same position is selected (its key is found once it is loaded), or
 * the last row when the list became shorter (BR1.5).
 */
export function withPositions(
  state: ExpansionState,
  positions: ReadonlyMap<RowKey, number | null>,
  rowCount: number,
): ExpansionState {
  const expanded = new Map(state.expanded);
  for (const key of expanded.keys()) {
    const position = positions.get(key);
    if (position !== undefined) {
      expanded.set(key, position);
    }
  }
  return { ...state, expanded, selection: movedSelection(state.selection, positions, rowCount) };
}

/** Where the selection is after new positions (see `withPositions`). */
function movedSelection(
  selection: Selection,
  positions: ReadonlyMap<RowKey, number | null>,
  rowCount: number,
): Selection {
  const last = Math.max(0, rowCount - 1);
  const found = selection.key === null ? undefined : positions.get(selection.key);
  if (found !== undefined && found !== null) {
    return { key: selection.key, position: found };
  }
  if (found === null || selection.position > last) {
    // The key went away (or its row is past the end): same position, key unknown.
    return { key: null, position: Math.min(selection.position, last) };
  }
  return selection;
}

/** Finds the selected row's key once that row is loaded. */
export function withSelectedKey(state: ExpansionState, keyAt: KeyAt): ExpansionState {
  if (state.selection.key !== null) {
    return state;
  }
  const key = keyAt(state.selection.position);
  return key === undefined ? state : { ...state, selection: { ...state.selection, key } };
}

/**
 * BR1.5: moves the selection by one row (arrows), by `pageRows` (Page Up,
 * Page Down) or to either end (Home, End); `null` for any other key.
 * Expansions are not a unit of selection: Down goes to the next row.
 */
export function moveSelection(
  state: ExpansionState,
  key: string,
  rowCount: number,
  pageRows: number,
  keyAt: KeyAt,
): ExpansionState | null {
  if (!(MOVE_KEYS as readonly string[]).includes(key)) {
    return null;
  }
  const last = Math.max(0, rowCount - 1);
  const current = Math.min(state.selection.position, last);
  const page = Math.max(1, pageRows);
  const target: Record<(typeof MOVE_KEYS)[number], number> = {
    ArrowUp: current - 1,
    ArrowDown: current + 1,
    PageUp: current - page,
    PageDown: current + page,
    Home: 0,
    End: last,
  };
  const position = Math.min(Math.max(target[key as (typeof MOVE_KEYS)[number]], 0), last);
  return {
    ...state,
    selection: { key: rowCount > 0 ? (keyAt(position) ?? null) : null, position },
  };
}
