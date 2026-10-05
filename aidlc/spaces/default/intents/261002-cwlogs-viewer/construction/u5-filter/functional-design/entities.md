# Entities — U5 絞り込み（u5-filter）

上流の成果物：`inception/domain-design/components.md`（FilterEngine の FilterCondition・FilterResult、AppSession の SessionState）、`inception/units-generation/unit-of-work.md`（U5 の範囲）、`inception/requirements-analysis/requirements.md`（FR6.1〜FR6.5、NFR1、NFR2）。質問票の回答は `functional-design-questions.md` の Q1〜Q4。

U5 で新しく扱うエンティティと、U1〜U4 のエンティティに足す・変える属性だけを、論理的な型と制約のレベルで定める。前の単位のルールを指すときは `U3:BRx.y` のように作業単位名を前に付ける。前に何も付けない `BRx.y` はこの単位（U5）の rules.md のルール。

## エンティティ（機械可読）

```yaml
entities:
  - name: FilterCondition
    owner: FilterEngine
    components_md: FilterCondition（同名）
    description: いま適用する絞り込みの条件
    identifier: [filterId]
    attributes:
      - name: filterId
        type: integer
        required: true
        min: 0
        constraints: 絞り込みの文字列が変わるたびに 1 増える番号。古い絞り込みの結果を見分けるために使う（BR1.4）
      - name: filterText
        type: text
        required: true
        default: ""
        constraints: 前後の空白を除いた文字列。空なら絞り込まない（BR1.1）
    constraints:
      - アプリを終了すると残らない（保存しない）
    relationships: []

  - name: FilterResult
    owner: FilterEngine
    components_md: FilterResult（同名）
    description: 保持ログのうち、条件に合う行の並び
    identifier: [filterId]
    attributes:
      - name: filterId
        type: integer
        required: true
        constraints: どの FilterCondition の結果か
      - name: matchedPositions
        type: list
        required: true
        constraints: 条件に合う行の、保持ログの中での位置の並び。保持ログの並び（U3:BR4.1）と同じ順（BR2.3）
      - name: matchedCount
        type: integer
        required: true
        min: 0
      - name: totalCount
        type: integer
        required: true
        min: 0
        constraints: 結果を作った時点の保持件数
      - name: status
        type: enum
        allowed_values: [Filtering, Ready]
        required: true
        constraints: Filtering は保持ログ全体の絞り込みの途中（BR1.5）。Ready は保持ログのすべてに条件をかけ終えた状態（逐次の追加の分も含む、BR2.1）
    constraints:
      - matchedCount <= totalCount
      - 保持ログを破棄したら空（matchedCount = 0、totalCount = 0）に戻り、filterText は残る（BR2.2）
    relationships:
      - target: LogEvent
        cardinality: 0..*
        direction: FilterResult が位置で参照する

  - name: RowWindow
    owner: EventTimeline
    components_md: U3・U4 の補助 RowWindow に属性を足す
    description: 画面が表示範囲だけを描くために取り寄せる行のまとまり
    attributes:
      - name: filtered
        type: boolean
        required: true
        constraints: 絞り込み中（filterText が空でない）なら true。true のとき offset・rows・totalCount は絞り込み結果の中での値（BR3.1）
      - name: allCount
        type: integer
        required: true
        min: 0
        constraints: 保持ログの全件数（「絞り込み後 / 全件」の全件、FR6.3）
    constraints:
      - U3・U4 の属性（offset・rows・rows[].displayTime・totalCount・timelineVersion）はそのまま
    relationships: []

  - name: SessionState
    owner: AppSession
    components_md: SessionState（同名）。U1〜U4 の属性に足す
    description: 画面の状態
    attributes:
      - name: filter
        type: reference
        references: FilterCondition
        required: true
      - name: filterSummary
        type: text
        required: false
        constraints: 画面に出す絞り込みの状態（matchedCount・totalCount・status）。filterText が空なら持たない（BR3.3）
    constraints: []
    relationships:
      - target: FilterCondition
        cardinality: 1..1
        direction: SessionState が持つ
      - target: FilterResult
        cardinality: 0..1
        direction: SessionState が直近の結果を参照する
```

## まとめ

| エンティティ | 持ち主 | components.md との対応 | 備考 |
|--------------|--------|------------------------|------|
| FilterCondition | FilterEngine | 同名 | filterId を足す。空なら絞り込まない |
| FilterResult | FilterEngine | 同名 | 合う行の位置の並び、Filtering と Ready |
| RowWindow | EventTimeline | U3・U4 の補助（属性を足す） | 絞り込み中は絞り込み結果の中の行 |
| SessionState | AppSession | 同名（属性を足す） | 絞り込みの条件と状態 |
