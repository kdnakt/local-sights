/**
 * Message catalog (BR6.1). Every string shown on screen is looked up here by
 * key, in English or Japanese. Keys starting with `validation.`,
 * `failure.kind.` and `session.` are produced by the Rust core and must stay
 * in sync with it.
 */

export type Locale = "en" | "ja";

export const messages = {
  "app.title": { en: "local-sights", ja: "local-sights" },
  "form.label": { en: "Fetch conditions", ja: "取得条件" },
  "form.selectedLogGroup.label": { en: "Log group", ja: "ロググループ" },
  "form.selectedLogGroup.none": { en: "Not selected", ja: "未選択" },
  "form.logStream.label": { en: "Log stream", ja: "ストリーム" },
  "form.logStream.placeholder": {
    en: "2024/01/02/[$LATEST]0123456789abcdef",
    ja: "2024/01/02/[$LATEST]0123456789abcdef",
  },
  "form.start.label": { en: "Start (UTC)", ja: "開始（UTC）" },
  "form.end.label": { en: "End (UTC)", ja: "終了（UTC）" },
  "form.dateTime.placeholder": { en: "yyyy-mm-dd hh:mm:ss", ja: "yyyy-mm-dd hh:mm:ss" },
  "form.fetch": { en: "Fetch", ja: "取得" },
  "form.reasons.title": { en: "Fetch is unavailable because:", ja: "取得できない理由：" },
  "validation.logGroupRequired": {
    en: "Enter a log group name.",
    ja: "ロググループ名を入力してください。",
  },
  "validation.logGroupTooLong": {
    en: "The log group name must be 512 characters or fewer.",
    ja: "ロググループ名は 512 文字以内にしてください。",
  },
  "validation.logStreamRequired": {
    en: "Enter a log stream name.",
    ja: "ストリーム名を入力してください。",
  },
  "validation.logStreamTooLong": {
    en: "The log stream name must be 512 characters or fewer.",
    ja: "ストリーム名は 512 文字以内にしてください。",
  },
  "validation.startFormat": {
    en: "Enter the start as a valid yyyy-mm-dd hh:mm:ss (UTC).",
    ja: "開始日時を正しい yyyy-mm-dd hh:mm:ss（UTC）で入力してください。",
  },
  "validation.endFormat": {
    en: "Enter the end as a valid yyyy-mm-dd hh:mm:ss (UTC).",
    ja: "終了日時を正しい yyyy-mm-dd hh:mm:ss（UTC）で入力してください。",
  },
  "validation.rangeOrder": {
    en: "The start must be before the end.",
    ja: "開始日時は終了日時より前にしてください。",
  },
  "session.busy": { en: "A fetch is in progress.", ja: "取得中です。" },
  "session.confirmationPending": {
    en: "Answer the confirmation first.",
    ja: "先に確認に答えてください。",
  },
  "session.notConnected": {
    en: "Choose a profile and a region first.",
    ja: "先にプロファイルとリージョンを選んでください。",
  },
  "session.unknownProfile": {
    en: "That profile is not in the list.",
    ja: "そのプロファイルは一覧にありません。",
  },
  "session.unknownRegion": {
    en: "That region is not in the list.",
    ja: "そのリージョンは一覧にありません。",
  },
  "session.unknownLogGroup": {
    en: "That log group is not in the list.",
    ja: "そのロググループは一覧にありません。",
  },
  "session.nothingPending": {
    en: "There is no change to confirm.",
    ja: "確認する変更はありません。",
  },
  "selection.profileRequired": { en: "Choose a profile.", ja: "プロファイルを選んでください。" },
  "selection.regionRequired": { en: "Choose a region.", ja: "リージョンを選んでください。" },
  "selection.logGroupRequired": {
    en: "Choose a log group.",
    ja: "ロググループを選んでください。",
  },
  "connection.label": { en: "Connection", ja: "接続" },
  "connection.profile.label": { en: "Profile", ja: "プロファイル" },
  "connection.profile.placeholder": { en: "Choose a profile", ja: "プロファイルを選択" },
  "connection.profile.sdkDefault": {
    en: "Default settings (left to the SDK)",
    ja: "既定の設定（SDK に任せる）",
  },
  "connection.region.label": { en: "Region", ja: "リージョン" },
  "connection.region.placeholder": { en: "Choose a region", ja: "リージョンを選択" },
  "catalog.unreadable.config": {
    en: "The AWS config file could not be read; its profiles are not listed.",
    ja: "AWS の config ファイルを読めませんでした。そのプロファイルは一覧にありません。",
  },
  "catalog.unreadable.credentials": {
    en: "The AWS credentials file could not be read; its profiles are not listed.",
    ja: "AWS の credentials ファイルを読めませんでした。そのプロファイルは一覧にありません。",
  },
  "logGroups.label": { en: "Log groups", ja: "ロググループ" },
  "logGroups.filter.label": { en: "Filter", ja: "絞り込み" },
  "logGroups.filter.placeholder": { en: "Part of a name", ja: "名前の一部" },
  "logGroups.reload": { en: "Reload", ja: "再読み込み" },
  "logGroups.loading": { en: "Loading…", ja: "読み込み中…" },
  "logGroups.partial": {
    en: "The list is incomplete: {kind}",
    ja: "一覧は途中までです：{kind}",
  },
  "logGroups.empty.noGroups": { en: "There are no log groups.", ja: "ロググループがありません。" },
  "logGroups.empty.noMatches": {
    en: "No log groups match.",
    ja: "一致するロググループがありません。",
  },
  "logGroups.prompt.profile": { en: "Choose a profile.", ja: "プロファイルを選んでください。" },
  "logGroups.prompt.region": { en: "Choose a region.", ja: "リージョンを選んでください。" },
  "confirm.title": { en: "Change the connection?", ja: "接続を変えますか？" },
  "confirm.message": {
    en: "The logs shown will be cleared. Change the connection?",
    ja: "表示中のログが消えます。変えてよいですか？",
  },
  "confirm.change": { en: "Change", ja: "変える" },
  "confirm.cancel": { en: "Cancel", ja: "キャンセル" },
  "session.invalid": {
    en: "The fetch conditions are not valid.",
    ja: "取得条件が正しくありません。",
  },
  "status.idle": {
    en: "Enter the conditions and press Fetch.",
    ja: "条件を入力して［取得］を押してください。",
  },
  "status.fetching": {
    en: "Fetching… {count} events so far",
    ja: "取得中… これまでに {count} 件",
  },
  "status.count": { en: "{count} events", ja: "{count} 件" },
  "status.zero": {
    en: "0 events: no logs in this range.",
    ja: "0 件：この範囲にログはありません。",
  },
  "status.failed": { en: "Failed: {kind}", ja: "失敗：{kind}" },
  "status.detail": { en: "Details: {detail}", ja: "詳細：{detail}" },
  "failure.kind.AuthRequired": {
    en: "Authentication required",
    ja: "認証が必要",
  },
  "failure.kind.AccessDenied": { en: "Access denied", ja: "権限がない" },
  "failure.kind.Throttled": { en: "Throttled", ja: "スロットリング" },
  "failure.kind.Network": { en: "Network error", ja: "通信エラー" },
  "failure.kind.NotFound": { en: "Not found", ja: "見つからない" },
  "failure.kind.InvalidInput": { en: "Invalid input", ja: "入力の誤り" },
  "failure.kind.RegionMissing": {
    en: "No default region for the profile",
    ja: "プロファイルに既定のリージョンがない",
  },
  "failure.kind.Other": { en: "Other error", ja: "その他のエラー" },
  "table.label": { en: "Log events", ja: "ログ" },
  "table.time": { en: "Time (UTC)", ja: "時刻（UTC）" },
  "table.message": { en: "Message", ja: "メッセージ" },
  "error.command": {
    en: "The operation failed: {detail}",
    ja: "操作に失敗しました：{detail}",
  },
  "error.dismiss": { en: "Dismiss (Esc)", ja: "閉じる（Esc）" },
} as const satisfies Record<string, Record<Locale, string>>;

export type MessageKey = keyof typeof messages;

export type Translate = (key: string, params?: Record<string, string | number>) => string;

/** Whether `key` exists in the catalog. */
export function isMessageKey(key: string): key is MessageKey {
  return Object.prototype.hasOwnProperty.call(messages, key);
}

/** Japanese when the first preferred OS/browser language is Japanese, else English. */
export function detectLocale(languages: readonly string[]): Locale {
  const first = languages[0]?.toLowerCase() ?? "";
  return first === "ja" || first.startsWith("ja-") ? "ja" : "en";
}

/**
 * Looks up `key` and fills `{name}` placeholders. An unknown key is returned
 * as is, so a missing entry is visible instead of hidden.
 */
export function translate(
  locale: Locale,
  key: string,
  params: Record<string, string | number> = {},
): string {
  const template = isMessageKey(key) ? messages[key][locale] : key;
  return template.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    name in params ? String(params[name]) : placeholder,
  );
}

/** A translator bound to one locale. */
export function createTranslator(locale: Locale): Translate {
  return (key, params) => translate(locale, key, params);
}
