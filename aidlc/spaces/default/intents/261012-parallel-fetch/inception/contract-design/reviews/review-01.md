## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-12T17:05:10Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C2 StreamSchedule > finish.inputs.outcome | C2 は `finish` が `StreamFetchOutcome` を「そのまま」受け取り、そこから成功・使い切り・使い切り以外の失敗を分けると書いている（components.md は StreamFetchOutcome に `endReason` があるとする）。しかし現行コードの `StreamFetchOutcome`（crates/local-sights-core/src/fetcher.rs）には `status`（Completed/Failed）と `failure` しかなく、「使い切りか」「止めた（Aborted）か」を持つのは別の型 `StreamFetchEnd`（`StreamFetch.end`）である。このままでは、U3 は使い切りを数えられず、U1 の暫定の ExhaustionStreak も、止めたストリームの扱い（U5）も、約束どおりには書けない。 | C2 の `finish` の入力を決める。`StreamFetchOutcome` に終わり方（使い切り／止めた）の項目を足すのか、`StreamFetch`（outcome と end の組）を渡すのかを明記し、ADR-007 と components.md の `endReason` の記述もそれに合わせる。FetchSink の `on_stream_finished(&StreamFetchOutcome)` の形（C4）に影響するかどうかも書く。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 SessionView.phase と C6 cancel_fetch | `phase` に `Cancelling` を足すとだけ書いていて、現行の「取得中」の判定が `Cancelling` でどう振る舞うかが約束にない。現行コードは `phase == Fetching` で多くの処理を分けている（session.rs の 1321・1372・1438・1558・1579 行、画面の src/api.ts の `withProgress`（391 行）、ConditionArea・StatusLine）。たとえば 1438 行は `phase != Fetching` なら `on_finished` を無視するため、`Cancelling` のままでは終わりの通知が捨てられ、中断中のまま止まる。同様に、中断中にも届く取得済みページの `fetch-progress` が画面で捨てられ、件数が止まって見える。 | `Cancelling` を「取得中の判定をすべて満たす段階」として扱うのか、個別に決めるのかを C5 に書く。少なくとも、`Cancelling` 中の `on_batch`・`on_stream_finished`・`on_finished` の受け付け、`fetch-progress` の適用（`withProgress` の条件）、［取得］が押せないこと、接続変更の拒否を表で決める。`phase == Fetching` の判定箇所の洗い出しは U5 の機能設計の入口条件として残す。 | New |
| R-03 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C1 acquire.inputs.stop、C6 behaviour、約束の一覧 | C1 と C6 は `StopSignal`（理由つき、「閉じる」が「中断」より強い）を前提にしているが、`StopSignal` 自体の約束が一覧（C1〜C6）にない。現行は理由のない `AbortHandle`/`AbortSignal`（retry.rs）で、U1 の CallPacer・EventFetcher・StreamPlanner・FetchCheck・TauriApp がこれを使う。U1 は理由なしの型で作り、U5 が理由を足すと、U1→U5 のつなぎ目の型（理由の値、強める規則、理由の読み方、`is_aborted` を残すか）が決まっておらず、U5 で U1 の呼び出し側すべてを書き換えることになりうる。unit-of-work-dependency.md の「つなぎ目」にも載っていない。 | StopSignal を約束に加える（持ち主 U1 か U5 を決める）。理由の値（中断／閉じる）、強める規則（ADR-004）、送り口と受け口の API、受け口が「止めた」を返すだけで理由は FetchCoordinator だけが読むのか、を書く。U1 が作る形と U5 が足す部分を分けて示す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C6 cancel_fetch behaviour と C5 fetchCancelAvailable | C5 は「キャッシュに保存中・キャッシュから表示中は `fetchCancelAvailable` が false」とするが、C6 は「取得中なら理由『中断』を送り Cancelling にする」とだけ書き、`fetchCancelAvailable` が false の間に届いたコマンドの扱いがない。保存中（`on_saving` の後）に届くと、保存は止まらないのに Cancelling になり、終わりが Completed になりうる（FR5.9 の「全部終わった後の中断は終わった扱い」は C2 のストリームを読む段にしか書かれていない）。 | C6 に「`fetchCancelAvailable` が false のときは何もせず成功を返す」と、保存の段に入った後の中断の扱い（終わった扱い）を足す。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 SessionView.elapsed と Open questions | `elapsed` は `running` と `finalMs` だけで、取得中の「ここまでの経過」や開始の基準を持たない。`session-changed` は接続変更や設定など多くの操作で送られるため、画面が受信のたびに時計を起こし直すと経過が戻り、画面を開き直したときは 0 から数え直しになりうる。Open questions に挙げているが、約束の項目の形そのものが答えを縛る。 | 項目の形を決める前提として、画面の時計の起点を送信時点の経過（例：`elapsedMsAtSend`）にするか、「取得の始まりの受信で一度だけ起こす」と約束に書くかを選ぶ。中断中（Cancelling）に時計が動き続けるかも書く。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C4 FetchSink guarantees と FetchJob existing | 同時に読むようになると、あるストリームの `on_batch` がそのストリームの `on_stream_finished` より先に届くこと（件数・`finished_stream_count` の整合）が、1 列に集める方式の約束として書かれていない。また `FetchJob` の `existing` に現行の `served_from_cache`・`cache_outcome`・`read_failed` が載っておらず、「今の項目」の一覧が完全ではない。`on_planned` を経ずに終わる経路（一覧の失敗、一覧の前の中断）も call_order に表れない。 | 「ストリームごとに、その `on_batch` はすべてその `on_stream_finished` より前」を guarantees に足し、`existing` を現行の項目に合わせ、`on_planned` が省かれうる経路を call_order に注記する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし | 該当なし | 検証ツールは指定されていない。fenced yaml の構文は作成者が確認済みとのことで、本レビューでは現行コード（fetcher.rs、retry.rs、coordinator.rs、session.rs、lib.rs、api.ts）と突き合わせて形を確かめた。 |

### Summary

約束の範囲と持ち主の分け方は妥当で、秘密情報・API の制限・版番号なしの決まりも上流と矛盾しない。ただし、StreamSchedule への入力型が現行コードに存在しない項目を前提にしていること、`Cancelling` 段階が現行の「取得中」判定とどう結びつくかが未定であること、理由つき StopSignal がつなぎ目として約束化されていないことの 3 点は、U2〜U5 の実装者が推測なしに進めるには足りないため、ゲートで直してから承認することを勧める。
