/**
 * Thin wrapper around the Tauri commands and events of the desktop app
 * (Q1: commands for operations, events for pushed state and log pages).
 * Tests replace this module with `vi.mock`, so Tauri is never started.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "Idle" | "Fetching" | "Done" | "Failed";

/** Typed fields; the profile and log group are selected from lists (U2). */
export type InputField = "logStreamName" | "startText" | "endText";

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

export interface FetchInput {
  profileName: string;
  logGroupName: string;
  logStreamName: string;
  startText: string;
  endText: string;
}

export interface ApiFailure {
  kind: FailureKind;
  /** Contains no secret credential and no access key ID (BR4.3). */
  safeDetail: string;
  retryable: boolean;
}

export interface JobSummary {
  jobId: string;
  status: "Running" | "Completed" | "Failed";
  eventCount: number;
  pageCount: number;
  failure: ApiFailure | null;
}

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
  input: FetchInput;
  /** Message-catalog keys of the reasons why Fetch cannot be pressed. */
  validationErrors: string[];
  canFetch: boolean;
  currentJobId: string | null;
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
  /** Changes when a connection change discards the shown logs. */
  timelineGeneration: number;
}

export interface LogEvent {
  timestamp: number;
  ingestionTime: number | null;
  message: string;
  logStreamName: string;
  sequence: number;
}

export interface LogBatch {
  jobId: string;
  events: LogEvent[];
  total: number;
}

/** Error returned by a command: a message-catalog key. */
export interface CommandError {
  key: string;
}

export const SESSION_CHANGED = "session-changed";
export const LOG_BATCH = "log-batch";

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
 * Starts a fetch. Resolves with nothing: the resulting state arrives only
 * through `session-changed` and `log-batch`, so a late response can never
 * overwrite newer state.
 */
export function startFetch(): Promise<void> {
  return invoke<void>("start_fetch");
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
export function reloadLogGroups(): Promise<void> {
  return invoke<void>("reload_log_groups");
}

/** Changes the log group filter text. */
export function updateLogGroupFilter(text: string): Promise<void> {
  return invoke<void>("update_log_group_filter", { text });
}

/** Selects one log group. */
export function selectLogGroup(name: string): Promise<void> {
  return invoke<void>("select_log_group", { name });
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

export function onLogBatch(handler: (batch: LogBatch) => void): Promise<UnlistenFn> {
  return listen<LogBatch>(LOG_BATCH, (event) => handler(event.payload));
}
