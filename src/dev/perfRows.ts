/**
 * Developer-only measurement harness for NFR2 (plan Step 10, R-03). Nothing
 * here runs unless the Vite environment variable
 * `VITE_LOCAL_SIGHTS_PERF_ROWS` is set at `npm run tauri dev` time, for
 * example `VITE_LOCAL_SIGHTS_PERF_ROWS=1000000 npm run tauri dev`. Then
 * `api.getRows`, `api.rowPositions` and the log filter are answered here,
 * from generated rows (messages of 80 to 400 characters, plain and JSON),
 * without going through the core, and `App` shows that many rows in the log
 * list; the status line keeps showing the real (empty) session. With
 * `VITE_LOCAL_SIGHTS_PERF_GROW=1` the rows arrive 100,000 at a time, every
 * two seconds, to imitate a running fetch. `VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape`
 * switches the list to the Cloudscape Table prototype (`LogTableCloudscape`);
 * anything else keeps the app's virtual-scroll list. How to measure is in
 * README.md ("Performance check of the log list").
 *
 * Review R-03: the harness is a development aid only. It is also gated on
 * Vite's `import.meta.env.DEV`, so a production build (`vite build`,
 * `npm run tauri build`) never shows generated rows, whatever variables were
 * set when it was built.
 */

import { useSyncExternalStore } from "react";
import type { DisplayRow, RowKeyParts, RowPositions, RowWindow } from "../api";

/** The row count asked for, in a development build only (R-03). */
export function configuredPerfRowCount(env: ImportMetaEnv = import.meta.env): number {
  if (!env.DEV) {
    return 0;
  }
  const count = Number(env.VITE_LOCAL_SIGHTS_PERF_ROWS ?? "0");
  return Number.isFinite(count) && count > 0 ? Math.floor(count) : 0;
}

const ROW_COUNT = configuredPerfRowCount();
const GROWING = import.meta.env.VITE_LOCAL_SIGHTS_PERF_GROW === "1";
const GROW_STEP = 100_000;
const GROW_INTERVAL_MS = 2_000;
/** 2024/03/01 00:00:00.000 UTC. */
const FIRST_TIMESTAMP = Date.UTC(2024, 2, 1);
const STREAM_COUNT = 7;

/** Which log list `App` renders (plan Step 10: the prototype is opt-in). */
export type LogTableVariant = "virtual" | "cloudscape";

export const LOG_TABLE_VARIANT: LogTableVariant =
  import.meta.env.VITE_LOCAL_SIGHTS_LOG_TABLE === "cloudscape" ? "cloudscape" : "virtual";

const PLAIN_MESSAGES = [
  "START RequestId: 7f2c8d1e-4b6a-4e55-9d0e-0d2b6b1c9a{n} Version: $LATEST",
  "INFO request id={n} path=/api/orders/{n} status=200 took {ms} ms user=anonymous region=ap-northeast-1 cache=miss",
  "WARN connection timeout after 30s while calling downstream service orders-{n}; retrying with backoff (attempt 2 of 5)",
  "ERROR read timeout on stream {n}: the socket was closed by the peer before the response headers arrived; see trace {n}",
  "END RequestId: 7f2c8d1e-4b6a-4e55-9d0e-0d2b6b1c9a{n}\tREPORT Duration: {ms} ms\tBilled Duration: {ms} ms\tMemory Size: 512 MB\tMax Memory Used: 96 MB",
];

/** One generated row; deterministic for a given position. */
export function makePerfRow(position: number): DisplayRow {
  const timestamp = FIRST_TIMESTAMP + position * 37;
  const stream = position % STREAM_COUNT;
  const logStreamName = `2024/03/01/[$LATEST]${stream.toString(16).padStart(8, "a")}`;
  let message: string;
  if (position % 5 === 3) {
    const extra = "x".repeat(60 + (position % 7) * 40);
    message = JSON.stringify(
      {
        level: position % 2 === 0 ? "error" : "info",
        msg: `connection timeout after 30s (request ${position})`,
        requestId: `7f2c8d1e-4b6a-4e55-9d0e-${position.toString(16).padStart(12, "0")}`,
        path: `/api/orders/${position}`,
        tookMs: (position * 13) % 1500,
        detail: extra,
      },
      null,
      2,
    );
  } else {
    const template = PLAIN_MESSAGES[position % PLAIN_MESSAGES.length]!;
    message = template
      .replaceAll("{n}", String(position))
      .replaceAll("{ms}", String((position * 13) % 1500));
    const padding = 80 - message.length;
    if (padding > 0) {
      message += " " + "-".repeat(padding);
    }
  }
  return {
    timestamp,
    ingestionTime: null,
    message,
    logStreamName,
    sequence: Math.floor(position / STREAM_COUNT),
    displayTime: formatDisplayTime(timestamp),
  };
}

/** `yyyy/mm/dd hh:mm:ss.SSS` in UTC, as the core writes it (FR2.2). */
export function formatDisplayTime(timestamp: number): string {
  const date = new Date(timestamp);
  const pad = (value: number, width = 2) => String(value).padStart(width, "0");
  return (
    `${date.getUTCFullYear()}/${pad(date.getUTCMonth() + 1)}/${pad(date.getUTCDate())} ` +
    `${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}:${pad(date.getUTCSeconds())}.` +
    pad(date.getUTCMilliseconds(), 3)
  );
}

interface PerfState {
  held: number;
  version: number;
  filterText: string;
  /** Positions of the matching rows while a filter is in force, else `null`. */
  matching: number[] | null;
}

const state: PerfState = {
  held: GROWING ? Math.min(GROW_STEP, ROW_COUNT) : ROW_COUNT,
  version: 1,
  filterText: "",
  matching: null,
};
const listeners = new Set<() => void>();

function notify() {
  for (const listener of listeners) {
    listener();
  }
}

function recomputeMatching() {
  if (state.filterText === "") {
    state.matching = null;
    return;
  }
  const needle = state.filterText.toLowerCase();
  const matching: number[] = [];
  for (let position = 0; position < state.held; position += 1) {
    if (makePerfRow(position).message.toLowerCase().includes(needle)) {
      matching.push(position);
    }
  }
  state.matching = matching;
}

function positionOf(key: RowKeyParts): number | null {
  const stream = Number.parseInt(key.logStreamName.slice(-8), 16);
  if (Number.isNaN(stream)) {
    return null;
  }
  const position = key.sequence * STREAM_COUNT + stream;
  if (position < 0 || position >= state.held) {
    return null;
  }
  if (state.matching === null) {
    return position;
  }
  const index = binarySearch(state.matching, position);
  return index < 0 ? null : index;
}

function binarySearch(sorted: readonly number[], value: number): number {
  let low = 0;
  let high = sorted.length - 1;
  while (low <= high) {
    const middle = (low + high) >> 1;
    const found = sorted[middle]!;
    if (found === value) {
      return middle;
    }
    if (found < value) {
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return -1;
}

/** The generated rows as the screen's api boundary serves them. */
export const perfRows = {
  /** Whether the harness is on (the environment variable is set). */
  enabled: ROW_COUNT > 0,

  /** Rows of the current list: all held rows, or the matching ones. */
  total(): number {
    return state.matching === null ? state.held : state.matching.length;
  },

  version(): number {
    return state.version;
  },

  getRows(offset: number, limit: number): RowWindow {
    const total = this.total();
    const rows: DisplayRow[] = [];
    for (let index = offset; index < Math.min(total, offset + limit); index += 1) {
      rows.push(makePerfRow(state.matching === null ? index : state.matching[index]!));
    }
    return {
      offset,
      rows,
      totalCount: total,
      timelineVersion: state.version,
      filtered: state.matching !== null,
      allCount: state.held,
      resultVersion: state.matching === null ? null : state.version,
    };
  },

  rowPositions(keys: RowKeyParts[]): RowPositions {
    return {
      positions: keys.map(positionOf),
      timelineVersion: state.version,
      resultVersion: state.version,
      discardGeneration: 0,
      totalCount: this.total(),
    };
  },

  /** Filters the held rows by message, like the core does (U5:BR1.2). */
  setFilter(text: string): void {
    const trimmed = text.trim();
    if (trimmed === state.filterText) {
      return;
    }
    state.filterText = trimmed;
    recomputeMatching();
    state.version += 1;
    notify();
  },

  subscribe(listener: () => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

if (perfRows.enabled && GROWING && typeof setInterval === "function") {
  const timer = setInterval(() => {
    if (state.held >= ROW_COUNT) {
      clearInterval(timer);
      return;
    }
    state.held = Math.min(state.held + GROW_STEP, ROW_COUNT);
    recomputeMatching();
    state.version += 1;
    notify();
  }, GROW_INTERVAL_MS);
}

export interface PerfRowsView {
  enabled: boolean;
  total: number;
  version: number;
}

let snapshot: PerfRowsView = { enabled: perfRows.enabled, total: perfRows.total(), version: 1 };

function getSnapshot(): PerfRowsView {
  if (snapshot.total !== perfRows.total() || snapshot.version !== perfRows.version()) {
    snapshot = { enabled: perfRows.enabled, total: perfRows.total(), version: perfRows.version() };
  }
  return snapshot;
}

/** The harness state for `App`: disabled, or the row count and version to show. */
export function usePerfRows(): PerfRowsView {
  return useSyncExternalStore(perfRows.subscribe, getSnapshot, getSnapshot);
}
