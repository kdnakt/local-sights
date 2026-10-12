## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-12T03:07:47Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | src-tauri/build.rs（`AppManifest::new().commands(&[...])`）、src-tauri/capabilities/default.json（`permissions`） | 新しい 4 つのコマンド `start_sso_login`・`cancel_sso_login`・`dismiss_sso_login`・`open_sso_url` が、アプリのコマンド一覧（build.rs）にも、ウィンドウに許可する権限（capabilities/default.json の `allow-...`）にも加わっていない。このプロジェクトは各コマンドを明示的な権限にしており、`git status` でもこの 2 ファイルは変更されていない。実機では画面側の `invoke("start_sso_login")` などが権限エラーで拒否され、ボタンを押しても何も起きない。Rust と Vitest のテストは `invoke` を差し替えているので、この欠落を検出できず、全部緑のまま残っている。 | build.rs の `commands` に 4 つを追加し、capabilities/default.json に `allow-start-sso-login`・`allow-cancel-sso-login`・`allow-dismiss-sso-login`・`allow-open-sso-url` を追加する。あわせて、権限一覧と `invoke` 呼び出しが食い違わないことを確かめる仕組み（例：api.ts の `invoke` 名と capabilities の突き合わせテスト）か、少なくとも README の手動確認に「実機でボタンが反応する」を明記する。 | New |
| R-02 | Minor | .github/workflows/ci.yml（Rust library ジョブは `cargo test -p local-sights-core` のみ）、code-summary.md「CI の範囲」 | プロセス制御（SIGTERM/SIGKILL、時間切れ、グループ停止）の 12 テストは `src-tauri` の `sso_cli.rs` にあるが、CI は `local-sights` クレートのテストを実行しない。NFR4（子プロセスを残さない）の自動検証が CI で守られない。 | CI の macOS/Ubuntu ジョブに `cargo test -p local-sights` を足すか、足さない理由を残す。 | New |
| R-03 | Minor | src-tauri/src/lib.rs（`start_sso_login` / `cancel_sso_login`）、crates/local-sights-core/src/session.rs（`cancel_sso_login`） | キャンセル直後は状態が `Cancelled` になり、ボタンが再び押せる（`can_start`）一方、旧 CLI は猶予時間（最大 2 秒）まで生きている。この間に押し直すと、2 つの CLI が短時間並走し、旧試行の `running` 記録は新試行で上書きされる（旧試行の終了報告は試行番号で無視されるので状態は壊れない）。FR2.6「同時に 1 つ」を厳密には満たさない。 | 旧 CLI の終了が確認できるまで `can_start` を false に保つか、並走を許容する旨を仕様に残す。 | New |
| R-04 | Minor | crates/local-sights-core/src/sso_login.rs（`is_allowed_login_url`）、functional-spec BR3.1 | URL の許可リスト（`*.amazonaws.com` / `*.awsapps.com` のみ）は BR3.1 の「https:// で始まる最初の 1 つ」より厳しい。安全側の逸脱で、計画の Major 指摘への対応としては妥当だが、機能設計の文面には反映されていない。末尾の句読点除去で URL の末尾の `.` などを削る点も同様。 | BR3.1 の文面を実装に合わせて更新するか、逸脱として code-summary に明記済みであることを承認時に確認する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test --workspace | PASS（失敗 0） | 緑。ただし新コマンドの権限は検査対象外 |
| cargo clippy --workspace --all-targets | PASS（警告 0） | 問題なし |
| cargo fmt --all --check | PASS | 問題なし |
| npx tsc --noEmit | PASS | 問題なし |
| npx eslint . | PASS | 問題なし |
| npx vitest run | PASS（291 件） | `invoke` を差し替えているため、権限の欠落は検出できない |
| npx prettier --check src | PASS | 問題なし |

### Summary

コアの判定ロジック（ログイン先の解決、引数の組み立て、URL・コードの許可リスト）、CLI のプロセス管理、秘密情報の扱い（出力を保持・記録しない）、SSO の API を呼ばない点は、読んだ範囲で要件どおりで健全。ただし新しい Tauri コマンド 4 つが build.rs と capabilities に登録されておらず、実機では機能が動かない。ここを直すまでは承認すべきではない。
