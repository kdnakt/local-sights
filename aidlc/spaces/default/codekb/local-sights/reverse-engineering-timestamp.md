# Reverse Engineering Timestamp

## Run Record

- Date: 2026-10-25
- Commit: 793da172687f6273a32e128ebca7650d8b8b9a78
- 走査時の HEAD は `79081e4`（本作業の AI-DLC 記録を加えただけのコミット）で、コードは `793da17` と同じ。
- 種類：焦点を絞った読み直し（focused、`kind: partial`）。既存の知識ベース（2026-10-20、コミット `3208713`、`kind: full`）は STALE だったため、その後の 4 コミット（`5ced3bf`、`bd65e49`、`082dc92`、`a2dd083`）が触れた場所と、本作業（`stats` による集計と途中結果の表示）に効く場所を深く読み、他の節の記述は前回のものを保った。前回の `analyzed.paths`（`./` を含む全面）は再確認できないため `shallow.paths` に格下げした（`./` そのものは記録せず、前回の個々の場所を並べた）。
- 実行したもの：フィンガープリントの mint と check だけ。テスト・ビルド・lint は実行せず、AWS にはつないでいない。
- 深く読んだ場所と流し読みの場所は `developer-scan.md`（`inception/reverse-engineering/`）の Scan Coverage にある。

## Scope of Analysis

```yaml
scope_version: 1
kind: partial
intent: insights-stats
fingerprint: 0a6a01eb18277a4866c4542a558fed47f69c2f91
analyzed:
  paths:
    - crates/local-sights-core/src/lib.rs
    - crates/local-sights-core/src/query/mod.rs
    - crates/local-sights-core/src/query/plan.rs
    - crates/local-sights-core/src/query/parser.rs
    - crates/local-sights-core/src/query/evaluator.rs
    - crates/local-sights-core/src/query/runner.rs
    - crates/local-sights-core/src/query/store.rs
    - crates/local-sights-core/src/insights_session.rs
    - crates/local-sights-core/src/session.rs
    - crates/local-sights-core/src/export/mod.rs
    - crates/local-sights-core/src/export/exporter.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/export_boundary.rs
    - src-tauri/Cargo.toml
    - src-tauri/tauri.conf.json
    - src-tauri/capabilities/default.json
    - src/api.ts
    - src/main.tsx
    - src/analytics.ts
    - src/commandRegistration.test.ts
    - src/i18n/messages.ts
    - src/i18n/regions.ts
    - src/components/LogAnalyticsPage.tsx
    - src/components/QueryResultTable.tsx
    - src/components/ExpandedMessage.tsx
    - src/components/ExportMenu.tsx
    - src/components/ExportNoticeAlert.tsx
    - src/components/CancelNotice.tsx
    - Cargo.toml
    - deny.toml
    - package.json
    - README.md
  components:
    - QueryParser
    - QueryPlan
    - QueryEvaluator
    - QueryRunner
    - QueryStore
    - InsightsSession
    - AppSession
    - ExportJob
    - TauriApp
    - TauriSink
    - ExportBoundary
    - ApiBridge
    - AnalyticsHelpers
    - MessageCatalog
    - RegionNames
    - LogAnalyticsPage
    - QueryResultTable
    - ExpandedMessage
    - ExportMenu
    - ExportNoticeAlert
    - CancelNotice
shallow:
  paths:
    - crates/local-sights-core/Cargo.toml
    - crates/local-sights-core/src/
    - crates/local-sights-core/src/query/fields.rs
    - crates/local-sights-core/src/query/natural.rs
    - crates/local-sights-core/src/query/instant.rs
    - crates/local-sights-core/src/export/formatter.rs
    - crates/local-sights-core/src/cache/settings.rs
    - crates/local-sights-core/src/cache/plan.rs
    - crates/local-sights-core/src/coordinator.rs
    - crates/local-sights-core/tests/
    - crates/local-sights-core/tests/u5_cancel.rs
    - crates/local-sights-core/tests/support/
    - crates/local-sights-core/examples/
    - src-tauri/build.rs
    - src-tauri/src/
    - src-tauri/permissions/
    - src-tauri/icons/
    - src/App.tsx
    - src/jsonPretty.ts
    - src/elapsed.ts
    - src/components/
    - src/components/QueryEditor.tsx
    - src/components/SettingsDialog.tsx
    - src/components/TopBar.tsx
    - src/hooks/
    - src/dev/
    - src/test/
    - scripts/
    - docs/
    - .github/workflows/
    - .github/dependabot.yml
    - .gitattributes
    - vitest.config.ts
    - vite.config.ts
    - tsconfig.json
    - eslint.config.js
    - Cargo.lock
```
