## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/functional-design/rules.md の BR2.7、functional-spec.md の 1 章 手順 6 | `min_rate=` の値がどこから来るかが決まっていない。U1 の実装では、要約を作る `report_summary` が受け取るのは `FetchStats` だけで、CallPacer は渡されない。さらに CallPacer は一覧の後（coordinator.rs の `CallPacer::for_region`）で作られるため、一覧の途中で止めた場合の要約（Aborted、`elapsed_ms=0`）の時点では CallPacer がまだ存在しない。この経路で `min_rate` に何を出すかが書かれておらず、BR2.7 の「値は整数だけ」を満たせない。また要約の 1 行の形（U1 の NFR2.2、`FetchSummary` の欄と `the_line_has_the_fixed_fields_in_the_fixed_order` のテスト）が変わることも、U1 の約束の変更として書かれていない。 | 最も低い速さの持ち主と渡し方を決めて書く（例：`FetchStats::summary` の引数にするか、`FetchSummary` に欄を足す）。CallPacer が無い経路（一覧の途中の中断）では、リージョンの上限の値を出す、と決める。U1 の要約の形と既存テストを U2 で更新することを 4 章の確かめ方に足す。 | Resolved (reviewer) |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/functional-design/entities.md の PacingState.quietSince の制約 | 戻したときの quietSince の扱いが、rules.md と食い違う。entities.md は「速さを 1 戻すたびに、戻した時刻にする」と書くが、BR2.4 と functional-spec.md の擬似コードは quietSince = quietSince + n×5 秒（戻しの時刻ではなく数え進めた時刻）である。戻しは次の許可の要求のときに遅れて当てはめるため、2 つは別の動きになる（例：最後の知らせから 7 秒後に要求が来ると、前者は次の戻しまで 5 秒、後者は 3 秒）。4 章の「戻す」のテストは 5・10・45・60 秒ちょうどで見るので、この差を見分けられない。 | entities.md の記述を BR2.4 に合わせて「quietSince + n×5 秒にする」と直す。端数の秒が残る場合を見分けるテスト（例：知らせから 7 秒後に 1 戻り、さらに 3 秒後に 2 戻る）を 4 章の表に足す。 | Resolved (reviewer) |
| R-03 | Minor | functional-spec.md の 1 章 手順 4 と擬似コード report_throttled | report_throttled は、まだ当てはめていない戻しを先に当てはめずに、速さを半分にして quietSince を今にする。戻しは次の reserve_slot まで遅れるので、直前の許可の要求から知らせまでの間（低い速さでは、最大 8 件の予約済みの枠のために数秒開く）に溜まった戻しの 1 ステップが捨てられることがある。許可の要求の時刻で戻した速さと、呼び出しが実際に出る枠の時刻にも差がある。 | report_throttled の先頭でも reserve_slot と同じ戻しを当てはめる、または「この分は捨てる」と書いて受け入れる。どちらかを決め、テストに足す。 | Resolved (reviewer) |
| R-04 | Minor | rules.md の BR2.3、functional-spec.md の 3 章の例 | 1 秒の間は下げない決まりは、下げた時刻から数える。しかし予約済みの枠は古い速さのまま、最大 8 件ぶん後で出る（3 → 1 のときは 333 ミリ秒 × 8 で約 2.7 秒）。新しい速さの最初の呼び出しが出る前に、古い速さの呼び出しの制限の知らせが 1 秒を過ぎて届くと、もう一度半分に下げてしまい、まだ試していない速さを待たずに 1 まで落ちる。AC2.2.3 は満たすが、BR2.3 の趣旨（1 回の混雑を 1 回の引き下げにする）からは外れる。 | 受け入れるなら、その旨を BR2.3 に書く。直すなら、予約済みの枠が出終わる時刻（下げたときの nextPermitAt）まで数えない、とする。 | Resolved (reviewer) |
| R-05 | Minor | functional-spec.md の 1 章、entities.md の PacingState | 実装の入り口が決まっていない点がある。(a) report_throttled の呼び出し位置は、U1 のコードでは `counted_call` が自然（Throttled かどうかをすでに調べている）だが、名前が書かれていない。(b) 今ある `CallPacer::min_interval()` と固定の `interval` 欄は、今の速さで変わる値になる。getter が上限の間隔か今の間隔かが決まっていない。(c) pacing.rs の「ロックは 1 つの時刻だけを守るので、ロックが壊れても途中の状態は残らない」というコメントは、複数の欄を持つ状態では成り立たない。 | (a)〜(c) を 1 行ずつ書き足す。(c) は R-01 の持ち越し（冒頭の文書の書き直し）と同じ箇所で直す。 | Resolved (reviewer) |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/functional-design/rules.md の BR2.6 の logic | 取得の始めの状態として、廃止された欄 lastDecreaseAt（無し）が残っている。今の欄は holdDecreaseUntil で、entities.md と spec にはこの名前がない。実装者が欄を取り違える。 | 「holdDecreaseUntil は無し」に書き直す。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/functional-design/functional-spec.md の 4 章「予約済みの枠の後から届いた知らせ」の条件 | 条件が「3 に下げたとき予約済みの枠が 2.7 秒先まである」となっているが、2.7 秒は 3 の速さで予約した 8 件ぶん（333 ミリ秒 × 8）で、3 から 1 に下げるときの値である。合格の基準も「3.0 秒の後の知らせで 1 に」と 3 から 1 を指している。6 から 3 に下げる場合の予約済みの枠は約 1.3 秒ぶんなので、文のとおりに読むと条件と基準が合わない。 | 条件を「速さ 3 で枠が 2.7 秒先まで予約されているとき、1 に下げる」の形に直す。 | New |

> R-01 Reviewer note: BR2.7 と spec 1 章 6・4 章に、FetchCoordinator が lowestRatePerSecond を読んで FetchStats に渡すこと、欄の順、CallPacer を作る前に終わった取得ではリージョンの上限を出すこと、U1 の書式のテストを直すことが書かれた。テストの行も足されている。

> R-02 Reviewer note: entities.md の quietSince が quietSince + n × 5 秒（端数の持ち越し）になり、rules.md の BR2.4 と擬似コードに一致した。7 秒後・10 秒後のテストで見分けられる。

> R-03 Reviewer note: report_throttled の先頭で apply_recovery を呼ぶ形になった。12 秒後に知らせるテスト（5 にしてから 2）の数値は合っている。

> R-04 Reviewer note: holdDecreaseUntil = max(今 + 1 秒, nextPermitAt) になり、3 章の例（3.0 秒、3.5 秒で 3）の数値も合っている。

> R-05 Reviewer note: counted_call、min_interval() と各 getter、ロックの説明の書き直しが entities.md と spec 6 章に書かれた。

### Summary

1 回目の Major 2 件（min_rate の出どころと CallPacer が無い経路、quietSince の食い違い）と Minor 3 件は、成果物の上ですべて直っている。lazy recovery の n 段、holdDecreaseUntil、予約済みの枠、1 秒の上限、3 章の例、4 章のテストの数値を確かめ、計算は合っている。画面は変わらず、禁止事項にも触れない。残るのは名前の古い参照と、テスト条件の言い回しの 2 件の Minor だけで、実装を妨げないため、判定は READY とする。
