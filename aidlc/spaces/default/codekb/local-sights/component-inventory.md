# Component Inventory

> リポジトリ `local-sights` の部品一覧。2026-10-12 の全体スキャン（コミット `9d08cbf`）に基づく。各見出しが部品名（`reverse-engineering-timestamp.md` の `components:` と一致）。ファイル分類は `code-structure.md`、依存の全体像は `dependencies.md` を参照。

## Core library（`crates/local-sights-core/`）

### CloudWatchLogsGateway
- **場所**: `src/gateway/mod.rs`
- **責務**: AWS に触れる唯一の境界（trait）。`get_log_events`、`describe_log_groups`、`describe_log_streams` の読み取り 3 操作だけを持つ。要求／ページ型と API 名定数も定義。
- **依存**: `failure`（`ApiFailure`）、`log_groups`（`LogGroup`）、`streams`（`LogStream`）
- **利用側**: `EventFetcher`、`StreamPlanner`、`LogGroupBrowser`、`FetchCheck`、テストの偽ゲートウェイ

### AwsCloudWatchLogsGateway
- **場所**: `src/gateway/aws.rs`、`src/gateway/classify.rs`
- **責務**: `CloudWatchLogsGateway` の AWS SDK 実装。SDK 既定の認証チェーンに任せ、接続先（プロファイルの既定リージョン）を解決し、SDK エラーを `ErrorSignal` → `FailureKind` に分類して安全な詳細だけを返す。SDK クライアントをキャッシュ（SSO 成功時に破棄）。
- **依存**: aws-config、aws-sdk-cloudwatchlogs、aws-credential-types、`ApiFailure`
- **利用側**: `TauriApp`、`FetchCheck`

### ApiFailure
- **場所**: `src/failure.rs`
- **責務**: 失敗の種別（`FailureKind`）、`retryable`、許可リストのフィールド（種別、API 名、リクエスト ID、プロファイル名、ロール ARN、アカウント ID、ロググループ、ストリーム）だけから組む `safeDetail`、アクセスキー ID らしき文字列のマスク。
- **依存**: regex、serde
- **利用側**: core 全体、`TauriApp`、画面（`ApiFailure` 型）

### FetchRequestValidation
- **場所**: `src/request.rs`
- **責務**: `validate_fetch_input`。アプリと `fetch_check` が共用する取得条件の検証。失敗は文言カタログのキー。
- **依存**: `time_range`、`catalog`（`ProfileSelector`）
- **利用側**: `AppSession`、`FetchCheck`

### TimeRangeModel
- **場所**: `src/time_range.rs`、`src/time_zone.rs`、`src/date_input.rs`
- **責務**: `yyyy/mm/dd hh:mm:ss` の解析と整形、Local（`TimeZoneContext` 境界、DST の存在しない時刻・重複する時刻の扱い）／UTC の切替、取得範囲（終了秒の 999 ミリ秒まで）、入力 1 つの状態 `DateTimeInput`。
- **依存**: chrono、chrono-tz、iana-time-zone
- **利用側**: `AppSession`、`FetchCheck`（`parse_utc_seconds`、`format_utc_millis`）

### EventTimeline
- **場所**: `src/event.rs`、`src/timeline.rs`
- **責務**: 取得中の全イベントを常に `(timestamp, logStreamName, sequence)` 順に保持し、ページを到着順にマージ。表示域の行と位置を答える。
- **依存**: `event`
- **利用側**: `LogView`、`FetchCheck`

### RetryPolicy
- **場所**: `src/retry.rs`、`src/paging.rs`
- **責務**: どの失敗を再試行するか、待ち時間（ランダムな広がり）、上限（要求ごと最大 5 回）、`AbortSignal`。`Retrier` が 1 回の API 呼び出しを包む。`paging.rs` はページ終端（同じ forward token）の判定。
- **依存**: tokio（`time`）、fastrand
- **利用側**: `EventFetcher`、`StreamPlanner`、`TauriApp`、`FetchCheck`

### EventFetcher
- **場所**: `src/fetcher.rs`
- **責務**: 1 ストリームを最後のページまで読む。スロットリング・通信失敗は再試行し、使い切りは `StreamFetchOutcome` で報告。
- **依存**: `CloudWatchLogsGateway`、`RetryPolicy`、`paging`
- **利用側**: `FetchCoordinator`

### StreamPlanner
- **場所**: `src/streams/mod.rs`、`src/streams/planner.rs`
- **責務**: どのストリームを取るか。範囲判定、最終イベントが開始の 1 時間以上前のストリームでの早期停止、ページ終端、エラー時の部分結果（純粋判断）と、`DescribeLogStreams` を新しい順に読み進める流れ。
- **依存**: `CloudWatchLogsGateway`、`RetryPolicy`
- **利用側**: `FetchCoordinator`

### FetchCoordinator
- **場所**: `src/coordinator.rs`
- **責務**: 1 回の取得全体。一覧→選択→1 ストリームずつ取得→ページを `TimelineStore` へ→全成功ならキャッシュ保存。進捗は `FetchSink` へ。失敗が 3 ストリーム続いたら残りを失敗とする。`run_fetch` / `run_fetch_with_cache`。
- **依存**: `StreamPlanner`、`EventFetcher`、`LogCache`、`EventTimeline`／`LogView`（`TimelineStore`）、`ApiFailure`
- **利用側**: `TauriApp`、`FetchCheck`、統合テスト

### LogGroupBrowser
- **場所**: `src/log_groups/mod.rs`、`src/log_groups/listing.rs`
- **責務**: 1 接続のロググループ一覧。ページング、名前順、部分結果、古い応答の破棄、大文字小文字を無視した絞り込み、空状態（純粋判断）と、`DescribeLogGroups` を全ページ読んで `ListingSink` に渡す流れ。
- **依存**: `CloudWatchLogsGateway`
- **利用側**: `AppSession`、`TauriApp`

### FilterEngine
- **場所**: `src/filter.rs`
- **責務**: 保持イベントをメッセージの部分一致（大文字小文字無視）で絞る。結果は順序キーで保持し、追加でずれない。走査チケット（`ScanTicket`）で古い走査を止め、進捗報告の要否を判断。
- **依存**: `event`
- **利用側**: `LogView`

### LogView
- **場所**: `src/log_view.rs`
- **責務**: `EventTimeline` と `FilterEngine` を 1 つのロックに束ねる。取得の `TimelineStore` として、追加時にフィルタ判定、破棄時に結果も空にする。`RowKey` と `RowPositions`、`discardGeneration`。
- **依存**: `EventTimeline`、`FilterEngine`
- **利用側**: `TauriApp`、`FetchCoordinator`

### LogCache
- **場所**: `src/cache/plan.rs`、`src/cache/mod.rs`、`src/cache/settings.rs`
- **責務**: 任意のディスクキャッシュ。記録してよい範囲（取得開始 5 分前まで、全ストリーム成功時のみ）、範囲の結合、ヒット判定（純粋判断）。キーの SHA-256 名の JSON Lines ファイルの読み書き（所有者のみ、一時ファイル経由）。設定ファイル `settings.json`（`cacheEnabled` のみ）。
- **依存**: serde_json、sha2、tempfile（テスト）
- **利用側**: `FetchCoordinator`、`AppSession`、`TauriApp`

### ConnectionCatalog
- **場所**: `src/catalog/mod.rs`、`src/catalog/files.rs`、`src/catalog/regions.rs`、`src/connection.rs`
- **責務**: プロファイル一覧（`~/.aws/config` / `credentials` のセクション名、`region`、SSO 設定の有無、`source_profile` の名前だけを読む）、リージョンの静的一覧、接続選択の状態機械（表示中のログがあるときだけ確認）。
- **依存**: home、`ProfileSelector`
- **利用側**: `AppSession`、`TauriApp`（`load_catalog`）

### SsoLogin
- **場所**: `src/sso_login.rs`
- **責務**: SSO ログインの純粋な判断。[Sign in with SSO] を出す条件（認証エラー + SSO プロファイル、`source_profile` を最大 5 段辿る）、CLI 引数、出力行から取り出してよい URL（`https://` の `*.amazonaws.com` / `*.awsapps.com`）とコード、`MAX_LINE_BYTES`、`SsoLoginOutcome`。プロセス・ファイル・AWS には触れない。
- **依存**: `catalog`
- **利用側**: `AppSession`、`SsoCli`、`TauriApp`

### AppSession
- **場所**: `src/session.rs`
- **責務**: 画面が表示するすべての状態。入力・検証、接続と一覧、取得の進捗と結果、タイムゾーン、フィルタ要約、キャッシュ設定とダイアログ、終了確認、SSO ログインの状態。`SessionView` を直列化して返す。
- **依存**: `TimeRangeModel`、`ConnectionCatalog`、`LogGroupBrowser`、`LogCache`、`SsoLogin`、`ApiFailure`、`FetchRequestValidation`
- **利用側**: `TauriApp`

### FetchCheck
- **場所**: `examples/fetch_check.rs`
- **責務**: 開発者専用。同じ検証と `FetchCoordinator`、SDK 実装で実 AWS からロググループ全体を取り、標準出力に時刻順で出す。`cargo test` と CI は実行しない。
- **依存**: `FetchCoordinator`、`AwsCloudWatchLogsGateway`、`FetchRequestValidation`、`RetryPolicy`、`EventTimeline`、`TimeRangeModel`
- **利用側**: 開発者の手元（team.md Walking Skeleton の確認 2）

### CoreTestSupport
- **場所**: `tests/support/fake_gateway.rs`、`tests/support/recording_sink.rs`、`tests/u1_fetch_flow.rs`、`tests/u2_log_group_listing.rs`、`tests/u3_fetch_flow.rs`、`tests/u5_filter_flow.rs`、`tests/u6_cache_flow.rs`
- **責務**: 偽ゲートウェイ（台本どおりに応答し要求を記録）、記録シンク、統合テスト 5 本。AWS には接続しない。
- **依存**: `CloudWatchLogsGateway`、`FetchSink`、tokio `test-util`、tempfile

## Tauri app（`src-tauri/`）

### TauriApp
- **場所**: `src/lib.rs`、`src/main.rs`、`build.rs`、`tauri.conf.json`、`capabilities/default.json`、`permissions/autogenerated/`
- **責務**: 25 個の Tauri コマンドと 3 イベントの配線、`AppState`（session、`LogView`、取得の中止ハンドル、SSO 制御）、ロック順序（session → LogView）、起動時の一覧開始、フィルタ走査スレッド、設定の非同期 I/O、メニュー（Cmd+Q）、`CloseRequested` / `ExitRequested` での終了確認、`AWS_PROFILE` と OS タイムゾーンの読み取り、キャッシュ場所の決定。判断は core に委ねる。
- **依存**: tauri、`AppSession`、`FetchCoordinator`、`LogView`、`AwsCloudWatchLogsGateway`、`ConnectionCatalog`、`SsoLogin`、`SsoCli`、serde
- **利用側**: 画面（IPC）

### SsoCli
- **場所**: `src/sso_cli.rs`
- **責務**: AWS CLI の発見（`/opt/homebrew/bin/aws`、`/usr/local/bin/aws`。`PATH` は使わない）と `aws sso login` の実行。シェル無し、stdin 閉、別プロセスグループ、stdout/stderr を行単位で読み `extract_login_hints` に渡して捨てる。キャンセル・5 分のタイムアウト・終了で SIGTERM → 2 秒後 SIGKILL（`libc::killpg`）。Unix 専用。偽 CLI スクリプトによるテスト 11 件。
- **依存**: libc、`SsoLogin`
- **利用側**: `TauriApp`

## Screen（`src/`）

### ScreenApi
- **場所**: `src/api.ts`
- **責務**: Tauri コマンドの薄いラッパー 25 個、イベント購読 3 種、`SessionView` ほかの型定義、`withProgress` / `withFilter` / `mergeSessionView`（版の逆転を防ぐ純粋関数）、`isSsoLoginRunning` / `isSsoLoginModalShown`。計測ハーネス（`perfRows`）が有効なときは `getRows` などを生成行に差し替える。テストは `vi.mock` で丸ごと差し替える。
- **依存**: @tauri-apps/api、`dev/perfRows`
- **利用側**: `App`、`ScreenComponents`

### App
- **場所**: `src/App.tsx`、`src/main.tsx`、`index.html`、`src/styles.css`
- **責務**: 起点（Cloudscape のグローバルスタイル、i18n、デザイントークン適用）、`SessionView` の購読と 13 部品の組み立て、ダイアログの排他、`LOG_TABLE_VARIANT` による計測用テーブル切替。
- **依存**: react、@cloudscape-design/*、`ScreenApi`、`ScreenComponents`、`ScreenHooks`、`Messages`
- **利用側**: 利用者

### ScreenComponents
- **場所**: `src/components/`（`TopBar`、`LogGroupPane`、`ConditionArea`、`LogFilterInput`、`LogTable`、`LogTableCloudscape`、`ExpandedMessage`、`StatusLine`、`FailureList`、`FetchErrorBanner`、`ErrorMessage`、`ConfirmDialog`、`CloseConfirmDialog`、`SettingsDialog`、`SsoLoginButton`、`SsoLoginModal`、`PaneResizer`）
- **責務**: Cloudscape 部品で描く表示部品。`LogTable` はアプリ自身の仮想スクロール（本番）、`LogTableCloudscape` は計測用プロトタイプ。各部品に同名の `*.test.tsx`。
- **依存**: @cloudscape-design/components、`ScreenHooks`、`ScreenLogic`、`Messages`
- **利用側**: `App`

### ScreenHooks
- **場所**: `src/hooks/`（`useColorScheme`、`useDebouncedValue`、`useEscapeKey`、`useInitialFocus`、`usePaneWidth`、`useRowLayout`、`useRowPositions`、`useRowWindow`、`expansionState`）
- **責務**: OS のライト／ダーク追従、デバウンス、Escape、初期フォーカス、ペイン幅（`localStorage`）、行配置と表示域、展開状態（最大 10,000 行）。
- **依存**: react、`ScreenApi`（`getRows`、`rowPositions`）
- **利用側**: `App`、`ScreenComponents`

### ScreenLogic
- **場所**: `src/dateTime.ts`、`src/rowLayout.ts`、`src/virtualScroll.ts`、`src/errorText.ts`、`src/designTokens.ts`、`src/format.ts`
- **責務**: 画面側の純粋ロジック。時刻入力の補完（`hh:mm` → `:00` / `:59`）、行配置、仮想スクロールの窓、エラー文の組み立て、Cloudscape トークンの CSS 変数化、1 行化。
- **依存**: なし（`designTokens` は @cloudscape-design/design-tokens）
- **利用側**: `ScreenComponents`、`ScreenHooks`

### Messages
- **場所**: `src/i18n/messages.ts`
- **責務**: 日本語／英語の文言カタログ、`detectLocale`、`createTranslator`。core が返すキーを文言にする。
- **依存**: なし
- **利用側**: `App`、`ScreenComponents`

### PerfRowsHarness
- **場所**: `src/dev/perfRows.ts`
- **責務**: 開発ビルド専用の計測ハーネス。`VITE_LOCAL_SIGHTS_PERF_ROWS` / `VITE_LOCAL_SIGHTS_PERF_GROW` / `VITE_LOCAL_SIGHTS_LOG_TABLE` で生成行を `getRows` 境界に流す。本番ビルドでは無効。
- **依存**: なし
- **利用側**: `ScreenApi`、`App`

### ScreenTestSupport
- **場所**: `src/test/setup.ts`、`src/test/fixtures.ts`、`src/commandRegistration.test.ts`、各 `*.test.ts(x)`
- **責務**: jest-dom の登録と `cleanup`、`SessionView` のフィクスチャ、コマンド登録の 4 か所の整合検査、部品・フック・ロジックのテスト（Vitest 282 件）。
- **依存**: vitest、@testing-library/*、jsdom

## Scripts, docs, workflows

### ReleaseVersionCheck
- **場所**: `scripts/release/versions.ts`、`scripts/release/check-version.mjs`、`scripts/release/versions.test.ts`
- **責務**: `Cargo.toml`（`[workspace.package]`）、`Cargo.lock`（`local-sights`、`local-sights-core`）、`package.json`、`src-tauri/tauri.conf.json` の版の一致と、`vMAJOR.MINOR.PATCH` タグの形式・照合。エラー文はファイル名・パッケージ名・版だけ（他の内容を漏らさない）。Node 22.18+ の型除去 import。
- **依存**: node:fs（`.mjs` のみ）
- **利用側**: `CiWorkflow`、`ReleaseWorkflow`、`ReleaseDryRunWorkflow`、開発者

### IconRenderer
- **場所**: `scripts/render-icon.mjs`、`src-tauri/icons/source/icon.svg`
- **責務**: SVG を 1024px PNG に描き、四隅と 100px の余白が透明・中央が不透明であることを検証して書く。Tauri の 6 アイコンは `tauri icon` で生成。
- **依存**: @resvg/resvg-js
- **利用側**: 開発者

### CiWorkflow
- **場所**: `.github/workflows/ci.yml`、`.github/dependabot.yml`
- **責務**: PR と `main` push で、`rust-core`（fmt、clippy、test）、`frontend`（check:version、tsc、eslint、prettier、vitest、vite build、`npm audit --audit-level=high`）、`cargo-deny`、`tauri-app`（ubuntu + macos で workspace clippy、`cargo build -p local-sights`、`cargo test -p local-sights`）。Dependabot は cargo / npm / github-actions を週次でグループ化。
- **依存**: GitHub Actions（SHA 固定）、`ReleaseVersionCheck`、`deny.toml`

### ReleaseWorkflow
- **場所**: `.github/workflows/release.yml`、`docs/releasing.md`
- **責務**: `v*` タグで `verify`（タグ形式、版一致、`main` 上）→ `test` → `build-and-publish`（macos-latest の arm64 確認、`npm run tauri build -- --bundles dmg`、`.dmg` が 1 つかつ `aarch64`、`Local-Sights_<ver>_aarch64.dmg` に改名、`gh release create` または `--clobber`）。`contents: write` は公開ジョブのみ。Apple 証明書・AWS 認証情報は使わない。ビルドステップは現状、署名の環境変数を渡していない（本インテントの変更対象: `APPLE_SIGNING_IDENTITY="-"`）。`docs/releasing.md` が手順を説明。
- **依存**: GitHub Actions、Tauri CLI、`ReleaseVersionCheck`、`gh`

### ReleaseDryRunWorkflow
- **場所**: `.github/workflows/release-dry-run.yml`
- **責務**: 手動実行で同じ `.dmg` ビルドを公開せずに行い、artifact `local-sights-dmg-dry-run` として 7 日保持。`ReleaseWorkflow` とビルドステップが重複している（署名を足すなら両方）。
- **依存**: GitHub Actions、Tauri CLI、`ReleaseVersionCheck`

### InstallGuide
- **場所**: `docs/install.md`、`README.md`（Install 節）
- **責務**: 配布された `.dmg` のインストール（`/Applications` または `~/Applications`）、会社の規則の確認、初回起動の Gatekeeper 警告の開け方（macOS 15 では **Done** → システム設定 **Open Anyway**、14 以前は Control クリック → **Open**、「壊れている」と出た場合の `xattr -dr com.apple.quarantine`）、版の確認、更新、戻し方、キャッシュ／設定の場所。本インテントで初回起動の節を実際のダイアログに合わせて更新する対象。
- **依存**: なし
- **利用側**: 会社の PC の利用者
