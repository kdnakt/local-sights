# Business Rules — U3 あきらめの決まり（u3-give-up-rule）

上流：`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4）、`inception/user-stories/stories.md`（US2.3：AC2.3.1〜AC2.3.4）、`inception/units-generation/unit-of-work.md`（U3）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）、`entities.md`、U1 の `rules.md`（BR1.13・BR1.19）。まとめの確認は `functional-design-questions.md`。

## ルール（機械可読）

```yaml
rules:
  - id: BR3.1
    statement: ストリームが終わった順に、再試行を使い切って失敗したものを数える。成功が 1 本終われば 0 に戻す
    category: calculation
    applies_to: ExhaustionStreak.consecutive
    trigger: ストリームが終わったとき（StreamSchedule が終わりを受け取ったとき）
    logic: IF 終わり方が Exhausted THEN consecutive + 1、IF Succeeded THEN 0。数える順は、FetchCoordinator が終わりを受け取った順（読み始めた順ではない）
    violation: なし
    source: FR3.1、AC2.3.1
  - id: BR3.2
    statement: 使い切りが limit 本（既定 3）たまったら、まだ読み始めていないストリームはすべて呼ばずに失敗（Skipped）とし、limit 本目の使い切りが起きたストリームの失敗の内容を写す
    category: policy
    applies_to: StreamSlot、GiveUp
    trigger: consecutive が limit に達したとき
    logic: IF consecutive >= limit AND まだあきらめていない THEN 未着手の StreamSlot をすべて Failed（Skipped）にし、そのストリームの失敗の内容を写した結果を、未着手の並び順（選ばれた順）に記録する。GiveUp を残す
    violation: なし（Skipped のストリームでは GetLogEvents を呼ばない）
    source: FR3.2、AC2.3.2
  - id: BR3.3
    statement: あきらめたとき読み途中のストリームは、最後まで読む
    category: policy
    applies_to: StreamSlot（InFlight）
    trigger: あきらめたとき
    logic: InFlight のストリームはそのまま読み続け、終わったら成功・失敗のどちらもそのまま記録する。あきらめの後に終わったものは数えに影響しない（もう一度あきらめることはない）
    violation: なし
    source: FR3.3、AC2.3.3
  - id: BR3.4
    statement: 使い切り以外の失敗（権限がないなど、再試行しない失敗）は数を 0 に戻す
    category: calculation
    applies_to: ExhaustionStreak.consecutive
    trigger: ストリームが FailedWithoutExhaustion で終わったとき
    logic: consecutive = 0
    violation: なし
    source: FR3.4、AC2.3.4
  - id: BR3.5
    statement: 閉じて止めたストリーム（Aborted）は数えない
    category: policy
    applies_to: StreamEnding.NotCounted
    trigger: ストリームが止める信号で終わったとき
    logic: consecutive は変えない（増やしも 0 に戻しもしない）。閉じた取得は Aborted で終わる（U1 の BR1.16）。中断（Cancel）のときの扱いは U5 で決める
    violation: なし
    source: ADR-004、U1 の BR1.16
  - id: BR3.6
    statement: 数えの持ち主は StreamSchedule。RetryPolicy は 1 回の要求の再試行と止める信号だけを持ち、EventFetcher は自分の終わり方だけを返す
    category: constraint
    applies_to: StreamSchedule、RetryPolicy、EventFetcher
    trigger: 設計の決まり
    logic: ExhaustionStreak・limit（既定 3）・StreamEnding と終わり方の分け方を StreamSchedule に置く。RetryPolicy から max_consecutive_exhausted をなくし、EventFetcher の streak_ending をなくす。依存は StreamSchedule → EventFetcher の片方向。FetchCoordinator は StreamSchedule を limit とともに作る
    violation: なし
    source: ADR-007、約束 C2
  - id: BR3.7
    statement: 終わった本数・失敗の一覧・取得の終わりの状態の決め方は U1 のまま
    category: constraint
    applies_to: FetchJob、FetchCoordinator
    trigger: ストリームが終わったとき、Skipped にしたとき
    logic: finishedStreams は終わった順に 1 ずつ（Skipped も 1 本ずつ、U1 の BR1.19）。Skipped は失敗の一覧に入る。終わりの状態は U1 の BR1.15 のとおり（失敗があれば CompletedWithFailures）
    violation: なし
    source: U1 の BR1.15・BR1.19
```

## まとめ

| ID | 内容 | 分類 |
|----|------|------|
| BR3.1 | 終わった順に使い切りを数え、成功で 0 | calculation |
| BR3.2 | limit 本で未着手を呼ばずに失敗、失敗の内容を写す | policy |
| BR3.3 | 読み途中は最後まで読む、あきらめは一度だけ | policy |
| BR3.4 | 使い切り以外の失敗で 0 | calculation |
| BR3.5 | 閉じて止めたものは数えない | policy |
| BR3.6 | 持ち主は StreamSchedule（ADR-007） | constraint |
| BR3.7 | 本数・一覧・終わりの状態は U1 のまま | constraint |

## Assumptions & Open Questions

None.
