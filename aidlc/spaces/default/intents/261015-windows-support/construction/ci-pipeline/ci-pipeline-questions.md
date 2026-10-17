# CI Pipeline Questions — 261015-windows-support

## 記録から答えが決まっている項目

次の項目は team.md・project.md と今のリポジトリで決まっているため、質問しない。

| 項目 | 答え | 出典 |
|------|------|------|
| CI の道具 | GitHub Actions（`.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`）。新しいワークフローは作らない | リポジトリ、team.md の Deployment |
| 枝の進め方 | トランクベース開発。短命の枝で PR を作り、CI がすべて通ってから `main` にスクワッシュマージする | team.md の Way of Working |
| マージ前の品質ゲート | `cargo fmt --check`、clippy（警告はエラー扱いにしない）、ライブラリと画面の行カバレッジ 80%（Linux）、doc テストと確認用プログラムのテスト、`tsc --noEmit`、ESLint、Prettier、Vitest、vite build、版の一致、cargo-deny、`npm audit --audit-level=high`、アプリのビルドとテスト（Linux・macOS・Windows） | team.md の Testing Posture・Code Style、`ci.yml` |
| 成果物の置き場所 | GitHub Releases。`vX.Y.Z` のタグで `.dmg` と `x64-setup.exe` を 1 つの Release に公開する。ECR・CodeArtifact・S3 は使わない | team.md の Deployment |
| AWS の認証情報 | CI には置かない。CI から実際の AWS に接続しない | project.md の Forbidden |
| CI のジョブを足したときの必須チェック | 同じ PR の中で `main` のルールセットの必須の状態確認も合わせて直す（例：`Tauri app (windows-latest)`） | team.md の Way of Working、要件 FR1.3 |

## Q1 `main` の必須チェックに `Tauri app (windows-latest)` を加える方法

`main` のルールセット（id 24831821）の必須チェックは、2026-10-17 の時点で 5 つのまま（`Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (macos-latest)`、`Tauri app (ubuntu-latest)`）で、`Tauri app (windows-latest)` が入っていません。この作業の要件 FR1.3 と team.md の決まりで必要なものです。Construction を締める確認（phase-check）では、これが残っていると「未解決の指摘」になります。どうしますか？

A. このセッションから GitHub の API でルールセットに `Tauri app (windows-latest)` を加える（ほかの 5 つと設定はそのまま。加えたあと読み直して確かめる） (Recommended)
B. 自分で GitHub の設定画面から加える（Settings → Rules → Rulesets → main → Require status checks）。加えたら知らせる
C. 今回は加えない（FR1.3 を未解決のまま受け入れ、phase-check に記録する）
X. Other (please specify)

[Answer]: B
