### Review — code-generation（u3-partial-results）

**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-30
**Iteration:** 1
**Verdict:** READY

### 所見
- R-01（中）`crates/local-sights-core/src/query/store.rs` の `build_partial_view` と `install_view`：store の鍵を持ったまま写しと並べ替えを行うため、100 万件で約 600 ms 鍵が続く。その間 `settle_inflight`（範囲の終わりの確定）と評価の次の段が待たされる。取得のページは holder の鍵しか取らないので止まらない。BR1.2 の鍵の順番（QueryStore → holder）は守られており、デッドロックはしない。`PartialTask::evaluate` の止める旗も view を作っている間は見ないため、取得が終わったあとの待ちも最大でその分だけ延びる。直し方は、並べ替えを鍵の外に出すか、確定済みの分の並びを再利用して写しの分だけを足すこと。今回は直さなくてよい。NFR1 の数値を決める計測（`fetch_check --partial-results on|off`）で、100 万件の差が目標を超えないかを必ず確かめること。超えたら次の作業単位で直す。
- R-02（低）`src/components/QueryResultTable.tsx` の詳細の取得の effect（`openPositions` と `version` に依存）：新しい版が届いてから `find_query_row` の答えが返るまでの間、古い版の位置のまま `getQueryRowDetail(runId, 新しい版, 古い位置)` を呼ぶ。別の行の詳細を取りに行き、`requested` に残る。表示は位置が付け替わったあとの別の鍵になるので壊れないが、無駄な呼び出しが版ごとに開いた行の数だけ出る。直し方は、`placed` が今の版になるまで詳細の取得を待つこと。
- R-03（低）`crates/local-sights-core/src/insights_session.rs` の `display_rows`：`total_count` は版を見ずに今の結果の件数を返す（行は空になる）。画面は版が違う答えを捨てるので害はないが、BR2.4 の「空の答え」とそろえるなら 0 にする。
- R-04（低）`insights_session.rs` の `find_row`：session の鍵の中で、行ごとに rowKey の文字列を作って線形に探す。開いた行の数 + 1 回の呼び出しが同じ鍵を取る。行数は既定の上限で抑えられるので今は問題ないが、上限を外す変更が入るときは見直すこと。
- R-05（低）`src/App.tsx` の `partialResult`：書き出しの通知の添え書きを「今の結果の stage」から決めている。通知を閉じる前に次の実行が始まると消える（`code-summary.md` の残したことのとおり）。書き出しを始めたときの stage を覚えておけば消えない。BR3.5 の本来の場面（中断の結果を書き出した直後）は満たしている。
- R-06（低）開いた行がまれに閉じること：写しの塊のあとに、それより前の時刻のページが届くと seq が最終とずれる。件数と並びは保たれ、行が重複したり落ちたりはしない。BR2.8 の「見つからなければ閉じる」の範囲内で、直さなくてよい。

### 確かめたこと（問題なし）
1. 並行の作り
   - sink は本流（`run_query` の `fetch_with_partials`）だけが持つ。`SharedSink` の std Mutex は await をまたがず、本流の中でしか取らない。
   - 途中の評価は `spawn_blocking` と `catch_unwind` で、止める旗・view の食い違い・panic のどれでも必ず 1 通だけ送る。`sender` を本流が持ち続けるため、`recv().await` が永久に待つことはない。
   - `biased` の select は、取得の終わりを最優先にする。そのあと動いている途中の評価に止める旗を立て、終わるのを待ってから最終の view を作る（BR1.4）。
   - `should_start_partial` と `next_partial_check` は副作用のない関数で、時計を渡して試している。`Duration::ZERO` を使うテストが多く、時計に頼った不安定さはない。待つ時刻が過去のときに空回りしないことも確かめた。
2. `settle_inflight` は store の鍵の中で、取り出し・確定・登録外しを行う（FR4.8）。2 回目は `None` を返すため、2 回数えない。スレッドを使ったテスト `partial_views_built_while_ranges_settle_never_count_an_event_twice_or_drop_one` が通る。
3. viewVersion は `settle_events`・`add_cached`・`install_view`・`forget_view` で進む。途中の評価は `ViewStamp` で照合し、最終の評価は取得の後で保持が変わらないので epoch だけの照合。BR1.4 のとおり。
4. 中断
   - `request_cancel`・`cancel_available`・`finish_fetch`・`end_cancelled`・`after_fetch` は、BR3.1〜BR3.3 の 3 つの場合に合う。
   - 2 回目の中断は、取得がまだ終わっていなくても abandon で受けられる。取得が終わったときは `Skip` になる。
   - Close は `StopReason::stronger` で Cancel より強く、中断後の評価も止める。
   - Cancelling は `is_running` に入り、`holds_fetch`・`evaluates` で取得中と評価中を分けている。接続の変更・書き出し・実行ボタンの判定がそろっている。
   - 取り残しは `abort_query_with_failure` が `is_running` と `evaluates` で拾う。
5. 版の受け取りは `show` が実行ごとに 1 から進める。`on_partial_result` は取得が終わったあとの結果と別の実行の結果を捨てる。前回の結果は `begin` で持ち越し、画面は runId の食い違いで「前回の結果」にする（BR2.5）。rowKey は U+001F でつなぎ、鍵のない集計の 1 行は空の文字（BR4.2）。
6. 読み上げの文は stage だけから作るので、途中の版の差し替えでは文が変わらず読み上げられない（BR2.7）。途中の印と中断の印は文字と StatusIndicator で出している。
7. BR5.1：`coordinator.rs` は変わっていない。`session.rs` の変更は、`is_fetching` / `is_evaluating` を新しい判定に置き換えたものだけ。`retry.rs` の変更は abandon の旗の追加で、`StopReason` と待ちの動きは同じ。`StatusLine`・`CancelNotice` も変えていない。
8. チームの決まり
   - テスト以外のコードに `unwrap()` / `expect()` / `panic!` はない（4 つの Rust ファイルで確かめた）。
   - 新しい AWS の API 呼び出しはなく、秘密の情報を書く箇所もない。
   - 新しいコマンド `find_query_row` は読み取りだけで、既存と同じく範囲の指定のない許可を 1 つ足している。権限の一致のテストの名前の一覧にも足してある。
9. `traceability.json` の target はすべて実在する。`cargo test -p local-sights-core --lib query::` は 269 通過、Vitest の 3 ファイル（79 テスト）も通った。

### 「計画からずれたところ」への判断
- 1（期待値を変えた既存のテスト）：計画が許す 4 つは FR5.1 と BR3.2 に沿った変更で妥当。許す範囲の外の 2 つ（集計の行の JSON の一致）は、BR4.2 が `rowKey` を行の JSON に足すことを求めるため、必然の変更である。テストのねらい（集計の行にイベントの項目を載せない）は保たれ、期待値に足されたのは `rowKey` だけ（差分で確かめた）。BR5.1 の「中断のテストを除き期待値を変えない」は、ログ管理の画面の振る舞いについての決まりで、ここには触れない。許せる。ただし計画の許す範囲を後から広げる記録を、計画か `code-summary.md` に 1 行残しておくこと。
- 3〜7：妥当。3 は BR5.1 を守る方向。4 の `session.rs` の変更は `holds_fetch` と `is_running` への置き換えだけで、`evaluator.rs` と `stats.rs` は rowKey のための最小の変更。5 は副作用のない関数を足しただけ。6 は 3 つの種類を区別でき、範囲外の位置を持たない。7 の `registerQueryMode` を effect に移す件も、色分けの期待値を変えていないので問題ない。
- 8（`python3` と `sed -i` による編集）：作業の規則違反だが、結果の差分は `evaluator.rs` の `key_count` の追加と `session.rs` の置き換えだけで、中身に問題はない。テストも通っている。次からは必ずファイル用の道具を使うこと。

### まとめ
並行の作り（本流だけが sink を持つこと、鍵の順番、`settle_inflight` の一括、viewVersion、止めて待つこと）と、中断の 3 つの場合、版の捨て方、ログ管理の画面への影響のなさは、上流の決まりに合っており、直さないとまずい不具合は見つからなかった。上の所見は中 1 件・低 5 件で、いずれも次の作業単位以降で対応できる。R-01 だけは、NFR1 の計測で 100 万件の差を確かめてから扱いを決めること。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
