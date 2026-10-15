## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.4 画面 ConditionArea.tsx（「同じ要素のままなのでフォーカスとタブの順は変わらない」） | 今の ConditionArea.tsx には R-06 の useEffect（225〜230 行）があり、canSubmit が false になった時点で、フォーカスが取得ボタンの囲み（fetchWrapRef）の中にあれば絞り込み欄へ移す。canSubmit は取得中（locked）に false になるため、［取得］を押した直後にフォーカスが［中断］に変わったボタンから絞り込み欄へ奪われる。設計の「フォーカスは変わらない」は今のコードと食い違い、キーボードだけの操作では［取得］のあとにそのまま Space や Enter で中断できない。設計にはこの effect をどうするかの記述がない | R-06 の effect を直す方法を設計に書く（例：Fetching の間は canSubmit ではなく「ボタンが押せない状態」だけを条件にする。［中断］が押せる間はフォーカスを動かさない）。canSubmit と submit（日付欄の Enter）が取得中に中断へ流れないことも書き、画面のテストの確かめることに「［取得］を押した後もフォーカスがボタンに残る」を足す | Resolved (reviewer) |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.3 アプリ cancel_fetch（view を作り、ロックを放してから session-changed を送る） | 今のコードの各コマンドと TauriSink::update は、ロックの中で view を作り、ロックを放してから emit する（lib.rs 1283〜1291 行など）。画面側には view の世代や順番の検査がない。応答待ちを打ち切る新しい設計では、cancel_fetch が handle.stop した直後に取得タスクが一瞬で終わり、別スレッドが finish_fetch を終えて「Done の view」を先に emit できる。その後で cancel_fetch の古い view（Fetching、cancelling=true）が届くと、画面は取得中・［中断中…］のまま固まり、次の session-changed まで直らない。これまでは応答待ちで間があったが、打ち切りでこの窓が現実的になる。設計にこの順序の扱いがない | cancel_fetch の emit の順序を決めて書く。例：emit をセッションのロックを持ったまま行う、view に増える番号を持たせて画面が古い view を捨てる、または cancel_fetch は emit せず取得側の終わりに任せる（cancelling の見た目は request_cancel の直後にだけ必要なので、その場合は最小限の別の手当てを書く）。競合のテスト（stop の直後に finish_fetch）を確かめることに足す | Resolved (reviewer) |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 coordinator.rs ReadStage::run（`tokio::select!` か futures の select） | 選び方が決まっていない。biased でない tokio::select! は両方が準備できているとき無作為に選ぶので、止める信号が来た後に届いたページを足す・次の読みを積む道が残り、NFR4.4（そのページが一覧にない）と N の数えが揺れ、テストが不安定になりうる。retry.rs の wait_or_stop は biased で止めを先にしている | `biased;` で止めを先に置く（または各枝の後で is_stopped を見て捨てる）と書く。止めた時点の「届いたが処理していない」応答は捨てる決まりを明記する | Resolved (reviewer) |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 schedule.rs と FetchJob.unfinished_stream_count | N は ReadStage::run の中の StreamSchedule にしかないが、run は今 u64（最新の版）だけを返し、FetchJob への渡し方が書かれていない。また打ち切りでループを抜けると finish が呼ばれないため、StreamSchedule::stop()（あきらめを止める）は呼ばれる場面がなく、実質何もしない | run が job.unfinished_stream_count を直接設定する、または戻り値に含めるなど、N の渡し方を一つ決めて書く。stop() が要るなら呼ぶ場面を書き、要らなければ外す（BR5.4 の確かめるもの「止めた後はあきらめない」も合わせる） | Resolved (reviewer) |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.1 fetch_status.rs（網羅の検査で漏れる match：CachePlan など）と 2.2 finish_fetch | CachePlan（cache/plan.rs 374 行）は match ではなく `==` で Completed を見ており、Cancelled を足しても直す箇所はない（書く決まりも変わらない）。一方、finish_fetch の cache_notices は Aborted 以外で job の cache_outcome から作られるため、Cancelled でも read_failed などの通知が session に入る。StatusLine が出さないだけで、session の中身は設計に書かれていない | 直す箇所の一覧を実際の match に合わせる。Cancelled のとき cache_notices を空にするか残すかを 2.2 に一言書く | Resolved (reviewer) |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/performance-design.md > NFR4.4 と fetch_stats（NFR2.5 変更なし） | 応答待ちの呼び出しを落とすと、counted_call は await の後で record_attempt するため、打ち切った呼び出しは要約の呼び出し数・試行数に入らない。また最後のページを足し終えて Finished を返すだけの読みも打ち切られて N に数えられ、データは欠けていないのに「未完了 N 本」と出る場合がある | 要約の数え方（打ち切った呼び出しは数えない）と、N に「全ページを足し終えたが終わりを処理していない」ストリームが入ることを設計に一言明記する | Resolved (reviewer) |
| R-07 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/logical-components.md > 部品の表（RecordingSink の行まで）と security-design.md > 3 節「中断の後に始まる呼び出しが 0 回」「一覧の途中の中断」の行 | テストで使う FakeGateway（crates/local-sights-core/tests/support/fake_gateway.rs）は、GetLogEvents にだけ時間（call_duration）を持たせられる。DescribeLogStreams は即座に台本どおり返し、遅らせることも応答を返さないこともできない。一覧の呼び出しを打ち切る検査（NFR4.3 の一覧の段、BR5.5 の一覧の前の中断）はこの偽物の拡張なしには書けないが、部品の表に FakeGateway の変更が載っていない | logical-components.md の表に FakeGateway（tests/support）の変更を一行足す。DescribeLogStreams を遅らせる、または返さない台本を持たせる、と書く | New |
| R-08 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.3 cancel_fetch（「画面が古い［中断中…］で上書きされることはない」） | この保証は取得の終わりの view についてだけ成り立つ。TauriSink::update（lib.rs 1283 から 1291 行）は、ロックの中で view を作り、ロックを放してから emit する。on_stream_finished の更新が cancel_fetch の直前に view を作り、emit が cancel_fetch の emit より後になると、cancelling=false の view が届いて［中断］が一時的に押せる見た目に戻る。そのあとすぐ打ち切りで終わりの view が来て直るので害は小さいが、設計にはこの経路が書かれていない | 2.3 に「途中の更新の古い view は一時的に届きうるが、終わりの view で必ず直る。再び押しても request_cancel は None で何も起きない」と一言書く（または許容と明記する） | New |
| R-09 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-design/security-design.md > 2.4 ConditionArea.tsx（「それ以外（キャッシュに保存中）は disabled の［中断］」） | fetch_cancel_available が false になる理由は保存中だけではない。2.2 の定義どおり、閉じる確認が Pending のときも false で、cancelling でなければ disabled の［中断］になる。また、中断が先に立ったまま全部読み終えてキャッシュの保存に入ると、cancelling が true のままなので disabled の［中断］ではなく loading の［中断中…］が出る。文と実際の分岐が少しずれている | 2.4 のボタンの 4 つの見た目の条件を 2.2 の fetch_cancel_available と cancelling の組み合わせの表で書く。画面のテストの「ボタンの 4 つの見た目」もその表に合わせる | New |

> R-01 Reviewer note: 2.4 に R-06 の effect の条件を「ボタンが要素として無効か」に変える記述が入り、取得中の日付欄の Enter が何もしないこと、画面のテストに「取得を押した後もフォーカスがボタンに残る」が足された。Cloudscape の button/internal.js は loading のとき disabled 属性を付けず aria-disabled だけを付けるので、［中断中…］の間もフォーカスが残るという設計の説明はコードと合う

> R-02 Reviewer note: cancel_fetch が session のロックを持ったまま emit し、取得側の view はロックを取った後に作る、と 2.3 に順序が書かれた。TauriSink の on_finished は取得の同じタスクから呼ばれ、finish_fetch はロックの中、emit はその後なので、最後に届くのは終わりの view になる。順序のテストも確かめることに入った

> R-03 Reviewer note: ReadStage::run を biased で止めを先に置き、reads.next() の枝でも受け取った直後に is_stopped を見て捨てる、と明記された

> R-04 Reviewer note: N は ReadStage が schedule.outcome().unfinished_streams を job.unfinished_stream_count に直接入れる、と決まった。打ち切りで finish が呼ばれないので StreamSchedule に新しいメソッドは足さない、とも書かれ、schedule.rs の今の outcome（slots.len() - finished）と合う

> R-05 Reviewer note: 直す match の一覧から CachePlan を外し（cache/plan.rs は == で見ている）、Cancelled のとき cache_notices を空にすると 2.2 に書かれた

> R-06 Reviewer note: 打ち切った呼び出しは calls と throttled に数えないこと（counted_call は await の後で record_attempt）と、全ページを足し終えて終わりの処理だけが残ったストリームも N に入ることが 2.1 と performance-design に書かれた

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 設計とコードの突き合わせ（coordinator.rs の run_fetch・ReadStage・serve_from_cache・run_fetch_with_cache、schedule.rs、retry.rs、fetcher.rs の counted_call、session.rs の finish_fetch、lib.rs の AppState・begin_and_spawn_fetch・TauriSink、build.rs、capabilities/default.json、ConditionArea.tsx、Cloudscape の button/internal.js） | 食い違いなし。上の Minor 3 件のみ | 打ち切りの核、N の数え、ロックの順（session → abort。abort だけを取る箇所は逆向きの取り方をしない）、権限の 3 か所は今のコードと合う |
| traceability.json | 上流の NFR ID は設計の文書か traceability に載っている | 抜けなし |

### Summary

前回の Major 2 件と Minor 4 件はすべて設計に反映され、コードとも矛盾しない。残るのはテスト用の偽物の拡張の書き漏れと、設計の説明の細かいずれ（Minor 3 件）で、実装を止めるものではないため READY とする。
