import { describe, expect, it } from "vitest";
import type { FailureKind } from "./api";
import { errorLines, errorText, type ErrorScene } from "./errorText";
import { createTranslator } from "./i18n/messages";

const en = createTranslator("en");
const ja = createTranslator("ja");

const KINDS: FailureKind[] = [
  "AuthRequired",
  "AccessDenied",
  "Throttled",
  "Network",
  "NotFound",
  "InvalidInput",
  "RegionMissing",
  "Other",
];
const SCENES: ErrorScene[] = ["Fetch", "Listing", "Stream"];

describe("errorText", () => {
  it("gives every kind in every scene a what-happened and a next-action text in both languages", () => {
    for (const kind of KINDS) {
      for (const scene of SCENES) {
        for (const t of [en, ja]) {
          const lines = errorLines(kind, scene, "", t);
          expect(lines.what, `${kind}/${scene}`).not.toMatch(/^error\./);
          expect(lines.next, `${kind}/${scene}`).not.toMatch(/^error\./);
          expect(lines.next, `${kind}/${scene}`).not.toContain("{retry}");
          expect(lines.what.trim()).not.toBe("");
          expect(lines.detail).toBeNull();
        }
      }
    }
  });

  it("uses the canonical wording of BR2.2 with [Fetch] for Fetch and Stream", () => {
    expect(errorLines("AuthRequired", "Fetch", "", ja)).toEqual({
      what: "認証情報を使えませんでした（SSO のログイン切れなど）。",
      next: "SSO を使っているときは aws sso login を実行してから、[Fetch] でもう一度取得してください。それ以外はプロファイルの設定を確認するか、別のプロファイルを選んでください。",
      detail: null,
    });
    expect(errorLines("Throttled", "Stream", "", en)).toEqual({
      what: "AWS throttled the requests.",
      next: "Wait a while, then Press [Fetch] to fetch again.",
      detail: null,
    });
    expect(errorLines("Network", "Fetch", "", ja).next).toBe(
      "接続を確認してから、[Fetch] でもう一度取得してください。",
    );
    expect(errorLines("NotFound", "Fetch", "", en)).toEqual({
      what: "The log group or stream was not found.",
      next: "Press [Reload] to load the log group list again and choose again.",
      detail: null,
    });
    expect(errorLines("RegionMissing", "Fetch", "", ja)).toEqual({
      what: "このプロファイルには既定のリージョンがありません。",
      next: "上部バーでリージョンを選んでください。",
      detail: null,
    });
  });

  it("puts [Reload] into the next action of the Listing scene", () => {
    expect(errorLines("Network", "Listing", "", en).next).toBe(
      "Check your connection, then Press [Reload] to load the list again.",
    );
    expect(errorLines("Other", "Listing", "", ja).next).toBe(
      "[再読み込み] でもう一度読み込んでください。続くときは詳細を確かめてください。",
    );
    expect(errorText("Throttled", "Listing", "").retryKey).toBe("error.retry.listing");
    expect(errorText("Throttled", "Fetch", "").retryKey).toBe("error.retry.fetch");
    expect(errorText("Throttled", "Stream", "").retryKey).toBe("error.retry.fetch");
  });

  it("uses the Other texts for combinations that cannot happen in a scene", () => {
    for (const [kind, scene] of [
      ["NotFound", "Listing"],
      ["InvalidInput", "Listing"],
      ["RegionMissing", "Stream"],
    ] as Array<[FailureKind, ErrorScene]>) {
      expect(errorText(kind, scene, "")).toEqual(errorText("Other", scene, ""));
      expect(errorLines(kind, scene, "", en).what).toBe("Something went wrong while calling AWS.");
    }
    expect(errorText("RegionMissing", "Listing", "").whatKey).toBe("error.what.RegionMissing");
    expect(errorText("NotFound", "Stream", "").whatKey).toBe("error.what.NotFound");
  });

  it("adds the safe detail as a third line only when there is one", () => {
    expect(
      errorLines("AccessDenied", "Fetch", "api=GetLogEvents code=AccessDenied", en).detail,
    ).toBe("Details: api=GetLogEvents code=AccessDenied");
    expect(errorLines("AccessDenied", "Listing", "code=X", ja).detail).toBe("詳細：code=X");
    expect(errorLines("AccessDenied", "Stream", "   ", en).detail).toBeNull();
    expect(errorText("AccessDenied", "Stream", "").detail).toBeNull();
  });
});
