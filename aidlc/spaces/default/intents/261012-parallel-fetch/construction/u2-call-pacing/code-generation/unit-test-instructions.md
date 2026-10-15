# Unit Test Instructions — U2 呼び出しの速さ（u2-call-pacing）

## テストの道具と設定

- Rust の標準のテストと tokio のテスト。時間を使うテストは `#[tokio::test(start_paused = true)]` で時計を止め、`tokio::time::advance` と自動の進みで待ちを一瞬で進める（dev-dependencies の `tokio` の `test-util` は既にある）
- 結合テストは `crates/local-sights-core/tests/` に置き、偽物のゲートウェイ `tests/support/fake_gateway.rs`（U1 で呼んだ時刻の記録を足した版）を使う
- 新しいテストの道具・設定ファイルは足さない

## この作業単位のテストの走らせ方

最初のテスト先行の手順（Step 3）より前に、`cargo test -p local-sights-core --lib pacing::` が動くことを確かめる（Step 2）。

| 対象 | コマンド |
|------|----------|
| CallPacer の下げ方・戻し方 | `cargo test -p local-sights-core --lib pacing::` |
| 要約の 1 行の `min_rate` | `cargo test -p local-sights-core --lib fetch_stats::` |
| 制限の知らせ（EventFetcher） | `cargo test -p local-sights-core --lib fetcher::` |
| 取得の流れの結合テスト | `cargo test -p local-sights-core --test u2_call_pacing` |
| U1 の同時取得のテストが変わらないこと | `cargo test -p local-sights-core --test u1_concurrent_fetch` |

## カバレッジの目標

- ライブラリ（local-sights-core）の行カバレッジ 80% 以上。CI の `Rust library (fmt, clippy, test)` のジョブが `cargo llvm-cov -p local-sights-core --fail-under-lines 80` で確かめる。下限は下げない
- `CallPacer` の新しい公開関数（`report_throttled`・`current_rate_per_second`・`lowest_rate_per_second`）には必ずテストを書く（team.md）
- テストの量は Standard：CallPacer に 10 件ほど、主な境目（取得の流れ）に結合テスト

## 偽物と差し替え

- AWS の呼び出しは `CloudWatchLogsGateway` の trait の裏にあり、テストでは `FakeGateway` の台本で Throttled を返す。実際の AWS には接続しない（NFR6.1）
- 時間：止めた時計で、許可の間隔・下げてよい時刻・5 秒ごとの戻し・再試行の待ちを進める。壁の時計には頼らない
- 再試行のゆらぎ：`FixedJitter(0.0)`
- CallPacer の単体テストでは、許可を求める・制限を知らせる順と時刻を、`tokio::time::advance` で決めて確かめる

## テストのデータ

- リージョンは東京（`ap-northeast-1`、上限 12）を主に使い、指定なし（上限 5）も 1 件使う
- ストリーム名は `s00` のような短い名前、時刻は U1 と同じ固定の値を使う
- 期待する速さと時刻は functional-spec.md の 3 章の例と 4 章の表のとおり
