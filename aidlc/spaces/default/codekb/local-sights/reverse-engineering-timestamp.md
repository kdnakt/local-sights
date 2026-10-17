# Reverse Engineering Timestamp

## Run Record

- Date: 2026-10-17
- Commit: 2760b13b8bc5b6daba36a6c283b3958c974a5a2d

この回は、intent 261017-insights-query（IB-11「Logs Insights 互換のクエリ」）のためのリポジトリ全体の走査し直し（全面走査）である。前回の知識ベース（2026-10-12、コミット `9d08cbf`、intent 261012-parallel-fetch 向け）は古くなっていたため、向きをつかむためだけに読み、9 つの成果物はすべてこの回に読んだコードから書き直した。前回からの主な変化は、取得の高速化（PR #22：同時取得・呼び出しの速さの調整・あきらめの決まりの移動・経過時間・[Cancel]）、Windows 11（x64）対応（PR #23・#24：AWS CLI のジョブオブジェクト、`ShellExecuteW`、NSIS のインストーラ、版 `0.2.0`）、SSO ログインの再試行（PR #21）、行カバレッジ 80% の CI（PR #20）である。

特に詳しく読んだのは、今の絞り込み（`filter.rs`、`log_view.rs`）、タイムライン（`timeline.rs`、`event.rs`）、取得条件とその流れ（`request.rs`、`time_range.rs`、`session.rs` の取得と絞り込みの部分、`coordinator.rs`）、キャッシュ（`cache/`）、Tauri のコマンドとイベントの境界（`src-tauri/src/lib.rs`、`capabilities/default.json`）、画面の絞り込みの欄と一覧（`api.ts`、`App.tsx`、`LogFilterInput.tsx`、`LogTable.tsx`、`useRowWindow.ts`）、OS ごとのコード（`cfg(unix)`／`cfg(windows)`）の位置、テストとカバレッジの設定（`ci.yml`、`vitest.config.ts`）である。それ以外は、モジュールの説明・公開項目の一覧・設定ファイルの単位で全体を押さえた。テスト・ビルド・lint は実行しておらず、AWS にもつないでいない。ビルド出力（`target/`、`dist/`、`node_modules/`、`src-tauri/gen/`）はこの作業場に無く、開いていない。AI-DLC 自身の導入物は対象外とした。

`shallow.paths` は、`./` の中でも説明と公開項目の一覧だけを見た場所の記録である（人が読むためのもので、道具は比べない）。

## Scope of Analysis

```yaml
scope_version: 1
kind: full
intent: insights-query
fingerprint: 841cd2625f68ca3209ceb262a864c4ba656df9f3
analyzed:
  paths:
    - ./
  components:
    - FetchCoordinator
    - EventFetcher
    - StreamPlanner
    - StreamSchedule
    - CallPacer
    - RetryPolicy
    - FetchStats
    - FetchStatus
    - PageCursor
    - LogEvent
    - EventTimeline
    - LogView
    - FilterEngine
    - ApiFailure
    - CloudWatchLogsGateway
    - AwsCloudWatchLogsGateway
    - ErrorClassifier
    - LogCache
    - CachePlan
    - CacheSettings
    - AppSession
    - ConnectionState
    - ConnectionCatalog
    - LogGroupBrowser
    - FetchRequest
    - TimeRangeModel
    - TimeZoneContext
    - DateTimeInput
    - SsoLoginPlanner
    - FetchCheck
    - TauriApp
    - TauriSink
    - SsoCliRunner
    - BrowserOpener
    - ApiBridge
    - AppShell
    - ConditionArea
    - LogFilterInput
    - StatusLine
    - ElapsedTime
    - CancelNotice
    - FetchErrorBanner
    - FailureList
    - LogTable
    - LogTableCloudscape
    - ExpandedMessage
    - TopBar
    - LogGroupPane
    - PaneResizer
    - SettingsDialog
    - CloseConfirmDialog
    - ConfirmDialog
    - SsoLoginModal
    - ScreenHooks
    - MessageCatalog
    - PerfRowsHarness
    - FakeGateway
    - RecordingSink
    - ReleaseScripts
    - CiWorkflows
shallow:
  paths:
    - crates/local-sights-core/src/fetcher.rs
    - crates/local-sights-core/src/streams/
    - crates/local-sights-core/src/retry.rs
    - crates/local-sights-core/src/pacing.rs
    - crates/local-sights-core/src/schedule.rs
    - crates/local-sights-core/src/gateway/aws.rs
    - crates/local-sights-core/src/catalog/
    - crates/local-sights-core/src/connection.rs
    - crates/local-sights-core/src/log_groups/
    - crates/local-sights-core/src/sso_login.rs
    - crates/local-sights-core/src/time_zone.rs
    - crates/local-sights-core/src/date_input.rs
    - crates/local-sights-core/src/cache/settings.rs
    - crates/local-sights-core/tests/
    - src-tauri/src/sso_cli/
    - src-tauri/src/browser.rs
    - src-tauri/permissions/
    - src-tauri/icons/
    - src/hooks/
    - src/i18n/
    - src/dev/
    - scripts/
    - docs/
```
