/**
 * Thin wrapper around the Tauri commands and events of the desktop app
 * (commands for operations, the `session-changed` event for pushed state).
 * Since U3 the screen holds no log events: it reads the rows of its viewport
 * with `getRows` (U3:BR4.3). Since U4 every time comes from the core already
 * written in the chosen time zone (U4:BR3.4): the screen converts nothing.
 * Since U5 the core also filters the held events by message (U5:BR1.2):
 * `setLogFilter` hands it the text, and `session-changed`, `fetch-progress`
 * and `filter-progress` carry the filter summary with its result version.
 * Since U6 the settings dialog commands (`openSettings`, `cancelSettings`,
 * `saveSettings`, `clearCache`) and the cache fields of the session view and
 * of the progress message (U6:BR5.1, BR5.3).
 * Since U7 `rowPositions` asks the positions of several rows at once with the
 * discard generation (U7:BR1.6, BR1.7), the session view carries the discard
 * generation and the close confirmation, and `confirmClose` / `cancelClose`
 * answer it (U7:BR3.1-BR3.3). U3's `findRowPosition` was removed at the
 * code generation review (R-05): `rowPositions` replaces it. A request of
 * more keys than the core accepts is refused with an error key, never cut
 * short (review R-04).
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

/** Whether the whole timeline has been filtered yet (U5:BR1.5). */
export type FilterStatus = "Filtering" | "Ready";

/** What the screen shows of the log filter (U5:BR3.3, BR3.6). */
export interface FilterSummary {
  filterId: number;
  /** Matching events found so far. */
  matchedCount: number;
  /** Held events when the result was last updated. */
  allCount: number;
  status: FilterStatus;
  resultVersion: number;
}

/** Whether the settings dialog is open (U6:BR5.1). */
export type SettingsDialog = "Closed" | "Open";

/** What the settings dialog tells after an operation (U6:BR1.3, BR1.4). */
export type SettingsNotice = "Cleared" | "ClearFailed" | "SaveFailed";

/** Whether closing the window awaits an answer (U7:BR3.1). */
export type CloseConfirmation = "None" | "Pending";

/** One cache notice of the status line; wording in U6:BR5.3. */
export type CacheNotice = "Hit" | "ReadFailed" | "SaveFailed";

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
  /** The log filter text as typed (U5:BR3.4). */
  logFilter: string;
  /** `null` when no log filter is in force (U5:BR1.1). */
  filterSummary: FilterSummary | null;
  /** Never decreases; orders the filter messages (U5:BR3.6). */
  filterResultVersion: number;
  /** Whether the disk cache is enabled (U6:BR1.1). */
  cacheEnabled: boolean;
  /** The cache folder shown in the settings dialog (FR7.3). */
  cacheDirectory: string | null;
  settingsDialog: SettingsDialog;
  settingsNotice: SettingsNotice | null;
  /** False while fetching or while a connection change awaits confirmation. */
  canOpenSettings: boolean;
  /** While the fetched logs are written to the cache (U6:BR3.7). */
  cacheSaving: boolean;
  /** Cache notices of the last fetch (U6:BR5.3). */
  cacheNotices: CacheNotice[];
  /** Advances whenever the held logs are discarded (U7:BR1.6). */
  discardGeneration: number;
  /** Pending while the close confirmation is shown (U7:BR3.1). */
  closeConfirmation: CloseConfirmation;
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

/**
 * The rows of one viewport request (U3:BR4.3, U4:BR3.4). While a log filter is
 * in force, `offset`, `rows` and `totalCount` are those of the filter result
 * (U5:BR3.1).
 */
export interface RowWindow {
  offset: number;
  rows: DisplayRow[];
  totalCount: number;
  timelineVersion: number;
  filtered: boolean;
  /** Number of held events. */
  allCount: number;
  /** The filter result version the rows come from, when filtered. */
  resultVersion: number | null;
}

/** One held event, as `rowPositions` asks for it (U7:BR1.7). */
export interface RowKeyParts {
  logStreamName: string;
  sequence: number;
}

/**
 * The current positions of several rows, in the order asked, with the
 * versions they belong to (U7:BR1.7). A position is `null` when the row is
 * hidden by the log filter or no longer held.
 */
export interface RowPositions {
  positions: Array<number | null>;
  timelineVersion: number;
  resultVersion: number;
  discardGeneration: number;
  /** Rows of the current list (the filter result while a filter is in force). */
  totalCount: number;
}

/** Error returned by a command: a message-catalog key. */
export interface CommandError {
  key: string;
}

export const SESSION_CHANGED = "session-changed";
export const FETCH_PROGRESS = "fetch-progress";
export const FILTER_PROGRESS = "filter-progress";

/**
 * The light message sent for every listing page and every added page while a
 * fetch runs, instead of the whole session view.
 */
export interface FetchProgressUpdate {
  jobId: number;
  progress: FetchProgress;
  eventCount: number;
  timelineVersion: number;
  filterSummary: FilterSummary | null;
  filterResultVersion: number;
  /** While the fetched logs are written to the cache (U6:BR3.7). */
  cacheSaving: boolean;
}

/**
 * The light message sent while the log filter scan runs and when it becomes
 * Ready (U5:BR3.6).
 */
export interface FilterProgressUpdate {
  filterSummary: FilterSummary | null;
  filterResultVersion: number;
}

/**
 * Applies the filter part of a message to the last session view, unless the
 * view already shows a newer filter result (U5:BR3.6): messages may arrive
 * out of order.
 */
export function withFilter(view: SessionView | null, update: FilterProgressUpdate) {
  if (view === null || update.filterResultVersion < view.filterResultVersion) {
    return view;
  }
  return {
    ...view,
    filterSummary: update.filterSummary,
    filterResultVersion: update.filterResultVersion,
  };
}

/**
 * Takes a new session view, keeping the filter part of the previous one when
 * that is newer (U5:BR3.6).
 */
export function mergeSessionView(previous: SessionView | null, next: SessionView) {
  if (previous === null || next.filterResultVersion >= previous.filterResultVersion) {
    return next;
  }
  return {
    ...next,
    filterSummary: previous.filterSummary,
    filterResultVersion: previous.filterResultVersion,
  };
}

/**
 * Applies a progress message to the last session view. It applies only to
 * the running job; anything else (e.g. a message overtaken by the end of the
 * fetch) leaves the view as it is. Its filter part is applied like a
 * `filter-progress` message (U5:BR3.6).
 */
export function withProgress(view: SessionView | null, update: FetchProgressUpdate) {
  if (view === null || view.phase !== "Fetching" || view.currentJobId !== update.jobId) {
    return withFilter(view, update);
  }
  return withFilter(
    {
      ...view,
      progress: update.progress,
      eventCount: update.eventCount,
      timelineVersion: Math.max(view.timelineVersion, update.timelineVersion),
      cacheSaving: update.cacheSaving,
    },
    update,
  );
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

/** U7:BR1.7: the current positions of several rows in one call. */
export function rowPositions(keys: RowKeyParts[]): Promise<RowPositions> {
  return invoke<RowPositions>("row_positions", { keys });
}

/** [Keep fetching] or Escape on the close confirmation (U7:BR3.2). */
export function cancelClose(): Promise<void> {
  return invoke<void>("cancel_close");
}

/** [Close] on the close confirmation: stop fetching and end the app (U7:BR3.2). */
export function confirmClose(): Promise<void> {
  return invoke<void>("confirm_close");
}

/**
 * Switches the time zone (U4:BR1.4); accepted while fetching too. The new
 * state arrives through `session-changed`.
 */
export function selectTimeZone(timeZone: TimeZoneChoice): Promise<void> {
  return invoke<void>("select_time_zone", { timeZone });
}

/**
 * Hands the log filter text to the core (U5:BR1.1); accepted while fetching
 * too. The new state arrives through `session-changed` and `filter-progress`.
 */
export function setLogFilter(text: string): Promise<void> {
  return invoke<void>("set_log_filter", { text });
}

/**
 * Opens the settings dialog (U6:BR5.1). The new state arrives through
 * `session-changed`.
 */
export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

/** Closes the settings dialog without saving ([Cancel], Escape, U6:BR5.2). */
export function cancelSettings(): Promise<void> {
  return invoke<void>("cancel_settings");
}

/**
 * Saves the cache setting; switching it off removes the cached files
 * (U6:BR1.3). A failure is told in the dialog through `session-changed`.
 */
export function saveSettings(enabled: boolean): Promise<void> {
  return invoke<void>("save_settings", { enabled });
}

/** Removes every cached file at once ([Clear cache], U6:BR1.4). */
export function clearCache(): Promise<void> {
  return invoke<void>("clear_cache");
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

export function onFilterProgress(
  handler: (update: FilterProgressUpdate) => void,
): Promise<UnlistenFn> {
  return listen<FilterProgressUpdate>(FILTER_PROGRESS, (event) => handler(event.payload));
}
