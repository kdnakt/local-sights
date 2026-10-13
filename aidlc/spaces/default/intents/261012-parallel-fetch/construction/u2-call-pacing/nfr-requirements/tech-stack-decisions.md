# Tech Stack Decisions — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/functional-design/`、`codekb/local-sights/technology-stack.md`、U1 の `nfr-requirements/tech-stack-decisions.md`。まとめの確認は `nfr-requirements-questions.md`。

U2 は新しい技術を足さない。U1 で使っているものだけで作る。

| 分野 | 使うもの | 理由 |
|------|----------|------|
| 速さの状態と時計 | `std::sync::Mutex` と `tokio::time::Instant`（U1 の CallPacer と同じ） | 下げ・戻し・枠の予約を 1 つのロックの中で決める（entities.md の PacingState）。止めた時計でテストできる |
| 待ち | `tokio::time::sleep_until` と `tokio::select!`（biased、U1 と同じ） | 変えない |
| テスト | `#[tokio::test(start_paused = true)]`、`tests/support/fake_gateway.rs`（U1 で時間・時刻・同時の最大の記録を足した版） | 実際の AWS に接続せず時刻を確かめる（NFR6.1） |
| カバレッジ | `cargo-llvm-cov`（PR #20 の CI） | 行カバレッジ 80%（NFR7.2） |
| 依存 | 追加なし | cargo-deny の対象は変わらない |

## 却下した案

| 案 | 却下の理由 |
|----|------------|
| トークンバケットのクレートを足す（例：速さ制限のライブラリ） | 決まり（半分に下げる・5 秒ごとに 1 戻す・予約済みの枠を動かさない）を自分で持つほうが単純で、テストで時刻を確かめやすい。依存を増やさない |
| 下げ・戻しを別のロックやアトミックで持つ | 枠の予約と同じロックの中で決めないと、下げた直後の予約が古い速さを使う。U1 の 1 つのロックに入れる |

## Assumptions & Open Questions

None.
