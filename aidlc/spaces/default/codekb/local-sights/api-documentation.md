# API の一覧（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直し。公開サーバーの API はなく、ここに挙げるのは (1) 画面とアプリの間の Tauri の IPC、(2) 中核ライブラリの公開 API、(3) 外へ向かう呼び出しである。JSON の項目名はすべて `camelCase`（`serde(rename_all = "camelCase")`）。

## Tauri のコマンド（画面 → アプリ）

`src-tauri/src/lib.rs` の `#[tauri::command]`（行は定義の位置）。`src/api.ts` の同名の関数が `invoke` する。3 か所の登録（`generate_handler!`、`build.rs`、`capabilities/default.json`）は常に同じ 32 個。戻りが「なし」のコマンドは結果を `session-changed` で届ける（古い応答が新しい状態を上書きしないため）。エラーは `CommandError { key, … }`（中核のメッセージ鍵）。

| コマンド | 引数 | 戻り | 役割 | lib.rs |
|---|---|---|---|---|
| `get_session` | なし | `SessionView` | 今の画面の状態をまるごと | 384 |
| `update_input` | `field: InputField ("startText" / "endText")`, `value` | `SessionView` | 時刻の入力。取得中は拒否 | 390 |
| `get_rows` | `offset`, `limit`（最大 1,000） | `DisplayRowWindow` | 見えている行だけを読む | 406 |
| `select_time_zone` | `timeZone: "Local" / "Utc"` | なし | 表示のゾーンを変える | 421 |
| `row_positions` | `keys: RowKey[]`（最大 10,002）, 版 | `RowPositions` | 開いた行の今の位置 | 439 |
| `cancel_close` / `confirm_close` | なし | なし | 閉じる確認の答え | 457 / 471 |
| `set_log_filter` | `text` | なし | 絞り込みの文字。走査を始める | 639 |
| `set_failure_list_open` | `open` | なし | 失敗したストリームの表の開閉 | 724 |
| `start_fetch` | `connectionGeneration` | なし | 取得を始める（検証は中核） | 743 |
| `cancel_fetch` | なし | なし | 取得の中断（`StopReason::Cancel`） | 848 |
| `update_analytics_input` | `field: AnalyticsField`, `value: AnalyticsValue` | なし | クエリの画面の入力（期間・ロググループ・クエリ文） | 885 |
| `run_query` | なし | なし（読みの問題は `CommandError`） | クエリを読んで実行を始める | 921 |
| `cancel_query` | なし | なし | 実行の中断 | 1066 |
| `get_query_rows` | `runId`, `holdingsEpoch`, `offset`, `limit` | `QueryRowWindow` | 結果の行の窓。`holdingsEpoch` は送り返すだけ | 1098 |
| `get_query_row_detail` | `runId`, `holdingsEpoch`, `position` | `QueryRowDetail` | 1 行の項目と値 | 1127 |
| `get_completion_candidates` | なし | `CompletionCandidate[]` | エディタの補完の候補（固定の項目＋JSON の項目名） | 1152 |
| `open_settings` / `cancel_settings` | なし | なし | 設定ダイアログの開閉 | 1179 / 1185 |
| `save_settings` | `cacheEnabled` | なし | 設定の保存（`settings.json`） | 1197 |
| `clear_cache` | なし | なし | キャッシュのフォルダを空にする | 1208 |
| `select_profile` | `profile: ProfileSelector` | なし | プロファイルの選択。表示中のログがあれば確認待ち | 1387 |
| `select_region` | `region` | なし | リージョンの選択（同上） | 1397 |
| `confirm_connection_change` / `cancel_connection_change` | なし | なし | 接続の変更の確認の答え | 1407 / 1416 |
| `reload_log_groups` | なし | なし | 一覧の再読み込み | 1429 |
| `update_log_group_filter` | `text` | なし | 一覧の絞り | 1445 |
| `select_log_group` | `name` | なし | ロググループの選択 | 1459 |
| `start_sso_login` | なし | なし | `aws sso login` の起動 | 1476 |
| `cancel_sso_login` / `dismiss_sso_login` | なし | なし | ログインの中断／ダイアログを閉じる | 1494 / 1510 |
| `open_sso_url` | なし | なし | CLI が出した URL を既定ブラウザで開く（ホストの許可一覧） | 1530 |

## Tauri のイベント（アプリ → 画面）

| イベント | 中身 | いつ |
|---|---|---|
| `session-changed` | `SessionView` 全体 | 状態が変わるたび（取得の始めと終わり、接続、ダイアログ、クエリの進み具合 250 ms ごと） |
| `fetch-progress` | `FetchProgressUpdate`（jobId、progress の件数、eventCount、timelineVersion、filterSummary、cacheSaving） | 取得中にページが届くたび。画面は `withProgress` で合成 |
| `filter-progress` | `FilterProgressUpdate` | 絞り込みの走査中 100 ms ごとと終わり。画面は `withFilter` で合成 |

定数は `src-tauri/src/lib.rs:198-205` と `src/api.ts:553-557`。

## 主なデータの形（JSON）

- `SessionView`（`session.rs:661`）：`phase`（Idle / Fetching / Done / CompletedWithFailures / Failed / Cancelled …）、`connection`、`connectionGeneration`、`profiles`、`regions: string[]`（リージョンのコードだけ、`session.rs:1976`）、`timeZone`、`startText` / `endText`、`validationErrors`、`listing`（ロググループの一覧）、`eventCount`、`timelineVersion`、`filter`、`failedStreams`、`elapsed: ElapsedView | null`、`cancelNotice`、`cacheSaving` / `cacheNotices`、`settingsDialog`、`closeConfirmation`、`ssoLogin`、`insights: InsightsView`。
- `ElapsedView`（`session.rs:599`）：`running`、`elapsedMs`、`fetchNumber`。画面は同じ `fetchNumber` の running の値だけを保持して時計を進める（`src/elapsed.ts`）。
- `DisplayRow` / `DisplayRowWindow`（`session.rs:768-781`）：表示用の時刻文字、ストリーム名、メッセージ、`sequence`。
- `InsightsView`（`insights_session.rs:332`）：`phase`（AnalyticsPhase）、`selectedLogGroups`（最大 50）、`queryText`（最大 10,000 文字）、期間と検証の結果、`progress`、`result: ResultSummary | null`、`groupFailures`、`failedStreams`。
- `ResultSummary`（`insights_session.rs:301`）：`runId`、`holdingsEpoch`、`columns`、`matchedCount`、`truncatedAt`、`evaluatedCount`。
- `QueryRow` / `QueryRowWindow` / `QueryRowDetail`（`insights_session.rs:389-429`）：セルの文字（真偽値は `1` / `0`）、`runId` と `holdingsEpoch` を送り返す。
- `ApiFailure`（`failure.rs`）：`kind`（Throttling / Auth / Permission / NotFound / Network / Timeout / Server / Other）、`api`、`safeDetail`、`requestId`。
- `ProfileSelector`：`{ kind: "SdkDefault" }` か `{ kind: "Named", profileName }`。

## 中核ライブラリの公開 API（主なもの）

`crates/local-sights-core` の 27 モジュールはすべて `pub`。アプリと例・テストが使う入口を挙げる（細かな型は `code-structure.md` の表と各モジュールの doc）。

- `coordinator::run_fetch(gateway, validated, store, sink, stop, …)` と `run_fetch_with_cache(…, cache_plan, …)`：取得の流れ。`FetchSink`（`on_started` / `on_listing_progress` / `on_planned` / `on_batch` / `on_stream_finished` / `on_saving` / `on_summary` / `on_finished`）、`TimelineStore`（`discard` / `add`）、`HeldEvents::copy_events_in_range`、`decide_job_status`。
- `query::runner::run_query(gateway, run: QueryRun, store: &QueryStore, cache, sink: impl QuerySink, …) -> QueryOutcome`：クエリの流れ。`QuerySink: FetchSink + Send` に `on_reading_cache` / `on_group_started` / `on_range_fetched` / `on_evaluation_progress` が足される。
- `query::parser::parse_query(text) -> Result<QueryPlan, QueryProblem>`、`query::instant::resolve_date_texts(plan, zone)`、`query::evaluator::{evaluate, Evaluation, QueryResult, row_cells, row_detail}`、`query::fields::{EventFields, collect_json_field_names}`、`query::natural::compare_sort_values`。
- `query::store::QueryStore`：`begin_run`、`discard_all`、`epoch`、`missing_ranges`、`add_cached`、`settle_range`、`build_view`、`evaluate_step`、`json_field_names`、`copy_rows`。
- `session::AppSession`：`new` / `with_catalog` / `with_cache_location` / `with_clock`、入力と選択（`update_input`、`select_profile`、`select_region`、`select_log_group`、`select_time_zone`）、取得（`begin_fetch`、`on_*`、`finish_fetch`、`request_cancel`、`abort_fetch_with_failure`）、絞り込み（`update_log_filter`、`set_filter_state`）、閉じる（`request_close`、`confirm_close`）、設定（`open_settings`、`begin_save_settings`、`begin_clear_cache`、`finish_settings_work`）、SSO（`start_sso_login`、`on_sso_login_hints`、`finish_sso_login`）、クエリの転送（`update_analytics_input`、`begin_query`、`on_query_*`、`finish_query_evaluation`、`display_query_rows`、`display_query_row_detail`）、`view() -> SessionView`、`display_rows`。
- `insights_session::InsightsSession`：`update`、`blocker`、`snapshot`、`begin`、`on_*`、`finish_fetch`、`finish_evaluation`、`request_cancel`、`on_connection_changed`、`display_rows`、`display_row_detail`、`completion_candidates`。
- `gateway::CloudWatchLogsGateway`（trait、`Send + Sync`）：`describe_log_groups`、`describe_log_streams`、`get_log_events` の 3 操作。`gateway::aws::AwsCloudWatchLogsGateway` が実装。
- `cache::LogCache`：`new` / `with_writes`、`lookup`、`read_overlaps`、`write`、`remove_entry`、`clear_all`、`read_header`；`cache::plan`：鍵と合流の計画；`cache::settings`：設定ファイル。
- `log_view::LogView`：`append`、`clear`、`set_filter`、`scan_chunk`、`rows`、`positions_of` / `checked_positions_of`、`filter_state`。
- `catalog::ConnectionCatalog`：`profiles`、`regions`、`find`、`has_region`、`auth_info`；`catalog::regions::{STANDARD_REGIONS, region_options}`。
- `log_groups::{LogGroupListing, apply_page_if_current, filter_groups}`、`log_groups::listing`。
- `request::{validate_fetch_input, validate_session_input, ValidatedFetch}`。
- `retry::{RetryPolicy, Retrier, stop_pair, StopHandle, StopSignal, StopReason}`、`pacing::CallPacer`、`schedule::StreamSchedule`、`streams::StreamSelection`、`fetcher::next_page`。
- `sso_login::{resolve_login_target, cli_args, extract_login_hints, is_allowed_login_url}`。
- `time_range::{TimeRange, parse_in_zone, format_display_in_zone}`、`time_zone`、`date_input`。

## AWS への外向きの呼び出し

`gateway/aws.rs` だけが AWS SDK を呼ぶ。操作は `DescribeLogGroups`（一覧、ページあり）、`DescribeLogStreams`（期間に関わるストリーム、`LastEventTime` 降順で読み、1 時間の余白を過ぎたら止める）、`GetLogEvents`（`startFromHead`、範囲を毎回渡す、`nextForwardToken` が同じになったら終端）。SDK の再試行は `GetLogEvents` で 1 回に抑え、再試行は中核の `RetryPolicy` が行う。認証情報は `aws-config` のプロファイル解決に任せ、アプリは読まない。

## OS への呼び出し

- `src-tauri/src/sso_cli/`：`aws sso login [--profile NAME]` を候補パス（Unix：`/opt/homebrew/bin/aws`、`/usr/local/bin/aws`；Windows：`Program Files\Amazon\AWSCLIV2\aws.exe`、次に `PATH` の各フォルダ）から直接起動。標準出力・標準エラーは 1 行ずつ読んで URL とコードだけ抜く。
- `src-tauri/src/browser.rs`：`/usr/bin/open <url>`（Unix）／`ShellExecuteW("open")`（Windows）。
- `cache/`、`cache/settings.rs`：アプリの設定フォルダ（Tauri の `app_config_dir`）に `settings.json` と `log-cache/`。

## 開発者用の入口（配布物には入らない）

- `cargo run -p local-sights-core --example fetch_check -- [--profile NAME] --region CODE --log-group NAME …`：本物の AWS に対して取得し、1 行のまとめを出す。
- `scripts/compat/insights-export.sh` → `cargo run -p local-sights-core --example compat_check -- --export-dir DIR [--queries FILE]`：Logs Insights の結果と比べ、終了コード 0（互換）/ 1（違い）/ 2（比べられない）。
- `npm run check:version [-- --tag vX.Y.Z]`：4 つのファイルの版の一致。
