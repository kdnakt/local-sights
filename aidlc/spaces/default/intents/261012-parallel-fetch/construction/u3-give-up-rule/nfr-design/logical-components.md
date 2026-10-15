# Logical Components — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/nfr-requirements/`、`construction/u3-give-up-rule/functional-design/`、`inception/domain-design/components.md`、`decisions.md`（ADR-007）。まとめの確認は `nfr-design-questions.md`。

U3 は新しい部品を足さず、数えの持ち主を移す。依存は StreamSchedule → EventFetcher の片方向（EventFetcher は StreamSchedule を知らない）。

| 部品 | U3 での変更 | 失敗の範囲 |
|------|-------------|------------|
| StreamSchedule | 数え・本数（`GIVE_UP_LIMIT` = 3、`with_limit`）・終わり方の分け方を持つ。`finish(index, StreamFetchEnd)` で受け取る。あきらめの記録 `GiveUp` を `ScheduleOutcome.give_up` に載せる（`ScheduleOutcome` は Clone）。あきらめた後は数えを動かさない | 取得 1 回 |
| RetryPolicy | `max_consecutive_exhausted`・`ExhaustionStreak`・`StreamEnding` をなくす。1 回の要求の再試行と止める信号だけ | 1 回の要求 |
| EventFetcher | `streak_ending` をなくす。`StreamFetchEnd::Failed { exhausted, failure: ApiFailure }` にして、使い切りの終わり方が失敗の内容を必ず持つ | 1 本のストリーム |
| FetchCoordinator | 終わり方の変換をなくし、`StreamSchedule::finish` にそのまま渡す。`RetryPolicy` から本数を読まない。FetchJob への記録の仕方は変えない（BR3.7） | 取得 1 回 |
| 確認用プログラム・Tauri アプリ | 変えない（`RetryPolicy::default()` の使い方は同じ） | — |

## Assumptions & Open Questions

None.
