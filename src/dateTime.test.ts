import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import {
  completeTime,
  completionWouldEnableFetch,
  DATE_FORMAT_KEY,
  DATE_REQUIRED_KEY,
  fieldOfValidationKey,
  fieldReasons,
  isFutureDate,
  isMaskCompletionOf,
  isoToSlashed,
  isValidDateText,
  isValidTimeText,
  joinDateTime,
  slashedToIso,
  splitDateTime,
  TIME_FORMAT_KEY,
  todayInZone,
} from "./dateTime";

describe("splitDateTime / joinDateTime (interaction-spec §8)", () => {
  it("splits the core's text at the first space into a date and a time", () => {
    expect(splitDateTime("2024/03/01 10:00:00")).toEqual({ date: "2024/03/01", time: "10:00:00" });
    // Trailing whitespace is trimmed; a leading space means an empty date
    // (that is how `joinDateTime` writes it).
    expect(splitDateTime("2024/03/01 10:00:00 \t")).toEqual({
      date: "2024/03/01",
      time: "10:00:00",
    });
  });

  it("splits an empty or malformed text without inventing parts", () => {
    expect(splitDateTime("")).toEqual({ date: "", time: "" });
    expect(splitDateTime("   ")).toEqual({ date: "", time: "" });
    expect(splitDateTime("2024/03/01")).toEqual({ date: "2024/03/01", time: "" });
    expect(splitDateTime(" 10:00:00")).toEqual({ date: "", time: "10:00:00" });
    expect(splitDateTime("2024-03-01 10:00:00")).toEqual({
      date: "2024-03-01",
      time: "10:00:00",
    });
  });

  it("joins the two fields with one space and round-trips through split", () => {
    expect(joinDateTime("2024/03/01", "10:00:00")).toBe("2024/03/01 10:00:00");
    expect(joinDateTime("", "")).toBe("");
    expect(joinDateTime("2024/03/01", "")).toBe("2024/03/01 ");
    expect(joinDateTime("", "10:00:00")).toBe(" 10:00:00");
    for (const [date, time] of [
      ["2024/03/01", "10:00:00"],
      ["2024/03/01", ""],
      ["", "10:00:00"],
      ["", ""],
    ]) {
      expect(splitDateTime(joinDateTime(date!, time!))).toEqual({ date, time });
    }
  });

  it("converts between the DatePicker's ISO value and the slashed text", () => {
    expect(isoToSlashed("2024-03-01")).toBe("2024/03/01");
    expect(isoToSlashed("2024-03")).toBe("2024/03");
    expect(isoToSlashed("")).toBe("");
    expect(slashedToIso("2024/03/01")).toBe("2024-03-01");
    expect(slashedToIso("")).toBe("");
  });
});

describe("completeTime (FR1.3, mockups Q4/Q5)", () => {
  it("adds the seconds: :00 for the start, :59 for the end", () => {
    expect(completeTime("10:00", "start")).toBe("10:00:00");
    expect(completeTime("10:00", "end")).toBe("10:00:59");
  });

  it("fills an empty time with the start or the end of the day", () => {
    expect(completeTime("", "start")).toBe("00:00:00");
    expect(completeTime("", "end")).toBe("23:59:59");
    expect(completeTime("   ", "end")).toBe("23:59:59");
  });

  it("fills missing minutes and pads a short segment like the mask does", () => {
    expect(completeTime("10", "start")).toBe("10:00:00");
    expect(completeTime("10", "end")).toBe("10:59:59");
    expect(completeTime("10:0", "end")).toBe("10:00:59");
    expect(completeTime("9:5", "start")).toBe("09:05:00");
  });

  it("leaves a complete or an invalid time unchanged", () => {
    expect(completeTime("10:00:00", "end")).toBe("10:00:00");
    expect(completeTime("25:00:00", "start")).toBe("25:00:00");
    expect(completeTime("10:60", "end")).toBe("10:60");
    expect(completeTime("abc", "end")).toBe("abc");
    expect(completeTime("10:00:00:00", "end")).toBe("10:00:00:00");
  });

  it("recognizes the mask's own zero-padding so the end rule can replace it", () => {
    expect(isMaskCompletionOf("10:00", "10:00:00")).toBe(true);
    expect(isMaskCompletionOf("10", "10:00:00")).toBe(true);
    expect(isMaskCompletionOf("10:0", "10:00:00")).toBe(true);
    expect(isMaskCompletionOf("10:00:00", "10:00:00")).toBe(false);
    expect(isMaskCompletionOf("10:0", "10:01:00")).toBe(false);
    expect(isMaskCompletionOf("", "10:00:00")).toBe(false);
    expect(isMaskCompletionOf("", "00:00:00")).toBe(false);
  });
});

describe("isValidDateText / isValidTimeText (FR2.1, FR2.4)", () => {
  it("accepts yyyy/mm/dd dates that exist", () => {
    expect(isValidDateText("2024/03/01")).toBe(true);
    expect(isValidDateText("2024/02/29")).toBe(true);
    expect(isValidDateText("0000/01/01")).toBe(true);
  });

  it("rejects hyphens, nonexistent dates and incomplete dates", () => {
    expect(isValidDateText("2024-03-01")).toBe(false);
    expect(isValidDateText("2024/02/30")).toBe(false);
    expect(isValidDateText("2023/02/29")).toBe(false);
    expect(isValidDateText("2024/13/01")).toBe(false);
    expect(isValidDateText("2024/03")).toBe(false);
    expect(isValidDateText("2024/3/1")).toBe(false);
    expect(isValidDateText("")).toBe(false);
  });

  it("accepts hh:mm:ss and rejects anything else", () => {
    expect(isValidTimeText("00:00:00")).toBe(true);
    expect(isValidTimeText("23:59:59")).toBe(true);
    expect(isValidTimeText("25:00:00")).toBe(false);
    expect(isValidTimeText("10:60:00")).toBe(false);
    expect(isValidTimeText("10:00:60")).toBe(false);
    expect(isValidTimeText("10:00")).toBe(false);
    expect(isValidTimeText("")).toBe(false);
  });
});

describe("fieldOfValidationKey (plan Step 4 table, R-02)", () => {
  it("assigns every key the core can send to a field", () => {
    expect(fieldOfValidationKey("selection.logGroupRequired")).toBe("logGroup");
    expect(fieldOfValidationKey("validation.logGroupRequired")).toBe("logGroup");
    expect(fieldOfValidationKey("validation.logGroupTooLong")).toBe("logGroup");
    expect(fieldOfValidationKey("validation.startFormat")).toBe("startDate");
    expect(fieldOfValidationKey("validation.startNonexistentLocalTime")).toBe("startTime");
    expect(fieldOfValidationKey("validation.endFormat")).toBe("endDate");
    expect(fieldOfValidationKey("validation.endNonexistentLocalTime")).toBe("endTime");
    expect(fieldOfValidationKey("validation.rangeOrder")).toBe("endDate");
    expect(fieldOfValidationKey("selection.profileRequired")).toBe("logGroup");
    expect(fieldOfValidationKey("selection.regionRequired")).toBe("logGroup");
  });

  it("knows no field for an unknown key", () => {
    expect(fieldOfValidationKey("session.busy")).toBeNull();
  });
});

describe("fieldReasons (FR3.1, R-02)", () => {
  const valid = { date: "2024/03/01", time: "10:00:00" };

  it("puts the core's reasons under their fields when the screen finds nothing wrong", () => {
    expect(
      fieldReasons({
        start: valid,
        end: { date: "2024/03/01", time: "09:00:00" },
        validationErrors: ["selection.logGroupRequired", "validation.rangeOrder"],
      }),
    ).toEqual({ logGroup: "selection.logGroupRequired", endDate: "validation.rangeOrder" });
    expect(
      fieldReasons({
        start: { date: "2024/03/10", time: "02:30:00" },
        end: valid,
        validationErrors: ["validation.startNonexistentLocalTime"],
      }),
    ).toEqual({ startTime: "validation.startNonexistentLocalTime" });
  });

  it("checks the date and time shapes first and then hides the core's format reason", () => {
    expect(
      fieldReasons({
        start: { date: "2024-03-01", time: "10:00:00" },
        end: { date: "2024/02/30", time: "25:00:00" },
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    ).toEqual({ startDate: DATE_FORMAT_KEY, endDate: DATE_FORMAT_KEY, endTime: TIME_FORMAT_KEY });
  });

  it("asks for a date when the date is empty, from the first moment", () => {
    expect(
      fieldReasons({
        start: { date: "", time: "" },
        end: { date: "", time: "" },
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    ).toEqual({ startDate: DATE_REQUIRED_KEY, endDate: DATE_REQUIRED_KEY });
  });

  it("shows the core's format reason on the date field when both shapes look right", () => {
    expect(
      fieldReasons({
        start: valid,
        end: valid,
        validationErrors: ["validation.startFormat"],
      }),
    ).toEqual({ startDate: "validation.startFormat" });
  });

  it("says nothing about an empty time with a valid date: it will be completed", () => {
    expect(
      fieldReasons({
        start: { date: "2024/03/01", time: "" },
        end: valid,
        validationErrors: ["validation.startFormat"],
      }),
    ).toEqual({});
  });

  it("keeps the first reason per field only", () => {
    expect(
      fieldReasons({
        start: valid,
        end: valid,
        validationErrors: ["validation.logGroupTooLong", "selection.logGroupRequired"],
      }),
    ).toEqual({ logGroup: "validation.logGroupTooLong" });
  });
});

describe("todayInZone / isFutureDate (gate fix 17a)", () => {
  // The "Local" zone is the machine's; fix it so the test does not depend on
  // where it runs (Node re-reads TZ when it is assigned).
  beforeAll(() => {
    vi.stubEnv("TZ", "Asia/Tokyo");
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });

  it("gives today in the local zone or in UTC, which differ around midnight", () => {
    // 2024-03-01 16:00 UTC is 2024-03-02 01:00 in Tokyo (UTC+9).
    const nowMs = Date.UTC(2024, 2, 1, 16, 0, 0);
    expect(todayInZone(nowMs, "Local")).toBe("2024/03/02");
    expect(todayInZone(nowMs, "Utc")).toBe("2024/03/01");
  });

  it("crosses months and years with the zone", () => {
    expect(todayInZone(Date.UTC(2024, 1, 29, 20, 0, 0), "Local")).toBe("2024/03/01");
    expect(todayInZone(Date.UTC(2024, 1, 29, 20, 0, 0), "Utc")).toBe("2024/02/29");
    expect(todayInZone(Date.UTC(2024, 11, 31, 15, 30, 0), "Local")).toBe("2025/01/01");
    expect(todayInZone(Date.UTC(2024, 11, 31, 15, 30, 0), "Utc")).toBe("2024/12/31");
    expect(todayInZone(Date.UTC(2024, 0, 5, 0, 0, 0), "Utc")).toBe("2024/01/05");
  });

  it("does not take today or an earlier day for the future", () => {
    expect(isFutureDate("2024/03/01", "2024/03/01")).toBe(false);
    expect(isFutureDate("2024/02/29", "2024/03/01")).toBe(false);
    expect(isFutureDate("2023/12/31", "2024/01/01")).toBe(false);
  });

  it("takes tomorrow and later days for the future, across months and years", () => {
    expect(isFutureDate("2024/03/02", "2024/03/01")).toBe(true);
    expect(isFutureDate("2024/03/01", "2024/02/29")).toBe(true);
    expect(isFutureDate("2025/01/01", "2024/12/31")).toBe(true);
  });

  it("does not take a malformed text for the future", () => {
    expect(isFutureDate("2099-01-01", "2024/03/01")).toBe(false);
    expect(isFutureDate("2099/02/30", "2024/03/01")).toBe(false);
    expect(isFutureDate("", "2024/03/01")).toBe(false);
    expect(isFutureDate("2099/01/01", "")).toBe(false);
  });
});

describe("completionWouldEnableFetch (R-01)", () => {
  const valid = { date: "2024/03/01", time: "10:00:00" };

  it("is true when the only reasons are empty times next to valid dates", () => {
    expect(
      completionWouldEnableFetch({
        start: { date: "2024/03/01", time: "" },
        end: { date: "2024/03/02", time: "" },
        validationErrors: ["validation.startFormat", "validation.endFormat"],
      }),
    ).toBe(true);
    expect(
      completionWouldEnableFetch({
        start: valid,
        end: { date: "2024/03/02", time: "" },
        validationErrors: ["validation.endFormat"],
      }),
    ).toBe(true);
  });

  it("is false without reasons, with other reasons, or with an invalid date", () => {
    expect(completionWouldEnableFetch({ start: valid, end: valid, validationErrors: [] })).toBe(
      false,
    );
    expect(
      completionWouldEnableFetch({
        start: { date: "2024/03/01", time: "" },
        end: valid,
        validationErrors: ["validation.startFormat", "selection.logGroupRequired"],
      }),
    ).toBe(false);
    expect(
      completionWouldEnableFetch({
        start: { date: "2024/02/30", time: "" },
        end: valid,
        validationErrors: ["validation.startFormat"],
      }),
    ).toBe(false);
    expect(
      completionWouldEnableFetch({
        start: { date: "2024/03/01", time: "10:00" },
        end: valid,
        validationErrors: ["validation.startFormat"],
      }),
    ).toBe(false);
  });
});
