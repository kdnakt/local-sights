# CD Config — リリースの .app をアドホック署名する

このプロジェクトの「デプロイ」は、GitHub Releases に置いた `.dmg` を会社の Mac に入れる配布のこと（team.md の Deployment）。サーバー環境はなく、ステージング・本番の段階もない。配布の仕組みは既存の Release ワークフロー（`.github/workflows/release.yml`）で、今回の変更（Code Generation）でアドホック署名と署名検証が加わった。新しいワークフローは作らない。

## 配布の流れ（`release.yml`、変更後）

| 順 | ジョブ / ステップ | 内容 | 失敗時 |
|---|---|---|---|
| 1 | `verify` | タグが `vX.Y.Z` の形で、4 ファイルの版と一致し、コミットが `main` 上にある | 何もビルドしない |
| 2 | `test` | `cargo test -p local-sights-core`、`npx vitest run`（今回の `signing.test.ts` 24 件を含む） | ビルドしない |
| 3 | `build-and-publish` / Apple Silicon runner | ランナーが arm64 であることを確認 | 止まる |
| 4 | 同 / build the .dmg (ad-hoc signed) | `APPLE_SIGNING_IDENTITY=-` で `npm run tauri build -- --bundles dmg`。Tauri が `.app` をアドホック署名して `.dmg` を作る | 止まる |
| 5 | 同 / the .app carries a valid ad-hoc signature（**新規**） | `codesign --verify --deep --strict` と `codesign -dvv` の `Signature=adhoc` を確認 | `::error::` で止まり、Release は作られない |
| 6 | 同 / exactly one Apple Silicon .dmg | `.dmg` が 1 つだけで `aarch64` であることを確認 | 止まる |
| 7 | 同 / publish the Release | `gh release create`（既存なら `upload --clobber`）。ノートに「Ad-hoc signed (not notarized)」 | 止まる |

トリガー：`main` のコミットへの `v*` タグの push。承認ゲート：タグを打つ人（開発者本人）の判断が唯一の承認。GitHub Environments の保護は使わない（開発者 1 人、team.md）。

## 秘密情報と権限

- 使う秘密情報は自動の `GITHUB_TOKEN`（`contents: write` は `build-and-publish` ジョブだけ）のみ。Apple の証明書・鍵、AWS の認証情報は置かない（NFR1、project.md の Forbidden）。
- `APPLE_SIGNING_IDENTITY` の値 `-` は codesign のアドホック識別子で、秘密ではない。ワークフローに平文で書く。

## 成果物

- `Local-Sights_<version>_aarch64.dmg`（GitHub Releases のアセット）。中の `Local Sights.app` はアドホック署名付き、公証なし。
- リリースノートに、ビルド元のコミットと Actions の実行 URL、署名の種類を記す。

## 環境の昇格

なし。リリースは 1 段階（タグ → GitHub Releases）。`release-dry-run.yml`（手動実行、公開なし）はタグ前のビルド確認用に残るが、今回は署名を入れていない（FR1.3）。そのためドライランの `.dmg` では署名まわりは確かめられない。

## 次のリリース

- 版は `v0.1.1`（Q1）。`Cargo.toml`、`Cargo.lock`、`package.json`／`package-lock.json`、`src-tauri/tauri.conf.json` を `0.1.1` にそろえる（`npm run check:version` で確認）。
- 手順は `deployment-strategy.md`。実行は次のステージ（Deployment Execution）で扱う。
