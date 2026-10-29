# Entities — U1 薄い一本（u1-stats-skeleton）

上流：`inception/units-generation/unit-of-work.md`（U1）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1.1〜FR1.3・FR1.5・FR1.6・FR1.8、FR2.1〜FR2.3、FR3.1〜FR3.6、FR6.3）、`inception/domain-design/components.md`（QueryPlan・QueryParser・StatsAggregator・QueryEvaluator・InsightsSession・ExportJob）・`decisions.md`（ADR-001 形つきの QueryResult、ADR-002 StatsAggregator）とそのレビュー（DD R-01 行は写し、DD R-05 時刻のセルは表示時に整形）、質問票 `functional-design-questions.md`（Q1〜Q6）、今のコード（`plan.rs` の QueryPlan・CommandKind・FieldRef、`evaluator.rs` の QueryResult・ResultRow・EvalOutput、`fields.rs` の FieldValue、`insights_session.rs` の ResultSummary・QueryRow・QueryRowDetail、`exporter.rs` の ExportSnapshotSource）。

型の名前は設計の上の名前で、作る段階で言語の慣用に合わせてよい。LogEvent・EventTimeline・Condition・SortKey・QueryProblem は今のものをそのまま使い、ここでは形を変えない。U1 で変えるのは、計画に `stats` の命令が増えること、結果が「形」を持つこと、行が 2 種類になること、画面と書き出しに渡す値が形に従うことの 4 つである。

## エンティティ（機械可読）

```yaml
entities:
  - name: QueryPlan
    description: 読み取ったクエリ（今の QueryPlan を広げる）。命令の並びに Stats が加わる
    attributes:
      - name: commands
        type: list<Command>
        required: true
        min: 1
        constraints: 文に現れた順。Stats は 1 つまでで最後の命令（BR1.6。U2 で sort・limit の後置を許す）
    relationships:
      - target: StatsCommand
        cardinality: 1 対 0..1
        direction: QueryPlan が持つ（CommandKind::Stats の中身）

  - name: StatsCommand
    description: stats 命令の中身。集計の指定の並びと、by の鍵の並び
    attributes:
      - name: aggregates
        type: list<AggregateSpec>
        required: true
        min: 1
        constraints: 書いた順。同じ列の名前が 2 つあるときは読み取りのエラー（BR1.4）
      - name: groupKeys
        type: list<GroupKey>
        required: true
        min: 0
        constraints: 書いた順。by を書かないときは空（全体を 1 行に集計、BR2.6）
    relationships:
      - target: AggregateSpec
        cardinality: 1..n
        direction: StatsCommand が持つ
      - target: GroupKey
        cardinality: 0..n
        direction: StatsCommand が持つ

  - name: AggregateSpec
    description: 1 つの集計の指定（関数・引き数・列の名前）
    attributes:
      - name: function
        type: enum
        required: true
        allowed_values: [count, sum, avg, min, max]
        constraints: U1 の 6 つの書き方（count(*)・count(項目)・sum・avg・min・max、UG Q2 A）。count_distinct・pct・stddev・earliest・latest・sortsFirst・sortsLast は U2（U1 では Unsupported の問題、BR1.2）
      - name: argument
        type: AggregateArgument
        required: true
        constraints: count は * か項目、sum・avg・min・max は項目（BR1.3）
      - name: columnName
        type: string
        required: true
        constraints: as があればその名前、なければ書いたとおりの文字（例 `count(*)`、`avg(latency)`、`sum(toMillis(@timestamp))`、BR1.4）

  - name: AggregateArgument
    description: 集計の引き数。* か、項目の参照
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [all, field]
        constraints: all（`*`）は count だけ（BR1.3）
      - name: field
        type: FieldRef
        required: false
        constraints: kind が field のとき。決まった項目（@timestamp・@message・@logStream・@log）、toMillis(@timestamp)、JSON の項目（Q6 A）

  - name: GroupKey
    description: by の 1 つの鍵。項目か bin(期間)
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [field, bin]
      - name: field
        type: FieldRef
        required: false
        constraints: kind が field のとき。決まった項目か JSON の項目（toMillis(@timestamp) は鍵には書けない、Q6 A）
      - name: period
        type: BinPeriod
        required: false
        constraints: kind が bin のとき
      - name: columnName
        type: string
        required: true
        constraints: as があればその名前、なければ書いたとおりの文字（例 `level`、`bin(5m)`、BR1.4）

  - name: BinPeriod
    description: bin() の期間
    attributes:
      - name: count
        type: integer
        required: true
        min: 1
        constraints: 0 は読み取りのエラー（BR1.5）
      - name: unit
        type: enum
        required: true
        allowed_values: [s, m, h, d]
        constraints: ms・w・mo・q・y は Unsupported の問題（Q1 A、BR1.5）
      - name: millis
        type: integer
        required: true
        constraints: count × 単位のミリ秒。区切りはエポックの倍数（BR2.5）

  - name: AggregateGroup
    description: 集計の途中の 1 つのグループ（StatsAggregator の中。結果には写しだけが残る）
    attributes:
      - name: keyValues
        type: list<CellValue>
        required: true
        constraints: groupKeys と同じ数・同じ順。鍵の項目を持たないイベントは Missing（Q3 A、BR2.1）。bin の鍵は区切りの始まりのミリ秒（Timestamp）
      - name: accumulators
        type: list<Accumulator>
        required: true
        constraints: aggregates と同じ数・同じ順
    relationships:
      - target: Accumulator
        cardinality: 1..n
        direction: AggregateGroup が持つ

  - name: Accumulator
    description: 1 つの集計の累積。関数ごとに持つものが違う
    attributes:
      - name: count
        type: integer
        required: true
        constraints: count(*) は見たイベントの数、count(項目) は項目を持つイベントの数、sum・avg は足した数の個数（BR2.2・BR2.3）
      - name: sum
        type: decimal
        required: false
        constraints: sum・avg。数に読めた値の合計（BR2.3）
      - name: extreme
        type: CellValue
        required: false
        constraints: min・max。今までの最小・最大（数と文字列の比べ方は BR2.4）

  - name: CellValue
    description: 集計の行の 1 つのセルの値（型つき。表示の文字は InsightsSession が作る、DD R-05）
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [missing, integer, decimal, text, timestamp]
      - name: value
        type: integer | decimal | string
        required: false
        constraints: missing は値なし（鍵なしのグループの鍵、足せる値のなかった sum・avg・min・max）。timestamp はエポックのミリ秒（bin の鍵、@timestamp の min・max・toMillis の集計は integer）。表示の文字は BR2.8・BR4.2

  - name: QueryResult
    description: 評価の結果（今の QueryResult を広げる）。形を持ち、行は 2 種類の写し
    attributes:
      - name: runId
        type: integer
        required: true
        constraints: 作った実行。画面はこれだけで結果を見分ける（今と同じ）
      - name: shape
        type: enum
        required: true
        allowed_values: [events, aggregates]
        constraints: 計画に stats があれば aggregates、なければ events（BR3.1）
      - name: columns
        type: list<string>
        required: true
        constraints: 列の名前。events は今の FieldRef の名前（空の列を落とす今の決まりのまま）、aggregates は鍵の列の名前 then 集計の列の名前（BR3.3）
      - name: rows
        type: list<ResultRow>
        required: true
        constraints: すべて写し。保持（QueryStore）を位置で指さない（DD R-01）。events の行は event と logGroupName、aggregates の行は columns と同じ数の CellValue
      - name: matchedCount
        type: integer
        required: true
        constraints: events は今と同じ。aggregates はグループの数（BR3.4）
      - name: evaluatedCount
        type: integer
        required: true
      - name: truncatedAt
        type: integer
        required: false
        constraints: aggregates では U1 は常になし（上限は U2、FR1.7）
    relationships:
      - target: ResultRow
        cardinality: 1 対 0..n
        direction: QueryResult が持つ

  - name: ResultRow
    description: 結果の 1 行。イベントの行か集計の行
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [event, aggregate]
      - name: event
        type: LogEvent
        required: false
        constraints: event のとき（今の ResultRow と同じ写し）
      - name: logGroupName
        type: string
        required: false
        constraints: event のとき
      - name: cells
        type: list<CellValue>
        required: false
        constraints: aggregate のとき。QueryResult.columns と同じ数・同じ順

  - name: ResultSummary
    description: 画面に渡す結果の見出し（今の ResultSummary を広げる）
    attributes:
      - name: runId
        type: integer
        required: true
      - name: shape
        type: enum
        required: true
        allowed_values: [events, aggregates]
        constraints: QueryResult.shape（BR4.1）
      - name: columns
        type: list<string>
        required: true
      - name: rowCount
        type: integer
        required: true
      - name: evaluatedCount
        type: integer
        required: true
      - name: matchedCount
        type: integer
        required: true
      - name: truncatedAt
        type: integer
        required: false

  - name: QueryRow
    description: 画面に渡す 1 行（今の QueryRow を広げる）
    attributes:
      - name: values
        type: list<string | null>
        required: true
        constraints: 列ごとのセルの文字。events は今の row_cells、aggregates は CellValue を BR2.8・BR4.2 で文字にしたもの。null は値なし
      - name: event
        type: LogEvent
        required: false
        constraints: events のときだけ（今の平たく含めた形のまま）。aggregates では持たない（BR4.1）
      - name: displayTime
        type: string
        required: false
        constraints: events のときだけ
      - name: logGroupName
        type: string
        required: false
        constraints: events のときだけ

  - name: QueryRowDetail
    description: 開いた行の項目と値（今の QueryRowDetail のまま。中身の作り方だけ形で分かれる）
    attributes:
      - name: runId
        type: integer
        required: true
      - name: index
        type: integer
        required: true
      - name: fields
        type: list<DetailField>
        required: true
        constraints: events は今のとおり（決まった 4 項目 then JSON の項目）。aggregates は columns の順に（列の名前, セルの文字）（BR4.3）

  - name: ExportSnapshotSource.LogAnalytics
    description: 書き出しの元（今の LogAnalytics を広げる）
    attributes:
      - name: columns
        type: list<string>
        required: true
        constraints: 列の名前（今は FieldRef の並び。名前の文字列に変える）
      - name: rows
        type: list<ResultRow>
        required: true
        constraints: 形つきの行の写し
      - name: zone
        type: TimeZone
        required: true
        constraints: 時刻のセルと @timestamp の整形に使う（BR4.4）
```

## エンティティの要約

- **計画**：QueryPlan の命令に `Stats` が増える。中身は AggregateSpec（関数・引き数・列の名前）の並びと GroupKey（項目か bin(期間)・列の名前）の並び。U1 では `stats` は最後の命令で 1 つだけ。
- **集計の途中**：AggregateGroup は鍵の値の組ごとに 1 つ。鍵のないイベントは Missing の鍵で 1 つのグループにまとまる（Q3 A）。Accumulator は関数ごとに数・合計・最小最大を持つ。結果には写しだけが残り、途中の状態は評価が終われば捨てる。
- **結果**：QueryResult が `shape`（events / aggregates）と列の名前の一覧を持ち、行は ResultRow の列挙型（イベントの写し／型つきのセルの列）。どちらの行も写しで、保持を指さない（DD R-01）。
- **画面と書き出し**：ResultSummary に `shape` が増え、QueryRow は `values` を必ず持ち、イベントの項目は events のときだけ。書き出しの元は列の名前と形つきの行。時刻のセルの文字は、表示と書き出しのときに画面のタイムゾーンで作る（DD R-05）。
