/**
 * Message catalog (BR6.1). Every string shown on screen is looked up here by
 * key, in English or Japanese. Keys starting with `validation.`,
 * `failure.kind.`, `session.` and `rows.` are produced by the Rust core and
 * must stay in sync with it. Since U4 the time labels take a `{zone}` placeholder filled
 * with `timeZone.local` or `timeZone.utc` (U4:BR3.2). Since U5 the log
 * filter field and its status texts (`logFilter.*`, `status.filter.*`).
 * Since U6 the settings dialog (`settings.*`) and the cache notices of the
 * status line (`cache.*`), whose wording is U6:BR5.3 word for word.
 * Since U7 the error texts (`error.*`, U7:BR2.2 word for word, replacing the
 * provisional "Failed: kind name" of U1:BR4.4), the close confirmation
 * (`close.*`, U7:BR3.3 word for word) and the expanded message's name
 * (`table.expanded`), the notice when no more rows can be opened
 * (`table.expandLimit`, code generation review R-04); the U3 scrolling hint of the list is gone (U7:BR1.5).
 * The `{retry}` texts (`error.retry.*`) differ from BR2.2 as approved at
 * the code generation review (R-02): the Japanese names the button as
 * shown ([取得]), and the English starts with a lowercase "press" because
 * it is put inside a sentence (`errorLines` capitalizes it when the next
 * action starts with it).
 * `failure.kind.*` stay because the core names the kinds with them.
 * Since the console-style screen (261010-console-ui-polish, interaction-spec
 * §9): the date and time are written `yyyy/mm/dd` and `hh:mm:ss` (FR2.3),
 * the start and end are split into a date and a time field with their own
 * reasons (`form.date.*`, `form.time.*`, FR3.1), the heading says "Log
 * group: none" when none is selected (gate fix 15c: the prompt "Choose a log
 * group." is said once, by the reason line), the two filter fields read "Find
 * log groups" and "Filter messages" (gate fix 15e), the log list has an empty state and a
 * no-match text (`table.empty`, `table.noMatches`), and the "Fetch is
 * unavailable because:" list is gone (FR3.1). `error.what.*` / `error.next.*`
 * are unchanged (FR3.4).
 * Since 261011-sso-login the SSO login texts (`ssoLogin.*`, the wording of
 * frontend-components.md, plus `ssoLogin.failedToStart` for a CLI that did
 * not start and `ssoLogin.openFailed` for a URL that could not be opened)
 * and the core's `session.ssoLoginInProgress` / `session.ssoLoginUnavailable`.
 * Since 261012-parallel-fetch U4 the elapsed time of a fetch on the status
 * line (`status.elapsed`, `{time}` is mm:ss or h:mm:ss, BR4.7, BR4.8).
 * Since 261012-parallel-fetch U5 the [Cancel] button (`fetch.cancel`,
 * `fetch.cancelling`), the status word of a cancelled fetch
 * (`status.cancelled`) and the two cancel notices (`cancel.notice.*`, `{n}`
 * is the number of streams not read to their end), word for word as the
 * refined mockups' wording list (BR5.13, NFR9.3).
 * Since 261017-insights-query U1 the navigation (`nav.*`), the Log Analytics
 * screen (`insights.*`, the refined mockups' wording list where U1 has the
 * same thing), the core's reasons why [Run query] cannot be pressed
 * (`insights.blocked.*`), the reading problems of a query (`query.syntax.*`,
 * `query.unsupported.*`, `{word}` is the query's own word only, FR6.4), and
 * the core's `session.queryRunning` and `session.staleInput`.
 * Since 261017-insights-query U2 the reading problems of `fields`, `sort`,
 * `limit`, `not` and JSON fields, the results' counts and cut
 * (`insights.results.count`, `insights.results.truncated`, the refined
 * mockups' wording), the opened row's loading text, the evaluation failure
 * (`insights.evaluationFailed*`, U2:BR10.2), the commands this version runs
 * (`insights.error.supportedList`, the refined mockups' wording) and the
 * examples and completion of the query field (`insights.examples*`,
 * `insights.completion.*`, Should).
 * Since 261020-carryover-fixes FR4.1 the English texts that count streams
 * have a singular form too (`englishSingular`): `messages` keeps the plural
 * form, and `translate` alone picks the singular when the text's deciding
 * count is exactly 1 (0 stays plural). The Japanese texts have one form.
 * Since 261020-carryover-fixes FR5 the region picker's texts
 * (`connection.region.noMatch`, `connection.region.filterPlaceholder`); the
 * region names themselves are in `regions.ts`.
 * Since 261022-small-features U1 the language choice of the settings dialog
 * (`settings.language*`, U1:BR1.1: "日本語" and "English" are fixed in their
 * own language, only the OS choice is translated) and `resolveLocale`, which
 * turns the saved preference and the OS languages into the one locale every
 * text follows (U1:BR1.2).
 * Since 261022-small-features U2 the export menu and its four items
 * (`export.menu`, `export.item.*`, U2:BR4.1), the notices of the last export
 * (`export.notice.*`, `{path}` and `{count}` are the core's, U2:BR4.9), the
 * failure kinds the core names (`export.failure.*`, review R-02), the core's
 * `session.exportUnavailable` / `session.exportBusy`, and the "Formatted /
 * Original" switch of an opened row (`expanded.mode*`, U2:BR5.3).
 */

import type { TimeZoneChoice } from "../api";

export type Locale = "en" | "ja";

/**
 * The user's language preference as saved in the settings file and carried
 * by the session view (261022-small-features U1, ADR-005): how the locale is
 * decided, not the locale itself. Spelled as the Rust core spells it.
 */
export type LanguagePreference = "system" | "ja" | "en";

/** Every screen text by message key, in English and Japanese (NFR12). */
export const messages = {
  "app.title": { en: "Local Sights", ja: "Local Sights" },
  "form.label": { en: "Fetch conditions", ja: "取得条件" },
  "form.selectedLogGroup.label": { en: "Log group", ja: "ロググループ" },
  "form.selectedLogGroup.none": { en: "Log group: none", ja: "ロググループ: 未選択" },
  "form.start.label": { en: "Start ({zone})", ja: "開始（{zone}）" },
  "form.end.label": { en: "End ({zone})", ja: "終了（{zone}）" },
  "form.start.date": { en: "Start date", ja: "開始日" },
  "form.start.time": { en: "Start time", ja: "開始時刻" },
  "form.end.date": { en: "End date", ja: "終了日" },
  "form.end.time": { en: "End time", ja: "終了時刻" },
  "form.date.placeholder": { en: "yyyy/mm/dd", ja: "yyyy/mm/dd" },
  "form.time.placeholder": { en: "hh:mm:ss", ja: "hh:mm:ss" },
  "form.date.required": { en: "Enter a date.", ja: "日付を入力してください。" },
  "form.date.format": {
    en: "Enter the date as yyyy/mm/dd.",
    ja: "日付は yyyy/mm/dd の形で入力してください。",
  },
  "form.time.format": {
    en: "Enter the time as hh:mm:ss.",
    ja: "時刻は hh:mm:ss の形で入力してください。",
  },
  "form.fetch": { en: "Fetch", ja: "取得" },
  "validation.logGroupRequired": {
    en: "Enter a log group name.",
    ja: "ロググループ名を入力してください。",
  },
  "validation.logGroupTooLong": {
    en: "The log group name must be 512 characters or fewer.",
    ja: "ロググループ名は 512 文字以内にしてください。",
  },
  "validation.startFormat": {
    en: "Enter the start as a valid yyyy/mm/dd hh:mm:ss ({zone}).",
    ja: "開始は yyyy/mm/dd hh:mm:ss の形で入力してください（{zone}）。",
  },
  "validation.startNonexistentLocalTime": {
    en: "The start does not exist because of the daylight saving time change.",
    ja: "開始日時は夏時間の切り替えで存在しない日時です。",
  },
  "validation.endFormat": {
    en: "Enter the end as a valid yyyy/mm/dd hh:mm:ss ({zone}).",
    ja: "終了は yyyy/mm/dd hh:mm:ss の形で入力してください（{zone}）。",
  },
  "validation.endNonexistentLocalTime": {
    en: "The end does not exist because of the daylight saving time change.",
    ja: "終了日時は夏時間の切り替えで存在しない日時です。",
  },
  "validation.rangeOrder": {
    en: "End must be after start.",
    ja: "終了は開始より後にしてください。",
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
  "session.staleGeneration": {
    en: "The connection changed; the operation was ignored.",
    ja: "接続先が変わったため、操作は取り消されました。",
  },
  "session.noFailures": {
    en: "There are no failures to show.",
    ja: "表示する失敗はありません。",
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
  "connection.profile.sdkDefault": { en: "Default", ja: "デフォルト" },
  "connection.region.label": { en: "Region", ja: "リージョン" },
  "connection.region.placeholder": { en: "Choose a region", ja: "リージョンを選択" },
  "connection.region.noMatch": {
    en: "No matching regions",
    ja: "一致するリージョンはありません",
  },
  "connection.region.filterPlaceholder": {
    en: "Find a region",
    ja: "リージョンを検索",
  },
  "timeZone.label": { en: "Time zone", ja: "タイムゾーン" },
  "timeZone.local": { en: "Local", ja: "ローカル" },
  "timeZone.utc": { en: "UTC", ja: "UTC" },
  "catalog.unreadable.config": {
    en: "The AWS config file could not be read; its profiles are not listed.",
    ja: "AWS の config ファイルを読めませんでした。そのプロファイルは一覧にありません。",
  },
  "catalog.unreadable.credentials": {
    en: "The AWS credentials file could not be read; its profiles are not listed.",
    ja: "AWS の credentials ファイルを読めませんでした。そのプロファイルは一覧にありません。",
  },
  "logGroups.label": { en: "Log groups", ja: "ロググループ" },
  "pane.resizer": { en: "Log group pane width", ja: "ロググループ一覧の幅" },
  "pane.collapse": { en: "Close the log group list", ja: "ロググループ一覧を閉じる" },
  "pane.expand": { en: "Open the log group list", ja: "ロググループ一覧を開く" },
  "logGroups.column.name": { en: "Name", ja: "名前" },
  "logGroups.selection": { en: "Log group selection", ja: "ロググループの選択" },
  "logGroups.filter.label": { en: "Find log groups", ja: "ロググループを検索" },
  "logGroups.filter.placeholder": { en: "Find log groups", ja: "ロググループを検索" },
  "logGroups.filter.clear": { en: "Clear", ja: "消去" },
  "logGroups.reload": { en: "Reload", ja: "再読み込み" },
  "logGroups.loading": { en: "Loading…", ja: "読み込み中…" },
  "logGroups.partial": {
    en: "The list is incomplete.",
    ja: "一覧は途中までです。",
  },
  "logGroups.empty.noGroups": { en: "There are no log groups.", ja: "ロググループがありません。" },
  "logGroups.empty.noMatches": {
    en: "No log groups match.",
    ja: "一致するロググループがありません。",
  },
  "logGroups.prompt.profile": { en: "Choose a profile.", ja: "プロファイルを選んでください。" },
  "logGroups.prompt.region": { en: "Choose a region.", ja: "リージョンを選んでください。" },
  "confirm.title": { en: "Change the connection?", ja: "接続を変えますか？" },
  // 261017-insights-query U1:BR7.1: both screens' logs and results go.
  "confirm.message": {
    en: "The logs shown in Log Management and the query results in Log Analytics will be cleared. Change the connection?",
    ja: "ログ管理の画面に表示中のログと、Log Analytics のクエリの結果が消えます。変えてよいですか？",
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
  "status.listing": {
    en: "Listing streams… {count} streams selected so far",
    ja: "ストリームを列挙中… これまでに {count} ストリームを対象にしました",
  },
  "status.fetching": {
    en: "Fetching… {finished}/{planned} streams, {count} events so far",
    ja: "取得中… {finished}/{planned} ストリーム、これまでに {count} 件",
  },
  "status.failedStreams": {
    en: "Failed in {count} streams",
    ja: "{count} ストリームで失敗",
  },
  "status.showFailures": { en: "Show failure details", ja: "失敗の詳細を見る" },
  "status.listingPartial": {
    en: "The stream listing is incomplete.",
    ja: "ストリームの列挙は途中までです。",
  },
  "status.count": { en: "{count} events", ja: "{count} 件" },
  "status.zero": {
    en: "0 events: no logs in this range.",
    ja: "0 件：この範囲にログはありません。",
  },
  "status.detail": { en: "Details: {detail}", ja: "詳細：{detail}" },
  "status.elapsed": { en: "Elapsed {time}", ja: "経過 {time}" },
  "status.cancelled": { en: "Cancelled", ja: "中断" },
  "fetch.cancel": { en: "Cancel", ja: "中断" },
  "fetch.cancelling": { en: "Cancelling…", ja: "中断中…" },
  "cancel.notice.afterListing": {
    en: "Fetch cancelled. {n} streams were not finished, so some logs are missing.",
    ja: "取得を中断しました。{n} 本のストリームは読み終えていないため、一部のログが含まれていません。",
  },
  "cancel.notice.beforeListing": {
    en: "Fetch cancelled before the stream listing finished, so some logs are missing.",
    ja: "ストリームの一覧を読み終える前に中断したため、一部のログが含まれていません。",
  },
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
  "table.time": { en: "Time ({zone})", ja: "時刻（{zone}）" },
  "table.stream": { en: "Stream", ja: "ストリーム名" },
  "table.message": { en: "Message", ja: "メッセージ" },
  "table.expanded": { en: "Full message", ja: "メッセージの全文" },
  "table.empty": {
    en: "No events. Select a log group, set a time range, then choose Fetch.",
    ja: "イベントがありません。ロググループを選び、期間を指定して [取得] を押してください。",
  },
  "table.noMatches": {
    en: "No events match the filter.",
    ja: "絞り込みに一致するイベントがありません。",
  },
  "rows.tooManyKeys": {
    en: "Too many rows were asked for at once, so the list was not updated.",
    ja: "一度に問い合わせた行が多すぎるため、一覧を更新できませんでした。",
  },
  "table.expandLimit": {
    en: "Up to {max} rows can be open at once. Close a row to open another.",
    ja: "同時に開ける行は {max} 行までです。別の行を開くには、開いている行を閉じてください。",
  },
  "failures.title": { en: "Failures", ja: "失敗の一覧" },
  "failures.listing": { en: "Stream listing", ja: "ストリームの列挙" },
  "failures.column.stream": { en: "Stream", ja: "ストリーム" },
  "failures.column.groupStream": { en: "Log group / stream", ja: "ロググループ / ストリーム" },
  "failures.column.reason": {
    en: "What happened / what to do",
    ja: "何が起きたか／次の行動",
  },
  "failures.close": { en: "Close (Esc)", ja: "閉じる（Esc）" },
  "error.command": {
    en: "The operation failed: {detail}",
    ja: "操作に失敗しました：{detail}",
  },
  "error.dismiss": { en: "Dismiss (Esc)", ja: "閉じる（Esc）" },
  "logFilter.label": { en: "Filter messages", ja: "メッセージを絞り込み" },
  "logFilter.placeholder": { en: "Filter messages", ja: "メッセージを絞り込み" },
  "logFilter.clear": { en: "Clear", ja: "消去" },
  "status.filter.count": {
    en: "Filtered: {matched} of {all} events",
    ja: "絞り込み後 {matched} 件 / 全 {all} 件",
  },
  "status.filter.zero": {
    en: "Filtered: 0 of {all} events, no logs match.",
    ja: "絞り込み後 0 件 / 全 {all} 件：一致するログはありません。",
  },
  "status.filter.filtering": { en: "Filtering…", ja: "絞り込み中…" },
  "session.settingsClosed": {
    en: "The settings dialog is not open.",
    ja: "設定ダイアログは開いていません。",
  },
  "settings.open": { en: "Settings", ja: "設定" },
  "settings.title": { en: "Settings", ja: "設定" },
  "settings.cacheEnabled": {
    en: "Keep fetched logs in a disk cache",
    ja: "取得したログをディスクにキャッシュする",
  },
  // 261022-small-features U1:BR1.1: the two languages are named in their own
  // language, whatever the screen's, so the way back stays readable.
  "settings.language": { en: "Language", ja: "言語" },
  "settings.language.system": { en: "Follow the OS setting", ja: "OS の設定に合わせる" },
  "settings.language.ja": { en: "日本語", ja: "日本語" },
  "settings.language.en": { en: "English", ja: "English" },
  "settings.location": { en: "Cache location: {path}", ja: "保存場所：{path}" },
  "settings.locationUnknown": { en: "not available", ja: "なし" },
  "settings.warning": {
    en: "Logs may contain confidential information. Cached logs stay on this computer until you clear them.",
    ja: "ログには機密情報が含まれることがあります。キャッシュしたログは消すまでこのコンピューターに残ります。",
  },
  "settings.clear": { en: "Clear cache", ja: "キャッシュを消す" },
  "settings.cancel": { en: "Cancel", ja: "キャンセル" },
  "settings.save": { en: "Save", ja: "保存" },
  "settings.notice.Cleared": { en: "The cache was cleared.", ja: "キャッシュを消しました。" },
  "settings.notice.ClearFailed": {
    en: "The cache could not be cleared.",
    ja: "キャッシュを消せませんでした。",
  },
  "settings.notice.SaveFailed": {
    en: "The setting could not be saved.",
    ja: "設定を保存できませんでした。",
  },
  "cache.saving": { en: "Saving to cache", ja: "キャッシュに保存中" },
  "cache.hit": {
    en: "Shown from cache (AWS was not called)",
    ja: "キャッシュから表示（AWS は呼んでいない）",
  },
  "cache.readFailed": {
    en: "The cache could not be read, so the logs were fetched again",
    ja: "キャッシュが読めなかったので取り直した",
  },
  "cache.saveFailed": { en: "Could not save to the cache", ja: "キャッシュに保存できなかった" },
  "error.retry.fetch": {
    en: "press [Fetch] to fetch again",
    ja: "[取得] でもう一度取得してください",
  },
  "error.retry.listing": {
    en: "press [Reload] to load the list again",
    ja: "[再読み込み] でもう一度読み込んでください",
  },
  "error.what.AuthRequired": {
    en: "Credentials could not be used (for example, the SSO session expired).",
    ja: "認証情報を使えませんでした（SSO のログイン切れなど）。",
  },
  "error.next.AuthRequired": {
    en: "If you use SSO, run aws sso login, then {retry}. Otherwise, check the profile settings or choose another profile.",
    ja: "SSO を使っているときは aws sso login を実行してから、{retry}。それ以外はプロファイルの設定を確認するか、別のプロファイルを選んでください。",
  },
  "error.what.AccessDenied": { en: "Access was denied.", ja: "権限がないため拒否されました。" },
  "error.next.AccessDenied": {
    en: "Check the IAM permissions or choose another profile.",
    ja: "IAM の権限を確認するか、別のプロファイルを選んでください。",
  },
  "error.what.Throttled": {
    en: "AWS throttled the requests.",
    ja: "AWS の呼び出しが多すぎるため、制限されました。",
  },
  "error.next.Throttled": {
    en: "Wait a while, then {retry}.",
    ja: "しばらく待ってから、{retry}。",
  },
  "error.what.Network": { en: "Could not connect to AWS.", ja: "AWS に接続できませんでした。" },
  "error.next.Network": {
    en: "Check your connection, then {retry}.",
    ja: "接続を確認してから、{retry}。",
  },
  "error.what.NotFound": {
    en: "The log group or stream was not found.",
    ja: "ロググループかストリームが見つかりませんでした。",
  },
  "error.next.NotFound": {
    en: "Press [Reload] to load the log group list again and choose again.",
    ja: "[再読み込み] でロググループの一覧を読み込み直して、選び直してください。",
  },
  "error.what.InvalidInput": {
    en: "AWS rejected the request.",
    ja: "AWS が条件を受け付けませんでした。",
  },
  "error.next.InvalidInput": {
    en: "Check the time range, then {retry}.",
    ja: "時間範囲を見直してから、{retry}。",
  },
  "error.what.RegionMissing": {
    en: "This profile has no default region.",
    ja: "このプロファイルには既定のリージョンがありません。",
  },
  "error.next.RegionMissing": {
    en: "Choose a region in the top bar.",
    ja: "上部バーでリージョンを選んでください。",
  },
  "error.what.Other": {
    en: "Something went wrong while calling AWS.",
    ja: "AWS の呼び出しで問題が起きました。",
  },
  "error.next.Other": {
    en: "{retry}. If it keeps happening, check the details.",
    ja: "{retry}。続くときは詳細を確かめてください。",
  },
  "close.title": { en: "Fetching is in progress.", ja: "取得中です。" },
  "close.message": {
    en: "If you close the window, the logs fetched so far will be lost.",
    ja: "ウィンドウを閉じると、ここまで取得したログは失われます。",
  },
  "close.keep": { en: "Keep fetching", ja: "取得を続ける" },
  "close.close": { en: "Close", ja: "閉じる" },
  "ssoLogin.button": { en: "Sign in with SSO", ja: "SSO にログイン" },
  "ssoLogin.title": { en: "Sign in with SSO", ja: "SSO にログイン" },
  "ssoLogin.waiting": {
    en: "Approve the sign-in in your browser. If the browser does not open, open this URL and enter the code.",
    ja: "ブラウザでログインを承認してください。ブラウザが開かない場合は、次の URL を開いて確認コードを入力してください。",
  },
  "ssoLogin.code": { en: "Verification code", ja: "確認コード" },
  "ssoLogin.cancel": { en: "Cancel", ja: "キャンセル" },
  "ssoLogin.close": { en: "Close", ja: "閉じる" },
  "ssoLogin.succeeded": {
    en: "Signed in. Reloading the log groups.",
    ja: "ログインしました。ロググループの一覧を読み込み直しています。",
  },
  "ssoLogin.timedOut": {
    en: "The sign-in timed out. Try again.",
    ja: "時間切れになりました。もう一度ログインしてください。",
  },
  "ssoLogin.failed": {
    en: "Could not sign in (exit code {code}).",
    ja: "ログインできませんでした（終了コード {code}）。",
  },
  "ssoLogin.failedToStart": {
    en: "Could not sign in: the AWS CLI could not be started.",
    ja: "ログインできませんでした。AWS CLI を起動できませんでした。",
  },
  "ssoLogin.cliNotFound": {
    en: "AWS CLI was not found. Install AWS CLI v2.",
    ja: "AWS CLI が見つかりません。AWS CLI v2 をインストールしてください。",
  },
  "ssoLogin.fallback": {
    en: "You can also run aws sso login {profileOption} in a terminal and press [Reload].",
    ja: "ターミナルで aws sso login {profileOption} を実行してから、[再読み込み] を押してもログインできます。",
  },
  "ssoLogin.openFailed": {
    en: "The sign-in page could not be opened. Open the URL in your browser yourself.",
    ja: "ログインのページを開けませんでした。URL をブラウザで開いてください。",
  },
  "session.ssoLoginInProgress": {
    en: "This cannot be done while signing in with SSO.",
    ja: "SSO にログイン中はこの操作はできません。",
  },
  "session.ssoLoginUnavailable": {
    en: "Signing in with SSO is not available for the selected profile.",
    ja: "選んでいるプロファイルでは SSO にログインできません。",
  },
  "session.queryRunning": {
    en: "A query is running in Log Analytics.",
    ja: "Log Analytics でクエリを実行中です。",
  },
  "session.staleInput": {
    en: "The query changed while it was being read. Run it again.",
    ja: "読み取っている間にクエリが変わりました。もう一度実行してください。",
  },
  "nav.label": { en: "Screens", ja: "画面" },
  "nav.title": { en: "CloudWatch Logs", ja: "CloudWatch Logs" },
  "nav.logManagement": { en: "Log Management", ja: "ログ管理" },
  "nav.logAnalytics": { en: "Log Analytics", ja: "Log Analytics" },
  "nav.close": { en: "Close navigation", ja: "ナビゲーションを閉じる" },
  "nav.open": { en: "Open navigation", ja: "ナビゲーションを開く" },
  "insights.title": { en: "Log Analytics", ja: "Log Analytics" },
  "insights.logGroups.label": { en: "Log groups", ja: "ロググループ" },
  "insights.logGroups.placeholder": { en: "Select log groups", ja: "ロググループを選ぶ" },
  "insights.logGroups.limit": {
    en: "You can choose up to {max} log groups.",
    ja: "ロググループは {max} 個まで選べます。",
  },
  "insights.logGroups.filter": { en: "Find log groups", ja: "ロググループを探す" },
  "insights.logGroups.loading": { en: "Loading log groups", ja: "ロググループを読み込み中" },
  "insights.logGroups.error": {
    en: "Couldn't load log groups",
    ja: "ロググループを読み込めませんでした",
  },
  "insights.logGroups.retry": { en: "Retry", ja: "再試行" },
  "insights.logGroups.empty": { en: "No log groups", ja: "ロググループがありません" },
  "insights.logGroups.noMatch": {
    en: "No matching log groups",
    ja: "一致するロググループがありません",
  },
  "insights.logGroups.remove": { en: "Remove {name}", ja: "{name} を外す" },
  "insights.query.label": { en: "Query", ja: "クエリ" },
  "insights.query.loading": { en: "Loading the editor", ja: "エディタを読み込み中" },
  "insights.query.loadFailed": {
    en: "The editor could not be loaded.",
    ja: "エディタを読み込めませんでした。",
  },
  "insights.query.language": { en: "Logs Insights query", ja: "Logs Insights のクエリ" },
  "insights.completion.command": { en: "command", ja: "命令" },
  "insights.completion.keyword": { en: "keyword", ja: "言葉" },
  "insights.completion.field": { en: "field", ja: "項目" },
  "insights.completion.jsonField": { en: "JSON field", ja: "JSON の項目" },
  "insights.completion.function": { en: "function", ja: "関数" },
  "insights.examples": { en: "Examples", ja: "例" },
  "insights.examples.errors": {
    en: "Errors that are not timeouts",
    ja: "タイムアウト以外のエラー",
  },
  "insights.examples.jsonLevel": {
    en: "JSON logs at error or warn level",
    ja: "JSON のログで level が error か warn",
  },
  "insights.examples.httpStatus": {
    en: "HTTP status 500 and above, largest first",
    ja: "HTTP の状態が 500 以上、大きい順",
  },
  "insights.examples.excluded": {
    en: "Error codes other than two",
    ja: "2 つを除いたエラーコード",
  },
  "insights.examples.stream": {
    en: "One kind of stream without health checks, oldest first",
    ja: "ヘルスチェックを除いた特定のストリーム、古い順",
  },
  "insights.examples.confirm.title": { en: "Replace the query?", ja: "クエリを置き換えますか？" },
  "insights.examples.confirm.message": {
    en: "The query you wrote is replaced by the example.",
    ja: "書いたクエリは例に置き換わります。",
  },
  "insights.examples.confirm.replace": { en: "Replace", ja: "置き換える" },
  "insights.examples.confirm.cancel": { en: "Keep my query", ja: "今のクエリを残す" },
  "insights.run": { en: "Run query", ja: "クエリを実行" },
  "insights.cancel": { en: "Cancel", ja: "中止" },
  "insights.cancelling": { en: "Cancelling…", ja: "中止中…" },
  "insights.shortcut.mac": { en: "Cmd+Enter", ja: "Cmd+Enter" },
  "insights.shortcut.other": { en: "Ctrl+Enter", ja: "Ctrl+Enter" },
  "insights.blocked.notConnected": {
    en: "Choose a profile and a region.",
    ja: "プロファイルとリージョンを選んでください。",
  },
  "insights.blocked.logGroupRequired": {
    en: "Choose at least one log group.",
    ja: "ロググループを 1 つ以上選んでください。",
  },
  "insights.blocked.rangeInvalid": { en: "Fix the time range.", ja: "期間を直してください。" },
  "insights.blocked.queryEmpty": { en: "Write a query.", ja: "クエリを書いてください。" },
  "insights.blocked.queryTooLong": {
    en: "The query must be 10,000 characters or fewer.",
    ja: "クエリは 10,000 文字以内にしてください。",
  },
  "insights.blocked.queryRunning": { en: "The query is running.", ja: "クエリを実行中です。" },
  "insights.blocked.logManagementFetching": {
    en: "Log Management is fetching logs.",
    ja: "Log Management で取得中です。",
  },
  "insights.blocked.otherOperation": {
    en: "Finish the open dialog or the sign-in first.",
    ja: "開いているダイアログかログインを先に終えてください。",
  },
  "insights.results.title": { en: "Results", ja: "結果" },
  "insights.results.matched": { en: "{count} matched", ja: "{count} 件が一致" },
  "insights.results.count": {
    en: "{shown} of {matched} matched",
    ja: "{matched} 件中 {shown} 件",
  },
  "insights.results.truncated": { en: "Showing {shown}", ja: "{shown} 件まで表示" },
  "insights.results.detailLoading": {
    en: "Loading the fields",
    ja: "項目を読み込み中",
  },
  "insights.evaluationFailed": {
    en: "The query could not be evaluated.",
    ja: "クエリを評価できませんでした。",
  },
  "insights.evaluationFailed.next": {
    en: "Run the query again. The logs are fetched again.",
    ja: "もう一度クエリを実行してください。ログは取得し直します。",
  },
  "insights.results.initial": {
    en: "No results yet. Choose log groups and a time range, write a query, then choose Run query.",
    ja: "まだ結果はありません。ロググループと期間を選び、クエリを書いて「クエリを実行」を押してください。",
  },
  "insights.results.none": {
    en: "No matching events. No events in the selected log groups and time range matched the query.",
    ja: "一致するイベントはありません。選んだロググループと期間に、クエリに一致するイベントはありませんでした。",
  },
  // ---- 261025-insights-stats U1: aggregate rows (BR3.4, BR4.5, BR4.6) ----
  "insights.results.rows": { en: "{count} rows", ja: "{count} 行" },
  // 261025-insights-stats U2:BR3.2: the cut of aggregate rows, in rows.
  "insights.results.rowsOfMatched": {
    en: "{shown} of {matched} rows",
    ja: "{matched} 行中 {shown} 行",
  },
  "insights.results.truncatedRows": {
    en: "Showing {shown} rows",
    ja: "{shown} 行まで表示",
  },
  "insights.results.noRows": {
    en: "The query returned 0 rows. No events in the selected log groups and time range made a group.",
    ja: "結果は 0 行です。選んだロググループと期間に、グループになるイベントはありませんでした。",
  },
  "insights.results.column.timestamp": { en: "@timestamp ({zone})", ja: "@timestamp（{zone}）" },
  "insights.results.column.message": { en: "@message", ja: "@message" },
  "insights.results.expand": { en: "Show details", ja: "詳細を開く" },
  "insights.results.collapse": { en: "Hide details", ja: "詳細を閉じる" },
  "insights.results.field": { en: "Field", ja: "項目" },
  "insights.results.value": { en: "Value", ja: "値" },
  "insights.status.preparing": {
    en: "Fetching logs: finding what is missing",
    ja: "ログを取得中：足りない範囲を確かめています",
  },
  "insights.status.readingCache": {
    en: "Fetching logs: reading the cache",
    ja: "ログを取得中：キャッシュを読み込み中",
  },
  "insights.status.listing": {
    en: "Fetching logs (log group {number} / {groups}): listing streams, {count} selected so far",
    ja: "ログを取得中（ロググループ {number} / {groups}）：ストリームを列挙中、これまでに {count} 本",
  },
  "insights.status.fetching": {
    en: "Fetching logs (log group {number} / {groups}): {done} / {total} streams, {events} events",
    ja: "ログを取得中（ロググループ {number} / {groups}）：ストリーム {done} / {total}、{events} 件",
  },
  "insights.status.failedGroups": {
    en: "{count} log groups could not be fetched.",
    ja: "{count} 個のロググループを取得できませんでした。",
  },
  "insights.results.previous": {
    en: "Results of the previous run",
    ja: "前回の実行の結果",
  },
  "insights.fetchFailed.groups": {
    en: "Log groups that could not be fetched:",
    ja: "取得できなかったロググループ：",
  },
  "insights.status.running": {
    en: "Running query: {scanned} / {total} events",
    ja: "クエリを実行中：{scanned} / {total} 件",
  },
  "insights.partial": {
    en: "Results are based on part of the logs. {count} streams could not be fetched.",
    ja: "結果は一部のログだけから作られています。{count} 本のストリームを取得できませんでした。",
  },
  "insights.partial.listing": {
    en: "Results are based on part of the logs. The stream listing is incomplete.",
    ja: "結果は一部のログだけから作られています。ストリームの列挙は途中までです。",
  },
  "insights.partial.show": { en: "Show failed streams", ja: "失敗したストリームを見る" },
  "insights.cancelled": { en: "Query cancelled.", ja: "クエリを中止しました。" },
  "insights.notRun": { en: "The query was not run.", ja: "クエリは実行していません。" },
  "insights.error.at": {
    en: "Line {line}, column {column}: ",
    ja: "{line} 行 {column} 列：",
  },
  // 261025-insights-stats U2:BR4.2: the lists come from the core's QueryCatalog.
  "insights.error.supportedList": {
    en: "Supported commands: {names}.",
    ja: "使える命令：{names}",
  },
  "insights.error.functionList": {
    en: "Supported functions: {names}.",
    ja: "使える関数：{names}",
  },
  "insights.error.listSeparator": { en: ", ", ja: "、" },
  "query.syntax.noCommand": {
    en: "The query has no command. Start it with fields, filter, sort or limit.",
    ja: "クエリに命令がありません。fields、filter、sort、limit のどれかで始めてください。",
  },
  "query.syntax.emptyCommand": {
    en: 'There is no command before or after "|".',
    ja: "「|」の前か後ろに命令がありません。",
  },
  "query.syntax.unknownCommand": {
    en: '"{word}" is not a command.',
    ja: "「{word}」は命令ではありません。",
  },
  "query.syntax.missingCondition": {
    en: "filter needs a condition.",
    ja: "filter の後ろに条件がありません。",
  },
  "query.syntax.unclosedParen": {
    en: "The parenthesis is not closed.",
    ja: "括弧が閉じていません。",
  },
  "query.syntax.unmatchedParen": {
    en: "This parenthesis closes nothing.",
    ja: "この閉じ括弧に対応する開き括弧がありません。",
  },
  "query.syntax.unclosedString": {
    en: "The quotation is not closed.",
    ja: "引用符が閉じていません。",
  },
  "query.syntax.unclosedRegex": {
    en: "The regular expression is not closed with /.",
    ja: "正規表現が / で閉じていません。",
  },
  "query.syntax.unclosedList": {
    en: "The list is not closed with ].",
    ja: "値の並びが ] で閉じていません。",
  },
  "query.syntax.unclosedName": {
    en: "The name in backquotes is not closed.",
    ja: "` で囲んだ名前が閉じていません。",
  },
  "query.syntax.emptyList": {
    en: "The list needs at least one value.",
    ja: "値の並びに値が 1 つもありません。",
  },
  "query.syntax.unexpectedToken": {
    en: '"{word}" cannot be written here.',
    ja: "ここに「{word}」は書けません。",
  },
  "query.syntax.unexpectedEnd": {
    en: "The query ends in the middle of a condition.",
    ja: "条件の途中でクエリが終わっています。",
  },
  "query.syntax.expectedOperator": {
    en: "An operator such as =, like or in is needed here.",
    ja: "ここには =、like、in などの演算子が要ります。",
  },
  "query.syntax.expectedValue": {
    en: "A value is needed here.",
    ja: "ここには値が要ります。",
  },
  "query.syntax.expectedList": {
    en: 'in needs a list such as ["a", "b"].',
    ja: 'in の後ろには ["a", "b"] のような値の並びが要ります。',
  },
  "query.syntax.unexpectedCharacter": {
    en: '"{word}" cannot be used in a query.',
    ja: "「{word}」はクエリに使えません。",
  },
  "query.syntax.invalidNumber": {
    en: "This cannot be read as a number.",
    ja: "数値として読めません。",
  },
  "query.syntax.invalidRegex": {
    en: "This regular expression cannot be used.",
    ja: "この正規表現は使えません。",
  },
  "query.syntax.regexNotAllowed": {
    en: "A regular expression can only follow like or =~.",
    ja: "正規表現は like と =~ の後ろにだけ書けます。",
  },
  "query.syntax.matchNeedsRegex": {
    en: "=~ needs a regular expression such as /error/.",
    ja: "=~ の後ろには /error/ のような正規表現が要ります。",
  },
  "query.syntax.likeNeedsText": {
    en: "like needs a string or a regular expression.",
    ja: "like の後ろには文字列か正規表現が要ります。",
  },
  "query.syntax.invalidDateTime": {
    en: 'Compare @timestamp with milliseconds, "YYYY-MM-DD HH:MM:SS[.sss]" (the selected time zone) or "YYYY-MM-DDTHH:MM:SS[.sss]Z" (UTC).',
    ja: "@timestamp は、ミリ秒の数値か、「YYYY-MM-DD HH:MM:SS[.sss]」（選んでいるタイムゾーン）か「YYYY-MM-DDTHH:MM:SS[.sss]Z」（UTC）と比べてください。",
  },
  "query.syntax.nonexistentDateTime": {
    en: "This time does not exist in the selected time zone (daylight saving time).",
    ja: "選んでいるタイムゾーンでは、夏時間の切り替えのためこの時刻はありません。",
  },
  "query.unsupported.command": {
    en: '"{word}" is not supported in this version.',
    ja: "「{word}」にはこの版では対応していません。",
  },
  "query.unsupported.field": {
    en: 'The field "{word}" cannot be used in this version. Use @timestamp, @message, @logStream, @log or a JSON field.',
    ja: "項目「{word}」はこの版では使えません。@timestamp、@message、@logStream、@log か JSON の項目を使ってください。",
  },
  "query.unsupported.function": {
    en: 'The function "{word}" is not supported here in this version.',
    ja: "関数「{word}」はこの版ではここに使えません。",
  },
  "query.unsupported.toMillisArgument": {
    en: "toMillis can only take @timestamp.",
    ja: "toMillis に渡せるのは @timestamp だけです。",
  },
  "query.unsupported.timestampOperator": {
    en: '"{word}" cannot be used with @timestamp. Compare it with =, <, > or in.',
    ja: "@timestamp には「{word}」を使えません。=、<、>、in で比べてください。",
  },
  "query.unsupported.operator": {
    en: 'The operator "{word}" is not supported in this version.',
    ja: "演算子「{word}」にはこの版では対応していません。",
  },
  "query.unsupported.fieldComparison": {
    en: 'A field such as "{word}" cannot be written after like, =~ or in. Write a string, a number or a /regex/.',
    ja: "like、=~、in の後ろには「{word}」のような項目を書けません。文字列、数値、/正規表現/ を書いてください。",
  },
  "query.syntax.notNeedsParen": {
    en: "Put the condition after not in parentheses, such as not (@message like /x/). Use not like, not in or != after a field.",
    ja: "not の後ろの条件は not (@message like /x/) のように括弧で囲んでください。項目の後ろには not like、not in、!= を使えます。",
  },
  "query.syntax.missingFields": {
    en: "{word} needs at least one field.",
    ja: "{word} の後ろに項目がありません。",
  },
  "query.syntax.expectedField": {
    en: "A field name is needed here.",
    ja: "ここには項目の名前が要ります。",
  },
  "query.syntax.invalidSortDirection": {
    en: 'Write asc or desc after a sort field, not "{word}".',
    ja: "sort の項目の後ろには「{word}」ではなく asc か desc を書いてください。",
  },
  "query.syntax.missingLimit": {
    en: "limit needs a number of rows.",
    ja: "limit の後ろに件数がありません。",
  },
  "query.syntax.invalidLimit": {
    en: "limit takes a whole number from 0 to 100,000.",
    ja: "limit には 0 から 100,000 までの整数を書いてください。",
  },
  "query.syntax.limitTooLarge": {
    en: "limit can be at most 100,000.",
    ja: "limit は 100,000 までです。",
  },
  "query.syntax.nestingTooDeep": {
    en: "Parentheses are nested too deeply. Nest them 100 levels deep at most.",
    ja: "括弧の入れ子が深すぎます。入れ子は 100 段までにしてください。",
  },
  "query.unsupported.arrayElement": {
    en: 'Array elements such as "{word}" cannot be used in this version. A key made of digits can be written in backquotes.',
    ja: "「{word}」のような配列の要素にはこの版では対応していません。数字だけのキーは ` で囲めば使えます。",
  },
  "query.unsupported.alias": {
    en: 'Naming a field with "{word}" is not supported in this version.',
    ja: "「{word}」で項目に名前を付けることにはこの版では対応していません。",
  },
  "query.unsupported.limitAny": {
    en: 'limit "{word}" is not supported in this version.',
    ja: "limit の「{word}」にはこの版では対応していません。",
  },
  // ---- 261025-insights-stats U1: reading stats (BR1.2-BR1.9, BR4.6) ----
  "query.syntax.missingAggregate": {
    en: "stats needs at least one function such as count(*).",
    ja: "stats の後ろに count(*) のような関数がありません。",
  },
  "query.syntax.expectedAggregate": {
    en: "A function such as count(*) or avg(field) is needed here.",
    ja: "ここには count(*) や avg(項目) のような関数が要ります。",
  },
  "query.syntax.missingGroupKey": {
    en: "by needs at least one field or bin().",
    ja: "by の後ろに項目か bin() がありません。",
  },
  "query.syntax.trailingComma": {
    en: 'There is nothing after ",".',
    ja: "「,」の後ろに何もありません。",
  },
  "query.syntax.argumentCount": {
    en: "The function has the wrong number of arguments. It takes one field.",
    ja: "関数の引数の数が違います。項目を 1 つ書いてください。",
  },
  "query.syntax.aggregateStar": {
    en: "Only count can take *. Write a field here.",
    ja: "* を書けるのは count だけです。ここには項目を書いてください。",
  },
  "query.syntax.expectedAlias": {
    en: "as needs a column name.",
    ja: "as の後ろに列の名前がありません。",
  },
  "query.syntax.duplicateColumn": {
    en: 'The column "{word}" is written twice. Give one of them another name with as.',
    ja: "列「{word}」が 2 つあります。どちらかに as で別の名前を付けてください。",
  },
  "query.syntax.invalidBinPeriod": {
    en: "bin() needs a period such as 5m: a whole number from 1 to 1000000 and s, m, h or d.",
    ja: "bin() には 5m のような期間が要ります。1 から 1000000 までの整数と s・m・h・d のどれかです。",
  },
  "query.unsupported.binUnit": {
    en: 'The period "{word}" is not supported in this version. Use s, m, h or d.',
    ja: "期間「{word}」にはこの版では対応していません。s・m・h・d を使ってください。",
  },
  "query.unsupported.aggregateFunction": {
    en: 'The stats function "{word}" is not supported in this version.',
    ja: "stats の関数「{word}」にはこの版では対応していません。",
  },
  // ---- 261025-insights-stats U2: pct and the commands after stats (BR1.2-BR1.5) ----
  "query.syntax.pctOutOfRange": {
    en: 'The percentile of pct must be a number from 0 to 100 (up to 15 decimal places), not "{word}".',
    ja: "pct の百分位には「{word}」ではなく 0 から 100 までの数（小数は 15 桁まで）を書いてください。",
  },
  "query.syntax.unknownResultColumn": {
    en: 'After stats, "{word}" is not a column of its result. Write a column name as stats shows it, in backquotes when it has signs, such as `count(*)`.',
    ja: "stats の後ろでは「{word}」は結果の列ではありません。列の名前を stats の結果のとおりに書き、`count(*)` のように記号を含む名前は ` で囲んでください。",
  },
  "query.unsupported.fieldsAfterStats": {
    en: '"{word}" after stats is not supported in this version. Use filter, sort or limit after stats.',
    ja: "stats の後ろの「{word}」にはこの版では対応していません。stats の後ろには filter・sort・limit を使えます。",
  },
  "query.unsupported.secondStats": {
    en: 'A second "{word}" is not supported in this version. Write one stats.',
    ja: "2 つ目の「{word}」にはこの版では対応していません。stats は 1 つにしてください。",
  },
  // ---- 261022-small-features U2: the export (BR4.1, BR4.9, review R-02) ----
  "export.menu": { en: "Export", ja: "エクスポート" },
  "export.exporting": { en: "Exporting…", ja: "エクスポート中…" },
  "export.item.csvFile": { en: "Save as CSV", ja: "CSV で保存" },
  "export.item.markdownFile": { en: "Save as Markdown", ja: "Markdown で保存" },
  "export.item.csvClipboard": { en: "Copy as CSV", ja: "CSV をコピー" },
  "export.item.markdownClipboard": { en: "Copy as Markdown", ja: "Markdown をコピー" },
  "export.notice.saved": {
    en: "Saved {count} rows to {path}.",
    ja: "{count} 行を {path} に保存しました。",
  },
  "export.notice.copied": {
    en: "Copied {count} rows to the clipboard.",
    ja: "{count} 行をクリップボードにコピーしました。",
  },
  "export.notice.saveFailed": {
    en: "The file could not be saved. {detail}",
    ja: "ファイルを保存できませんでした。{detail}",
  },
  "export.notice.copyFailed": {
    en: "The text could not be copied to the clipboard. {detail}",
    ja: "クリップボードにコピーできませんでした。{detail}",
  },
  "export.notice.success": { en: "Success", ja: "成功" },
  "export.notice.failure": { en: "Error", ja: "エラー" },
  "export.notice.dismiss": { en: "Close", ja: "閉じる" },
  "export.failure.PermissionDenied": {
    en: "You do not have permission to write {path}.",
    ja: "{path} に書き込む権限がありません。",
  },
  "export.failure.NoSpace": {
    en: "There is no space left to write {path}.",
    ja: "{path} を書き込む空き容量がありません。",
  },
  "export.failure.NotFound": {
    en: "The folder of {path} was not found.",
    ja: "{path} のフォルダが見つかりません。",
  },
  "export.failure.Other": {
    en: "{path} could not be written.",
    ja: "{path} に書き込めませんでした。",
  },
  "export.failure.Clipboard": {
    en: "The clipboard did not accept the text.",
    ja: "クリップボードが受け付けませんでした。",
  },
  "export.failure.Dialog": {
    en: "The save dialog could not be used.",
    ja: "保存ダイアログを使えませんでした。",
  },
  "session.exportUnavailable": {
    en: "Nothing can be exported right now.",
    ja: "今はエクスポートできません。",
  },
  "session.exportBusy": {
    en: "An export is already in progress.",
    ja: "エクスポートが進行中です。",
  },
  "expanded.mode": { en: "Display", ja: "表示" },
  "expanded.mode.pretty": { en: "Formatted", ja: "整形" },
  "expanded.mode.raw": { en: "Original", ja: "元のまま" },
} as const satisfies Record<string, Record<Locale, string>>;

export type MessageKey = keyof typeof messages;

export type Translate = (key: string, params?: Record<string, string | number>) => string;

/** The singular form of an English text and the name of the count that decides it. */
export interface SingularForm {
  /** The placeholder whose value decides the form: the singular when it is exactly 1. */
  count: string;
  /** The text with that count being 1, with the same placeholders as the plural. */
  text: string;
}

/**
 * 261020-carryover-fixes FR4.1: the English singular of every text whose
 * nouns (and verbs) follow a number of streams. The plural is the text of
 * `messages`; only the stream counts are covered (A8).
 */
export const englishSingular: Partial<Record<MessageKey, SingularForm>> = {
  "status.listing": {
    count: "count",
    text: "Listing streams… {count} stream selected so far",
  },
  "status.fetching": {
    count: "planned",
    text: "Fetching… {finished}/{planned} stream, {count} events so far",
  },
  "status.failedStreams": { count: "count", text: "Failed in {count} stream" },
  "cancel.notice.afterListing": {
    count: "n",
    text: "Fetch cancelled. {n} stream was not finished, so some logs are missing.",
  },
  "insights.status.fetching": {
    count: "total",
    text: "Fetching logs (log group {number} / {groups}): {done} / {total} stream, {events} events",
  },
  "insights.partial": {
    count: "count",
    text: "Results are based on part of the logs. {count} stream could not be fetched.",
  },
};

/** The template of `key` in `locale`: the English singular when its count is exactly 1. */
function templateOf(locale: Locale, key: MessageKey, params: Record<string, string | number>) {
  const singular = locale === "en" ? englishSingular[key] : undefined;
  if (singular !== undefined && singular.count in params) {
    if (Number(params[singular.count]) === 1) {
      return singular.text;
    }
  }
  return messages[key][locale];
}

/** The message key naming a time zone choice (U4:BR3.2). */
export function timeZoneLabelKey(timeZone: TimeZoneChoice): MessageKey {
  return timeZone === "Local" ? "timeZone.local" : "timeZone.utc";
}

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
 * The locale the whole screen shows (261022-small-features U1:BR1.2): the
 * saved preference when it names a language, else the OS language as
 * `detectLocale` reads it. Pure; `languages` is the OS/browser list.
 */
export function resolveLocale(
  preference: LanguagePreference,
  languages: readonly string[],
): Locale {
  return preference === "system" ? detectLocale(languages) : preference;
}

/**
 * Looks up `key` and fills `{name}` placeholders. An unknown key is returned
 * as is, so a missing entry is visible instead of hidden. An English text
 * with a singular form uses it when its deciding count is 1 (FR4.1.5).
 */
export function translate(
  locale: Locale,
  key: string,
  params: Record<string, string | number> = {},
): string {
  const template = isMessageKey(key) ? templateOf(locale, key, params) : key;
  return template.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    name in params ? String(params[name]) : placeholder,
  );
}

/** A translator bound to one locale. */
export function createTranslator(locale: Locale): Translate {
  return (key, params) => translate(locale, key, params);
}
