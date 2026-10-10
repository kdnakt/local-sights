## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-08T09:56:09Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | src-tauri/src/lib.rs（`app_menu`、`quit_requested`、`handle_menu_event`、`run` の `#[cfg(target_os = "macos")] builder.menu(app_menu)`）、README.md | Cmd+Q とアプリメニューの Quit をこのアプリ自身の項目（`QUIT_MENU_ID`、`CmdOrCtrl+Q`）に替え、`on_menu_event` から `must_stop_closing` に通し、止めなければ `app.exit(0)` する形になった。`app.exit` は `ExitRequested` に届き、[Close] 後は `exit_allowed` で通る。Edit の標準項目も残してあり、コピー・貼り付けは保たれる。ウィンドウの閉じるは従来どおり。Dock の終了だけは止められないため既知の制約として README と code-summary.md（計画との違い 10）に記録された。実機（macOS）では未確認で、README の手元の確認項目に載っている。`cargo test -p local-sights-core` は通る（src-tauri のメニューは自動テストなし） | Cmd+Q の経路は直った。Dock の終了の扱いは R-08 に切り出した | Resolved |
| R-02 | Minor | src/i18n/messages.ts（`error.retry.*`）、src/errorText.ts（`retryStartsSentence`、`capitalized`） | 日本語は「[取得]」に、英語は文中で小文字の「press」になり、文頭に来る Other だけ `errorLines` が頭文字を大文字にする。`errorText.test.ts` と `messages.test.ts` で確認され、正本との違いは code-summary.md の計画との違い 9 に記録されている。functional-spec.md の BR2.2 は書き換えていない（機能設計の上限のため）が、違いは明示されている | なし。次のユニットか Build and Test で functional-spec.md を実装に合わせてよい | Resolved |
| R-03 | Minor | crates/local-sights-core/src/coordinator.rs（`prepare_write`、`save`）、cache/mod.rs（`WriteTracker::on_idle`、`WriteGuard::drop`）、src-tauri/src/lib.rs（`watch_cache_writes`） | 書き込み中の印を `spawn_blocking` の前に立ててクロージャへ move するため、積んでから走るまでの隙間は無くなった。最後の書き込みが終わったときに `on_idle` が 1 回呼ばれ、アプリは非同期の実行環境に渡し、取得中でなければ `session-changed` を再送する。リスナーはスロットのロックを外して呼ぶ。テスト 5 件（印が積む前に立つ、走らず捨てても下りる、`save` 後に下りて 1 回聞こえる、聞き手なしでも下りる）が通る | なし | Resolved |
| R-04 | Minor | src-tauri/src/lib.rs（`row_positions`）、log_view.rs（`checked_positions_of`、`TooManyKeys`、`MAX_POSITION_KEYS`）、src/hooks/expansionState.ts、src/components/LogTable.tsx、src/hooks/useRowPositions.ts | 上限（10,002）を超えると黙って切り詰めず `rows.tooManyKeys` の Err を返す。画面も開ける行を 10,000 までにして知らせを出すので、通常は Err に届かない。Err は `onError` に渡る。上限ちょうどは答え、1 つ超えたら Err になるテストがあり、診断は件数だけでキーを含まない | なし | Resolved |
| R-05 | Minor | src/rowLayout.test.ts、src/virtualScroll.ts、src/hooks/expansionState.ts、src-tauri（`find_row_position`） | 自己参照の総当たりは U3 の固定の期待値を `rowLayout` に直接当てる形に替わった。未使用の `scrollTopForKey`・`preservedScrollTop`・`toggleSelected`・`find_row_position`（権限・toml・api の型まで）は削除された。`virtualScroll.test.ts` の残った期待値は変わっていない。`npx vitest run` は 171 件成功 | なし | Resolved |
| R-06 | Minor | src/components/ExpandedMessage.tsx、src/components/LogTable.test.tsx | 展開部分に `role="region"` が付き、`getByRole("region", { name: "Full message" })` のテストがある | なし | Resolved |
| R-07 | Minor | construction/u7-ui-polish/code-generation/traceability.json、code-summary.md「複数のファイルにまたがる ID の対応」 | traceability.json 自体は変わっていない（1 ID 1 ファイルの書式のため）。代わりに code-summary.md に、複数ファイルにまたがる ID と R-11〜R-14・U6 持ち越しの対応表が追加された。source-manifest.json は更新されている。機械が読む対応は代表の 1 ファイルのまま | なし（対応表で足りるとする） | Resolved |
| R-08 | Minor | src-tauri/src/lib.rs（`app_menu` の注記）、README.md の U7 確認項目、code-summary.md の計画との違い 10 | Dock の「終了」は取得中でも確認なしでアプリが終わる（FR4.9 / BR3.1 (2) の一部が未達）。tao 0.37.1 の `applicationWillTerminate` を経由するため、アプリから止められない。取得中のログは失われるがキャッシュは壊れない（書き込みは一時ファイル経由）。制約として記録されているが、人間の承認の記録はまだ無い | チェックポイントで、この制約を受け入れるか（Accepted risk）、`applicationShouldTerminate:` の仕組みを次のユニットに回すかを人間が決める | Accepted risk |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core` | lib 305 件成功（5 ignored）、他の統合テストも全て成功 | ライブラリ側は合格。R-03・R-04 の追加テストを含む |
| `npx vitest run` | 171 件成功 | 画面側は合格。R-02・R-04・R-05・R-06 の更新を含む |
| 禁止事項の確認 | 該当なし | 追加された診断は `eprintln!` で件数のみ。読み取り API 以外の呼び出し、AWS 接続、テレメトリ、非テストの `unwrap`・`expect` の追加はなし。権限は `find_row_position` の分が減っただけ |

### Summary

R-01 から R-07 はすべて修正が確認できた。Cmd+Q は独自メニューで止められるようになり、書き込み中の印、問い合わせの上限、テストの自己参照、アクセシビリティも直っている。残るのは Dock の終了が止められないという記録済みの制約（R-08、Minor）だけで、チェックポイントで人間が受け入れを決めればよい。
