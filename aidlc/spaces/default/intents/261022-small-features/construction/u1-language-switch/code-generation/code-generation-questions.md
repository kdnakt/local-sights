# Code Generation Plan Approval

AI-DLC writes this file when it asks you to approve the plan. To answer here
instead of in chat, write your answer after `[Answer]:` and say done.

## Plan Approval

Approve the code plan for u1-language-switch?

- Builds: 設定ダイアログの「言語」の 3 択（OS に合わせる／日本語／English）を settings.json に保存し、起動し直さずに画面の文言を切り替える薄い一本
- Touches: `crates/local-sights-core/src/cache/settings.rs`・`session.rs`、`src-tauri/src/lib.rs`、`src/main.tsx`・`App.tsx`・`api.ts`・`i18n/messages.ts`・`components/SettingsDialog.tsx`・`test/fixtures.ts`
- Tests: 新規およそ 26 本（Rust の純粋ロジック 8 本をテスト先行、Rust の境界 6 本・画面 12 本は実装後）。既存の約 480 本は緑のまま

Full plan: aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/code-generation/code-generation-plan.md
Test instructions: aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/code-generation/unit-test-instructions.md

[Approval Fingerprint]: sha256:v3:5ba4f50a4687f63ceb6453cf31ed3bf0957e6bf01e6035776441c619db7eede4
[Planned Source]: 037f7accd4e2274cb02659a57a9cad4550cbd21dc8262b561a2b6f6a03038b54

- A. Approve Plan
- B. Request Changes
- C. I'll edit the files

[Answer]: A. Approve Plan
