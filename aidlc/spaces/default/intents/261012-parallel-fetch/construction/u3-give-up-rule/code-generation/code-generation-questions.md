# Code Generation Plan Approval

AI-DLC writes this file when it asks you to approve the plan. To answer here
instead of in chat, write your answer after `[Answer]:` and say done.

## Plan Approval

Approve the code plan for u3-give-up-rule?

- Builds: 使い切りの数え・あきらめる本数・終わり方の分け方を StreamSchedule に移し（ADR-007）、あきらめの記録 `GiveUp` を結果に載せ、あきらめた後は数えを動かさない形にする
- Touches: `crates/local-sights-core/src/schedule.rs`（数え・GiveUp）、`retry.rs`（数えの型と項目をなくす）、`fetcher.rs`（`StreamFetchEnd::Failed` に失敗の内容）、`coordinator.rs`（変換をなくす）、`tests/u3_give_up_rule.rs`（新規）
- Tests: StreamSchedule の単体テスト約 8 件の追加（テスト先行）と retry.rs から移す 2 件、偽物のゲートウェイによる結合テスト約 5 件。既存の結合テストは変えずに通し、行カバレッジ 80% 以上

Full plan: aidlc/spaces/default/intents/261012-parallel-fetch/construction/u3-give-up-rule/code-generation/code-generation-plan.md
Test instructions: aidlc/spaces/default/intents/261012-parallel-fetch/construction/u3-give-up-rule/code-generation/unit-test-instructions.md

[Approval Fingerprint]: sha256:v3:2f9906203434f5edf278733d480e70a7ea531201d964007c1f80685ab0b8738e
[Planned Source]: e6295615d3e4a3df65a4812df9303d70453a2c5c2936b9258980b5f090b41c08

- A. Approve Plan
- B. Request Changes
- C. I'll edit the files

[Answer]: A. Approve Plan
