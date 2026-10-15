import { describe, expect, it } from "vitest";
import type { ElapsedView } from "./api";
import { formatElapsed, msUntilNextSecond, nextAnchor, shownMs, type Anchor } from "./elapsed";

function running(elapsedMs: number, fetchNumber = 1): ElapsedView {
  return { running: true, elapsedMs, fetchNumber };
}

function stopped(elapsedMs: number, fetchNumber = 1): ElapsedView {
  return { running: false, elapsedMs, fetchNumber };
}

describe("formatElapsed (261012-parallel-fetch U4:BR4.7)", () => {
  it.each([
    [0, "00:00"],
    [999, "00:00"],
    [7_999, "00:07"],
    [59_999, "00:59"],
    [3_599_999, "59:59"],
    [3_600_000, "1:00:00"],
    [75 * 60_000, "1:15:00"],
    [10 * 3_600_000, "10:00:00"],
    [100 * 3_600_000 + 61_000, "100:01:01"],
  ])("writes %i ms as %s", (ms, text) => {
    expect(formatElapsed(ms)).toBe(text);
  });

  it("writes a negative or unreadable value as 00:00", () => {
    expect(formatElapsed(-1)).toBe("00:00");
    expect(formatElapsed(-60_000)).toBe("00:00");
    expect(formatElapsed(Number.NaN)).toBe("00:00");
  });
});

describe("nextAnchor (261012-parallel-fetch U4:BR4.6, NFR6.6)", () => {
  it("has no anchor without an elapsed time, also for undefined", () => {
    const previous: Anchor = { baseMs: 1_000, receivedAt: 0, running: true, fetchNumber: 1 };
    expect(nextAnchor(previous, null, 5_000)).toBeNull();
    expect(nextAnchor(previous, undefined, 5_000)).toBeNull();
    expect(nextAnchor(null, undefined, 5_000)).toBeNull();
  });

  it("starts from the received value at a new fetch", () => {
    expect(nextAnchor(null, running(250, 3), 10_000)).toEqual({
      baseMs: 250,
      receivedAt: 10_000,
      running: true,
      fetchNumber: 3,
    });
  });

  it("keeps a larger shown value when the same fetch reports a smaller one", () => {
    const first = nextAnchor(null, running(5_000), 1_000);
    // 3 s later the screen shows 8,000 ms; a delayed view says 7,600 ms.
    const next = nextAnchor(first, running(7_600), 4_000);
    expect(next).toEqual({ baseMs: 8_000, receivedAt: 4_000, running: true, fetchNumber: 1 });
  });

  it("takes a larger value of the same fetch", () => {
    const first = nextAnchor(null, running(5_000), 1_000);
    const next = nextAnchor(first, running(20_000), 3_000);
    expect(next?.baseMs).toBe(20_000);
    expect(shownMs(next as Anchor, 3_000)).toBe(20_000);
  });

  it("starts again from the received value when the fetch number changes", () => {
    const finished = nextAnchor(null, stopped(31_400, 1), 1_000);
    const fresh = nextAnchor(finished, running(0, 2), 2_000);
    expect(fresh).toEqual({ baseMs: 0, receivedAt: 2_000, running: true, fetchNumber: 2 });
    expect(formatElapsed(shownMs(fresh as Anchor, 2_000))).toBe("00:00");

    // A running anchor of another fetch is not kept either.
    const other = nextAnchor(nextAnchor(null, running(9_000, 4), 0), running(1_000, 5), 3_000);
    expect(other?.baseMs).toBe(1_000);
  });

  it("replaces the shown value with the final one, even when it goes back", () => {
    const first = nextAnchor(null, running(30_000), 0);
    // The screen shows 32,000 ms; the end says 31,400 ms.
    const end = nextAnchor(first, stopped(31_400), 2_000);
    expect(end).toEqual({ baseMs: 31_400, receivedAt: 2_000, running: false, fetchNumber: 1 });
    expect(shownMs(end as Anchor, 60_000)).toBe(31_400);
  });

  it("ignores a late running value of a fetch already seen as ended (nfr-design R-01)", () => {
    const end = nextAnchor(nextAnchor(null, running(30_000), 0), stopped(31_400), 2_000);
    const late = nextAnchor(end, running(31_000), 3_000);
    expect(late).toBe(end);
    expect(shownMs(late as Anchor, 10_000)).toBe(31_400);
  });
});

describe("shownMs and msUntilNextSecond", () => {
  const anchor: Anchor = { baseMs: 5_000, receivedAt: 1_000, running: true, fetchNumber: 1 };

  it("advances a running anchor by the time since it was received", () => {
    expect(shownMs(anchor, 1_000)).toBe(5_000);
    expect(shownMs(anchor, 17_000)).toBe(21_000);
  });

  it("never goes below the received value when the clock is behind", () => {
    expect(shownMs(anchor, 500)).toBe(5_000);
  });

  it("does not advance a stopped anchor", () => {
    expect(shownMs({ ...anchor, running: false }, 99_000)).toBe(5_000);
  });

  it("tells the time until the shown second changes", () => {
    expect(msUntilNextSecond(anchor, 1_000)).toBe(1_000);
    expect(msUntilNextSecond(anchor, 1_250)).toBe(750);
    expect(msUntilNextSecond({ ...anchor, baseMs: 5_999 }, 1_000)).toBe(1);
  });
});
