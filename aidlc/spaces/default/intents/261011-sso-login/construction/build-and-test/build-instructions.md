# Build Instructions — アプリから SSO ログインする

上流：`construction/code-generation/code-generation-plan.md`、`unit-test-instructions.md`、`code-summary.md`。

## 前提

| 道具 | 版 | 備考 |
|---|---|---|
| Rust | stable（この確認では cargo 1.97.0） | 開発者の Mac では既定のツールチェーンが古いので `cargo +stable` を使う |
| Node.js | 22 | 開発者の Mac では PATH 上の node が v16 なので、Node 22 のバイナリを使う |
| AWS CLI v2 | 手動確認だけで使う | `/opt/homebrew/bin/aws` または `/usr/local/bin/aws`。自動テストは使わない |
| Linux だけ | GTK／WebKit の開発ライブラリ | `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`（CI の tauri-app ジョブと同じ） |

環境変数や設定ファイルは追加していない。AWS の認証情報はビルドにもテストにも要らない。

## 依存関係の導入

```
npm ci
```

Rust の依存関係は `Cargo.lock` のとおりに `cargo` が取得する。今回の変更で `src-tauri` に `libc`（本体）と `tempfile`（開発用）が加わったが、どちらも既にロック済みの crate で、新しい crate は増えていない。

## ビルド

```
npm run check:version
npx vite build
cargo +stable build -p local-sights
```

配布用の `.dmg` は今回の変更の対象外。リリースのワークフロー（タグ `vX.Y.Z`）で作る。

## ビルドの確認

```
npx tsc --noEmit
npx eslint .
npx prettier --check .
cargo +stable fmt --all --check
cargo +stable clippy --workspace --all-targets
```

## よくある問題

- **`cargo build -p local-sights` が Linux で `webkit2gtk` を見つけられない**：上の GTK／WebKit の開発ライブラリを入れる。
- **`cargo` が MSRV を満たさないと言う**：`cargo +stable` で実行する。
- **vitest が構文エラーで落ちる**：Node 16 で動いている。Node 22 で実行する。
- **実機でボタンを押しても何も起きない**：Tauri のコマンドが `src-tauri/build.rs` と `src-tauri/capabilities/default.json` に登録されていない（レビュー R-01）。`build-and-test-summary.md` の「残っている項目」を見る。
