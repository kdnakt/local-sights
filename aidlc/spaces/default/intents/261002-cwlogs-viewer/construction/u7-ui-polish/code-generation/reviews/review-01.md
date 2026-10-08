## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-08T05:56:41Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | src-tauri/src/lib.rs:1034-1042（`app.run` のコールバックの `RunEvent::ExitRequested`）、src-tauri/src/lib.rs:358（`must_stop_closing`）、README.md の手元の確認項目 | BR3.1 (2) は Cmd+Q と Dock の「終了」を `RunEvent::ExitRequested` + `prevent_exit` で止める前提だが、依存の実装を読むと macOS ではその経路に届かない。アプリはカスタムメニューを持たず Tauri の既定メニューを使い、その Quit は muda 0.20.0 で `terminate:`（macos/mod.rs:944）に結び付く。tao 0.37.1 の app_delegate は `applicationWillTerminate` だけを実装し（app_delegate.rs:130、`applicationShouldTerminate` は無い）、そこから `AppState::exit()` → `LoopDestroyed` へ進む。tauri-runtime-wry 2.12.1 が `ExitRequested` を出すのは最後のウィンドウの Destroyed（lib.rs:4262）と `Message::RequestExit`（`app.exit()`、lib.rs:4302）だけである。Cmd+Q・Dock の終了では `ExitRequested` が出ず、取得中でも確認なしでプロセスが終わる可能性が高い（FR4.9 の 2 経路が未達）。ウィンドウの閉じるボタンは `CloseRequested` で止まるため、そちらは問題ない。自動テストでは確かめられず、code-summary.md も「来るか未確認」と書くだけで、動かなかった場合の代替がない。実機は未確認で、ソース読みによる判断である | 実機（macOS）で Cmd+Q と Dock の終了を取得中に試し、止まらなければ次のどれかで対処する。(a) 独自のアプリメニューを置き、Quit 項目（Cmd+Q）を `on_menu_event` で `must_stop_closing` に通してから `app.exit(0)` する。(b) Dock の終了は (a) では拾えないため、`applicationShouldTerminate:` を返す仕組み（delegate への追加など）を入れるか、「Dock の終了は確認できない」を BR3.1 の既知の制約として記録して人間の承認を取る。どの場合も README と code-summary.md の記述を実際の動きに合わせる | New |
| R-02 | Minor | src/i18n/messages.ts（`error.retry.fetch` の ja、`error.next.*` の en）、functional-spec.md の BR2.2 | 正本どおりの文言は実装されている（9 種類 × 英日をすべて突き合わせて一致を確認）。ただし正本自体に 2 つの不具合が残っている。(1) 日本語の画面のボタンは `form.fetch` = 「取得」なのに、次の行動は「[Fetch] でもう一度取得してください」で、BR2.2 の「ボタン名は画面の表示どおり」と食い違う。(2) 英語は「Wait a while, then Press [Fetch] to fetch again.」のように文中で Press が大文字になる。code-summary.md の未解決欄に記載済みだが直っていない | 機能設計の上限に達しているため、このユニットでは直さず、次のユニットか Build and Test で扱う項目として明示する（ja は「[取得]」、en は小文字の「press」を差し込む形）。受け入れる場合は Accepted risk として記録する | New |
| R-03 | Minor | crates/local-sights-core/src/coordinator.rs:634-641（`save`）、cache/mod.rs（`LogCache::write` の `_writing`）、session.rs:712-720 | U6 R-02 の「書き込み中の印」は `LogCache::write` の中（`spawn_blocking` のクロージャの内側）で立つため、タスクを積んでから実行が始まるまでの間は `is_writing()` が false になる。取得のタスクがちょうどそこで異常終了すると、設定ダイアログが開けて全削除のあとに書き戻される隙間が残る。また、異常終了後の書き込みが終わって印が下りても `session-changed` は出ず、`canOpenSettings=false` のビューが次のイベントまで残る | `save` で `cache.writes().start()` を `spawn_blocking` の前に呼び、ガードをクロージャへ move する。書き込みが終わったとき（異常終了後の孤立した書き込みの場合）にビューを再送する経路を足すか、制約として記録する | New |
| R-04 | Minor | src-tauri/src/lib.rs:322-325（`row_positions`）、src/hooks/useRowPositions.ts:93-94 | 1 回の求めが 10,000 キーを超えると `keys` を黙って切り詰めて返す。画面は足りない分を `answer.positions[index] ?? null`（隠れた行）として扱うため、展開は残るが位置が無くなり、選択はキー不明のまま同じ位置に戻る。実際の利用で 10,000 展開は現実的でないが、失敗が無言である | 上限を超えたら `Err` を返す（または画面側で展開の数に上限を設ける）。少なくとも切り詰めを `null` と区別できるようにする | New |
| R-05 | Minor | src/rowLayout.test.ts（「gives U3's answers when nothing is expanded」の総当たり）、src/virtualScroll.ts、src/hooks/expansionState.ts（`toggleSelected`）、src-tauri/src/lib.rs の `find_row_position` | R-12 の「U3 と同じ答え」の確認のうち、`rowLayout` と `virtualScroll` の総当たりの比較は、`virtualScroll.ts` が `rowLayout.ts` へ委譲するため同じコードを比べるだけで検証になっていない。独立した確認になるのは変更されていない `virtualScroll.test.ts` の期待値のほう（これは通っている）。また `virtualScroll.ts` の `scrollTopForKey`・`preservedScrollTop`、`expansionState.toggleSelected`、画面が使わなくなった `find_row_position`（権限付き）は、テスト以外から使われない | 総当たりの比較は U3 の固定値の期待に置き換えるか、委譲であることを明記して重複を外す。未使用の関数・コマンドは次のユニットで削除するか、残す理由を記録する | New |
| R-06 | Minor | src/components/ExpandedMessage.tsx:63-70 | キーボードで入る展開部分は `tabIndex=0` で `aria-label` を持つが role が無く、role の無い要素への aria-label は支援技術に伝わらない。読み上げでは「メッセージの全文」の名前が出ない | 展開部分に `role="region"`（または `group`）を付ける。スクロール可能な領域としての名前と、`aria-readonly` のテキスト領域のどちらにするかを決めてテストを足す | New |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u7-ui-polish/code-generation/traceability.json | 40 の ID に対し対応先が 1 ファイルだけで、BR1.7（`log_view.rs` のみで `lib.rs`・`useRowPositions.ts` が無い）、BR3.1（`session.rs` のみで `lib.rs` の `CloseRequested`/`ExitRequested` が無い）などは実装が複数のファイルにまたがる。plan §1 の R-11〜R-14 と U6 の持ち越し（R-01〜R-04、R-06）は coverage に載っていない。source-manifest.json は変更ファイル 46 件と一致しており正確 | 複数ファイルにまたがる ID は対応先を追加し、R-11〜R-14 と U6 の持ち越しを coverage に足す | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `npx vitest run`（U7 の 21 ファイル） | 167 件成功 | 画面側は通る。テストの質は上の R-05 のとおり、総当たりの一部が自己参照 |
| `cargo test -p local-sights-core --lib -- log_view:: session:: coordinator:: cache::` | 138 件成功、3 件 ignored | ライブラリ側は通る。ただし Tauri の閉じる・終えるのつなぎは自動テストの対象外で、R-01 は実機でしか分からない |
| `cargo fmt --check`・`npx tsc --noEmit`・`npx eslint .`・`npx prettier --check .` | 問題なし | 書式と型は合格 |
| 禁止事項の確認（認証情報・外部送信・読み取り API 以外・ログファイル） | 該当なし | 診断は `eprintln!` だけで、ログ用の依存も無い。非テストの `unwrap`・`expect` の追加なし。`capabilities/default.json` に足した権限は 3 つだけ。展開部分は `{message}` のテキストノードで、`dangerouslySetInnerHTML` は無い |

**確認して問題がなかった点**

- rowLayout の計算：R-12 の正規化（最大の仮想の上端 ↔ 最大のスクロール位置）は 100 万件で末尾の行に届く（`visibleRange` の描画の合計が末尾まで続く）。`rowAt` の境界（展開の末尾の直後は次の行）、ゼロ高さ、範囲外の位置は問題なし。
- discardGeneration（R-11）：LogView が `clear()` の中で、保持ログがあったときだけ増やす。`positions_of` と SessionView の両方に載り、画面は新しい世代を正にする。`set_discard_generation` は `max` で戻らない。
- 閉じる・終える（`CloseRequested` 経路）：`exit_allowed` を先に立て、セッションのロックは `app.exit` の前に放されるため再入でも詰まらない。Pending 中の再要求は止めたまま。Destroyed は中断だけ。
- U6 の持ち越し：設定の I/O は async＋`spawn_blocking` でロックの外、`settings_busy` で別操作を断る、`eventCount` v2（範囲ごとの件数の突き合わせと版 1 の破棄）、保持ログの写し取りは `spawn_blocking`、R-06 の 3 つのテストが追加済み。
- 画面：`aria-activedescendant` は一覧の外枠、見出しは `aria-rowindex=1`、`tabIndex=0` は選択行のスクロールする展開部分だけ（R-13）。展開部分の中ではキーを処理しない。

### Summary

画面の計算（rowLayout）、discardGeneration の持ち主、U6 の持ち越し、エラー文の正本との一致は妥当で、テストも通る。一方、依存（tao 0.37.1・muda 0.20.0）のソースを読む限り、macOS の Cmd+Q と Dock の終了では `RunEvent::ExitRequested` が出ず、取得中の確認が効かない可能性が高い（R-01、Major 1 件）。他は Minor で、BR2.2 の正本の文言の不具合（R-02）は機能設計の上限のため先送りの扱い。
