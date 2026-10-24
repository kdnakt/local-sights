import type {
  DisplayRow,
  InsightsView,
  QueryRow,
  QueryRowWindow,
  ResultSummary,
  RowWindow,
  SessionView,
  SsoLoginView,
} from "../api";
import { createTranslator } from "../i18n/messages";

/** English translator used by component tests. */
export const t = createTranslator("en");

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
    ...overrides,
  };
}

/**
 * A shown result with `rowCount` rows and the two default columns, nothing
 * cut (261017-insights-query U2:BR3.3).
 */
export function resultSummary(overrides: Partial<ResultSummary> = {}): ResultSummary {
  const rowCount = overrides.rowCount ?? 2;
  return {
    runId: 1,
    rowCount,
    evaluatedCount: 40,
    columns: ["@timestamp", "@message"],
    matchedCount: rowCount,
    truncatedAt: null,
    ...overrides,
  };
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
  return {
    ...event,
    logGroupName: "/aws/lambda/MyFunction",
    values: values ?? [event.displayTime, message],
  };
}

/** A `get_query_rows` answer cut from `rows`. */
export function queryRowWindow(
  rows: readonly QueryRow[],
  offset: number,
  limit: number,
  runId = 1,
): QueryRowWindow {
  return {
    runId,
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
