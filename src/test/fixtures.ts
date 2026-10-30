import type {
  DisplayRow,
  InsightsView,
  QueryRow,
  QueryRowWindow,
  QueryVocabulary,
  ResultSummary,
  RowWindow,
  SessionView,
  SsoLoginView,
  VocabularyKind,
} from "../api";
import { createTranslator } from "../i18n/messages";

/** English translator used by component tests. */
export const t = createTranslator("en");

function words(kind: VocabularyKind, supported: boolean, names: string[]): QueryVocabulary[] {
  return names.map((name) => ({ name, kind, supported, reserved: false }));
}

/**
 * The core's QueryCatalog as `get_query_catalog` returns it
 * (261025-insights-stats U2:BR4.1); `analytics.test.ts` checks it against
 * `catalog.rs`.
 */
export const QUERY_CATALOG: readonly QueryVocabulary[] = [
  ...words("command", true, ["fields", "filter", "sort", "limit", "stats"]),
  ...words("command", false, [
    "parse",
    "dedup",
    "display",
    "pattern",
    "diff",
    "unmask",
    "unnest",
    "lookup",
    "filterIndex",
    "source",
  ]),
  ...words("aggregateFunction", true, [
    "count",
    "count_distinct",
    "sum",
    "avg",
    "min",
    "max",
    "pct",
    "stddev",
    "earliest",
    "latest",
    "sortsFirst",
    "sortsLast",
  ]),
  ...words("function", true, ["toMillis", "bin"]),
  ...["by", "as", "and", "or", "not", "like", "in", "asc", "desc"].map((name) => ({
    name,
    kind: "keyword" as const,
    supported: true,
    reserved: ["and", "or", "not", "like", "in"].includes(name),
  })),
  ...words("fixedField", true, ["@timestamp", "@message", "@logStream", "@log"]),
];

export function sessionView(overrides: Partial<SessionView> = {}): SessionView {
  return {
    sessionId: "session-1",
    phase: "Idle",
    timeZone: "Local",
    startInput: { text: "", instant: null, error: null },
    endInput: { text: "", instant: null, error: null },
    validationErrors: [],
    canFetch: true,
    currentJobId: null,
    eventCount: 0,
    lastJob: null,
    profiles: [
      { kind: "SdkDefault", profileName: null, defaultRegion: null },
      { kind: "Named", profileName: "dev", defaultRegion: "ap-northeast-1" },
      { kind: "Named", profileName: "prod", defaultRegion: null },
    ],
    regions: ["ap-northeast-1", "eu-west-1", "us-east-1"],
    catalogNotices: [],
    connection: { profile: null, region: null },
    pendingChange: null,
    logGroups: null,
    logGroupFilter: "",
    selectedLogGroupName: null,
    canChangeConnection: true,
    canReload: false,
    timelineVersion: 0,
    connectionGeneration: 0,
    progress: null,
    failedStreams: [],
    listingStatus: null,
    listingFailure: null,
    failureListOpen: false,
    logFilter: "",
    filterSummary: null,
    filterResultVersion: 0,
    cacheEnabled: false,
    language: "system",
    cacheDirectory: "/Users/me/Library/Caches/dev.local-sights.app/log-cache",
    settingsDialog: "Closed",
    settingsNotice: null,
    canOpenSettings: true,
    cacheSaving: false,
    cacheNotices: [],
    discardGeneration: 0,
    closeConfirmation: "None",
    ssoLogin: ssoLoginView(),
    elapsed: null,
    cancelling: false,
    fetchCancelAvailable: false,
    cancelNotice: null,
    fetchOwner: "None",
    insights: insightsView(),
    export: { exportNumber: 0, status: "Idle", request: null, rowCount: null },
    exportNotice: null,
    canExportLogs: false,
    ...overrides,
  };
}

/** The Log Analytics part of a session view: nothing run yet, ready to run. */
export function insightsView(overrides: Partial<InsightsView> = {}): InsightsView {
  return {
    selectedLogGroupNames: ["/aws/lambda/MyFunction"],
    maxLogGroups: 50,
    logGroupOptions: ["/aws/ecs/web", "/aws/lambda/MyFunction"],
    logGroupStatus: "Complete",
    startInput: { text: "2024/01/02 03:04:05", instant: 1_704_164_645_000, error: null },
    endInput: { text: "2024/01/02 03:05:05", instant: 1_704_164_705_000, error: null },
    queryText: "filter @message like /ERROR/",
    rangeErrors: [],
    runBlocker: null,
    canRun: true,
    runId: 0,
    phase: "Idle",
    progress: null,
    problem: null,
    failure: null,
    failedGroups: [],
    partial: false,
    failedStreams: [],
    listingFailure: null,
    result: null,
    elapsed: null,
    cancelling: false,
    cancelAvailable: false,
    canExport: false,
    ...overrides,
  };
}

/**
 * A shown result with `rowCount` rows and the two default columns, nothing
 * cut (261017-insights-query U2:BR3.3): event rows unless `shape` is given.
 */
export function resultSummary(overrides: Partial<ResultSummary> = {}): ResultSummary {
  const rowCount = overrides.rowCount ?? 2;
  return {
    runId: 1,
    shape: "events",
    rowCount,
    evaluatedCount: 40,
    columns: ["@timestamp", "@message"],
    matchedCount: rowCount,
    truncatedAt: null,
    version: 1,
    stage: "final",
    coverage: null,
    ...overrides,
  };
}

/**
 * A partial result (261025-insights-stats U3:BR2.6): log group `groupNumber` of
 * `groupCount` under fetch, `included` events in it.
 */
export function partialSummary(
  { groupNumber = 1, groupCount = 1, included = 40 } = {},
  overrides: Partial<ResultSummary> = {},
): ResultSummary {
  return resultSummary({
    stage: "partial",
    coverage: { groupNumber, groupCount, includedEventCount: included },
    ...overrides,
  });
}

/** A result made after [Cancel] from what was fetched (U3:BR3.1). */
export function cancelledSummary(overrides: Partial<ResultSummary> = {}): ResultSummary {
  return resultSummary({ stage: "cancelledPartial", ...overrides });
}

/**
 * A result row as `get_query_rows` gives it, with the cells of the two
 * default columns unless `values` are given.
 */
export function queryRow(
  sequence: number,
  message = `ERROR ${sequence}`,
  values?: (string | null)[],
): QueryRow {
  const event = logEvent(sequence, message);
  const logGroupName = "/aws/lambda/MyFunction";
  return {
    ...event,
    logGroupName,
    values: values ?? [event.displayTime, message],
    // 261025-insights-stats U3:BR4.2: the log group, stream and order.
    rowKey: `${logGroupName}\u001f${event.logStreamName}\u001f${event.sequence}`,
  };
}

/**
 * An aggregate row as `get_query_rows` gives it (261025-insights-stats BR4.1): its
 * cells only, written by the core, and its key (U3:BR4.2), here its first cell.
 */
export function aggregateRow(values: (string | null)[], rowKey = values[0] ?? ""): QueryRow {
  return { values, rowKey };
}

/** A shown aggregates result of `columns` with `rowCount` rows (BR3.4: never cut in U1). */
export function aggregatesSummary(
  columns: string[],
  overrides: Partial<ResultSummary> = {},
): ResultSummary {
  return resultSummary({ shape: "aggregates", columns, ...overrides });
}

/** A `get_query_rows` answer cut from `rows`. */
export function queryRowWindow(
  rows: readonly QueryRow[],
  offset: number,
  limit: number,
  runId = 1,
  version = 1,
): QueryRowWindow {
  return {
    runId,
    version,
    offset,
    rows: rows.slice(offset, offset + limit),
    totalCount: rows.length,
  };
}

/** The SSO login part of a session view; nothing to show by default. */
export function ssoLoginView(overrides: Partial<SsoLoginView> = {}): SsoLoginView {
  return {
    available: false,
    canStart: false,
    state: "Idle",
    verificationUrl: null,
    userCode: null,
    exitCode: null,
    attemptId: 0,
    profileOption: "",
    ...overrides,
  };
}

/**
 * A row as `get_rows` gives it. `displayTime` stands for the text the core
 * wrote in the chosen zone; the screen must show it unchanged (U4:BR3.4).
 */
export function logEvent(
  sequence: number,
  message = `message ${sequence}`,
  logStreamName = "stream-a",
  displayTime = `core time ${sequence}`,
): DisplayRow {
  return {
    timestamp: 1_704_164_645_000 + sequence,
    ingestionTime: null,
    message,
    logStreamName,
    sequence,
    displayTime,
  };
}

/** A `get_rows` answer cut from `events`, as the core would give it. */
export function rowWindow(
  events: readonly DisplayRow[],
  offset: number,
  limit: number,
  timelineVersion = 1,
): RowWindow {
  return {
    offset,
    rows: events.slice(offset, offset + limit),
    totalCount: events.length,
    timelineVersion,
    filtered: false,
    allCount: events.length,
    resultVersion: null,
  };
}

/** A session connected to `dev` / ap-northeast-1 with a complete list. */
export function connectedView(overrides: Partial<SessionView> = {}): SessionView {
  return sessionView({
    connection: { profile: { kind: "Named", profileName: "dev" }, region: "ap-northeast-1" },
    canReload: true,
    logGroups: {
      status: "Complete",
      visibleGroups: ["/aws/ecs/web", "/aws/lambda/MyFunction"],
      totalCount: 2,
      emptyState: null,
      failure: null,
    },
    ...overrides,
  });
}
