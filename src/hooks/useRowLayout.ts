import { useCallback, useMemo, useState } from "react";
import {
  EXPANDED_MAX_LINES,
  buildLayout,
  expandedHeightForLines,
  lineUnits,
  wrappedLineCount,
  type Layout,
} from "../rowLayout";
import type { RowKey } from "./expansionState";

/** Character width used when it cannot be measured (12 px monospace). */
const FALLBACK_CHAR_WIDTH = 7.2;
/** Horizontal padding of an expansion's text (6 px each side, styles.css). */
const EXPANDED_HORIZONTAL_PADDING = 12;

let measuredCharWidth: number | null = null;

/**
 * BR1.9: the width of one character of the expansion's monospace font,
 * measured once at startup; the fallback when nothing can be measured.
 */
export function monospaceCharWidth(): number {
  if (measuredCharWidth !== null) {
    return measuredCharWidth;
  }
  let width = 0;
  if (typeof document !== "undefined" && document.body !== null) {
    const probe = document.createElement("span");
    probe.className = "log-expanded-probe";
    probe.textContent = "0".repeat(100);
    document.body.appendChild(probe);
    width = probe.getBoundingClientRect().width / 100;
    probe.remove();
  }
  measuredCharWidth = width > 0 ? width : FALLBACK_CHAR_WIDTH;
  return measuredCharWidth;
}

/** A measured expansion: its height and whether its text scrolls inside. */
interface Measured {
  height: number;
  scrolls: boolean;
}

export interface RowLayoutOptions {
  rowCount: number;
  /** Expanded rows and their positions (`null`: hidden). */
  expanded: ReadonlyMap<RowKey, number | null>;
  /** Width of the list, in pixels; measured heights belong to one width. */
  listWidth: number;
  /** Width of one character; measured once when absent. */
  charWidth?: number;
}

export interface RowLayout {
  layout: Layout;
  /** Remembers the line widths of an expanded row's message, for estimates. */
  remember: (key: RowKey, message: string) => void;
  /** Forgets everything about `key` (closed), or about every row (`null`). */
  forget: (key: RowKey | null) => void;
  /** Records a measured height (BR1.9). */
  measure: (key: RowKey, height: number, scrolls: boolean) => void;
  /** The height used for `key`: measured at this width, else estimated. */
  heightOf: (key: RowKey) => number;
  /** Whether the text of `key` scrolls inside its expansion (BR1.10). */
  scrolls: (key: RowKey) => boolean;
}

/**
 * U7:BR1.4, BR1.9: the layout of the list with its expanded rows. Each
 * expansion is as high as measured (ResizeObserver, reported through
 * `measure`) when it was measured at the current list width, else as
 * estimated from the line widths of its message (`remember`). A new list
 * width drops every measured height: the estimates are used until the
 * expansions on screen are measured again. Expansions off screen keep
 * their last measured height while the width is the same.
 */
export function useRowLayout({
  rowCount,
  expanded,
  listWidth,
  charWidth,
}: RowLayoutOptions): RowLayout {
  const [fontWidth] = useState(() => charWidth ?? monospaceCharWidth());
  const [units, setUnits] = useState<ReadonlyMap<RowKey, readonly number[]>>(new Map());
  const [measured, setMeasured] = useState<{
    width: number;
    rows: ReadonlyMap<RowKey, Measured>;
  }>({ width: listWidth, rows: new Map() });
  const contentWidth = Math.max(1, listWidth - EXPANDED_HORIZONTAL_PADDING);
  const current = measured.width === listWidth ? measured.rows : null;

  const remember = useCallback((key: RowKey, message: string) => {
    setUnits((previous) => new Map(previous).set(key, lineUnits(message)));
  }, []);

  const forget = useCallback((key: RowKey | null) => {
    const drop = <T>(previous: ReadonlyMap<RowKey, T>) => {
      if (key === null) {
        return new Map<RowKey, T>();
      }
      const next = new Map(previous);
      next.delete(key);
      return next;
    };
    setUnits(drop);
    setMeasured((previous) => ({ width: previous.width, rows: drop(previous.rows) }));
  }, []);

  const measure = useCallback(
    (key: RowKey, height: number, scrolls: boolean) => {
      if (!(height > 0)) {
        return;
      }
      setMeasured((previous) => {
        const sameWidth = previous.width === listWidth;
        const old = sameWidth ? previous.rows.get(key) : undefined;
        if (old !== undefined && old.height === height && old.scrolls === scrolls) {
          return previous;
        }
        const rows = new Map(sameWidth ? previous.rows : []);
        rows.set(key, { height, scrolls });
        return { width: listWidth, rows };
      });
    },
    [listWidth],
  );

  const estimatedLines = useCallback(
    (key: RowKey) => wrappedLineCount(units.get(key) ?? [0], fontWidth, contentWidth),
    [units, fontWidth, contentWidth],
  );

  const heightOf = useCallback(
    (key: RowKey) => current?.get(key)?.height ?? expandedHeightForLines(estimatedLines(key)),
    [current, estimatedLines],
  );

  const scrolls = useCallback(
    (key: RowKey) => current?.get(key)?.scrolls ?? estimatedLines(key) > EXPANDED_MAX_LINES,
    [current, estimatedLines],
  );

  const layout = useMemo(() => {
    const expansions = [];
    for (const [key, position] of expanded) {
      if (position !== null) {
        expansions.push({ position, height: heightOf(key) });
      }
    }
    return buildLayout(rowCount, expansions);
  }, [rowCount, expanded, heightOf]);

  return { layout, remember, forget, measure, heightOf, scrolls };
}
