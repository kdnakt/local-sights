# Code Generation Plan Approval

AI-DLC writes this file when it asks you to approve the plan. To answer here
instead of in chat, write your answer after `[Answer]:` and say done.

## Plan Approval

Approve the code plan?

- Builds: 認証エラーの表示に「SSO にログイン」ボタンを足し、アプリが AWS CLI の `aws sso login` をシェルなしで起動して、中央のダイアログで待機・URL・確認コード・キャンセル・5 分の時間切れを扱う。成功したら SDK のクライアントを捨てて一覧を読み直す
- Touches: crates/local-sights-core（catalog の SSO 判定、新規 sso_login.rs、session.rs、gateway/aws.rs）、src-tauri/src（新規 sso_cli.rs、lib.rs）、src/（api.ts、新規 SsoLoginButton・SsoLoginModal、LogGroupPane、FetchError...
- Tests: コアの純粋ロジック約 30（先行）、Tauri 側の CLI 起動 約 8（偽の CLI スクリプトで、実装後）、画面の部品 約 15（実装後）。本物の AWS と AWS CLI は呼ばない

Full plan: aidlc/spaces/default/intents/261011-sso-login/construction/code-generation/code-generation-plan.md
Test instructions: aidlc/spaces/default/intents/261011-sso-login/construction/code-generation/unit-test-instructions.md

[Approval Fingerprint]: sha256:v3:496beb5f965f4747a99f1156569f7dbddc628a0b811fa815f89e16b256463a7b
[Planned Source]: 29d6fae6da60d37d5491955d1b8afc00b5fc753b6de75f7d3aa04d21198b545f

- A. Approve Plan
- B. Request Changes
- C. I'll edit the files

[Answer]: A. Approve Plan
