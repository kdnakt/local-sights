# Test Results — アプリから SSO ログインする

実行日：2026-10-12。実行環境：Linux x86_64 のコンテナ、cargo 1.97.0（stable）、Node v22.22.0、GTK／WebKit の開発ライブラリあり。AWS には接続していない。

## ビルド

| コマンド | 結果 |
|---|---|
| `npm run check:version` | 成功。Version 0.1.0 が 4 か所で一致 |
| `npx vite build` | 成功。チャンクサイズの警告は以前からのもの |
| `cargo +stable build -p local-sights` | 成功 |
| `npx tsc --noEmit` | 成功 |
| `npx eslint .` | 成功（警告なし） |
| `npx prettier --check .` | 成功（全ファイル一致） |
| `cargo +stable fmt --all --check` | 成功 |
| `cargo +stable clippy --workspace --all-targets` | 成功（警告 0） |
| `npm audit --audit-level=high` | 成功（0 vulnerabilities） |
| `cargo deny check` | 未実行。このコンテナに cargo-deny がない。CI の cargo-deny ジョブで確かめる。新しい crate は増えていない |

## 単体テスト（unit-test-instructions.md のコマンドを 1 回ずつ）

| コマンド | 合計 | 成功 | 失敗 | 省略 |
|---|---|---|---|---|
| `cargo +stable test -p local-sights-core catalog` | 24 | 24 | 0 | 0 |
| `cargo +stable test -p local-sights-core sso_login` | 22 | 22 | 0 | 0 |
| `cargo +stable test -p local-sights-core session` | 102 | 102 | 0 | 0 |
| `cargo +stable test -p local-sights-core gateway` | 22 | 22 | 0 | 0 |
| `cargo +stable test -p local-sights sso_cli` | 12 | 12 | 0 | 0 |
| `node node_modules/vitest/vitest.mjs run`（Step 7 の 7 ファイル） | 92 | 92 | 0 | 0 |

名前のフィルタが重なるため、上の行を足し合わせても全体の数にはならない。全体の数は次の表を見る。手順書の `/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/node` はこの環境にないため、PATH 上の Node 22 で実行した。

## 全体の回帰

| コマンド | 結果 |
|---|---|
| `cargo +stable test --workspace` | 成功：local-sights（アプリ）12、core lib 359、u1 11、u2 10、u3 14、u5 4、u6 3、失敗 0。ignored 5 件は以前からのもの |
| `npx vitest run` | 成功：31 ファイル、291 件、失敗 0（変更前は 270 件） |

## 失敗

なし。

## カバレッジ

カバレッジの数値の下限は設けていない（team.md）。カバレッジの計測は行っていない。コアの公開関数（`resolve_login_target`、`cli_args`、`extract_login_hints`、`is_allowed_login_url`、`clear_clients`、セッションの SSO ログインの関数）にはテストがある。

## 自動テストで確かめていない点

- **コマンドの登録（R-01、重大）**：`src-tauri/build.rs` と `src-tauri/capabilities/default.json` に新しい 4 つのコマンドがないことを、ファイルを読んで確かめた。実機では画面からの `invoke` が拒否され、「SSO にログイン」を押しても何も起きない見込み。テストは `invoke` を差し替えているため、すべて通っている。
- **CI の範囲（R-02）**：CI は `cargo test -p local-sights` を実行しないため、`sso_cli` の 12 件は CI では動かない。
- **実機の確認**：Mac での手動確認（integration-test-instructions.md）は未実施。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| N/A | `nfr-requirements/`・`nfr-design/`（このワークフローになし）、`code-generation-plan.md` の Testing Contract | 数値で測る品質の目標なし（Testing Contract はカバレッジの下限を定めていない） | N/A | 上の表 | build-and-test | N/A |
