import { describe, expect, it } from "vitest";
import { detectLocale, isMessageKey, messages, timeZoneLabelKey, translate } from "./messages";

// Keys produced by the Rust core (request.rs, failure.rs, session.rs, catalog).
const CORE_KEYS = [
  "validation.logGroupRequired",
  "validation.logGroupTooLong",
  "validation.startFormat",
  "validation.startNonexistentLocalTime",
  "validation.endFormat",
  "validation.endNonexistentLocalTime",
  "validation.rangeOrder",
  "failure.kind.AuthRequired",
  "failure.kind.AccessDenied",
  "failure.kind.Throttled",
  "failure.kind.Network",
  "failure.kind.NotFound",
  "failure.kind.InvalidInput",
  "failure.kind.RegionMissing",
  "failure.kind.Other",
  "session.busy",
  "session.invalid",
  "session.confirmationPending",
  "session.notConnected",
  "session.unknownProfile",
  "session.unknownRegion",
  "session.unknownLogGroup",
  "session.nothingPending",
  "session.staleGeneration",
  "session.noFailures",
  "selection.profileRequired",
  "selection.regionRequired",
  "selection.logGroupRequired",
  "catalog.unreadable.config",
  "catalog.unreadable.credentials",
];

describe("message catalog", () => {
  it("has a non-empty English and Japanese text for every key", () => {
    for (const [key, texts] of Object.entries(messages)) {
      expect(texts.en.trim(), `${key} (en)`).not.toBe("");
      expect(texts.ja.trim(), `${key} (ja)`).not.toBe("");
    }
  });

  it("contains every key the core can send", () => {
    for (const key of CORE_KEYS) {
      expect(isMessageKey(key), key).toBe(true);
    }
  });

  it("uses Japanese only when the first OS language is Japanese", () => {
    expect(detectLocale(["ja"])).toBe("ja");
    expect(detectLocale(["ja-JP", "en-US"])).toBe("ja");
    expect(detectLocale(["en-US", "ja-JP"])).toBe("en");
    expect(detectLocale(["fr-FR"])).toBe("en");
    expect(detectLocale(["jam"])).toBe("en");
    expect(detectLocale([])).toBe("en");
  });

  it("fills placeholders in the chosen language", () => {
    expect(translate("en", "status.count", { count: 5 })).toBe("5 events");
    expect(translate("ja", "status.count", { count: 5 })).toBe("5 件");
  });

  it("has the U3 screen texts in both languages and no stream name keys", () => {
    for (const key of [
      "table.stream",
      "table.scroll",
      "status.listing",
      "status.fetching",
      "status.failedStreams",
      "status.showFailures",
      "status.listingPartial",
      "failures.title",
      "failures.listing",
      "failures.close",
    ]) {
      expect(isMessageKey(key), key).toBe(true);
    }
    expect(Object.keys(messages).filter((key) => key.includes("logStream"))).toEqual([]);
    expect(translate("ja", "status.failedStreams", { count: 3 })).toBe("3 ストリームで失敗");
    expect(translate("en", "status.fetching", { finished: 1, planned: 4, count: 10 })).toBe(
      "Fetching… 1/4 streams, 10 events so far",
    );
  });

  it("names the chosen time zone instead of a fixed UTC in the U4 texts", () => {
    const zoned = [
      "form.start.label",
      "form.end.label",
      "validation.startFormat",
      "validation.endFormat",
      "table.time",
    ] as const;
    for (const key of zoned) {
      for (const locale of ["en", "ja"] as const) {
        expect(messages[key][locale], `${key} (${locale})`).toContain("{zone}");
        expect(messages[key][locale], `${key} (${locale})`).not.toContain("UTC");
      }
    }
    const local = translate("ja", timeZoneLabelKey("Local"));
    expect(translate("ja", "form.start.label", { zone: local })).toBe("開始日時（ローカル）");
    expect(translate("ja", "table.time", { zone: local })).toBe("時刻（ローカル）");
    expect(translate("en", "table.time", { zone: translate("en", timeZoneLabelKey("Utc")) })).toBe(
      "Time (UTC)",
    );
    expect(translate("ja", "validation.startFormat", { zone: local })).toBe(
      "開始日時は yyyy-mm-dd hh:mm:ss の形で入力してください（ローカル）。",
    );
  });

  it("has distinct daylight saving reasons in both languages", () => {
    expect(translate("ja", "validation.startNonexistentLocalTime")).toBe(
      "開始日時は夏時間の切り替えで存在しない日時です。",
    );
    expect(translate("ja", "validation.endNonexistentLocalTime")).toBe(
      "終了日時は夏時間の切り替えで存在しない日時です。",
    );
    expect(translate("en", "validation.startNonexistentLocalTime")).not.toBe(
      translate("en", "validation.startFormat"),
    );
    for (const key of ["timeZone.label", "timeZone.local", "timeZone.utc"]) {
      expect(isMessageKey(key), key).toBe(true);
    }
  });

  it("returns an unknown key unchanged so the gap is visible", () => {
    expect(translate("en", "no.such.key")).toBe("no.such.key");
  });
});
