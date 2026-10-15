# Unit Test Instructions — U1 薄い一本・同時取得（u1-concurrent-skeleton）

## テストの道具と設定

- Rust の標準のテスト（`cargo test`）と tokio のテスト（`#[tokio::test]`）。時間を使うテストは `#[tokio::test(start_paused = true)]` で時計を止め、`tokio::time` の自動の進みで待ちを一瞬で進める（dev-dependencies の `tokio` の `test-util` は既にある）
- 結合テストは `crates/local-sights-core/tests/` に置き、偽物のゲートウェイ `tests/support/fake_gateway.rs` を使う
- 新しいテストの道具・設定ファイルは足さない

## この作業単位のテストの走らせ方

すべて `crates/local-sights-core`（と Tauri アプリ）の中の、この作業単位で足すか変えるモジュールに絞ったコマンドである。最初のテスト先行の手順（Step 4）より前に、`cargo test -p local-sights-core --lib retry::` が動くことを確かめる（Step 2）。

| 対象 | コマンド |
|------|----------|
| JobStatus の移動 | `cargo test -p local-sights-core --lib fetch_status::` |
| 止める信号と Retrier の待ち | `cargo test -p local-sights-core --lib retry::` |
| CallPacer | `cargo test -p local-sights-core --lib pacing::` |
| StreamSchedule | `cargo test -p local-sights-core --lib schedule::` |
| FetchStats と要約の 1 行 | `cargo test -p local-sights-core --lib fetch_stats::` |
| EventFetcher | `cargo test -p local-sights-core --lib fetcher::` |
| SDK の再試行の上書き | `cargo test -p local-sights-core --lib gateway::aws::` |
| 同時取得の結合テスト | `cargo test -p local-sights-core --test u1_concurrent_fetch` |
| 既存の流れのテスト（取り込みの変更の確かめ） | `cargo test -p local-sights-core --test u1_fetch_flow --test u3_fetch_flow --test u5_filter_flow --test u6_cache_flow` |
| 確認用プログラムの引数 | `cargo test -p local-sights-core --example fetch_check` |
| Tauri アプリ（止める信号の置き換え） | `cargo test -p local-sights` |

## カバレッジの目標

- ライブラリ（local-sights-core）の行カバレッジ 80% 以上。CI の `Rust library (fmt, clippy, test)` のジョブが `cargo llvm-cov -p local-sights-core --fail-under-lines 80` で確かめる（PR #20）。下限は下げない
- 新しいモジュール（`pacing`・`schedule`・`fetch_status`・`fetch_stats`）の公開関数には必ずテストを書く（team.md）
- テストの量は Standard：部品ごとに 5〜8 件、主な境目に結合テスト

## 偽物と差し替え

- AWS の呼び出しは `CloudWatchLogsGateway` の trait の裏にあり、テストでは `FakeGateway`（結合テスト）か、モジュールの中の小さな偽物（`coordinator.rs` の `FakeLogs` と同じ形）に差し替える。実際の AWS には接続しない（NFR6.1）
- `FakeGateway` に足すもの：1 回の呼び出しにかかる時間（ストリームごとの台本で指定、`tokio::time::sleep`）、呼んだ時刻（`tokio::time::Instant`）の記録、同時に出ている呼び出しの数とその最大の記録
- 時間：止めた時計で、呼び出しの時間・許可の間隔・再試行の待ちを進める。壁の時計には頼らない
- 再試行のゆらぎ：`FixedJitter(0.0)`
- SDK の再試行の上書きは、組み立てた設定の値（最大試行回数 1）を確かめる。クライアントで送信はしない

## テストのデータ

- ストリーム名は `s00`〜`s99` のような短い名前、時刻は 2024/01/02 03:04:05 UTC からの固定の値を使う
- ログの中身・ロググループ名に秘密の値に見えるものを使わない。アクセスキー ID の形の文字列が必要なときは、今のテストと同じく分割して組み立てる
- 速度の確かめの条件は functional-spec 5 節のとおり（東京・100 本・各 1 回・1 回 200 ミリ秒で 10 秒以下、10 本・200 ミリ秒で同時の最大 8、700 ミリ秒の呼び出しで同時の最大 8）
