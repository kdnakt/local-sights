# Build Instructions — ビルドの手順（全単位）

上流：U1〜U7 の `code-generation/code-generation-plan.md`・`unit-test-instructions.md`・`code-summary.md`。配布はソースからのビルドだけ（team.md の Deployment）。

## 構成

| 部分 | 場所 | ビルドの道具 |
|------|------|--------------|
| ライブラリ（GUI 非依存） | `crates/local-sights-core/` | `cargo`（Rust 2021、ワークスペースの一員） |
| Tauri のつなぎ（Rust） | `src-tauri/` | `cargo`（パッケージ名 `local-sights`）。macOS では WebKit、Linux では WebKitGTK 4.1 と GDK 3 が要る |
| 画面（TypeScript + React） | `src/`、`index.html`、`vite.config.ts` | `npm` + Vite 7 + TypeScript 5.9 |

## 依存関係の準備

1. Rust の安定版ツールチェーン（`rustup`）。`rustfmt`・`clippy` のコンポーネントを入れる。
2. Node.js 20 以上と npm。リポジトリのルートで `npm ci` を実行する（`package-lock.json` を使う）。
3. macOS（MVP の対象 OS、NFR9）：Xcode Command Line Tools。Tauri の WebView は OS の WebKit を使うため追加のライブラリは要らない。
4. Linux で Tauri アプリまで組み立てる場合だけ：`libwebkit2gtk-4.1-dev`・`libgtk-3-dev`・`libayatana-appindicator3-dev`・`librsvg2-dev`（Debian/Ubuntu の名前）。ライブラリと画面のテストには要らない。

## 環境変数と設定ファイル

- 自動テストとビルドは実際の AWS に接続しない（project.md Forbidden）。AWS の認証情報・プロファイルは要らない。
- ディスクの空きが少ない環境では `CARGO_INCREMENTAL=0` を付ける（U5 以降の単位の手順と同じ）。
- 設定ファイルは `Cargo.toml`（ワークスペース）、`src-tauri/tauri.conf.json`（ウィンドウの最小サイズ 1024×640、CSP）、`src-tauri/capabilities/default.json`（画面が呼べるコマンドの一覧）、`vite.config.ts`・`vitest.config.ts`・`tsconfig.json`・`eslint.config.js`。手で変える必要はない。

## ビルドのコマンド

リポジトリのルートで、上から順に実行する。

```bash
# 1. 画面側：型検査と Vite のバンドル（dist/ に出力）
npm ci
npx tsc --noEmit
npx vite build

# 2. ライブラリ側：コンパイル（テストのビルドを含む）
CARGO_INCREMENTAL=0 cargo build -p local-sights-core --all-targets

# 3. Tauri アプリ：デバッグビルド（Rust のつなぎがライブラリと型で合うことの確認）
CARGO_INCREMENTAL=0 cargo build -p local-sights

# 4. 配布用（macOS、任意）：Tauri のバンドル
npm run tauri build
```

3 は macOS で実行する。WebKitGTK のない Linux のコンテナでは `gdk-sys` のビルドスクリプトが `Package gdk-3.0 was not found` で止まる（この環境で確認。`test-results.md`）。

## ビルドの検証

| 確認 | コマンド | 期待 |
|------|----------|------|
| 整形 | `cargo fmt --all --check`、`npx prettier --check .` | 差分なし（CI で必須。team.md Code Style） |
| 静的検査 | `cargo clippy -p local-sights-core --all-targets`、`npx eslint .` | エラーなし（clippy の警告はエラー扱いにしない） |
| 静的検査（アプリ全体、macOS で） | `cargo clippy --workspace --all-targets` | エラーなし |
| 画面のバンドル | `npx vite build` | `dist/index.html`・`dist/assets/*.js`・`*.css` ができる |
| アプリの組み立て（macOS で） | `cargo build -p local-sights` | `target/debug/local-sights` ができる |
| 起動（macOS で） | `npm run tauri dev` | 1024×640 以上のウィンドウが開き、上部バーにプロファイル・リージョン・タイムゾーン、左にロググループ一覧、右に条件と一覧が出る |

## よくあるつまずき

| 症状 | 原因と対処 |
|------|------------|
| `Package gdk-3.0 was not found` / `webkit2gtk-4.1` が見つからない | Linux に WebKitGTK・GTK の開発パッケージがない。ライブラリと画面の確認だけなら不要。アプリまで組み立てるなら上の 4 を入れる |
| `No space left on device` | `CARGO_INCREMENTAL=0` を付ける。`target/release` は速さの計測のあとに消す（`rm -rf target/release`） |
| `npx vitest` が `ResizeObserver is not defined` で落ちる | jsdom には ResizeObserver がない。テストは `src/test/setup.ts` と各テストの偽物で補っているので、新しいテストでも同じ偽物を使う |
| 実行時に `aws sso login` を促される | 想定どおり。アプリは認証情報を保存しないので、手元の AWS CLI で `aws sso login --profile <名前>` を済ませてから使う（FR1.5） |
| `cargo-deny` がない | 依存関係の脆弱性の検査は次の CI Pipeline のステージで設定する。手元で試すなら `cargo install cargo-deny` のあと `cargo deny check` |
