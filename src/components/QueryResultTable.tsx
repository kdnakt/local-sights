import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type UIEvent } from "react";
import { firstLine } from "../analytics";
import {
  findQueryRow,
  getQueryRowDetail,
  getQueryRows,
  type QueryRow,
  type QueryRowDetail,
  type QueryRowWindow,
  type ResultSummary,
  type TimeZoneChoice,
} from "../api";
import {
  openedKeyAt,
  openedPositions,
  reopenedRows,
  toggleOpenedRow,
  type OpenedRows,
} from "../hooks/expansionState";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";
import {
  ROW_HEIGHT,
  buildLayout,
  rowOffset,
  scrollHeight,
  scrollTopToReveal,
  toScrollTop,
  visibleRange,
  type Layout,
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
  /**
   * The result is the previous run's, shown while the next one fetches and
   * evaluates (261017-insights-query U3:BR3.4): said in words above the rows,
   * which are not dimmed (interaction-spec).
   */
  previous?: boolean;
}

/**
 * Least width of one aggregate column (261025-insights-stats BR4.5): with many columns
 * the table scrolls sideways instead of squeezing them.
 */
const AGGREGATE_COLUMN_MIN_WIDTH_REM = 8;

/**
 * The class of a column's cells: in an events result the time and the message keep
 * their widths; in an aggregates result every column is a plain cell, whatever its name
 * (261025-insights-stats BR4.5: `by @message` and `by @timestamp` are keys).
 */
function columnClass(name: string, aggregates: boolean): string {
  if (aggregates) {
    return "query-result-cell";
  }
  if (name === "@timestamp") {
    return "query-result-time";
  }
  return name === "@message" ? "query-result-message" : "query-result-cell";
}

/**
 * What a cell shows (U2:BR7.1): the first line of the message, any other value as it
 * is. An aggregate cell is shown as the core wrote it, a `by @message` key in full
 * (261025-insights-stats BR4.5).
 */
function cellText(row: QueryRow | null, index: number, name: string, aggregates: boolean): string {
  const value = row?.values[index] ?? null;
  if (value === null) {
    return "";
  }
  return !aggregates && name === "@message" ? firstLine(value) : value;
}

/** The layout of `rowCount` rows with the rows of `opened` open. */
function layoutOf(rowCount: number, opened: OpenedRows): Layout {
  return buildLayout(
    rowCount,
    openedPositions(opened).map((position) => ({ position, height: DETAIL_HEIGHT })),
  );
}

/** No row open. */
const NONE_OPEN: OpenedRows = new Map();

/** The opened rows, for the run they were opened in (another run's mean nothing). */
interface OpenedState {
  runId: number;
  rows: OpenedRows;
}

/** What the last drawn table showed, read when a newer version comes (BR2.8). */
interface Shown {
  answer: QueryRowWindow | null;
  opened: OpenedRows;
  first: number;
  viewport: number;
}

/**
 * The rows of the last result (261017-insights-query U1:BR6.3, BR6.4, U2:BR7.1,
 * BR7.3, BR7.4, refined-mockups screen 5): one column per result column, in
 * its order (`fields`, or `@timestamp` and `@message`), `@timestamp` in the
 * chosen zone with the zone in its heading, `@message` by its first line,
 * other values as they are, a cell without a value empty. Only the rows that
 * cover the viewport are read, with `get_query_rows` and the result's run
 * number, which alone tells one result from another (261020-carryover-fixes
 * FR2.3.3: a connection change clears the result, so one run number never
 * names two holdings); an answer for another run or an older request is
 * dropped, and the screen waits for the newer state. The row
 * positions come from the Log Management list's layout (`rowLayout.ts`).
 * ▶ or Enter / Space on the selected row opens it below its line with every
 * field and value, read with `get_query_row_detail`: the four fixed fields,
 * then the JSON fields by name (U2:BR7.3); ↑ and ↓ move the selection as on
 * Log Management. Rows can be opened several at once.
 *
 * 261025-insights-stats BR4.5, ADR-006: the summary's `shape` picks how the rows are
 * drawn, in the same place. An aggregates result (`stats`) shows its columns as named,
 * with no zone added to a heading and no column cut to its first line, and an opened row
 * lists its columns with their cells. Many columns make the table scroll sideways.
 *
 * 261025-insights-stats U3: a result now comes in versions (partial results while
 * fetching, then the final or the cancelled one). The rows and an opened row's fields
 * are read with `(runId, version)` and an answer for another is dropped (BR2.4). Opened
 * rows are kept by the core's row key, not by place: when a newer version of the same
 * run comes, the place of each opened row and of the row shown first is asked with
 * `find_query_row`; an opened row found there stays open at its new place, one not
 * found is closed, and the list scrolls so that the row shown first is first again
 * (BR2.8). Rows opened in another run are not kept.
 */
export function QueryResultTable({
  result,
  timeZone,
  t,
  onError,
  previous = false,
}: QueryResultTableProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(FALLBACK_VIEWPORT);
  const { runId, version, rowCount } = result;
  const [openedState, setOpenedState] = useState<OpenedState>(() => ({
    runId,
    rows: NONE_OPEN,
  }));
  const opened = openedState.runId === runId ? openedState.rows : NONE_OPEN;
  const [selected, setSelected] = useState<number | null>(null);
  const [answer, setAnswer] = useState<QueryRowWindow | null>(null);
  const [details, setDetails] = useState<ReadonlyMap<string, QueryRowDetail>>(() => new Map());
  const latestRequest = useRef(0);

  const layout = useMemo(() => layoutOf(rowCount, opened), [rowCount, opened]);
  const range = visibleRange(layout, scrollTop, viewport);
  const openPositions = useMemo(() => new Set(openedPositions(opened)), [opened]);

  // BR2.8: a newer version of the same run keeps the opened rows and the row shown
  // first by their keys. Declared before the effect below that records what is shown,
  // so it reads what the previous version showed.
  const placed = useRef({ runId, version });
  const shown = useRef<Shown>({ answer: null, opened, first: 0, viewport });
  useEffect(() => {
    const before = placed.current;
    placed.current = { runId, version };
    if (before.runId !== runId || before.version === version) {
      return undefined;
    }
    const last = shown.current;
    const top = last.answer?.rows[last.first - last.answer.offset]?.rowKey ?? null;
    const keys = [...new Set([...last.opened.keys(), ...(top === null ? [] : [top])])];
    if (keys.length === 0) {
      return undefined;
    }
    let current = true;
    Promise.all(
      keys.map(async (key) => [key, await findQueryRow(runId, version, key)] as const),
    ).then(
      (pairs) => {
        if (!current) {
          return;
        }
        const found = new Map(pairs);
        const rows = reopenedRows(shown.current.opened, found);
        setOpenedState({ runId, rows });
        const first = top === null ? null : (found.get(top) ?? null);
        if (first !== null) {
          const next = layoutOf(rowCount, rows);
          const to = toScrollTop(next, rowOffset(next, first), shown.current.viewport);
          setScrollTop(to);
          if (listRef.current) {
            listRef.current.scrollTop = to;
          }
        }
      },
      (error: unknown) => {
        if (current) {
          onError(error);
        }
      },
    );
    return () => {
      current = false;
    };
  }, [runId, version, rowCount, onError]);
  useEffect(() => {
    shown.current = { answer, opened, first: range.first, viewport };
  });

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

  useEffect(() => {
    latestRequest.current += 1;
    const request = latestRequest.current;
    if (range.count <= 0) {
      return;
    }
    getQueryRows(runId, version, range.first, range.count).then(
      (window) => {
        // BR2.4: an answer for another run or version is dropped.
        if (
          request === latestRequest.current &&
          window.runId === runId &&
          window.version === version
        ) {
          setAnswer(window);
        }
      },
      (error: unknown) => {
        if (request === latestRequest.current) {
          onError(error);
        }
      },
    );
  }, [runId, version, range.first, range.count, timeZone, onError]);

  // U2:BR7.3: an opened row's fields, read once per result version, row and zone
  // (261025-insights-stats U3:BR2.8: a newer version reads them again).
  const detailKey = (position: number) => `${runId}:${version}:${timeZone}:${position}`;
  const requested = useRef(new Set<string>());
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    for (const position of openPositions) {
      const key = `${runId}:${version}:${timeZone}:${position}`;
      if (requested.current.has(key)) {
        continue;
      }
      requested.current.add(key);
      getQueryRowDetail(runId, version, position).then(
        (detail) => {
          if (mounted.current && detail.runId === runId && detail.version === version) {
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
  }, [openPositions, runId, version, timeZone, onError]);

  const rowAt = (position: number): QueryRow | null => {
    if (answer === null || answer.runId !== runId || answer.version !== version) {
      return null;
    }
    return answer.rows[position - answer.offset] ?? null;
  };

  const toggle = (position: number) => {
    setSelected(position);
    // BR2.8: kept by the row's key; a row not read yet by its place until it is.
    const key = openedKeyAt(opened, position) ?? rowAt(position)?.rowKey ?? `\u0000${position}`;
    setOpenedState({ runId, rows: toggleOpenedRow(opened, key, position) });
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

  const aggregates = result.shape === "aggregates";
  const zone = t(timeZoneLabelKey(timeZone));
  const heading = (name: string) =>
    !aggregates && name === "@timestamp" ? t("insights.results.column.timestamp", { zone }) : name;
  const positions = Array.from({ length: range.count }, (_, index) => range.first + index);
  // BR4.5: the header and the rows keep the same least width, so they scroll together.
  const wide = aggregates
    ? { minWidth: `calc(28px + ${result.columns.length * AGGREGATE_COLUMN_MIN_WIDTH_REM}rem)` }
    : undefined;

  return (
    <div
      className={`query-result-table${aggregates ? " is-aggregates" : ""}`}
      data-shape={result.shape}
      data-testid="query-result-table"
    >
      {previous && (
        <div className="query-result-previous" data-testid="query-result-previous">
          {t("insights.results.previous")}
        </div>
      )}
      <div
        className="query-result-header"
        role="presentation"
        style={wide}
        data-testid="query-result-header"
      >
        <span className="query-result-toggle-cell" />
        {result.columns.map((name, index) => (
          <span
            key={`${index}:${name}`}
            className={columnClass(name, aggregates)}
            data-testid="query-result-heading"
          >
            {heading(name)}
          </span>
        ))}
      </div>
      <div
        ref={listRef}
        className="query-result-list"
        style={wide}
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
              const open = openPositions.has(position);
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
                        className={columnClass(name, aggregates)}
                        role="gridcell"
                        data-testid="query-result-cell"
                      >
                        {cellText(row, index, name, aggregates)}
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
