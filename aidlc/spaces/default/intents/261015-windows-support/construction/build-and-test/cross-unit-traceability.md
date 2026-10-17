# Cross-Unit Traceability — 261015-windows-support

## 判定

**PASS**。数え上げた ID は 35 件（`requirements.md` の FR の細目 29 件と NFR 6 件。user-stories のステージは実行していないため AC はない）。作業単位はなく、対応表はステージ単位の `construction/code-generation/traceability.json` の 1 つ。

| 区分 | 件数 | 中身 |
|------|------|------|
| OK | 34 | status `OK` の行があり、対象のファイルが存在する |
| Deferred | 1 | FR1.3（`main` のルールセットの必須チェックに `Tauri app (windows-latest)` を加える）。リポジトリの設定でコードではないため、利用者が GitHub の設定で行う。対象は `docs/releasing.md`（PR #24 の確認項目にも記載） |
| GAP / ORPHAN | 0 | — |

親の ID（FR1〜FR5）は細目の ID を通じて対応する。

## 指摘

| ID | 指摘 | 中身 |
|----|------|------|
| BT-TR-1 | FR1.3 が `Deferred` | コード生成のレビュー（review-01 の Major 1）と同じ。PR #24 をマージする前に、利用者が GitHub の Settings → Rules → Rulesets → main の Require status checks に `Tauri app (windows-latest)` を加える。加えた時点で満たされる |

## ID ごとの対応

| ID | 対応 | 対象 | 状態 |
|----|------|------|------|
| FR1.1 | OK | `.github/workflows/ci.yml`（あり） | OK |
| FR1.2 | OK | `.github/workflows/ci.yml`（あり） | OK |
| FR1.3 | Deferred | `docs/releasing.md`（あり） | 利用者の操作待ち（BT-TR-1） |
| FR1.4 | OK | `.gitattributes`（あり） | OK |
| FR1.5 | OK | `.github/workflows/ci.yml`（あり） | OK |
| FR2.1 | OK | `src-tauri/tauri.conf.json`（あり） | OK |
| FR2.2 | OK | `.github/workflows/release-dry-run.yml`（あり） | OK |
| FR2.3 | OK | `.github/workflows/release.yml`（あり） | OK |
| FR2.4 | OK | `.github/workflows/release-dry-run.yml`（あり） | OK |
| FR2.5 | OK | `.github/workflows/release.yml`（あり） | OK |
| FR2.6 | OK | `.github/workflows/release.yml`（あり） | OK |
| FR2.7 | OK | `docs/install.md`（あり） | OK |
| FR2.8 | OK | `src-tauri/tauri.conf.json`（あり） | OK |
| FR2.9 | OK | `src-tauri/tauri.conf.json`（あり） | OK |
| FR3.1 | OK | `README.md`（あり） | OK |
| FR3.2 | OK | `docs/install.md`（あり） | OK |
| FR3.3 | OK | `crates/local-sights-core/src/cache/mod.rs`（あり） | OK |
| FR3.4 | OK | `src-tauri/src/lib.rs`（あり） | OK |
| FR3.5 | OK | `src-tauri/src/lib.rs`（あり） | OK |
| FR3.6 | OK | `src-tauri/src/browser.rs`（あり） | OK |
| FR4.1 | OK | `src-tauri/src/sso_cli/mod.rs`（あり） | OK |
| FR4.2 | OK | `src-tauri/src/sso_cli/windows.rs`（あり） | OK |
| FR4.3 | OK | `src-tauri/src/sso_cli/windows.rs`（あり） | OK |
| FR4.4 | OK | `src-tauri/src/sso_cli/windows.rs`（あり） | OK |
| FR4.5 | OK | `src-tauri/src/browser.rs`（あり） | OK |
| FR4.6 | OK | `docs/install.md`（あり） | OK |
| FR5.1 | OK | `docs/install.md`（あり） | OK |
| FR5.2 | OK | `README.md`（あり） | OK |
| FR5.3 | OK | `docs/releasing.md`（あり） | OK |
| NFR1 | OK | `.github/workflows/ci.yml`（あり） | OK |
| NFR2 | OK | `src-tauri/src/sso_cli/mod.rs`（あり） | OK |
| NFR3 | OK | `crates/local-sights-core/src/cache/mod.rs`（あり） | OK |
| NFR4 | OK | `docs/install.md`（あり） | OK |
| NFR5 | OK | `src-tauri/src/sso_cli/mod.rs`（あり） | OK |
| NFR6 | OK | `README.md`（あり） | OK |
