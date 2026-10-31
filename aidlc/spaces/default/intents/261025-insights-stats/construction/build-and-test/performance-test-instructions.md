# Performance Test Instructions — 261025-insights-stats

## 目標（requirements.md NFR1・NFR2、U2 BR2.6、U3 の確かめ方）

| 目標 | 中身 | 測り方 |
|---|---|---|
| U2 BR2.6 | 重いクエリ `stats count_distinct(@message), pct(latency, 99) by @logStream` の評価が、基準 `stats sum(latency) by @logStream` の 3 倍以内（作ったイベント 100 万件、利用者の答え Q1 A） | `#[ignore]` の測定のテスト `the_heavy_stats_takes_at_most_three_times_the_sum_by_stream` |
| NFR2 | 取得済み 100 万件に `stats` を実行しても落ちない。評価は塊ごとに進めて止められる | 同じ測定のテストと、止める信号のテスト |
| U3 Step 22 | 取得中の保持の写しの 1 回の鍵が 50 ms 以内（塊に分けたあと） | `query::store` の `#[ignore]` の測定のテスト |
| 既存の速さ（C5） | 10 万件の評価が 10 秒以内、100 万件が 100 秒以内（前の作業の目標） | 既存の `#[ignore]` の速さのテスト |
| NFR1 | 途中結果があっても取得の速さを下げない。合否の数値は手元の実測から決め、決めたら下げない（暫定の目安は 10%） | 開発者の手元で `fetch_check --query … --partial-results on|off` を 3 回ずつ流し、取得の終わりまでの時間と、最初のページから最初の途中結果までの時間を記録する |

## 実行のしかた

```bash
cargo test -p local-sights-core --release --lib -- --ignored
```

NFR1 は実際の AWS が要るため、開発者の手元で次のように流す（自動テストと CI では流さない）。

```bash
cargo run -p local-sights-core --release --example fetch_check -- --query 'filter @message like /x/' --partial-results on
cargo run -p local-sights-core --release --example fetch_check -- --query 'filter @message like /x/' --partial-results off
```

引き数のプロファイル・ロググループ・期間は今の `fetch_check` のとおり（`README.md`）。100 万件ほどを取得する期間を選ぶ。

## 記録

結果は `test-results.md` に、測った日・条件（件数・ロググループの数・期間）・3 回の値・中央値を書く。U3 のレビューの R-01（途中の view を作る間の store の鍵、100 万件で約 600 ms）は、この NFR1 の実測で取得が遅くならないかを見て扱いを決める。
