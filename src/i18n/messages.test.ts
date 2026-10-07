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
  "session.settingsClosed",
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

  it("has the U5 log filter texts in both languages", () => {
    for (const key of [
      "logFilter.label",
      "logFilter.placeholder",
      "status.filter.count",
      "status.filter.zero",
      "status.filter.filtering",
    ]) {
      expect(isMessageKey(key), key).toBe(true);
    }
    expect(translate("ja", "status.filter.count", { matched: 3, all: 10 })).toBe(
      "絞り込み後 3 件 / 全 10 件",
    );
    expect(translate("en", "status.filter.count", { matched: 3, all: 10 })).toBe(
      "Filtered: 3 of 10 events",
    );
    expect(translate("ja", "status.filter.zero", { all: 10 })).toBe(
      "絞り込み後 0 件 / 全 10 件：一致するログはありません。",
    );
    expect(translate("ja", "status.filter.filtering")).toBe("絞り込み中…");
  });

  it("returns an unknown key unchanged so the gap is visible", () => {
    expect(translate("en", "no.such.key")).toBe("no.such.key");
  });

  it("has the U6 cache notices word for word as U6:BR5.3 and the settings texts", () => {
    expect(messages["cache.saving"]).toEqual({ en: "Saving to cache", ja: "キャッシュに保存中" });
    expect(messages["cache.hit"]).toEqual({
      en: "Shown from cache (AWS was not called)",
      ja: "キャッシュから表示（AWS は呼んでいない）",
    });
    expect(messages["cache.readFailed"]).toEqual({
      en: "The cache could not be read, so the logs were fetched again",
      ja: "キャッシュが読めなかったので取り直した",
    });
    expect(messages["cache.saveFailed"]).toEqual({
      en: "Could not save to the cache",
      ja: "キャッシュに保存できなかった",
    });
    for (const key of [
      "settings.open",
      "settings.title",
      "settings.cacheEnabled",
      "settings.location",
      "settings.warning",
      "settings.clear",
      "settings.cancel",
      "settings.save",
      "settings.notice.Cleared",
      "settings.notice.ClearFailed",
      "settings.notice.SaveFailed",
    ]) {
      expect(isMessageKey(key), key).toBe(true);
    }
    expect(translate("ja", "settings.location", { path: "/c" })).toBe("保存場所：/c");
  });
});
