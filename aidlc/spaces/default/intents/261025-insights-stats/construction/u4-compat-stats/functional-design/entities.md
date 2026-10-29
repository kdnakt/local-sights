# Entities — U4 互換の確かめ（u4-compat-stats）

上流：`inception/units-generation/unit-of-work.md`（U4）・`unit-of-work-story-map.md`（FR7.1〜FR7.3）、`inception/requirements-analysis/requirements.md`（FR7、NFR3・NFR4、OQ4）、U1・U2 の機能設計（コンソールと比べるとした仮の決まり、BR2.8 の小数の文字、ResultColumns・ResultRow）、質問票 `functional-design-questions.md`（Q1〜Q4）、今のコード（`examples/compat_check.rs`、`examples/compat/{queries,settings,export,format,compare,report}.rs`、`examples/compat_queries.txt`、`scripts/compat/insights-export.sh`）。

今の確認用プログラムの型（ComparedQuery、CompareMode、Row = (列の名前, 値の文字) の並び、AppSide、InsightsSide、QueryComparison、報告）は残し、集計の結果を比べるための部分だけを足す。

## エンティティ（機械可読）

```yaml
entities:
  - name: ComparedQuery
    description: 比べる 1 つのクエリ（今のものを広げる）。組の 1 行
    attributes:
      - name: id
        type: string
        required: true
        unique: true
        constraints: 今の書き方（q01・p01 など）。集計のクエリは s01〜（CloudTrail の組）と n01〜（数の組）
      - name: mode
        type: enum
        required: true
        allowed_values: [order, set]
        constraints: 今のまま。stats のあとに sort を書いたクエリは order（FR7.1）
      - name: purpose
        type: string
        required: true
      - name: query
        type: string
        required: true
      - name: tolerances
        type: list<ColumnTolerance>
        required: false
        default: 空（すべて完全一致）
        constraints: 列ごとの比べ方の緩め。組のファイルの 5 つ目の欄（省略可）に書く（BR1.3）

  - name: ColumnTolerance
    description: 列ごとの比べ方
    attributes:
      - name: column
        type: string
        required: true
        constraints: 結果の列の名前
      - name: kind
        type: enum
        required: true
        allowed_values: [decimal, approximate]
        constraints: decimal は小数の誤差（BR2.2）、approximate は count_distinct の近似（BR2.3）

  - name: QuerySet
    description: 比べるクエリの組のファイル
    attributes:
      - name: path
        type: string
        required: true
        constraints: 今の `compat_queries.txt`（CloudTrail）に集計のクエリを足す。数の集計は新しい `compat_queries_numeric.txt`（BR1.1・BR1.2）
      - name: queries
        type: list<ComparedQuery>
        required: true

  - name: AppRows
    description: アプリの結果（今の AppRows を広げる）
    attributes:
      - name: shape
        type: enum
        required: true
        allowed_values: [events, aggregates]
        constraints: アプリの QueryResult の形（U1 の BR3.1）
      - name: columns
        type: list<string>
        required: true
      - name: rows
        type: list<Row>
        required: true
        constraints: aggregates のセルは表と同じ文字（U1 の BR2.8・BR4.2。時刻は UTC の Logs Insights の書き方、今の insights_timestamp）

  - name: CellDifference
    description: 1 つのセルの違いの種類（報告に使う）
    attributes:
      - name: kind
        type: enum
        required: true
        allowed_values: [exact, withinDecimal, withinApproximate, different]
        constraints: withinDecimal と withinApproximate は「同じとみなした違い」として報告に分けて出す（Q1 A・Q2 A）

  - name: ProvisionalRuleCheck
    description: U1・U2 の仮の決まりのうち、比べて確かめるもの（報告と記録に使う）
    attributes:
      - name: rule
        type: string
        required: true
        constraints: U1・U2 の BR の番号（例 U1:BR2.1）
      - name: queryIds
        type: list<string>
        required: true
        constraints: その決まりを確かめるクエリの id（BR1.4）
      - name: outcome
        type: enum
        required: false
        allowed_values: [matches, differs, notChecked]
        constraints: 作る段階で手元で比べた結果。differs なら直したか諮ったかを記録する（BR3.1）
```

## エンティティの要約

- **組**：今の CloudTrail の組に集計のクエリ（s01〜）を足し、数の集計用に新しい組（n01〜）を作る。列ごとの比べ方の緩め（小数・近似）を 5 つ目の欄に書ける。
- **アプリの結果**：AppRows が形を持ち、集計の行は表と同じ文字で比べる。
- **違いの種類**：完全一致、小数の誤差の範囲、近似の範囲、違い。範囲内の違いは報告に分けて出す。
- **仮の決まりの確かめ**：U1・U2 の仮の決まりと、それを確かめるクエリの対応を持ち、作る段階の結果を記録する。
