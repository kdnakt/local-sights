# 部品の一覧（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直しを土台に、2026-10-25、コミット `793da17` の焦点を絞った読み直し（261025-insights-stats）で更新。見出し（`###`）の名前は `reverse-engineering-timestamp.md` の `analyzed.components`（今回深く読んだ部品）と一致する。部品は書くコードの単位で、AWS SDK・ファイルシステム・GUI 枠組み・OS のダイアログは外部の依存として扱い、それらに触れる境界のコードを部品にしている。各項目は「役割／場所／頼る部品」。関係の図は `architecture.md`、公開 API は `api-documentation.md`。今回読んでいない部品（取得・再試行・SSO など）の記述は前回のまま。

## 中核ライブラリ（`crates/local-sights-core/`）

### FetchCoordinator
取得の流れ全体。捨てる → ストリームの計画 → 最大 8 本の同時読み → ページを `TimelineStore` へ（`FetchSink::on_batch(BatchProgress { added, total, timeline_version })` で知らせる）→ 終わりの判定（`decide_job_status`）。キャッシュつきの入口 `run_fetch_with_cache` は検索を blocking スレッドで行い、当たれば `serve_from_cache`、成功した取得を `DeferredFinishSink` で書き終わりまで保留して保存する。`src/coordinator.rs`。頼る：StreamPlanner、StreamSchedule、EventFetcher、CallPacer、RetryPolicy、LogCache、CloudWatchLogsGateway。

### EventFetcher
1 ストリームの 1 ページを読む `next_page`（`GetLogEvents`、範囲を毎回渡す、同じトークンで終端）と `StreamReader` の数え。`src/fetcher.rs`。頼る：PageCursor、RetryPolicy、CallPacer、CloudWatchLogsGateway。

### StreamPlanner
`DescribeLogStreams` を `LastEventTime` 降順で読み、期間に関わるストリームだけを選ぶ（`LISTING_MARGIN_MS` = 1 時間の余白で止める）。`src/streams/mod.rs`、`src/streams/planner.rs`。頼る：CloudWatchLogsGateway、RetryPolicy。

### StreamSchedule
同時に読む本数（`MAX_IN_FLIGHT` = 8）と、使い切りが 3 本続いたら残りを呼ばずに失敗にする規則（`GIVE_UP_LIMIT`）。`src/schedule.rs`。

### CallPacer
リージョンごとの `GetLogEvents` の上限（AWS の制限の半分。25 → 12、10 → 5、30 → 15、既定 5）、スロットルで半分、5 秒静かなら +1。`src/pacing.rs`。リージョンのコードで引く（`SPECIAL_AWS_LIMITS`）。

### RetryPolicy
再試行（1/2/4/8/16 秒 ±20%、最大 5 回）、`Retrier`、止める合図（`StopReason` Cancel / Close、`StopHandle` / `StopSignal`、`wait_or_stop`）、`JitterSource`（本番 `RandomJitter`、テスト `FixedJitter`）。`src/retry.rs`。

### FetchStats
1 行のまとめ（`streams= calls= throttled= elapsed_ms= max_in_flight= min_rate= status=`）。`src/fetch_stats.rs`。

### FetchStatus
取得の終わり方の列挙とその判定の補助。`src/fetch_status.rs`。

### PageCursor
ページの終端判定（トークンが同じになったら終わり、空ページも進める）。`src/paging.rs`。

### LogEvent
1 件のログ（時刻、メッセージ、ストリーム名、順番）。`src/event.rs`。

### EventTimeline
時刻順の列（ストリーム名・順番で安定）、`append`、`rows`、`version`、`position_of`、`take_events`、100 万件の速度テスト（`--ignored`）。`src/timeline.rs`。

### LogView
`EventTimeline` と `FilterEngine` を 1 つの鍵で持つ。`TimelineStore` と `HeldEvents` を実装し、ページを足すと同時に絞り込みを判定する。`MAX_EXPANDED_ROWS` = 10,000、`MAX_POSITION_KEYS` = 10,002。`src/log_view.rs`。頼る：EventTimeline、FilterEngine。

### FilterEngine
メッセージに含む文字（大文字小文字を無視）の絞り込み。`ScanTicket` で走査を切り替え、`on_added` / `on_discarded` で取得中のページも判定、100 ms ごとの進み具合。`src/filter.rs`。

### ApiFailure
失敗の種類（8 種）と安全な詳細（アクセスキーらしい文字を `[REDACTED]`）、リクエスト ID。`src/failure.rs`。

### CloudWatchLogsGateway
AWS の 3 操作だけを持つ trait と要求・応答の型、`ConnectionTarget`。中核が AWS に触れる唯一の境界。`src/gateway/mod.rs`。

### AwsCloudWatchLogsGateway
`aws-sdk-cloudwatchlogs` による実装。プロファイルとリージョンの解決は `aws-config`、`GetLogEvents` の SDK の再試行は 1 回。`src/gateway/aws.rs`。頼る：ErrorClassifier、CloudWatchLogsGateway。

### ErrorClassifier
SDK のエラーを `FailureKind` に分ける（スロットル、認証、権限、見つからない、ネットワーク、時間切れ、サーバー、その他）。`src/gateway/classify.rs`。

### LogCache
JSON Lines のキャッシュファイル（名前は鍵の SHA-256、形式版 2）。`lookup`（検索）、`read_overlaps`（重なる範囲の読み。`5ced3bf` から、求める範囲と重なる最後の範囲まで読んで止まり、そこまでの件数を `verify_through` で確かめる）、`write`（一時ファイル → rename、unix は 0700/0600）、`clear_all`、壊れたファイルの除去、`WriteTracker`。`src/cache/mod.rs`。頼る：CachePlan。

### CachePlan
鍵（接続・ロググループ・期間）と、既存の範囲との合流の計画、記録の余白（`RECORD_MARGIN_MS` = 5 分）、`FORMAT_VERSION` = 2、`last_overlapping_range`（読みを止める範囲）、`verify_through`（読んだところまでの件数の検査）。`src/cache/plan.rs`。

### CacheSettings
`settings.json` の読み書き。形式版 1、`cacheEnabled` と、`bd65e49` から `language`（`LanguagePreference`：`system` / `ja` / `en`。ない値・読めない値は `system` で、キャッシュの設定には触れない）。一時ファイル → rename、owner-only。プロファイル・リージョン・ロググループ・認証情報は書かない。`src/cache/settings.rs`。

### AppSession
Log Management の画面の状態機械と `SessionView` の組み立て。入力・接続・一覧・取得・絞り込み・経過時間・中断の注意・閉じる確認・設定（キャッシュの有無と言語を同じ仕事で保存し、成功したときだけ `language` を変える）・SSO・クエリの画面への転送（`on_query_*`）・1 つの書き出しの仕事（`export: ExportJob`。`can_export_logs`、`begin_export`、`finish_export`、`dismiss_export_notice`）。`src/session.rs`（本体 1〜2,791 行）。頼る：ConnectionState、ConnectionCatalog、LogGroupBrowser、FetchRequest、TimeRangeModel、TimeZoneContext、SsoLoginPlanner、InsightsSession、CacheSettings、ExportJob。

### ConnectionState
今の接続（プロファイル・リージョン）と世代、変更の効果。`src/connection.rs`。

### ConnectionCatalog
`~/.aws/config` と `credentials` を読んでプロファイル（SDK 既定・名前つき、SSO の有無、`source_profile`）とリージョンの候補を作る。`src/catalog/mod.rs`、`src/catalog/files.rs`。頼る：RegionCatalog。

### RegionCatalog
標準リージョンのコードの一覧（`STANDARD_REGIONS`）と、プロファイルの既定リージョンを足して並べる `region_options`。名前は持たず、画面の RegionNames が付ける。`src/catalog/regions.rs`。

### LogGroupBrowser
ロググループの一覧（`DescribeLogGroups` のページを追い、名前順、古い一覧の応答を捨てる）と名前の一部での絞り。`src/log_groups/mod.rs`、`src/log_groups/listing.rs`。頼る：CloudWatchLogsGateway。

### FetchRequest
取得の入力の検証（`FetchInput` → `ValidatedFetch`、名前の長さ、期間の前後）とメッセージ鍵。`src/request.rs`。頼る：TimeRangeModel。

### TimeRangeModel
`yyyy/mm/dd hh:mm:ss` の読み（ゾーンつき、夏時間の重複は早いほう）と書き、終了時刻の 999 ms、表示用の文字。`src/time_range.rs`。

### TimeZoneContext
ローカル／UTC の選択と IANA のゾーンの解決。`src/time_zone.rs`。

### DateTimeInput
日付・時刻の欄の補完と形の検査（中核側）。`src/date_input.rs`。

### SsoLoginPlanner
プロファイルから SSO の設定を解く（`source_profile` を最大 5 段）、CLI の引数、出力から URL とコードだけを抜く、URL のホストの許可（`.amazonaws.com`、`.awsapps.com`）。`src/sso_login.rs`。

### InsightsSession
Log Analytics の画面の状態機械。選んだロググループ（最大 50）、期間、クエリ文（最大 10,000 文字）、入力の `revision`、`run_id`、相（`AnalyticsPhase`：Idle → Fetching → Evaluating → Done、または QueryError / FetchFailed / Cancelled / EvaluationFailed）、進み具合（`InsightsProgress`）、結果（`ResultSummary`、`QueryResult`。`begin` で前回の結果を持ち越し、`finish_evaluation` でだけ差し替える。`fail_evaluation` と FetchFailed では捨てる）、行と詳細の表示（`display_rows` / `display_row_detail`：結果の複製から `run_id` で読み、セルの文字をそのとき作る）、補完の候補、書き出しの元（`can_export`、`export_source`、`export_log_group_name`：実行の始めに選んでいたロググループの最初の名前）。`src/insights_session.rs`（本体 1〜1,265 行）。頼る：QueryParser、QueryPlan、DateTextResolver、QueryEvaluator、EventFieldReader、TimeRangeModel、ExportJob（型）。

### QueryParser
`fields` / `filter` / `sort` / `limit` の読み。語を順に読み、最初の問題を場所つきで返す。命令名は `command()` で小文字にして振り分け、`OTHER_COMMANDS`（`stats`、`parse`、`dedup`、`display`、`pattern`、`diff`、`unmask`、`unnest`、`lookup`、`filterindex`、`source`）は `UNSUPPORTED_COMMAND`、他の語は `UNKNOWN_COMMAND`。関数は `function()` が `toMillis(@timestamp)` だけを読み、他（`count`、`sum`、`bin` など）は `UNSUPPORTED_FUNCTION`。`fields … as` と `limit any` も Unsupported。`keys` に 42 の文言の鍵、`MAX_NESTING` = 100。`src/query/parser.rs`（2,553 行、テストは 1,400 行以降）。頼る：QueryPlan、DateTextResolver（`is_date_text`）。

### QueryPlan
クエリの計画の型：`QueryPlan { commands }`、`Command { kind, span }`、`CommandKind`（`Filter(Condition)` / `Fields(Vec<FieldRef>)` / `Sort(Vec<SortKey>)` / `Limit(u32)` の 4 種）、`Condition` / `ConditionKind`（And / Or / Not / Compare / CompareFields / Like / Match / In）、`FieldRef`（`Timestamp` / `TimestampMillis` / `Message` / `LogStream` / `Log` / `Json(JsonPath)`）、`Literal`、`QueryRegex`、`Span`、`QueryProblem { kind, message_key, word, span }`、`ProblemKind`（Syntax / Unsupported）、定数 `MAX_LIMIT` = 100,000、`DEFAULT_RESULT_LIMIT` = 10,000。`columns()` は `fields` の項目を書いた順に 1 回ずつ、なければ `@timestamp` と `@message`。`has_row_limit`（`limit 0` は 0 行）。`src/query/plan.rs`。

### QueryEvaluator
計画を一覧に適用する `Evaluation`（段階 `Stage`：固定の順序に並べる Ordering → 命令を書いた順に Command（`filter` と JSON 項目の `sort` は `EVALUATION_CHUNK` = 10,000 行ずつ `Pass` を進め、`fields` と `limit` は塊なし）→ 値のない列を落とす Columns → Done。`rows: Vec<usize>` は保持の位置）、`EvalOutput`（位置の列、`columns`、`matched_count`、`truncated_at`、`evaluated_count`）、`ResultRow { event: LogEvent, log_group_name }`（結果の行は 1 イベントの写し）、`QueryResult`（`run_id`、`evaluated_count`、`rows: Vec<ResultRow>`、`columns`、`matched_count`、`truncated_at`。`window(run_id, offset, limit)` と `row_at` は `run_id` が合うときだけ返す。`holdings_epoch` は `5ced3bf` で廃止）、`row_cells(columns, EventFields, format_time)`、`row_detail`、`evaluate`（止める合図と進み具合つきの一括）、`matches` / `condition_holds`。`src/query/evaluator.rs`。頼る：EventFieldReader、NaturalSort、QueryPlan。

### EventFieldReader
1 イベントの項目。固定 4 項目と、メッセージが JSON オブジェクトのときの入れ子の項目（1 回だけ読む `OnceCell`）、`FieldValue`（Missing / Number / Text / Boolean）、`display`（セルの文字。JSON の数は serde_json の書き方、真偽値は `1` / `0`）、`json_fields`（開いた行の項目）、項目名の収集（`FIELD_NAME_SCAN_EVENTS` = 10,000 件まで、`MAX_FIELD_NAMES` = 500 個まで）。`src/query/fields.rs`。

### NaturalSort
`sort` の順序（ない値 → 数でない値 → 数、文字は自然順、降順は逆）。`SortValue`（所有）と `SortKeyRef`（借用）。`src/query/natural.rs`。

### DateTextResolver
`@timestamp` と比べる日付の文字（`YYYY-MM-DD HH:MM:SS[.sss]` と `…Z`）をミリ秒に解く `parse_date_text` / `resolve_date_texts`、`is_date_text`。`src/query/instant.rs`。

### QueryStore
クエリの画面の保持。`HoldingKey`（接続・ロググループ）ごとの committed / staging の列（`GroupHoldings`）、`StreamSequences`（ストリーム内の番号）、`epoch`、`begin_run(keys)`（選ばれなくなったロググループの保持を捨て、残りの staging を捨て、世代を進める。`5ced3bf` FR2.2.1）、`discard_all`（接続の変更）、`missing_ranges`、`add_cached`、`settle_range`（終わった範囲を committed と staging に分ける）、`build_view(epoch, start, end)`（世代を固定した時刻順の一覧 `ViewEntry` を作る）、`evaluate_step`（鍵の中で 1 塊）、`json_field_names`、`copy_rows(epoch, positions)`（結果の行を複製）。保持はロググループごとの `Vec<LogEvent>` で、一覧は `RunItems` が `EvalItems` として貸す。`src/query/store.rs`。頼る：EventTimeline（型）、CachePlan、QueryEvaluator（型）。

### QueryRunner
クエリの実行の流れ `run_query`：`begin_run` → `fetch_missing`（ロググループを選んだ順に、足りない範囲を時刻順に。キャッシュを先に読み、`fetch_range` が範囲ごとの holder（`Arc<Mutex<EventTimeline>>`）へ `run_fetch_with_cache` で取り、終わってから `settle_range`。認証の失敗で残りは始めない）→ `on_fetch_ended` → `build_view` → `gather_field_names` → `evaluate_held`（blocking スレッドで 1 塊ずつ、`on_evaluation_progress` で続けるか聞く）→ `copy_rows` → `on_evaluated(QueryResult)`。評価は取得がすべて終わってから 1 回だけで、途中の結果を渡す口はない。`QuerySink`（`FetchSink` ＋ 9 つの通知）、`QueryRun`（`run_id`、`plan`、`range`、`keys`、`cache`、`run_started_at_ms`、`clock`）、`QueryOutcome`、`QueryRunSummary`、`GroupFetchOutcome` / `GroupFetchStatus`（NotNeeded / FromCache / Completed / CompletedWithFailures / Failed / Cancelled）、`GroupFailedStream`。`src/query/runner.rs`。頼る：QueryStore、LogCache、FetchCoordinator、QueryEvaluator、RetryPolicy。

### ExportJob
書き出しの仕事とその型：`ExportRequest { screen, format, target }`、`ExportScreen`、`ExportTarget`、`ExportBlocker`（NoRows / Fetching / Filtering / Running / NoResult / Exporting）と `ExportAvailability`、`ExportJob`（`begin(request, row_count) -> export_number`、`finish(export_number, outcome) -> bool`、`dismiss_notice`、`view`。アプリに 1 つ）、`ExportStatus`、`ExportNotice`（Saved / Copied / SaveFailed / CopyFailed）、`ExportFailure`、`ExportOutcome`、`ExportSnapshotSource`（`LogManagement { events, zone }` / `LogAnalytics { columns: Vec<FieldRef>, rows: Vec<ResultRow>, zone }`。鍵の中で複製し、`into_snapshot` で鍵の外で文字にする：Log Management は `@timestamp` / `@logStream` / `@message` の 3 列、Log Analytics は結果の列を `row_cells` で）、`ExportSnapshot`、`render_to(out, snapshot, format, bom)`、`default_file_name(log_group, taken_at, zone, format)`（`sanitize_file_stem`、最大 100 文字、だめなら `export`）、`UTF8_BOM`。`src/export/exporter.rs`、`src/export/mod.rs`。頼る：ExportFormatter、QueryEvaluator（`ResultRow`、`row_cells`）、EventFieldReader、TimeRangeModel。

### ExportFormatter
純粋な書き手：`ExportFormat`（Csv / Markdown、`extension`）、`write` / `write_csv`（RFC 4180。必要なときだけ引用符）/ `write_markdown`（表。`|` と改行をエスケープ）。行ごとに `Write` へ書き、100 万行を 1 つの文字列にしない。値は変えない（`=` で始まる値もそのまま）。`src/export/formatter.rs`。

### FetchCheck
開発者が手元で本物の AWS に対して取得を確かめる例。`examples/fetch_check.rs`。

### CompatCheck
`insights-export.sh` が書き出した Logs Insights の結果と、アプリの評価結果を比べる例。`compat/`（`queries`、`settings`、`export`、`format`、`compare`、`report`）は純粋なロジックで、終了コード 0/1/2。結果の形（1 行 = 1 イベント）に依存する。`examples/compat_check.rs`、`examples/compat/*.rs`、`examples/compat_queries.txt`。今回は開いていない。

## アプリ（`src-tauri/`）

### TauriApp
Tauri のアプリ本体。`AppState`（`AppSession`・`LogView`・`QueryStore`・ゲートウェイ・取得とクエリの止める合図・SSO の制御）、34 個のコマンド、3 つのイベント、取得・クエリ・絞り込み・書き出しのタスクの起動と回復（`recover_if_still_fetching` / `recover_if_query_still_running`）、2 つのプラグイン（ダイアログ、クリップボード）の登録、メニュー（Quit）、閉じる確認、起動時の一覧、設定とキャッシュの場所。`start_export` は async で、鍵の中の開始 → 鍵の外の `run_export` → 鍵の中の終わり。`src/lib.rs`、`src/main.rs`、`build.rs`、`capabilities/default.json`、`tauri.conf.json`。頼る：AppSession、InsightsSession、FetchCoordinator、QueryRunner、LogView、QueryStore、LogCache、AwsCloudWatchLogsGateway、TauriSink、SsoCliRunner、BrowserOpener、ExportBoundary。

### TauriSink
`FetchSink` / `QuerySink` / `ListingSink` の実装（`TauriSink`、`TauriQuerySink`、`TauriListingSink`）。中核の進み具合を `AppSession` に写し、`session-changed` / `fetch-progress` を emit する。`TauriQuerySink` は `update`（毎回押す）と `update_progress`（前回から `QUERY_PROGRESS_INTERVAL` = 250 ms 経っていれば押す）を持ち、`on_batch` は進み具合の数にしか使わない。`src/lib.rs:1808-2083`。

### ExportBoundary
書き出しの OS 境界：`dialog_filter`、`ask_save_path`（`tauri-plugin-dialog` の `blocking_save_file`。取り消しは `None`、パスでない答えは `ExportFailure::Dialog`）、`copy_to_clipboard`（`tauri-plugin-clipboard-manager`）、`failure_kind`（`io::ErrorKind` → `ExportFailure`）、`write_file_atomically`（同じフォルダの一時ファイルに書いて rename）。OS のエラーの言葉は標準エラーだけ。`cfg` は使わない。すべて blocking スレッドで、鍵を持たずに呼ばれる。`src/export_boundary.rs`。頼る：ExportJob（型）、ExportFormatter（型）。

### SsoCliRunner
AWS CLI の場所の探索、起動（シェルなし、プロセスグループ／ジョブオブジェクト）、出力の 1 行ずつの処理、5 分の時間切れ、中断と終了時の停止。`src/sso_cli/mod.rs`、`unix.rs`、`windows.rs`。頼る：SsoLoginPlanner。

### BrowserOpener
URL を既定ブラウザで開く（`/usr/bin/open`／`ShellExecuteW`）。許可の判断は呼び出し側。`src/browser.rs`。

## 画面（`src/`）

### ApiBridge
Tauri のコマンドの関数（34）、イベントの購読（`onSessionChanged` ほか）、`SessionView` と部分更新の型、`mergeSessionView` / `withProgress` / `withFilter` の合成、`isCommandError`、`selectorOf`。`saveSettings(enabled, language)`、`startExport(screen, format, target)`、`dismissExportNotice`、`getQueryRows(runId, offset, limit)`。`api.ts`。

### AppShell
1 つの窓の組み立てとページの切り替え、`SessionView` の受け取りと合成、言語の決定（`session.language` と OS の言語から `resolveLocale` で `locale` を決め、`createTranslator`、Cloudscape の `I18nProvider`（`all.en` / `all.ja`）、`document.documentElement.lang` を従わせる。最初の view が届くまでは `system`）、ダイアログの表示、各コマンドの呼び出し、書き出しの通知の表示、ページを見せたときのフォーカス。`App.tsx`、`main.tsx`。頼る：ApiBridge、MessageCatalog と下の各部品。

### AppNavigation
左端の SideNavigation（Log Management / Log Analytics）、開閉とフォーカスの移動。`components/AppNavigation.tsx`。

### TopBar
黒い上部のバー。プロファイルとゾーンの `ButtonDropdown`、リージョンは `5ced3bf` から Cloudscape の `Select`（`filteringType="manual"`、コードと名前を並べ、どちらの言語の名前でも絞れる）、設定ボタン。`components/TopBar.tsx`。頼る：RegionNames、ApiBridge。

### ConditionArea
開始・終了の日付（DatePicker）と時刻（TimeInput）、理由の表示、［Fetch］と［Cancel］、Log Management の［Export］メニュー。`components/ConditionArea.tsx`。頼る：DateTimeHelpers、ExportMenu。

### LogGroupPane
左のロググループの表と検索、再読み込み、開閉。`components/LogGroupPane.tsx`。

### PaneResizer
ペインの幅の掴み手（割合で保存）。`components/PaneResizer.tsx`。頼る：ScreenHooks。

### LogFilterInput
絞り込みの文字の欄（300 ms のデバウンス）。`components/LogFilterInput.tsx`。

### LogTable
仮想スクロールの一覧。見えている行だけを `get_rows` で読み、開いた行・選択・キーボード、位置の追従（`row_positions`）。`components/LogTable.tsx`、`rowLayout.ts`、`virtualScroll.ts`。頼る：ScreenHooks、ExpandedMessage、ApiBridge。

### LogTableCloudscape
Cloudscape の Table での試作（環境変数 `VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape` で切り替え）。`components/LogTableCloudscape.tsx`。

### ExpandedMessage
開いた行のメッセージ全文（文字ノードだけ、最大 20 行で内側スクロール）。`082dc92` から、メッセージの中の最初の完全な JSON を `findJson` で見つけて整形し、`SegmentedControl` の「整形／原文」（`PrettyPrintMode`：`pretty` / `raw`。開いた行ごとに持ち、閉じて開き直すと `pretty` に戻る）で切り替える。切替の操作は行やグリッドに届かない。JSON がなければ切替を出さない。`components/ExpandedMessage.tsx`。頼る：JsonPrettyPrinter。

### JsonPrettyPrinter
純粋な関数：`findJson(message) -> JsonSegments | null`（`{` と `[` の位置を先頭から試し、RFC 8259 の字句走査で最後まで読めた最初のものを採る。スカラーだけの配列は候補にしない。`MAX_DEPTH` = 256、`MAX_CANDIDATES` = 64、`MAX_MESSAGE_LENGTH` = 1 MiB を超えたら「見つからない」）と `prettyPrint(json)`（字句をそのまま保って整形。`JSON.parse` は使わない）。`jsonPretty.ts`。

### StatusLine
一覧の下の 1 行（列挙中・取得中・件数・失敗の本数・絞り込みの状態・キャッシュの注意）。`components/StatusLine.tsx`。頼る：ElapsedTime、MessageCatalog。

### ElapsedTime
「Elapsed mm:ss」。画面の時計で進め、読み上げから隠す。`components/ElapsedTime.tsx`、`elapsed.ts`、`hooks/useElapsed.ts`。

### CancelNotice
中断の注意（未完了の本数、または列挙前の中断）。`5ced3bf` から `role="status"` の囲みを常に置き、注意があるときだけ中に `Alert` を出す（空のときは `.cancel-notice:empty` で場所を取らない）。`components/CancelNotice.tsx`。

### FetchErrorBanner
取得全体が失敗したときの Alert（何が起きたか・次に何をするか・安全な詳細）、SSO のボタン。`components/FetchErrorBanner.tsx`。頼る：ErrorMessage、SsoLoginButton。

### ErrorMessage
エラーの言葉を行に分けて出す。`components/ErrorMessage.tsx`、`errorText.ts`。

### FailureList
失敗したストリームの表。`components/FailureList.tsx`。

### ExportMenu
両方の画面の［Export］`ButtonDropdown`。4 項目を決まった順に（CSV で保存、Markdown で保存、CSV でコピー、Markdown でコピー）。書き出せないときは全体を無効、書き出し中は busy。`components/ExportMenu.tsx`。

### ExportNoticeAlert
最後の書き出しの通知。種類と引数（件数、パス、失敗の種類）から文を作るので言語切替で書き直る。成功は `role="status"` の success、失敗は `role="alert"` の error、閉じると `dismissExportNotice`。`components/ExportNoticeAlert.tsx`。

### SettingsDialog
設定のダイアログ（キャッシュの有効・場所・警告、`bd65e49` から「言語」の `Select`（「OS の設定に合わせる」は画面の言語で、「日本語」「English」はそれぞれの言語で固定）、［Clear cache］［Cancel］［Save］。保存は 2 つの設定を 1 回で）。`components/SettingsDialog.tsx`。

### CloseConfirmDialog
取得中に窓を閉じるときの確認。`components/CloseConfirmDialog.tsx`。

### ConfirmDialog
接続の変更で表示中のログと結果を捨てることの確認。`components/ConfirmDialog.tsx`。

### SsoLoginButton
［Sign in with SSO］（認証エラーの中、SSO につながるプロファイルのとき）。`components/SsoLoginButton.tsx`。

### SsoLoginModal
SSO ログインのダイアログ（状態の文、URL のリンク、コード、中断と閉じる）。`components/SsoLoginModal.tsx`。

### LogAnalyticsPage
Log Analytics の画面全体（ロググループの選択、期間、クエリ欄、実行と中断、進み具合 `RunProgress`（`5ced3bf` からクエリの容器の中、結果の容器には出さない）、結果の領域 `ResultArea` / `ResultBody`（`result !== null` のときだけ `QueryResultTable` を描き、実行中は `previous` の印を付ける。実行中で前回の結果がないときは空）、結果の見出しの件数と切り捨ての注記、［Export］メニュー、失敗の一覧、例のクエリの差し替えダイアログ）。`components/LogAnalyticsPage.tsx`。頼る：LogGroupMultiselect、QueryEditor、QueryResultTable、ExportMenu、FailureList、ElapsedTime、AnalyticsHelpers、DateTimeHelpers。

### QueryEditor
Cloudscape の CodeEditor（Ace）。問題の下線、Cmd/Ctrl+Enter の実行、補完、色分け（`QUERY_HIGHLIGHT_RULES`）。`components/QueryEditor.tsx`、`aceLoader.ts`。頼る：AnalyticsHelpers。

### QueryResultTable
結果の仮想スクロールの表。`result.runId` で `get_query_rows(runId, first, count)` / `get_query_row_detail(runId, position)` を引き（古い応答と別の `runId` の答えは捨てる）、開いた行の詳細、`previous` のときは「前回の結果」の文を上に出す。`@message` は 1 行目だけ。`components/QueryResultTable.tsx`。頼る：ApiBridge、AnalyticsHelpers（`firstLine`）、`rowLayout.ts`。

### LogGroupMultiselect
最大 50 のロググループの Multiselect（名前の一部で検索）。`components/LogGroupMultiselect.tsx`。

### AnalyticsHelpers
純粋な補助：ページの種類、問題の言葉と範囲（`problemText`、`problemRange`、`offsetOf`）、構文の色分けの規則 `QUERY_HIGHLIGHT_RULES`（命令は `fields|filter|sort|limit`、関数は `toMillis` だけ）、補完の項目 `completionItems`、例のクエリ `QUERY_EXAMPLES`（5 つ）、`firstLine`、`limitSelection`、ショートカットの修飾キー。`analytics.ts`、`platform.ts`。

### DateTimeHelpers
純粋な補助：日付と時刻の欄の分割・結合・補完・検査、理由の欄の割り当て、1 行表示。`dateTime.ts`、`format.ts`。

### ScreenHooks
画面だけの状態と計測：開いた行（`expansionState.ts`）、ペインの幅（`localStorage`）、色の様式、デバウンス、Escape、初回フォーカス、行の配置と窓、位置の問い合わせ。`hooks/*.ts`、`designTokens.ts`。

### MessageCatalog
日英の対訳表 `messages`、`MessageKey`、`Locale`（`en` / `ja`）、`LanguagePreference`（`system` / `ja` / `en`）、`englishSingular`（`5ced3bf`：本数を数える英語の文の単数形。数が 1 のときだけ）、`detectLocale`（OS の最初の言語）、`resolveLocale(preference, languages)`、`translate`（`{name}` の置き換えと単数形の選択。知らない鍵はそのまま返す）、`createTranslator`、`timeZoneLabelKey`。`i18n/messages.ts`。

### RegionNames
リージョンのコードごとの名前（日英、AWS の文書の表記）`REGION_NAMES`、`regionLabel(code, locale)`（表にないコードはコードだけ）、`filterRegionOptions(codes, text)`。`i18n/regions.ts`。

### PerfRowsHarness
速度測定用の生成行（`VITE_LOCAL_SIGHTS_PERF_ROWS`）。本番では動かない。`dev/perfRows.ts`。

## テストの道具・スクリプト・CI

### FakeGateway
結合テストの偽のゲートウェイ。ロググループ・ストリーム・ページの台本、呼び出しの記録、応答の遅れと同時数、答えないストリーム。`crates/local-sights-core/tests/support/fake_gateway.rs`。

### RecordingSink
`FetchSink` の呼ばれ方を記録する偽の sink。`crates/local-sights-core/tests/support/recording_sink.rs`。

### ReleaseScripts
版の一致の検査（`check-version.mjs`、`versions.ts`）、アドホック署名のワークフローの検査（`signing.ts`）、アイコンの PNG 描画（`render-icon.mjs`）。`scripts/release/`、`scripts/render-icon.mjs`。

### InsightsExportScript
Logs Insights の結果を手で書き出す bash スクリプト。`scripts/compat/insights-export.sh`。

### CiWorkflows
`ci.yml`（4 ジョブ、Linux / macOS / Windows）、`release.yml`、`release-dry-run.yml`、`dependabot.yml`、`deny.toml`（`BSL-1.0` を許可に追加）。`.github/`、`deny.toml`。
