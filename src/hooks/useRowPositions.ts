import { useCallback, useEffect, useRef } from "react";
import { rowPositions } from "../api";
import { parseRowKey, type RowKey } from "./expansionState";

/** An answer of `row_positions`, with the positions by key. */
export interface RowPositionsAnswer {
  positions: ReadonlyMap<RowKey, number | null>;
  timelineVersion: number;
  resultVersion: number;
  discardGeneration: number;
  totalCount: number;
}

export interface RowPositionsOptions {
  /** The keys to ask for, read when a request is sent. */
  getKeys: () => readonly RowKey[];
  /** Called with every answer that is not older than one already seen. */
  onAnswer: (answer: RowPositionsAnswer) => void;
  onError: (error: unknown) => void;
}

interface Versions {
  timelineVersion: number;
  resultVersion: number;
  discardGeneration: number;
}

function isOlder(answer: Versions, known: Versions): boolean {
  return (
    answer.timelineVersion < known.timelineVersion ||
    answer.resultVersion < known.resultVersion ||
    answer.discardGeneration < known.discardGeneration
  );
}

/**
 * U7:BR1.7: asks the core for the positions of the expanded rows, the
 * selected row and the top row in one `row_positions` call. Only one call
 * is in flight: a request made meanwhile is sent once the answer is back,
 * with the keys of that moment, so the answer always fits the latest
 * versions (the keys are read when sending, never queued). An answer from
 * older versions than one already seen is dropped. Returns `request`.
 */
export function useRowPositions({ getKeys, onAnswer, onError }: RowPositionsOptions): () => void {
  const inFlight = useRef(false);
  const again = useRef(false);
  const disposed = useRef(false);
  const known = useRef<Versions>({ timelineVersion: 0, resultVersion: 0, discardGeneration: 0 });
  const callbacks = useRef({ getKeys, onAnswer, onError });
  /** The request itself, for the request made while one was in flight. */
  const requestRef = useRef<() => void>(() => undefined);

  useEffect(() => {
    callbacks.current = { getKeys, onAnswer, onError };
  }, [getKeys, onAnswer, onError]);

  useEffect(() => {
    disposed.current = false;
    return () => {
      disposed.current = true;
    };
  }, []);

  const request = useCallback(() => {
    if (inFlight.current) {
      again.current = true;
      return;
    }
    const asked = [...new Set(callbacks.current.getKeys())].flatMap((key) => {
      const parts = parseRowKey(key);
      return parts === null ? [] : [{ key, parts }];
    });
    if (asked.length === 0) {
      return;
    }
    inFlight.current = true;
    const settle = () => {
      inFlight.current = false;
      if (again.current && !disposed.current) {
        again.current = false;
        requestRef.current();
      }
    };
    rowPositions(asked.map(({ parts }) => parts)).then(
      (answer) => {
        const older = isOlder(answer, known.current);
        known.current = {
          timelineVersion: Math.max(known.current.timelineVersion, answer.timelineVersion),
          resultVersion: Math.max(known.current.resultVersion, answer.resultVersion),
          discardGeneration: Math.max(known.current.discardGeneration, answer.discardGeneration),
        };
        if (!older && !disposed.current) {
          const positions = new Map<RowKey, number | null>();
          asked.forEach(({ key }, index) => positions.set(key, answer.positions[index] ?? null));
          callbacks.current.onAnswer({
            positions,
            timelineVersion: answer.timelineVersion,
            resultVersion: answer.resultVersion,
            discardGeneration: answer.discardGeneration,
            totalCount: answer.totalCount,
          });
        }
        settle();
      },
      (error: unknown) => {
        if (!disposed.current) {
          callbacks.current.onError(error);
        }
        settle();
      },
    );
  }, []);

  useEffect(() => {
    requestRef.current = request;
  }, [request]);

  return request;
}
