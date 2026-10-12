# Deployment Log — リリースの .app をアドホック署名する

上流：`operation/deployment-pipeline/cd-config.md`、`deployment-strategy.md`、`rollback-runbook.md`、`construction/build-and-test/test-results.md`。

## 配布の範囲（Q1〜Q3 の回答）

- この場で行う：ブランチ作成、コミット、push、PR 作成、版上げ（`0.1.1`）を同じ PR に含める。
- 開発者が行う：CI が緑になったのを見てのスクワッシュマージ、`v0.1.1` のタグ打ち（Release ワークフローの起動）、公開後のスモークテスト、会社の Mac への配布。

## 実行記録（2026-10-12）

| 順 | 操作 | 結果 |
|---|---|---|
| 1 | 版上げ：`Cargo.toml` と `src-tauri/tauri.conf.json` を `0.1.1` に、`npm version 0.1.1 --no-git-tag-version`、`cargo update -w` | `npm run check:version -- --tag v0.1.1` → `Version 0.1.1 is the same in Cargo.toml, Cargo.lock, package.json, src-tauri/tauri.conf.json and matches tag v0.1.1.` |
| 2 | ブランチ `adhoc-signing` を `main`（`9d08cbf`）から作成し、変更をすべてコミット | コミット `a93bf83` |
| 3 | `git push -u origin adhoc-signing` | 成功 |
| 4 | PR 作成（`adhoc-signing` → `main`） | **#18** <https://github.com/kdnakt/local-sights/pull/18> |
| 5 | CI（`ci.yml`：rust-core、frontend、cargo-deny、tauri-app ×2） | PR 上で実行中。緑になるまでマージしない（team.md） |
| 6 | スクワッシュマージ | 未（開発者） |
| 7 | `v0.1.1` のタグ push → Release ワークフロー | 未（開発者）。手順は `docs/releasing.md` |
| 8 | スモークテスト（開発者の macOS 15 で初回起動） | 未（開発者）。期待値は `smoke-test-results.md` |
| 9 | 会社の Mac への配布 | 未。スモークテストが通ってから |

## コミットに含めたもの

- `.github/workflows/release.yml`、`docs/install.md`、`docs/releasing.md`、`README.md`
- `scripts/release/signing.ts`、`scripts/release/signing.test.ts`
- 版上げの 5 ファイル
- `aidlc/spaces/default/intents/261012-adhoc-signing/`（この作業の記録）、`aidlc/spaces/default/codekb/local-sights/`（コードの知識ベース）、`aidlc/spaces/default/intents/intents.json`

## データベースの移行・依存サービス

該当なし（ローカルアプリの配布で、サーバーもデータベースもない）。

## 戻し方

`operation/deployment-pipeline/rollback-runbook.md`。PR のマージ前であれば、PR を閉じるだけで何も配布されない。
