# CI Configuration — リリース準備

## 使う道具と進め方

- CI の道具：GitHub Actions（`.github/workflows/`）。
- ブランチ：トランクベース。短命のブランチ → PR → CI がすべて緑 → `main` にスクワッシュマージ（team.md の Way of Working）。
- 成果物の置き場所：GitHub Releases（タグ `vX.Y.Z` ごとに、Apple Silicon 用の未署名 `.dmg` を 1 つ）。試しビルドの `.dmg` は、ワークフローの成果物（artifact）として 7 日間残す。
- 秘密情報：使うのは GitHub が自動で発行する `GITHUB_TOKEN` だけ。AWS・Apple の認証情報は置かない（project.md の Forbidden）。

## ワークフロー

| ファイル | きっかけ | 権限 | ジョブ | 役割 |
|----------|----------|------|--------|------|
| `ci.yml`（既存＋版の検査） | PR、`main` への push | `contents: read` | `rust-core`、`frontend`（`npm run check:version` を追加）、`cargo-deny`、`tauri-app`（ubuntu・macOS） | マージ前の品質の門 |
| `release.yml`（新規） | タグ `v*` の push | 全体 `contents: read`、`build-and-publish` だけ `contents: write` | `verify` → `test` → `build-and-publish` | `.dmg` を作ってすぐ公開。同じタグの再実行は `.dmg` を置き換える |
| `release-dry-run.yml`（新規、この工程の Q1） | 手動（`workflow_dispatch`） | `contents: read` | `build-dmg` | タグなしで `.dmg` を試しに作り、公開はせず artifact に残す |

## 外部 action の固定（この工程の Q2）

すべてのワークフローの外部 action を、コミットの SHA で固定した（コメントに版を書く）。Dependabot の `github-actions` の更新（毎週）が、SHA と版のコメントを一緒に上げる PR を出す。

| action | 固定した SHA | 版 |
|--------|--------------|----|
| `actions/checkout` | `3d3c42e5aac5ba805825da76410c181273ba90b1` | v7.0.1 |
| `actions/setup-node` | `949feb2413d6458794dcd2491c4babbbce0c15c1` | v7.1.0 |
| `dtolnay/rust-toolchain` | `686976e191b89faba57d3206551f0f330d8cb249` | `stable` ブランチの先頭 |
| `Swatinem/rust-cache` | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` | v2.9.2 |
| `EmbarkStudios/cargo-deny-action` | `3c6349835b2b7b196a839186cb8b78e02f7b5f25` | v2.1.1 |
| `actions/upload-artifact` | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | v7.0.2 |

SHA は `git ls-remote` で各リポジトリのタグ（`dtolnay/rust-toolchain` はブランチ `stable`）から取得した。`dtolnay/rust-toolchain` は版のタグを持たないため、Dependabot はこの固定を自動では更新しない。必要なときに手で `stable` ブランチの新しい SHA に上げる。

## 確かめた結果

- `actionlint`（v1.7.7）で 3 つのワークフローすべてに問題なし。
- `uses:` のうち、40 桁の SHA で固定されていないものは 0 件（`grep` で確認）。
- `npx prettier --check .` は問題なし。
- 試しビルドと公開の実際の動きは、GitHub 上で実行したときに確かめる（このコンテナは Linux）。

## 手順書

- `docs/releasing.md` の手順 4 に、タグを付ける前の任意の確認として「Release dry run」の実行を足した。
