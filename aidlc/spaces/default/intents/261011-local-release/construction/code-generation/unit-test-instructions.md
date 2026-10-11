# Unit Test Instructions — リリース準備

## テストの枠組み

- 既存の Vitest（`vitest.config.ts`、環境 `jsdom`、`src/test/setup.ts`）をそのまま使う。
- この作業で `vitest.config.ts` の `include` に `scripts/**/*.test.ts` を足す。既存の `src/**/*.test.{ts,tsx}` は残す。
- `tsconfig.json` の `include` に `scripts` を足し、`npx tsc --noEmit` で型も確かめる。
- Rust 側のコードは変えないため、Rust の新しいテストはない。既存の `cargo test -p local-sights-core` が通ったままであることだけを確かめる。

## この作業のテストだけを実行するコマンド

Step 3 の前（テスト先行の最初の段階の前）に、実行できることを確かめる：

```bash
npx vitest run scripts/release
```

`scripts/release` にテストファイルがない間は「テストファイルが見つからない」で終わる（Vitest の実行自体はできることの確認）。Step 3 以降は次のコマンドで、この作業のテストだけを実行する：

```bash
npx vitest run scripts/release/versions.test.ts src/App.title.test.tsx
```

版の検査の入口の手動確認（Node 22.18 以降）：

```bash
npm run check:version
npm run check:version -- --tag v0.1.0
npm run check:version -- --tag v9.9.9   # 失敗すること
npm run check:version -- --tag v1.2     # 失敗すること
```

## テストの範囲（Standard）

- `scripts/release/versions.test.ts`：12 件（計画の Step 3 の 1〜12）。タグの形、各ファイルからの版の取り出し、そろっている・ずれている・タグと違う、の場合と、失敗の文の形。
- `src/App.title.test.tsx`：1 件。見出しが `Local Sights` と表示される。
- 既存のテストはすべて通ったままにする（スコープの下限）。

## カバレッジの目安

数値の下限は設けない（team.md）。ただし `scripts/release/versions.ts` の公開関数（`parseTag`、`readCargoWorkspaceVersion`、`readCargoLockVersions`、`readJsonVersion`、`checkVersions`）にはすべてテストを書く。

## モック・スタブの方針

- `versions.ts` は文字列を受け取って結果を返す純粋な関数だけにし、ファイルの読み込みは入口（`check-version.mjs`）に置く。テストではファイルの中身を文字列で直接渡すため、モックは要らない。
- 画面テストは既存のテストと同じく `@tauri-apps/api` の呼び出しをスタブにする（既存の `src/test/` の仕組みを使う）。
- 実際の AWS や GitHub には接続しない。

## テストデータ

- `Cargo.toml`・`Cargo.lock`・`package.json`・`tauri.conf.json` の中身は、テストの中に最小限の文字列として書く（実際のファイルは読まない）。
- `Cargo.lock` の例には、`local-sights` と `local-sights-core` のほかに、名前が似た別のパッケージ（例：`local-sights-extra`）を入れて、拾い間違えないことを確かめる。
