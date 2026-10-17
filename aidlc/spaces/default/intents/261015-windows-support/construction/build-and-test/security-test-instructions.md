# Security Test Instructions — 261015-windows-support

この作業の安全上の決まりは project.md の Forbidden（呼ぶのは CloudWatch Logs の読み取り 3 つだけ、秘密の認証情報を書かない、自動テストと CI から実際の AWS に接続しない、テレメトリを送らない）と、`requirements.md` の NFR3（Windows のキャッシュと設定のアクセス権は OS の既定に任せる）。加えて、Windows で新しく OS のプロセスとシェルに触れるため、シェルを通さないこと・外部の入力をコマンドにしないことを確かめる。

## 1. 自動の検査

| 検査 | コマンド | 期待 | 目標 |
|------|----------|------|------|
| 依存関係（脆弱性・禁止・ライセンス・取得元）。`windows-sys` を含む | `cargo deny check` | advisories・bans・licenses・sources がすべて ok | NFR3 |
| 画面側の依存関係 | `npm audit --audit-level=high` | high 以上が 0 件 | NFR3 |
| 秘密情報のスキャン | GitGuardian（PR のチェック） | 成功 | NFR3、project.md Forbidden |
| CLI の出力の扱い（URL とコードだけを拾う、許可した host だけ開く） | `cargo test -p local-sights sso_login` | 成功（既存） | NFR3 |
| CLI の起動がシェルを通らず、引数をそのまま渡す | `cargo test -p local-sights sso_cli`（Unix・Windows それぞれ） | 成功 | NFR3 |
| 止めるときに CLI の子まで残らない | `windows_tests`（CI の Windows） | 成功 | FR4.4 |
| リリースのワークフローが秘密を使わない（`GITHUB_TOKEN` だけ） | `scripts/release/signing.test.ts` と下の差分の確認 | 成功 | NFR3 |

## 2. 差分の確認（main との比較。コードの読み取り）

基点は Part A の分かれ目（`f09f448` = PR #22 の後の `main`）と現在の head。

```sh
B=f09f448
# 呼ぶ API：trait と SDK の呼び出しは変えていない
git diff $B HEAD -- crates/local-sights-core/src/gateway/
# 新しい依存：windows-sys（cfg(windows)）だけ、libc は cfg(unix) に移動
git diff $B HEAD -- Cargo.toml src-tauri/Cargo.toml crates/local-sights-core/Cargo.toml package.json
# プロセスの起動：Command::new(<絶対パス>) に引数を配列で渡し、shell を通さない。CREATE_NO_WINDOW
grep -n "Command::new\|creation_flags\|ShellExecuteW" src-tauri/src/sso_cli/*.rs src-tauri/src/browser.rs
# ワークフローの secrets：GITHUB_TOKEN 以外を使っていない
grep -n "secrets\.\|AWS_" .github/workflows/*.yml
# 手順書・ログ・画面に秘密を書いていない（キー・トークンの形）
git diff $B HEAD | grep -nE "AKIA[0-9A-Z]{16}|aws_secret_access_key|session_token" || echo "none"
```

期待：

- `gateway/` に差分がない（呼ぶ API は 3 つのまま）。
- 依存の差分は `windows-sys`（`cfg(windows)`）の追加と `libc` の `cfg(unix)` への移動だけ。
- `Command::new` の引数は `locate_cli` が返した絶対パス、`ShellExecuteW` に渡すのは `is_allowed_login_url` を通った URL だけ。
- ワークフローは `${{ github.token }}` / `GITHUB_TOKEN` だけ。`AWS_` を含む secrets はない。
- 差分にキーやトークンの形の文字列がない。

## 3. Windows のアクセス権（NFR3）

アプリは Windows で `AppData` の下のフォルダの権限を変えない（`ensure_private_dir` は Windows ではフォルダを作るだけ）。手順書（`docs/install.md` の Settings and cache on Windows）に、権限は Windows の既定（本人・管理者・SYSTEM だけが読める）のままと書いてあることを確かめる。

```sh
grep -n "default" docs/install.md | grep -i "right\|permission"
```

## 4. インストーラの署名の言い回し（FR5.1、FR2.4）

Windows のインストーラはコード署名証明書で署名していない。手順書とワークフローの `NOTES` が、`unsigned` / `not signed` 単体ではなく何の署名がないかを言うこと（`findUnsignedClaims` の検査）：

```sh
npx vitest run scripts/release/signing.test.ts
```
