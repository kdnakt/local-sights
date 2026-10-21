# ビルドの手引き（261020-carryover-fixes）

## 前提

- Rust（`rust-toolchain` なし。CI と同じ stable）、Node.js（CI と同じ LTS）、npm。
- Linux で Tauri のアプリを作るときは、CI と同じ開発用ライブラリ：`libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`（`.github/workflows/ci.yml` の「Linux WebKitGTK / GTK development packages」）。macOS と Windows では追加はない。
- ライブラリの行カバレッジを手元で測るときだけ：`rustup component add llvm-tools-preview` と `cargo install cargo-llvm-cov --version 0.9.1 --locked`（CI と同じ版）。
- AWS の認証情報は要らない。自動テストは AWS につながない（project.md Forbidden）。

## 依存関係を入れる

```bash
npm ci
```

Rust の依存関係は `Cargo.lock` のとおり、最初の `cargo` のコマンドで取られる。この作業で依存関係は変えていない。

## ビルド

```bash
# 画面（Tauri の frontendDist）
npm run build          # package.json の build スクリプト：tsc --noEmit && vite build

# ライブラリとアプリ
cargo build -p local-sights-core
cargo build -p local-sights
```

## ビルドの確かめ（CI と同じ検査）

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
npx tsc --noEmit
npx eslint .
npx prettier --check .
npm run check:version
npm audit --audit-level=high
```

依存関係のチェック（`cargo-deny`）は CI の `cargo-deny` のジョブで行う。この作業で依存関係は変えていないため、手元では流さない。

## つまずきやすい点

- `cargo build -p local-sights` が `webkit2gtk` などで落ちる：上の Linux の開発用ライブラリが入っていない。
- Vitest の全体の実行で、重い処理（`cargo install` など）と同時に流すと、時間に余裕のないテスト（5 秒の上限）が時間切れで落ちることがある。単体で流し直して確かめる（test-results.md の「時間切れ」の項）。
- 改行コードは LF（`.gitattributes`）。Windows で取り出しても `cargo fmt --check` と `prettier --check` の結果は同じ。
