# Cross-Unit Traceability — 要件 ID の対応の最終確認（全単位）

対象：`inception/requirements-analysis/requirements.md` の FR1.1〜FR8.3（54 件）と NFR1〜NFR16（16 件）。User Stories のステージはこのスコープでは実行していないため、受け入れ条件（AC）の ID はない。確認した対応表：U1〜U7 の `construction/<unit>/code-generation/traceability.json`（7 ファイル。ステージ単位の `construction/code-generation/traceability.json` はない）。

確認方法：各 ID について、どれか 1 つの単位の対応表に `status: OK` の行があり、その行の対応先のファイルがリポジトリに存在するかを、スクリプトで突き合わせた（2026-10-10）。

## 判定

**FAIL（対応なしの ID が 4 件）** — FR7.8、NFR7、NFR9、NFR14。いずれも機能のコードが欠けているのではなく、対応表への記入がない（下の「対応のない ID」）。承認の場で扱いを決める。

## 要件ごとの対応

| ID | 対応 | 担当の単位 | 対応先のファイル（代表） |
|----|------|------------|--------------------------|
| FR1.1 | OK | U2 | `crates/local-sights-core/src/catalog/mod.rs` |
| FR1.2 | OK | U1 | `crates/local-sights-core/src/gateway/aws.rs` |
| FR1.3 | OK | U2 | `crates/local-sights-core/src/catalog/regions.rs` |
| FR1.4 | OK | U2 | `crates/local-sights-core/src/session.rs` |
| FR1.5 | OK | U7 | `src/i18n/messages.ts` |
| FR2.1 | OK | U2 | `crates/local-sights-core/tests/u2_log_group_listing.rs` |
| FR2.2 | OK | U2 | `crates/local-sights-core/src/log_groups/mod.rs` |
| FR2.3 | OK | U2 | `src/components/FetchForm.tsx` |
| FR3.1 | OK | U1・U4 | `crates/local-sights-core/src/time_range.rs`、`date_input.rs` |
| FR3.2 | OK | U4 | `src/components/TimeZoneToggle.tsx` |
| FR3.3 | OK | U4 | `crates/local-sights-core/src/session.rs` |
| FR3.4 | OK | U4 | `crates/local-sights-core/src/session.rs` |
| FR3.5 | OK | U1・U4 | `crates/local-sights-core/src/time_range.rs`、`request.rs` |
| FR3.6 | OK | U4 | `crates/local-sights-core/src/time_range.rs` |
| FR4.1 | OK | U3 | `crates/local-sights-core/src/streams/planner.rs` |
| FR4.2 | OK | U3 | `crates/local-sights-core/src/streams/mod.rs` |
| FR4.3 | OK | U1 | `crates/local-sights-core/src/fetcher.rs` |
| FR4.4 | OK | U1 | `crates/local-sights-core/src/paging.rs` |
| FR4.5 | OK | U3 | `crates/local-sights-core/src/retry.rs` |
| FR4.6 | OK | U1 | `crates/local-sights-core/src/timeline.rs` |
| FR4.7 | OK | U3 | `crates/local-sights-core/src/timeline.rs` |
| FR4.8 | OK | U3 | `src/components/StatusLine.tsx` |
| FR4.9 | OK | U7 | `src-tauri/src/lib.rs` |
| FR4.10 | OK | U3 | `crates/local-sights-core/src/coordinator.rs` |
| FR4.11 | OK | U3 | `crates/local-sights-core/src/coordinator.rs` |
| FR5.1 | OK | U1 | `src/components/LogTable.tsx` |
| FR5.2 | OK | U7 | `src/components/LogTable.tsx` |
| FR5.3 | OK | U7 | `src/components/ExpandedMessage.tsx` |
| FR5.4 | OK | U7 | `src/components/ExpandedMessage.tsx` |
| FR6.1 | OK | U5 | `crates/local-sights-core/src/log_view.rs` |
| FR6.2 | OK | U5 | `src-tauri/src/lib.rs` |
| FR6.3 | OK | U5 | `src/components/StatusLine.tsx` |
| FR6.4 | OK | U5 | `crates/local-sights-core/src/log_view.rs` |
| FR6.5 | OK | U5 | `crates/local-sights-core/src/filter.rs`（Functional Design で「区別しない」に決めた） |
| FR7.1 | OK | U6 | `crates/local-sights-core/src/cache/settings.rs` |
| FR7.2 | OK | U6 | `src/components/SettingsDialog.tsx` |
| FR7.3 | OK | U6 | `src/components/SettingsDialog.tsx` |
| FR7.4 | OK | U6 | `crates/local-sights-core/src/coordinator.rs` |
| FR7.5 | OK | U6 | `crates/local-sights-core/src/cache/plan.rs` |
| FR7.6 | OK | U6 | `crates/local-sights-core/src/cache/mod.rs` |
| FR7.7 | OK | U6 | `crates/local-sights-core/src/coordinator.rs` |
| FR7.8 | **なし** | U6（`N/A`） | — （下の「対応のない ID」） |
| FR7.9 | OK | U6 | `crates/local-sights-core/src/cache/plan.rs` |
| FR8.1 | OK | U7 | `src/errorText.ts` |
| FR8.2 | OK | U7 | `src/components/ErrorMessage.tsx` |
| FR8.3 | OK | U1・U7 | `crates/local-sights-core/src/failure.rs`、`src/errorText.ts` |
| NFR1 | OK | U5 | `crates/local-sights-core/src/log_view.rs` |
| NFR2 | OK | U3・U5・U6・U7 | `timeline.rs`、`log_view.rs`、`coordinator.rs`、`src/rowLayout.ts` |
| NFR3 | OK | U3 | `src-tauri/src/lib.rs` |
| NFR4 | OK | U3 | `crates/local-sights-core/src/retry.rs` |
| NFR5 | OK | U6・U7 | `crates/local-sights-core/src/cache/mod.rs`、`src-tauri/src/lib.rs` |
| NFR6 | OK | U3 | `crates/local-sights-core/src/gateway/mod.rs` |
| NFR7 | **なし** | — | — （下の「対応のない ID」） |
| NFR8 | OK | U6 | `crates/local-sights-core/src/session.rs` |
| NFR9 | **なし** | — | — （下の「対応のない ID」） |
| NFR10 | OK | U7 | `src/styles.css` |
| NFR11 | OK | U7 | `src-tauri/tauri.conf.json` |
| NFR12 | OK | U3〜U7 | `src/i18n/messages.ts` |
| NFR13 | OK | U3〜U7 | `src/components/LogTable.tsx` ほか |
| NFR14 | **なし** | — | — （下の「対応のない ID」） |
| NFR15 | OK | U6 | `crates/local-sights-core/src/coordinator.rs` |
| NFR16 | OK | U7 | `src-tauri/src/lib.rs` |

対応先のファイルは、`OK` の行についてはすべて存在した（欠けたファイルなし）。

## 対応のない ID

| ID | 内容 | 状況 | このステージで確かめたこと |
|----|------|------|----------------------------|
| FR7.8 | 期限が厳しいときは FR7（キャッシュ）全体を MVP から外してよい | U6 の対応表に `N/A`（機能のルールではなく選択肢。U6 はライブラリの `cache` モジュール・`run_fetch_with_cache`・設定ダイアログにまとまっており、U6 ごと外せる構成で満たす、と記載） | 選択肢であり、コードの対応先は要らない。対応表の書式では `N/A` が「対応なし」に数えられるため、ここに挙げる |
| NFR7 | テレメトリやクラッシュレポートを送らない | どの単位の対応表にも ID がない | 画面側に外部通信のコードなし、CSP の `connect-src` は IPC だけ、通信系の Tauri プラグインなし、AWS SDK 以外の HTTP クライアントなし（`security-test-instructions.md`、`test-results.md`） |
| NFR9 | macOS で動く | どの単位の対応表にも ID がない | この環境では確かめられない。手元の macOS で `cargo build -p local-sights` と起動の確認（`build-instructions.md`） |
| NFR14 | テストしやすい構成（ライブラリと GUI の分離、AWS は trait の裏、公開関数にテスト、テストと CI は AWS に接続しない） | どの単位の対応表にも ID がない | `crates/local-sights-core`（GUI 非依存）と `src-tauri`・`src`（GUI）に分かれている。`aws_sdk_cloudwatchlogs` を使うのは `gateway/aws.rs` だけで、trait `CloudWatchLogsGateway` の裏にある。結合テストは偽物の gateway だけを使う。ライブラリの単体テスト 305 件・結合テスト 41 件 |

扱いの案：NFR7・NFR14 はこのステージの確認で満たしていることを確かめた。NFR9 は手元で確かめる。対応表（`traceability.json`）への追記は、コード生成のステージを閉じたあとに気づいた訂正のため、この記録に残し、次に対応表を触る機会（CI Pipeline のステージ、または次の作業単位）で行う（project.md Corrections：レビュー後に気づいた修正は次の作業単位に回す）。
