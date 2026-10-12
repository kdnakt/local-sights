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

## リリースの実行結果（2026-10-12、マージ後）

| 実行 | タグの指す先 | 結果 |
|---|---|---|
| Release #2 | `f25a193`（古いコミット、版 `0.1.0`） | `verify` で失敗：`Tag v0.1.1 does not match version 0.1.0 in the files.` タグを打った手元が `main` の最新ではなかった。何も公開されていない |
| Release #3 | `645d77e`（PR #18 のマージ） | `verify` と `test` は成功。ビルドでは `Signing with identity "-"` と出て署名は付いた。しかし署名検証ステップが `target/release/bundle/macos/Local Sights.app was not built` で失敗。何も公開されていない |

**Release #3 の原因**：`--bundles dmg` だけを指定すると、Tauri は `.dmg` を作ったあとで中間の `.app` を消す（ログの `Cleaning .../Local Sights.app`）。署名検証ステップはビルド直後にその `.app` を見る作りで、前提 A3 が誤っていた。Build and Test で自動では確かめられないとしていた点が、最初の実リリースで表に出た。

**修正**：署名検証ステップを「exactly one Apple Silicon .dmg」の後、公開の前に移し、公開する `.dmg` そのものを `hdiutil attach` で開いて、中の `Local Sights.app` に `codesign --verify --deep --strict` と `Signature=adhoc` の確認を行う。配る成果物そのものを検証するので、元の作りより確かになる。`scripts/release/signing.ts` の順序の決まりも「`.dmg` の後、`gh release` の前」に改め、今回の失敗した並びを拒むテストを足した（Vitest 322 件すべて成功）。

## コミットに含めたもの

- `.github/workflows/release.yml`、`docs/install.md`、`docs/releasing.md`、`README.md`
- `scripts/release/signing.ts`、`scripts/release/signing.test.ts`
- 版上げの 5 ファイル
- `aidlc/spaces/default/intents/261012-adhoc-signing/`（この作業の記録）、`aidlc/spaces/default/codekb/local-sights/`（コードの知識ベース）、`aidlc/spaces/default/intents/intents.json`

## データベースの移行・依存サービス

該当なし（ローカルアプリの配布で、サーバーもデータベースもない）。

## 戻し方

`operation/deployment-pipeline/rollback-runbook.md`。PR のマージ前であれば、PR を閉じるだけで何も配布されない。
