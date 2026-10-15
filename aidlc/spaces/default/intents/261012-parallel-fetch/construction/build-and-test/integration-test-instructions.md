# Integration Test Instructions — 261012-parallel-fetch

Test Strategy は `Standard`。主な境目（取得の流れ全体、Tauri のコマンド、画面とバックエンドの受け渡し）を結合テストで確かめる。

## テストの道具と設定

- ライブラリの結合テスト：`crates/local-sights-core/tests/*.rs`。tokio のテストで、時間を使うものは `#[tokio::test(start_paused = true)]` で時計を止めて進める。
- 偽物：`tests/support/fake_gateway.rs`（`FakeGateway`。呼び出しの時刻・遅れ・返さない応答・Throttled・AccessDenied の台本を持つ）と `tests/support/recording_sink.rs`（`FetchSink` に届いた知らせを記録する）。
- アプリ（Tauri）のテスト：`cargo test -p local-sights`。コマンドの一覧と権限の対応、止める信号の登録を確かめる。
- 画面の結合テスト：`src/App.test.tsx`（Vitest・jsdom・Testing Library）。`invoke` とイベントを差し替え、SessionView と `fetch-progress` の受け取りから表示までを確かめる。
- 実際の AWS には接続しない（project.md の Forbidden、各作業単位の NFR6.x）。

## 作業単位をまたぐ境目と、それを確かめるテスト

| 境目 | 関わる作業単位 | テスト |
|------|----------------|--------|
| 8 本同時の読み取り × 呼び出しの速さの上限 | U1・U2 | `tests/u1_concurrent_fetch.rs`、`tests/u2_call_pacing.rs` |
| 制限で間隔を広げる × 再試行の待ち × あきらめ | U2・U3 | `tests/u2_call_pacing.rs`、`tests/u3_give_up_rule.rs` |
| 中断 × 許可待ち・再試行の待ち・応答待ち | U2・U3・U5 | `tests/u5_cancel.rs` |
| 中断 × 閉じる（閉じるが常に強い） | U1・U5 | `tests/u5_cancel.rs`、`tests/u1_concurrent_fetch.rs` の閉じるテスト |
| 中断 × キャッシュ（当たり・外れ） | U5・既存のキャッシュ | `tests/u5_cancel.rs`、`tests/u6_cache_flow.rs` |
| 経過の記録 × 終わり方（Completed・Failed・Cancelled・Aborted） | U4・U5 | `src/session.rs` のテスト |
| 取得の要約の 1 行（`min_rate=`・`status=Cancelled`） | U1・U2・U5 | `src/fetch_stats.rs`、`src/coordinator.rs` のテスト |
| `cancel_fetch` コマンド × セッションのロック × 止める信号の登録 | U5 | `cargo test -p local-sights`（`src-tauri/src/lib.rs`） |
| SessionView（`elapsed`・`cancelling`・`cancelNotice`）× 画面の表示 | U4・U5 | `src/App.test.tsx`、`src/components/StatusLine.test.tsx`、`src/components/ConditionArea.test.tsx`、`src/components/CancelNotice.test.tsx` |
| 既存の取得・絞り込み・一覧の流れ（回帰） | 既存 | `tests/u1_fetch_flow.rs`、`tests/u3_fetch_flow.rs`、`tests/u5_filter_flow.rs`、`tests/u2_log_group_listing.rs` |

## 走らせ方

```sh
cargo test -p local-sights-core --tests        # ライブラリの結合テストすべて
cargo test -p local-sights-core --test u5_cancel   # 1 つのファイルだけ
cargo test -p local-sights                     # アプリ（Tauri のコマンドと権限）
npx vitest run src/App.test.tsx                # 画面の結合テスト
cargo test --workspace && npx vitest run       # まとめて（Construction の確認コマンドの一部）
```

## カバレッジの目標

- ライブラリの行カバレッジ 80% 以上：`cargo llvm-cov -p local-sights-core --fail-under-lines 80`（結合テストを含む）
- 画面の行カバレッジ 80% 以上：`npm run test:coverage`
- 下限は下げない（team.md の Testing Posture）。

## テストのデータ

- ストリーム名は `s00` のような短い名前。本数は 0・1・5・8・20・100・1,000 本の例。
- 失敗は `ApiFailure::new(FailureKind::AccessDenied, …)` などで作り、秘密に見える値を入れない。
- 時刻は止めた時計で進めるので、テストは実時間に左右されない。
