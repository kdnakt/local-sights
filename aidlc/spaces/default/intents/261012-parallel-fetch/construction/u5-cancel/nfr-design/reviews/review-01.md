## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.4 画面 ConditionArea.tsx（「同じ要素のままなのでフォーカスとタブの順は変わらない」） | 今の ConditionArea.tsx には R-06 の useEffect（225〜230 行）があり、canSubmit が false になった時点で、フォーカスが取得ボタンの囲み（fetchWrapRef）の中にあれば絞り込み欄へ移す。canSubmit は取得中（locked）に false になるため、［取得］を押した直後にフォーカスが［中断］に変わったボタンから絞り込み欄へ奪われる。設計の「フォーカスは変わらない」は今のコードと食い違い、キーボードだけの操作では［取得］のあとにそのまま Space や Enter で中断できない。設計にはこの effect をどうするかの記述がない | R-06 の effect を直す方法を設計に書く（例：Fetching の間は canSubmit ではなく「ボタンが押せない状態」だけを条件にする。［中断］が押せる間はフォーカスを動かさない）。canSubmit と submit（日付欄の Enter）が取得中に中断へ流れないことも書き、画面のテストの確かめることに「［取得］を押した後もフォーカスがボタンに残る」を足す | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.3 アプリ cancel_fetch（view を作り、ロックを放してから session-changed を送る） | 今のコードの各コマンドと TauriSink::update は、ロックの中で view を作り、ロックを放してから emit する（lib.rs 1283〜1291 行など）。画面側には view の世代や順番の検査がない。応答待ちを打ち切る新しい設計では、cancel_fetch が handle.stop した直後に取得タスクが一瞬で終わり、別スレッドが finish_fetch を終えて「Done の view」を先に emit できる。その後で cancel_fetch の古い view（Fetching、cancelling=true）が届くと、画面は取得中・［中断中…］のまま固まり、次の session-changed まで直らない。これまでは応答待ちで間があったが、打ち切りでこの窓が現実的になる。設計にこの順序の扱いがない | cancel_fetch の emit の順序を決めて書く。例：emit をセッションのロックを持ったまま行う、view に増える番号を持たせて画面が古い view を捨てる、または cancel_fetch は emit せず取得側の終わりに任せる（cancelling の見た目は request_cancel の直後にだけ必要なので、その場合は最小限の別の手当てを書く）。競合のテスト（stop の直後に finish_fetch）を確かめることに足す | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 coordinator.rs ReadStage::run（`tokio::select!` か futures の select） | 選び方が決まっていない。biased でない tokio::select! は両方が準備できているとき無作為に選ぶので、止める信号が来た後に届いたページを足す・次の読みを積む道が残り、NFR4.4（そのページが一覧にない）と N の数えが揺れ、テストが不安定になりうる。retry.rs の wait_or_stop は biased で止めを先にしている | `biased;` で止めを先に置く（または各枝の後で is_stopped を見て捨てる）と書く。止めた時点の「届いたが処理していない」応答は捨てる決まりを明記する | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 schedule.rs と FetchJob.unfinished_stream_count | N は ReadStage::run の中の StreamSchedule にしかないが、run は今 u64（最新の版）だけを返し、FetchJob への渡し方が書かれていない。また打ち切りでループを抜けると finish が呼ばれないため、StreamSchedule::stop()（あきらめを止める）は呼ばれる場面がなく、実質何もしない | run が job.unfinished_stream_count を直接設定する、または戻り値に含めるなど、N の渡し方を一つ決めて書く。stop() が要るなら呼ぶ場面を書き、要らなければ外す（BR5.4 の確かめるもの「止めた後はあきらめない」も合わせる） | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 fetch_status.rs（網羅の検査で漏れる match：CachePlan など）と 2.2 finish_fetch | CachePlan（cache/plan.rs 374 行）は match ではなく `==` で Completed を見ており、Cancelled を足しても直す箇所はない（書く決まりも変わらない）。一方、finish_fetch の cache_notices は Aborted 以外で job の cache_outcome から作られるため、Cancelled でも read_failed などの通知が session に入る。StatusLine が出さないだけで、session の中身は設計に書かれていない | 直す箇所の一覧を実際の match に合わせる。Cancelled のとき cache_notices を空にするか残すかを 2.2 に一言書く | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/performance-design.md > NFR4.4 と fetch_stats（NFR2.5 変更なし） | 応答待ちの呼び出しを落とすと、counted_call は await の後で record_attempt するため、打ち切った呼び出しは要約の呼び出し数・試行数に入らない。また最後のページを足し終えて Finished を返すだけの読みも打ち切られて N に数えられ、データは欠けていないのに「未完了 N 本」と出る場合がある | 要約の数え方（打ち切った呼び出しは数えない）と、N に「全ページを足し終えたが終わりを処理していない」ストリームが入ることを設計に一言明記する | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 設計とコードの突き合わせ（coordinator.rs、schedule.rs、retry.rs、session.rs、lib.rs、ConditionArea.tsx、build.rs、capabilities） | 上の Major 2 件を確認 | ReadStage の打ち切り、plan_streams の Aborted、serve_from_cache の分岐、StopReason の強さ、build.rs と capabilities の 3 か所はコードと矛盾しない |
| traceability.json | 読み込み可。上流の NFR ID は設計の文書か traceability に載っている | 抜けなし |

### Summary

打ち切りの核（reads と stop の並べ待ち、decide_job_status の表、ロック順 session → abort、Close が勝つ決まり）は今のコードと整合する。ただし、取得ボタンを［中断］に兼ねる案は今の R-06 のフォーカス移動と衝突し、応答を待たない打ち切りは cancel_fetch の古い view が最終の view を上書きする競合を現実にする。この 2 点を設計に書いてから確認する必要がある。
