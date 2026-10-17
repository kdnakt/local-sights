# CI Config — 261015-windows-support

## 前提

- CI は GitHub Actions。今の 3 つのワークフローを使い、新しいワークフローは作らない（`ci-pipeline-questions.md` の「記録から答えが決まっている項目」）。
- CI は実際の AWS に接続せず、AWS の認証情報も置かない（project.md の Forbidden）。使う秘密は自動の `GITHUB_TOKEN` だけ。
- 成果物（`.dmg` と `x64-setup.exe`）の置き場所は GitHub Releases。ECR・CodeArtifact・S3 は使わない（team.md の Deployment）。
- この作業の CI の変更はすべて Code Generation で入れた（Part A：PR #23、Part B：PR #24）。このステージでワークフローの追加の変更はしない。

## ワークフロー

| ファイル | きっかけ | 中身 | この作業での変更 |
|----------|----------|------|------------------|
| `.github/workflows/ci.yml` | すべての PR、`main` への push | ライブラリ（fmt・clippy・カバレッジ付きテスト・doc テスト・確認用プログラムのテスト）、画面（版の一致・tsc・ESLint・Prettier・カバレッジ付き Vitest・vite build・npm audit）、cargo-deny、アプリ（Linux・macOS・Windows でビルド・clippy・テスト） | `tauri-app` の matrix に `windows-latest` を足し、Linux 以外で `tests (library)` を足した（PR #23） |
| `.github/workflows/release.yml` | `v*` のタグの push | `verify`（タグの形・版の一致・`main` 上のコミット）→ `test`（ライブラリと画面）→ `build-macos`（`.dmg`、アドホック署名の検証）と `build-windows`（NSIS の `x64-setup.exe`）→ `publish`（両方がそろったときだけ 1 つの Release に公開） | 3 ジョブから 5 ジョブに（PR #24） |
| `.github/workflows/release-dry-run.yml` | 手動（workflow_dispatch） | `.dmg` と Windows のインストーラを作るだけ（タグも公開もしない。成果物は 7 日） | `build-windows` ジョブを足した（PR #23） |

## この作業での変更

### `ci.yml`（PR #23）

```yaml
  tauri-app:
    name: Tauri app (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      # ...
      - name: tests (library)
        if: runner.os != 'Linux'
        run: cargo test -p local-sights-core
```

- ジョブ名は matrix で `Tauri app (windows-latest)` が増えるだけで、既存の 5 つの名前は変えていない（project.md の Corrections）。
- ライブラリのテストは Linux では `rust-core` がカバレッジ付きで走らせるため、`tauri-app` では Linux 以外だけで走らせる（team.md の Testing Posture：Rust のテストは 3 OS で）。
- 改行は `.gitattributes` で LF に固定し、Windows の取り出しでも `cargo fmt --check` と `prettier --check` が同じ結果になる。

### `release.yml`（PR #24）

- `verify` → `test` → `build-macos` → `build-windows` → `publish` の 5 ジョブ。`publish` は `needs: [verify, test, build-macos, build-windows]` で、どちらかのビルドが失敗すれば何も公開しない（FR2.3）。
- `build-macos` は従来の `.dmg` のビルド、1 つだけの確認、`.dmg` の中の `.app` のアドホック署名の検証を引き継ぐ。`build-windows` は `npm run tauri build -- --bundles nsis` で、`*-setup.exe` が 1 つで `x64` を含むことを確かめる。
- 成果物は `actions/upload-artifact`（v7.0.2）で渡し、`publish` が `actions/download-artifact`（v8.0.2）で受け取る。どちらも SHA で固定している（v8.0.2 の SHA は `git ls-remote` でタグと一致することを確認済み）。
- `scripts/release/signing.test.ts` が、ファイル内のジョブの並び（`build-macos` が `build-windows` より前、署名の検証が `gh release` より前）を検査する。

### `release-dry-run.yml`（PR #23）

- `build-windows` ジョブ（成果物 `local-sights-windows-dry-run`）を足した。既存の `build-dmg` は変えていない。

## 必須チェック（ルールセット）

`main` のルールセット（id 24831821）の必須チェックに `Tauri app (windows-latest)` を加える（FR1.3、team.md の Way of Working）。利用者が GitHub の設定画面から行った（`ci-pipeline-questions.md` Q1 の B、2026-10-17T02:06:28Z に保存）。API で読み直し、必須チェックが 6 つになったことを確かめた。

## 変えなかったもの

| もの | 理由 |
|------|------|
| 100 万件の一覧の速度測定（`--release --ignored`） | 実行機の速さに左右されるため、今と同じく手元で走らせる |
| 実機での確認（GUI、実際の AWS、インストーラ） | CI から AWS に接続しない決まり。Release dry run の成果物を手元の Mac と Windows 11 で入れて確かめる（`docs/releasing.md` の順序） |
| カバレッジの計測の OS | Linux だけで測る（team.md の Testing Posture）。`cfg(windows)` のコードは計測の対象外（了解済み） |

## 走らせ方と確かめ方

- PR #24 は CI の 7 つのチェックがすべて緑になってから `main` にスクワッシュマージした（a231279、2026-10-17）。
- マージ後、`main` で Release dry run を動かし、両 OS で入れて確かめてから `v0.2.0` のタグを付ける（`docs/releasing.md`）。
