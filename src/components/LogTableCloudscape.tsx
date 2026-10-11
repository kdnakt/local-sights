import { useEffect, useRef, useState } from "react";
import Box from "@cloudscape-design/components/box";
import Table from "@cloudscape-design/components/table";
import { getRows, type DisplayRow, type TimeZoneChoice } from "../api";
import { toSingleLine } from "../format";
import { rowKeyOf, type RowKey } from "../hooks/expansionState";
import { timeZoneLabelKey, type Translate } from "../i18n/messages";

/** Rows read from the core in one request while the list is loaded. */
const PAGE_SIZE = 5_000;
const EMPTY_ROWS: readonly DisplayRow[] = [];
const EMPTY_KEYS: ReadonlySet<RowKey> = new Set();

export interface LogTableCloudscapeProps {
  totalCount: number;
  timelineVersion: number;
  timeZone: TimeZoneChoice;
  filterId?: number | null;
  resultVersion?: number;
  discardGeneration?: number;
  t: Translate;
  onError: (error: unknown) => void;
  emptyText?: string;
}

/** One table item: an event, or the full message shown below an expanded event. */
type Item =
  | { kind: "event"; key: RowKey; row: DisplayRow }
  | { kind: "message"; key: RowKey; row: DisplayRow };

/**
 * Plan Step 10: the log list rebuilt with Cloudscape's Table, as a prototype
 * for the NFR2 measurement (100 万行; see README "Performance check of the
 * log list"). It is switched on with `VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape`
 * and is not the default: Cloudscape's Table draws every row, so the whole
 * list is read from the core in pages and kept in memory (AS3). Columns are
 * time / stream / message as in `LogTable` (FR4.3); a row expands with its
 * expand button, a click on the row, or Enter / Space on the button, showing
 * the whole message below it (several rows at once, FR4.4); the arrow keys
 * move between cells (Cloudscape keyboard navigation).
 */
export function LogTableCloudscape({
  totalCount,
  timelineVersion,
  timeZone,
  filterId = null,
  resultVersion = 0,
  discardGeneration = 0,
  t,
  onError,
  emptyText,
}: LogTableCloudscapeProps) {
  const [loadedRows, setRows] = useState<DisplayRow[]>([]);
  // The open rows, valid for one discard generation only (U7:BR1.6): when
  // the held logs are discarded, every expansion closes.
  const [expandedState, setExpandedState] = useState<{
    generation: number;
    keys: ReadonlySet<RowKey>;
  }>(() => ({ generation: discardGeneration, keys: new Set() }));
  const expanded: ReadonlySet<RowKey> =
    expandedState.generation === discardGeneration ? expandedState.keys : EMPTY_KEYS;
  const loadRef = useRef(0);
  const rows = totalCount === 0 ? EMPTY_ROWS : loadedRows;

  // Read the whole list page by page; a newer version restarts the read.
  useEffect(() => {
    loadRef.current += 1;
    const load = loadRef.current;
    if (totalCount === 0) {
      return;
    }
    let cancelled = false;
    const collected: DisplayRow[] = [];
    const next = (offset: number) => {
      if (cancelled || offset >= totalCount) {
        return;
      }
      getRows(offset, Math.min(PAGE_SIZE, totalCount - offset)).then(
        (answer) => {
          if (cancelled || load !== loadRef.current) {
            return;
          }
          collected.push(...answer.rows);
          setRows([...collected]);
          if (answer.rows.length > 0) {
            next(offset + answer.rows.length);
          }
        },
        (error: unknown) => {
          if (!cancelled) {
            onError(error);
          }
        },
      );
    };
    next(0);
    return () => {
      cancelled = true;
    };
  }, [totalCount, timelineVersion, timeZone, filterId, resultVersion, onError]);

  const items: Item[] = rows.map((row) => ({
    kind: "event",
    key: rowKeyOf(row.logStreamName, row.sequence),
    row,
  }));
  const expandedItems = items.filter((item) => expanded.has(item.key));

  const toggle = (item: Item) => {
    setExpandedState((previous) => {
      const copy = new Set(previous.generation === discardGeneration ? previous.keys : []);
      if (copy.has(item.key)) {
        copy.delete(item.key);
      } else {
        copy.add(item.key);
      }
      return { generation: discardGeneration, keys: copy };
    });
  };

  return (
    <div className="log-table-cloudscape" data-testid="log-table-cloudscape">
      <Table<Item>
        variant="embedded"
        items={items}
        trackBy={(item) => (item.kind === "event" ? item.key : `${item.key}:message`)}
        enableKeyboardNavigation
        stickyHeader
        expandableRows={{
          getItemChildren: (item) =>
            item.kind === "event" ? [{ kind: "message", key: item.key, row: item.row }] : [],
          isItemExpandable: (item) => item.kind === "event",
          expandedItems,
          onExpandableItemToggle: ({ detail }) => toggle(detail.item),
        }}
        onRowClick={({ detail }) => toggle(detail.item)}
        ariaLabels={{
          tableLabel: t("table.label"),
          expandButtonLabel: () => t("table.expanded"),
          collapseButtonLabel: () => t("table.expanded"),
        }}
        empty={
          <Box textAlign="center" color="text-body-secondary" data-testid="log-table-empty">
            {emptyText}
          </Box>
        }
        columnDefinitions={[
          {
            id: "time",
            header: t("table.time", { zone: t(timeZoneLabelKey(timeZone)) }),
            width: 220,
            cell: (item) => (item.kind === "event" ? item.row.displayTime : ""),
          },
          {
            id: "stream",
            header: t("table.stream"),
            width: 220,
            cell: (item) => (item.kind === "event" ? item.row.logStreamName : ""),
          },
          {
            id: "message",
            header: t("table.message"),
            cell: (item) =>
              item.kind === "event" ? (
                toSingleLine(item.row.message)
              ) : (
                <div
                  className="log-expanded-text"
                  role="region"
                  aria-label={t("table.expanded")}
                  data-testid="log-table-expanded"
                >
                  {item.row.message}
                </div>
              ),
          },
        ]}
      />
    </div>
  );
}
