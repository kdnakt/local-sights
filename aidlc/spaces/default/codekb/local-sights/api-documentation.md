# API の一覧（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直しを土台に、2026-10-25、コミット `793da17` の焦点を絞った読み直し（261025-insights-stats）で更新。公開サーバーの API はなく、ここに挙げるのは (1) 画面とアプリの間の Tauri の IPC、(2) 中核ライブラリの公開 API、(3) 外へ向かう呼び出しである。JSON の項目名はすべて `camelCase`（`serde(rename_all = "camelCase")`）。

## Tauri のコマンド（画面 → アプリ）

`src-tauri/src/lib.rs` の `#[tauri::command]`（行は定義の位置）。`src/api.ts` の同名の関数が `invoke` する。3 か所の登録（`generate_handler!`、`build.rs`、`capabilities/default.json`）は常に同じ 34 個。戻りが「なし」のコマンドは結果を `session-changed` で届ける（古い応答が新しい状態を上書きしないため）。エラーは `CommandError { key, … }`（中核のメッセージ鍵）。

| コマンド | 引数 | 戻り | 役割 | lib.rs |
|---|---|---|---|---|
| `get_session` | なし | `SessionView` | 今の画面の状態をまるごと | 404 |
| `update_input` | `field: InputField ("startText" / "endText")`, `value` | `SessionView` | 時刻の入力。取得中は拒否 | 410 |
| `get_rows` | `offset`, `limit`（最大 1,000） | `DisplayRowWindow` | 見えている行だけを読む | 426 |
| `select_time_zone` | `timeZone: "Local" / "Utc"` | なし | 表示のゾーンを変える | 441 |
| `row_positions` | `keys: RowKey[]`（最大 10,002）, 版 | `RowPositions` | 開いた行の今の位置 | 459 |
| `cancel_close` / `confirm_close` | なし | なし | 閉じる確認の答え | 477 / 491 |
| `set_log_filter` | `text` | なし | 絞り込みの文字。走査を始める | 659 |
| `set_failure_list_open` | `open` | なし | 失敗したストリームの表の開閉 | 744 |
| `start_fetch` | `connectionGeneration` | なし | 取得を始める（検証は中核） | 763 |
| `cancel_fetch` | なし | なし | 取得の中断（`StopReason::Cancel`） | 868 |
| `update_analytics_input` | `field: AnalyticsField`, `value: AnalyticsValue` | なし | クエリの画面の入力（期間・ロググループ（最大 50 の一覧）・クエリ文） | 905 |
| `run_query` | なし | なし（読みの問題は QueryError の相、他の拒否は `CommandError`） | クエリを読んで実行を始める | 941 |
| `cancel_query` | なし | なし | 実行の中断 | 1086 |
| `get_query_rows` | `runId`, `offset`, `limit` | `QueryRowWindow` | 結果の行の窓。`5ced3bf` で `holdingsEpoch` の引数がなくなり、`runId` だけで引く | 1118 |
| `get_query_row_detail` | `runId`, `index` | `QueryRowDetail` | 1 行の項目と値 | 1145 |
| `get_completion_candidates` | なし | `CompletionCandidate[]` | エディタの補完の候補（命令・語句・固定の項目＋JSON の項目名） | 1160 |
| `open_settings` / `cancel_settings` | なし | なし | 設定ダイアログの開閉 | 1187 / 1193 |
| `save_settings`（async） | `cacheEnabled`, `language: "system" / "ja" / "en"` | なし | 設定の保存（`settings.json` に 2 つを一度に）。言語は成功したときだけ変わる（`bd65e49`） | 1206 |
| `clear_cache`（async） | なし | なし | キャッシュのフォルダを空にする | 1221 |
| `start_export`（async、新） | `screen: "logManagement" / "logAnalytics"`, `format: "csv" / "markdown"`, `target: "file" / "clipboard"` | なし（拒否は `CommandError`：`session.exportBusy` / `session.exportUnavailable`） | 表示中の行の書き出し。終わりまで待ち、Running と結果は `session-changed` で届く（`082dc92`） | 1287 |
| `dismiss_export_notice`（新） | なし | なし | 書き出しの通知を閉じる | 1399 |
| `select_profile` | `profile: ProfileSelector` | なし | プロファイルの選択。表示中のログがあれば確認待ち | 1531 |
| `select_region` | `region` | なし | リージョンの選択（同上） | 1541 |
| `confirm_connection_change` / `cancel_connection_change` | なし | なし | 接続の変更の確認の答え | 1551 / 1560 |
| `reload_log_groups` | なし | なし | 一覧の再読み込み | 1573 |
| `update_log_group_filter` | `text` | なし | 一覧の絞り | 1589 |
| `select_log_group` | `name` | なし | ロググループの選択 | 1603 |
| `start_sso_login` | なし | なし | `aws sso login` の起動 | 1620 |
| `cancel_sso_login` / `dismiss_sso_login` | なし | なし | ログインの中断／ダイアログを閉じる | 1638 / 1654 |
| `open_sso_url` | `url` | なし | CLI が出した URL を既定ブラウザで開く（ホストの許可一覧） | 1674 |

## Tauri のイベント（アプリ → 画面）

| イベント | 中身 | いつ |
|---|---|---|
| `session-changed` | `SessionView` 全体 | 状態が変わるたび（取得の始めと終わり、接続、ダイアログ、クエリの進み具合 250 ms ごと（`QUERY_PROGRESS_INTERVAL`）、書き出しの開始と終わり、設定の保存） |
| `fetch-progress` | `FetchProgressUpdate`（jobId、progress の件数、eventCount、timelineVersion、filterSummary、cacheSaving） | Log Management の取得中にページが届くたび。画面は `withProgress` で合成し、`timelineVersion` で行を読み直す |
| `filter-progress` | `FilterProgressUpdate` | 絞り込みの走査中 100 ms ごとと終わり。画面は `withFilter` で合成 |

クエリの画面の進み具合は `fetch-progress` ではなく `session-changed` の `insights.progress`（件数）で届く。定数は `src-tauri/src/lib.rs:218-225` と `src/api.ts:618-622`。

## 主なデータの形（JSON）

- `SessionView`（`session.rs:703`）：`phase`、`connection`、`connectionGeneration`、`profiles`、`regions: string[]`（コードだけ。名前は画面の `regions.ts`）、`timeZone`、`startInput` / `endInput`、`validationErrors`、`logGroups`、`eventCount`、`timelineVersion`、`filterSummary`、`failedStreams`、`elapsed: ElapsedView | null`、`cancelNotice`、`cacheEnabled`、`language: "system" | "ja" | "en"`（新）、`cacheDirectory`、`settingsDialog`、`cacheSaving` / `cacheNotices`、`closeConfirmation`、`ssoLogin`、`fetchOwner`、`insights: InsightsView`、`export: ExportJobView`（新）、`exportNotice: ExportNotice | null`（新）、`canExportLogs`（新）。
- `ElapsedView`：`running`、`elapsedMs`、`fetchNumber`。
- `DisplayRow` / `DisplayRowWindow`：表示用の時刻文字、ストリーム名、メッセージ、`sequence`。
- `InsightsView`（`insights_session.rs:342-400`）：`selectedLogGroupNames`（最大 50）、`maxLogGroups`、`logGroupOptions`、`logGroupStatus`、`startInput` / `endInput`、`queryText`（最大 10,000 文字）、`rangeErrors`、`runBlocker`、`canRun`、`runId`、`phase: AnalyticsPhase`、`progress: InsightsProgress | null`、`problem: QueryProblem | null`（`kind: "Syntax" | "Unsupported"`、`messageKey`、`word`、`span`）、`failure`、`failedGroups`、`partial`、`failedStreams`、`listingFailure`、`result: ResultSummary | null`、`elapsed`、`cancelling`、`cancelAvailable`、`canExport`（新）。
- `InsightsProgress`：`readingCache`、`groupNumber`、`groupCount`、`failedGroupCount`、`selectedStreamCount`、`plannedStreamCount`、`finishedStreamCount`、`eventCount`、`evaluatedCount`、`totalCount`。
- `ResultSummary`（`insights_session.rs:312-330`）：`runId`、`rowCount`、`evaluatedCount`、`columns: string[]`（項目の名前を書いたまま）、`matchedCount`、`truncatedAt`。`holdingsEpoch` はもうない。
- `QueryRow` / `QueryRowWindow` / `QueryRowDetail`（`insights_session.rs:402-451`）：`QueryRow` は `LogEvent` の項目（flatten）＋ `displayTime`、`logGroupName`、`values: (string | null)[]`（結果の列ごとのセル。真偽値は `1` / `0`）。`QueryRowWindow` は `runId`、`offset`、`rows`、`totalCount`。`QueryRowDetail` は `runId`、`index`、`fields: { name, value }[]`。
- `ExportJobView`（`exporter.rs:363`）：書き出しの仕事の状態（`status: "Idle" | "Running" | "Done" | "Failed"` など）。`ExportNotice` は `kind` でタグ付け：`Saved { path, rowCount }` / `Copied { rowCount }` / `SaveFailed { failure, path }` / `CopyFailed { failure }`。`ExportFailure` は `PermissionDenied` / `NoSpace` / `NotFound` / `Other` / `Clipboard` / `Dialog`。
- `ApiFailure`：`kind`（Throttling / Auth / Permission / NotFound / Network / Timeout / Server / Other）、`api`、`safeDetail`、`requestId`。
- `ProfileSelector`：`{ kind: "SdkDefault" }` か `{ kind: "Named", profileName }`。

## 中核ライブラリの公開 API（主なもの）

`crates/local-sights-core` の 28 モジュールはすべて `pub`。アプリと例・テストが使う入口を挙げる（細かな型は `code-structure.md` の表と各モジュールの doc）。

- `coordinator::run_fetch(gateway, validated, store, retrier, sink)` と `run_fetch_with_cache(…, cache, now_ms, sink)`：取得の流れ。`FetchSink`（`on_started` / `on_listing_progress` / `on_planned` / `on_batch(job_id, BatchProgress)` / `on_stream_finished` / `on_saving` / `on_summary` / `on_finished`）、`TimelineStore`（`discard` / `add`）、`HeldEvents::copy_events_in_range`、`decide_job_status`、`DeferredFinishSink`。
- `query::runner::run_query(gateway, run: &QueryRun, store: &Arc<QueryStore>, retrier, sink: &mut impl QuerySink) -> QueryOutcome`：クエリの流れ。`QuerySink: FetchSink + Send` に `on_reading_cache` / `on_group_started` / `on_range_fetched` / `on_fetch_ended -> bool` / `on_field_names` / `on_evaluation_progress -> bool` / `on_evaluated(QueryResult)` / `on_evaluation_stopped` / `on_evaluation_failed` が足される。`QueryOutcome` は `Evaluated { rows }` / `NotEvaluated(JobStatus)` / `EvaluationStopped` / `EvaluationFailed`。
- `query::parser::parse_query(text) -> Result<QueryPlan, QueryProblem>`（`parser::keys::*` の 42 の文言の鍵、`MAX_NESTING`）、`query::instant::resolve_date_texts(plan, zone)`、`query::evaluator::{evaluate, Evaluation, EvalOutput, EvalItem, EvalItems, GroupEvents, ResultRow, QueryResult, row_cells, row_detail, matches, condition_holds, EVALUATION_CHUNK}`、`query::fields::{EventFields, FieldValue, read_field, parse_message, collect_json_field_names}`、`query::natural::{compare_sort_keys, compare_sort_values, natural_cmp}`、`query::plan::{QueryPlan, Command, CommandKind, FieldRef, Condition, QueryProblem, MAX_LIMIT, DEFAULT_RESULT_LIMIT}`。
- `query::store::QueryStore`：`begin_run(keys) -> epoch`、`discard_all`、`epoch`、`covered_ranges`、`counts`、`missing_ranges`、`add_cached`、`settle_range`、`build_view(epoch, start, end) -> Option<usize>`、`view_len`、`evaluate_step(plan, &mut Evaluation, epoch) -> bool`、`json_field_names`、`copy_rows(epoch, positions) -> Option<Vec<ResultRow>>`。
- `export::exporter::{ExportRequest, ExportScreen, ExportTarget, ExportAvailability, ExportBlocker, ExportJob, ExportJobView, ExportNotice, ExportOutcome, ExportFailure, ExportSnapshotSource, ExportSnapshot, render_to, default_file_name, sanitize_file_stem}`、`export::formatter::{ExportFormat, write, write_csv, write_markdown}`（新）。
- `session::AppSession`：`new` / `with_catalog` / `with_catalog_and_time_zones` / `with_cache_location` / `with_clock` / `with_aws_profile_env` / `with_default_time_range`、入力と選択、取得（`begin_fetch`、`on_*`、`finish_fetch`、`request_cancel`、`abort_fetch_with_failure`）、絞り込み、閉じる、設定（`open_settings`、`begin_save_settings(enabled, language)`、`begin_clear_cache`、`finish_settings_work`）、SSO、クエリの転送（`update_analytics_input`、`query_snapshot`、`begin_query`、`on_query_*`、`finish_query_fetch`、`finish_query_evaluation`、`stop_query_evaluation`、`fail_query_evaluation`、`set_query_field_names`、`display_query_rows(run_id, offset, limit)`、`display_query_row_detail`、`request_query_cancel`）、書き出し（`can_export_logs`、`export_job`、`begin_export(request, taken_at) -> ExportBegun`、`finish_export(export_number, outcome) -> bool`、`dismiss_export_notice`）、`view() -> SessionView`、`display_rows`。`LanguagePreference` を再公開。
- `insights_session::InsightsSession`：`update`、`switch_zone`、`blocker`、`snapshot`、`begin(revision, parsed, context, now) -> QueryBegin`、`on_*`、`finish_fetch`、`on_evaluation_progress`、`finish_evaluation(QueryResult, now)`、`fail_evaluation`、`stop_evaluation`、`request_cancel`、`on_connection_changed`、`display_rows(run_id, (offset, limit), zone)`、`display_row_detail`、`completion_candidates`、`can_export(exporting)`、`export_source(zone)`、`export_log_group_name`。
- `gateway::CloudWatchLogsGateway`（trait、`Send + Sync`）：`describe_log_groups`、`describe_log_streams`、`get_log_events` の 3 操作。`gateway::aws::AwsCloudWatchLogsGateway` が実装。
- `cache::LogCache`：`new` / `with_writes`、`lookup`、`read_overlaps`（重なる最後の範囲まで読んで止まる）、`write`、`remove_entry`、`clear_all`、`read_header`；`cache::plan`：鍵と合流の計画、`last_overlapping_range`、`verify_through`；`cache::settings::{LanguagePreference, SettingsLoad, load_settings, save_settings(path, enabled, language), SETTINGS_FORMAT_VERSION}`。
- `log_view::LogView`、`catalog::ConnectionCatalog`、`catalog::regions::{STANDARD_REGIONS, region_options}`、`log_groups::*`、`request::*`、`retry::*`、`pacing::CallPacer`、`schedule::StreamSchedule`、`streams::StreamSelection`、`fetcher::next_page`、`sso_login::*`、`time_range::*`、`time_zone`、`date_input`：前回のとおり。

## AWS への外向きの呼び出し

`gateway/aws.rs` だけが AWS SDK を呼ぶ。操作は `DescribeLogGroups`（一覧、ページあり）、`DescribeLogStreams`（期間に関わるストリーム、`LastEventTime` 降順で読み、1 時間の余白を過ぎたら止める）、`GetLogEvents`（`startFromHead`、範囲を毎回渡す、`nextForwardToken` が同じになったら終端）。SDK の再試行は `GetLogEvents` で 1 回に抑え、再試行は中核の `RetryPolicy` が行う。認証情報は `aws-config` のプロファイル解決に任せ、アプリは読まない。クエリの画面も書き出しもこの 3 操作の外には出ない。

## OS への呼び出し

- `src-tauri/src/sso_cli/`：`aws sso login [--profile NAME]` を候補パスから直接起動。標準出力・標準エラーは 1 行ずつ読んで URL とコードだけ抜く。
- `src-tauri/src/browser.rs`：`/usr/bin/open <url>`（Unix）／`ShellExecuteW("open")`（Windows）。
- `src-tauri/src/export_boundary.rs`（新）：`tauri-plugin-dialog` の `blocking_save_file`（既定のファイル名と 1 つのフィルタ `CSV` / `Markdown`）、`tauri-plugin-clipboard-manager` の `write_text`、一時ファイル → rename の書き。いずれも blocking スレッドで、鍵を持たずに呼ぶ。
- `cache/`、`cache/settings.rs`：アプリの設定フォルダ（Tauri の `app_config_dir`）に `settings.json`（`cacheEnabled`、`language`）と `log-cache/`。

## 開発者用の入口（配布物には入らない）

- `cargo run -p local-sights-core --example fetch_check -- [--profile NAME] --region CODE --log-group NAME …`：本物の AWS に対して取得し、1 行のまとめを出す。
- `scripts/compat/insights-export.sh` → `cargo run -p local-sights-core --example compat_check -- --export-dir DIR [--queries FILE]`：Logs Insights の結果と比べ、終了コード 0（互換）/ 1（違い）/ 2（比べられない）。結果の形（1 行 = 1 イベント）に依存する。
- `npm run check:version [-- --tag vX.Y.Z]`：4 つのファイルの版の一致。
