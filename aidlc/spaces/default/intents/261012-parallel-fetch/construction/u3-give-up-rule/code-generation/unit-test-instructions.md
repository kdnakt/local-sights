# Unit Test Instructions — U3 あきらめの決まり（u3-give-up-rule）

## テストの道具と設定

- Rust の標準のテストと tokio のテスト。結合テストで時間を使うものは `#[tokio::test(start_paused = true)]` で時計を止める
- 結合テストは `crates/local-sights-core/tests/` に置き、偽物のゲートウェイ `tests/support/fake_gateway.rs`（U1 で呼び出しの時間と時刻の記録を足した版）を使う
- 新しいテストの道具・設定ファイルは足さない

## この作業単位のテストの走らせ方

最初のテスト先行の手順（Step 3）より前に、`cargo test -p local-sights-core --lib schedule::` が動くことを確かめる（Step 2）。

| 対象 | コマンド |
|------|----------|
| StreamSchedule の数えとあきらめ | `cargo test -p local-sights-core --lib schedule::` |
| EventFetcher の終わり方 | `cargo test -p local-sights-core --lib fetcher::` |
| RetryPolicy（数えの型をなくした後） | `cargo test -p local-sights-core --lib retry::` |
| 取得の流れの結合テスト | `cargo test -p local-sights-core --test u3_give_up_rule` |
| 今までのあきらめの結合テストが変わらないこと | `cargo test -p local-sights-core --test u3_fetch_flow --test u1_concurrent_fetch` |

## カバレッジの目標

- ライブラリ（local-sights-core）の行カバレッジ 80% 以上。CI の `Rust library (fmt, clippy, test)` のジョブが `cargo llvm-cov -p local-sights-core --fail-under-lines 80` で確かめる。下限は下げない
- 新しい公開の項目（`GIVE_UP_LIMIT`・`with_limit`・`GiveUp`・`ScheduleOutcome.give_up`）には必ずテストを書く（team.md）
- テストの量は Standard：StreamSchedule に 8 件ほど追加、主な境目（取得の流れ）に結合テスト

## 偽物と差し替え

- AWS の呼び出しは `CloudWatchLogsGateway` の trait の裏にあり、テストでは `FakeGateway` の台本で Throttled（使い切るまで）や AccessDenied を返す。実際の AWS には接続しない（NFR6.1）
- 終わる順の入れ替えは、ストリームごとの呼び出しの時間を変えて作る（止めた時計）
- 再試行の待ちは止めた時計で進める。ゆらぎは `FixedJitter(0.0)`
- StreamSchedule の単体テストは純粋な関数呼び出しだけで、非同期も偽物も使わない

## テストのデータ

- ストリーム名は `s00` のような短い名前。あきらめの例は nfr-design/security-design.md の正本どおり（8 本の枠は 1 本終わるごとに埋まる）
- 失敗は `ApiFailure::new(FailureKind::Throttled, …)` と `FailureKind::AccessDenied` を使い、秘密に見える値を入れない
