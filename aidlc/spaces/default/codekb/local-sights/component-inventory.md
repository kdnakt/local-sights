# 部品の一覧（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直し。見出し（`###`）の名前は `reverse-engineering-timestamp.md` の `analyzed.components` と同じ。部品は書くコードの単位で、AWS SDK・ファイルシステム・GUI 枠組みは外部の依存として扱い、それらに触れる境界のコードを部品にしている。各項目は「役割／場所／頼る部品」。関係の図は `architecture.md`、公開 API は `api-documentation.md`。

## 中核ライブラリ（`crates/local-sights-core/`）

### FetchCoordinator
取得の流れ全体。捨てる → ストリームの計画 → 最大 8 本の同時読み → ページを `TimelineStore` へ → 終わりの判定（`decide_job_status`）。キャッシュつきの入口 `run_fetch_with_cache` は検索を blocking スレッドで行い、当たれば `serve_from_cache`、成功した取得を `DeferredFinishSink` で書き終わりまで保留して保存する。`src/coordinator.rs`。頼る：StreamPlanner、StreamSchedule、EventFetcher、CallPacer、RetryPolicy、LogCache、CloudWatchLogsGateway。

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
時刻順の列（ストリーム名・順番で安定）、`append`、`rows`、`version`、`position_of`、100 万件の速度テスト（`--ignored`）。`src/timeline.rs`。

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
JSON Lines のキャッシュファイル（名前は鍵の SHA-256、形式版 2）。`lookup`（検索）、`read_overlaps`（重なる範囲の読み。今は重なりがあるとファイルを丸ごと読む）、`write`（一時ファイル → rename、unix は 0700/0600）、`clear_all`、壊れたファイルの除去、`WriteTracker`。`src/cache/mod.rs`。頼る：CachePlan。

### CachePlan
鍵（接続・ロググループ・期間）と、既存の範囲との合流の計画、記録の余白（`RECORD_MARGIN_MS` = 5 分）、`FORMAT_VERSION` = 2。`src/cache/plan.rs`。

### CacheSettings
`settings.json`（キャッシュの有効・無効）の読み書き。`src/cache/settings.rs`。

### AppSession
Log Management の画面の状態機械と `SessionView` の組み立て。入力・接続・一覧・取得・絞り込み・経過時間（`fetch_number`、`ElapsedView`）・中断の注意・閉じる確認・設定・SSO・クエリの画面への転送。`src/session.rs`（本体 1〜3120 行）。頼る：ConnectionState、ConnectionCatalog、LogGroupBrowser、FetchRequest、TimeRangeModel、TimeZoneContext、SsoLoginPlanner、InsightsSession、CacheSettings。

### ConnectionState
今の接続（プロファイル・リージョン）と世代、変更の効果。`src/connection.rs`。

### ConnectionCatalog
`~/.aws/config` と `credentials` を読んでプロファイル（SDK 既定・名前つき、SSO の有無、`source_profile`）とリージョンの候補を作る。`src/catalog/mod.rs`、`src/catalog/files.rs`。頼る：RegionCatalog。

### RegionCatalog
標準リージョンのコードの一覧（`STANDARD_REGIONS`、34 個）と、プロファイルの既定リージョンを足して並べる `region_options`。名前（東京など）は持たない。`src/catalog/regions.rs`。

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
プロファイルから SSO の設定を解く（`source_profile` を最大 5 段）、CLI の引数、出力から URL とコードだけを抜く（1 行 `MAX_LINE_BYTES` まで）、URL のホストの許可（`.amazonaws.com`、`.awsapps.com`）。`src/sso_login.rs`。

### InsightsSession
Log Analytics の画面の状態機械。選んだロググループ（最大 50）、期間、クエリ文（最大 10,000 文字）、`run_id`、進み具合、結果（`ResultSummary`、`QueryResult`）、行と詳細の表示、補完の候補。`src/insights_session.rs`。頼る：QueryParser、QueryPlan、DateTextResolver、QueryEvaluator、TimeRangeModel。

### QueryParser
`fields` / `filter` / `sort` / `limit` の読み。最初の問題を場所つきで返し、他のコマンドは Unsupported。`src/query/parser.rs`。頼る：QueryPlan。

### QueryPlan
クエリの計画の型（`Command`、`Condition`、`FieldRef`、`JsonPath`、`QueryRegex`、`Span`、`QueryProblem`）と定数 `MAX_LIMIT` = 100,000、`DEFAULT_RESULT_LIMIT` = 10,000、`has_row_limit`（`limit 0` は 0 行）。`src/query/plan.rs`。

### QueryEvaluator
計画を一覧に適用する `Evaluation`（固定の順序 → コマンドを書いた順に → 値のない列を落とす）、10,000 行ずつ止められる、`QueryResult`（`run_id`、`holdings_epoch`、`rows`、`columns`、`truncated_at`）、`row_cells` / `row_detail`。`src/query/evaluator.rs`。頼る：EventFieldReader、NaturalSort。

### EventFieldReader
1 イベントの項目。固定 4 項目と、メッセージが JSON オブジェクトのときの入れ子の項目（1 回だけ読む）、項目名の収集（10,000 件まで、500 個まで）。`src/query/fields.rs`。

### NaturalSort
`sort` の順序（ない値 → 数でない値 → 数、文字は自然順、降順は逆）。`src/query/natural.rs`。

### DateTextResolver
`@timestamp` と比べる日付の文字（`YYYY-MM-DD HH:MM:SS[.sss]` と `…Z`）をミリ秒に解く。`src/query/instant.rs`。

### QueryStore
クエリの画面の保持。`HoldingKey`（接続・ロググループ）ごとの committed / staging の列、`StreamSequences`（ストリーム内の番号）、`epoch`、`begin_run`（staging だけ捨てる）、`discard_all`（接続の変更）、`missing_ranges`、`add_cached`、`settle_range`、`build_view`、`evaluate_step`、`copy_rows`。`src/query/store.rs`。頼る：EventTimeline、CachePlan。

### QueryRunner
クエリの実行の流れ `run_query`：ロググループごとに足りない範囲を求め、キャッシュの重なりを読み、`run_fetch` で取り足し、評価を blocking スレッドで塊ごとに進める。`QuerySink`、`QueryRun`、`QueryOutcome`、`QueryRunSummary`。`src/query/runner.rs`。頼る：QueryStore、LogCache、FetchCoordinator、QueryEvaluator。

### FetchCheck
開発者が手元で本物の AWS に対して取得を確かめる例。アプリと同じ検証と `FetchCoordinator`、`--region` 必須、まとめを標準エラーへ。引数の読みだけがテストされる。`examples/fetch_check.rs`。

### CompatCheck
`insights-export.sh` が書き出した Logs Insights の結果と、アプリの評価結果を比べる例。`compat/`（`queries`、`settings`、`export`、`format`、`compare`、`report`）は純粋なロジックで、終了コード 0/1/2。`limit 0` を絞りなしと扱う箇所がある。`examples/compat_check.rs`、`examples/compat/*.rs`、`examples/compat_queries.txt`。

## アプリ（`src-tauri/`）

### TauriApp
Tauri のアプリ本体。`AppState`（`AppSession`・`LogView`・`QueryStore`・ゲートウェイ・止める合図・SSO の制御）、32 個のコマンド、3 つのイベント、取得・クエリ・絞り込みのタスクの起動と回復、メニュー（Quit）、閉じる確認、起動時の一覧、設定とキャッシュの場所。`src/lib.rs`、`src/main.rs`、`build.rs`、`capabilities/default.json`、`tauri.conf.json`。頼る：AppSession、InsightsSession、FetchCoordinator、QueryRunner、LogView、QueryStore、LogCache、AwsCloudWatchLogsGateway、TauriSink、SsoCliRunner、BrowserOpener。

### TauriSink
`FetchSink` / `QuerySink` / `ListingSink` の実装（`TauriSink`、`TauriQuerySink`、`TauriListingSink`）。中核の進み具合を `AppSession` に写し、`session-changed` / `fetch-progress` を emit する。`src/lib.rs:1664-1884`。

### SsoCliRunner
AWS CLI の場所の探索（`cli_candidates` / `locate_cli`）、起動（シェルなし、標準入力を閉じ、プロセスグループ／ジョブオブジェクト）、出力の 1 行ずつの処理、5 分の時間切れ、中断と終了時の停止（SIGTERM → 2 秒 → SIGKILL／`TerminateJobObject`）。`src/sso_cli/mod.rs`、`unix.rs`、`windows.rs`。頼る：SsoLoginPlanner。

### BrowserOpener
URL を既定ブラウザで開く（`/usr/bin/open`／`ShellExecuteW`）。許可の判断は呼び出し側。`src/browser.rs`。

## 画面（`src/`）

### ApiBridge
Tauri のコマンドの関数、イベントの購読（`onSessionChanged` ほか）、`SessionView` と部分更新の型、`withProgress` / `withFilter` の合成、`isCommandError`、`selectorOf`。`api.ts`。

### AppShell
1 つの窓の組み立てとページの切り替え、`SessionView` の受け取りと合成、ダイアログの表示、各コマンドの呼び出し、ページを見せたときのフォーカス。`App.tsx`、`main.tsx`。頼る：ApiBridge と下の各部品。

### AppNavigation
左端の SideNavigation（Log Management / Log Analytics）、開閉とフォーカスの移動。`components/AppNavigation.tsx`。

### TopBar
黒い上部のバー。プロファイル・リージョン・ゾーンの `ButtonDropdown`（リージョンはコードの文字だけ、検索なし）、設定ボタン。`components/TopBar.tsx`。

### ConditionArea
開始・終了の日付（DatePicker）と時刻（TimeInput）、理由の表示、［Fetch］と［Cancel］。`components/ConditionArea.tsx`。頼る：DateTimeHelpers。

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
開いた行のメッセージ全文（文字ノードだけ、最大 20 行で内側スクロール）。`components/ExpandedMessage.tsx`。

### StatusLine
一覧の下の 1 行（列挙中・取得中・件数・失敗の本数・絞り込みの状態・キャッシュの注意）。`components/StatusLine.tsx`。頼る：ElapsedTime、MessageCatalog。

### ElapsedTime
「Elapsed mm:ss」。画面の時計で進め、読み上げから隠す。`components/ElapsedTime.tsx`、`elapsed.ts`、`hooks/useElapsed.ts`。

### CancelNotice
中断の注意（未完了の本数、または列挙前の中断）。`role="status"` の囲みと Alert を同時に描く。`components/CancelNotice.tsx`。

### FetchErrorBanner
取得全体が失敗したときの Alert（何が起きたか・次に何をするか・安全な詳細）、SSO のボタン。`components/FetchErrorBanner.tsx`。頼る：ErrorMessage、SsoLoginButton。

### ErrorMessage
エラーの言葉を行に分けて出す。`components/ErrorMessage.tsx`、`errorText.ts`。

### FailureList
失敗したストリームの表。`components/FailureList.tsx`。

### SettingsDialog
設定のダイアログ（キャッシュの有効・場所・警告、［Clear cache］［Cancel］［Save］）。`components/SettingsDialog.tsx`。

### CloseConfirmDialog
取得中に窓を閉じるときの確認。`components/CloseConfirmDialog.tsx`。

### ConfirmDialog
接続の変更で表示中のログと結果を捨てることの確認。`components/ConfirmDialog.tsx`。

### SsoLoginButton
［Sign in with SSO］（認証エラーの中、SSO につながるプロファイルのとき）。`components/SsoLoginButton.tsx`。

### SsoLoginModal
SSO ログインのダイアログ（状態の文、URL のリンク、コード、中断と閉じる）。`components/SsoLoginModal.tsx`。

### LogAnalyticsPage
Log Analytics の画面全体（ロググループの選択、期間、クエリ欄、実行と中断、進み具合、結果の表、失敗の一覧、例のクエリの差し替えダイアログ）。`components/LogAnalyticsPage.tsx`。頼る：LogGroupMultiselect、QueryEditor、QueryResultTable、FailureList、ElapsedTime、AnalyticsHelpers、DateTimeHelpers。

### QueryEditor
Cloudscape の CodeEditor（Ace）。問題の下線、Cmd/Ctrl+Enter の実行、補完。`components/QueryEditor.tsx`、`aceLoader.ts`。頼る：AnalyticsHelpers。

### QueryResultTable
結果の仮想スクロールの表。`runId` と `holdingsEpoch` で `get_query_rows` / `get_query_row_detail` を引き、開いた行の詳細。`components/QueryResultTable.tsx`。頼る：ApiBridge。

### LogGroupMultiselect
最大 50 のロググループの Multiselect（名前の一部で検索）。`components/LogGroupMultiselect.tsx`。

### AnalyticsHelpers
純粋な補助：ページの種類、問題の言葉と範囲、構文の色分けの規則、補完の項目、ショートカットの修飾キー。`analytics.ts`、`platform.ts`。

### DateTimeHelpers
純粋な補助：日付と時刻の欄の分割・結合・補完・検査、理由の欄の割り当て、1 行表示。`dateTime.ts`、`format.ts`。

### ScreenHooks
画面だけの状態と計測：開いた行（`expansionState.ts`）、ペインの幅（`localStorage`）、色の様式、デバウンス、Escape、初回フォーカス、行の配置と窓、位置の問い合わせ。`hooks/*.ts`、`designTokens.ts`。

### MessageCatalog
日英の対訳表 `messages`、`detectLocale`、`translate`（`{name}` の置き換えだけ。数による単複の切り替えはない）、`createTranslator`。`i18n/messages.ts`。

### PerfRowsHarness
速度測定用の生成行（`VITE_LOCAL_SIGHTS_PERF_ROWS`）。本番では動かない。`dev/perfRows.ts`。

## テストの道具・スクリプト・CI

### FakeGateway
結合テストの偽のゲートウェイ。ロググループ・ストリーム・ページの台本、呼び出しの記録、応答の遅れと同時数、答えないストリーム。台本より多い呼び出しは `expect` で落ちる。`crates/local-sights-core/tests/support/fake_gateway.rs`。

### RecordingSink
`FetchSink` の呼ばれ方を記録する偽の sink。`crates/local-sights-core/tests/support/recording_sink.rs`。

### ReleaseScripts
版の一致の検査（`check-version.mjs`、`versions.ts`）、アドホック署名のワークフローの検査（`signing.ts`）、アイコンの PNG 描画（`render-icon.mjs`）。`scripts/release/`、`scripts/render-icon.mjs`。

### InsightsExportScript
Logs Insights の結果を手で書き出す bash スクリプト（`start-query` / `get-query-results`、リポジトリの外のフォルダへ）。`scripts/compat/insights-export.sh`。

### CiWorkflows
`ci.yml`（4 ジョブ、Linux / macOS / Windows）、`release.yml`（タグで `.dmg` と NSIS を 1 つの Release に）、`release-dry-run.yml`、`dependabot.yml`、`deny.toml`。`.github/`、`deny.toml`。
