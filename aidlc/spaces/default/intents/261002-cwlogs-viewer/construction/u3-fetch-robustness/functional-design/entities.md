# Entities — U3 取得の作り込み（u3-fetch-robustness）

上流の成果物：`inception/domain-design/components.md`（StreamPlanner の LogStream、EventFetcher の StreamFetchOutcome、EventTimeline の LogEvent、FetchCoordinator の FetchJob）、`inception/units-generation/unit-of-work.md`（U3 の範囲）、`inception/requirements-analysis/requirements.md`（FR4.1、FR4.2、FR4.5、FR4.7、FR4.8、FR4.10、FR4.11、NFR2〜NFR4）。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U3 で新しく扱うエンティティと、U1・U2 のエンティティに足す・変える属性だけを、論理的な型と制約のレベルで定める。U1・U2 のルールを指すときは `U1:BRx.y`・`U2:BRx.y` と書く。前に何も付けない `BRx.y` はこの単位（U3）の rules.md のルール。

## エンティティ（機械可読）

```yaml
entities:
  - name: LogStream
    owner: StreamPlanner
    components_md: LogStream（同名）
    description: ロググループの中の 1 つのストリーム。DescribeLogStreams の結果から作る
    identifier: [logStreamName]
    attributes:
      - name: logStreamName
        type: text
        required: true
        unique: true
      - name: firstEventTimestamp
        type: instant-millis
        required: false
        constraints: API が返さないことがある
      - name: lastEventTimestamp
        type: instant-millis
        required: false
        constraints: API が返さないことがある。即時には更新されない（FR4.2）
    constraints: []
    relationships: []

  - name: StreamPlan
    owner: StreamPlanner
    components_md: components.md にはないが、StreamPlanner の責任「時間範囲によるストリームの選定」の結果を表す補助
    description: 1 回の取得で GetLogEvents を呼ぶストリームの一覧と、列挙の結果
    attributes:
      - name: selectedStreams
        type: list
        required: true
        constraints: LogStream の一覧。列挙で得た順（最後のイベント時刻の新しい順）のまま（BR1.1、BR1.3）
      - name: listingStatus
        type: enum
        allowed_values: [Complete, StoppedEarly, Partial]
        required: true
        constraints: StoppedEarly は打ち切りの条件（BR1.2）で列挙をやめた状態。Partial は列挙の途中でエラーになり、それまでの分だけを持つ状態（BR1.5）
      - name: listingFailure
        type: reference
        references: ApiFailure
        required: false
        constraints: listingStatus が Partial のときだけ持つ
    constraints: []
    relationships:
      - target: LogStream
        cardinality: 0..*
        direction: StreamPlan が持つ

  - name: RetryPolicy
    owner: EventFetcher
    components_md: components.md にはないが、EventFetcher・StreamPlanner の責任「スロットリング時の待機と再試行」の決まりを表す補助（StreamPlanner も同じ決まりを使う）
    description: 再試行してよいエラーと、待ち時間・回数の決まり（Q1）
    attributes:
      - name: retryableKinds
        type: list
        required: true
        default: [Throttled, Network]
      - name: baseDelaysSeconds
        type: list
        required: true
        default: [1, 2, 4, 8, 16]
        constraints: 1 回目の再試行の前に 1 秒、2 回目の前に 2 秒…と倍々にする
      - name: jitterRatio
        type: decimal
        required: true
        default: 0.2
        min: 0
        max: 1
        constraints: 各待ち時間を ±20% の範囲でばらつかせる
      - name: maxRetries
        type: integer
        required: true
        default: 5
        min: 0
        constraints: 最初の呼び出しを含めて最大 6 回呼ぶ
      - name: maxConsecutiveExhausted
        type: integer
        required: true
        default: 3
        min: 1
        constraints: 再試行を使い切って失敗したストリームがこの数だけ続いたら、残りのストリームは呼ばずに失敗とする（BR2.1、レビュー R-03）
    constraints: []
    relationships: []

  - name: StreamFetchOutcome
    owner: EventFetcher
    components_md: StreamFetchOutcome（同名）。U1 の属性に再試行の情報を足す
    description: 1 ストリームの取得の結果
    identifier: [logStreamName]
    attributes:
      - name: retryCount
        type: integer
        required: true
        min: 0
        constraints: このストリームで行った再試行の回数の合計（ページをまたいで数える）
    constraints:
      - U1 の属性（logStreamName・eventCount・pageCount・status・failure）はそのまま
    relationships: []

  - name: LogEvent
    owner: EventTimeline
    components_md: LogEvent（同名）。U1 の属性の意味を変える
    description: 取得した 1 件のログ
    identifier: [logStreamName, sequence]
    attributes:
      - name: sequence
        type: integer
        required: true
        min: 0
        constraints: ストリームごとに、API が返した順に 0 から振る通し番号（U1 では 1 回の取得の中の通し番号だった）
    constraints:
      - 並び順は timestamp の昇順、同じ timestamp は logStreamName の昇順、同じストリームの中では sequence の昇順（BR4.1、Q3）
      - (logStreamName, sequence) で 1 件が決まり、取得し直すまで変わらない（BR4.4）
    relationships: []

  - name: RowWindow
    owner: EventTimeline
    components_md: components.md にはないが、EventTimeline の責任「表示範囲の行の取り出し」の結果を表す補助
    description: 画面が表示範囲だけを描くために取り寄せる行のまとまり
    attributes:
      - name: offset
        type: integer
        required: true
        min: 0
      - name: rows
        type: list
        required: true
        constraints: offset から最大 limit 件の LogEvent（時刻・ストリーム名・メッセージ・(logStreamName, sequence)）。保持件数を超える分は返さない
      - name: totalCount
        type: integer
        required: true
        min: 0
        constraints: 取り出した時点の保持件数
      - name: timelineVersion
        type: integer
        required: true
        min: 0
        constraints: 保持ログに追加・破棄があるたびに増える番号。画面が位置を保つために使う（BR6.5）
    constraints: []
    relationships: []

  - name: FailedStream
    owner: FetchCoordinator
    components_md: components.md にはないが、FetchCoordinator の責任「部分失敗のまとめ」の 1 件を表す補助
    description: 取得に失敗した 1 つのストリームと理由（Q5）
    identifier: [logStreamName]
    attributes:
      - name: logStreamName
        type: text
        required: true
      - name: failure
        type: reference
        references: ApiFailure
        required: true
        constraints: U1:BR4.3 の安全な詳細だけを持つ
    constraints: []
    relationships: []

  - name: FetchJob
    owner: FetchCoordinator
    components_md: FetchJob（同名）。U1 の最小版を複数ストリームに広げる
    description: 1 回の取得の状態と結果
    identifier: [jobId]
    attributes:
      - name: jobId
        type: integer
        required: true
        unique: true
        min: 0
        constraints: アプリの実行の中で取得を始めるたびに増える番号
      - name: status
        type: enum
        allowed_values: [Running, Completed, CompletedWithFailures, Failed, Aborted]
        required: true
        constraints: U1 の Running・Completed・Failed に、CompletedWithFailures と Aborted を足す（BR5.2）
      - name: plannedStreamCount
        type: integer
        required: true
        min: 0
      - name: finishedStreamCount
        type: integer
        required: true
        min: 0
        constraints: 成功・失敗を問わず取得を終えたストリームの数
      - name: failedStreams
        type: list
        required: true
        constraints: FailedStream の一覧。U1 の failedStreamCount はこの件数になる
      - name: listingFailure
        type: reference
        references: ApiFailure
        required: false
        constraints: ストリームの列挙が途中でエラーになったときだけ持つ（BR1.5）。正は StreamPlan.listingFailure で、これはその写し（StreamPlan がないときも結果として渡せるように FetchJob にも持つ。値は常に同じ）
      - name: eventCount
        type: integer
        required: true
        min: 0
    constraints:
      - finishedStreamCount <= plannedStreamCount
    relationships:
      - target: StreamPlan
        cardinality: 0..1
        direction: FetchJob が参照する（列挙を終える前に中断したときは持たない）
      - target: FailedStream
        cardinality: 0..*
        direction: FetchJob が持つ
      - target: StreamFetchOutcome
        cardinality: 0..*
        direction: FetchJob がストリームごとに集める

  - name: FetchRequest
    owner: AppSession
    components_md: U1・U2 の補助 FetchRequest から属性を外す
    description: 1 回分の取得条件
    attributes: []
    constraints:
      - U1 の logStreamName をなくす（ロググループ全体を取得する）。そのほかの属性は U2 のまま
    relationships: []

  - name: SessionState
    owner: AppSession
    components_md: SessionState（同名）。U2 の属性に足す
    description: 画面の状態
    attributes:
      - name: progress
        type: reference
        references: FetchJob
        required: false
        constraints: 取得中と取得後の、計画したストリーム数・終えたストリーム数・件数・失敗の数
      - name: failureListOpen
        type: boolean
        required: true
        default: false
        constraints: 失敗したストリームの一覧を開いているか（Q5）
      - name: connectionGeneration
        type: integer
        required: true
        default: 0
        min: 0
        constraints: 接続先の変更を確定するたびに 1 増える。古い世代の操作の結果は反映しない（BR6.7、レビュー R-02）
    constraints: []
    relationships:
      - target: FetchJob
        cardinality: 0..1
        direction: SessionState が進み具合として表示する

  - name: ViewportAnchor
    owner: DesktopUi
    components_md: components.md にはないが、DesktopUi の責任「ログ一覧の表示」で見ている位置を保つための補助（Q4）
    description: 画面の一番上に見えている行と、その行の中のずれ
    attributes:
      - name: anchorEventKey
        type: text
        required: false
        constraints: 一番上に見えている行の (logStreamName, sequence)。一番上までスクロールしているときは持たない
      - name: pixelOffset
        type: integer
        required: true
        default: 0
        min: 0
    constraints: []
    relationships: []
```

## まとめ

| エンティティ | 持ち主 | components.md との対応 | 備考 |
|--------------|--------|------------------------|------|
| LogStream | StreamPlanner | 同名 | 最初・最後のイベント時刻はないことがある |
| StreamPlan | StreamPlanner | 選定の結果の補助 | 打ち切り（Q2）と、列挙の途中のエラー |
| RetryPolicy | EventFetcher | 再試行の決まりの補助 | 1・2・4・8・16 秒、±20%、最大 5 回（Q1）。使い切りが 3 ストリーム続いたら残りは失敗 |
| StreamFetchOutcome | EventFetcher | 同名（属性を足す） | 再試行の回数 |
| LogEvent | EventTimeline | 同名（意味を変える） | sequence はストリームごと。並びは時刻 → ストリーム名 → sequence（Q3） |
| RowWindow | EventTimeline | 表示範囲の行の補助 | 100 万件を表示範囲だけ描く |
| FailedStream | FetchCoordinator | 部分失敗の 1 件の補助 | 名前と安全な詳細（Q5） |
| FetchJob | FetchCoordinator | 同名（広げる） | CompletedWithFailures・Aborted を足す。jobId。StreamPlan は 0..1 |
| FetchRequest | AppSession | U1・U2 の補助（属性を外す） | ストリーム名をなくす |
| SessionState | AppSession | 同名（属性を足す） | 進み具合、失敗の一覧、接続先の世代番号 |
| ViewportAnchor | DesktopUi | 位置を保つ補助 | Q4 |
