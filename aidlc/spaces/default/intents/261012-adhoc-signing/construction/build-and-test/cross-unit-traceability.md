# Cross-Unit Traceability — リリースの .app をアドホック署名する

作業単位は分けないワークフロー（スコープ express）。`construction/code-generation/traceability.json` と `inception/requirements-analysis/requirements.md` を突き合わせた。User Stories のステージは実行していないため AC はない。

## 判定

**PASS**：要件の ID 16 件すべてが `OK` または理由つきの `N/A` で覆われ、`OK` の対象ファイルはすべて存在する。覆われていない ID はない。

## ID ごとの対応

| ID | 内容 | 状態 | 担当 | 対象ファイル |
|---|---|---|---|---|
| FR1.1 | ビルドステップに `APPLE_SIGNING_IDENTITY: "-"` | OK | code-generation | `.github/workflows/release.yml` |
| FR1.2 | `tauri.conf.json` は変えない | N/A（変更しないことが要件。差分で確認） | code-generation | `src-tauri/tauri.conf.json` |
| FR1.3 | `release-dry-run.yml` は変えない | OK | code-generation | `scripts/release/signing.test.ts` |
| FR1.4 | 「unsigned」の文言を改める | OK | code-generation | `.github/workflows/release.yml` |
| FR2.1 | `codesign --verify` のステップを足し失敗させる | OK | code-generation | `.github/workflows/release.yml` |
| FR2.2 | `Signature=adhoc` を確認する | OK | code-generation | `.github/workflows/release.yml` |
| FR2.3 | 検証ステップも秘密情報を使わない | OK | code-generation | `.github/workflows/release.yml` |
| FR3.1 | install.md「Before you install」の書き直し | OK | code-generation | `docs/install.md` |
| FR3.2 | install.md「First launch」を GUI だけの手順に | OK | code-generation | `docs/install.md` |
| FR3.3 | `xattr` を予備手順として残す | OK | code-generation | `docs/install.md` |
| FR3.4 | releasing.md と README.md の文言をそろえる | OK | code-generation | `docs/releasing.md`（README.md も変更済み） |
| FR3.5 | releasing.md のリリース後の確認手順 | OK | code-generation | `docs/releasing.md` |
| FR3.6 | 文書は英語で書く | OK | code-generation | `docs/install.md` |
| NFR1 | 秘密情報の不使用 | OK | code-generation | `.github/workflows/release.yml` |
| NFR2 | 既存の CI の緑を保つ | OK | code-generation | `scripts/release/signing.test.ts`（結果は `test-results.md`） |
| NFR3 | 再実行の安全性（`--clobber` の分岐を壊さない） | OK | code-generation | `.github/workflows/release.yml` |

## 覆われていない要素

なし。
