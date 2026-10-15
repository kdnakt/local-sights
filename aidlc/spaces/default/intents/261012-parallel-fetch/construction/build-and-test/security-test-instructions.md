# Security Test Instructions — 261012-parallel-fetch

この作業の安全上の決まりは project.md の Forbidden（呼ぶのは CloudWatch Logs の読み取り 3 つだけ、秘密の認証情報を書かない、自動テストと CI から実際の AWS に接続しない、テレメトリを送らない）と、各作業単位の security-requirements（NFR6.x・NFR8.x）。

## 1. 自動の検査

| 検査 | コマンド | 期待 | 目標 |
|------|----------|------|------|
| 依存関係（脆弱性・禁止・ライセンス・取得元） | `cargo deny check` | advisories・bans・licenses・sources がすべて ok | U1-NFR8.5、U4-NFR8.11、U5-NFR8.14 |
| 画面側の依存関係 | `npm audit --audit-level=high` | high 以上が 0 件 | U4-NFR8.11、U5-NFR8.14 |
| 要約の 1 行に入るのが数値と決まった語だけ | `cargo test -p local-sights-core --lib fetch_stats::` | 成功 | U1-NFR8.3・8.6、U2-NFR8.7、U5-NFR8.13 |
| 失敗の詳細は許可リストを通る（Skipped に写すときも同じ値） | `cargo test -p local-sights-core --lib fetcher::` | 成功 | U1-NFR8.2、U3-NFR8.9 |
| SDK の中の再試行を 1 回に上書き | `cargo test -p local-sights-core --lib gateway::aws::` | 成功 | U1-NFR3.4 |
| SessionView の `elapsed` の形（真偽値と数値だけ） | `cargo test -p local-sights-core --lib session::` | 成功 | U4-NFR8.10 |
| Tauri の権限に足したのは `cancel_fetch` だけ | `cargo test -p local-sights` | 成功（コマンドの一覧と権限の対応のテスト） | U5-NFR8.15 |
| 中断の注意の文言に入るのが本数と決まった語だけ | `npx vitest run src/components/CancelNotice.test.tsx src/i18n/messages.test.ts` | 成功 | U5-NFR8.13 |

自動テストはすべて `FakeGateway` と止めた時計で動き、実際の AWS には接続しない（U1〜U5 の NFR6.x）。CI に AWS の認証情報は置いていない。

## 2. 差分の確認（main との比較）

コードの読み取りで確かめる目標。基点は `main` との分かれ目（`git merge-base HEAD origin/main`）。

```sh
B=$(git merge-base HEAD origin/main)
# 呼ぶ API：trait は変えない。SDK の呼び出しは読み取り 3 つだけ
git diff $B HEAD -- crates/local-sights-core/src/gateway/mod.rs
grep -nE '\.(describe_[a-z_]+|get_[a-z_]+|filter_[a-z_]+|start_[a-z_]+|put_[a-z_]+)\(\)' crates/local-sights-core/src/gateway/aws.rs
# 新しい依存
git diff $B HEAD -- Cargo.toml '*/Cargo.toml' package.json
# 新しい出力・書き込み・送信
git diff $B HEAD -- crates src-tauri src | grep -nE '^\+.*(eprintln!|println!|std::fs|File::|reqwest|https?://)'
# Tauri の権限
git diff $B HEAD -- src-tauri/build.rs src-tauri/capabilities/default.json
```

期待：

- `gateway/mod.rs` に差分がない。SDK の呼び出しは `get_log_events`・`describe_log_groups`・`describe_log_streams` だけ（U1〜U3 の NFR8.1）。
- 新しい依存は `futures-util`（`std` だけ）だけ（U1-NFR8.5）。画面側の依存は増えない（U4-NFR8.11、U5-NFR8.14）。
- 新しい出力は標準エラー出力の要約の 1 行と、確認用プログラムの中断のメッセージだけ。ファイル・ネットワークへの書き込みはない（U1-NFR8.4）。
- 権限に足したのは `cancel_fetch` だけ（U5-NFR8.15）。

## 3. 秘密情報

- GitHub の秘密情報スキャンを有効にしている（team.md の Code Style）。
- 手元の確認の結果を書くときは、件数と時間だけにし、プロファイル名・ロググループ名・アカウント ID をリポジトリに置かない（project.md の Forbidden）。
