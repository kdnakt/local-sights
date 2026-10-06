import type { DisplayRow, RowWindow, SessionView } from "../api";
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
