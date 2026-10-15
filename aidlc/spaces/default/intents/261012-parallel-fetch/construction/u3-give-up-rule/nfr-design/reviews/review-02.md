## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | construction/u3-give-up-rule/nfr-design/security-design.md の設計表 NFR3.7 の行、決まりの正本の「終わり方の受け取り（R-02）」の行 | 使い切りが失敗の内容を必ず持つ形の作り方が二択のまま決まっていない。「StreamFetchEnd の使い切りの値が ApiFailure を持つか、finish が StreamFetchOutcome の失敗を必須で受ける」と書かれ、「StreamFetchEnd（と失敗の内容）をそのまま受け取る」とも書かれていて、finish の引数の型が 1 つに定まらない。現行コードでは StreamFetchEnd は Copy で、Failed は exhausted の真偽値だけを持ち、失敗の内容は StreamFetch.outcome.failure（Option）に別にある。前者を選ぶと Copy が外れ、fetcher.rs の既存テスト（StreamFetchEnd::Failed { exhausted: true } との等価比較、526・541・655 行付近）と next_page の生成箇所（276 行）が変わる。後者は Option を必須にする型の変更が要る。どちらも型で保証すると言うには形が要り、開発者は推測するしかない。脅威表の「型で保証」も、実際には start_next が next_unstarted を末尾へ進める状態に頼っている。 | 型を 1 つに決めて書く。例：StreamFetchEnd::Failed に ApiFailure を持たせる（Copy は外す）、または finish の引数を終わり方と必須の ApiFailure を持つ新しい型にする。それに合わせて fetcher.rs の生成箇所・既存テストの書き換え範囲を示す。型で保証するという表現は、実際の仕組み（未着手を末尾へ進める状態）に合わせて直す。 | Resolved (reviewer) |
| R-02 | Minor | construction/u3-give-up-rule/nfr-design/security-design.md の設計表 NFR7.1 の行、nfr-design-questions.md の「終わり方の型」 | NFR7.1 は「ほかは変えずに通す」と書くが、finish が StreamFetchEnd を受け、StreamSchedule::new が本数の引数を持たなくなると、schedule.rs の既存テスト（finish に SlotEnd と Option の失敗を渡す箇所、new の第 2 引数 3、ScheduleOutcome の完全一致の比較）の大半が書き換えになる。また StreamEnding は、security-design では「なくす」、質問票では「schedule.rs の中だけで使う型にする」と食い違い、schedule.rs に既にある SlotEnd::streak_ending と分け方が二重になるかどうかも書かれていない。 | 既存テストの書き換えは呼び出しの形だけで、確かめる中身は変えないと明記する。StreamEnding を消すのか schedule.rs の内部に移すのかを 1 つに決め、SlotEnd::streak_ending との関係を書く。 | Resolved (reviewer) |
| R-03 | Minor | construction/u3-give-up-rule/nfr-design/security-design.md の決まりの正本「観測の口（R-04）」の行、「本数（BR3.6）」の行 | あきらめの記録の欄の名前と型が決まっていない。ScheduleOutcome は今 Copy で、ストリーム名・ApiFailure・本数を足すと Copy が外れる。あきらめた後に数えを動かさない仕組み（あきらめ済みの印を持つのか）、テスト用の本数を渡す関数の可視性（cfg(test) か pub か）も書かれていない。このままではテストを先に書く手順が取れない。 | ScheduleOutcome に足す欄の名前と型（例：Option の GiveUp 構造体）、あきらめ済みの印、テスト用コンストラクタの可視性を書く。 | Resolved (reviewer) |
| R-04 | Minor | construction/u3-give-up-rule/nfr-design/traceability.json | upstream_ids と coverage に AC2.3.1〜AC2.3.4、BR3.1〜BR3.4、BR3.7、ADR-007、C2 が無い。NFR 要件のレビュー R-03 は AC2.3.1〜AC2.3.4 を載せるよう求めていたが、security-design.md の「確かめること」の由来欄に書かれているだけで、対応表には入っていない。 | traceability.json に AC2.3.1〜AC2.3.4 と BR3.1〜BR3.4・BR3.7 を足し、確かめること表の行に対応づける。 | Resolved (reviewer) |
| R-05 | Minor | construction/u3-give-up-rule/nfr-design/security-design.md の設計表 NFR7.1 の行 | fetcher.rs のテストで直す対象を StreamFetchEnd::Failed { exhausted: true } と比べているものだけにしているが、526 行付近と 655 行付近の Failed { exhausted: false } との等価比較も、failure を持つ形になると同じく書き換えが要る（失敗の内容を持つ値を作らないと比べられない）。 | 直すテストの範囲を、Failed との等価比較すべて（exhausted の真偽を問わず）に広げて書く。コード生成の中で直せる軽微な点であり、判定には影響しない。 | New |

> R-01 Reviewer note: StreamFetchEnd::Failed を { exhausted, failure: ApiFailure } の 1 形に一本化し、Copy をやめて Clone にした。finish が失敗を必ず写せるので、内容が無い場合の分岐はなくなった。

> R-02 Reviewer note: NFR7.1 に直すテストの範囲が限って書かれ、StreamEnding 型は作らず SlotEnd に数えの分け方を持たせる形に決まった。質問票との食い違いは security-design.md が優先と明記されている。

> R-03 Reviewer note: GIVE_UP_LIMIT、公開の with_limit、GiveUp の 3 欄、ScheduleOutcome.give_up（Clone）が決まった。ScheduleOutcome を使う箇所は schedule.rs の中だけなので、Copy をやめても影響は閉じている。

> R-04 Reviewer note: traceability.json に AC2.3.1〜4、BR3.1〜3.7、ADR-007、C2 が入った。

### Summary

1 回目の指摘 4 件はいずれも解消した。使い切りが失敗の内容を必ず持つ形が型で決まり、finish・本数・あきらめの記録の型も実装できる粒度で書かれ、ADR-007 の片方向の依存（StreamSchedule から EventFetcher へ）と合っている。残るのは直すテストの範囲の書き漏れという Minor 1 件のみで、Critical と Major がないため READY とする。
