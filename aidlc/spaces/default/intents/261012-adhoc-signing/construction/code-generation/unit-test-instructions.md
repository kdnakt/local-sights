# Unit Test Instructions — リリースの .app をアドホック署名する

## テストの枠組み

- 画面側と同じ Vitest（`vitest.config.ts` の `include` に `scripts/**/*.test.ts` が含まれている）。設定の追加は不要。
- テストの置き場所は `scripts/release/signing.test.ts`（新規）。対象は `scripts/release/signing.ts`（新規）の純粋な関数と、リポジトリの実ファイル。
- Rust 側（`cargo test`）は今回変更しないため、新しいテストは追加しない。既存のテストが通ることだけを確かめる。

## このユニットのテストの実行コマンド

最初のテスト先行のサイクル（Step 2）より前に、ランナーが動くことを次で確かめる：

```bash
npx vitest run scripts/release/versions.test.ts
```

このユニットのテストだけを実行する（Step 2 以降はこれを使う）：

```bash
npx vitest run scripts/release/signing.test.ts
```

## 期待するテストの範囲（Minimal）

- 要件 1 つにつき検証可能なテスト 1 件以上、部品（チェック関数）ごとに正常系 1 件以上。
- 純粋な関数の単体テスト：`readTauriBuildSigningIdentity`、`findSignatureCheckStep`、`findUnsignedClaims`、`terminalFallbackOnly` に正常系 1 件と境界 1 件ずつ（合計 6〜8 件）。
- 実ファイルに対する要件テスト（`describe("repository")`）：FR1.1、FR1.3、FR1.4、FR2.1/2.2、FR3.1/3.4、FR3.2/3.3、FR3.5 の各 1 件（合計 6 件程度）。
- 数値のカバレッジ下限は設けない（team.md）。

## モック／スタブ

- 不要。チェック関数は文字列を受け取る純粋な関数で、外部にも AWS にも接続しない。
- 実ファイルのテストは `readFileSync(new URL("../../<path>", import.meta.url))` でリポジトリ内のファイルを読むだけで、ネットワークも macOS のコマンド（`codesign`）も使わない。

## テストデータ

- 単体テストでは、`versions.test.ts` と同じく最小限の合成文字列（数行のワークフロー YAML、数行の Markdown）をテスト内で組み立てる。
- 実ファイルのテストが読むのは `.github/workflows/release.yml`、`.github/workflows/release-dry-run.yml`、`docs/install.md`、`docs/releasing.md`、`README.md`。

## 全体の確認（Step 6 の最後）

```bash
npx vitest run
npx prettier --check .
npx eslint .
npx tsc --noEmit
```
