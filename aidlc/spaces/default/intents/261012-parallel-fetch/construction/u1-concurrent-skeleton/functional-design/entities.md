# Entities — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`inception/domain-design/components.md`（エンティティの持ち主）、`inception/contract-design/contract-summary.md`（C1〜C4、C7）、`inception/units-generation/unit-of-work.md`（U1 の範囲）、`inception/requirements-analysis/requirements.md`。質問票の回答は `functional-design-questions.md` の Q1・Q2。アーキテクチャレビュー（1 回目）の指摘を反映済み。

U1 で新しく持つもの、または U1 で意味が変わるものだけを、論理的な型と制約のレベルで定める。名前と持ち主は components.md に合わせる。今のまま変えないエンティティ（PlannedStream、StreamFetchOutcome、ConnectionTarget など）は載せない。

## エンティティ（機械可読）

```yaml
entities:
  - name: RegionCallLimit
    owner: CallPacer
    description: リージョンごとの GetLogEvents の既定の上限（1 秒あたり、アカウントとリージョンごと、引き上げ不可）と、アプリが使う上限
    attributes:
      - name: regionCode
        type: text
        required: true
        unique: true
        constraints: AWS のリージョンコード（例 ap-northeast-1）
      - name: awsLimitPerSecond
        type: integer
        required: true
        allowed_values: [10, 25, 30]
        constraints: AWS の文書の既定の上限
      - name: ceilingPerSecond
        type: integer
        required: true
        constraints: awsLimitPerSecond の半分（小数は切り捨て）。25 → 12、10 → 5、30 → 15
    constraints:
      - 既定 25 回のリージョンは表に 1 行ずつ持たず「その他の公開リージョン」として扱ってよい（実装の判断）。ただし 10 回・30 回のリージョンは必ず個別に持つ
      - 表にないリージョンとリージョンの指定が無いときの上限は 5（FR2.2）
    relationships: []

  - name: PacingState
    owner: CallPacer
    description: 取得 1 回で全ストリームが共有する速さの状態
    attributes:
      - name: ceilingPerSecond
        type: integer
        required: true
        constraints: RegionCallLimit から決まる。U1 では取得の間ずっと変わらない（引き下げと戻しは U2）
      - name: minInterval
        type: duration
        required: true
        constraints: 1,000,000,000 ÷ ceilingPerSecond ナノ秒を切り上げた値（東京なら 83,333,334 ナノ秒）
      - name: nextPermitAt
        type: instant
        required: true
        constraints: 戻らない時計の時刻。次の許可を出してよい最も早い時刻。最初は取得の開始時刻（最初の許可はすぐ出る）
    constraints:
      - 許可は 1 つずつ出し、まとめて出さない（ためておく分はない）
      - nextPermitAt の読み書きはロックの中で行い、枠を予約してから待つ（BR1.3）
    relationships:
      - target: FetchJob
        cardinality: one-to-one
        direction: PacingState → FetchJob（fetchJobId の値だけの参照）

  - name: StreamSlot
    owner: StreamSchedule
    description: 選ばれたストリーム 1 本の読み取りの状態
    attributes:
      - name: logStreamName
        type: text
        required: true
        unique: true
      - name: order
        type: integer
        required: true
        constraints: 選ばれた順（最終イベントの新しい順）。0 から
      - name: state
        type: enum
        required: true
        allowed_values: [Unstarted, InFlight, Succeeded, Failed]
        default: Unstarted
      - name: end
        type: enum
        required: false
        allowed_values: [Completed, FailedExhausted, FailedNotExhausted, Aborted, Skipped]
        constraints: state が Succeeded か Failed になったときに入る。Skipped は呼ばずに失敗とした未着手（あきらめ）。Aborted は止める信号で止めたもの（U1 では閉じるときだけ）
    constraints:
      - InFlight の数は同時に読む本数の上限（8）を超えない
    relationships:
      - target: PlannedStream
        cardinality: one-to-one
        direction: StreamSlot → PlannedStream（logStreamName の値だけの参照）

  - name: ScheduleOutcome
    owner: StreamSchedule
    description: ストリームを読む段の終わり方
    attributes:
      - name: endState
        type: enum
        required: true
        allowed_values: [Completed, CompletedWithFailures]
        constraints: U1 では 2 つだけ（Cancelled は U5 で足す）。これは FetchCoordinator が終わりの状態を決めるための材料で、取得 1 回の終わりの状態の正本は FetchJob.status（BR1.15）。Aborted（閉じる）は FetchCoordinator が決める
      - name: exhaustionStreak
        type: integer
        required: true
        min: 0
        constraints: 終わった順に数えた、続けて使い切りで失敗した本数
      - name: unfinishedStreams
        type: integer
        required: true
        min: 0
        constraints: 読み途中＋未着手。U1 ではすべて終わったときに 0（中断の本数は U5 で使う）
    relationships:
      - target: FetchJob
        cardinality: one-to-one
        direction: ScheduleOutcome → FetchJob（fetchJobId の値だけの参照）

  - name: PageStep
    owner: EventFetcher
    description: 1 本のストリームの「次の 1 ページ」の読み取りの結果。FetchCoordinator が受け取る（別の通り道は持たない）
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [Page, Finished]
      - name: logStreamName
        type: text
        required: true
      - name: events
        type: list
        required: false
        constraints: kind が Page のとき。ストリーム内の順番を振った 1 ページ分のイベント
      - name: fetch
        type: record
        required: false
        constraints: kind が Finished のとき。今の StreamFetch（件数・状態・失敗と終わり方）
    constraints:
      - 1 本のストリームは、前の PageStep を渡し終えてから次の 1 ページを読む。そのため Page は必ず Finished より先で、渡し待ちは読み途中の本数（最大 8）を超えない
    relationships: []

  - name: StopReason
    owner: RetryPolicy
    description: 止める信号の理由（約束 C7）
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Cancel, Close]
        constraints: U1 では Close（ウィンドウを閉じる）だけを送る。Cancel を送る道は U5 で足す。Close は常に Cancel より強い
    relationships: []

  - name: JobStatus
    owner: FetchStatus
    description: 取得 1 回の終わりの状態（今の coordinator.rs から FetchStatus に移す）
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Running, Completed, CompletedWithFailures, Failed, Aborted]
        constraints: 値は今のまま。Cancelled は U5 で足す
    relationships: []
```

## まとめ

| エンティティ | 持ち主 | U1 での役割 |
|--------------|--------|-------------|
| RegionCallLimit | CallPacer | リージョンごとの上限と、その半分のアプリの上限 |
| PacingState | CallPacer | 取得 1 回で共有する速さ（次の許可を出してよい時刻） |
| StreamSlot | StreamSchedule | ストリームごとの状態（未着手・読み途中・成功・失敗）と終わり方 |
| ScheduleOutcome | StreamSchedule | ストリームを読む段の終わり方と、使い切りの連続の数 |
| PageStep | EventFetcher | 1 本のストリームの次の 1 ページの読み取り結果（ページか終わり） |
| StopReason | RetryPolicy | 止める理由（U1 では Close だけ） |
| JobStatus | FetchStatus | 取得 1 回の終わりの状態（値は今のまま、置き場所だけ移す） |

## Assumptions & Open Questions

None.
