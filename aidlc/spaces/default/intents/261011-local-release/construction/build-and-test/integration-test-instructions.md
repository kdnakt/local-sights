# Integration Test Instructions — リリース準備

## 対象とする境界

この作業の境界は 3 つある。

1. **版の検査の入口とリポジトリのファイル**：`scripts/release/check-version.mjs` が実際の `Cargo.toml`・`Cargo.lock`・`package.json`・`src-tauri/tauri.conf.json` を読み、`versions.ts` の判定を呼ぶ。
2. **CI とリリース用ワークフロー**：`ci.yml` が版の検査を実行し、`release.yml` が `verify` → `test` → `build-and-publish` の順に動く。
3. **ワークフローと GitHub Releases**：`gh release create` / `gh release upload --clobber` による公開。

## 実行の方法

### 1. 入口と実ファイル（この環境で実行できる）

```bash
npm run check:version                   # 成功（終了コード 0）
npm run check:version -- --tag v0.1.0   # 成功（終了コード 0）
npm run check:version -- --tag v0.2.0   # 失敗（終了コード 1、タグと版の不一致）
npm run check:version -- --tag v1.2     # 失敗（終了コード 1、タグの形の誤り）
```

### 2. ワークフローの定義（この環境で実行できる）

```bash
actionlint .github/workflows/release.yml .github/workflows/ci.yml
```

加えて、次をワークフローの定義で確かめる。
- `release.yml` のきっかけが `push` の `tags: ["v*"]` だけ。
- `build-and-publish` が `needs: [verify, test]`。
- `contents: write` が `build-and-publish` だけにある。
- `secrets.` の参照がない（`github.token` だけ）。

### 3. 実際の公開（この環境では実行できない）

マージ後に本人がタグ `v0.1.0` を `main` のコミットに付けて push し、`docs/releasing.md` のとおりに次を確かめる。
- Release ワークフローの 3 つのジョブがすべて成功する。
- GitHub Releases に下書きでない `v0.1.0` があり、`Local-Sights_0.1.0_aarch64.dmg` が 1 つ添付されている。
- 公開された `.dmg` を手元の Mac に入れて起動できる。

## カバレッジの目安

数値の下限はない（team.md）。境界 1・2 はこの工程ですべて確かめる。境界 3 はタグの push のときに確かめる。

## テストデータと環境

- 境界 1 はリポジトリの実ファイルをそのまま使う（版は 0.1.0）。
- AWS や GitHub の認証情報は使わない。
