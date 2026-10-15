## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/functional-design/rules.md > BR5.2 と BR5.5、functional-spec.md > 1.5 | 今のコードでは、キャッシュの参照中に止める信号が届くと、理由を問わず中止（Aborted）になる。serve_from_cache は retrier.is_stopped() が true だと abort() を呼び、ログを捨てて Idle に戻す。一方 run_fetch_with_cache の look_up はキャッシュの読み取り（非同期）を待つ間も phase は Fetching で、BR5.2 の式では［中断］が押せる。したがってキャッシュに当たる範囲で参照中に［中断］を押すと、BR5.8 の「Done に移り注意を出す」ではなく、注意なしで Idle に戻る。また BR5.2 の「キャッシュから表示中」は、AppSession にその状態を示す項目も FetchSink のメソッドもなく（serve_from_cache は on_started から on_finished まで同期で進む）、何を見て false にするのかが定まっていない。 | キャッシュ参照中（look_up の待ちの間と serve_from_cache）に Cancel が届いたときの終わりを一つに決めて BR5.5 に書く（例：キャッシュで表示せず Cancelled・一覧の前の印あり・0 件にする、または Cancel の理由なら serve_from_cache を最後まで通して Completed とする）。「キャッシュから表示中」「キャッシュ参照中」の判定元（AppSession の項目か、fetchCancelAvailable を false にする別の条件か）を BR5.2 に定める。serve_from_cache と abort() の理由を見る分岐を、BR5.5 の対象箇所として挙げる。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/functional-design/rules.md > BR5.3、BR5.4、functional-spec.md > 1.1 と 1.3 | 止めた後に届く応答の扱いが、ページ以外について定まっていない。今の ReadStage::run は stop 後に届いた結果をすべて捨てて continue する（StreamSchedule::finish も呼ばない）。BR5.3 は「ページは渡す」とだけ書き、BR5.4 は「最後のページだったら Finished」と書くが、(1) 止めた後に届いた最後のページや失敗の応答を schedule.finish に渡すのか、(2) その finish が使い切りの数え（GIVE_UP_LIMIT）を進めて、中断の後に未着手の 133 本を Skipped の失敗に変えうるのか、が書かれていない。(2) が起きると N は 0 近くに縮み、失敗の一覧と件数が増え、3 節の例（N = 138、失敗 2）と食い違う。また、止めた後に非最終ページが届いたストリームを Stopped として schedule に記録する経路も、今の StreamSchedule（Aborted の終わり方を finish で受ける形）をどう使うか書かれていない。 | 止めた後に届く応答を、種類ごとに（非最終ページ、最終ページ、失敗）、理由 Cancel の場合の記録先（ページ追加、finish の呼び方、Stopped にする条件）で表にして BR5.3 または BR5.4 に足す。止めた後は使い切りの数えを進めず、あきらめ（Skipped）を起こさないことを明記する。理由 Close のときは今のとおり捨てることも合わせて書く。 | New |
| R-03 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/functional-design/rules.md > BR5.5、functional-spec.md > 1.3 | 「N = 0 のときは今の決まり」の実現方法が、今のコードの二か所の固定の分岐と衝突するのに触れていない。run_fetch は読む段の後に retrier.is_stopped() なら理由を見ずに Aborted を返し、abort() は status を Aborted に固定する。BR5.5 は decide_job_status の式だけを定めるが、decide_job_status の引数（今は aborted: bool）に理由と N を渡す形、is_stopped() の分岐を理由つきに置き換えること、Cancel で N = 0 のときに Aborted にならず Completed 系で on_finished に進み、キャッシュの書き込みにも進むことが明記されていない。この判定がずれると AC3.1.6（FR5.9）が満たせず、最後のストリーム直後の中断でログが消える。 | BR5.5 に、decide_job_status の新しい入力（止めた理由、N、一覧を読み終えたか）と、run_fetch の is_stopped() 分岐の置き換え方（Close のときだけ abort()、Cancel は表のとおり）を足す。Cancel で N = 0 の取得がキャッシュの書き込みの判定（BR5.7）に進む流れを 1.3 に一文で書く。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/functional-design/rules.md > BR5.1、BR5.8 | cancelling を戻す条件が Cancelled のときしか書かれていない。実際には Completed 系・Failed・Aborted（閉じる）・取得の異常終了（abort_fetch_with_failure の復旧）のどれで終わっても false に戻す必要があり、戻さないとボタンが［中断中…］のまま残る。また begin_and_spawn_fetch は begin_fetch が session のロックを手放してから止める信号の持ち手を AppState.abort に登録するため、その間に cancel_fetch が届くと、session は cancelling を立てるが送る先がなく、永遠に中断中になる。BR5.1 は止める信号を送る主体を AppSession と書いているが、持ち手を持つのは src-tauri の AppState である。 | cancelling と cancelNotice を、どの終わり方でも終わりの処理で初期化すると規則に書く。cancel_fetch で持ち手が無いときは cancelling を立てない（または持ち手の登録を session のロックの中で行う）ことと、送る主体が AppState であることを BR5.1 に明記する。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/functional-design/rules.md > BR5.5、BR5.14 | 上流の約束との違いが明記されていない。C2 と ADR-009 は、読む段の終わり（Cancelled を含む）を StreamSchedule の outcome が決め、unfinished_count を StreamSchedule に足すとしているが、この仕様は Cancelled を FetchCoordinator の decide_job_status で決め、ScheduleEndState は二値のままにしている。また単位の部品表にある RecordingSink の Cancelled と N の記録、ApiBridge の追従が規則に出てこない。 | 決め方の置き場所を FetchCoordinator にする理由（今のコードも ScheduleOutcome を使っていない）を一文で書き、C2 の outcome の扱いを「使わない」と明記する。RecordingSink と ApiBridge の追従を BR5.14 または BR5.13 に足す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| traceability.json の確認（目視） | AC3.1.1 から AC4.2.1 まですべて OK、逆引きは N/A 3 件 | 対応は網羅されている。ただし AC3.1.6（FR5.9）の対応先 BR5.4・BR5.5 は、上の第 3 の指摘のとおり実現方法に穴がある |
| 今のコードとの突き合わせ（coordinator.rs、schedule.rs、src-tauri/src/lib.rs、session.rs） | 上記の指摘 5 件 | キャッシュ参照中の中断、止めた後の応答の扱い、is_stopped() の固定分岐が未整理 |

### Summary

画面と文言、数え方の例はそろっているが、止める信号が来たときの今のコードの分岐（キャッシュ参照中の中止、止めた後の応答の破棄、is_stopped() 一律の Aborted）をどう置き換えるかが書かれておらず、実装者が AC3.1.6 と BR5.8 を満たすには推測が要る。Major 3 件を直せば READY にできる。
