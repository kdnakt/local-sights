# API の一覧（Local Sights）

走査したコミット：`2760b13b8bc5b6daba36a6c283b3958c974a5a2d`（2026-10-17）。HTTP のサーバーは無い。API に当たるのは (1) 画面とアプリの間の Tauri のコマンドとイベント、(2) AWS への外向きの呼び出し、(3) 中核ライブラリの公開の型と関数、(4) AWS CLI の起動である。

## Tauri のコマンド（画面 → アプリ）

定義は `src-tauri/src/lib.rs`、画面側の包みは `src/api.ts`、許可は `src-tauri/capabilities/default.json`（`core:event:default` と 26 個の `allow-*`）。引数名は Tauri の慣例で画面からは camelCase（`timeZone` など）。状態を変えるコマンドは成功しても何も返さず、新しい状態は `session-changed` で届く（遅れた応答が新しい状態を上書きしないため）。失敗は `CommandError { key }`（文言のキー、`SessionError::message_key()`）で返る。接続に依存する操作は `generation`（接続の世代）を持ち、古い世代の操作は答えずに捨てる（U3:BR6.7）。

| コマンド | 引数 | 戻り値 | 内容 |
|----------|------|--------|------|
| `get_session` | なし | `SessionView` | 今の画面の状態 |
| `update_input` | `field: "startText" \| "endText"`, `value` | `SessionView` か `CommandError` | 開始・終了の入力 |
| `start_fetch` | `generation` | なし | 取得を始める（非同期タスク） |
| `cancel_fetch` | なし | なし | [Cancel]。押せないときは何もしない |
| `get_rows` | `offset`, `limit`（1,000 まで） | `DisplayRowWindow` | 見えている範囲の行。絞り込み中は結果の中で数える |
| `row_positions` | `keys: RowKey[]`（10,002 まで） | `RowPositions` か `CommandError`（`rows.tooManyKeys`） | 開いた行・選んだ行・先頭行の今の位置と各版 |
| `set_log_filter` | `text` | なし／`CommandError` | 絞り込みの文字列。変われば別スレッドで走査を始める |
| `select_time_zone` | `timeZone: "Local" \| "Utc"` | なし | タイムゾーンの切り替え（いつでも可） |
| `set_failure_list_open` | `open` | なし／`CommandError` | 失敗一覧の開閉 |
| `select_profile` | `profile: ProfileSelector` | なし／`CommandError` | プロファイル（表示中のログがあれば確認待ち） |
| `select_region` | `region` | なし／`CommandError` | リージョン（同上） |
| `confirm_connection_change`、`cancel_connection_change` | なし | なし／`CommandError` | 接続の変更の確認への答え |
| `reload_log_groups` | `generation` | なし／`CommandError` | ロググループの一覧を読み直す |
| `update_log_group_filter` | `text` | なし／`CommandError` | ロググループ名の絞り込み（手元） |
| `select_log_group` | `name`, `generation` | なし／`CommandError` | ロググループを 1 つ選ぶ |
| `open_settings`、`cancel_settings` | なし | なし／`CommandError` | 設定ダイアログ |
| `save_settings` | `enabled` | なし／`CommandError` | キャッシュの有効・無効（非同期、ファイル作業は別スレッド） |
| `clear_cache` | なし | なし／`CommandError` | キャッシュの全消去（同上） |
| `cancel_close`、`confirm_close` | なし | なし | 閉じる確認への答え |
| `start_sso_login`、`cancel_sso_login`、`dismiss_sso_login` | なし | なし／`CommandError` | AWS CLI による SSO ログイン |
| `open_sso_url` | `url`（その試行の許可された URL だけ） | なし／`CommandError` | 既定のブラウザで開く |

## Tauri のイベント（アプリ → 画面）

| イベント | 中身 | いつ |
|----------|------|------|
| `session-changed` | `SessionView`（全体） | 状態が変わるたび。取得の開始・計画・各ストリームの終わり・取得の終わりなど |
| `fetch-progress` | `FetchProgressUpdate { jobId, progress, eventCount, timelineVersion, filterSummary, filterResultVersion, cacheSaving }` | 取得中、一覧の各ページ・追加した各ページ・キャッシュへの保存の開始 |
| `filter-progress` | `FilterProgressUpdate { filterSummary, filterResultVersion }` | 絞り込みの走査中（100 ms に 1 回まで）と Ready になったとき |

画面は `mergeSessionView`・`withProgress`・`withFilter`（`api.ts`）で、版が古い更新を捨てながら手元の写しに重ねる。

## 主なデータの形（JSON）

```text
LogEvent      { timestamp: number(ms), ingestionTime: number|null, message: string,
                logStreamName: string, sequence: number }
DisplayRow    = LogEvent + { displayTime: "yyyy/mm/dd hh:mm:ss.SSS"（選んだゾーン） }
DisplayRowWindow { offset, rows: DisplayRow[], totalCount, timelineVersion,
                   filtered: boolean, allCount, resultVersion: number|null }
RowKey        { logStreamName, sequence }
RowPositions  { positions: (number|null)[], timelineVersion, resultVersion,
                discardGeneration, totalCount }
FilterSummary { filterId, matchedCount, allCount, status: "Filtering"|"Ready", resultVersion }
SessionView   { sessionId, phase: "Idle"|"Fetching"|"Done"|"Failed", timeZone,
                startInput, endInput, validationErrors, canFetch, currentJobId, eventCount,
                lastJob, profiles, regions, catalogNotices, connection, pendingChange,
                logGroups, logGroupFilter, selectedLogGroupName, canChangeConnection,
                canReload, timelineVersion, connectionGeneration, progress, failedStreams,
                listingStatus, listingFailure, failureListOpen, logFilter, filterSummary,
                filterResultVersion, cacheEnabled, cacheDirectory, settingsDialog,
                settingsNotice, canOpenSettings, cacheSaving, cacheNotices,
                discardGeneration, closeConfirmation, ssoLogin, elapsed, cancelling,
                fetchCancelAvailable, cancelNotice }
```

- `LogEvent` はロググループ名を持たない（取得が 1 ロググループに限られるため）。`message` は API が返した生の文字列で、JSON として読まれることはない。
- `DisplayRow` は `LogEvent` を `#[serde(flatten)]` で平らにして `displayTime` を足したもの（`session.rs`）。
- 絞り込み中の `totalCount` は一致件数、`allCount` は保持件数。

## AWS への外向きの呼び出し

`crates/local-sights-core/src/gateway/mod.rs` の `CloudWatchLogsGateway` trait が唯一の口で、実装は `gateway/aws.rs`（AWS SDK for Rust）。ほかの API を呼ぶ口はわざと無い（project.md Forbidden）。

| 操作 | 渡すもの | 受け取るもの | 備考 |
|------|----------|--------------|------|
| `GetLogEvents` | プロファイル、リージョン、ロググループ、ストリーム、`startTime`（含む）、`endTime`（含まない）、`startFromHead: true`、次のトークン | イベント（時刻・取り込み時刻・メッセージ）、`nextForwardToken` | 送ったトークンと同じものが返ったら終わり（`paging.rs`）。SDK 自身の再試行は切り、アプリの `RetryPolicy` と `CallPacer` で制御 |
| `DescribeLogStreams` | ロググループ、次のトークン | ストリーム（名前・最初と最後のイベント時刻など） | 常に `orderBy=LastEventTime`、`descending=true`。開始の 1 時間前より古いストリームが出たら打ち切る |
| `DescribeLogGroups` | 次のトークンだけ | ロググループ名 | 全ページ読み、名前の絞り込みは手元 |

認証は SDK の既定の連鎖に任せ、アプリは認証情報を読まない・持たない・書かない。クライアントは（プロファイル, リージョン）ごとに使い回し、SSO ログインの成功時に捨てる。失敗は `ApiFailure`（種類 8 つと、許可リストの項目だけで作る安全な詳細）に変える。

## 中核ライブラリの公開 API（主なもの）

IB-11 に関わるものを中心に挙げる。すべて `local_sights_core::…`。

| 型・関数 | 場所 | 内容 |
|----------|------|------|
| `LogEvent`, `compare_events` | `event` | イベントと並びキー |
| `EventTimeline::{append, rows, position_of, timestamp_of, clear, events, version}` | `timeline` | 時刻順の保持 |
| `RowWindow` | `timeline` | 行の窓（絞り込みの有無と各版を含む） |
| `FilterEngine::{set_text, scan_chunk, on_added, on_discarded, matched_key, position_of, summary, state, ticket}` | `filter` | 絞り込みの状態と結果 |
| `FilterCondition::{new, matches, is_active}`, `normalize_filter_text`, `should_report_progress` | `filter` | 条件と判定、進み具合の知らせ方 |
| `FilterChange`, `ScanStep`, `ScanTicket`, `FilterStatus`, `FilterSummary`, `FilterState` | `filter` | 走査の券と結果 |
| `LogView::{append, clear, set_filter, scan_chunk, rows, position_of, positions_of, checked_positions_of, filter_state}` | `log_view` | タイムラインと絞り込みを 1 つのロックで |
| `impl TimelineStore for Mutex<LogView>`, `impl HeldEvents for Mutex<LogView>` | `log_view` | 取得とキャッシュ書き込みからの入口 |
| `run_fetch`, `run_fetch_with_cache`, `decide_job_status` | `coordinator` | 取得の流れ |
| `FetchSink`, `TimelineStore`, `HeldEvents`, `FetchJob`, `BatchProgress` | `coordinator` | 受け口と保持先の trait、ジョブ |
| `FetchInput`, `FetchRequest`, `ValidatedFetch`, `validate_fetch_input`, `validate_session_input` | `request` | 取得条件と検証 |
| `TimeRange::{from_seconds, start_instant, end_instant, api_end_time}` | `time_range` | 取得範囲 |
| `AppSession::{begin_fetch, update_log_filter, set_filter_state, display_rows, finish_fetch, view, …}` | `session` | 画面の状態 |
| `LogCache::{lookup, write, clear_all}`, `CacheKey`, `CoveredRange`, `merge_ranges`, `find_covering_range`, `record_range`, `resequence` | `cache` | キャッシュ |
| `CloudWatchLogsGateway` と要求・応答の型 | `gateway` | AWS の境界 |

## AWS CLI の起動（SSO ログイン）

`src-tauri/src/sso_cli/`。利用者が [Sign in with SSO] を押したときだけ、`aws sso login [--profile <名前>]` をシェルを通さずに起動する。場所は Unix が `/opt/homebrew/bin/aws`・`/usr/local/bin/aws`、Windows が `Program Files` の公式インストーラの場所、次に `PATH` の各フォルダの `aws.exe`。出力からは許可した `https://` の URL と確認コードだけを取り出す。5 分で時間切れ。アプリ自身は SSO の API を呼ばず、トークンも読み書きしない（project.md Corrections）。
