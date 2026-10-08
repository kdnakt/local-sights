import { render } from "@testing-library/react";
import { createElement, type ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { CloseConfirmDialog } from "../components/CloseConfirmDialog";
import { FailureList } from "../components/FailureList";
import { FetchErrorBanner } from "../components/FetchErrorBanner";
import { LogGroupPane } from "../components/LogGroupPane";
import { StatusLine } from "../components/StatusLine";
import type { ApiFailure, FailureKind } from "../api";
import { connectedView, sessionView } from "../test/fixtures";
import {
  detectLocale,
  isMessageKey,
  messages,
  timeZoneLabelKey,
  translate,
  type Translate,
} from "./messages";

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

  it("has the U7 error texts word for word as BR2.2", () => {
    const canonical: Record<FailureKind, { what: [string, string]; next: [string, string] }> = {
      AuthRequired: {
        what: [
          "Credentials could not be used (for example, the SSO session expired).",
          "認証情報を使えませんでした（SSO のログイン切れなど）。",
        ],
        next: [
          "If you use SSO, run aws sso login, then {retry}. Otherwise, check the profile settings or choose another profile.",
          "SSO を使っているときは aws sso login を実行してから、{retry}。それ以外はプロファイルの設定を確認するか、別のプロファイルを選んでください。",
        ],
      },
      AccessDenied: {
        what: ["Access was denied.", "権限がないため拒否されました。"],
        next: [
          "Check the IAM permissions or choose another profile.",
          "IAM の権限を確認するか、別のプロファイルを選んでください。",
        ],
      },
      Throttled: {
        what: ["AWS throttled the requests.", "AWS の呼び出しが多すぎるため、制限されました。"],
        next: ["Wait a while, then {retry}.", "しばらく待ってから、{retry}。"],
      },
      Network: {
        what: ["Could not connect to AWS.", "AWS に接続できませんでした。"],
        next: ["Check your connection, then {retry}.", "接続を確認してから、{retry}。"],
      },
      NotFound: {
        what: [
          "The log group or stream was not found.",
          "ロググループかストリームが見つかりませんでした。",
        ],
        next: [
          "Press [Reload] to load the log group list again and choose again.",
          "[再読み込み] でロググループの一覧を読み込み直して、選び直してください。",
        ],
      },
      InvalidInput: {
        what: ["AWS rejected the request.", "AWS が条件を受け付けませんでした。"],
        next: ["Check the time range, then {retry}.", "時間範囲を見直してから、{retry}。"],
      },
      RegionMissing: {
        what: [
          "This profile has no default region.",
          "このプロファイルには既定のリージョンがありません。",
        ],
        next: ["Choose a region in the top bar.", "上部バーでリージョンを選んでください。"],
      },
      Other: {
        what: ["Something went wrong while calling AWS.", "AWS の呼び出しで問題が起きました。"],
        next: [
          "{retry}. If it keeps happening, check the details.",
          "{retry}。続くときは詳細を確かめてください。",
        ],
      },
    };
    for (const [kind, texts] of Object.entries(canonical)) {
      const what = `error.what.${kind}`;
      const next = `error.next.${kind}`;
      expect(isMessageKey(what) && messages[what], what).toEqual({
        en: texts.what[0],
        ja: texts.what[1],
      });
      expect(isMessageKey(next) && messages[next], next).toEqual({
        en: texts.next[0],
        ja: texts.next[1],
      });
    }
    expect(messages["error.retry.fetch"]).toEqual({
      en: "Press [Fetch] to fetch again",
      ja: "[Fetch] でもう一度取得してください",
    });
    expect(messages["error.retry.listing"]).toEqual({
      en: "Press [Reload] to load the list again",
      ja: "[再読み込み] でもう一度読み込んでください",
    });
    expect(translate("ja", "status.detail", { detail: "x" })).toBe("詳細：x");
  });

  it("has the close confirmation word for word as BR3.3", () => {
    expect(messages["close.title"]).toEqual({ en: "Fetching is in progress.", ja: "取得中です。" });
    expect(messages["close.message"]).toEqual({
      en: "If you close the window, the logs fetched so far will be lost.",
      ja: "ウィンドウを閉じると、ここまで取得したログは失われます。",
    });
    expect(messages["close.keep"]).toEqual({ en: "Keep fetching", ja: "取得を続ける" });
    expect(messages["close.close"]).toEqual({ en: "Close", ja: "閉じる" });
  });

  it("drops the provisional kind-name texts and uses the same placeholders in both languages", () => {
    for (const gone of ["status.failed", "table.scroll"]) {
      expect(isMessageKey(gone), gone).toBe(false);
    }
    expect(messages["logGroups.partial"].en).not.toContain("{kind}");
    const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    for (const [key, texts] of Object.entries(messages)) {
      expect(placeholders(texts.ja), key).toEqual(placeholders(texts.en));
    }
  });

  it("draws no fixed text: every word on the U7 parts comes from a message key", () => {
    // A translator that shows only the key: anything else on screen is fixed text.
    const keysOnly: Translate = (key) => `⟦${key}⟧`;
    const failure: ApiFailure = { kind: "Throttled", safeDetail: "d", retryable: true };
    const failedJob = {
      jobId: 1,
      status: "Failed" as const,
      eventCount: 0,
      plannedStreamCount: 0,
      finishedStreamCount: 0,
      failedStreamCount: 0,
      failure,
    };
    const parts: Array<[ReactElement, string[]]> = [
      [
        createElement(FetchErrorBanner, {
          session: sessionView({ phase: "Failed", lastJob: failedJob }),
          t: keysOnly,
        }),
        [],
      ],
      [
        createElement(StatusLine, {
          session: sessionView({ phase: "Failed", lastJob: failedJob }),
          t: keysOnly,
          onOpenFailures: vi.fn(),
        }),
        [],
      ],
      [
        createElement(LogGroupPane, {
          session: connectedView({
            logGroups: {
              status: "Partial",
              visibleGroups: [],
              totalCount: 0,
              emptyState: null,
              failure,
            },
          }),
          t: keysOnly,
          onFilterChange: vi.fn(),
          onReload: vi.fn(),
          onSelect: vi.fn(),
        }),
        [],
      ],
      [
        createElement(FailureList, {
          failedStreams: [{ logStreamName: "stream-1", failure }],
          listingFailure: failure,
          t: keysOnly,
          onClose: vi.fn(),
        }),
        ["stream-1"],
      ],
      [createElement(CloseConfirmDialog, { t: keysOnly, onKeep: vi.fn(), onClose: vi.fn() }), []],
    ];
    for (const [element, data] of parts) {
      const { container, unmount } = render(element);
      let rest = (container.textContent ?? "").replace(/⟦[^⟧]*⟧/g, "");
      for (const value of data) {
        rest = rest.replaceAll(value, "");
      }
      expect(rest.trim(), container.innerHTML).toBe("");
      unmount();
    }
  });
});
