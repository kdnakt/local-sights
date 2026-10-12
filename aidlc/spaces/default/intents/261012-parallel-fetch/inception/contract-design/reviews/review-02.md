## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### 前回の指摘の状態

- R-01（Major）：解消。C2 の `finish` は `StreamFetch`（`outcome` と `end` の組）をそのまま受け取り、`StreamFetchEnd = Completed | Failed { exhausted } | Aborted` を明記した。fetcher.rs:58-90 の型と一致し、`streak_ending()` が使う情報（使い切り・止めた）が入力に揃っている。
- R-02（Major）：解消。`Phase` に値を足さず、`phase` は中断中も `Fetching` のままとし、`cancelling` を別の真偽値にした。session.rs の `phase == Fetching` の判断（1438 の `finish_fetch`、1276 の開始拒否、1502 の閉じる確認、1527 の `abort_fetch` ほか）はそのまま働く。`finish_fetch` の `Failed` 以外は `Done` に移る分岐があるので、`Cancelled` は `Done` に落ちる。残る未決の点は下の新しい指摘に書いた。
- R-03（Major）：解消。C7 で `StopReason`・`StopHandle`・`StopSignal` を契約として定義し、「閉じる」が常に「中断」より強いことと、冪等であることを書いた。retry.rs:205-270 の `AbortHandle`・`AbortSignal`（`watch<bool>`、理由なし）から置き換える範囲と、U1 は `Close` だけを使う方針が分かる。
- R-04（Minor）：ほぼ解消。C5 の `fetchCancelAvailable` と C6 の「false なら何もせず成功」がそろった。ただしキャッシュを読んでいる間の扱いが未決（下の Major）。
- R-05（Minor）：解消。`elapsed` を「`SessionView` を作った時点までの経過」とし、画面が受け取った時点を起点に進め、次を受け取ったら合わせ直す、と起点を定めた。
- R-06（Minor）：解消。C4 に「`on_batch` は同じストリームの `on_stream_finished` より先」を足し、`FetchJob` の既存の項目 13 個は coordinator.rs:100-129 と一致した。`on_planned` を経ない経路も書いた。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C2 StreamSchedule > finish.inputs.outcome | C2 は `finish` が `StreamFetchOutcome` を「そのまま」受け取り、そこから成功・使い切り・使い切り以外の失敗を分けると書いている（components.md は StreamFetchOutcome に `endReason` があるとする）。しかし現行コードの `StreamFetchOutcome`（crates/local-sights-core/src/fetcher.rs）には `status`（Completed/Failed）と `failure` しかなく、「使い切りか」「止めた（Aborted）か」を持つのは別の型 `StreamFetchEnd`（`StreamFetch.end`）である。このままでは、U3 は使い切りを数えられず、U1 の暫定の ExhaustionStreak も、止めたストリームの扱い（U5）も、約束どおりには書けない。 | C2 の `finish` の入力を決める。`StreamFetchOutcome` に終わり方（使い切り／止めた）の項目を足すのか、`StreamFetch`（outcome と end の組）を渡すのかを明記し、ADR-007 と components.md の `endReason` の記述もそれに合わせる。FetchSink の `on_stream_finished(&StreamFetchOutcome)` の形（C4）に影響するかどうかも書く。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 SessionView.phase と C6 cancel_fetch | `phase` に `Cancelling` を足すとだけ書いていて、現行の「取得中」の判定が `Cancelling` でどう振る舞うかが約束にない。現行コードは `phase == Fetching` で多くの処理を分けている（session.rs の 1321・1372・1438・1558・1579 行、画面の src/api.ts の `withProgress`（391 行）、ConditionArea・StatusLine）。たとえば 1438 行は `phase != Fetching` なら `on_finished` を無視するため、`Cancelling` のままでは終わりの通知が捨てられ、中断中のまま止まる。同様に、中断中にも届く取得済みページの `fetch-progress` が画面で捨てられ、件数が止まって見える。 | `Cancelling` を「取得中の判定をすべて満たす段階」として扱うのか、個別に決めるのかを C5 に書く。少なくとも、`Cancelling` 中の `on_batch`・`on_stream_finished`・`on_finished` の受け付け、`fetch-progress` の適用（`withProgress` の条件）、［取得］が押せないこと、接続変更の拒否を表で決める。`phase == Fetching` の判定箇所の洗い出しは U5 の機能設計の入口条件として残す。 | New |
| R-03 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C1 acquire.inputs.stop、C6 behaviour、約束の一覧 | C1 と C6 は `StopSignal`（理由つき、「閉じる」が「中断」より強い）を前提にしているが、`StopSignal` 自体の約束が一覧（C1〜C6）にない。現行は理由のない `AbortHandle`/`AbortSignal`（retry.rs）で、U1 の CallPacer・EventFetcher・StreamPlanner・FetchCheck・TauriApp がこれを使う。U1 は理由なしの型で作り、U5 が理由を足すと、U1→U5 のつなぎ目の型（理由の値、強める規則、理由の読み方、`is_aborted` を残すか）が決まっておらず、U5 で U1 の呼び出し側すべてを書き換えることになりうる。unit-of-work-dependency.md の「つなぎ目」にも載っていない。 | StopSignal を約束に加える（持ち主 U1 か U5 を決める）。理由の値（中断／閉じる）、強める規則（ADR-004）、送り口と受け口の API、受け口が「止めた」を返すだけで理由は FetchCoordinator だけが読むのか、を書く。U1 が作る形と U5 が足す部分を分けて示す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C6 cancel_fetch behaviour と C5 fetchCancelAvailable | C5 は「キャッシュに保存中・キャッシュから表示中は `fetchCancelAvailable` が false」とするが、C6 は「取得中なら理由『中断』を送り Cancelling にする」とだけ書き、`fetchCancelAvailable` が false の間に届いたコマンドの扱いがない。保存中（`on_saving` の後）に届くと、保存は止まらないのに Cancelling になり、終わりが Completed になりうる（FR5.9 の「全部終わった後の中断は終わった扱い」は C2 のストリームを読む段にしか書かれていない）。 | C6 に「`fetchCancelAvailable` が false のときは何もせず成功を返す」と、保存の段に入った後の中断の扱い（終わった扱い）を足す。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 SessionView.elapsed と Open questions | `elapsed` は `running` と `finalMs` だけで、取得中の「ここまでの経過」や開始の基準を持たない。`session-changed` は接続変更や設定など多くの操作で送られるため、画面が受信のたびに時計を起こし直すと経過が戻り、画面を開き直したときは 0 から数え直しになりうる。Open questions に挙げているが、約束の項目の形そのものが答えを縛る。 | 項目の形を決める前提として、画面の時計の起点を送信時点の経過（例：`elapsedMsAtSend`）にするか、「取得の始まりの受信で一度だけ起こす」と約束に書くかを選ぶ。中断中（Cancelling）に時計が動き続けるかも書く。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C4 FetchSink guarantees と FetchJob existing | 同時に読むようになると、あるストリームの `on_batch` がそのストリームの `on_stream_finished` より先に届くこと（件数・`finished_stream_count` の整合）が、1 列に集める方式の約束として書かれていない。また `FetchJob` の `existing` に現行の `served_from_cache`・`cache_outcome`・`read_failed` が載っておらず、「今の項目」の一覧が完全ではない。`on_planned` を経ずに終わる経路（一覧の失敗、一覧の前の中断）も call_order に表れない。 | 「ストリームごとに、その `on_batch` はすべてその `on_stream_finished` より前」を guarantees に足し、`existing` を現行の項目に合わせ、`on_planned` が省かれうる経路を call_order に注記する。 | New |
| R-07 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 `fetchCancelAvailable` と C6 `cancel_fetch` の behaviour | 「キャッシュから表示中は `fetchCancelAvailable` が false」と書いてあるが、今の `AppSession` にはキャッシュを読んでいる状態がなく、持っているのは `cache_saving` だけ（session.rs:461）。しかもキャッシュの照会（`look_up`）は `on_started` より前に行われるため（coordinator.rs の `run_fetch_with_cache`）、その間は `phase == Fetching` のまま `fetchCancelAvailable` が true になり、［中断］を受け付けてしまう。受け付けた後、`serve_from_cache` は止める信号を見て今の `abort` に落ち、`Aborted`（Idle、何も表示しない）で終わる。理由が `Cancel` のときの終わり方は C1〜C7 のどこにも書いていない。 | `fetchCancelAvailable` を何から求めるかを書く（例：`phase == Fetching` かつ `cache_saving` でなく、かつ 取得を始めた後）。キャッシュ照会中と、キャッシュから表示する経路に `Cancel` が届いたときの終わり方（無視する、または `Cancelled` で取れた分を残す）を C6 か C3 に 1 つに決めて書く。 | New |
| R-08 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C7 StopHandle の rule と C3 の `Cancelled` / `Aborted` | 「閉じるは中断より強い」は、信号を送る側の約束としては決まったが、取得の終わりの状態を決める側が理由をいつ読むかがない。`Cancel` で止まり `on_finished` に向かう途中で `Close` が届いたとき、`Cancelled`（ログを残す）と `Aborted`（捨てる）のどちらになるか分からない。 | 終わりの状態を確定する時点（`on_finished` を呼ぶ直前）の理由で決める、と C3 か C7 に 1 行書く。`on_finished` の後に届いた `Close` は取得の結果を変えない、も合わせて書く。 | New |
| R-09 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C5 `cancelling` | `cancelling` が false に戻る条件が「止まったら」としか書かれていない。中断中に「閉じる」で `Aborted`（Idle）になる経路や、取得の開始の失敗で終わる経路でも false に戻るのか、`finish_fetch` のどの分岐で戻すのかが分からない。 | `cancelling` は取得の終わり（`Done`・`Failed`・`Idle` へのどの移り方でも）で false に戻す、と書く。 | New |
| R-10 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > Assumptions & Open Questions と C5 `phase` | 質問票 Q1 の A は「段階（Phase）に『中断中』を足す」を選んでいたが、この版は `phase` に値を足さず `cancelling` を別に足す形に変えた。理由は C5 の note にあるが、Q1 の回答からの変更として仮の決めに載っていない。interaction-spec の FetchCancelButton の `phase` に `Cancelling` があることとの関係（画面側で `cancelling` から導く）もない。 | 仮の決めに「Q1 A の『Phase に中断中を足す』は、`phase == Fetching` の既存の判断を守るため `cancelling` の真偽値に替えた。画面の `Cancelling` は `cancelling` から導く」と 1 行足す。 | New |
| R-11 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/contract-design/contract-summary.md > C7 の見出し注記「RetryPolicy が持つ」 | 今のコードで止める信号を持つのは `Retrier` で（retry.rs:301 の `abort: &AbortSignal`）、`RetryPolicy` ではない。実装者が置き場所を取り違えうる。 | 「Retrier が持つ（RetryPolicy は設定値のまま）」のように持ち主を直す。 | New |

> R-01 Not re-checked this round

> R-02 Not re-checked this round

> R-03 Not re-checked this round

> R-04 Not re-checked this round

> R-05 Not re-checked this round

> R-06 Not re-checked this round

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし | 該当なし | このステージに検証ツールはない。7 つの yaml ブロックが読めることは作成者が確かめたと聞いており、こちらでは確認していない。 |

### Summary

R-01〜R-03 の Major は、コードと突き合わせて解消されている。Critical はなく、Major は 1 件（キャッシュ照会中の［中断］の扱い）なので、U1 の機能設計で決めることを条件に READY とする。決める内容は、キャッシュ経路で理由が `Cancel` のときの終わり方と、`fetchCancelAvailable` の求め方である。
