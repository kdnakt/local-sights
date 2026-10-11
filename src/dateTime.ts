/**
 * Pure helpers of the console-style condition area (261010-console-ui-polish,
 * interaction-spec §3 and §8): the start and end are each a date field
 * (`yyyy/mm/dd`, a DatePicker) and a time field (`hh:mm:ss`, a TimeInput) on
 * the screen, while the core keeps one text per input, `yyyy/mm/dd hh:mm:ss`.
 * The screen joins the two fields before `updateInput` and splits the text
 * the core gives back (FR1.5). It also completes a short or empty time
 * (FR1.3, mockups Q4/Q5), checks the shape of each field before the core's
 * reasons are shown (FR2.4, R-02) and assigns every reason the core can send
 * to the field it is shown under (FR3.1, plan Step 4 table). Nothing here
 * converts a time zone: that stays in the core (U4:BR3.4); only today's
 * date for the calendar's limit (gate fix 17a) is read in the shown zone.
 */

import type { TimeZoneChoice } from "./api";

/** Which of the two inputs a field belongs to. */
export type Side = "start" | "end";

/** The two fields of one input as the screen holds them. */
export interface DateTimeParts {
  date: string;
  time: string;
}

/** Where a reason is shown: under the heading or under one of the fields. */
export type ReasonField = "logGroup" | "startDate" | "startTime" | "endDate" | "endTime";

/** Message key: the date field is empty (interaction-spec §9 "日付が空"). */
export const DATE_REQUIRED_KEY = "form.date.required";
/** Message key: the date field is not `yyyy/mm/dd` (§9 "日付の形式の誤り"). */
export const DATE_FORMAT_KEY = "form.date.format";
/** Message key: the time field is not `hh:mm:ss` (§9 "時刻の形式の誤り"). */
export const TIME_FORMAT_KEY = "form.time.format";

/** The core's reason keys that mean "the whole input is not yyyy/mm/dd hh:mm:ss". */
const FORMAT_KEYS: Record<Side, string> = {
  start: "validation.startFormat",
  end: "validation.endFormat",
};

/** The field each reason key of the core is shown under (plan Step 4 table). */
const FIELD_OF_KEY: Record<string, ReasonField> = {
  "selection.logGroupRequired": "logGroup",
  "validation.logGroupRequired": "logGroup",
  "validation.logGroupTooLong": "logGroup",
  "validation.startFormat": "startDate",
  "validation.startNonexistentLocalTime": "startTime",
  "validation.endFormat": "endDate",
  "validation.endNonexistentLocalTime": "endTime",
  "validation.rangeOrder": "endDate",
  "selection.profileRequired": "logGroup",
  "selection.regionRequired": "logGroup",
};

/** What fills a missing time segment: the first instant for the start, the last for the end. */
const FILL: Record<Side, readonly [string, string, string]> = {
  start: ["00", "00", "00"],
  end: ["23", "59", "59"],
};

/** The largest value of each time segment (24-hour clock). */
const SEGMENT_MAX: readonly [number, number, number] = [23, 59, 59];

/**
 * Splits the core's text at its first space: the part before is the date,
 * the part after is the time, both trimmed. No space means a date only.
 */
export function splitDateTime(text: string): DateTimeParts {
  const index = text.indexOf(" ");
  if (index < 0) {
    return { date: text.trim(), time: "" };
  }
  return { date: text.slice(0, index).trim(), time: text.slice(index + 1).trim() };
}

/**
 * Joins the two fields into the one text the core reads. Both empty gives
 * an empty text; otherwise the separator space is always present, so
 * `splitDateTime` gives the two fields back.
 */
export function joinDateTime(date: string, time: string): string {
  const d = date.trim();
  const t = time.trim();
  return d === "" && t === "" ? "" : `${d} ${t}`;
}

/** The DatePicker's ISO value (`yyyy-mm-dd`, possibly partial) as slashed text. */
export function isoToSlashed(iso: string): string {
  return iso.replace(/-/g, "/");
}

/** The slashed date text as the DatePicker's ISO value. */
export function slashedToIso(slashed: string): string {
  return slashed.replace(/\//g, "-");
}

/** Days of `month` (1-12) in `year`, with leap years. */
function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

/** FR2.1, FR2.4: exactly `yyyy/mm/dd`, and a date that exists. */
export function isValidDateText(text: string): boolean {
  const match = /^(\d{4})\/(\d{2})\/(\d{2})$/.exec(text);
  if (match === null) {
    return false;
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  return month >= 1 && month <= 12 && day >= 1 && day <= daysInMonth(year, month);
}

/** FR2.1: exactly `hh:mm:ss` on the 24-hour clock. */
export function isValidTimeText(text: string): boolean {
  const match = /^(\d{2}):(\d{2}):(\d{2})$/.exec(text);
  if (match === null) {
    return false;
  }
  return [match[1], match[2], match[3]].every(
    (segment, index) => Number(segment) <= SEGMENT_MAX[index]!,
  );
}

/**
 * The time segments of a partial `hh:mm:ss` text, or `null` when the text is
 * not a prefix of one (more than three segments, a letter, a segment longer
 * than two digits or beyond its maximum).
 */
function partialSegments(text: string): string[] | null {
  const trimmed = text.trim();
  if (trimmed === "") {
    return [];
  }
  const segments = trimmed.split(":");
  if (segments.length > 3) {
    return null;
  }
  const valid = segments.every(
    (segment, index) =>
      /^\d{0,2}$/.test(segment) && (segment === "" || Number(segment) <= SEGMENT_MAX[index]!),
  );
  return valid ? segments : null;
}

/**
 * FR1.3, mockups Q4/Q5: completes a short or empty time. A missing or empty
 * segment is filled with the first instant for the start (`00`) and the
 * last for the end (`59`, hour `23`); a one-digit segment is padded with a
 * zero like the mask does. A complete time, and anything that is not the
 * beginning of a valid time, is returned unchanged.
 */
export function completeTime(text: string, side: Side): string {
  const segments = partialSegments(text);
  if (segments === null || (segments.length === 3 && segments.every((s) => s.length === 2))) {
    return text;
  }
  const fill = FILL[side];
  return [0, 1, 2]
    .map((index) => {
      const segment = segments[index] ?? "";
      return segment === "" ? fill[index] : segment.padStart(2, "0");
    })
    .join(":");
}

/**
 * Whether `next` is what Cloudscape's time mask makes of `previous` when the
 * field loses focus or Enter is pressed (missing parts filled with zeros).
 * The condition area then applies {@link completeTime} to `previous`
 * instead, so the end field gets `:59` (Q5). An empty field is never
 * completed by the mask.
 */
export function isMaskCompletionOf(previous: string, next: string): boolean {
  const trimmed = previous.trim();
  if (trimmed === "" || trimmed === next || isValidTimeText(trimmed)) {
    return false;
  }
  return completeTime(trimmed, "start") === next;
}

/** Zero-pads a date segment to `width` digits. */
function pad(value: number, width: number): string {
  return String(value).padStart(width, "0");
}

/**
 * Gate fix 17a: today's date (`yyyy/mm/dd`) at `nowMs` in the zone the
 * screen shows, the machine's own zone for `"Local"` and UTC for `"Utc"`.
 * The calendar uses it to tell which days lie in the future.
 */
export function todayInZone(nowMs: number, zone: TimeZoneChoice): string {
  const now = new Date(nowMs);
  if (zone === "Utc") {
    return `${pad(now.getUTCFullYear(), 4)}/${pad(now.getUTCMonth() + 1, 2)}/${pad(now.getUTCDate(), 2)}`;
  }
  return `${pad(now.getFullYear(), 4)}/${pad(now.getMonth() + 1, 2)}/${pad(now.getDate(), 2)}`;
}

/**
 * Gate fix 17a: whether the date `slashed` lies after `today` (both
 * `yyyy/mm/dd`). Today itself is not in the future. A text that is not a
 * valid date, on either side, is never taken for the future: the calendar
 * then restricts nothing and the field's own reason says what is wrong.
 * Valid `yyyy/mm/dd` texts sort like the dates they name.
 */
export function isFutureDate(slashed: string, today: string): boolean {
  return isValidDateText(slashed) && isValidDateText(today) && slashed > today;
}

/** The field a reason key of the core is shown under; `null` for other keys. */
export function fieldOfValidationKey(key: string): ReasonField | null {
  return FIELD_OF_KEY[key] ?? null;
}

/** What the screen itself finds wrong with one side's two fields (R-02). */
interface SideCheck {
  /** The screen's own reasons for the date and time fields, if any. */
  reasons: Partial<Record<"date" | "time", string>>;
  /** Whether the core's "format" reason for this side is hidden by them. */
  hidesCoreFormat: boolean;
  /** The date is valid and the time is empty: completion will fix it (R-01). */
  awaitsCompletion: boolean;
}

function checkSide({ date, time }: DateTimeParts): SideCheck {
  const reasons: SideCheck["reasons"] = {};
  if (date === "") {
    reasons.date = DATE_REQUIRED_KEY;
  } else if (!isValidDateText(date)) {
    reasons.date = DATE_FORMAT_KEY;
  }
  if (time !== "" && !isValidTimeText(time)) {
    reasons.time = TIME_FORMAT_KEY;
  }
  const awaitsCompletion = reasons.date === undefined && time === "";
  return {
    reasons,
    hidesCoreFormat: reasons.date !== undefined || reasons.time !== undefined || awaitsCompletion,
    awaitsCompletion,
  };
}

export interface ReasonInput {
  start: DateTimeParts;
  end: DateTimeParts;
  /** The core's `validationErrors` (message keys). */
  validationErrors: readonly string[];
}

/**
 * FR3.1, R-02: the message key shown under each field. The shape of each
 * date and time field is checked first and shown under that field; the
 * core's "format" reason for that side is then hidden, because the screen
 * already says what is wrong (or, with a valid date and an empty time, the
 * time will be completed). Every other reason goes to its field from the
 * table; the first reason per field wins.
 */
export function fieldReasons({ start, end, validationErrors }: ReasonInput) {
  const reasons: Partial<Record<ReasonField, string>> = {};
  const checks: Record<Side, SideCheck> = { start: checkSide(start), end: checkSide(end) };
  for (const side of ["start", "end"] as const) {
    const own = checks[side].reasons;
    if (own.date !== undefined) {
      reasons[`${side}Date`] = own.date;
    }
    if (own.time !== undefined) {
      reasons[`${side}Time`] = own.time;
    }
  }
  for (const key of validationErrors) {
    const hidden = (["start", "end"] as const).some(
      (side) => key === FORMAT_KEYS[side] && checks[side].hidesCoreFormat,
    );
    const field = fieldOfValidationKey(key);
    if (!hidden && field !== null && reasons[field] === undefined) {
      reasons[field] = key;
    }
  }
  return reasons;
}

/**
 * R-01: whether [Fetch] may be offered although the core says it cannot
 * fetch, because the only reasons are empty times next to valid dates: the
 * press completes the times first and the core then decides.
 */
export function completionWouldEnableFetch({ start, end, validationErrors }: ReasonInput) {
  if (validationErrors.length === 0) {
    return false;
  }
  const checks: Record<Side, SideCheck> = { start: checkSide(start), end: checkSide(end) };
  return validationErrors.every((key) =>
    (["start", "end"] as const).some(
      (side) => key === FORMAT_KEYS[side] && checks[side].awaitsCompletion,
    ),
  );
}
