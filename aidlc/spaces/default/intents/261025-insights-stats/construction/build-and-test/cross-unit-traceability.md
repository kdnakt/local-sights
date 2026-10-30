# Cross-Unit Traceability — 261025-insights-stats

## 判定

**Fail（見つかったことあり）**

- FR1.1〜FR7.3 の 37 件はすべて、どれかの作業単位の `traceability.json` に `OK` で載っており、指しているファイルもある。
- NFR は 12 件のうち 7 件が載っている。残りの 5 件（NFR5・NFR6・NFR8・NFR11・NFR12）は、どの作業単位の `traceability.json` にも載っていない。
- この 5 件は特定の部品ではなく作業全体にかかる決まりで、作業単位の追跡表に入れていなかった。Build and Test で確かめ、その証拠を `build-and-test-summary.md` の Target Verification Matrix に書く。ただし、追跡表に載っていないこと自体は見つかったこととして承認の場で示す。

ストーリーの段（User Stories）はこの計画で行っていないため、AC はない。

## 要件ごとの追跡

| ID | 作業単位 | 指すファイル | 状態 |
|---|---|---|---|
| FR1.1 | U1 | `crates/local-sights-core/src/query/parser.rs` | OK |
| FR1.2 | U1・U2 | `crates/local-sights-core/src/query/stats.rs` | OK |
| FR1.3 | U1 | `crates/local-sights-core/src/query/stats.rs` | OK |
| FR1.4 | U1・U2 | `query/parser.rs`、`query/evaluator.rs` | OK |
| FR1.5 | U1 | `query/stats.rs` | OK |
| FR1.6 | U1・U2 | `query/stats.rs` | OK |
| FR1.7 | U2 | `query/evaluator.rs` | OK |
| FR1.8 | U1 | `query/evaluator.rs` | OK |
| FR2.1 | U1 | `query/parser.rs` | OK |
| FR2.2 | U1 | `query/stats.rs` | OK |
| FR2.3 | U1 | `crates/local-sights-core/src/insights_session.rs` | OK |
| FR3.1 | U1・U2 | `query/plan.rs`、`query/parser.rs` | OK |
| FR3.2 | U1 | `src/components/LogAnalyticsPage.tsx` | OK |
| FR3.3 | U1 | `insights_session.rs` | OK |
| FR3.4 | U1 | `crates/local-sights-core/src/export/exporter.rs` | OK |
| FR3.5 | U1 | `src/components/QueryResultTable.tsx` | OK |
| FR3.6 | U1 | `src/components/QueryResultTable.tsx` | OK |
| FR4.1 | U3 | `query/runner.rs` | OK |
| FR4.2 | U3 | `query/store.rs` | OK |
| FR4.3 | U3 | `insights_session.rs` | OK |
| FR4.4 | U3 | `src/components/QueryResultTable.tsx` | OK |
| FR4.5 | U3 | `src/components/LogAnalyticsPage.tsx` | OK |
| FR4.6 | U3 | `src/components/LogAnalyticsPage.tsx` | OK |
| FR4.7 | U3 | `insights_session.rs` | OK |
| FR4.8 | U3 | `query/store.rs` | OK |
| FR4.9 | U3 | `query/runner.rs` | OK |
| FR4.10 | U3 | `src/components/LogAnalyticsPage.tsx` | OK |
| FR5.1 | U3 | `query/runner.rs` | OK |
| FR5.2 | U3 | `src/components/ExportNoticeAlert.tsx` | OK |
| FR5.3 | U3 | `query/store.rs` | OK |
| FR5.4 | U3 | `src/components/LogAnalyticsPage.tsx` | OK |
| FR6.1 | U2 | `query/catalog.rs` | OK |
| FR6.2 | U2 | `query/catalog.rs` | OK |
| FR6.3 | U1・U2 | `query/parser.rs` | OK |
| FR7.1 | U4 | `crates/local-sights-core/examples/compat/compare.rs` | OK |
| FR7.2 | U4 | `crates/local-sights-core/examples/compat_queries.txt` | OK |
| FR7.3 | U4 | `crates/local-sights-core/examples/compat/report.rs` | OK |
| NFR1 | U3 | `crates/local-sights-core/examples/fetch_check.rs` | OK（合否の数値は手元の実測待ち） |
| NFR2 | U2・U3 | `query/evaluator.rs`、`query/store.rs` | OK |
| NFR3 | U4 | `README.md` | OK |
| NFR4 | U4 | `examples/compat_queries.txt` | OK |
| NFR5 | — | — | **追跡表になし**（Build and Test の T-NFR5 で確かめる） |
| NFR6 | — | — | **追跡表になし**（Build and Test の T-NFR6 で確かめる） |
| NFR7 | U2 | `query/catalog.rs` | OK |
| NFR8 | — | — | **追跡表になし**（Build and Test の T-NFR8-* で確かめる） |
| NFR9 | U3 | `src/i18n/messages.ts` | OK |
| NFR10 | U3 | `src/components/LogAnalyticsPage.tsx` | OK |
| NFR11 | — | — | **追跡表になし**（Build and Test の T-NFR11 で確かめる） |
| NFR12 | — | — | **追跡表になし**（Build and Test の T-NFR12 で確かめる） |

## 追跡表になかったもの

| ID | 中身 | 確かめる場所 |
|---|---|---|
| NFR5 | 呼ぶ AWS API は読み取りの 3 つだけ | security-test-instructions.md、T-NFR5 |
| NFR6 | macOS と Windows で同じように動く | CI の各 OS のランナーと手元の確かめ、T-NFR6 |
| NFR8 | カバレッジ 80% 以上、各 OS でテストが通る、既存のテストが通ったまま | test-results.md、T-NFR8-CORE・T-NFR8-UI・T-NFR8-OS・T-C5 |
| NFR11 | Cloudscape の標準の部品だけ | 差分で独自の部品・デザインを足していないこと、T-NFR11 |
| NFR12 | 外部に何も送らない | security-test-instructions.md、T-NFR12 |
