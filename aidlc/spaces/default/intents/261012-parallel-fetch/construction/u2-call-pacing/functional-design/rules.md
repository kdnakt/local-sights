# Business Rules — U2 呼び出しの速さ（u2-call-pacing）

上流：`inception/requirements-analysis/requirements.md`（FR2.4、FR2.5、FR2.6、NFR3）、`inception/user-stories/stories.md`（US2.2：AC2.2.1〜AC2.2.6）、`inception/units-generation/unit-of-work.md`（U2）、`inception/contract-design/contract-summary.md`（C1）、`entities.md`、U1 の `rules.md`（BR1.1〜BR1.6）。質問票の回答は `functional-design-questions.md` の Q1〜Q3。

## ルール（機械可読）

```yaml
rules:
  - id: BR2.1
    statement: GetLogEvents の試行が制限（Throttled）で終わったら、そのたびに CallPacer に知らせる
    category: constraint
    applies_to: EventFetcher、ThrottleReport
    trigger: GetLogEvents の 1 回の試行が Throttled の失敗で終わったとき（最初の試行でも再試行でも）
    logic: IF 試行の結果が Throttled THEN CallPacer に制限を知らせる。IF DescribeLogStreams か DescribeLogGroups の制限 THEN 知らせない。知らせる場所は、U1 で試行ごとに Throttled を数えている EventFetcher の `counted_call`（fetcher.rs）とする
    violation: なし
    source: FR2.4、約束 C1（report_throttled）
  - id: BR2.2
    statement: 制限の知らせを受けたら、今の速さを半分にする（小数は切り捨て、下限は 1 秒 1 回）
    category: calculation
    applies_to: PacingState.currentRatePerSecond
    trigger: 制限の知らせを受けたとき（BR2.3 で数えないものを除く）
    logic: 先に BR2.4 の戻しを当てはめてから（たまっていた戻しを捨てない）、currentRatePerSecond = max(1, floor(currentRatePerSecond ÷ 2))。holdDecreaseUntil = max(今 + 1 秒, nextPermitAt)。lowestRatePerSecond = min(lowestRatePerSecond, currentRatePerSecond)。東京なら 12 → 6 → 3 → 1 → 1
    violation: なし
    source: FR2.4、Q1
  - id: BR2.3
    statement: 下げてから、1 秒たつまでと、下げたときに予約済みだった枠（古い速さの許可）が出終わるまでのうち遅いほうまでは、制限の知らせでもう一度下げない
    category: policy
    applies_to: PacingState.holdDecreaseUntil
    trigger: 制限の知らせを受けたとき
    logic: IF holdDecreaseUntil があり、今 < holdDecreaseUntil THEN 速さは変えない（ただし BR2.4 の quietSince は今にする）、ELSE BR2.2 で下げる。holdDecreaseUntil は下げたときに max(下げた時刻 + 1 秒, そのときの nextPermitAt)
    violation: なし（同時に読む 8 本の試行がまとめて制限を受けても、また古い速さで予約済みの呼び出しの制限が後から届いても、新しい速さを試す前に重ねて下げない）
    source: Q1 の背景
  - id: BR2.4
    statement: 制限を受けない状態が 5 秒続くごとに、速さを 1 秒あたり 1 回ずつ戻す。リージョンの上限を超えない
    category: calculation
    applies_to: PacingState.currentRatePerSecond、PacingState.quietSince
    trigger: 許可の枠を予約するときと、制限の知らせを受けたとき（戻しは、どちらかが起きたときにまとめて当てはめる）
    logic: n = floor((今 − quietSince) ÷ 5 秒) とし、n > 0 なら currentRatePerSecond = min(ceilingPerSecond, currentRatePerSecond + n)、quietSince = quietSince + n × 5 秒（端数の時間は次の戻しに持ち越す）。currentRatePerSecond が ceilingPerSecond なら速さは変えず、quietSince も変えない。制限の知らせでは、この当てはめの後に quietSince = 今 とする。東京で 3 → 12 に戻るのは、最後の知らせから約 45 秒。例：最後の知らせから 7 秒後に枠を予約すると +1（quietSince は知らせ + 5 秒）、その 3 秒後（知らせから 10 秒）に予約すると、さらに +1
    violation: なし
    source: FR2.4、Q2
  - id: BR2.5
    statement: 速さが変わったら、次に予約する枠からその間隔を使う。予約済みの枠は動かさない
    category: constraint
    applies_to: PacingState.minInterval、PacingState.nextPermitAt
    trigger: 枠を予約するとき
    logic: BR2.4 を当てはめた後の currentRatePerSecond から minInterval を求め（1,000,000,000 ÷ 速さ を切り上げ、U1 の BR1.3）、slot = max(今, nextPermitAt)、nextPermitAt = slot + minInterval をロックの中で決める。下げたときにすでに予約済みの枠は、その時刻のまま。CallPacer の `min_interval()` は今の速さの間隔を返し、上限の速さは今までどおり `ceiling_per_second()` で、今の速さは `current_rate_per_second()`、最も低い速さは `lowest_rate_per_second()` で読めるようにする
    violation: なし。許可の時刻の間は、そのとき使った速さの minInterval 以上あく
    source: FR2.4、U1 の BR1.3
  - id: BR2.6
    statement: 引き下げは取得 1 回の中だけ。次の取得はリージョンの上限から始める
    category: policy
    applies_to: PacingState
    trigger: 取得を始めるとき
    logic: CallPacer は取得ごとに作るので、currentRatePerSecond は ceilingPerSecond、lastDecreaseAt は無し、quietSince は取得の開始時刻から始まる
    violation: なし
    source: ADR-001
  - id: BR2.7
    statement: 引き下げていることは画面に出さない。取得 1 回の要約の 1 行に、取得の間の最も低い速さを足す
    category: policy
    applies_to: FetchStats、StatusLine
    trigger: 取得が終わったとき
    logic: 画面（状態行・通知）には何も足さない。FetchCoordinator が取得の終わりに CallPacer の lowestRatePerSecond を読んで FetchStats に渡し、要約の 1 行の max_in_flight の後・status の前に min_rate=<値> を足す（U1 の要約の欄と順を U2 で変える）。CallPacer を作る前に終わった取得（一覧の途中の中断・一覧の最初のページの失敗）では、リージョンの上限（U1 の BR1.1・BR1.2 の値）を出す。値は整数だけ
    violation: なし
    source: FR2.6、U1 の NFR2.2・NFR8.3
  - id: BR2.8
    statement: 再試行の待ちと回数、同時に読む本数は変えない
    category: constraint
    applies_to: RetryPolicy、StreamSchedule
    trigger: 制限を受けたとき
    logic: 再試行の待ち（1・2・4・8・16 秒 ±20%）と回数（要求ごとに最大 5 回）は今のまま。同時に読む本数は 8 本のまま（Q3）。再試行の前にも許可を待つ（U1 の BR1.4）ので、下げた速さは再試行にも効く
    violation: なし
    source: FR2.5、Q3
```

## まとめ

| ID | 内容 | 分類 |
|----|------|------|
| BR2.1 | GetLogEvents の試行が制限で終わったら知らせる | constraint |
| BR2.2 | 知らせを受けたら速さを半分に（下限 1） | calculation |
| BR2.3 | 下げてから 1 秒と予約済みの枠が出終わるまでは下げない | policy |
| BR2.4 | 制限なしが 5 秒続くごとに +1、上限まで | calculation |
| BR2.5 | 速さの変化は次の枠から | constraint |
| BR2.6 | 引き下げは取得 1 回の中だけ | policy |
| BR2.7 | 画面に出さず、要約の 1 行に最も低い速さ | policy |
| BR2.8 | 再試行と同時に読む本数は変えない | constraint |

## Assumptions & Open Questions

None.
