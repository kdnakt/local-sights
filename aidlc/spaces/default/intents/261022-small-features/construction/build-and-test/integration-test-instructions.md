# Integration Test Instructions — アプリ内の言語切替・書き出し・JSON の整形（261022-small-features）

Test Strategy は `Standard`。単体テストは各作業単位の `unit-test-instructions.md` のとおり。ここでは作業単位をまたぐ境目と、部品の間の境目のテストを並べる。自動テストは実際の AWS に接続しない（project.md Forbidden）。この作業は AWS を呼ばない。

## 1. 作業単位をまたぐ境目と、それを確かめるテスト

| 境目 | 作業単位 | テスト | 場所 |
|---|---|---|---|
| 設定ファイル → AppSession → SessionView → App の言語の決定 | U1 | `cache::settings` と `session` の `language` のテスト、`App.test.tsx` の `switches the texts to Japanese…`・`starts in the saved language…` | `crates/local-sights-core/src/cache/settings.rs`、`session.rs`、`src/App.test.tsx` |
| 設定ダイアログ → `save_settings` コマンド → AppSession | U1 | `SettingsDialog.test.tsx`（`onSave(enabled, language)`）、`commandRegistration.test.ts`（3 か所の一致） | `src/components/SettingsDialog.test.tsx`、`src/commandRegistration.test.ts` |
| U1 の SessionView の形の上に U2 が足したもの（`export`・`exportNotice`・`canExportLogs`・`insights.canExport`） | U1 → U2 | `session::tests::*export*`、`App.test.tsx` の `export` の describe | `crates/local-sights-core/src/session.rs`、`src/App.test.tsx` |
| LogView の表示中の行（絞り込み後）→ 写し → 3 列の文字 | U2 | `the_log_screen_export_snapshot_has_three_columns_display_times_and_full_messages` | `crates/local-sights-core/src/session.rs` |
| QueryResult の行 → 写し → 表の列の文字（`@message` 全文、欠けは空） | U2 | `the_query_export_snapshot_has_the_result_columns_full_messages_and_empty_cells` | `crates/local-sights-core/src/insights_session.rs` |
| 写し → ExportFormatter → ファイル（一時ファイル → rename、BOM） | U2 | `render_prepends_a_bom_only_for_csv_files`、`export_boundary` の 5 本（一時フォルダ） | `crates/local-sights-core/src/export/exporter.rs`、`src-tauri/src/export_boundary.rs` |
| `start_export` / `dismiss_export_notice` コマンド ↔ 画面 | U2 | `commandRegistration.test.ts`（3 か所の一致）、`App.test.tsx`（`startExport("logManagement","csv","file")`、知らせの表示と閉じる、言語切替後の知らせ）、`LogAnalyticsPage.test.tsx`・`ConditionArea.test.tsx`（メニューの配置と可否） | `src/` |
| JsonPrettyPrinter ↔ ExpandedMessage ↔ LogTable（高さの計測、キー操作の伝播） | U2 | `ExpandedMessage.test.tsx`（6 本）、`LogTable.test.tsx`（計測の対象、開いた行の文字） | `src/components/` |
| 言語切替 × 書き出しの知らせ（鍵と引数で持つ） | U1 × U2 | `App.test.tsx` の「言語を切り替えると知らせの文が切り替わる」 | `src/App.test.tsx` |
| 変えないもの（取得・絞り込み・キャッシュ・SSO・中断・クエリ、NFR1） | 既存 | 結合テスト `tests/u1_*`〜`tests/u6_*`、既存の単体テスト | `crates/local-sights-core/tests/`、各モジュール |

保存ダイアログとクリップボードは公式プラグインを Rust 側から呼ぶ薄い境界で、自動テストの外（手元の目視で確かめる。5 を見る）。

## 2. 流し方

```bash
# 中核ライブラリ（単体と結合）。行カバレッジつき（80% を下回ったら失敗）
cargo llvm-cov -p local-sights-core --fail-under-lines 80

# Tauri アプリのテスト（export_boundary の一時フォルダのテストと、SSO CLI の偽の CLI のテスト）
cargo test -p local-sights

# 画面のテスト（行カバレッジつき、80% を下回ったら失敗）
npm run test:coverage
```

Rust のテストは、CI の Linux・macOS・Windows（`windows-latest`）の各ランナーで流れる（team.md Testing Posture）。画面のテストは Linux のランナーだけ。

## 3. 期待するカバレッジ

- 中核ライブラリの行カバレッジ 80% 以上（`cargo-llvm-cov`、Linux）。
- 画面の行カバレッジ 80% 以上（Vitest）。
- 中核の公開関数（`LanguagePreference`・`load_settings`・`save_settings`、`write_csv`・`write_markdown`・`sanitize_file_stem`・`default_file_name`・`into_snapshot`・`render_to`・`begin_export`・`finish_export`・`can_export_logs`・`can_export`・`export_source`）にはすべてテストがある。

## 4. テストのデータと環境

- 設定ファイル・書き出しのファイルは `tempfile` の一時フォルダに書く。「書けない」はフォルダのはずの位置に通常のファイルを置いて作る。
- 画面の `src/api.ts` は `vi.mock` で差し替え、`session-changed` は `push(view)` で届ける。OS の言語は `App` の `languages` prop で渡す。
- 秘密の値はテストデータにも入れない。実際のログはリポジトリに置かない。

## 5. 手元で行う結合の確かめ（自動テストではない）

本物の GUI・保存ダイアログ・クリップボード・Excel を使う確かめは、開発者の手元で macOS と Windows 11 の両方で行う（team.md Walking Skeleton の 3。この作業は AWS を新しく呼ばないため 2 の確認用プログラムはあてはまらない）。手順は README の「Language, export and JSON (261022-small-features)」のチェックリストと、`build-and-test-summary.md` の「手元で確かめること」を見る。記録は `test-results.md` に足す。
