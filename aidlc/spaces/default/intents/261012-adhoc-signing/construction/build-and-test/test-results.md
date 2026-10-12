# Test Results — リリースの .app をアドホック署名する

実行日：2026-10-12。環境：Linux（この作業環境）、Node v22.22.0、cargo 1.97.0。

## ビルド

| コマンド | 結果 |
|---|---|
| `npm ci` | 成功（`EBADENGINE` の警告のみ） |
| `npm run check:version` | 成功：`Version 0.1.0 is the same in Cargo.toml, Cargo.lock, package.json, src-tauri/tauri.conf.json.` |
| `npx vite build` | 成功（`✓ built in 1.66s`） |
| `release.yml` の YAML 構文（`yaml.safe_load`） | 成功。`build-and-publish` の step 順：`build the .dmg (ad-hoc signed)` → `the .app carries a valid ad-hoc signature` → `exactly one Apple Silicon .dmg` → `publish the Release ...` |

## テスト

| コマンド | 合計 | 成功 | 失敗 | スキップ |
|---|---|---|---|---|
| `npx vitest run`（画面と `scripts/`） | 320（33 ファイル） | 320 | 0 | 0 |
| うち `npx vitest run scripts/release/signing.test.ts`（このユニット） | 24 | 24 | 0 | 0 |
| `cargo test -p local-sights-core` | 404 | 404 | 0 | 5（`ignored`、既存） |

Rust 側（`cargo test -p local-sights`、Tauri アプリのテスト）はアプリのコードを変えていないため、この環境では実行していない（GTK／WebKit が要る）。CI の `tauri-app` ジョブが実行する。

## 静的検査

| コマンド | 結果 |
|---|---|
| `npx tsc --noEmit` | 成功 |
| `npx eslint .` | 成功（警告なし） |
| `npx prettier --check .` | 成功 |
| `cargo fmt --all --check` | 成功 |
| `npm audit --audit-level=high` | 成功（0 件） |
| `cargo clippy` / `cargo deny` | この環境では未実行（Rust の変更なし）。CI に任せる |

## 失敗の詳細

なし。

## カバレッジ

数値のカバレッジ下限は設けていない（team.md）。Minimal 戦略の「要件 1 つにつきテスト 1 件」は `scripts/release/signing.test.ts` の `describe("repository")` が満たす（FR1.1、FR1.3、FR1.4、FR2.1/2.2、FR3.1/3.4、FR3.2/3.3、FR3.5）。FR1.2（`tauri.conf.json` を変えない）は差分で確認、FR2.3・NFR1 は `security-test-instructions.md` の確認、NFR2 は上記のテスト結果、NFR3 は差分で `gh release upload --clobber` の分岐が変わっていないことを確認。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| N/A | `nfr-requirements/`・`nfr-design/`（このワークフローになし）、`code-generation-plan.md` の Testing Contract | 数値で測る品質の目標なし（Testing Contract はカバレッジの下限を定めず、「要件ごとに 1 テスト」と「既存のテストを緑に保つ」だけを求める。どちらも上記のとおり満たしている） | N/A | この文書の「テスト」「カバレッジ」 | build-and-test | N/A |

## 自動で確かめられないもの

- 署名検証ステップが macOS ランナーで通ること（A2、A3）と、Gatekeeper のダイアログが変わること（A1）。次のタグ付きリリースと、その後の会社の Mac での初回起動で確認する（`docs/releasing.md` 手順 7）。これは要件分析（Q3）で決めた確認方法であり、Build and Test の失敗ではない。
