## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | construction/u3-give-up-rule/nfr-design/security-design.md の設計表 NFR3.7 の行、決まりの正本の「終わり方の受け取り（R-02）」の行 | 使い切りが失敗の内容を必ず持つ形の作り方が二択のまま決まっていない。「StreamFetchEnd の使い切りの値が ApiFailure を持つか、finish が StreamFetchOutcome の失敗を必須で受ける」と書かれ、「StreamFetchEnd（と失敗の内容）をそのまま受け取る」とも書かれていて、finish の引数の型が 1 つに定まらない。現行コードでは StreamFetchEnd は Copy で、Failed は exhausted の真偽値だけを持ち、失敗の内容は StreamFetch.outcome.failure（Option）に別にある。前者を選ぶと Copy が外れ、fetcher.rs の既存テスト（StreamFetchEnd::Failed { exhausted: true } との等価比較、526・541・655 行付近）と next_page の生成箇所（276 行）が変わる。後者は Option を必須にする型の変更が要る。どちらも型で保証すると言うには形が要り、開発者は推測するしかない。脅威表の「型で保証」も、実際には start_next が next_unstarted を末尾へ進める状態に頼っている。 | 型を 1 つに決めて書く。例：StreamFetchEnd::Failed に ApiFailure を持たせる（Copy は外す）、または finish の引数を終わり方と必須の ApiFailure を持つ新しい型にする。それに合わせて fetcher.rs の生成箇所・既存テストの書き換え範囲を示す。型で保証するという表現は、実際の仕組み（未着手を末尾へ進める状態）に合わせて直す。 | New |
| R-02 | Minor | construction/u3-give-up-rule/nfr-design/security-design.md の設計表 NFR7.1 の行、nfr-design-questions.md の「終わり方の型」 | NFR7.1 は「ほかは変えずに通す」と書くが、finish が StreamFetchEnd を受け、StreamSchedule::new が本数の引数を持たなくなると、schedule.rs の既存テスト（finish に SlotEnd と Option の失敗を渡す箇所、new の第 2 引数 3、ScheduleOutcome の完全一致の比較）の大半が書き換えになる。また StreamEnding は、security-design では「なくす」、質問票では「schedule.rs の中だけで使う型にする」と食い違い、schedule.rs に既にある SlotEnd::streak_ending と分け方が二重になるかどうかも書かれていない。 | 既存テストの書き換えは呼び出しの形だけで、確かめる中身は変えないと明記する。StreamEnding を消すのか schedule.rs の内部に移すのかを 1 つに決め、SlotEnd::streak_ending との関係を書く。 | New |
| R-03 | Minor | construction/u3-give-up-rule/nfr-design/security-design.md の決まりの正本「観測の口（R-04）」の行、「本数（BR3.6）」の行 | あきらめの記録の欄の名前と型が決まっていない。ScheduleOutcome は今 Copy で、ストリーム名・ApiFailure・本数を足すと Copy が外れる。あきらめた後に数えを動かさない仕組み（あきらめ済みの印を持つのか）、テスト用の本数を渡す関数の可視性（cfg(test) か pub か）も書かれていない。このままではテストを先に書く手順が取れない。 | ScheduleOutcome に足す欄の名前と型（例：Option の GiveUp 構造体）、あきらめ済みの印、テスト用コンストラクタの可視性を書く。 | New |
| R-04 | Minor | construction/u3-give-up-rule/nfr-design/traceability.json | upstream_ids と coverage に AC2.3.1〜AC2.3.4、BR3.1〜BR3.4、BR3.7、ADR-007、C2 が無い。NFR 要件のレビュー R-03 は AC2.3.1〜AC2.3.4 を載せるよう求めていたが、security-design.md の「確かめること」の由来欄に書かれているだけで、対応表には入っていない。 | traceability.json に AC2.3.1〜AC2.3.4 と BR3.1〜BR3.4・BR3.7 を足し、確かめること表の行に対応づける。 | New |

### Summary

方向は ADR-007 と合っている。数えの持ち主を StreamSchedule に移し、依存は StreamSchedule から EventFetcher への片方向で、schedule.rs が fetcher.rs の StreamFetchEnd を使い、fetcher.rs は schedule を使わない。RetryPolicy::default() だけを使う呼び出し側（結合テスト、src-tauri）は変更不要で、これも現行コードと一致した。未着手を呼ばない保証の要となる「使い切りが失敗の内容を必ず持つ」形が二択のままで、現行の StreamFetchEnd（Copy で失敗の内容を持たない）と fetcher.rs の既存テストへの影響も示されていないため、開発者が推測なしに実装できず、NOT-READY とする（Major 1 件）。残りは Minor 3 件である。
