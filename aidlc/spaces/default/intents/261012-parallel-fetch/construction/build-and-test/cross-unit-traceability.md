# Cross-Unit Traceability — 261012-parallel-fetch

## 判定

**FAIL（直接の対応では）**。数え上げた ID は 94 件（requirements.md の FR・NFR と、stories.md の 3 段の AC）。

| 区分 | 件数 | 中身 |
|------|------|------|
| 直接 OK | 49 | 作業単位の `code-generation/traceability.json` に status `OK` の行があり、対象のファイルがある |
| 間接 | 45 | code-generation の traceability.json に行がない。子の ID・受け入れ条件・別の PR・手元の確認を通じて実装と確認にたどれる |

このステージの決まりでは「code-generation の traceability.json に status `OK` の行がある」ことを対応ありとするため、間接の 45 件はすべて承認のときに示す指摘とする。

## 指摘

| ID | 指摘 | 中身 |
|----|------|------|
| BT-TR-1 | 親の ID が code-generation の対応表にない | FR1〜FR5 とその子（FR1.1 など）、NFR1〜NFR10 の親。作業単位の code-generation の対応表は AC・BR・NFRn.m の細かい ID で書かれ、FR と NFR の親は user-stories の対応表（FR → US → AC）と NFR の子の ID を通じてだけたどれる。たどった先はすべて直接 OK（AC1.1.4 を除く） |
| BT-TR-2 | AC2.2.2・AC2.2.5 が U2 の code-generation の対応表にない | U2 の機能設計の対応表にはある。実装とテストは U1 で入っている（リージョンの表にないときの 5 回／秒、SDK の再試行を 1 回に上書き）。テストは今回すべて成功 |
| BT-TR-3 | FR6.1〜FR6.4 が作業単位の対応表の外 | FR6 は作業単位の前の下準備で、PR #20（`a79f2ad`）として main に入り済み。ストーリーにしないと決めている（user-stories-questions Q3） |
| BT-TR-4 | AC1.1.4 が code-generation の対応表にない | 実際の AWS での手元の確認（2026-10-15）で合格。U1-NFR2.1 と同じ確認 |

レビューを記録した後の作業単位の成果物は直さない決まり（project.md の Corrections）のため、code-generation の traceability.json には手を入れず、ここに記録する。

## ID ごとの対応

| ID | 対応 | 作業単位 | 対象 | 状態 |
|----|------|----------|------|------|
| FR1 | 間接 | u1-concurrent-skeleton | US1.1, US1.2, US1.3 の受け入れ条件 10 件（うち AC1.1.4 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR1.1 | 間接 | u1-concurrent-skeleton | US1.1 の受け入れ条件 4 件（うち AC1.1.4 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR1.2 | 間接 | u1-concurrent-skeleton | US1.2 の受け入れ条件 4 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR1.3 | 間接 | u1-concurrent-skeleton | US1.2 の受け入れ条件 4 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR1.4 | 間接 | u1-concurrent-skeleton | US1.1 の受け入れ条件 4 件（うち AC1.1.4 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR1.5 | 間接 | u1-concurrent-skeleton | US1.3 の受け入れ条件 2 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR2 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.1 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.2 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.3 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.4 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.5 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR2.6 | 間接 | u2-call-pacing | US2.2 の受け入れ条件 6 件（うち AC2.2.2, AC2.2.5 は下の AC の行を見る） | 要確認（親の ID は子の ID 経由） |
| FR3 | 間接 | u3-give-up-rule | US2.3 の受け入れ条件 4 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR3.1 | 直接 | u3-give-up-rule | `crates/local-sights-core/src/schedule.rs`（あり） | OK |
| FR3.2 | 直接 | u3-give-up-rule | `crates/local-sights-core/src/schedule.rs`（あり） | OK |
| FR3.3 | 直接 | u3-give-up-rule | `crates/local-sights-core/src/schedule.rs`（あり） | OK |
| FR3.4 | 直接 | u3-give-up-rule | `crates/local-sights-core/src/schedule.rs`（あり） | OK |
| FR4 | 間接 | u4-elapsed-time | US2.1 の受け入れ条件 7 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR4.1 | 直接 | u4-elapsed-time | `src/components/StatusLine.tsx`（あり） | OK |
| FR4.2 | 直接 | u4-elapsed-time | `src/hooks/useElapsed.ts`（あり） | OK |
| FR4.3 | 直接 | u4-elapsed-time | `crates/local-sights-core/src/session.rs`（あり） | OK |
| FR4.4 | 直接 | u4-elapsed-time | `crates/local-sights-core/src/session.rs`（あり） | OK |
| FR4.5 | 直接 | u4-elapsed-time | `src/elapsed.ts`（あり） | OK |
| FR4.6 | 直接 | u4-elapsed-time | `src/components/ElapsedTime.tsx`（あり） | OK |
| FR5 | 間接 | u5-cancel | US3.1, US4.1, US4.2 の受け入れ条件 15 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.1 | 間接 | u5-cancel | US3.1 の受け入れ条件 8 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.2 | 間接 | u5-cancel | US3.1 の受け入れ条件 8 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.3 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.4 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.5 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.6 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.7 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.8 | 間接 | u5-cancel | US4.2 の受け入れ条件 1 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.9 | 間接 | u5-cancel | US3.1 の受け入れ条件 8 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.10 | 間接 | u5-cancel | US3.1 の受け入れ条件 8 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR5.11 | 間接 | u5-cancel | US4.1 の受け入れ条件 6 件（すべて直接 OK） | 要確認（親の ID は子の ID 経由） |
| FR6 | 間接 | 作業単位の外（下準備） | PR #20（`a79f2ad`、main に入り済み） | 要確認（作業単位の traceability.json の対象外） |
| FR6.1 | 間接 | 作業単位の外（下準備） | `.github/workflows/ci.yml` の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` と `npm run test:coverage`（PR #20） | 要確認（作業単位の traceability.json の対象外） |
| FR6.2 | 間接 | 作業単位の外（下準備） | 今回の結果：ライブラリ 97.67%、画面 90.85% | 要確認（作業単位の traceability.json の対象外） |
| FR6.3 | 間接 | 作業単位の外（下準備） | `.github/workflows/ci.yml` の cargo-deny のコメント（PR #20） | 要確認（作業単位の traceability.json の対象外） |
| FR6.4 | 間接 | 作業単位の外（下準備） | delivery-planning の順番（PR #20 → U1 の薄い一本）。U1 の骨組みのチェックポイントで確認済み | 要確認（作業単位の traceability.json の対象外） |
| NFR1 | 間接 | — | 子の ID 9 件（NFR1.1〜NFR1.9）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR2 | 間接 | — | 子の ID 5 件（NFR2.1〜NFR2.5）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR3 | 間接 | — | 子の ID 7 件（NFR3.1〜NFR3.7）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR4 | 間接 | — | 子の ID 6 件（NFR4.1〜NFR4.6）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR5 | 間接 | — | 子の ID 5 件（NFR5.1〜NFR5.5）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR6 | 間接 | — | 子の ID 20 件（NFR6.1〜NFR6.20）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR7 | 間接 | — | 子の ID 4 件（NFR7.1〜NFR7.4）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR8 | 間接 | — | 子の ID 14 件（NFR8.1〜NFR8.15）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR9 | 間接 | — | 子の ID 4 件（NFR9.1〜NFR9.4）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| NFR10 | 間接 | — | 子の ID 1 件（NFR10.1〜NFR10.1）がすべて直接 OK | 要確認（親の ID は子の ID 経由） |
| AC1.1.1 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.1.2 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.1.3 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.1.4 | 間接 | u1-concurrent-skeleton | `crates/local-sights-core/examples/fetch_check.rs` と状態行の経過（U4） | 要確認（code-generation の traceability.json に行がない。手元の確認で合格） |
| AC1.2.1 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.2.2 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.2.3 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/src/log_view.rs`（あり） | OK |
| AC1.2.4 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.3.1 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC1.3.2 | 直接 | u1-concurrent-skeleton | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`（あり） | OK |
| AC2.1.1 | 直接 | u4-elapsed-time | `src/components/StatusLine.test.tsx`（あり） | OK |
| AC2.1.2 | 直接 | u4-elapsed-time | `src/components/ElapsedTime.test.tsx`（あり） | OK |
| AC2.1.3 | 直接 | u4-elapsed-time | `crates/local-sights-core/src/session.rs`（あり） | OK |
| AC2.1.4 | 直接 | u4-elapsed-time | `crates/local-sights-core/src/session.rs`（あり） | OK |
| AC2.1.5 | 直接 | u4-elapsed-time | `src/elapsed.test.ts`（あり） | OK |
| AC2.1.6 | 直接 | u4-elapsed-time | `src/components/StatusLine.test.tsx`（あり） | OK |
| AC2.1.7 | 直接 | u4-elapsed-time | `src/components/ElapsedTime.tsx`（あり） | OK |
| AC2.2.1 | 直接 | u2-call-pacing | `crates/local-sights-core/tests/u2_call_pacing.rs`（あり） | OK |
| AC2.2.2 | 間接 | u2-call-pacing | `crates/local-sights-core/src/pacing.rs`（あり）：`pacing::tests::an_unknown_or_missing_region_gets_five`（U1 の BR1.2 で実装） | 要確認（code-generation の traceability.json に行がない） |
| AC2.2.3 | 直接 | u2-call-pacing | `crates/local-sights-core/tests/u2_call_pacing.rs`（あり） | OK |
| AC2.2.4 | 直接 | u2-call-pacing | `crates/local-sights-core/tests/u2_call_pacing.rs`（あり） | OK |
| AC2.2.5 | 間接 | u2-call-pacing | `crates/local-sights-core/src/gateway/aws.rs`（あり）：`gateway::aws::tests::the_get_log_events_override_allows_one_attempt_only`（U1 の NFR3.4 で実装） | 要確認（code-generation の traceability.json に行がない） |
| AC2.2.6 | 直接 | u2-call-pacing | `crates/local-sights-core/src/coordinator.rs`（あり） | OK |
| AC2.3.1 | 直接 | u3-give-up-rule | `crates/local-sights-core/tests/u3_give_up_rule.rs`（あり） | OK |
| AC2.3.2 | 直接 | u3-give-up-rule | `crates/local-sights-core/tests/u3_give_up_rule.rs`（あり） | OK |
| AC2.3.3 | 直接 | u3-give-up-rule | `crates/local-sights-core/tests/u3_give_up_rule.rs`（あり） | OK |
| AC2.3.4 | 直接 | u3-give-up-rule | `crates/local-sights-core/tests/u3_give_up_rule.rs`（あり） | OK |
| AC3.1.1 | 直接 | u5-cancel | `src/components/ConditionArea.tsx`（あり） | OK |
| AC3.1.2 | 直接 | u5-cancel | `src-tauri/src/lib.rs`（あり） | OK |
| AC3.1.3 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC3.1.4 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC3.1.5 | 直接 | u5-cancel | `src/components/ConditionArea.test.tsx`（あり） | OK |
| AC3.1.6 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC3.1.7 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC3.1.8 | 直接 | u5-cancel | `src/components/ConditionArea.test.tsx`（あり） | OK |
| AC4.1.1 | 直接 | u5-cancel | `crates/local-sights-core/src/session.rs`（あり） | OK |
| AC4.1.2 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC4.1.3 | 直接 | u5-cancel | `src/components/StatusLine.tsx`（あり） | OK |
| AC4.1.4 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
| AC4.1.5 | 直接 | u5-cancel | `crates/local-sights-core/src/session.rs`（あり） | OK |
| AC4.1.6 | 直接 | u5-cancel | `src/components/CancelNotice.tsx`（あり） | OK |
| AC4.2.1 | 直接 | u5-cancel | `crates/local-sights-core/tests/u5_cancel.rs`（あり） | OK |
