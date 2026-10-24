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
 * Since 261011-sso-login the session view carries the SSO login (`ssoLogin`)
 * and `startSsoLogin`, `cancelSsoLogin`, `dismissSsoLogin` and `openSsoUrl`
 * drive it; the app runs the AWS CLI, the screen only shows its state.
 * Since 261012-parallel-fetch U4 the session view carries the elapsed time of
 * the fetch (`elapsed`); `fetch-progress` does not (NFR5.4).
 * Since 261012-parallel-fetch U5 `cancelFetch` is [Cancel] (BR5.1) and the
 * session view says whether it is under way (`cancelling`), whether it can be
 * pressed (`fetchCancelAvailable`) and what the last cancelled fetch left
 * unread (`cancelNotice`, a count only, NFR8.13); a job can end `Cancelled`.
 * Since 261017-insights-query U1 the session view also carries the Log
 * Analytics screen (`insights`) and which screen's fetch runs (`fetchOwner`,
 * U1:BR2.3); `updateAnalyticsInput`, `runQuery`, `cancelQuery` and
 * `getQueryRows` drive it. The query is read and run by the core; a reading
 * problem comes back as the QueryError phase with its place, never as a
 * refusal (U1:BR3.9).
 * Since 261017-insights-query U2 a result carries its columns, the matched
 * count and the cut (U2:BR3.2-BR3.4, BR5.3), each row its cells for those
 * columns (U2:BR7.1), `getQueryRowDetail` gives the fields of one opened row
 * (U2:BR7.3), `getCompletionCandidates` the words the query field completes
 * (U2:BR9.1), and a run can end `EvaluationFailed` (U2:BR10.2).
 * Since 261022-small-features U1 the session view carries the language
 * preference (`language`, U1:BR1.3) and `saveSettings` saves it with the
 * cache setting in one call (U1:BR1.1).
 * Tests replace this module with `vi.mock`, so Tauri is never started.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { perfRows } from "./dev/perfRows";
import type { LanguagePreference } from "./i18n/messages";

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

export type JobStatus =
  "Running" | "Completed" | "CompletedWithFailures" | "Failed" | "Aborted" | "Cancelled";

/**
 * The notice of a cancelled fetch (261012-parallel-fetch U5:BR5.8): the
 * streams not read to their end, or `null` when the fetch was cancelled before
 * the stream listing ended.
 */
export interface CancelNoticeView {
  unfinishedStreams: number | null;
}

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

/** State of the SSO login (261011-sso-login); `Idle`: nothing is shown. */
export type SsoLoginState =
  | "Idle"
  | "Starting"
  | "Waiting"
  | "Succeeded"
  | "Failed"
  | "TimedOut"
  | "Cancelled"
  | "CliNotFound";

/** The SSO login as the core shows it (261011-sso-login, BR1.5-BR4.2). */
export interface SsoLoginView {
  /** Whether [Sign in with SSO] is shown: an auth error with an SSO profile (BR1.5). */
  available: boolean;
  /** Whether it can be pressed now (BR2.5). */
  canStart: boolean;
  state: SsoLoginState;
  /** The allow-listed sign-in URL the CLI printed (BR3.1). */
  verificationUrl: string | null;
  /** The verification code the CLI printed (BR3.1). */
  userCode: string | null;
  /** The exit code of a failed CLI (BR4.2). */
  exitCode: number | null;
  attemptId: number;
  /** `--profile <name>` for the terminal fallback text, or empty. */
  profileOption: string;
}

/**
 * The elapsed time of the current or last fetch (261012-parallel-fetch U4,
 * BR4.5): only a flag and numbers (NFR8.10). Sent with every session view,
 * never every second; the screen advances it by its own clock while
 * `running` (BR4.6, `elapsed.ts`).
 */
export interface ElapsedView {
  /** Whether the fetch still runs. */
  running: boolean;
  /** Milliseconds since the start when the view was made, or the final value. */
  elapsedMs: number;
  /** Number of the fetch; a running value is kept only for the same number. */
  fetchNumber: number;
}

export interface FailedStream {
  logStreamName: string;
  failure: ApiFailure;
}

/**
 * A stream of the query screen that could not be fetched, with its log group
 * (261017-insights-query U3:BR2.6): two log groups may have streams of one name.
 */
export interface GroupFailedStream extends FailedStream {
  logGroupName: string;
}

/** A log group of the query screen whose fetch failed, with why (U3:BR2.6). */
export interface GroupFailure {
  logGroupName: string;
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

/** Which screen the running fetch belongs to (261017-insights-query U1:BR2.3). */
export type FetchOwner = "None" | "LogManagement" | "LogAnalytics";

/** Phase of the last query run (261017-insights-query functional-spec 4). */
export type AnalyticsPhase =
  | "Idle"
  | "Fetching"
  | "Evaluating"
  | "Done"
  | "QueryError"
  | "FetchFailed"
  | "Cancelled"
  | "EvaluationFailed";

/**
 * A field of the Log Analytics screen. `logGroup` takes the chosen log groups as
 * a list (up to 50, U3:BR4.1; an empty list deselects every one), the others a text.
 */
export type AnalyticsField = "logGroup" | "startText" | "endText" | "queryText";

/** A place in the query text: line and column from 1, in characters. */
export interface QuerySpan {
  line: number;
  column: number;
  length: number;
}

/** Why a query cannot be run, found before anything is fetched (U1:BR3.9). */
export interface QueryProblem {
  kind: "Syntax" | "Unsupported";
  /** Message key of the text (`query.syntax.*` / `query.unsupported.*`). */
  messageKey: string;
  /** The word of the query it is about; only text taken from the query. */
  word: string | null;
  span: QuerySpan;
}

/** Progress of the running query (U1:BR2.4, U3:BR2.7). */
export interface InsightsProgress {
  /** The disk cache is being read. */
  readingCache: boolean;
  /** The log group being fetched from AWS, from 1; `null` before the first. */
  groupNumber: number | null;
  /** The chosen log groups. */
  groupCount: number;
  /** Log groups whose fetch failed so far. */
  failedGroupCount: number;
  /** Streams of the current log group's fetch. */
  selectedStreamCount: number;
  /** `null` while the streams are being listed. */
  plannedStreamCount: number | null;
  finishedStreamCount: number;
  /** Events fetched from AWS by the run so far. */
  eventCount: number;
  evaluatedCount: number;
  totalCount: number;
}

/**
 * The shown result; its rows are read with `runId` alone (U3:BR3.2: the core keeps
 * copies of them, so they stay readable while the next run fetches, U3:BR3.4;
 * 261020-carryover-fixes FR2.3.3).
 */
export interface ResultSummary {
  runId: number;
  /** Result rows shown (U2:BR7.2). */
  rowCount: number;
  evaluatedCount: number;
  /** The column headings: the fields' names as written (U2:BR3.2-BR3.4). */
  columns: string[];
  /** Rows that matched before the cut (U2:BR5.3). */
  matchedCount: number;
  /** The rows shown when a `limit` or the default 10,000 cut dropped some (U2:BR5.3). */
  truncatedAt: number | null;
}

/** The Log Analytics screen as the core shows it (261017-insights-query U1). */
export interface InsightsView {
  /** The chosen log groups, in the order chosen (U3:BR4.1). */
  selectedLogGroupNames: string[];
  /** The most log groups that can be chosen (50). */
  maxLogGroups: number;
  /** Every log group of the current list, ascending: Log Management's before its filter. */
  logGroupOptions: string[];
  /** How that list stands; `null` without a complete connection. */
  logGroupStatus: ListingStatus | null;
  startInput: DateTimeInput;
  endInput: DateTimeInput;
  queryText: string;
  /** The time range's reasons (`validation.*` keys). */
  rangeErrors: string[];
  /** The first reason why [Run query] cannot be pressed (U1:BR1.5). */
  runBlocker: string | null;
  canRun: boolean;
  runId: number;
  phase: AnalyticsPhase;
  progress: InsightsProgress | null;
  problem: QueryProblem | null;
  /** The failure of the whole fetch (FetchFailed): the first failed log group's. */
  failure: ApiFailure | null;
  /** Every log group whose fetch failed (FetchFailed, U3:BR2.6). */
  failedGroups: GroupFailure[];
  /** Some streams failed: the result is based on part of the logs (U1:BR2.5). */
  partial: boolean;
  /** The streams that failed, with their log group (U3:BR2.6). */
  failedStreams: GroupFailedStream[];
  listingFailure: ApiFailure | null;
  result: ResultSummary | null;
  elapsed: ElapsedView | null;
  cancelling: boolean;
  cancelAvailable: boolean;
}

/** One result row (U1:BR6.3): the event, its time in the chosen zone and `@log`. */
export interface QueryRow extends LogEvent {
  displayTime: string;
  logGroupName: string;
  /** One cell per result column; `null` without a value (U2:BR7.1). */
  values: (string | null)[];
}

/** One field of an opened row (U2:BR7.3). */
export interface QueryDetailField {
  /** A JSON field as the core writes it: backquoted when it holds a `.` and the like. */
  name: string;
  value: string | null;
}

/** The fields of one opened result row (U2:BR7.3). */
export interface QueryRowDetail {
  runId: number;
  index: number;
  /** `@timestamp`, `@log`, `@logStream`, `@message`, then the JSON fields by name; empty when stale. */
  fields: QueryDetailField[];
}

/** What a completion candidate is (U2:BR9.1). */
export type CompletionKind = "Command" | "Keyword" | "FixedField" | "JsonField";

/** One completion candidate of the query field (U2:BR9.1). */
export interface CompletionCandidate {
  text: string;
  kind: CompletionKind;
}

/** The result rows of one viewport request (U1:BR6.4). */
export interface QueryRowWindow {
  runId: number;
  offset: number;
  /** Empty when the run is not the shown result's. */
  rows: QueryRow[];
  totalCount: number;
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
  /**
   * The saved language preference (261022-small-features U1:BR1.3, BR1.4);
   * the screen decides the shown locale from it and the OS languages.
   */
  language: LanguagePreference;
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
  /** The SSO login (261011-sso-login). */
  ssoLogin: SsoLoginView;
  /**
   * The elapsed time of the current or last fetch (261012-parallel-fetch U4).
   * `null` when nothing was fetched yet, after a fetch stopped for closing
   * (BR4.3), and after a connection change cleared the previous result
   * (BR4.4).
   */
  elapsed: ElapsedView | null;
  /** [Cancel] was pressed and the fetch has not ended yet (U5:BR5.1). */
  cancelling: boolean;
  /** Whether [Cancel] can be pressed now (U5:BR5.2). */
  fetchCancelAvailable: boolean;
  /** The notice of the last fetch when it was cancelled (U5:BR5.8). */
  cancelNotice: CancelNoticeView | null;
  /** Which screen's fetch runs (261017-insights-query U1:BR2.3). */
  fetchOwner: FetchOwner;
  /** The Log Analytics screen (261017-insights-query U1). */
  insights: InsightsView;
}

/** Whether an SSO login is under way (Starting or Waiting). */
export function isSsoLoginRunning(view: SsoLoginView): boolean {
  return view.state === "Starting" || view.state === "Waiting";
}

/**
 * Whether the SSO login dialog is shown: while the login runs and for a
 * result to read (time-out, failure, no CLI). Idle, Cancelled and Succeeded
 * close it.
 */
export function isSsoLoginModalShown(view: SsoLoginView): boolean {
  return (
    isSsoLoginRunning(view) ||
    view.state === "Failed" ||
    view.state === "TimedOut" ||
    view.state === "CliNotFound"
  );
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

/** Event that carries the whole SessionView after every state change. */
export const SESSION_CHANGED = "session-changed";
/** Light event sent for each listing page and added page while fetching (U3 review R-05). */
export const FETCH_PROGRESS = "fetch-progress";
/** Light event sent while the log filter scans the held events (U5:BR3.6). */
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
 * `filter-progress` message (U5:BR3.6). The start of the cache write arrives
 * only as this message, so it also makes [Cancel] unavailable, as the core
 * does (261012-parallel-fetch U5:BR5.2).
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
      fetchCancelAvailable: view.fetchCancelAvailable && !update.cacheSaving,
    },
    update,
  );
}

/** Whether a rejected command answered with a core message key. */
export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { key?: unknown }).key === "string"
  );
}

/** Reads the current SessionView. */
export function getSession(): Promise<SessionView> {
  return invoke<SessionView>("get_session");
}

/** Changes one time input; refused while fetching (U1:BR1.4). */
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

/**
 * [Cancel] (261012-parallel-fetch U5:BR5.1): stops the running fetch, keeping
 * what was fetched. Always resolves; when it cannot be pressed nothing
 * happens. The new state arrives through `session-changed`.
 */
export function cancelFetch(): Promise<void> {
  return invoke<void>("cancel_fetch");
}

/**
 * Reads at most `limit` rows of the timeline from `offset`. With the
 * developer-only measurement harness on (`VITE_LOCAL_SIGHTS_PERF_ROWS`,
 * `dev/perfRows.ts`) the rows are generated here instead.
 */
export function getRows(offset: number, limit: number): Promise<RowWindow> {
  if (perfRows.enabled) {
    return Promise.resolve(perfRows.getRows(offset, limit));
  }
  return invoke<RowWindow>("get_rows", { offset, limit });
}

/** U7:BR1.7: the current positions of several rows in one call. */
export function rowPositions(keys: RowKeyParts[]): Promise<RowPositions> {
  if (perfRows.enabled) {
    return Promise.resolve(perfRows.rowPositions(keys));
  }
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
  if (perfRows.enabled) {
    perfRows.setFilter(text);
  }
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
 * Saves the cache setting and the language preference in one file (U6:BR1.3;
 * 261022-small-features U1:BR1.1, BR1.3); switching the cache off removes the
 * cached files. A failure is told in the dialog through `session-changed`
 * and changes neither (U1:BR1.6).
 */
export function saveSettings(enabled: boolean, language: LanguagePreference): Promise<void> {
  return invoke<void>("save_settings", { enabled, language });
}

/** Removes every cached file at once ([Clear cache], U6:BR1.4). */
export function clearCache(): Promise<void> {
  return invoke<void>("clear_cache");
}

/**
 * [Sign in with SSO] (261011-sso-login FR2.1): the app runs `aws sso login`.
 * Resolves with nothing; the login's state arrives through `session-changed`.
 */
export function startSsoLogin(): Promise<void> {
  return invoke<void>("start_sso_login");
}

/** [Cancel] in the login dialog: the CLI is stopped (FR2.4). */
export function cancelSsoLogin(): Promise<void> {
  return invoke<void>("cancel_sso_login");
}

/** [Close] on a login result, or closing the success notice. */
export function dismissSsoLogin(): Promise<void> {
  return invoke<void>("dismiss_sso_login");
}

/**
 * Opens the sign-in URL of the running login in the browser. The app opens
 * only the URL the CLI printed for this attempt (functional-design review).
 */
export function openSsoUrl(url: string): Promise<void> {
  return invoke<void>("open_sso_url", { url });
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

/**
 * Changes one field of the Log Analytics screen (U1:BR1.3, BR1.4); refused
 * while its query runs. The log groups go as a list of names (U3:BR4.1), the
 * other fields as a text. The new state arrives through `session-changed`.
 */
export function updateAnalyticsInput(
  field: AnalyticsField,
  value: string | readonly string[],
  generation: number,
): Promise<void> {
  return invoke<void>("update_analytics_input", { field, value, generation });
}

/**
 * [Run query] (U1 UC1): the core reads the query, then fetches and evaluates.
 * Resolves with nothing; a reading problem arrives as the QueryError phase.
 */
export function runQuery(): Promise<void> {
  return invoke<void>("run_query");
}

/** [Cancel] of the Log Analytics screen (UC4); nothing happens when it cannot be pressed. */
export function cancelQuery(): Promise<void> {
  return invoke<void>("cancel_query");
}

/** At most `limit` result rows from `offset` of the shown result (U1:BR6.4). */
export function getQueryRows(
  runId: number,
  offset: number,
  limit: number,
): Promise<QueryRowWindow> {
  return invoke<QueryRowWindow>("get_query_rows", { runId, offset, limit });
}

/** The fields of result row `index` of the shown result (U2:BR7.3). */
export function getQueryRowDetail(runId: number, index: number): Promise<QueryRowDetail> {
  return invoke<QueryRowDetail>("get_query_row_detail", { runId, index });
}

/** The words and field names the query field completes (U2:BR9.1, Should). */
export function getCompletionCandidates(): Promise<CompletionCandidate[]> {
  return invoke<CompletionCandidate[]>("get_completion_candidates");
}

/** The selector that identifies a profile row. */
export function selectorOf(profile: ConnectionProfile): ProfileSelector {
  return profile.kind === "Named" && profile.profileName !== null
    ? { kind: "Named", profileName: profile.profileName }
    : { kind: "SdkDefault" };
}

/** Listens to `session-changed`. */
export function onSessionChanged(handler: (view: SessionView) => void): Promise<UnlistenFn> {
  return listen<SessionView>(SESSION_CHANGED, (event) => handler(event.payload));
}

/** Listens to `fetch-progress` (U3 review R-05). */
export function onFetchProgress(
  handler: (update: FetchProgressUpdate) => void,
): Promise<UnlistenFn> {
  return listen<FetchProgressUpdate>(FETCH_PROGRESS, (event) => handler(event.payload));
}

/** Listens to `filter-progress` (U5:BR3.6). */
export function onFilterProgress(
  handler: (update: FilterProgressUpdate) => void,
): Promise<UnlistenFn> {
  return listen<FilterProgressUpdate>(FILTER_PROGRESS, (event) => handler(event.payload));
}
