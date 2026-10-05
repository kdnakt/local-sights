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
        constraints: 前後の空白を除いた文字列。空なら「条件なし」と同じに扱い、絞り込まない（BR1.1）
    constraints:
      - アプリを終了すると残らない（保存しない）
      - filterText が空の FilterCondition は、components.md の「セッションは絞り込み条件を 0 個参照する」と同じ意味（レビュー R-05）
    relationships: []

  - name: FilterResult
    owner: FilterEngine
    components_md: FilterResult（同名）。components.md の matchedPositions を、ずれない行のキーの並び（matchedKeys）として持つ（レビュー R-01）
    description: 保持ログのうち、条件に合う行の並び
    identifier: [filterId, timelineEpoch]
    attributes:
      - name: filterId
        type: integer
        required: true
        constraints: どの FilterCondition の結果か
      - name: timelineEpoch
        type: integer
        required: true
        min: 0
        constraints: どの保持ログに対する結果か。保持ログを破棄するたびに 1 増える番号（取得の開始・中断・接続先の変更、BR2.2）。いまの番号と違う結果は捨てる（BR2.4）
      - name: matchedKeys
        type: list
        required: true
        constraints: 条件に合う行の並べ替えのキー（timestamp、logStreamName、sequence。U3:BR4.1）の並び。キーの昇順（保持ログと同じ順、BR2.3）。キーは行の追加で変わらないため、途中に行が差し込まれても値がずれない
      - name: scanCursor
        type: reference
        references: LogEvent のキー
        required: false
        constraints: 保持ログ全体への絞り込みで、どのキーまで条件をかけ終えたか。status = Filtering の間だけ持つ（BR1.5、BR2.1）
      - name: matchedCount
        type: integer
        required: true
        min: 0
        constraints: matchedKeys の数
      - name: allCount
        type: integer
        required: true
        min: 0
        constraints: 結果を最後に更新した時点の保持ログの全件数（レビュー R-05 で totalCount から名前を変えた）
      - name: status
        type: enum
        allowed_values: [Filtering, Ready]
        required: true
        constraints: Filtering は保持ログ全体の絞り込みの途中で、matchedKeys は「ここまでに見つかった行」（BR1.5）。Ready は保持ログのすべてに条件をかけ終えた状態
      - name: resultVersion
        type: integer
        required: true
        min: 0
        constraints: 結果の中身が変わる（行が増える、入れ替わる、空に戻る）たびに 1 増える番号。画面が行を取り寄せ直す合図に使う（BR3.6、レビュー R-03）
    constraints:
      - matchedCount <= allCount
      - 保持ログを破棄したら、新しい timelineEpoch の空の結果（matchedCount = 0、allCount = 0）に置き換え、filterText は残す（BR2.2）
    relationships:
      - target: LogEvent
        cardinality: 0..*
        direction: FilterResult がキーで参照する

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
        constraints: 保持ログの全件数（「絞り込み後 / 全件」の全件、FR6.3）。絞り込んでいないときは totalCount と同じ
      - name: resultVersion
        type: integer
        required: false
        constraints: filtered のとき、どの結果の版から取り出したか（BR3.6）
    constraints:
      - U3・U4 の属性（offset・rows・rows[].displayTime・totalCount・timelineVersion）はそのまま。totalCount は「この取り出しの母数」（絞り込み中は matchedCount、そうでなければ保持件数）
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
        constraints: filterText が空なら「条件なし」と同じ（components.md の 0..1 の 0 に当たる、レビュー R-05）
      - name: filterSummary
        type: object
        required: false
        constraints: 画面に出す絞り込みの状態（filterId・matchedCount・allCount・status・resultVersion）。filterText が空なら持たない。session-changed と fetch-progress の両方に入れる（BR3.3、BR3.6）
    constraints: []
    relationships:
      - target: FilterCondition
        cardinality: 1..1
        direction: SessionState が持つ（空の文字列は条件なしと同じ）
      - target: FilterResult
        cardinality: 0..1
        direction: SessionState が直近の結果を参照する
```

## まとめ

| エンティティ | 持ち主 | components.md との対応 | 備考 |
|--------------|--------|------------------------|------|
| FilterCondition | FilterEngine | 同名 | filterId を足す。空なら条件なしと同じ |
| FilterResult | FilterEngine | 同名（位置をキーの並びで持つ） | 保持ログの世代、走査の位置、結果の版（R-01、R-03） |
| RowWindow | EventTimeline | U3・U4 の補助（属性を足す） | 絞り込み中は絞り込み結果の中の行 |
| SessionState | AppSession | 同名（属性を足す） | 絞り込みの条件と状態 |
