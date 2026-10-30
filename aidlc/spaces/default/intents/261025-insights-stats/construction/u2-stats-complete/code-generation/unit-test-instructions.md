# Unit Test Instructions — U2 `stats` をそろえる（u2-stats-complete）

## テストの道具と設定

- Rust：標準の `cargo test`。テストは各モジュールの中の `#[cfg(test)] mod tests` に書く（今の書き方）。新しい依存は足さない。
- 画面：Vitest（`vitest.config.ts` は今のまま）。部品のテストは `src/components/*.test.tsx`、補助は `src/*.test.ts`。Testing Library を今のまま使う。
- 自動テストは実際の AWS に接続しない。取得は今の偽のゲートウェイ（`CloudWatchLogsGateway` の trait の裏）に差し替える。

## この作業単位のテストの実行のしかた

この作業単位のテストだけを、次のコマンドで実行する（Step 1 で、作業の前に通ることを確かめる）。

```bash
cargo test -p local-sights-core --lib query::
cargo test -p local-sights-core --lib insights_session
cargo test -p local-sights --lib
npx vitest run src/analytics.test.ts src/commandRegistration.test.ts src/i18n/messages.test.ts src/components/QueryEditor.test.tsx src/components/QueryResultTable.test.tsx src/components/LogAnalyticsPage.test.tsx
```

- `query::` は catalog（新規）・plan・parser・stats・evaluator のテストと、README と一覧の一致のテストを含む。
- 純粋なロジックの段（Step 3・5・7・8・10・20）では、テストを書いたあとに上の 1 行目を実行し、新しいテストが失敗することを確かめてから実装する。
- `src/analytics.test.ts` がまだなければ Step 19 で作る。Step 1 では、ほかのファイルだけで通ることを確かめる。
- 測定のテスト（Step 12）は、ふだんは流れない。手で次のように流す：`cargo test -p local-sights-core --release --lib query:: -- --ignored`

## テストの量（Standard）

| 部品 | テストの数の目安 | 中身 |
|---|---|---|
| QueryCatalog | 6 | 語の一覧・種類・supported と reserved・重複なし・大文字小文字 |
| QueryParser（12 の関数・pct・stats のあと） | 12 以上 | 関数名、pct の引き数と範囲、列の名前、列の参照とない列、fields と 2 つ目の stats、項目として読む語、前のエラーが変わらない |
| StatsAggregator（7 つの関数） | 16 以上 | count_distinct・stddev・sortsFirst/Last・earliest/latest の普通の場合と端の場合 |
| pct の順位 | 8 | p と n の境界の表、0 件、数に読めない値 |
| QueryEvaluator（仕上げ） | 8 | filter・sort・limit の順、打ち切りと件数、止める信号、前と同じ結果 |
| InsightsSession | 4 | 補完の候補、今の候補の文字、打ち切りの見出し |
| README の一致 | 1 | 3 つの行と一覧の一致 |
| Tauri | 2 | get_query_catalog の値、権限の一覧の一致 |
| Vitest | 12 前後 | 色分けの規則、一覧がないとき、「使える命令」の文、打ち切りの注記、文言の鍵、権限 |

各部品で、普通の場合と、少なくとも 2 つの端の場合（0 件、値なし、境界の値）を確かめる。

## カバレッジ

- 中核ライブラリ（`cargo-llvm-cov`）と画面（Vitest のカバレッジ）の行カバレッジは 80% を下回らない（CI の Linux のランナーで測る）。下限は下げない。
- `catalog.rs` と `stats.rs` は 90% 以上を目安にする（純粋なロジックのため）。

## モックとスタブ

- 取得：今の偽のゲートウェイ（テストの中で `LogEvent` の並びを返す）を使う。本物の AWS は使わない。
- 画面：`api.ts` のコマンドは今のテストと同じく差し替える。`getQueryCatalog` の見本の一覧を `src/test/fixtures.ts` に足し、取れない場合（拒否される）も差し替えで確かめる。
- 測定のテストのイベントは、テストの中で作る（`latency` の数と、`@message` の種類が多い JSON のメッセージ）。

## テストのデータ

- イベントは、テストの中で `LogEvent` を組み立てる（`level`・`latency`・`user` などの JSON のメッセージ）。ログの中身の実物はリポジトリに置かない。
- pct の表は、1 から n までの整数を逆順に入れて、並べ替えが効いていることも確かめる。
- earliest・latest は、同じ時刻のイベントを複数のストリームに置いて、並びの決まりを確かめる。
