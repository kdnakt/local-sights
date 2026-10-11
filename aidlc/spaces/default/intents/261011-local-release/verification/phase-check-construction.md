# Phase Check — Construction → Operation

## 判定

**条件付きで通過（未完了 1 件、承認済みの先送り）**

この計画には Operation の工程がなく、CI Pipeline がこの作業の最後の工程である。

## 確かめたこと

| 確認 | 結果 | 根拠 |
|------|------|------|
| すべての作業がビルド・テスト済み | 作業単位なし（1 回の実装）。実行できるビルドとテストはすべて成功 | `construction/build-and-test/test-results.md` |
| Code Generation の対応表に未解決がない | 34 件中 33 件が `OK`。FR3.2 が `Deferred` | `construction/code-generation/traceability.json` |
| 要件全体の対応（FR/NFR） | 33/34。FR3.2（Release を下書きにせずすぐ公開）が未カバー。実装はあり、実際の公開はタグの push まで確かめられない。Build and Test の承認の時点で人間が確認済み | `construction/build-and-test/cross-unit-traceability.md` |
| CI の門が Build and Test のコマンドを実行する | `ci.yml` が `cargo fmt --check`・`cargo test -p local-sights-core`・`tsc --noEmit`・`eslint`・`prettier --check`・`vitest run`・`vite build`・`npm audit`・`check:version` を実行する。`release.yml` が公開の前に `cargo test`・`vitest run` を実行する | `construction/ci-pipeline/quality-gates.md` |
| ユーザーストーリーの受け入れ基準 | User Stories の工程は計画にないため対象なし | — |

## 残っている点

- FR3.2：マージ後に本人がタグ `v0.1.0` を push し、`docs/releasing.md` の手順で、Release が下書きでなく公開されたことを確かめると完了する。その前に「Release dry run」で `.dmg` のビルドだけを確かめることもできる。
