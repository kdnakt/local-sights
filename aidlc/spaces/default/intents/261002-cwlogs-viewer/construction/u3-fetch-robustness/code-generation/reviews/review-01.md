## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-05T00:04:38Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/components/LogTable.tsx > 「BR6.5: keep the top row in place」の useEffect（timelineVersion 依存） | 位置を保つ基準の行を、バージョンが変わるたびに「最後のスクロール位置（scrollRef）」と「最後に受けた行のまとまり（windowRef）」から取り直している。1 回目の find_row_position の返答が戻る前に次の timelineVersion が届くと、スクロール位置は未補正のまま、windowRef が新しいバージョンの行（挿入で下にずれた別の行）に置き換わっていることがあり、基準の行が別の行にすり替わる。ずれは取得中の追加のたびに積み上がりうる。また、返答前の 1〜数フレームは、新しい件数・古いスクロール位置で描かれるため、行が一瞬ずれて見える | 基準の行の (logStreamName, sequence) と pixelOffset を ref に保持し、find_row_position の返答を適用するまでは取り直さない（返答前に次のバージョンが来たら、同じ基準で問い合わせ直す）。この単位では直さず、次の作業単位に回してよい（project.md の決まり） | New |
| R-02 | Minor | crates/local-sights-core/src/session.rs > finish_fetch（Aborted の分岐）、src-tauri/src/lib.rs > run（CloseRequested/Destroyed） | BR4.5 は「中断時も timelineVersion を増やす」。コア側の `abort()` は timeline.discard() で増やすが、Aborted の結果を受ける finish_fetch は timelineVersion をセッションに書かない（件数を 0 にして Idle に戻すだけ）。ウィンドウを閉じるときだけ起きるため、いまは画面に影響しないが、U7 で閉じる操作をキャンセルできるようになると、保持ログが破棄されたのに画面の timelineVersion が古いままになる | U7 で閉じる確認を足すときに、Aborted の終了で破棄後のバージョンを受け口に渡してセッションに書く（FetchSink::on_finished か on_aborted に版を持たせる）。いまは未解決事項として記録する | New |
| R-03 | Minor | src-tauri/src/lib.rs > begin_and_spawn_fetch の監視タスクと recover_if_still_fetching | 回復処理は「いまの phase が Fetching か」だけを見ていて、自分の取得（fetch_number・job_id）のものか確かめない。取得 A のタスクが終わってから監視タスクが回復処理を実行するまでの間に、取得 B が始まっていると、B を「結果なしで終了」と誤って Failed にしうる（間隔は実際にはごく短く、確率は低い） | recover_if_still_fetching に fetch_number（または job_id）を渡し、セッションの現在の取得が自分のものと一致するときだけ失敗に移す | New |
| R-04 | Minor | crates/local-sights-core/src/timeline.rs > append、src-tauri/src/lib.rs > get_rows・find_row_position | append は挿入位置から後ろを split_off してマージするため、新しいストリームの時刻が既存の範囲と重なると、1 ページごとに保持件数に比例する移動（と一時的に 2 倍の領域）が起きる。その間 Mutex を握り、同期コマンドの get_rows・find_row_position（Tauri のメインスレッド）が待たされる。index() も、すでに登録済みのストリームでも 1 件ごとに stream 名の String を複製している。100 万件の自動測定は rows と position だけで、append と、その間に画面の取り出しが待たされる時間は測っていない（code-summary.md に「数十ミリ秒程度」とあるのは推測） | リリースビルドの ignored テストに「100 万件を保持した状態での 1 ページ（約 10,000 件）の append の時間」を加えて確かめる。index() は get_mut で既存の項目を引いて複製を避ける。手元の確認（README の U3 の項目）で、100 万件近いときの取得中の画面の引っかかりも見る | New |
| R-05 | Minor | src-tauri/src/lib.rs > TauriSink::update、crates/local-sights-core/src/session.rs > view | 取得中は 1 ページごとに SessionView 全体（プロファイル一覧・ロググループ一覧の visible_groups を含む）を作って emit する。ロググループが数千件あると、1 ページごとの直列化とフロントの再描画の負荷になる。code-summary.md の「まだ確かめていないこと」に自己申告済み | 手元で確かめ、必要なら進み具合だけの軽いイベントに分ける（次の作業単位） | New |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/code-generation/traceability.json > coverage（BR5.5、FR4.10、BR6.5、BR6.8、BR4.5） | 対応先が実装を示していないものがある。BR5.5 と FR4.10 はテストファイル（tests/u3_fetch_flow.rs）だけ。実装は coordinator.rs（abort）・retry.rs（wait_or_abort）・src-tauri/src/lib.rs（ウィンドウを閉じるときの中断）と、失敗の取りまとめは coordinator.rs・session.rs。BR6.5 は virtualScroll.ts の純粋な計算だけで、位置を保つ処理の本体は LogTable.tsx。BR6.8 は messages.ts だけで、キーボード操作は LogTable.tsx・virtualScroll.ts。BR4.5 は timeline.rs だけで、破棄を起こす coordinator.rs・src-tauri/src/lib.rs が入っていない | 実装のファイルを対応先に足す（1 つの ID に複数の対応先を並べる）か、テストは別の欄に分ける。トレーサビリティの正確さだけの修正で、コードには影響しない | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| コード読みによる確認（BR1.1〜BR6.9 と実装の突き合わせ） | 重大な不一致なし | 時刻なしストリームの打ち切り（BR1.2・BR1.3）、再試行の待ち・上限・要求ごとの回数・使い切りの連続数のリセット（BR2.1・BR2.2・R-03・R-09）、中断（BR2.3・BR5.5）、並べ替えのキー・マージ・位置の索引（BR4.1〜BR4.4）、timelineVersion の増え方（取得の開始・追加・接続先の変更での破棄、BR4.5）、結果の状態の決め方（BR5.2）、世代番号による古い操作の破棄（BR6.7）は、仕様どおりに実装されていることをコードで確かめた |
| unwrap()/expect()/panic!/todo! の検索（テスト部分を除く） | 0 件 | team.md の「テスト以外で unwrap/expect を使わない」を満たす |
| 禁止事項の確認（project.md Forbidden） | 問題なし | 呼ぶ API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ（gateway の trait にそれ以外の手段がない）。標準エラー・診断ログに出すのは safe_detail だけ。テストは FakeGateway で、AWS には接続しない。テレメトリはなし。Tauri の権限は新しいコマンドの 3 つだけを足している |
| ロックの順（セッション → 保持ログ）の確認 | デッドロックなし | 取得の流れはページごとに保持ログのロックを手放してからセッションのロックを取る。change_connection だけが両方を握り、順は常にセッション → 保持ログ。接続先の変更での破棄は取得中（phase Fetching）は拒まれるため、遅れた破棄が次の取得のログを消すことはない |
| 依頼で報告された cargo test・vitest・fmt・clippy・tsc・prettier・eslint・build | 提出された結果を信頼（今回は再実行していない） | 実行の結果は code-summary.md の記録と矛盾しない |

### Summary

Critical・Major の指摘はなく、READY とする。BR の仕様どおりに、列挙・再試行・時刻順のマージ・位置の問い合わせ・世代番号・ロックの順序が実装されており、project.md の禁止事項も守られている。残る指摘はすべて Minor で、画面の位置保持の基準の行の取り方（R-01）、中断時の timelineVersion の伝え方（R-02）、回復処理の取得の同一性（R-03）、大量件数での append の負荷の未測定（R-04）、トレーサビリティの対応先（R-06）は、project.md の決まりに従って次の作業単位に回してよい。
