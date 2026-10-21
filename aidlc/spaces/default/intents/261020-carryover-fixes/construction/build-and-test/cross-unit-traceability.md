# 要件の網羅の確かめ（261020-carryover-fixes）

## 判定

**合格（PASS）**。`inception/requirements-analysis/requirements.md` の FR と NFR、53 項目のうち 52 項目は、`construction/code-generation/traceability.json` で `OK` になっていて、指す先のファイルもある。

残りの 1 項目（FR4.1.3）は、要件そのものが「今回の対象外とする」と決めた項目のため `N/A` とした（A8）。作業単位はなく（bugfix のスコープ）、記録は段の単位の `construction/code-generation/traceability.json` だけ。User Stories の段は通っていないため、AC はない。

## 項目ごとの網羅

| ID | 状態 | 持ち主 | 指す先 |
|---|---|---|---|
| FR1.1 | OK | code-generation（段の単位） | `crates/local-sights-core/tests/u5_cancel.rs` |
| FR1.2 | OK | code-generation | `crates/local-sights-core/tests/u5_cancel.rs` |
| FR1.3 | OK | code-generation | `crates/local-sights-core/tests/u5_cancel.rs` |
| FR2.1.1 | OK | code-generation | `crates/local-sights-core/src/cache/plan.rs` |
| FR2.1.2 | OK | code-generation | `crates/local-sights-core/src/cache/plan.rs` |
| FR2.1.3 | OK | code-generation | `crates/local-sights-core/src/cache/mod.rs` |
| FR2.1.4 | OK | code-generation | `crates/local-sights-core/src/cache/mod.rs` |
| FR2.1.5 | OK | code-generation | `crates/local-sights-core/src/cache/mod.rs` |
| FR2.2.1 | OK | code-generation | `crates/local-sights-core/src/query/store.rs` |
| FR2.2.2 | OK | code-generation | `crates/local-sights-core/src/query/runner.rs` |
| FR2.2.3 | OK | code-generation | `crates/local-sights-core/src/query/runner.rs` |
| FR2.2.4 | OK | code-generation | `crates/local-sights-core/src/query/store.rs` |
| FR2.2.5 | OK | code-generation | `crates/local-sights-core/src/query/store.rs` |
| FR2.3.1 | OK | code-generation | `crates/local-sights-core/src/insights_session.rs` |
| FR2.3.2 | OK | code-generation | `src-tauri/src/lib.rs` |
| FR2.3.3 | OK | code-generation | `src/components/QueryResultTable.tsx` |
| FR2.3.4 | OK | code-generation | `crates/local-sights-core/src/insights_session.rs` |
| FR2.3.5 | OK | code-generation | `crates/local-sights-core/src/query/store.rs` |
| FR3.1 | OK | code-generation | `crates/local-sights-core/src/query/plan.rs` |
| FR3.2 | OK | code-generation | `crates/local-sights-core/examples/compat_check.rs` |
| FR3.3 | OK | code-generation | `src/elapsed.ts` |
| FR3.4 | OK | code-generation | `docs/releasing.md` |
| FR4.1.1 | OK | code-generation | `src/i18n/messages.ts` |
| FR4.1.2 | OK | code-generation | `src/i18n/messages.test.ts` |
| FR4.1.3 | N/A | — | 要件が対象外とした項目（A8：ストリーム以外の数の英語の文は変えない）。変えていないことは、イベントの数の文の期待値が変わっていない既存のテスト（`src/components/StatusLine.test.tsx` ほか）で保たれる |
| FR4.1.4 | OK | code-generation | `src/i18n/messages.test.ts` |
| FR4.1.5 | OK | code-generation | `src/i18n/messages.ts` |
| FR4.2.1 | OK | code-generation | `src/components/CancelNotice.tsx` |
| FR4.2.2 | OK | code-generation | `src/components/CancelNotice.tsx` |
| FR4.3.1 | OK | code-generation | `src-tauri/src/lib.rs` |
| FR4.3.2 | OK | code-generation | `src-tauri/src/lib.rs` |
| FR4.4.1 | OK | code-generation | `src/components/LogAnalyticsPage.tsx` |
| FR4.4.2 | OK | code-generation | `src/components/LogAnalyticsPage.tsx` |
| FR4.4.3 | OK | code-generation | `src/components/LogAnalyticsPage.tsx` |
| FR4.4.4 | OK | code-generation | `src/components/LogAnalyticsPage.test.tsx` |
| FR5.1 | OK | code-generation | `src/i18n/regions.ts` |
| FR5.1.1 | OK | code-generation | `src/i18n/regions.ts` |
| FR5.2 | OK | code-generation | `src/components/TopBar.tsx` |
| FR5.3 | OK | code-generation | `src/components/TopBar.tsx` |
| FR5.4 | OK | code-generation | `src/i18n/regions.ts` |
| FR5.5 | OK | code-generation | `src/i18n/regions.ts` |
| FR5.6 | OK | code-generation | `src/components/TopBar.tsx` |
| FR5.7 | OK | code-generation | `src/components/TopBar.tsx` |
| FR5.8 | OK | code-generation | `src/components/TopBar.tsx` |
| FR5.9 | OK | code-generation | `src/components/TopBar.test.tsx` |
| FR5.10 | OK | code-generation | `src/i18n/regions.test.ts` |
| NFR1 | OK | code-generation | `aidlc/spaces/default/intents/261020-carryover-fixes/construction/code-generation/code-summary.md`（実行の結果は `build-and-test/test-results.md`） |
| NFR2 | OK | code-generation | `aidlc/spaces/default/intents/261020-carryover-fixes/construction/code-generation/code-summary.md` |
| NFR3 | OK | code-generation | `vitest.config.ts`（測った値は `build-and-test/test-results.md`） |
| NFR4 | OK | code-generation | `crates/local-sights-core/src/cache/mod.rs`（速さのテストの結果は `build-and-test/test-results.md`） |
| NFR5 | OK | code-generation | `src/i18n/regions.ts` |
| NFR6 | OK | code-generation | `src/i18n/messages.test.ts` |
| NFR7 | OK | code-generation | `aidlc/spaces/default/intents/261020-carryover-fixes/construction/code-generation/code-summary.md`（検査の結果は `build-and-test/test-results.md`） |

## 網羅されていない項目

ない。FR4.1.3 は対象外の決まり（A8）で、承認の場で確かめてもらうために上の表に挙げた。
