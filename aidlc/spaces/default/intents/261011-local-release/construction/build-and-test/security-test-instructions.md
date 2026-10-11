# Security Test Instructions — リリース準備

## 対象とする要件

- NFR1：リリース用ワークフローは `GITHUB_TOKEN` だけを使い、AWS・Apple などの秘密情報を CI に置かない。リリース用ワークフローから実際の AWS に接続しない。
- NFR2：書き込み権限（`contents: write`）は Release を作るジョブだけに与える。既存の CI は `contents: read` のまま。
- NFR5：自動更新・テレメトリ・クラッシュレポートの仕組みを入れない。
- project.md の Forbidden：秘密情報をリポジトリ・ログ・エラー表示に書かない。

## 実行の方法（この工程で実行できる）

```bash
grep -n 'secrets\.' .github/workflows/release.yml          # 0 件であること
grep -n -i 'aws' .github/workflows/release.yml              # コメント以外に 0 件であること
grep -n 'contents: write' .github/workflows/*.yml           # release.yml の build-and-publish だけ
grep -n 'contents: read' .github/workflows/ci.yml           # 既存のまま
grep -n -i 'updater\|telemetry' src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml   # 0 件
npm audit --audit-level=high                                # 0 件
```

- 版の検査の失敗の文が、ファイル名・パッケージ名・版だけの固定の形であることは、`scripts/release/versions.test.ts` の該当テストで確かめる。
- タグ名がシェルに直接埋め込まれず `env` 経由で渡されていることを、`release.yml` の定義で確かめる。
- 既存の `cargo-deny`（CI）はそのまま残る。

## 既知の残り

- 書き込み権限を持つジョブが、外部の action（`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`）を可変タグで参照している。既存の CI と同じ方式で、Code Generation のレビューで受け入れ済み（R-02）。
