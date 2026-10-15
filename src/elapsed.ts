/**
 * The elapsed time of a fetch on the screen (261012-parallel-fetch U4,
 * ADR-005): pure functions, so the time can be tested without a clock.
 * The core sends `SessionView.elapsed` only when the session view changes;
 * the screen keeps an anchor (the value it received and when, by its own
 * monotonic clock) and advances it every second while the fetch runs
 * (BR4.6). `formatElapsed` writes the value as mm:ss or h:mm:ss (BR4.7).
 */

import type { ElapsedView } from "./api";

/** What the screen counts the shown elapsed time from. */
export interface Anchor {
  /** The elapsed milliseconds at `receivedAt`. */
  baseMs: number;
  /** When the value was taken, by the screen's monotonic clock (ms). */
  receivedAt: number;
  /** Whether the fetch still runs, so the time advances. */
  running: boolean;
  /** The fetch the value belongs to (`ElapsedView.fetchNumber`). */
  fetchNumber: number;
}

const SECOND_MS = 1_000;

function twoDigits(value: number): string {
  return String(value).padStart(2, "0");
}

/**
 * BR4.7: whole seconds only; below one hour `mm:ss`, from one hour `h:mm:ss`
 * with the hours unpadded. A negative or unreadable value is 00:00.
 */
export function formatElapsed(ms: number): string {
  const totalSeconds = Number.isFinite(ms) && ms > 0 ? Math.floor(ms / SECOND_MS) : 0;
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) {
    return `${twoDigits(totalMinutes)}:${twoDigits(seconds)}`;
  }
  const hours = Math.floor(totalMinutes / 60);
  return `${hours}:${twoDigits(totalMinutes % 60)}:${twoDigits(seconds)}`;
}

/** The elapsed milliseconds to show at `nowMs`; never below the received value. */
export function shownMs(anchor: Anchor, nowMs: number): number {
  if (!anchor.running) {
    return anchor.baseMs;
  }
  return anchor.baseMs + Math.max(0, nowMs - anchor.receivedAt);
}

/** Milliseconds until the shown second changes (always 1 to 1,000). */
export function msUntilNextSecond(anchor: Anchor, nowMs: number): number {
  return SECOND_MS - (shownMs(anchor, nowMs) % SECOND_MS);
}

/**
 * The anchor after receiving `incoming` at `nowMs` (BR4.6, NFR6.6,
 * nfr-design review R-01):
 * - no elapsed time (`null`, or `undefined` from an older view): no anchor;
 * - a fetch that ended: its final value, which may be a little below the
 *   shown one;
 * - a running value of a fetch already seen as ended: ignored (a late view
 *   must not start the clock again);
 * - a running value of the same running fetch: the larger of it and the
 *   shown value, so the time never goes back while fetching;
 * - otherwise (a new fetch): the received value.
 */
export function nextAnchor(
  previous: Anchor | null,
  incoming: ElapsedView | null | undefined,
  nowMs: number,
): Anchor | null {
  if (incoming === null || incoming === undefined) {
    return null;
  }
  const { running, elapsedMs, fetchNumber } = incoming;
  const sameFetch = previous !== null && previous.fetchNumber === fetchNumber;
  if (running && sameFetch && !previous.running) {
    return previous;
  }
  const baseMs =
    running && sameFetch && previous.running
      ? Math.max(elapsedMs, shownMs(previous, nowMs))
      : elapsedMs;
  return { baseMs, receivedAt: nowMs, running, fetchNumber };
}
