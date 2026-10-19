# Performance Test Instructions — Logs Insights 互換のクエリ（IB-11）

NFR1（クエリの速さ）と NFR2（大量件数でも落ちない）、それに今の速さの基準（C5：ログの画面は変えない）を確かめる。速さのテストは `#[ignore]` つきで、ふだんの `cargo test` と CI では流れない。手で release ビルドで流す。

## 1. 合否の数値（NFR1、OQ3）

Build and Test で人と決めた（2026-10-18、選択肢 C）。今の部分一致の絞り込みの基準と同じにした。決めた数値は下げない。

| 件数 | 合格の条件 |
|---|---|
| 取得済み 10 万件 | 実行ボタンからの評価の時間が 10 秒以内 |
| 取得済み 100 万件 | 実行ボタンからの評価の時間が 100 秒以内 |

- 測るのは、取得の時間を含まない評価の時間（NFR1）。
- 判定には、次の 4 種類のクエリのうちいちばん遅いものを使う：filter だけ、JSON の項目の filter、JSON の項目の sort、fields・filter・sort・limit。
- イベントは 8 つのストリームにまたがり、9 割が JSON のオブジェクト、1 割が JSON でない文（U2 の速さのテストの作り）。

## 2. 流し方

```bash
cargo test --release -p local-sights-core --lib -- --ignored --nocapture --test-threads=1
```

流れるテスト（9 件）：

| テスト | 確かめること |
|---|---|
| `log_view::tests::one_hundred_thousand_events_are_filtered_within_ten_seconds` ほか 2 件 | ログの画面の今の基準（10 万件 10 秒、100 万件 100 秒、ページの追加） |
| `timeline::tests::*`（2 件） | タイムラインの今の基準 |
| `query::evaluator::speed::four_queries_over_one_hundred_thousand_events` | NFR1（10 万件） |
| `query::evaluator::speed::four_queries_over_one_million_events` | NFR1（100 万件） |
| `query::evaluator::speed::a_cancel_during_a_json_sort_of_one_million_events` | 並べ替えの間の [Cancel] の遅れ（評価中も止められる） |
| `query::store::memory::both_screens_full_and_a_previous_result_evaluate_a_json_sort` | NFR2：2 つの画面に 100 万件ずつ、50 個のロググループ、前の結果 10 万行を持っても落ちないこと。Linux の `VmHWM` でピークのメモリを出す |

## 3. 記録

- 出力の時間を `test-results.md` の Target Verification Matrix に写す。測った機械（OS、release ビルドか）を添える。
- macOS でメモリを測るときは `/usr/bin/time -l` でピークを見る（U3 の code-summary）。
