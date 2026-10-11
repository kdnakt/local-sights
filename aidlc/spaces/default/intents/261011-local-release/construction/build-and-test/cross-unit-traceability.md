# Cross-Unit Traceability — リリース準備

## 判定

**FAIL（1 件が未カバー）**：要件 34 件のうち 33 件が `OK` で、対象のファイルはすべて存在する。FR3.2 だけが `Deferred`（実際の公開はタグの push まで確かめられない）であり、承認の際に判断してほしい。

作業単位はないため、カバーの持ち主はすべて Code Generation（作業単位なし）である。User Stories の工程は計画に含まれないため、`AC` の ID はない。

## 要件ごとのカバー

| ID | 状態 | 持ち主 | 対象のファイル |
|----|------|--------|----------------|
| FR1.1 | OK | code-generation | `src-tauri/tauri.conf.json` |
| FR1.2 | OK | code-generation | `src/App.title.test.tsx` |
| FR1.3 | OK | code-generation | `src/App.title.test.tsx` |
| FR2.1 | OK | code-generation | `scripts/release/versions.ts` |
| FR2.2 | OK | code-generation | `scripts/release/check-version.mjs` |
| FR2.3 | OK | code-generation | `scripts/release/versions.test.ts` |
| FR2.4 | OK | code-generation | `package.json` |
| FR2.5 | OK | code-generation | `.github/workflows/ci.yml` |
| FR3.1 | OK | code-generation | `.github/workflows/release.yml` |
| FR3.2 | **Deferred（未カバー）** | code-generation | 公開の手順は `.github/workflows/release.yml` にある。実際の公開はタグ `v0.1.0` の push のときに確かめる |
| FR3.3 | OK | code-generation | `.github/workflows/release.yml` |
| FR3.4 | OK | code-generation | `.github/workflows/release.yml` |
| FR3.5 | OK | code-generation | `.github/workflows/release.yml` |
| FR3.6 | OK | code-generation | `.github/workflows/release.yml` |
| FR3.7 | OK | code-generation | `docs/releasing.md` |
| FR3.8 | OK | code-generation | `.github/workflows/release.yml` |
| FR4.1 | OK | code-generation | `.github/workflows/release.yml` |
| FR4.2 | OK | code-generation | `docs/install.md` |
| FR4.3 | OK | code-generation | `docs/install.md` |
| FR5.1 | OK | code-generation | `docs/install.md` |
| FR5.2 | OK | code-generation | `docs/install.md` |
| FR5.3 | OK | code-generation | `docs/install.md` |
| FR5.4 | OK | code-generation | `docs/install.md` |
| FR5.5 | OK | code-generation | `docs/install.md` |
| FR5.6 | OK | code-generation | `docs/releasing.md` |
| FR5.7 | OK | code-generation | `README.md` |
| FR5.8 | OK | code-generation | `docs/install.md` |
| NFR1 | OK | code-generation | `.github/workflows/release.yml` |
| NFR2 | OK | code-generation | `.github/workflows/release.yml` |
| NFR3 | OK | code-generation | `.github/workflows/release.yml` |
| NFR4 | OK | code-generation | `.github/workflows/release.yml` |
| NFR5 | OK | code-generation | `src-tauri/tauri.conf.json` |
| NFR6 | OK | code-generation | `.github/workflows/ci.yml` |
| NFR7 | OK | code-generation | `.github/workflows/release.yml` |

## 未カバーの要素

- **FR3.2**：ビルドに成功したら、そのタグの GitHub Release を下書きにせず公開する。`release.yml` の `gh release create ... --verify-tag`（`--draft` なし）で実装済みだが、実際の公開はこのコンテナでは実行できない。要件の前提（「`v0.1.0` の Release が実際に公開されるのは、マージ後に本人がタグを push した後」）のとおり、タグの push のときに `docs/releasing.md` の手順で確かめる。
