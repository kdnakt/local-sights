import type { LogEvent, SessionView } from "../api";
import { createTranslator } from "../i18n/messages";

/** English translator used by component tests. */
export const t = createTranslator("en");

export function sessionView(overrides: Partial<SessionView> = {}): SessionView {
  return {
    sessionId: "session-1",
    phase: "Idle",
    input: {
      profileName: "",
      logGroupName: "",
      logStreamName: "",
      startText: "",
      endText: "",
    },
    validationErrors: [],
    canFetch: true,
    currentJobId: null,
    eventCount: 0,
    lastJob: null,
    ...overrides,
  };
}

export function logEvent(sequence: number, message = `message ${sequence}`): LogEvent {
  return {
    timestamp: 1_704_164_645_000 + sequence,
    ingestionTime: null,
    message,
    logStreamName: "stream-a",
    sequence,
  };
}
