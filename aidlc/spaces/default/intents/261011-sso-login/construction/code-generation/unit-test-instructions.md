# Unit Test Instructions — アプリから SSO ログインする

計画は `code-generation-plan.md`。テスト戦略は Standard（部品ごとに 5〜8 テスト）、スコープ refactor の下限は「既存スイートが緑のまま」。Testing Contract の順序：コアの純粋なロジック（SSO の判定、ログイン先の決定、CLI 出力の取り出し、ログインの状態）はテストを先に書き、Tauri 側の CLI 起動と画面は実装してからテストを書く。

## テストの枠組み

| 層 | 枠組み | 設定 |
|---|---|---|
| コア（Rust） | `cargo test`（モジュール内の `#[cfg(test)]`） | 追加設定なし。AWS には接続しない |
| Tauri 側（Rust） | `cargo test -p local-sights`（`sso_cli.rs` のモジュール内テスト） | 偽の CLI（一時ディレクトリのシェルスクリプト）を使う。本物の AWS CLI は呼ばない |
| 画面（TypeScript/React） | vitest + @testing-library/react（jsdom） | 既存の `vitest.config.ts`。Tauri の API は `src/test/` の既存モックで差し替える |

## この作業のテストだけを動かすコマンド

最初のテスト先行ステップ（Step 2）の前に、次が今の状態で通ることを確認する（Step 1）：

```
cargo +stable test -p local-sights-core catalog
/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/node node_modules/vitest/vitest.mjs run src/components/LogGroupPane.test.tsx
```

| ステップ | コマンド |
|---|---|
| Step 2 SSO の判定（Red → Green） | `cargo +stable test -p local-sights-core catalog` |
| Step 3 ログイン先と出力の取り出し（Red → Green） | `cargo +stable test -p local-sights-core sso_login` |
| Step 4 ログインの状態（Red → Green） | `cargo +stable test -p local-sights-core session` |
| Step 5 クライアントの破棄 | `cargo +stable test -p local-sights-core gateway` |
| Step 6 CLI の起動 | `cargo +stable test -p local-sights sso_cli` |
| Step 7 画面 | `/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/node node_modules/vitest/vitest.mjs run src/components/SsoLoginButton.test.tsx src/components/SsoLoginModal.test.tsx src/components/LogGroupPane.test.tsx src/components/FetchErrorBanner.test.tsx src/components/TopBar.test.tsx src/App.test.tsx src/i18n/messages.test.ts` |

注：`cargo test` の名前フィルタは 1 つだけ受け取る。手元の既定ツールチェーンが MSRV より古いので `cargo +stable` を使う（CI は stable）。PATH 上の node は v16 のため、vitest は Node 22 のバイナリで実行する。

## 期待するテストの量

| 対象 | 数 | 種類 |
|---|---|---|
| catalog の SSO 判定 | 7 前後 | 単体（先行） |
| `sso_login.rs`（ログイン先・引数・出力の取り出し・URL 判定） | 20 前後 | 単体（先行） |
| ログインの状態（session） | 8 前後 | 単体（先行） |
| `clear_clients` | 1 | 単体 |
| `sso_cli.rs` | 8 前後 | 結合（偽の CLI） |
| `SsoLoginButton`・`SsoLoginModal` | 4・6 | 部品（jsdom） |
| 既存部品への追加 | 各 1〜2 | 部品（jsdom） |

カバレッジの数値の下限は設けない（team.md）。コアの公開関数（`resolve_login_target`、`cli_args`、`extract_login_hints`、`is_allowed_login_url`、`clear_clients`、セッションの SSO ログインの関数）には必ずテストがあること。

## モック・スタブ

- 偽の CLI：一時ディレクトリに `#!/bin/sh` のスクリプトを書いて実行権限を付ける。例：`echo https://device.sso.us-east-1.amazonaws.com/; echo ABCD-EFGH; exit 0`、`exit 3`、`sleep 600`、`sleep 600 & wait`（子プロセスを作る）。探索の候補にその場所を渡す。
- 画面のテストでは `vi.mock("../api")` の既存の形で `startSsoLogin` などを差し替え、`sessionView({ ssoLogin: {...} })` で状態を与える。
- 自動テストは本物の AWS・本物の AWS CLI・本物のブラウザを使わない（`open_sso_url` の `/usr/bin/open` 起動はテストしない。URL の判定だけをテストする）。

## テストデータ

- 設定ファイル：`[default]\nsso_session = corp`、`[profile dev]\nsso_start_url = https://x.awsapps.com/start`、`[profile role]\nsource_profile = dev\nrole_arn = arn:aws:iam::111122223333:role/R`、循環 `a → b → a`、6 段の連鎖、`[sso-session corp]`。
- URL：許可 `https://device.sso.us-east-1.amazonaws.com/`、`https://my.awsapps.com/start/#/device`、`https://oidc.us-east-1.amazonaws.com/authorize?response_type=code`。不許可 `http://device.sso.us-east-1.amazonaws.com/`、`https://evil.example.com/`、`https://amazonaws.com.evil.example/`。
- 確認コード：許可 `ABCD-EFGH`。不許可 `abcd-efgh`、`ABC-DEFG`、`ABCDE-FGHI`。
