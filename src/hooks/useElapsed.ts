import { useEffect, useState } from "react";
import type { ElapsedView } from "../api";
import { msUntilNextSecond, nextAnchor, shownMs, type Anchor } from "../elapsed";

/** What the elapsed time part shows. */
export interface ElapsedDisplay {
  /** The elapsed milliseconds to show now. */
  ms: number;
  /** Whether the time still advances (the fetch runs). */
  running: boolean;
}

/** The screen's monotonic clock (BR4.10); read at call time so tests can fake it. */
function performanceNow(): number {
  return performance.now();
}

/**
 * The elapsed time of the fetch to show (261012-parallel-fetch U4:BR4.6,
 * nfr-design review R-03). The anchor lives in state and is replaced while
 * rendering when the `elapsed` object differs from the one of the previous
 * render (React's "adjust state when a prop changes"); the composed views of
 * `fetch-progress` and `filter-progress` keep the same object, so they do not
 * realign the clock (NFR6.9). While the fetch runs, a timeout redraws when the
 * shown second changes; it is cleared when the fetch ends, when `elapsed`
 * becomes empty and when the part unmounts. `undefined` counts as `null`.
 * `now` is the monotonic clock in milliseconds (`performance.now` by default).
 */
export function useElapsed(
  elapsed: ElapsedView | null | undefined,
  now: () => number = performanceNow,
): ElapsedDisplay | null {
  const incoming = elapsed ?? null;
  const [received, setReceived] = useState(incoming);
  const [anchor, setAnchor] = useState<Anchor | null>(() => nextAnchor(null, incoming, now()));
  const [tick, setTick] = useState(0);

  let current = anchor;
  if (incoming !== received) {
    current = nextAnchor(anchor, incoming, now());
    setReceived(incoming);
    setAnchor(current);
  }

  useEffect(() => {
    if (anchor === null || !anchor.running) {
      return undefined;
    }
    const timer = setTimeout(() => setTick((value) => value + 1), msUntilNextSecond(anchor, now()));
    return () => clearTimeout(timer);
  }, [anchor, tick, now]);

  if (current === null) {
    return null;
  }
  return { ms: shownMs(current, now()), running: current.running };
}
