# Unit Test Instructions — U1 薄い一本（u1-stats-skeleton）

## テストの道具と設定

- Rust：標準の `cargo test`。テストは各モジュールの中の `#[cfg(test)] mod tests` に書く（今の書き方）。新しい依存は足さない。
- 画面：Vitest（`vitest.config.ts` は今のまま）。部品のテストは `src/components/*.test.tsx`、Testing Library を今のまま使う。
- 自動テストは実際の AWS に接続しない。取得は今の偽のゲートウェイ（`CloudWatchLogsGateway` の trait の裏）に差し替える。

## この作業単位のテストの実行のしかた

この作業単位のテストだけを、次のコマンドで実行する（Step 1 で、作業の前に通ることを確かめる）。

```bash
cargo test -p local-sights-core --lib query::
cargo test -p local-sights-core --lib insights_session
cargo test -p local-sights-core --lib export::
cargo test -p local-sights --lib
cargo test -p local-sights-core --example compat_check
npx vitest run src/components/QueryResultTable.test.tsx
```

- `query::` は plan・parser・新規 stats・evaluator・store・runner のテストを含む。
- 純粋なロジックの段（Step 3・5・7・8・10）では、テストを書いたあとに上の 1 行目を実行し、新しいテストが失敗することを確かめてから実装する。

## テストの量（Standard）

| 部品 | テストの数の目安 | 中身 |
|---|---|---|
| QueryPlan | 5 | Stats の形、結果の形と列の導出 |
| QueryParser（stats・bin の字句） | 8 以上 | 書き方、列の名前、bin の単位と上限、Unsupported と Syntax の分け方、今のエラーが変わらないこと |
| StatsAggregator | 12 以上 | グループ化、各関数、bin、0 件、並び |
| format_aggregate | 8 | 丸めの境界の表 |
| QueryEvaluator | 6 | Aggregate の段、打ち切りと Columns の段をしない、止める信号、events は今と同じ |
| QueryStore・QueryRunner | 2 | aggregates の道 |
| InsightsSession | 6 | 見出し・行・詳細・時刻の整形・書き出しの元・0 行 |
| exporter | 3 | 集計の行の CSV・Markdown、events は今と同じ |
| compat | 2 | events は比べる、aggregates は報告だけ |
| QueryResultTable（Vitest） | 6 | 2 つの形の表示、`by @message`、`by @timestamp`、詳細、0 行 |

各部品で、普通の場合と、少なくとも 2 つの端の場合（0 件、値なし、境界の値）を確かめる。

## カバレッジ

- 中核ライブラリ（`cargo-llvm-cov`）と画面（Vitest のカバレッジ）の行カバレッジは 80% を下回らない（CI の Linux のランナーで測る）。下限は下げない。
- 新しい `stats.rs` は 90% 以上を目安にする（純粋なロジックのため）。

## モックとスタブ

- 取得：今の偽のゲートウェイ（テストの中で `LogEvent` の並びを返す）を使う。本物の AWS は使わない。
- 時刻：`InsightsSession` の `Instant` と、タイムゾーン（`chrono_tz::Tz`）は引き数で渡す（今のまま）。
- 画面：`api.ts` のコマンドは今のテストと同じく差し替える（`src/test/fixtures.ts` の結果の見本に、aggregates の見本を足す）。

## テストのデータ

- イベントは、テストの中で `LogEvent` を組み立てる（`level`・`latency` などの JSON のメッセージ）。ログの中身の実物はリポジトリに置かない。
- bin のテストは、決まったミリ秒（例：2025-08-24 16:00:00 UTC の前後）を使い、タイムゾーンは UTC と Asia/Tokyo の 2 つで確かめる。
