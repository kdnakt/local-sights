# Unit Test Instructions — U5 中断（u5-cancel）

## テストの道具と設定

- ライブラリ：Rust の標準のテストと tokio のテスト。時間を使う結合テストは `#[tokio::test(start_paused = true)]` で時計を止める
- 偽物：`crates/local-sights-core/tests/support/fake_gateway.rs`（GetLogEvents の呼び出しの時間と時刻の記録がある）。DescribeLogStreams を遅らせる・返さない台本をこの作業単位で足す
- アプリ：`cargo test -p local-sights`（src-tauri のテスト）
- 画面：Vitest（jsdom、globals、`src/test/setup.ts`）と Testing Library
- 新しいテストの道具・設定ファイルは足さない

## この作業単位のテストの走らせ方

最初のテスト先行の手順（Step 3）より前に、次が動くことを確かめる（Step 2）。

| 対象 | コマンド |
|------|----------|
| 終わりの状態の決め方 | `cargo test -p local-sights-core --lib coordinator::` |
| AppSession | `cargo test -p local-sights-core --lib session::` |
| JobStatus | `cargo test -p local-sights-core --lib fetch_status::` |
| 打ち切りの結合テスト | `cargo test -p local-sights-core --test u5_cancel` |
| 閉じるときの今の結合テスト | `cargo test -p local-sights-core --tests` |
| Tauri のコマンドと権限 | `cargo test -p local-sights` |
| ボタン・注意・状態行 | `npx vitest run src/components/ConditionArea.test.tsx src/components/CancelNotice.test.tsx src/components/StatusLine.test.tsx src/App.test.tsx` |
| 文言の一覧 | `npx vitest run src/i18n/messages.test.ts` |
| すべて（画面） | `npm run test:coverage` |

## カバレッジの目標

- ライブラリ（local-sights-core）の行カバレッジ 80% 以上（`cargo llvm-cov -p local-sights-core --fail-under-lines 80`）
- 画面の行カバレッジ 80% 以上（`npm run test:coverage`）
- 下限は下げない。新しい公開の項目（`JobStatus::Cancelled`、decide_job_status の新しい入力、`FetchJob` の 2 項目、`AppSession::request_cancel`・`fetch_cancel_available`、SessionView の 3 項目）には必ずテストを書く（team.md）
- テストの量は Standard：部品ごとに 5〜8 件ほど、主な境目（取得の流れ、Tauri のコマンド、画面）にテスト

## 偽物と差し替え

- 実際の AWS には接続しない（NFR6.13）。FakeGateway の台本で、遅い応答・返さない応答・Throttled・AccessDenied を作る
- 止める信号は `stop_pair()` で作り、テストの中から Cancel と Close を送る
- 時計は止めた時計で進める。「時計を進めずに終わる」は、中断の後に時計を進めずに取得の終わりが来ることで確かめる

## テストのデータ

- ストリーム名は `s00` のような短い名前。N の例は functional-spec 3 節の例を小さくしたもの（たとえば 20 本、同時 8 本）と、数え方の確認に 1,000 本
- 失敗は `ApiFailure::new(FailureKind::AccessDenied, …)` などで、秘密に見える値を入れない
