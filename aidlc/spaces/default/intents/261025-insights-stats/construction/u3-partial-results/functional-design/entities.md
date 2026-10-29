# Entities — U3 途中結果と中断後の結果（u3-partial-results）

上流：`inception/units-generation/unit-of-work.md`（U3、U2 との分担の表、レビューの持ち越し）・`unit-of-work-story-map.md`（FR4・FR5）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.10、FR5.1〜FR5.4、NFR1・NFR2・NFR10、A3・A4）、`inception/domain-design/components.md`（QueryStore・QueryRunner・InsightsSession・TauriSink・TauriApp・ApiBridge・QueryResultTable・LogAnalyticsPage）・`decisions.md`（ADR-003・ADR-004・ADR-006）とそのレビュー（DD R-02・R-03・R-04・R-08）、U1 の機能設計（形つきの QueryResult、ResultRow、BR3.5）、質問票 `functional-design-questions.md`（Q1〜Q4）、今のコード（`runner.rs` の fetch_range と範囲ごとの holder、`store.rs` の GroupHoldings・committed / staging・epoch・view・ViewEntry・StreamSequences、`insights_session.rs` の InsightsRun・AnalyticsPhase・InsightsProgress・ResultSummary、`src-tauri/src/lib.rs` の TauriQuerySink）。

## 用語（DD R-08）

- **確定済みの分**：`settle_range` を通った範囲のイベント。committed（取得の 5 分前より古い分）と staging（直近 5 分の分）の両方を含む。最終結果はこの分だけから作る。
- **取得中の分**：取得が終わっていない範囲の、範囲ごとの保持（holder）にあるイベント。確定していないため、取得済みの範囲の記録に入らない。
- **途中の view**：確定済みの分と取得中の分の写しを合わせ、結果の並びの順に並べた一覧。途中結果はこの view から作る。

## エンティティ（機械可読）

```yaml
entities:
  - name: InflightRegistry
    description: 取得中の範囲ごとの保持の登録先（新規）。QueryStore が持ち、QueryStore の鍵の中で登録・取り出し・確定・外しを行う（DD R-02、FR レビュー R-01）
    attributes:
      - name: runId
        type: integer
        required: true
      - name: holders
        type: list<InflightHolder>
        required: true
        constraints: 取得を始めた範囲を足し、settle_inflight（取り出し・確定・外しを 1 回の鍵で）で外す（BR1.2）
    relationships:
      - target: InflightHolder
        cardinality: 1 対 0..n
        direction: InflightRegistry が持つ

  - name: InflightHolder
    description: 取得中の 1 つの範囲の保持（今の fetch_range のローカルの holder を、登録先にも見える形にしたもの）
    attributes:
      - name: holdingKey
        type: HoldingKey
        required: true
        constraints: どのロググループの保持か
      - name: range
        type: TimeRange
        required: true
      - name: timeline
        type: shared<EventTimeline>
        required: true
        constraints: 今の Arc<Mutex<EventTimeline>>。取得の流れが足し、途中の view を作るときに短く鍵を取って写す（BR1.3）

  - name: PartialView
    description: 途中の view（QueryStore が作る）
    attributes:
      - name: epoch
        type: integer
        required: true
        constraints: 今の QueryStore の世代（接続の変更・begin_run で進む）
      - name: viewVersion
        type: integer
        required: true
        constraints: view を作るとき、および保持が変わるとき（settle_inflight・add_cached・begin_run・discard_all）に進む番号。evaluate_step と copy_rows は (epoch, viewVersion) で食い違いを確かめる（DD R-03、BR1.4、FR レビュー R-02）
      - name: entries
        type: list<ViewEntry>
        required: true
        constraints: ViewEntry の種類は Committed・Staging（今のもの）と Inflight（inflightCopies の位置、新規）。結果の並びの順（時刻, ロググループ, ストリーム, ストリーム内の順番）に並べたもの（BR1.3）
      - name: inflightCopies
        type: list<LogEvent>
        required: true
        constraints: 取得中の分の写し。この view の間だけ持ち、次の view を作るときに捨てる。ViewEntry は位置でここを指す（DD R-02）

  - name: ResultVersion
    description: 1 回の実行の中の結果の版（InsightsSession が持つ）
    attributes:
      - name: runId
        type: integer
        required: true
      - name: version
        type: integer
        required: true
        min: 1
        constraints: 途中結果と最終結果で 1 つずつ進む。古い版は捨てる（A4、BR2.4）
      - name: stage
        type: enum
        required: true
        allowed_values: [partial, final, cancelledPartial]
        constraints: partial は途中、final は取得がすべて終わった最終、cancelledPartial は中断後の最終（BR3.1）
      - name: coverage
        type: PartialCoverage
        required: false
        constraints: partial のとき（BR2.6）

  - name: PartialCoverage
    description: 途中結果がどこまでを含むか（印の文字の材料、Q2 A）
    attributes:
      - name: groupNumber
        type: integer
        required: true
        constraints: 取得中のロググループの番号（1 から）。今の InsightsProgress.group_number
      - name: groupCount
        type: integer
        required: true
      - name: includedEventCount
        type: integer
        required: true
        constraints: その途中結果の評価に含めたイベントの数（途中の view の長さ）

  - name: InsightsRun
    description: クエリの画面の 1 回の実行（今の InsightsRun を広げる）
    attributes:
      - name: phase
        type: enum
        required: true
        allowed_values: [Idle, Fetching, Evaluating, Done, QueryError, FetchFailed, Cancelling, Cancelled, EvaluationFailed]
        constraints: Cancelling を足す（取得を止めてから中断後の評価が終わるまで。実行中として扱う、BR3.1・BR3.2・BR3.7）
      - name: abandonRequested
        type: boolean
        required: true
        default: false
        constraints: Cancelling の間の 2 回目の［Cancel］（BR3.2）
      - name: result
        type: QueryResult
        required: false
        constraints: 最新の版の結果。前回の結果は previous の印つきで、最初の途中結果が届くまで残る（BR2.5）
      - name: resultVersion
        type: ResultVersion
        required: false
      - name: previous
        type: boolean
        required: true
        default: false
        constraints: result が前回の実行のものか（今の「前回の結果」の印）
      - name: failedStreams
        type: list<FailedStream>
        required: true
        constraints: 今のまま。中断と失敗の両方を示すときに使う（FR5.4）

  - name: RowIdentity
    description: 差し替えのあとも同じ行を見分ける鍵（画面の部品が持つ、Q4 A）
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [event, aggregate]
      - name: eventKey
        type: string
        required: false
        constraints: event のとき。（ロググループ名, ストリーム名, LogEvent の sequence）をつないだ文字。QueryRow に rowKey として載せる（BR4.2）
      - name: aggregateKey
        type: string
        required: false
        constraints: aggregate のとき。鍵の列のセルの文字（U1 の鍵の比べ方）をつないだ文字。by なしの 1 行は空の文字
```

## エンティティの要約

- **取得中の分**：QueryRunner が実行の間だけ InflightRegistry を持ち、範囲を取得し始めたら InflightHolder（今の範囲ごとの保持）を登録し、確定したら外す。
- **途中の view**：QueryStore が確定済みの分と取得中の分の写しを合わせて作る。epoch とは別の viewVersion を持ち、評価と写しは (epoch, viewVersion) で食い違いを見分ける。
- **結果の版**：InsightsSession が実行ごとに版の番号を進め、partial / final / cancelledPartial と、途中のときの含めた範囲（ロググループの番号と含めた件数）を持つ。古い版と別の実行の答えは捨てる。
- **段階**：取得を止めてから中断後の評価が終わるまでを表す Cancelling を足し、実行中として扱う。2 回目の［Cancel］の印 abandonRequested を持つ。
- **行の見分け**：画面が開いた行と先頭の行を、イベントの行は（ロググループ, ストリーム, 順番）、集計の行は鍵の文字で覚える。
