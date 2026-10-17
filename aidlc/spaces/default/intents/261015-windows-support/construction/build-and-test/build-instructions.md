# Build Instructions — 261015-windows-support

IB-16「Windows 対応」のビルドの手順。Part A（PR #23、`main` にマージ済み）と Part B（PR #24、枝 `claude/windows-support`）を合わせた状態が対象。Windows でしか確かめられないもの（Windows 向けのコンパイル、NSIS のインストーラ）は、CI の `Tauri app (windows-latest)` と Release dry run で確かめる。

## 前提

| もの | 版 | 備考 |
|------|----|------|
| Rust | stable（確認時 1.97.0） | CI は `dtolnay/rust-toolchain`（stable）。Windows は `x86_64-pc-windows-msvc`（Visual Studio Build Tools の「C++ によるデスクトップ開発」） |
| Node.js / npm | 22 系（確認時 v22.22.0） | CI は `node-version: 22` |
| cargo-llvm-cov | 0.9.1 | カバレッジの測定（Linux だけ。CI と同じ版） |
| cargo-deny | CI と同じもの | 依存関係の検査 |
| Tauri のビルドに要る OS の部品 | macOS は Xcode Command Line Tools、Linux は webkit2gtk など、Windows は WebView2（Windows 11 に同梱）と NSIS（Tauri CLI が初回の `--bundles nsis` で取得） | GUI を動かすのは開発者の手元の macOS と Windows 11 |

## 依存関係を入れる

```sh
npm ci                 # 画面側（package-lock.json どおり）
cargo fetch            # Rust 側（Cargo.lock どおり）
```

この作業で足した依存は `windows-sys 0.61`（`cfg(windows)` だけ。Tauri が既に使っている版）だけ。`libc` は `cfg(unix)` だけの依存に移した。画面側の依存は増えていない。

## 環境の準備

- 自動テストとビルドに環境変数・設定ファイル・手元のサービスは要らない。実際の AWS には接続しない（project.md の Forbidden）。
- 改行は `.gitattributes` で LF に固定している。Windows で取り出しても `cargo fmt --check` と `prettier --check` が同じ結果になる。
- 手元の確認（実際の AWS、GUI）だけは、開発者本人の AWS の設定と認証情報を使う。手順は `integration-test-instructions.md` の「手元の確認」を見る。

## ビルド

```sh
cargo build --workspace            # ライブラリ（local-sights-core）とアプリ（local-sights）
npx tsc --noEmit                   # 画面の型検査
npx vite build                     # 画面のバンドル
npm run check:version              # 版が 0.2.0 でそろっていること（Cargo.toml・Cargo.lock・package.json・tauri.conf.json）
```

インストーラを手元で作るとき（配布物は CI で作る。手元は確認用）：

```sh
npm run tauri build -- --bundles dmg    # macOS：target/release/bundle/dmg/*.dmg
npm run tauri build -- --bundles nsis   # Windows：target/release/bundle/nsis/*-setup.exe
```

アプリを手元で起動するとき：`npm run tauri dev`（macOS・Windows とも）。

## ビルドの確かめ方

| 確かめること | コマンド | 期待 |
|--------------|----------|------|
| 書式 | `cargo fmt --all --check`、`npx prettier --check .` | 終了コード 0 |
| リンタ | `cargo clippy --workspace --all-targets`、`npx eslint .` | 終了コード 0（clippy の警告はエラー扱いにしない：team.md） |
| アプリのビルド | `cargo build -p local-sights` | 終了コード 0（Linux・macOS・Windows の各 OS で。手元の OS 以外は CI） |
| 画面のビルド | `npx vite build` | 終了コード 0 |
| 版 | `npm run check:version` | `Version 0.2.0 is the same in ...` |
| Windows のコンパイルと NSIS | CI の `Tauri app (windows-latest)` と Release dry run の `Build Windows installer (no publish)` | 成功。dry run の成果物 `local-sights-windows-dry-run` に `*x64-setup.exe` が 1 つ |
| リリースのワークフローの並び | `npx vitest run scripts/release/signing.test.ts` | 31 件成功（`build-macos` → `build-windows` → `publish` の並びと署名の検証の位置） |

## よくあるつまずき

- **Linux では Windows のコードをコンパイルできない**：`cargo check --target x86_64-pc-windows-msvc` はリンカがないため通らない。`cfg(windows)` のコード（`src-tauri/src/sso_cli/windows.rs`、`browser.rs` の Windows 側、キャッシュの Windows のテスト）は CI の Windows のジョブで確かめる。
- **`check:version` の失敗**：版の欄のどれかだけを変えたとき。5 つのファイルの版をそろえ、`cargo update -w` を忘れない。
- **Windows で `npm run tauri build` が NSIS を取りに行く**：初回だけ。社内のネットワークで取得が止まるなら、Tauri の文書の手順で NSIS を手で置く。
- **Windows で `cargo build` が MSVC のリンカを見つけられない**：Visual Studio Build Tools の「C++ によるデスクトップ開発」を入れる。
- **ディスクが足りない（Linux の作業環境）**：`target/debug/incremental` と古い `target/debug/deps` の大きなファイルを消す。
- **Linux で `cargo build -p local-sights` が webkit2gtk を見つけられない**：`libwebkit2gtk-4.1-dev` などの OS の部品を入れる（CI の Ubuntu のジョブと同じ）。
