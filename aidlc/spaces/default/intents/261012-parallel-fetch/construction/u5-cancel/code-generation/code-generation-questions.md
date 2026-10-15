# Code Generation Plan Approval

AI-DLC writes this file when it asks you to approve the plan. To answer here
instead of in chat, write your answer after `[Answer]:` and say done.

## Plan Approval

Approve the code plan for u5-cancel?

- Builds: 取得中の［取得］を同じ場所で［中断］に変え、押すと出ている呼び出しを待たずに打ち切って止める。取れたログは残し、読み終えていない本数 N（または一覧の前）を注意で知らせ、中断した取得はキャッシュに書かない（FR5、ADR-004、ADR-009）
- Touches: `crates/local-sights-core/src/`（fetch_status.rs、coordinator.rs、session.rs、テストの偽物と RecordingSink）、`examples/fetch_check.rs`、`src-tauri/`（lib.rs、build.rs、capabilities/default.json）、`src/`（ap...
- Tests: 純粋な判断（decide_job_status の表、AppSession の押せるかの判定と状態の変わり方）はテスト先行で約 15 件。取得の流れの打ち切りの結合テスト約 8 件、Tauri のコマンドのテスト、画面のテスト約 12 件は実装の後。閉じるときの今のテストは変えずに通し、行カバレッジはライブラリ・画面とも 80% 以上

Full plan: aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/code-generation/code-generation-plan.md
Test instructions: aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/code-generation/unit-test-instructions.md

[Approval Fingerprint]: sha256:v3:83551d6ca07681925b0e50def8531d4bf75b11954f919c9e43157cc6e2a168f8
[Planned Source]: 538671a4265333a6c972a88531c6ae9738f3d55143d94402a68a76b9f0e1ed32

- A. Approve Plan
- B. Request Changes
- C. I'll edit the files

[Answer]: A. Approve Plan
