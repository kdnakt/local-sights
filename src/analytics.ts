/**
 * Pure helpers of the Log Analytics screen (261017-insights-query U1): the
 * two pages the navigation switches between (U1:BR1.1), the words of a
 * reading problem (U1:BR3.9), the place of a problem in the query text and
 * the one log group a new choice keeps (U1:BR1.3).
 */

import type { QueryProblem } from "./api";
import type { Translate } from "./i18n/messages";

/** The two screens of the app (FR1.1). */
export type AppPage = "logManagement" | "logAnalytics";

/** The pages in the order of the navigation and of Ctrl/Cmd+1, +2. */
export const APP_PAGES: readonly AppPage[] = ["logManagement", "logAnalytics"];

/** Message key of each page's name. */
export const PAGE_NAME_KEY: Record<AppPage, string> = {
  logManagement: "nav.logManagement",
  logAnalytics: "nav.logAnalytics",
};

/**
 * The words of a reading problem: the line and column, then what is wrong,
 * and for an unsupported command the commands this version runs (U1:BR3.9,
 * refined-mockups screen 7). Only the query's own word is put in (FR6.4).
 */
export function problemText(problem: QueryProblem, t: Translate): string {
  const place = t("insights.error.at", {
    line: problem.span.line,
    column: problem.span.column,
  });
  const words = t(problem.messageKey, { word: problem.word ?? "" });
  const supported =
    problem.messageKey === "query.unsupported.command"
      ? ` ${t("insights.error.supportedList")}`
      : "";
  return `${place}${words}${supported}`;
}

/**
 * The UTF-16 offset of `line` and `column` (both from 1, counted in
 * characters as the core counts them) in `text`, for a text field's
 * selection; past the end it is the end.
 */
export function offsetOf(text: string, line: number, column: number): number {
  const characters = Array.from(text);
  let currentLine = 1;
  let index = 0;
  while (index < characters.length && currentLine < line) {
    if (characters[index] === "\n") {
      currentLine += 1;
    }
    index += 1;
  }
  const characterIndex = Math.min(index + Math.max(column, 1) - 1, characters.length);
  return characters.slice(0, characterIndex).join("").length;
}

/**
 * U1:BR1.3: the one log group the newest choice makes the selection, the
 * previous one being replaced; nothing chosen is `null`.
 */
export function newestSelection(values: readonly string[], previous: string | null): string | null {
  if (values.length === 0) {
    return null;
  }
  return values.find((value) => value !== previous) ?? values[0] ?? null;
}
