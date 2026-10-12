# Cross-Unit Traceability — アプリから SSO ログインする

## 判定

**合格**。`inception/requirements-analysis/requirements.md` の FR 17 件と NFR 7 件のすべてが、`construction/code-generation/traceability.json`（作業単位を分けないワークフローの、ステージ単位の追跡）で `OK` になっており、対象のファイルはすべて存在する。User Stories のステージはこのワークフローで実行していないため、AC の ID はない。

ただし、追跡の対象ファイルがあることと、実機で動くことは別。FR1.1〜FR2.5 などの画面からの操作は、Tauri のコマンドが登録されていないため（R-01）、実機では動かない見込み。`build-and-test-summary.md` を見る。

## ID ごとの対応

作業単位：すべてステージ単位（`construction/code-generation/`）。

| ID | 状態 | 対象ファイル | ファイルの有無 |
|---|---|---|---|
| FR1.1 | OK | `src/components/SsoLoginButton.tsx` | あり |
| FR1.2 | OK | `crates/local-sights-core/src/sso_login.rs` | あり |
| FR1.3 | OK | `crates/local-sights-core/src/session.rs` | あり |
| FR1.4 | OK | `crates/local-sights-core/src/catalog/mod.rs` | あり |
| FR1.5 | OK | `src/components/SsoLoginButton.test.tsx` | あり |
| FR2.1 | OK | `src-tauri/src/sso_cli.rs` | あり |
| FR2.2 | OK | `src/components/SsoLoginModal.tsx` | あり |
| FR2.3 | OK | `src/components/SsoLoginModal.test.tsx` | あり |
| FR2.4 | OK | `src-tauri/src/sso_cli.rs` | あり |
| FR2.5 | OK | `src-tauri/src/sso_cli.rs` | あり |
| FR2.6 | OK | `crates/local-sights-core/src/session.rs` | あり |
| FR3.1 | OK | `src-tauri/src/lib.rs` | あり |
| FR3.2 | OK | `crates/local-sights-core/src/session.rs` | あり |
| FR3.3 | OK | `src/components/SsoLoginModal.tsx` | あり |
| FR3.4 | OK | `crates/local-sights-core/src/gateway/aws.rs` | あり |
| FR4.1 | OK | `src-tauri/src/sso_cli.rs` | あり |
| FR4.2 | OK | `src/components/SsoLoginModal.test.tsx` | あり |
| NFR1 | OK | `crates/local-sights-core/src/gateway/aws.rs` | あり |
| NFR2 | OK | `crates/local-sights-core/src/sso_login.rs` | あり |
| NFR3 | OK | `src-tauri/src/sso_cli.rs` | あり |
| NFR4 | OK | `src-tauri/src/sso_cli.rs` | あり |
| NFR5 | OK | `src/i18n/messages.test.ts` | あり |
| NFR6 | OK | `src-tauri/src/sso_cli.rs` | あり |
| NFR7 | OK | `src/components/SsoLoginModal.tsx` | あり |

## 対応していない要素

なし。

注：FR2.6（同時に 1 つ）は、キャンセル直後の最大 2 秒に CLI が 2 つ動きうる（R-03）。NFR4（子プロセスを残さない）のうち、アプリ終了の経路は実機でしか確かめられない。
