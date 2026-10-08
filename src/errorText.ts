/**
 * The words of an error (U7:BR2.1-BR2.3): from the failure kind, the scene
 * (where the error is shown) and the safe detail, which message keys say
 * "what happened" and "what to do next", and the detail line. The wording
 * itself is in the message catalog (`error.*`), word for word as BR2.2
 * except for the `{retry}` texts changed at the code generation review
 * (R-02, recorded in code-summary.md). Every function here is pure.
 */

import type { FailureKind } from "./api";
import type { Translate } from "./i18n/messages";

/**
 * Where an error is shown (BR2.1): the failure of a whole fetch job, of the
 * log group listing, or of one stream of a fetch.
 */
export type ErrorScene = "Fetch" | "Listing" | "Stream";

/** The message keys of one error and its detail. */
export interface ErrorText {
  whatKey: string;
  nextKey: string;
  /** Fills `{retry}` of the next action: [Fetch] or [Reload]. */
  retryKey: string;
  /**
   * Whether the next action starts with `{retry}`, so its first letter is
   * written in capitals (the English "press" is lowercase inside a
   * sentence, review R-02).
   */
  retryStartsSentence: boolean;
  /** The safe detail, or `null` when there is none (BR2.3). */
  detail: string | null;
}

/** The three lines as shown; `detail` is `null` without a safe detail. */
export interface ErrorLines {
  what: string;
  next: string;
  detail: string | null;
}

/** Combinations that cannot happen in a scene use the Other texts (BR2.2). */
const IMPOSSIBLE: Record<ErrorScene, readonly FailureKind[]> = {
  Fetch: [],
  Listing: ["NotFound", "InvalidInput"],
  Stream: ["RegionMissing"],
};

/** Next actions whose sentence starts with `{retry}` (BR2.2). */
const RETRY_FIRST: readonly FailureKind[] = ["Other"];

/** The text with its first character in capitals (no change for Japanese). */
function capitalized(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** BR2.2, BR2.3: the keys of `kind` in `scene`, and the detail when not blank. */
export function errorText(kind: FailureKind, scene: ErrorScene, safeDetail: string): ErrorText {
  const shown: FailureKind = IMPOSSIBLE[scene].includes(kind) ? "Other" : kind;
  const detail = safeDetail.trim();
  return {
    whatKey: `error.what.${shown}`,
    nextKey: `error.next.${shown}`,
    retryKey: scene === "Listing" ? "error.retry.listing" : "error.retry.fetch",
    retryStartsSentence: RETRY_FIRST.includes(shown),
    detail: detail === "" ? null : detail,
  };
}

/** The lines of an error in the language of `t`, `{retry}` filled in. */
export function errorLines(
  kind: FailureKind,
  scene: ErrorScene,
  safeDetail: string,
  t: Translate,
): ErrorLines {
  const text = errorText(kind, scene, safeDetail);
  const retry = t(text.retryKey);
  return {
    what: t(text.whatKey),
    next: t(text.nextKey, { retry: text.retryStartsSentence ? capitalized(retry) : retry }),
    detail: text.detail === null ? null : t("status.detail", { detail: text.detail }),
  };
}
