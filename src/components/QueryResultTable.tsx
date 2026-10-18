import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type UIEvent } from "react";
import { firstLine } from "../analytics";
import {
  getQueryRowDetail,
  getQueryRows,
  type QueryRow,
  type QueryRowDetail,
  type QueryRowWindow,
  type ResultSummary,
  type TimeZoneChoice,
} from "../api";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";
import {
  ROW_HEIGHT,
  buildLayout,
  scrollHeight,
  scrollTopToReveal,
  visibleRange,
} from "../rowLayout";

/** Height of an opened row's details: its fields and values (scrolls inside). */
export const DETAIL_HEIGHT = 240;
/** Viewport height used until the list is measured (for example in tests). */
const FALLBACK_VIEWPORT = 440;

export interface QueryResultTableProps {
  result: ResultSummary;
  timeZone: TimeZoneChoice;
  t: Translate;
  onError: (error: unknown) => void;
}

/** The class of a column's cells: the time and the message keep their widths. */
function columnClass(name: string): string {
  if (name === "@timestamp") {
    return "query-result-time";
  }
  return name === "@message" ? "query-result-message" : "query-result-cell";
}

/** What a cell shows (U2:BR7.1): the first line of the message, any other value as it is. */
function cellText(row: QueryRow | null, index: number, name: string): string {
  const value = row?.values[index] ?? null;
  if (value === null) {
    return "";
  }
  return name === "@message" ? firstLine(value) : value;
}

/**
 * The rows of the last result (261017-insights-query U1:BR6.3, BR6.4, U2:BR7.1,
 * BR7.3, BR7.4, refined-mockups screen 5): one column per result column, in
 * its order (`fields`, or `@timestamp` and `@message`), `@timestamp` in the
 * chosen zone with the zone in its heading, `@message` by its first line,
 * other values as they are, a cell without a value empty. Only the rows that
 * cover the viewport are read, with `get_query_rows` and the result's run
 * number and holdings epoch; an answer for another result or an older
 * request is dropped, and the screen waits for the newer state. The row
 * positions come from the Log Management list's layout (`rowLayout.ts`).
 * ▶ or Enter / Space on the selected row opens it below its line with every
 * field and value, read with `get_query_row_detail`: the four fixed fields,
 * then the JSON fields by name (U2:BR7.3); ↑ and ↓ move the selection as on
 * Log Management. Rows can be opened several at once.
 */
export function QueryResultTable({ result, timeZone, t, onError }: QueryResultTableProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(FALLBACK_VIEWPORT);
  const [opened, setOpened] = useState<ReadonlySet<number>>(() => new Set());
  const [selected, setSelected] = useState<number | null>(null);
  const [answer, setAnswer] = useState<QueryRowWindow | null>(null);
  const [details, setDetails] = useState<ReadonlyMap<string, QueryRowDetail>>(() => new Map());
  const latestRequest = useRef(0);

  const layout = useMemo(
    () =>
      buildLayout(
        result.rowCount,
        [...opened].map((position) => ({ position, height: DETAIL_HEIGHT })),
      ),
    [result.rowCount, opened],
  );
  const range = visibleRange(layout, scrollTop, viewport);

  useEffect(() => {
    const element = listRef.current;
    if (element === null || typeof ResizeObserver === "undefined") {
      return undefined;
    }
    const observer = new ResizeObserver(() => {
      if (element.clientHeight > 0) {
        setViewport(element.clientHeight);
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const { runId, holdingsEpoch } = result;
  useEffect(() => {
    latestRequest.current += 1;
    const request = latestRequest.current;
    if (range.count <= 0) {
      return;
    }
    getQueryRows(runId, holdingsEpoch, range.first, range.count).then(
      (window) => {
        const current = window.runId === runId && window.holdingsEpoch === holdingsEpoch;
        if (request === latestRequest.current && current) {
          setAnswer(window);
        }
      },
      (error: unknown) => {
        if (request === latestRequest.current) {
          onError(error);
        }
      },
    );
  }, [runId, holdingsEpoch, range.first, range.count, timeZone, onError]);

  // U2:BR7.3: an opened row's fields, read once per result, row and zone.
  const detailKey = (position: number) => `${runId}:${holdingsEpoch}:${timeZone}:${position}`;
  const requested = useRef(new Set<string>());
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    for (const position of opened) {
      const key = `${runId}:${holdingsEpoch}:${timeZone}:${position}`;
      if (requested.current.has(key)) {
        continue;
      }
      requested.current.add(key);
      getQueryRowDetail(runId, holdingsEpoch, position).then(
        (detail) => {
          const current = detail.runId === runId && detail.holdingsEpoch === holdingsEpoch;
          if (mounted.current && current) {
            setDetails((previous) => new Map(previous).set(key, detail));
          }
        },
        (error: unknown) => {
          requested.current.delete(key);
          if (mounted.current) {
            onError(error);
          }
        },
      );
    }
  }, [opened, runId, holdingsEpoch, timeZone, onError]);

  const rowAt = (position: number): QueryRow | null => {
    if (answer === null || answer.runId !== runId || answer.holdingsEpoch !== holdingsEpoch) {
      return null;
    }
    return answer.rows[position - answer.offset] ?? null;
  };

  const toggle = (position: number) => {
    setSelected(position);
    setOpened((current) => {
      const next = new Set(current);
      if (!next.delete(position)) {
        next.add(position);
      }
      return next;
    });
  };

  const select = (position: number) => {
    const clamped = Math.min(Math.max(position, 0), result.rowCount - 1);
    setSelected(clamped);
    const top = scrollTopToReveal(layout, clamped, scrollTop, viewport);
    if (top !== scrollTop) {
      setScrollTop(top);
      if (listRef.current) {
        listRef.current.scrollTop = top;
      }
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (result.rowCount === 0) {
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      select(selected === null ? range.first : selected + step);
    } else if ((event.key === "Enter" || event.key === " ") && selected !== null) {
      event.preventDefault();
      toggle(selected);
    }
  };

  const zone = t(timeZoneLabelKey(timeZone));
  const heading = (name: string) =>
    name === "@timestamp" ? t("insights.results.column.timestamp", { zone }) : name;
  const positions = Array.from({ length: range.count }, (_, index) => range.first + index);

  return (
    <div className="query-result-table" data-testid="query-result-table">
      <div className="query-result-header" role="presentation" data-testid="query-result-header">
        <span className="query-result-toggle-cell" />
        {result.columns.map((name, index) => (
          <span key={`${index}:${name}`} className={columnClass(name)}>
            {heading(name)}
          </span>
        ))}
      </div>
      <div
        ref={listRef}
        className="query-result-list"
        role="grid"
        aria-label={t("insights.results.title")}
        aria-rowcount={result.rowCount}
        aria-colcount={result.columns.length + 1}
        tabIndex={0}
        onScroll={(event: UIEvent<HTMLDivElement>) => setScrollTop(event.currentTarget.scrollTop)}
        onKeyDown={handleKeyDown}
        data-testid="query-result-list"
      >
        <div style={{ height: scrollHeight(layout), position: "relative" }}>
          <div style={{ position: "absolute", top: range.offsetY, left: 0, right: 0 }}>
            {positions.map((position) => {
              const row = rowAt(position);
              const open = opened.has(position);
              return (
                <div
                  key={position}
                  role="row"
                  aria-rowindex={position + 1}
                  aria-selected={selected === position}
                  aria-expanded={open}
                  className={`query-result-row${selected === position ? " is-selected" : ""}`}
                  data-testid="query-result-row"
                  style={{ minHeight: ROW_HEIGHT }}
                >
                  <div className="query-result-line" onClick={() => setSelected(position)}>
                    <span className="query-result-toggle-cell" role="gridcell">
                      <button
                        type="button"
                        className="query-result-toggle"
                        aria-label={t(
                          open ? "insights.results.collapse" : "insights.results.expand",
                        )}
                        aria-expanded={open}
                        tabIndex={-1}
                        onClick={() => toggle(position)}
                        data-testid="query-result-toggle"
                      >
                        {open ? "▼" : "▶"}
                      </button>
                    </span>
                    {result.columns.map((name, index) => (
                      <span
                        key={`${index}:${name}`}
                        className={columnClass(name)}
                        role="gridcell"
                        data-testid="query-result-cell"
                      >
                        {cellText(row, index, name)}
                      </span>
                    ))}
                  </div>
                  {open && <RowDetails detail={details.get(detailKey(position)) ?? null} t={t} />}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

interface RowDetailsProps {
  detail: QueryRowDetail | null;
  t: Translate;
}

/** An opened row: every field with its value (U2:BR7.3), or that they are being read. */
function RowDetails({ detail, t }: RowDetailsProps) {
  return (
    <div
      className="query-result-details"
      role="gridcell"
      style={{ height: DETAIL_HEIGHT }}
      data-testid="query-result-details"
    >
      {detail === null ? (
        <span data-testid="query-result-details-loading">
          {t("insights.results.detailLoading")}
        </span>
      ) : (
        <table className="query-result-fields">
          <thead>
            <tr>
              <th scope="col">{t("insights.results.field")}</th>
              <th scope="col">{t("insights.results.value")}</th>
            </tr>
          </thead>
          <tbody>
            {detail.fields.map(({ name, value }) => (
              <tr key={name}>
                <th scope="row">{name}</th>
                <td className="query-result-value" data-testid={`query-result-field-${name}`}>
                  {value ?? ""}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
