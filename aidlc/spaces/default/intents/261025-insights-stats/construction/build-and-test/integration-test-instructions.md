# Integration Test Instructions — 261025-insights-stats

## ねらい

4 つの作業単位がつながる境界を確かめる。どれも偽の取得元（`CloudWatchLogsGateway` の trait の裏）を使い、実際の AWS には接続しない。

| 境界 | 何を確かめるか | テストの場所 |
|---|---|---|
| 読み取り → 評価 → 結果の形（U1・U2） | `stats` を含むクエリが aggregates の結果になり、stats のあとの `filter`・`sort`・`limit` と既定 10,000 行の打ち切りが効く | `crates/local-sights-core/src/query/evaluator.rs`、`runner.rs` |
| 取得 → 途中の評価 → 最終結果（U3 × U1・U2） | 途中結果が版の順に届き、並びが最終結果と同じ決まり。stats と stats のあとの命令も途中で同じ。2 回数えない・落とさない | `query/runner.rs`、`query/store.rs` |
| 中断（U3） | 取得中の［Cancel］で中断後の結果、2 回目の［Cancel］、最終の評価中の［Cancel］ | `query/runner.rs`、`insights_session.rs` |
| 中核 → Tauri → 画面（U1〜U3） | 版つきの読み出し、`find_query_row`、`get_query_catalog`、権限の一覧の一致 | `src-tauri/src/lib.rs`、`src/commandRegistration.test.ts` |
| 画面の部品のつながり（U1〜U3） | 2 つの形の表、途中と中断の印、読み上げ、rowKey で開いた行と先頭の行を保つ、書き出しの通知 | `src/components/LogAnalyticsPage.test.tsx`、`QueryResultTable.test.tsx`、`src/App.test.tsx` |
| 確認用プログラム（U4 × U1〜U3） | 集計の結果の比べ方、組の読み方、報告、書き出しのスクリプトの keyorder（偽の CLI） | `crates/local-sights-core/examples/compat*` |
| ログ管理の画面（変えないもの） | 取得・表示・中断が前と同じ | 既存の `coordinator.rs`・`session.rs` のテスト |

## 実行のしかた

```bash
cargo test --workspace --all-targets
npx vitest run
```

## カバレッジ

```bash
cargo llvm-cov -p local-sights-core --lib --summary-only   # 中核ライブラリ、80% 以上
npx vitest run --coverage                                   # 画面、80% 以上
```

計測は CI の Linux のランナーで行い、下限は下げない（team.md Testing Posture）。Rust のテストは CI の Linux・macOS・Windows の各ランナーで流す。

## 手元で行うつながりの確かめ（自動テストの外）

実際の AWS と本物の GUI が要るため、開発者の手元で行う（team.md Walking Skeleton の 2・3）。

1. macOS と Windows 11 で、アプリを起動し、`stats` のクエリ（`stats count(*) by bin(5m)`、U2 の関数、`stats count(*) as n by level | filter n > 10 | sort n desc | limit 5`）を実行して結果が出ること。
2. 取得に時間のかかる期間で、途中結果が出て差し替わり、最終結果になること。［Cancel］で中断の印つきの結果が残ること。ログ管理の画面が前と同じに動くこと。
3. コンソールとの比較（`insights-export.sh` → `compat_check`）。手順は `README.md`、結果は `u4-compat-stats/code-generation/code-summary.md` の表に記入する。
