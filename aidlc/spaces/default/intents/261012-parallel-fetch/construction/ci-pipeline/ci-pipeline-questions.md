# CI Pipeline Questions — 261012-parallel-fetch

## 記録から答えが決まっている項目

次の項目は team.md と今のリポジトリで決まっているため、質問しない。

| 項目 | 答え | 出典 |
|------|------|------|
| CI の道具 | GitHub Actions（`.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`） | リポジトリ、team.md の Deployment |
| 枝の進め方 | トランクベース開発。短命の枝で PR を作り、CI がすべて通ってから `main` にスクワッシュマージする | team.md の Way of Working |
| マージ前の品質ゲート | `cargo fmt --check`、clippy（警告はエラー扱いにしない）、ライブラリと画面の行カバレッジ 80%、doc テスト、`tsc --noEmit`、ESLint、Prettier、Vitest、vite build、版の一致、cargo-deny、`npm audit --audit-level=high`、アプリのビルドとテスト（Ubuntu・macOS） | team.md の Testing Posture・Code Style、`ci.yml` |
| 成果物の置き場所 | GitHub Releases（`vX.Y.Z` のタグで `.dmg` を公開）。ECR・CodeArtifact・S3 は使わない | team.md の Deployment |
| AWS の認証情報 | CI には置かない。CI から実際の AWS に接続しない | project.md の Forbidden |

## Q1 確認用プログラムのテストを CI で走らせるか

Build and Test では `cargo test -p local-sights-core --example fetch_check`（3 件、確認用プログラムの `--region` 引数の扱い。AWS には接続しない）を走らせて通りましたが、今の CI（`cargo llvm-cov -p local-sights-core` と doc テスト）はこの 3 件を走らせていません。どうしますか？

A. CI の rust-core ジョブに `cargo test -p local-sights-core --examples` の手順を 1 つ足す（ジョブ名は変えないので、ブランチ保護の必須チェックは今のまま） (Recommended)
B. CI は今のまま変えない（3 件は手元で走らせる）
X. Other (please specify)

[Answer]: A

## Q2 要件の対応表の 45 件の扱い

Construction を締める確認では「作業単位をまたぐ要件の対応表（cross-unit-traceability.md）が合格していること」を求めます。Build and Test では、要件 94 件のうち 49 件は code-generation の対応表に直接の行があり、残り 45 件は直接の行がありませんでした。45 件の内訳は、親の ID（FR1〜FR5、NFR1〜NFR10 など）、U2 の AC2.2.2・AC2.2.5（実装とテストは U1 にある）、下準備の FR6（PR #20）、手元の確認で合格した AC1.1.4 です。どれも、子の ID や別の場所を通じて実装とテストまでたどれます。どうしますか？

A. 45 件は間接の対応として受け入れ、Construction の締めの確認を合格にする。受け入れたことを phase-check に記録する (Recommended)
B. 各作業単位の code-generation の traceability.json に 45 件の行を足してから締める（レビュー済みの成果物を書き直すため、作業単位のレビューのやり直しが要る場合がある）
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- CI は今の GitHub Actions（`ci.yml`・`release.yml`・`release-dry-run.yml`）を使い、新しいワークフローは作らない。AWS の認証情報は置かず、AWS には接続しない
- `ci.yml` の rust-core ジョブに `cargo test -p local-sights-core --examples`（確認用プログラムの `--region` のテスト 3 件）を 1 手順足す。ジョブ名は変えないので、必須チェックの名前は今のまま
- マージ前のゲートは Build and Test で走らせたコマンドをすべて含む。100 万件の一覧の速度測定と実際の AWS での速さの比べだけは手元に残す
- 要件の対応表で直接の行がない 45 件は間接の対応として受け入れ、Construction の締めの確認（phase-check）を合格にする
- PR は頼まれたときに作る。配布は `main` にマージした後に `v0.1.2` のタグで行う

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
