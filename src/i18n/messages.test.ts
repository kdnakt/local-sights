import { describe, expect, it } from "vitest";
import { detectLocale, isMessageKey, messages, translate } from "./messages";

// Keys produced by the Rust core (request.rs, failure.rs, session.rs).
const CORE_KEYS = [
  "validation.logGroupRequired",
  "validation.logGroupTooLong",
  "validation.logStreamRequired",
  "validation.logStreamTooLong",
  "validation.startFormat",
  "validation.endFormat",
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

  it("returns an unknown key unchanged so the gap is visible", () => {
    expect(translate("en", "no.such.key")).toBe("no.such.key");
  });
});
