import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  PANE_COLLAPSED_STORAGE_KEY,
  PANE_WIDTH_DEFAULT_PERCENT,
  PANE_WIDTH_MAX_PERCENT,
  PANE_WIDTH_MIN_PERCENT,
  PANE_WIDTH_STORAGE_KEY,
  clampPaneWidth,
  readStoredPaneCollapsed,
  readStoredPaneWidth,
  usePaneWidth,
} from "./usePaneWidth";

/** Runs `body` while `window.localStorage` throws on every access. */
function withoutStorage(body: () => void) {
  const storage = Object.getOwnPropertyDescriptor(window, "localStorage");
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    get() {
      throw new Error("storage is disabled");
    },
  });
  try {
    body();
  } finally {
    if (storage) {
      Object.defineProperty(window, "localStorage", storage);
    }
  }
}

describe("usePaneWidth (gate fixes 14d, 15b)", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it("starts open at 25% of the app body, between 20% and 50%", () => {
    const { result } = renderHook(() => usePaneWidth());
    expect(result.current.width).toBe(PANE_WIDTH_DEFAULT_PERCENT);
    expect(result.current.collapsed).toBe(false);
    expect(PANE_WIDTH_DEFAULT_PERCENT).toBe(25);
    expect(PANE_WIDTH_MIN_PERCENT).toBe(20);
    expect(PANE_WIDTH_MAX_PERCENT).toBe(50);
  });

  it("clamps the width to the minimum and the maximum and remembers it in percent", () => {
    const { result } = renderHook(() => usePaneWidth());
    act(() => result.current.setWidth(10));
    expect(result.current.width).toBe(PANE_WIDTH_MIN_PERCENT);
    expect(window.localStorage.getItem(PANE_WIDTH_STORAGE_KEY)).toBe(
      String(PANE_WIDTH_MIN_PERCENT),
    );
    act(() => result.current.setWidth(90));
    expect(result.current.width).toBe(PANE_WIDTH_MAX_PERCENT);
    act(() => result.current.setWidth(33.5));
    expect(result.current.width).toBe(33.5);
    expect(window.localStorage.getItem(PANE_WIDTH_STORAGE_KEY)).toBe("33.5");
    expect(clampPaneWidth(Number.NaN)).toBe(PANE_WIDTH_DEFAULT_PERCENT);
  });

  it("restores a stored percentage, falls back for garbage and ignores the old rem key", () => {
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEY, "40");
    expect(renderHook(() => usePaneWidth()).result.current.width).toBe(40);
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEY, "wide");
    expect(readStoredPaneWidth()).toBe(PANE_WIDTH_DEFAULT_PERCENT);
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEY, "999");
    expect(readStoredPaneWidth()).toBe(PANE_WIDTH_MAX_PERCENT);
    // The rem-based key of gate fix 14d is not read any more: 14 would be
    // clamped to 20% and misread as a share of the window.
    window.localStorage.clear();
    window.localStorage.setItem("local-sights.logGroupPaneWidth", "14");
    expect(readStoredPaneWidth()).toBe(PANE_WIDTH_DEFAULT_PERCENT);
    expect(PANE_WIDTH_STORAGE_KEY).toBe("local-sights.logGroupPaneWidthPercent");
  });

  it("remembers the collapsed state and keeps the width while collapsed", () => {
    const { result } = renderHook(() => usePaneWidth());
    act(() => result.current.setWidth(30));
    act(() => result.current.setCollapsed(true));
    expect(result.current.collapsed).toBe(true);
    expect(result.current.width).toBe(30);
    expect(window.localStorage.getItem(PANE_COLLAPSED_STORAGE_KEY)).toBe("true");
    // A new launch starts collapsed, at the remembered width once opened.
    const next = renderHook(() => usePaneWidth());
    expect(next.result.current.collapsed).toBe(true);
    expect(next.result.current.width).toBe(30);
    act(() => next.result.current.setCollapsed(false));
    expect(next.result.current.collapsed).toBe(false);
    expect(window.localStorage.getItem(PANE_COLLAPSED_STORAGE_KEY)).toBe("false");
    // Anything but "true" means open.
    window.localStorage.setItem(PANE_COLLAPSED_STORAGE_KEY, "yes");
    expect(readStoredPaneCollapsed()).toBe(false);
  });

  it("works with the defaults when localStorage cannot be used", () => {
    withoutStorage(() => {
      expect(readStoredPaneWidth()).toBe(PANE_WIDTH_DEFAULT_PERCENT);
      expect(readStoredPaneCollapsed()).toBe(false);
      const { result } = renderHook(() => usePaneWidth());
      act(() => result.current.setWidth(35));
      act(() => result.current.setCollapsed(true));
      expect(result.current.width).toBe(35);
      expect(result.current.collapsed).toBe(true);
    });
  });
});
