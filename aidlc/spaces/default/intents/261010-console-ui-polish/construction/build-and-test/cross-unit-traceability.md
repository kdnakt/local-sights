# Cross-Unit Traceability — 画面を AWS マネジメントコンソールに寄せる

ゼロ Unit の作業のため、追跡はステージ単位の `construction/code-generation/traceability.json` だけを読む。user-stories ステージは実行していないので、受け入れ基準（AC）の ID はない。

**判定：不合格（未カバー 2 件：NFR2、NFR7）**

| ID | 状態 | 対象ファイル | ファイルの有無 |
|---|---|---|---|
| FR1.1 | OK | src/components/ConditionArea.tsx | あり |
| FR1.2 | OK | src/components/ConditionArea.test.tsx | あり |
| FR1.3 | OK | src/dateTime.ts | あり |
| FR1.4 | OK | src/dateTime.ts | あり |
| FR1.5 | OK | src/App.test.tsx | あり |
| FR1.6 | OK | crates/local-sights-core/src/time_range.rs | あり |
| FR1.7 | OK | crates/local-sights-core/src/time_range.rs | あり |
| FR1.8 | OK | src/components/ConditionArea.test.tsx | あり |
| FR2.1 | OK | crates/local-sights-core/src/time_range.rs | あり |
| FR2.2 | OK | crates/local-sights-core/src/time_range.rs | あり |
| FR2.3 | OK | src/i18n/messages.test.ts | あり |
| FR2.4 | OK | crates/local-sights-core/src/time_range.rs | あり |
| FR3.1 | OK | src/components/ConditionArea.test.tsx | あり |
| FR3.2 | OK | src/components/FetchErrorBanner.test.tsx | あり |
| FR3.3 | OK | src/components/FailureList.tsx | あり |
| FR3.4 | OK | src/errorText.ts | あり |
| FR3.5 | OK | src/components/FetchErrorBanner.test.tsx | あり |
| FR4.1 | OK | src/components/TopBar.tsx | あり |
| FR4.2 | OK | src/App.tsx | あり |
| FR4.3 | OK | src/components/LogTable.tsx | あり |
| FR4.4 | OK | src/App.test.tsx | あり |
| FR4.5 | OK | src/hooks/useColorScheme.test.ts | あり |
| FR4.6 | OK | src/main.tsx | あり |
| NFR1 | OK | package.json | あり |
| NFR2 | **Deferred（未カバー）** | src/dev/perfRows.ts | あり |
| NFR3 | OK | src/components/ConditionArea.test.tsx | あり |
| NFR4 | OK | src/i18n/messages.test.ts | あり |
| NFR5 | OK | package-lock.json | あり |
| NFR6 | OK | src/components/LogTable.test.tsx | あり |
| NFR7 | **Deferred（未カバー）** | src/styles.css | あり |

ゲートの修正依頼とレビュー指摘の追跡（`GATE-1`〜`GATE-16`、`REV-R01`、`REV-R03`）は要求 ID ではないが、すべて `OK` で対象ファイルも存在する。

## 未カバーの要素

- **NFR2（性能）**：100 万行の測定が未実施。測定のハーネスはあるが、合否の根拠がない。手順は `performance-test-instructions.md`。
- **NFR7（最小ウィンドウ）**：1024×640 での見た目の確認が未実施。jsdom では描画の寸法を測れない。

どちらも Tauri の実機での手動確認が必要で、承認のゲートで扱う。
