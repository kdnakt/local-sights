import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Mode, applyMode } from "@cloudscape-design/global-styles";
import { DARK_SCHEME_QUERY, useColorScheme } from "./useColorScheme";

vi.mock("@cloudscape-design/global-styles", async (importOriginal) => {
  const original = await importOriginal<typeof import("@cloudscape-design/global-styles")>();
  return { ...original, applyMode: vi.fn() };
});

/** A fake `matchMedia` whose `matches` the test flips and whose listeners it fires. */
function fakeMatchMedia(initiallyDark: boolean) {
  const listeners = new Set<(event: MediaQueryListEvent) => void>();
  const media = {
    matches: initiallyDark,
    media: DARK_SCHEME_QUERY,
    addEventListener: vi.fn((_: string, listener: (event: MediaQueryListEvent) => void) => {
      listeners.add(listener);
    }),
    removeEventListener: vi.fn((_: string, listener: (event: MediaQueryListEvent) => void) => {
      listeners.delete(listener);
    }),
  };
  const matchMedia = vi.fn((query: string) => {
    expect(query).toBe(DARK_SCHEME_QUERY);
    return media as unknown as MediaQueryList;
  });
  const change = (dark: boolean) => {
    media.matches = dark;
    for (const listener of listeners) {
      listener({ matches: dark } as MediaQueryListEvent);
    }
  };
  return { matchMedia, media, change, listeners };
}

describe("useColorScheme (FR4.5)", () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    vi.mocked(applyMode).mockClear();
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it("applies the OS mode at mount and follows a change from light to dark", () => {
    const fake = fakeMatchMedia(false);
    window.matchMedia = fake.matchMedia;
    renderHook(() => useColorScheme());
    expect(applyMode).toHaveBeenLastCalledWith(Mode.Light);
    fake.change(true);
    expect(applyMode).toHaveBeenLastCalledWith(Mode.Dark);
    fake.change(false);
    expect(applyMode).toHaveBeenLastCalledWith(Mode.Light);
    expect(applyMode).toHaveBeenCalledTimes(3);
  });

  it("starts dark when the OS is dark and stops listening when unmounted", () => {
    const fake = fakeMatchMedia(true);
    window.matchMedia = fake.matchMedia;
    const { unmount } = renderHook(() => useColorScheme());
    expect(applyMode).toHaveBeenCalledWith(Mode.Dark);
    expect(fake.listeners.size).toBe(1);
    unmount();
    expect(fake.listeners.size).toBe(0);
  });

  it("stays light where matchMedia does not exist", () => {
    // @ts-expect-error jsdom may define it; the hook must cope without it.
    delete window.matchMedia;
    renderHook(() => useColorScheme());
    expect(applyMode).toHaveBeenCalledTimes(1);
    expect(applyMode).toHaveBeenCalledWith(Mode.Light);
  });
});
