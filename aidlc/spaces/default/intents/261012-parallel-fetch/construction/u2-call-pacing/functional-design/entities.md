# Entities — U2 呼び出しの速さ（u2-call-pacing）

上流：`inception/domain-design/components.md`（CallPacer）、`inception/contract-design/contract-summary.md`（C1 の `report_throttled`）、`inception/units-generation/unit-of-work.md`（U2）、`inception/requirements-analysis/requirements.md`（FR2.4、FR2.6）、U1 の `construction/u1-concurrent-skeleton/functional-design/entities.md`（PacingState、RegionCallLimit）。質問票の回答は `functional-design-questions.md` の Q1〜Q3。

U2 で意味が変わるのは、U1 の PacingState だけである。RegionCallLimit・StreamSlot・PageStep・StopReason・JobStatus は U1 のまま変えない。

## エンティティ（機械可読）

```yaml
entities:
  - name: PacingState
    owner: CallPacer
    description: 取得 1 回で全ストリームが共有する速さの状態。U2 で、今の速さと引き下げ・戻しの時刻を足す
    attributes:
      - name: ceilingPerSecond
        type: integer
        required: true
        constraints: U1 のまま。RegionCallLimit から決まり、取得の間ずっと変わらない。今の速さの上限
      - name: currentRatePerSecond
        type: integer
        required: true
        min: 1
        constraints: 今の速さ（1 秒あたりの許可の数）。最初は ceilingPerSecond。1 以上 ceilingPerSecond 以下
      - name: minInterval
        type: duration
        required: true
        constraints: 1,000,000,000 ÷ currentRatePerSecond ナノ秒を切り上げた値（U1 の BR1.3 の計算を今の速さに当てる）。速さが変わると次の予約から変わる
      - name: nextPermitAt
        type: instant
        required: true
        constraints: U1 のまま。次の許可を出してよい最も早い時刻
      - name: holdDecreaseUntil
        type: instant
        required: false
        constraints: この時刻より前に届いた制限の知らせでは、もう一度下げない（BR2.3）。下げたときに max(下げた時刻 + 1 秒, そのときの nextPermitAt) にする。まだ下げていなければ無い
      - name: quietSince
        type: instant
        required: true
        constraints: 制限を受けない状態の数え始め。最初は取得の開始時刻。制限の知らせを受けるたび（下げなかったときも）その時刻にする。戻しを n 段当てはめたら quietSince + n × 5 秒にする（端数の時間は次の戻しに持ち越す、BR2.4）
      - name: lowestRatePerSecond
        type: integer
        required: true
        min: 1
        constraints: 取得の間に currentRatePerSecond がとった最も低い値。最初は ceilingPerSecond。FetchCoordinator が取得の終わりに読み、FetchStats に渡して要約の 1 行に出す（BR2.7）
    constraints:
      - 1 <= currentRatePerSecond <= ceilingPerSecond
      - すべての読み書きは U1 と同じロックの中で行う。下げ・戻し・枠の予約は 1 回のロックの中で決める。欄の値はロックの中で先に計算し、まとめて書く（途中で panic する処理を挟まない）ので、ロックが壊れても中身はどれかの操作の前か後の値になる。U1 と同じく壊れたロックは中身をそのまま使う
      - 取得 1 回の中だけのもの。次の取得はリージョンの上限から始める
    relationships:
      - target: RegionCallLimit
        cardinality: many-to-one
        direction: PacingState → RegionCallLimit（ceilingPerSecond の値だけの参照）

  - name: ThrottleReport
    owner: EventFetcher
    description: GetLogEvents の 1 回の試行が制限（Throttled）で終わったことの知らせ。値を持たない
    attributes:
      - name: reportedAt
        type: instant
        required: true
        constraints: 知らせた時刻（CallPacer がその場で戻らない時計から読む）
    constraints:
      - GetLogEvents の試行（最初の試行と再試行のどちらも）だけが知らせる。DescribeLogStreams・DescribeLogGroups は知らせない
    relationships:
      - target: PacingState
        cardinality: many-to-one
        direction: ThrottleReport → PacingState
```

## まとめ

| エンティティ | 持ち主 | U2 での役割 |
|--------------|--------|-------------|
| PacingState | CallPacer | 今の速さ、次に下げてよい時刻、制限を受けない状態の数え始め、取得の間の最も低い速さを足す |
| ThrottleReport | EventFetcher | GetLogEvents の試行が制限で終わったことの知らせ（約束 C1 の `report_throttled`） |

## Assumptions & Open Questions

None.
