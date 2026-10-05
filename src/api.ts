/**
 * Thin wrapper around the Tauri commands and events of the desktop app
 * (commands for operations, the `session-changed` event for pushed state).
 * Since U3 the screen holds no log events: it reads the rows of its viewport
 * with `getRows` (U3:BR4.3). Since U4 every time comes from the core already
 * written in the chosen time zone (U4:BR3.4): the screen converts nothing.
 * Tests replace this module with `vi.mock`, so Tauri is never started.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "Idle" | "Fetching" | "Done" | "Failed";

/**
 * Typed fields; the profile and log group are selected from lists (U2) and
 * there is no stream name since U3 (U3:BR6.1).
 */
export type InputField = "startText" | "endText";

export type ProfileSelector = { kind: "SdkDefault" } | { kind: "Named"; profileName: string };

export interface ConnectionProfile {
  kind: "SdkDefault" | "Named";
  profileName: string | null;
  defaultRegion: string | null;
}

export interface ConnectionSelection {
  profile: ProfileSelector | null;
  region: string | null;
}

export interface PendingConnectionChange {
  proposedProfile: ProfileSelector | null;
  proposedRegion: string | null;
}

export type ListingStatus = "Loading" | "Complete" | "Partial";

export type EmptyState = "NoGroups" | "NoMatches";

export type FailureKind =
  | "AuthRequired"
  | "AccessDenied"
  | "Throttled"
  | "Network"
  | "NotFound"
  | "InvalidInput"
  | "RegionMissing"
  | "Other";

/** The time zone of the inputs and the log list; Local at every launch (U4:BR1.1). */
export type TimeZoneChoice = "Local" | "Utc";

/** Why a non-empty input has no instant (U4:BR1.2, BR2.2). */
export type DateTimeInputError = "Format" | "NonexistentLocalTime";

/** One start or end input as the core holds it (U4:BR1.3). */
export interface DateTimeInput {
  /** The text in the chosen time zone, as shown in the field. */
  text: string;
  /** Epoch milliseconds when the text could be read. */
  instant: number | null;
  error: DateTimeInputError | null;
}

export interface ApiFailure {
  kind: FailureKind;
  /** Contains no secret credential and no access key ID (BR4.3). */
  safeDetail: string;
  retryable: boolean;
}

export type JobStatus = "Running" | "Completed" | "CompletedWithFailures" | "Failed" | "Aborted";

export interface JobSummary {
  jobId: number;
  status: JobStatus;
  eventCount: number;
  plannedStreamCount: number;
  finishedStreamCount: number;
  failedStreamCount: number;
  /** The failure of the whole job (the stream listing), when it failed. */
  failure: ApiFailure | null;
}

/** Progress of the running fetch (U3:BR6.2). */
export interface FetchProgress {
  seenStreamCount: number;
  selectedStreamCount: number;
  /** `null` while the streams are being listed. */
  plannedStreamCount: number | null;
  finishedStreamCount: number;
  eventCount: number;
}

export interface FailedStream {
  logStreamName: string;
  failure: ApiFailure;
}

export type StreamListingStatus = "Complete" | "StoppedEarly" | "Partial";

export interface LogGroupListView {
  status: ListingStatus;
  /** Names that pass the filter, ascending. */
  visibleGroups: string[];
  totalCount: number;
  emptyState: EmptyState | null;
  failure: ApiFailure | null;
}

export interface SessionView {
  sessionId: string;
  phase: Phase;
  timeZone: TimeZoneChoice;
  startInput: DateTimeInput;
  endInput: DateTimeInput;
  /** Message-catalog keys of the reasons why Fetch cannot be pressed. */
  validationErrors: string[];
  canFetch: boolean;
  currentJobId: number | null;
  eventCount: number;
  lastJob: JobSummary | null;
  profiles: ConnectionProfile[];
  regions: string[];
  /** Message keys of the "could not read" notices (file kind only). */
  catalogNotices: string[];
  connection: ConnectionSelection;
  pendingChange: PendingConnectionChange | null;
  logGroups: LogGroupListView | null;
  logGroupFilter: string;
  selectedLogGroupName: string | null;
  canChangeConnection: boolean;
  canReload: boolean;
  /** Changes with every added page and every discard (U3:BR4.5). */
  timelineVersion: number;
  /** Sent back with the operations that depend on the connection (BR6.7). */
  connectionGeneration: number;
  progress: FetchProgress | null;
  failedStreams: FailedStream[];
  listingStatus: StreamListingStatus | null;
  listingFailure: ApiFailure | null;
  failureListOpen: boolean;
}

export interface LogEvent {
  timestamp: number;
  ingestionTime: number | null;
  message: string;
  logStreamName: string;
  /** Position within its stream; with the stream name it identifies the event. */
  sequence: number;
}

/**
 * One row as the core gives it: the event and its time already written in the
 * chosen time zone, or the raw number when that cannot be written (U4:BR3.1).
 */
export interface DisplayRow extends LogEvent {
  displayTime: string;
}

/** The rows of one viewport request (U3:BR4.3, U4:BR3.4). */
export interface RowWindow {
  offset: number;
  rows: DisplayRow[];
  totalCount: number;
  timelineVersion: number;
}

/** The current position of one event (U3:BR4.4). */
export interface RowPosition {
  position: number | null;
  timelineVersion: number;
}

/** Error returned by a command: a message-catalog key. */
export interface CommandError {
  key: string;
}

export const SESSION_CHANGED = "session-changed";
export const FETCH_PROGRESS = "fetch-progress";

/**
 * The light message sent for every listing page and every added page while a
 * fetch runs, instead of the whole session view.
 */
export interface FetchProgressUpdate {
  jobId: number;
  progress: FetchProgress;
  eventCount: number;
  timelineVersion: number;
}

/**
 * Applies a progress message to the last session view. It applies only to
 * the running job; anything else (e.g. a message overtaken by the end of the
 * fetch) leaves the view as it is.
 */
export function withProgress(view: SessionView | null, update: FetchProgressUpdate) {
  if (view === null || view.phase !== "Fetching" || view.currentJobId !== update.jobId) {
    return view;
  }
  return {
    ...view,
    progress: update.progress,
    eventCount: update.eventCount,
    timelineVersion: Math.max(view.timelineVersion, update.timelineVersion),
  };
}

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { key?: unknown }).key === "string"
  );
}

export function getSession(): Promise<SessionView> {
  return invoke<SessionView>("get_session");
}

export function updateInput(field: InputField, value: string): Promise<SessionView> {
  return invoke<SessionView>("update_input", { field, value });
}

/**
 * Starts a fetch of the selected log group. Resolves with nothing: the
 * resulting state arrives only through `session-changed`, so a late response
 * can never overwrite newer state. `generation` is the connection generation
 * of the last session view; the core drops the request when it is old.
 */
export function startFetch(generation: number): Promise<void> {
  return invoke<void>("start_fetch", { generation });
}

/** Reads at most `limit` rows of the timeline from `offset`. */
export function getRows(offset: number, limit: number): Promise<RowWindow> {
  return invoke<RowWindow>("get_rows", { offset, limit });
}

/** Finds the current position of one event. */
export function findRowPosition(logStreamName: string, sequence: number): Promise<RowPosition> {
  return invoke<RowPosition>("find_row_position", { logStreamName, sequence });
}

/**
 * Switches the time zone (U4:BR1.4); accepted while fetching too. The new
 * state arrives through `session-changed`.
 */
export function selectTimeZone(timeZone: TimeZoneChoice): Promise<void> {
  return invoke<void>("select_time_zone", { timeZone });
}

/** Opens or closes the list of failed streams. */
export function setFailureListOpen(open: boolean): Promise<void> {
  return invoke<void>("set_failure_list_open", { open });
}

/** Chooses a profile. The new state arrives through `session-changed`. */
export function selectProfile(profile: ProfileSelector): Promise<void> {
  return invoke<void>("select_profile", { profile });
}

/** Chooses a region. */
export function selectRegion(region: string): Promise<void> {
  return invoke<void>("select_region", { region });
}

/** Applies the connection change awaiting confirmation. */
export function confirmConnectionChange(): Promise<void> {
  return invoke<void>("confirm_connection_change");
}

/** Drops the connection change awaiting confirmation. */
export function cancelConnectionChange(): Promise<void> {
  return invoke<void>("cancel_connection_change");
}

/** Lists the log groups of the current connection again. */
export function reloadLogGroups(generation: number): Promise<void> {
  return invoke<void>("reload_log_groups", { generation });
}

/** Changes the log group filter text. */
export function updateLogGroupFilter(text: string): Promise<void> {
  return invoke<void>("update_log_group_filter", { text });
}

/** Selects one log group. */
export function selectLogGroup(name: string, generation: number): Promise<void> {
  return invoke<void>("select_log_group", { name, generation });
}

/** The selector that identifies a profile row. */
export function selectorOf(profile: ConnectionProfile): ProfileSelector {
  return profile.kind === "Named" && profile.profileName !== null
    ? { kind: "Named", profileName: profile.profileName }
    : { kind: "SdkDefault" };
}

export function onSessionChanged(handler: (view: SessionView) => void): Promise<UnlistenFn> {
  return listen<SessionView>(SESSION_CHANGED, (event) => handler(event.payload));
}

export function onFetchProgress(
  handler: (update: FetchProgressUpdate) => void,
): Promise<UnlistenFn> {
  return listen<FetchProgressUpdate>(FETCH_PROGRESS, (event) => handler(event.payload));
}
