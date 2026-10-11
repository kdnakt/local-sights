# Build Instructions — リリース準備

## 依存関係を入れる

- Node.js 22.18 以降（`.mjs` から `.ts` をそのまま読み込むため）。CI は `actions/setup-node@v7` の `node-version: 22`。
- Rust stable（`rust-version = "1.85"` 以上）。`dtolnay/rust-toolchain@stable` と同じ。
- 画面側の依存関係：`npm ci`（`package-lock.json` どおり）。
- macOS で `.dmg` を作る場合：Xcode Command Line Tools。Linux で `src-tauri` をビルドする場合は WebKitGTK などの開発用パッケージ（README の Prerequisites）。

## 環境の準備

- 環境変数や秘密情報は要らない。ビルドもテストも AWS に接続しない。
- `.dmg` のビルドは署名しないため、Apple の証明書も要らない。

## ビルドのコマンド

| 目的 | コマンド | 出力 |
|------|----------|------|
| 版がそろっているか | `npm run check:version`（リリース時は `-- --tag vX.Y.Z`） | 1 行の結果。失敗なら終了コード 1 |
| 画面側の型検査とバンドル | `npm run build`（`tsc --noEmit && vite build`） | `dist/` |
| ライブラリ | `cargo build -p local-sights-core` | `target/` |
| アプリ本体（Linux は開発用パッケージが必要） | `cargo build -p local-sights` | `target/debug/local-sights` |
| 配布用の `.dmg`（macOS、Apple Silicon） | `npm run tauri build -- --bundles dmg` | `target/release/bundle/dmg/Local Sights_<版>_aarch64.dmg` |

## ビルドの確かめ方

- `npm run check:version` が `Version X.Y.Z is the same in ...` と表示して終了コード 0 で終わる。
- `npm run build` が `✓ built` で終わる。
- `.dmg` を作った場合は、`target/release/bundle/dmg/` に `aarch64` を含む `.dmg` がちょうど 1 つある。

## よくある問題

- `npm run check:version` が `.ts` を読めない：Node が 22.18 より古い。Node を上げる。
- 版を上げたら検査が `Cargo.lock` で失敗する：`cargo update -w` で `Cargo.lock` を更新する。
- Linux で `cargo build -p local-sights` が `webkit2gtk-4.1` を見つけられない：README の Prerequisites のパッケージを入れる（この作業のコンテナには入っていないため、アプリ本体のビルドは CI と macOS で確かめる）。
- macOS の `.dmg` 作成が止まる：Tauri の `.dmg` 作成は Finder の AppleScript を使う。CI ではジョブのタイムアウト（40 分）で止まる。再実行すると同じタグの `.dmg` を置き換える。
