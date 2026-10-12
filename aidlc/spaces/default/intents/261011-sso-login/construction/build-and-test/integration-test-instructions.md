# Integration Test Instructions — アプリから SSO ログインする

テスト戦略は Standard。この変更での主な境界は、(1) コアの状態とアプリ側の CLI 起動の間、(2) アプリと外部プロセス（AWS CLI）の間、(3) 画面と Tauri コマンドの間、の 3 つ。作業単位は 1 つなので、作業単位の間の連携はない。単体テストの手順は `construction/code-generation/unit-test-instructions.md` にある。

## 自動の結合テスト

| 境界 | テスト | コマンド | 本物を使わないもの |
|---|---|---|---|
| アプリ ⇔ AWS CLI のプロセス | `src-tauri/src/sso_cli.rs` のモジュール内テスト 12 件（探索、成功、失敗、時間切れ、キャンセル、子プロセスを持つグループの停止、許可されない URL） | `cargo +stable test -p local-sights sso_cli` | 本物の AWS CLI。一時ディレクトリの偽の `#!/bin/sh` スクリプトを使う |
| コアの接続一覧 ⇔ 認証の流れ | `crates/local-sights-core/tests/u1_fetch_flow.rs`、`u2_log_group_listing.rs`、`u3_fetch_flow.rs` ほか（既存の回帰） | `cargo +stable test -p local-sights-core` | 本物の AWS。trait の裏のスタブを使う |
| 画面 ⇔ Tauri コマンド | `src/App.test.tsx` ほか（`invoke` は `vi.mock("../api")` で差し替え） | `npx vitest run` | Tauri の実行環境 |

Linux で `-p local-sights` のテストを動かすには、`build-instructions.md` の GTK／WebKit の開発ライブラリが要る。

## 自動テストで確かめられない境界

画面 ⇔ Tauri コマンドの境界のうち、**コマンドの登録と権限**は自動テストで確かめていない。画面のテストは `invoke` を差し替えており、Rust のテストはコマンド関数を直接呼ぶため。今は新しい 4 つのコマンド（`start_sso_login`、`cancel_sso_login`、`dismiss_sso_login`、`open_sso_url`）が `src-tauri/build.rs` の `AppManifest::commands` にも、`src-tauri/capabilities/default.json` の `allow-*` にもない（レビュー R-01）。

確かめる方法：

1. **静的な突き合わせ**：`src/api.ts` で `invoke` しているコマンド名の集合が、`build.rs` のコマンド一覧と `default.json` の `allow-<kebab>` に含まれることを確かめる。小さなテスト（Vitest か `scripts/`）で自動化できる。
2. **実機の確認**：Mac で `npm run tauri dev` を起動し、SSO プロファイルで認証エラーを出して「SSO にログイン」を押す。ダイアログが開くこと。

## 手動の結合確認（Mac の実機）

README.md の「Sign in with SSO」のチェックリストに従う。本人の AWS 認証情報を使い、本人の手元で行う（team.md）。

| 確認 | 期待 |
|---|---|
| 直接の SSO プロファイルでログイン | ブラウザで承認すると「ログインしました」、一覧が読み直される。取得は自動では始まらない |
| AssumeRole のプロファイル（source_profile が SSO） | source 側でログインし、一覧が読み直される |
| ブラウザが開かない | ダイアログの URL を押す、または確認コードを入れてログインできる |
| キャンセル／5 分の時間切れ | `pgrep -fl "aws sso login"` に何も残らない |
| ログイン中の Cmd+Q／Dock の「終了」 | `aws` のプロセスが残らない |
| AWS CLI を一時的に退避 | 「AWS CLI が見つかりません」と代わりの手順が出る |

## 期待するカバレッジ

数値の下限は設けない（team.md）。境界ごとに、成功・失敗・時間切れ・キャンセルの各経路にテストがあることを求める。`sso_cli` はこれを満たしている。
