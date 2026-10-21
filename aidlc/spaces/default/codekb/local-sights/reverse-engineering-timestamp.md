# Reverse Engineering Timestamp

## Run Record

- Date: 2026-10-20
- Commit: 3208713208aaecd256146304c857771017cd7097
- 走査時の HEAD は `8bb3759`（本作業の AI-DLC 記録を加えただけのコミット）で、コードは `3208713` と同じ。
- 種類：全面の読み直し（`kind: full`）。既存の codekb（2026-10-17、コミット `2760b13`）は見出しの確認だけに使い、内容は今回のコードから書き直した。
- 実行したもの：フィンガープリントの mint と check だけ。テスト・ビルド・lint は実行せず、AWS にはつないでいない。
- 深く読んだ場所と流し読みの場所は `developer-scan.md`（`inception/reverse-engineering/`）の Scan Coverage にある。

## Scope of Analysis

```yaml
scope_version: 1
kind: full
intent: carryover-fixes
fingerprint: 849d25c46e5b74b31c5fb96b42a5cc9a3af357d5
analyzed:
  paths:
    - ./
    - Cargo.toml
    - deny.toml
    - package.json
    - vitest.config.ts
    - vite.config.ts
    - tsconfig.json
    - eslint.config.js
    - .gitattributes
    - .github/workflows/
    - .github/dependabot.yml
    - crates/local-sights-core/Cargo.toml
    - crates/local-sights-core/src/
    - crates/local-sights-core/tests/u5_cancel.rs
    - crates/local-sights-core/tests/support/
    - src-tauri/Cargo.toml
    - src-tauri/tauri.conf.json
    - src-tauri/build.rs
    - src-tauri/capabilities/default.json
    - src-tauri/src/
    - src/api.ts
    - src/App.tsx
    - src/main.tsx
    - src/elapsed.ts
    - src/i18n/messages.ts
    - src/components/CancelNotice.tsx
    - src/components/TopBar.tsx
    - src/commandRegistration.test.ts
    - README.md
    - docs/
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
    - RegionCatalog
    - LogGroupBrowser
    - FetchRequest
    - TimeRangeModel
    - TimeZoneContext
    - DateTimeInput
    - SsoLoginPlanner
    - InsightsSession
    - QueryParser
    - QueryPlan
    - QueryEvaluator
    - EventFieldReader
    - NaturalSort
    - DateTextResolver
    - QueryStore
    - QueryRunner
    - FetchCheck
    - CompatCheck
    - TauriApp
    - TauriSink
    - SsoCliRunner
    - BrowserOpener
    - ApiBridge
    - AppShell
    - AppNavigation
    - TopBar
    - ConditionArea
    - LogGroupPane
    - PaneResizer
    - LogFilterInput
    - LogTable
    - LogTableCloudscape
    - ExpandedMessage
    - StatusLine
    - ElapsedTime
    - CancelNotice
    - FetchErrorBanner
    - ErrorMessage
    - FailureList
    - SettingsDialog
    - CloseConfirmDialog
    - ConfirmDialog
    - SsoLoginButton
    - SsoLoginModal
    - LogAnalyticsPage
    - QueryEditor
    - QueryResultTable
    - LogGroupMultiselect
    - AnalyticsHelpers
    - DateTimeHelpers
    - ScreenHooks
    - MessageCatalog
    - PerfRowsHarness
    - FakeGateway
    - RecordingSink
    - ReleaseScripts
    - InsightsExportScript
    - CiWorkflows
shallow:
  paths:
    - crates/local-sights-core/examples/
    - crates/local-sights-core/tests/
    - src/components/
    - src/hooks/
    - src/dev/
    - src/test/
    - scripts/
    - src-tauri/permissions/
    - src-tauri/icons/
```
