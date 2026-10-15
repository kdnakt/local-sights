# CI Config — 261012-parallel-fetch

## 前提

- CI は GitHub Actions。今の 3 つのワークフローを使い、新しいワークフローは作らない（`ci-pipeline-questions.md` の「記録から答えが決まっている項目」）。
- CI は実際の AWS に接続せず、AWS の認証情報も置かない（project.md の Forbidden）。この作業のテストはすべて `FakeGateway` と止めた時計で動く。
- 成果物（`.dmg`）の置き場所は GitHub Releases。ECR・CodeArtifact・S3 は使わない（team.md の Deployment）。

## ワークフロー

| ファイル | きっかけ | 中身 | この作業での変更 |
|----------|----------|------|------------------|
| `.github/workflows/ci.yml` | すべての PR、`main` への push | ライブラリ（fmt・clippy・カバレッジ付きテスト・doc テスト）、画面（版の一致・tsc・ESLint・Prettier・カバレッジ付き Vitest・vite build・npm audit）、cargo-deny、アプリ（Ubuntu と macOS でビルド・clippy・テスト） | rust-core ジョブに 1 手順を足した（下記） |
| `.github/workflows/release.yml` | `v*` のタグの push | タグの形と版の一致・`main` 上のコミットか → ライブラリと画面のテスト → `.dmg` を作って GitHub Releases に公開 | 変更なし |
| `.github/workflows/release-dry-run.yml` | 手動（workflow_dispatch） | `.dmg` を作るだけ（タグも公開もしない） | 変更なし |

## この作業での変更

`ci.yml` の rust-core ジョブ（`Rust library (fmt, clippy, test)`）の最後に、確認用プログラムのテストの手順を足した（Q1 の回答 A）。

```yaml
      # The developer-only fetch_check example has its own argument tests
      # (--region); they never connect to AWS.
      - name: example tests (library)
        run: cargo test -p local-sights-core --examples
```

- 対象は `examples/fetch_check.rs` の 3 件（`--region` が無いと使い方の誤り、他の引数と一緒に読み取る、リージョンの形でない値や空の値を受け付けない）。AWS には接続しない。
- ジョブ名は変えていない。ブランチ保護の必須チェックの名前（`Rust library (fmt, clippy, test)` など）は今のままで、合わせ直しは要らない（project.md の Corrections：ジョブ名を変えるときは必須チェックの名前と合わせる）。
- 手元の確認：`cargo test -p local-sights-core --examples` が 3 件成功。YAML の読み込みと `prettier --check .github/workflows/ci.yml` も成功。

## 変えなかったもの

| もの | 理由 |
|------|------|
| 100 万件の一覧の速度測定（`--release --ignored`） | 実行機の速さに左右されるため、今と同じく手元で走らせる（Build and Test の `performance-test-instructions.md`） |
| 実際の AWS での速さの比べ（`fetch_check` の実行） | CI から AWS に接続しない決まり。開発者の手元で行い、2026-10-15 に合格 |
| リリースのワークフロー | この作業は配布の仕組みを変えていない。次の版のタグは `v0.1.2`（team.md の Deployment） |

## 走らせ方と確かめ方

- この枝の PR を作ると `ci.yml` が走る。すべてのジョブが緑になってから `main` にスクワッシュマージする（team.md の Way of Working）。
- PR はまだ作っていない（頼まれたときに作る）。
