# Build Instructions — リリースの .app をアドホック署名する

上流：`construction/code-generation/code-generation-plan.md`、`unit-test-instructions.md`、`code-summary.md`。

## 前提

| 道具 | 版 | 備考 |
|---|---|---|
| Node.js | 22 | この確認では v22.22.0 |
| Rust | stable | この確認では cargo 1.97.0。アプリのコードは変えていないので、Rust 側は既存のまま通ることの確認だけ |
| macOS ランナー | GitHub Actions `macos-latest` | 署名ステップ（`codesign`）は macOS でしか動かない。手元の Linux / この環境では実行できない |

環境変数や設定ファイルは追加していない。`APPLE_SIGNING_IDENTITY` は Release ワークフローのビルドステップの `env` にだけ置き、秘密情報ではない。AWS の認証情報はビルドにもテストにも要らない。

## 依存関係の導入

```
npm ci
```

Rust の依存関係は `Cargo.lock` のとおりに `cargo` が取得する。今回の変更で依存関係は増えていない（npm・Cargo とも）。

## ビルド

```
npm run check:version
npx vite build
```

配布用の `.dmg` はこの変更の対象であるが、手元では作らない。`main` にタグ `vX.Y.Z` を押すと Release ワークフローが macOS ランナーで `npm run tauri build -- --bundles dmg` を `APPLE_SIGNING_IDENTITY=-` 付きで実行し、`.app` の署名を検証してから公開する。

## ビルドの確認

```
npx tsc --noEmit
npx eslint .
npx prettier --check .
cargo fmt --all --check
```

`release.yml` が YAML として正しいことは、Python の `yaml.safe_load` で `build-and-publish` の step 名を列挙して確かめた（`build the .dmg (ad-hoc signed)` → `the .app carries a valid ad-hoc signature` → `exactly one Apple Silicon .dmg` → `publish the Release ...` の順）。

## よくある問題

- **`npm ci` で `EBADENGINE` の警告が出る**：`jsdom` などが Node `^22.22.2` を求めるが、v22.22.0 でも導入とテストは問題なく動く。警告だけで止まらない。
- **Release ワークフローの署名検証ステップが失敗する**：ビルドステップの `env` に `APPLE_SIGNING_IDENTITY: "-"` が無い（付け忘れ）か、Tauri が署名しなくなった。`docs/releasing.md` の「When something fails」を見る。
- **vitest が構文エラーで落ちる**：Node 22 で実行する。
