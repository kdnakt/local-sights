# Cross-Unit Traceability — Logs Insights 互換のクエリ（IB-11）

`inception/requirements-analysis/requirements.md` のすべての FR・NFR・SM を、4 つの作業単位の `construction/*/code-generation/traceability.json` と突き合わせた。User Stories の段はこの計画にないため、AC は対象外。Express の段の traceability はない。

## 判定：不合格（覆われていない ID が 3 つ）

FR 50 個と SM 2 個はすべて、少なくとも 1 つの作業単位で `OK`、目標のファイルもある。NFR は 12 個のうち 9 個が `OK`。NFR6・NFR8・NFR12 は、どの作業単位でも `N/A`（作業単位をまたぐ決まりとして、どの単位も自分のものにしていない）。この 3 つは Build and Test で確かめた（下の「覆われていない ID」）。承認のときに示す。

## ID ごとの覆い

| ID | 状態 | 作業単位 | 目標のファイル |
|---|---|---|---|
| FR1.1 | OK | U1 | `src/components/AppNavigation.tsx` |
| FR1.2 | OK | U1 | `crates/local-sights-core/src/insights_session.rs` |
| FR1.3 | OK | U1・U3 | `src/components/LogGroupMultiselect.tsx` |
| FR1.4 | OK | U1 | `crates/local-sights-core/src/insights_session.rs` |
| FR1.5 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR1.6 | OK | U1・U3 | `crates/local-sights-core/src/insights_session.rs` |
| FR1.7 | OK | U1 | `src/components/LogAnalyticsPage.tsx` |
| FR1.8 | OK | U1 | `src/components/AppNavigation.tsx` |
| FR2.1 | OK | U1・U3 | `src/App.tsx`、`crates/local-sights-core/src/query/store.rs` |
| FR2.2 | OK | U3 | `crates/local-sights-core/src/query/runner.rs` |
| FR2.3 | OK | U3 | `crates/local-sights-core/src/query/runner.rs` |
| FR2.4 | OK | U1・U3 | `crates/local-sights-core/src/query/runner.rs` |
| FR2.5 | OK | U1・U3 | `crates/local-sights-core/src/query/runner.rs` |
| FR2.6 | OK | U1・U2 | `crates/local-sights-core/src/query/runner.rs`、`insights_session.rs` |
| FR2.7 | OK | U3 | `crates/local-sights-core/src/query/store.rs` |
| FR2.8 | OK | U3 | `crates/local-sights-core/src/query/store.rs` |
| FR3.1 | OK | U1・U2 | `crates/local-sights-core/src/query/parser.rs` |
| FR3.2 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR3.3 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR3.4 | OK | U2 | `crates/local-sights-core/src/query/natural.rs` |
| FR3.5 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR3.6 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR3.7 | OK | U2 | `src/components/LogAnalyticsPage.tsx` |
| FR3.8 | OK | U2 | `crates/local-sights-core/src/query/parser.rs` |
| FR4.1 | OK | U1・U2 | `crates/local-sights-core/src/query/parser.rs`、`fields.rs` |
| FR4.2 | OK | U2 | `crates/local-sights-core/src/query/parser.rs` |
| FR4.3 | OK | U2 | `crates/local-sights-core/src/query/fields.rs` |
| FR4.4 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR4.5 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR5.1 | OK | U1 | `crates/local-sights-core/src/query/evaluator.rs` |
| FR5.2 | OK | U1・U2 | `crates/local-sights-core/src/query/parser.rs` |
| FR5.3 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR5.4 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR5.5 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR5.6 | OK | U1 | `crates/local-sights-core/src/query/parser.rs` |
| FR6.1 | OK | U2 | `src/components/QueryEditor.tsx` |
| FR6.2 | OK | U2 | `crates/local-sights-core/src/query/parser.rs` |
| FR6.3 | OK | U1 | `src/components/QueryEditor.tsx` |
| FR6.4 | OK | U2 | `src/i18n/messages.ts` |
| FR7.1 | OK | U2 | `src/components/QueryResultTable.tsx` |
| FR7.2 | OK | U1 | `src/components/LogAnalyticsPage.tsx` |
| FR7.3 | OK | U2 | `src/components/LogAnalyticsPage.tsx` |
| FR7.4 | OK | U2 | `src/components/QueryResultTable.tsx` |
| FR7.5 | OK | U2・U3 | `src/components/QueryResultTable.tsx`、`query/evaluator.rs` |
| FR7.6 | OK | U1・U3 | `crates/local-sights-core/src/insights_session.rs` |
| FR8.1 | OK | U2 | `README.md` |
| FR8.2 | OK | U2 | `README.md` |
| FR9.1 | OK | U2・U3 | `src/analytics.ts`、`crates/local-sights-core/src/query/store.rs` |
| FR9.2 | OK | U2 | `src/aceLoader.ts` |
| FR9.3 | OK | U2 | `src/components/LogAnalyticsPage.tsx` |
| SM1 | OK | U4 | `crates/local-sights-core/examples/compat/compare.rs` |
| SM2 | OK | U4 | `crates/local-sights-core/examples/compat/compare.rs` |
| NFR1 | OK | U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| NFR2 | OK | U1・U2 | `crates/local-sights-core/src/query/evaluator.rs` |
| NFR3 | OK | U4 | `crates/local-sights-core/examples/compat_queries.txt` |
| NFR4 | OK | U4 | `crates/local-sights-core/examples/compat/settings.rs` |
| NFR5 | OK | U2・U3 | `crates/local-sights-core/src/query/runner.rs` |
| NFR6 | **覆われていない** | なし（U1〜U4 とも N/A） | — |
| NFR7 | OK | U2・U3 | `crates/local-sights-core/src/query/evaluator.rs`、`cache/plan.rs` |
| NFR8 | **覆われていない** | なし（U1〜U4 とも N/A） | — |
| NFR9 | OK | U2・U3 | `src/i18n/messages.ts` |
| NFR10 | OK | U2・U3 | `src/components/QueryEditor.tsx`、`LogGroupMultiselect.tsx` |
| NFR11 | OK | U2・U3 | `src/components/QueryResultTable.tsx`、`LogGroupMultiselect.tsx` |
| NFR12 | **覆われていない** | なし（U1〜U4 とも N/A） | — |

## 覆われていない ID

| ID | 要件 | Build and Test での確かめ | 残り |
|---|---|---|---|
| NFR6 | macOS と Windows で同じように動く。クエリの層は OS に依存しない純粋な処理 | クエリの層（`query/`）に `cfg` で分けたコードはない。Linux でテストが通った | macOS・Windows の CI と手元の確認が未実施（`test-results.md` の T-NFR8-OS、T-WS-MAC、T-WS-WIN） |
| NFR8 | 行カバレッジ 80% 以上、Rust のテストは 3 つの OS で通る | ライブラリ 96.52%、画面 89.88%（Linux） | macOS・Windows の CI が未実施（T-NFR8-OS） |
| NFR12 | 外部に何も送らない | 足した行に外部への送信がないことを差分で確かめた（T-NFR12：Met） | なし。作業単位の traceability には載っていない |
