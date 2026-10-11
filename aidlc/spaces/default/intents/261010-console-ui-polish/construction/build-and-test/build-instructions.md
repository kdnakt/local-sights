# Build Instructions — 画面を AWS マネジメントコンソールに寄せる

対象：作業ツリー直下の Rust ワークスペース（`crates/local-sights-core`、`src-tauri`）と React 画面（`src/`）。実際の AWS には接続しない。

## 前提

| 道具 | 版 | 備考 |
|---|---|---|
| Rust | stable（Cargo.lock の MSRV 1.94.1 以上） | 手元の既定が古い場合は `cargo +stable …` または `RUSTUP_TOOLCHAIN=stable` |
| Node.js | 22 系 | CI と同じ。Node 16 では vitest が動かない |
| npm | Node 22 に付属 | |
| macOS | Xcode Command Line Tools | Tauri（WKWebView）のビルドに必要 |

## 依存の導入

```
npm ci
```

Rust の依存は初回の `cargo build` で取得される。今回、Rust の直接依存は追加していない（Cargo.lock はパッチ版の更新のみ）。画面側に Cloudscape 3 パッケージ（Apache-2.0）と `@resvg/resvg-js`（devDependency、アイコンの描き出し用）を追加した。

## ビルド

| 何を | コマンド |
|---|---|
| 画面の束（`dist/`） | `npx vite build` |
| Rust ワークスペース全体（Tauri アプリを含む） | `cargo +stable build --workspace` |
| 配布用の .app（任意） | `RUSTUP_TOOLCHAIN=stable npm run tauri build` |
| 名前とアイコンを確かめるための .app（速い） | `RUSTUP_TOOLCHAIN=stable npm run tauri build -- --debug` の後、`open "src-tauri/target/debug/bundle/macos/Local Sights.app"` |
| 開発用の起動 | `RUSTUP_TOOLCHAIN=stable npm run tauri dev` |

`tauri dev` は .app ではなく素の実行ファイルを起動するため、Dock とメニューバーの名前は `local-sights` のままになる。「Local Sights」の表示は .app で確かめる。

## ビルドの確認

| 確認 | コマンド | 合格の条件 |
|---|---|---|
| 型 | `npx tsc --noEmit` | 出力なし |
| リント（画面） | `npx eslint .` | 出力なし |
| 書式（画面） | `npx prettier --check .` | All matched files use Prettier code style |
| 書式（Rust） | `cargo +stable fmt --all --check` | 出力なし |
| リント（Rust） | `cargo +stable clippy --workspace --all-targets` | エラーなし（警告はエラー扱いにしない：team.md） |
| 依存（npm） | `npm audit --audit-level=high` | found 0 vulnerabilities |
| 依存（Rust） | `cargo deny check` | CI の `cargo-deny` ジョブ。手元に cargo-deny がない場合は CI の結果で確かめる |

## よくある問題

- **`styleText` がない等で vitest が落ちる**：Node 16 で動いている。Node 22 に切り替える（`nvm use 22`）。
- **`rustc 1.90.0 is not supported`**：既定のツールチェーンが MSRV 未満。`cargo +stable` を使う。
- **アイコンが古いまま**：Dock のキャッシュ。`killall Dock` で作り直す。
- **起動時にフォントが既定に落ちる**：CSP の `font-src 'self' data:` が `src-tauri/tauri.conf.json` にあるか確かめる。
