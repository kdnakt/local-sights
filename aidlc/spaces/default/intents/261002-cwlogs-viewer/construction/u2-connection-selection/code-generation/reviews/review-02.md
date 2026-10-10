## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T14:10:00Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | crates/local-sights-core/src/catalog/mod.rs > テスト unclosed_profile_header_is_skipped_and_the_rest_is_used、code-summary.md「計画との違い」7 | 人間の判断（寛容に読む方針を BR1.3 からの意図した逸脱として扱う）どおりに記録されている。code-summary の項目 7 は、解釈できない行だけを読み飛ばすこと、その場合は知らせを出さないこと、知らせを出すのは開けない・ディレクトリ・UTF-8 でないときだけであることを正直に書いている。テストは 2 つあり、閉じていない見出しの region が前後のプロファイルに漏れないこと、読めた区画は使われることを断言している。逸脱そのものは再提起しない | なし（意図した逸脱として承認済み。BR1.3 の見直しは機能設計側の持ち越し事項） | Resolved |
| R-02 | Major | crates/local-sights-core/tests/u1_fetch_flow.rs（screen_connection_passes_the_selected_profile_and_region ほか 2 件）、traceability.json の BR2.8 | 解消した。with_connection で持たせたプロファイルとリージョンが、GetLogEvents の全ページの呼び出しに入ることを偽物の gateway の記録で断言している。SDK 既定の場合と、確認用プログラムの経路（region なし）も確かめている。traceability の BR2.8 の対応先もテストに直った。cargo test --workspace で 11 件成功を確認した | なし | Resolved |
| R-03 | Minor | crates/local-sights-core/src/session.rs > update_input・update_log_group_filter・ensure_can_change、src/components/ConfirmDialog.tsx > trapFocus、FetchForm.tsx、LogGroupPane.tsx | 解消した。BR2.6 のとおり、確認待ちの間は AppSession が入力・絞り込み・選択・再読み込み・取得を拒否する（ConfirmationPending）。画面側も入力欄・絞り込み・一覧・ボタンを無効にし、ダイアログは Tab と Shift+Tab を 2 つのボタンの間で循環させる。取得中の絞り込みは許可され、テストで確かめている。テスト 2 件（typing_and_filtering_are_refused_while_confirmation_is_pending ほか）と画面のテストがある。なお、ダイアログの外（背景）をクリックして body にフォーカスがある状態から Tab を押すと、aria-disabled の一覧（tabIndex=0）にフォーカスが入るが、操作は AppSession が拒否するため実害はない | なし | Resolved |
| R-04 | Minor | src/components/LogGroupPane.tsx > renderStatus | 解消した。Partial かつ emptyState が NoMatches のとき、「途中まで」と「一致するロググループがありません」の両方を出す。LogGroupPane.test.tsx にテストが足されている。npx vitest run は 59 件成功 | なし | Resolved |
| R-05 | Minor | src-tauri/src/lib.rs > change_connection | ほぼ解消した。timeline の鍵を取ったまま、セッションの変更と clear を行い、clear の後に session-changed を送る。取得は begin_fetch の後に timeline の鍵を待つため、遅れた clear が新しい取得のログを消すことはない。鍵の順序は timeline、次に session（std の短い鍵）で、start_fetch・TauriSink とも session の鍵を持ったまま timeline を待つ箇所はなく、デッドロックは見つからなかった。取得中は鍵を取らず、取得が先に終わって clear が要る場合だけ session の鍵を放してから待つ。残る小さな懸念は R-06 に分ける | なし | Resolved |
| R-06 | Minor | src-tauri/src/lib.rs > update_log_group_filter、select_log_group（change_connection 経由の async コマンド）、src/App.tsx > handleFilter | 絞り込みと一覧の選択は timeline を触らないのに change_connection を通り、同期コマンドから async コマンドになった。async コマンドは別々のタスクとして実行されるため、キーストロークごとに送られる update_log_group_filter が到着順に適用される保証がなくなり（以前は同期で順序どおり）、まれに入力欄の文字列とセッション側の絞り込み文字列がずれる可能性がある。また、取得中でない間は、これらの操作が無関係な timeline の鍵を待つようになった。実際の確率は低く、一覧の表示がずれるだけで、データや安全には影響しない。src-tauri にはこの経路のテストがない | 次の作業単位で、update_log_group_filter と select_log_group を timeline を取らない同期コマンドに戻す（change_connection はログを捨てる操作だけに使う）。または絞り込みの各呼び出しに連番を付けて古いものを捨てる | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test --workspace | PASS（lib 124、u1_fetch_flow 11、u2_log_group_listing 9） | 修正後も回帰なし |
| cargo fmt --check | PASS | 問題なし |
| cargo clippy --workspace --all-targets | PASS（警告 0） | 問題なし |
| npx vitest run | PASS（59 件） | R-03・R-04 の画面側テストを含む |
| npx tsc --noEmit・npx prettier --check .・npx eslint . | PASS | 問題なし |
| cargo build -p local-sights | PASS | async 化したコマンドもビルドできる |

### Summary

人間の判断どおり R-01 は意図した逸脱として正直に記録されテストもあり、R-02 から R-05 の修正は検証できた。ロック順序（timeline、次に session）にデッドロックはなく、重大な回帰は見つからなかった。残るのは、絞り込みの async 化による順序保証の低下という Minor の指摘（R-06）だけで、READY とする。
