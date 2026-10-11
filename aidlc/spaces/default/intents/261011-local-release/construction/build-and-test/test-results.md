# Test Results — リリース準備

実行日：2026-10-11。実行環境：Linux のコンテナ、Node v22.22.0、Rust stable。

## ビルド

| コマンド | 結果 |
|----------|------|
| `npm run build`（`tsc --noEmit && vite build`） | 成功（`✓ built in 375ms`） |
| `npm run check:version` | 成功：`Version 0.1.0 is the same in Cargo.toml, Cargo.lock, package.json, src-tauri/tauri.conf.json.` |
| `npm run check:version -- --tag v0.1.0` | 成功：`... and matches tag v0.1.0.` |
| `npm run check:version -- --tag v0.2.0` | 想定どおり失敗（終了コード 1）：`Tag v0.2.0 does not match version 0.1.0 in the files.` |
| `npm run check:version -- --tag v1.2` | 想定どおり失敗（終了コード 1）：`Tag v1.2 is not of the form vMAJOR.MINOR.PATCH; ...` |
| `cargo build -p local-sights`（アプリ本体） | 実行していない。コンテナに WebKitGTK の開発用パッケージがない（`pkg-config --exists webkit2gtk-4.1` が偽）。CI の `tauri-app` ジョブ（ubuntu・macOS）と、リリース時の macOS の `tauri build` で確かめる。Rust のコードはこの作業で変えていない |
| `npm run tauri build -- --bundles dmg` | 実行していない（macOS が必要）。タグの push のときに確かめる |

## テスト

| コマンド | 合計 | 成功 | 失敗 | スキップ |
|----------|------|------|------|----------|
| `npx vitest run scripts/release/versions.test.ts src/App.title.test.tsx`（この作業のテスト） | 24 | 24 | 0 | 0 |
| `npx vitest run`（画面側の全体） | 195（23 ファイル） | 195 | 0 | 0 |
| `cargo test -p local-sights-core`（ライブラリの全体） | 346 | 346 | 0 | 5（ignored、既存の性能確認） |

`cargo test` の内訳：単体 305、統合 11・9・14・4・3、doc 0。Code Generation の前のベースライン（Vitest 171 件、cargo 同数）からの減少はない。

## 静的検査・その他

| コマンド | 結果 |
|----------|------|
| `npx prettier --check .` | 成功 |
| `npx eslint .` | 成功 |
| `npx tsc --noEmit` | 成功 |
| `cargo fmt --all --check` | 成功 |
| `npm audit --audit-level=high` | 0 件 |
| `actionlint .github/workflows/release.yml .github/workflows/ci.yml`（v1.7.7） | 問題なし |
| `grep 'secrets\.' .github/workflows/release.yml` | 0 件 |
| `grep 'contents: write' .github/workflows/*.yml` | `release.yml` の 85 行目（`build-and-publish`）だけ |

## 失敗の詳細

なし。

## カバレッジ

数値の下限はない（team.md）。`scripts/release/versions.ts` の公開関数 5 つ（`parseTag`、`readCargoWorkspaceVersion`、`readCargoLockVersions`、`readJsonVersion`、`checkVersions`）はすべてテストがある。カバレッジの計測は行っていない（計測の設定はこのリポジトリにない）。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|-----------|--------|----------|--------|----------|--------------|---------|
| TC-STRATEGY | code-generation-plan.md › Testing Contract › obligations.strategy_volume | 部品ごとに 5〜8 件の単体テスト＋主要な境界の結合テスト | 版の検査 21 件、表示名 3 件。境界（入口と実ファイル、ワークフロー定義）を結合として確認 | 上の「テスト」と「ビルド」の表、integration-test-instructions.md | build-and-test | Met |
| TC-SCOPE-REGRESSION | Testing Contract › obligations.scope_floor[0] | 対象の振る舞いを狙った回帰テストがある | 版の不一致・タグの形の誤りを狙ったテストがあり、入口でも失敗を確認 | `scripts/release/versions.test.ts`、`check:version -- --tag v0.2.0 / v1.2` | build-and-test | Met |
| TC-SCOPE-GREEN | Testing Contract › obligations.scope_floor[1] | 既存のテストがすべて通る | Vitest 195/195、cargo 346/346 | 上の「テスト」の表 | build-and-test | Met |
| TC-TEAM-PUBLIC-FN | Testing Contract › applicable_notes（team）「ライブラリ側の公開関数には必ずテストを書く」 | `versions.ts` の公開関数すべてにテスト | 5/5 | `scripts/release/versions.test.ts` | build-and-test | Met |
| TC-TEAM-NO-AWS | Testing Contract › applicable_notes（team）「自動テストと CI では実際の AWS に接続しない」 | テストとワークフローに AWS の接続・認証情報がない | 0 件 | security-test-instructions.md の grep | build-and-test | Met |
