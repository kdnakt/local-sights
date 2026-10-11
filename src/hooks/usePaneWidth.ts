import { useCallback, useState } from "react";

/**
 * Where the chosen width is remembered between launches (gate fix 14d). Since
 * gate fix 15b the value is a percentage of the app body's width, under a new
 * key; the old rem-based key (`local-sights.logGroupPaneWidth`) is not read.
 */
export const PANE_WIDTH_STORAGE_KEY = "local-sights.logGroupPaneWidthPercent";
/** Where the collapsed state of the pane is remembered (gate fix 15b). */
export const PANE_COLLAPSED_STORAGE_KEY = "local-sights.logGroupPaneCollapsed";
/** The width at first launch, as a percentage of the app body's width. */
export const PANE_WIDTH_DEFAULT_PERCENT = 25;
/** The narrowest the pane can be dragged, in percent ("about 20% of the screen"). */
export const PANE_WIDTH_MIN_PERCENT = 20;
/** The widest the pane can be dragged, in percent. */
export const PANE_WIDTH_MAX_PERCENT = 50;
/** One arrow key press moves the handle by this much, in percent. */
export const PANE_WIDTH_STEP_PERCENT = 1;

/** Keeps `width` between the minimum and the maximum. */
export function clampPaneWidth(width: number): number {
  if (!Number.isFinite(width)) {
    return PANE_WIDTH_DEFAULT_PERCENT;
  }
  return Math.min(PANE_WIDTH_MAX_PERCENT, Math.max(PANE_WIDTH_MIN_PERCENT, width));
}

/**
 * The width remembered by the browser, or the default when nothing valid is
 * stored or `localStorage` cannot be used (a private window, a WebView
 * without storage). The value is a plain percentage, not a secret.
 */
export function readStoredPaneWidth(): number {
  try {
    const stored = window.localStorage.getItem(PANE_WIDTH_STORAGE_KEY);
    if (stored === null) {
      return PANE_WIDTH_DEFAULT_PERCENT;
    }
    const parsed = Number(stored);
    return Number.isFinite(parsed) ? clampPaneWidth(parsed) : PANE_WIDTH_DEFAULT_PERCENT;
  } catch {
    return PANE_WIDTH_DEFAULT_PERCENT;
  }
}

/** Remembers `width`; a storage that refuses is ignored (the width still applies). */
export function storePaneWidth(width: number): void {
  try {
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEY, String(width));
  } catch {
    // Not available: the width lives for this session only.
  }
}

/** Whether the pane was collapsed when the app was last used; open by default. */
export function readStoredPaneCollapsed(): boolean {
  try {
    return window.localStorage.getItem(PANE_COLLAPSED_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

/** Remembers whether the pane is collapsed; a storage that refuses is ignored. */
export function storePaneCollapsed(collapsed: boolean): void {
  try {
    window.localStorage.setItem(PANE_COLLAPSED_STORAGE_KEY, String(collapsed));
  } catch {
    // Not available: the state lives for this session only.
  }
}

export interface PaneState {
  /** The width of the open pane, in percent of the app body's width. */
  width: number;
  /** True while the pane is closed and only the opening strip is shown. */
  collapsed: boolean;
  /** Changes the width; the value is clamped and remembered. */
  setWidth: (width: number) => void;
  /** Closes or opens the pane; the state is remembered. The width is kept. */
  setCollapsed: (collapsed: boolean) => void;
}

/**
 * The log group pane's width and collapsed state, restored from the last
 * launch and saved on every change (gate fix 14d: "make the log group pane
 * narrower and resizable"; gate fix 15b: down to about 20% of the screen and
 * closable). The width is a percentage of the app body's width, so the same
 * stored value means the same share of the window at every window size.
 */
export function usePaneWidth(): PaneState {
  const [width, setWidthState] = useState(readStoredPaneWidth);
  const [collapsed, setCollapsedState] = useState(readStoredPaneCollapsed);
  const setWidth = useCallback((next: number) => {
    const clamped = clampPaneWidth(next);
    setWidthState(clamped);
    storePaneWidth(clamped);
  }, []);
  const setCollapsed = useCallback((next: boolean) => {
    setCollapsedState(next);
    storePaneCollapsed(next);
  }, []);
  return { width, collapsed, setWidth, setCollapsed };
}
