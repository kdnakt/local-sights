# Entities — U4 時間範囲とタイムゾーン（u4-time-range）

上流の成果物：`inception/domain-design/components.md`（TimeRangeModel・AppSession・DesktopUi）、`inception/units-generation/unit-of-work.md`（U4 の範囲）、`inception/requirements-analysis/requirements.md`（FR3.2〜FR3.4、FR3.6。FR3.1・FR3.5 は U1 で作ったもの）。質問票の回答は `functional-design-questions.md` の Q1〜Q4。

U4 で新しく扱うエンティティと、U1〜U3 のエンティティに足す・変える属性だけを、論理的な型と制約のレベルで定める。前の単位のルールを指すときは `U1:BRx.y` のように作業単位名を前に付ける。前に何も付けない `BRx.y` はこの単位（U4）の rules.md のルール。

## エンティティ（機械可読）

```yaml
entities:
  - name: TimeZoneChoice
    owner: TimeRangeModel
    components_md: components.md の TimeRangeModel の「タイムゾーンはローカルと UTC の 2 つ」を表す補助
    description: 日時の入力とログ一覧の時刻表示に使うタイムゾーン
    attributes:
      - name: value
        type: enum
        allowed_values: [Local, Utc]
        required: true
        default: Local
        constraints: 起動のたびに Local から始め、保存しない（BR1.1、Q2）。Local は OS のタイムゾーンの設定
    constraints: []
    relationships: []

  - name: DateTimeInput
    owner: TimeRangeModel
    components_md: components.md にはないが、TimeRangeModel の責任「日時の入力の解釈」と「同じ瞬間を保つ」を表す補助
    description: 開始日時または終了日時の 1 つの入力欄。文字列と、解釈できたときの瞬間を持つ
    attributes:
      - name: text
        type: text
        required: true
        default: ""
        constraints: 画面に表示する文字列。利用者が入力した文字列か、タイムゾーンの切り替えで瞬間から作り直した文字列（BR1.4）。瞬間を持たない入力は、切り替えても文字列を残し、新しいタイムゾーンで解釈し直す（BR1.4、レビュー R-02）
      - name: instant
        type: instant-millis
        required: false
        constraints: text を選んだタイムゾーンで解釈できたときだけ持つ。その秒の 0 ミリ秒（U1:BR2.1）
      - name: error
        type: enum
        allowed_values: [Format, NonexistentLocalTime]
        required: false
        constraints: 空でない text を、いまのタイムゾーンで解釈できなかったときだけ持つ。instant とは同時に持たない（BR1.2、BR2.2、Q3）。空の text はどちらも持たない。NonexistentLocalTime は timeZone が Local のときだけ取り得る（UTC に切り替えると解釈し直される。BR1.4）
    constraints:
      - instant と error は同時に持たない
      - タイムゾーンを切り替えても、instant を持つ入力の instant は変わらない（BR1.4、FR3.3）。ただし新しいタイムゾーンで 4 桁の年に表せない瞬間は、文字列を残して解釈し直す
      - 利用者が書き換えない限り、instant は解釈し直さない（BR1.5、レビュー R-06）
    relationships: []

  - name: RowWindow
    owner: EventTimeline
    components_md: U3 の補助 RowWindow に属性を足す
    description: 画面が表示範囲だけを描くために取り寄せる行のまとまり
    attributes:
      - name: rows[].displayTime
        type: text
        required: true
        constraints: 各行の timestamp を、取り寄せた時点の timeZone で yyyy-mm-dd hh:mm:ss.mmm に文字列化したもの。表せないときは数値の文字列（BR3.1、BR3.4、レビュー R-01・R-05）。画面は変換をせずにこの文字列を表示する
    constraints:
      - U3 の属性（offset・rows・totalCount・timelineVersion）はそのまま
    relationships: []

  - name: TimeRange
    owner: TimeRangeModel
    components_md: TimeRange（U1 の補助）。属性は変えない
    description: 1 回の取得のミリ秒の範囲
    attributes: []
    constraints:
      - U1 の startInstant・endInstant をそのまま使う。開始・終了の DateTimeInput の instant から作る（BR1.6）
    relationships: []

  - name: ValidationError
    owner: AppSession
    components_md: U1 の補助 ValidationError に種類を足す
    description: [Fetch] を押せない理由のうち、入力の検証の誤り
    attributes:
      - name: kind
        type: enum
        allowed_values: [LogGroupRequired, LogGroupTooLong, StartFormat, EndFormat, StartNonexistentLocalTime, EndNonexistentLocalTime, RangeOrder]
        required: true
        constraints: StartNonexistentLocalTime・EndNonexistentLocalTime を足す（BR2.2、Q3）。そのほかは U1 のまま
    constraints: []
    relationships: []

  - name: SessionState
    owner: AppSession
    components_md: SessionState（同名）。U1〜U3 の属性に足す
    description: 画面の状態
    attributes:
      - name: timeZone
        type: reference
        references: TimeZoneChoice
        required: true
        constraints: 起動時は Local（BR1.1）
      - name: startInput
        type: reference
        references: DateTimeInput
        required: true
        constraints: U1 の開始日時の文字列を DateTimeInput に置き換える
      - name: endInput
        type: reference
        references: DateTimeInput
        required: true
        constraints: U1 の終了日時の文字列を DateTimeInput に置き換える
    constraints: []
    relationships:
      - target: TimeZoneChoice
        cardinality: 1..1
        direction: SessionState が持つ
      - target: DateTimeInput
        cardinality: 2..2
        direction: SessionState が開始と終了の 2 つを持つ
```

## まとめ

| エンティティ | 持ち主 | components.md との対応 | 備考 |
|--------------|--------|------------------------|------|
| TimeZoneChoice | TimeRangeModel | タイムゾーンの補助 | Local と Utc。起動のたびに Local（Q2） |
| DateTimeInput | TimeRangeModel | 入力欄の補助 | 文字列と瞬間。切り替えで瞬間は変わらない |
| TimeRange | TimeRangeModel | U1 の補助（変えない） | 瞬間から作る |
| RowWindow | EventTimeline | U3 の補助（属性を足す） | 行の時刻をライブラリが文字列化して渡す（R-01） |
| ValidationError | AppSession | U1 の補助（種類を足す） | 夏時間で存在しない日時（Q3） |
| SessionState | AppSession | 同名（属性を足す） | タイムゾーン、開始・終了の入力 |
