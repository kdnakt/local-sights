/**
 * Thin wrapper around the Tauri commands and events of the desktop app
 * (Q1: commands for operations, events for pushed state and log pages).
 * Tests replace this module with `vi.mock`, so Tauri is never started.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "Idle" | "Fetching" | "Done" | "Failed";

export type InputField = "profileName" | "logGroupName" | "logStreamName" | "startText" | "endText";

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

export function startFetch(): Promise<SessionView> {
  return invoke<SessionView>("start_fetch");
}

export function onSessionChanged(handler: (view: SessionView) => void): Promise<UnlistenFn> {
  return listen<SessionView>(SESSION_CHANGED, (event) => handler(event.payload));
}

export function onLogBatch(handler: (batch: LogBatch) => void): Promise<UnlistenFn> {
  return listen<LogBatch>(LOG_BATCH, (event) => handler(event.payload));
}
