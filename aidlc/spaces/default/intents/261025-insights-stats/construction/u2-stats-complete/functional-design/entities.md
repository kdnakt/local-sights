# Entities — U2 `stats` をそろえる（u2-stats-complete）

上流：`inception/units-generation/unit-of-work.md`（U2、U3 との分担の表）・`unit-of-work-story-map.md`（FR1.2・FR1.4・FR1.6・FR1.7・FR6.1・FR6.2）、`inception/requirements-analysis/requirements.md`、`inception/domain-design/components.md`（StatsAggregator・QueryParser・QueryEvaluator・QueryCatalog・InsightsSession・AnalyticsHelpers・QueryEditor・MessageCatalog）・`decisions.md`（ADR-002・ADR-005）、U1 の機能設計（`construction/u1-stats-skeleton/functional-design/` の StatsCommand・AggregateSpec・Accumulator・CellValue・ResultColumns、BR2.1〜BR2.9、BR3.5）、質問票 `functional-design-questions.md`（Q1〜Q5）、AWS の資料「CloudWatch Logs Insights query syntax」の `stats` の関数の表。

U1 の型をそのまま広げる。ここに書くのは U2 で増える・変わる部分だけで、書かない属性は U1 の entities.md のとおり。

## エンティティ（機械可読）

```yaml
entities:
  - name: AggregateSpec
    description: 1 つの集計の指定（U1 を広げる）。関数が 13 種類になり、pct は百分位を持つ
    attributes:
      - name: function
        type: enum
        required: true
        allowed_values: [count, count_distinct, sum, avg, min, max, pct, stddev, earliest, latest, sortsFirst, sortsLast]
        constraints: 名前は資料の書き方（count_distinct・sortsFirst・sortsLast は大文字・小文字をそのまま）。U1 の 6 つの書き方を含む（BR1.1）
      - name: percentile
        type: decimal
        required: false
        min: 0
        max: 100
        constraints: pct のときだけ。第 2 引き数の数（BR1.2）
      - name: argument
        type: AggregateArgument
        required: true
        constraints: count だけが `*` を受ける。ほかは項目（U1 の BR1.3 と同じ項目）

  - name: Accumulator
    description: 1 つの集計の累積（U1 を広げる）。関数ごとに持つものが違う
    attributes:
      - name: count
        type: integer
        required: true
        constraints: U1 のとおり。stddev は足した数の個数
      - name: sum
        type: decimal
        required: false
        constraints: sum・avg
      - name: mean
        type: decimal
        required: false
        constraints: stddev。Welford の逐次の平均（BR2.3）
      - name: m2
        type: decimal
        required: false
        constraints: stddev。平均からの差の 2 乗の和（BR2.3）
      - name: values
        type: list<decimal>
        required: false
        constraints: pct。数に読めた値をすべて持つ（並べるのは finish のとき、BR2.2）
      - name: distinct
        type: set<string>
        required: false
        constraints: count_distinct。値の文字（U1 の鍵と同じ比べ方）の集まり（BR2.1）
      - name: extreme
        type: CellValue
        required: false
        constraints: min・max・sortsFirst・sortsLast
      - name: chosen
        type: CellValue
        required: false
        constraints: earliest・latest。今までで最も早い（遅い）イベントの値
      - name: chosenSeen
        type: boolean
        required: false
        constraints: earliest・latest。値を 1 つでも見たか。earliest は最初に見た値を保ち、latest は見るたびに置き換える（view は結果の並びの順のため、BR2.5）

  - name: StatsCommand
    description: stats 命令（U1 を広げる）。あとに続く命令を持つ
    attributes:
      - name: after
        type: list<AfterStatsCommand>
        required: true
        min: 0
        constraints: stats のあとに書いた filter・sort・limit を書いた順に（BR1.4。fields と 2 つ目の stats は書けない、BR1.5）

  - name: AfterStatsCommand
    description: stats のあとの 1 つの命令。集計の結果の列に対して働く
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [filter, sort, limit]
      - name: condition
        type: Condition
        required: false
        constraints: filter のとき。項目の参照は ColumnRef だけ（BR1.4）

  - name: ColumnRef
    description: stats のあとの命令で使う、集計の結果の列の参照（FieldRef に Column(index) として足す）
    attributes:
      - name: index
        type: integer
        required: true
        constraints: 結果の列の位置。読み取りの時に名前から解く（BR1.4 (b)）
      - name: name
        type: string
        required: true
        constraints: 書いた名前（エラーの文言と見出しに使う）
      - name: sortKeys
        type: list<ColumnSortKey>
        required: false
        constraints: sort のとき
      - name: limit
        type: integer
        required: false
        min: 0
        max: 100000
        constraints: limit のとき（今の MAX_LIMIT）

  - name: ColumnSortKey
    description: 集計の結果の列で並べる鍵
    attributes:
      - name: column
        type: string
        required: true
        constraints: 結果の列の名前（as の名前か、書いたとおりの文字）
      - name: direction
        type: enum
        required: true
        allowed_values: [asc, desc]
        default: asc（今の sort と同じ）

  - name: QueryVocabulary
    description: QueryCatalog の 1 つの語（新規）
    attributes:
      - name: name
        type: string
        required: true
        unique: true
      - name: kind
        type: enum
        required: true
        allowed_values: [command, aggregateFunction, function, keyword, fixedField]
      - name: supported
        type: boolean
        required: true
        constraints: false の語は読み取りで Unsupported の問題にする（今の OTHER_COMMANDS にあたる）
      - name: reserved
        type: boolean
        required: true
        default: false
        constraints: keyword のとき。true は and・or・not・like・in だけ（どこでも項目の名前として読まない、BR4.1）

  - name: QueryResult
    description: 評価の結果（U1 のまま）。aggregates でも打ち切りを持つ
    attributes:
      - name: matchedCount
        type: integer
        required: true
        constraints: aggregates では stats のあとの filter で残った行の数（打ち切りの前、BR3.2）
      - name: truncatedAt
        type: integer
        required: false
        constraints: aggregates でも、limit を書かずに 10,000 行を超えたら 10,000、limit を書いて残りを切ったらその数（BR3.2）
```

## エンティティの要約

- **関数**：AggregateSpec の関数が 12 種類（13 の書き方）になる。pct は百分位（0〜100）を持つ。Accumulator は関数ごとに、数・合計・逐次の平均と 2 乗和（stddev）・値の一覧（pct）・値の文字の集まり（count_distinct）・最小最大（min・max・sortsFirst・sortsLast）・選んだ値（earliest・latest。view の順で最初か最後）を持つ。
- **stats のあと**：StatsCommand があとに続く filter・sort・limit を書いた順に持つ。どれも集計の結果の列（ColumnRef。FieldRef の Column）に対して働く。
- **一覧**：QueryCatalog が QueryVocabulary（語・種類・対応の有無）を 1 か所に持つ。
- **結果**：aggregates でも打ち切り（truncatedAt）を持つ。
