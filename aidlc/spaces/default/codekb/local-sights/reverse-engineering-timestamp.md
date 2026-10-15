# Reverse Engineering Timestamp

## Run Record

- Date: 2026-10-12
- Commit: 9d08cbf5c6d97820f0ec7f276c3ce6020c903eb5

この回は、コード知識ベースが無い状態からのリポジトリ全体の走査（全面走査）である。intent 261012-parallel-fetch のために行い、取得経路（`crates/local-sights-core` の coordinator、fetcher、streams、retry、gateway、timeline、cache、session、`src-tauri` のコマンドとイベント、画面の状態行・条件欄・エラー表示・失敗一覧）を特に詳しく読んだ。それ以外の部分は、モジュールの説明・公開関数の一覧・設定ファイルの単位で全体を押さえた。ビルド出力（`target/`、`dist/`、`node_modules/`、`src-tauri/gen/schemas`）と AI-DLC 自身の導入物は開いていない。テスト・ビルド・lint は実行しておらず、AWS にも接続していない。

`shallow.paths` は、`./` の中でも説明と公開関数の一覧だけを見た場所の記録である（人が読むためのもの）。

## Scope of Analysis

```yaml
scope_version: 1
kind: full
intent: parallel-fetch
fingerprint: ba1943351d971557c348a452ac634ab60cc7d81b
analyzed:
  paths:
    - ./
  components:
    - FetchCoordinator
    - EventFetcher
    - StreamPlanner
    - RetryPolicy
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
    - ApiBridge
    - AppShell
    - ConditionArea
    - StatusLine
    - FetchErrorBanner
    - FailureList
    - LogTable
    - TopBar
    - LogGroupPane
    - SettingsDialog
    - CloseConfirmDialog
    - ConfirmDialog
    - SsoLoginModal
    - ScreenHooks
    - MessageCatalog
    - FakeGateway
    - RecordingSink
    - ReleaseScripts
    - CiWorkflows
shallow:
  paths:
    - crates/local-sights-core/src/filter.rs
    - crates/local-sights-core/src/catalog/
    - crates/local-sights-core/src/connection.rs
    - crates/local-sights-core/src/log_groups/
    - crates/local-sights-core/src/sso_login.rs
    - crates/local-sights-core/src/time_range.rs
    - crates/local-sights-core/src/time_zone.rs
    - crates/local-sights-core/src/date_input.rs
    - crates/local-sights-core/src/cache/settings.rs
    - src-tauri/src/sso_cli.rs
    - src-tauri/permissions/
    - src-tauri/icons/
    - src/hooks/
    - src/dev/
    - scripts/
```
