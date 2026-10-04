## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T07:26:48Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/App.tsx rowsForSession/applySession, src-tauri/src/lib.rs start_fetch | start_fetch は戻り値なし（Result of unit）になり、画面は応答から行を触らない。rowsForSession は null でない別の jobId のときだけ行を空にするため、応答の遅延で現ジョブの行が消えることはなくなった。順序逆転のテストも App.test.tsx に追加済み（vitest 全通過）。 | なし | Resolved |
| R-02 | Minor | crates/local-sights-core/src/failure.rs build_safe_detail | profile/logGroup/logStream は redact_access_key_ids のみ、他のフィールドは redact_secrets を通す形に変更済み。72 件のライブラリテストが通過。 | なし | Resolved |
| R-03 | Minor | src-tauri/capabilities/default.json | permissions は core:event:default と 3 コマンド（allow-get-session, allow-update-input, allow-start-fetch）のみ。build.rs の AppManifest と一致し、cargo build と生成済み capabilities.json で確認。listen/unlisten/invoke は維持される。 | なし | Resolved |
| R-04 | Minor | src-tauri/src/lib.rs start_fetch / recover_if_still_fetching | JoinHandle を監視する 2 つ目のタスクが、終了後に abort_fetch_with_failure を呼ぶ。Fetching のときだけ Failed(Other) に遷移し、正常終了済みなら何もしない（false を返す）ため競合しても安全。ログには安全な詳細のみ。 | なし | Resolved |
| R-05 | Minor | README.md 手動 GUI 確認 | 省略記号（…）の確認を含む手動 GUI チェックリストを追加済み。 | なし | Resolved |
| R-06 | Minor | src/App.tsx rowsForSession, src/components/StatusLine.tsx | 2 回目以降の取得では、begin_fetch 直後（currentJobId が null の session-changed）から on_started までの間、前回の行が残り、Fetching 表示の件数も前回の行数（rows.events.length）になる。on_started で別 jobId が来れば空になるため短時間だが、「取得し直すと前の行を置き換える」と一瞬だけ食い違う。 | 任意：Fetching かつ currentJobId が null のあいだは行を空にするか、Fetching 表示の件数を session.eventCount（begin_fetch で 0）にする。R-01 の順序逆転テストを壊さないこと。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test (core lib) | PASS 72 件 | failure と session の新規テストを含め通過 |
| cargo test u1_fetch_flow | PASS 8 件 | 取得フローに回帰なし |
| cargo fmt --check | PASS | 差分なし |
| cargo clippy --workspace --all-targets | PASS | 出力に警告・エラーなし |
| cargo build -p local-sights | PASS | capability と AppManifest が整合 |
| npx vitest run | PASS | 順序逆転・ジョブ ID なしビューのテストを含む |
| npx tsc, prettier, eslint | PASS | 指摘なし |
| テスト外の unwrap/expect 検索 | 該当なし | session.rs の unwrap はテストモジュール内のみ |

### Summary

前回の 5 件の指摘はすべて解消され、修正による重大な回帰は見つからなかった。残るのは 2 回目以降の取得直後に前回の行が一瞬残る軽微な点（R-06）のみで、ブロックしない。
