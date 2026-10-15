import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ElapsedView } from "../api";
import { createTranslator } from "../i18n/messages";
import { t } from "../test/fixtures";
import { ElapsedTime } from "./ElapsedTime";

function running(elapsedMs: number, fetchNumber = 1): ElapsedView {
  return { running: true, elapsedMs, fetchNumber };
}

function stopped(elapsedMs: number, fetchNumber = 1): ElapsedView {
  return { running: false, elapsedMs, fetchNumber };
}

/** Moves the fake clock in steps, letting React redraw after each one. */
function advance(ms: number, step = 250) {
  for (let left = ms; left > 0; left -= step) {
    act(() => {
      vi.advanceTimersByTime(Math.min(step, left));
    });
  }
}

function shownText() {
  return screen.getByTestId("status-line-elapsed").textContent;
}

/** The whole seconds shown, read back from "Elapsed mm:ss". */
function shownSeconds() {
  const [minutes = Number.NaN, seconds = Number.NaN] = (shownText() ?? "")
    .replace("Elapsed ", "")
    .split(":")
    .map(Number);
  return minutes * 60 + seconds;
}

describe("ElapsedTime (261012-parallel-fetch U4)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("advances by the screen's clock while no new value arrives (NFR6.5, AC2.1.2)", () => {
    render(<ElapsedTime elapsed={running(5_000)} t={t} />);
    expect(shownText()).toBe("Elapsed 00:05");
    advance(16_000);
    expect(shownText()).toBe("Elapsed 00:21");
  });

  it("stops at the final value once the fetch ended", () => {
    const { rerender } = render(<ElapsedTime elapsed={running(30_000)} t={t} />);
    advance(2_000);
    rerender(<ElapsedTime elapsed={stopped(31_400)} t={t} />);
    expect(shownText()).toBe("Elapsed 00:31");
    advance(10_000);
    expect(shownText()).toBe("Elapsed 00:31");
    expect(vi.getTimerCount()).toBe(0);
  });

  it("shows nothing without an elapsed time, also for undefined", () => {
    const { container, rerender } = render(<ElapsedTime elapsed={null} t={t} />);
    expect(container).toBeEmptyDOMElement();
    rerender(<ElapsedTime elapsed={undefined} t={t} />);
    expect(container).toBeEmptyDOMElement();
    rerender(<ElapsedTime elapsed={running(1_000)} t={t} />);
    expect(shownText()).toBe("Elapsed 00:01");
    rerender(<ElapsedTime elapsed={null} t={t} />);
    expect(container).toBeEmptyDOMElement();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("is hidden from screen readers only while the fetch runs (NFR9.2, BR4.9)", () => {
    const { rerender } = render(<ElapsedTime elapsed={running(1_000)} t={t} />);
    expect(screen.getByTestId("status-line-elapsed")).toHaveAttribute("aria-hidden", "true");
    rerender(<ElapsedTime elapsed={stopped(2_000)} t={t} />);
    expect(screen.getByTestId("status-line-elapsed")).not.toHaveAttribute("aria-hidden");
  });

  it("stays within one second of the received value plus the time since (NFR6.7)", () => {
    const received = 12_345;
    render(<ElapsedTime elapsed={running(received)} t={t} />);
    let waitedSoFar = 0;
    for (const waited of [0, 400, 1_000, 2_750, 9_999, 30_000]) {
      advance(waited);
      waitedSoFar += waited;
      const expectedMs = received + waitedSoFar;
      expect(Math.abs(shownSeconds() * 1_000 - expectedMs)).toBeLessThan(1_000);
    }
  });

  it("keeps the shown value when a delayed view of the same fetch is smaller (NFR6.6)", () => {
    const { rerender } = render(<ElapsedTime elapsed={running(5_000)} t={t} />);
    advance(3_000);
    expect(shownText()).toBe("Elapsed 00:08");
    rerender(<ElapsedTime elapsed={running(7_600)} t={t} />);
    expect(shownText()).toBe("Elapsed 00:08");
    advance(1_000);
    expect(shownText()).toBe("Elapsed 00:09");
    rerender(<ElapsedTime elapsed={running(0, 2)} t={t} />);
    expect(shownText()).toBe("Elapsed 00:00");
  });

  it("clears its redraw when it is removed", () => {
    const { unmount } = render(<ElapsedTime elapsed={running(1_000)} t={t} />);
    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("reads the given clock and writes Japanese", () => {
    let clock = 100;
    const ja = createTranslator("ja");
    const { rerender } = render(<ElapsedTime elapsed={running(59_000)} t={ja} now={() => clock} />);
    expect(shownText()).toBe("経過 00:59");
    clock += 3_601_000;
    rerender(<ElapsedTime elapsed={running(59_000)} t={ja} now={() => clock} />);
    expect(shownText()).toBe("経過 1:01:00");
  });
});
