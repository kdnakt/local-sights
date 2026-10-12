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

// Keys produced by the Rust core (request.rs, failure.rs, session.rs, catalog,
// log_view.rs).
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
  "session.ssoLoginInProgress",
  "session.ssoLoginUnavailable",
  "selection.profileRequired",
  "selection.regionRequired",
  "selection.logGroupRequired",
  "catalog.unreadable.config",
  "catalog.unreadable.credentials",
  "rows.tooManyKeys",
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

  it("has every SSO login text in both languages (261011-sso-login NFR5)", () => {
    for (const key of [
      "ssoLogin.button",
      "ssoLogin.title",
      "ssoLogin.waiting",
      "ssoLogin.code",
      "ssoLogin.cancel",
      "ssoLogin.close",
      "ssoLogin.succeeded",
      "ssoLogin.timedOut",
      "ssoLogin.failed",
      "ssoLogin.failedToStart",
      "ssoLogin.cliNotFound",
      "ssoLogin.fallback",
      "ssoLogin.openFailed",
    ]) {
      expect(isMessageKey(key), key).toBe(true);
    }
    expect(translate("en", "ssoLogin.failed", { code: 3 })).toBe(
      "Could not sign in (exit code 3).",
    );
    expect(translate("ja", "ssoLogin.fallback", { profileOption: "--profile dev" })).toBe(
      "ターミナルで aws sso login --profile dev を実行してから、[再読み込み] を押してもログインできます。",
    );
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
    expect(translate("ja", "form.start.label", { zone: local })).toBe("開始（ローカル）");
    expect(translate("ja", "table.time", { zone: local })).toBe("時刻（ローカル）");
    expect(translate("en", "table.time", { zone: translate("en", timeZoneLabelKey("Utc")) })).toBe(
      "Time (UTC)",
    );
    expect(translate("ja", "validation.startFormat", { zone: local })).toBe(
      "開始は yyyy/mm/dd hh:mm:ss の形で入力してください（ローカル）。",
    );
  });

  // ---- console-style screen (261010-console-ui-polish, interaction-spec §9) ----

  it("has the console-style texts in both languages", () => {
    for (const key of [
      "form.start.date",
      "form.start.time",
      "form.end.date",
      "form.end.time",
      "form.date.placeholder",
      "form.time.placeholder",
      "form.date.required",
      "form.date.format",
      "form.time.format",
      "logGroups.column.name",
      "logGroups.selection",
      "logGroups.filter.clear",
      "logFilter.clear",
      "table.empty",
      "table.noMatches",
      "failures.column.stream",
      "failures.column.reason",
    ]) {
      expect(isMessageKey(key), key).toBe(true);
    }
    // Gate fix 15c: the heading states the selection; the prompt is the reason line's.
    expect(messages["form.selectedLogGroup.none"]).toEqual({
      en: "Log group: none",
      ja: "ロググループ: 未選択",
    });
    for (const locale of ["en", "ja"] as const) {
      expect(messages["form.selectedLogGroup.none"][locale]).not.toBe(
        messages["selection.logGroupRequired"][locale],
      );
    }
    expect(messages["logGroups.filter.label"]).toEqual({
      en: "Find log groups",
      ja: "ロググループを検索",
    });
    // Gate fix 15e: the field filters messages, not "events".
    expect(messages["logFilter.label"]).toEqual({
      en: "Filter messages",
      ja: "メッセージを絞り込み",
    });
    expect(messages["logFilter.placeholder"]).toEqual(messages["logFilter.label"]);
    expect(messages["form.date.format"]).toEqual({
      en: "Enter the date as yyyy/mm/dd.",
      ja: "日付は yyyy/mm/dd の形で入力してください。",
    });
    expect(messages["form.time.format"]).toEqual({
      en: "Enter the time as hh:mm:ss.",
      ja: "時刻は hh:mm:ss の形で入力してください。",
    });
    expect(messages["form.date.required"]).toEqual({
      en: "Enter a date.",
      ja: "日付を入力してください。",
    });
    expect(messages["validation.rangeOrder"]).toEqual({
      en: "End must be after start.",
      ja: "終了は開始より後にしてください。",
    });
    expect(messages["table.empty"]).toEqual({
      en: "No events. Select a log group, set a time range, then choose Fetch.",
      ja: "イベントがありません。ロググループを選び、期間を指定して [取得] を押してください。",
    });
  });

  it("writes every date as yyyy/mm/dd and keeps no hyphenated format text (FR2.3)", () => {
    for (const gone of ["form.reasons.title", "form.dateTime.placeholder"]) {
      expect(isMessageKey(gone), gone).toBe(false);
    }
    for (const [key, texts] of Object.entries(messages)) {
      expect(texts.en, key).not.toMatch(/yyyy-mm-dd/);
      expect(texts.ja, key).not.toMatch(/yyyy-mm-dd/);
    }
    for (const key of ["validation.startFormat", "validation.endFormat"] as const) {
      expect(messages[key].en).toContain("yyyy/mm/dd hh:mm:ss");
      expect(messages[key].ja).toContain("yyyy/mm/dd hh:mm:ss");
    }
    expect(translate("en", "validation.endFormat", { zone: "UTC" })).toBe(
      "Enter the end as a valid yyyy/mm/dd hh:mm:ss (UTC).",
    );
  });

  it("names the SDK default profile Default in both languages (gate fixes 14b, 15a)", () => {
    expect(messages["connection.profile.sdkDefault"]).toEqual({
      en: "Default",
      ja: "デフォルト",
    });
    expect(translate("ja", "connection.profile.sdkDefault").length).toBeLessThan(10);
    expect(translate("en", "connection.profile.sdkDefault")).not.toMatch(/SDK/);
  });

  it("names the app Local Sights in both languages and has the pane texts (gate fixes 14h, 14d, 15b)", () => {
    expect(messages["app.title"]).toEqual({ en: "Local Sights", ja: "Local Sights" });
    expect(isMessageKey("pane.resizer")).toBe(true);
    expect(messages["pane.collapse"]).toEqual({
      en: "Close the log group list",
      ja: "ロググループ一覧を閉じる",
    });
    expect(messages["pane.expand"]).toEqual({
      en: "Open the log group list",
      ja: "ロググループ一覧を開く",
    });
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

  it("has the U7 error texts word for word as BR2.2, with the {retry} texts of review R-02", () => {
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
      en: "press [Fetch] to fetch again",
      ja: "[取得] でもう一度取得してください",
    });
    expect(messages["error.retry.listing"]).toEqual({
      en: "press [Reload] to load the list again",
      ja: "[再読み込み] でもう一度読み込んでください",
    });
    // R-02: the button names are the ones on the screen, in both languages.
    for (const locale of ["en", "ja"] as const) {
      expect(messages["error.retry.fetch"][locale]).toContain(
        `[${messages["form.fetch"][locale]}]`,
      );
      expect(messages["error.retry.listing"][locale]).toContain(
        `[${messages["logGroups.reload"][locale]}]`,
      );
    }
    expect(translate("ja", "status.detail", { detail: "x" })).toBe("詳細：x");
  });

  it("has the texts of the row limit and of a refused positions request (R-04)", () => {
    expect(translate("en", "table.expandLimit", { max: "10,000" })).toBe(
      "Up to 10,000 rows can be open at once. Close a row to open another.",
    );
    expect(translate("ja", "table.expandLimit", { max: "10,000" })).toBe(
      "同時に開ける行は 10,000 行までです。別の行を開くには、開いている行を閉じてください。",
    );
    // Produced by the core (LogView::checked_positions_of, TooManyKeys::message_key).
    expect(isMessageKey("rows.tooManyKeys")).toBe(true);
    expect(translate("ja", "rows.tooManyKeys")).toBe(
      "一度に問い合わせた行が多すぎるため、一覧を更新できませんでした。",
    );
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
