# Entities — U1 薄い一本（u1-walking-skeleton）

上流の成果物：`inception/domain-design/components.md`（エンティティの持ち主）、`inception/units-generation/unit-of-work.md`（U1 の範囲）、`inception/requirements-analysis/requirements.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U1 で扱うエンティティだけを、論理的な型と制約のレベルで定める。持ち主の部品は components.md のとおり。U2 以降で属性を足すものは、その旨を書く。

## エンティティ（機械可読）

```yaml
entities:
  - name: FetchRequest
    owner: AppSession
    description: 利用者が手入力した 1 回分の取得条件（U1 は 1 ストリームだけ）
    attributes:
      - name: profileName
        type: text
        required: false
        constraints: 空欄なら AWS SDK の既定の設定を使う（Q2）。前後の空白は取り除く
      - name: logGroupName
        type: text
        required: true
        constraints: 1〜512 文字。前後の空白は取り除く
      - name: logStreamName
        type: text
        required: true
        constraints: 1〜512 文字。前後の空白は取り除く
      - name: startText
        type: text
        required: true
        constraints: "yyyy-mm-dd hh:mm:ss 形式（UTC として解釈）"
      - name: endText
        type: text
        required: true
        constraints: "yyyy-mm-dd hh:mm:ss 形式（UTC として解釈）"
    constraints:
      - 開始日時は終了日時より前（同じ秒は不可）
    relationships:
      - target: TimeRange
        cardinality: 1..1
        direction: FetchRequest から TimeRange を算出する

  - name: TimeRange
    owner: TimeRangeModel
    description: 取得に使うミリ秒単位の範囲（U1 は UTC だけ。タイムゾーン切替は U4 で足す）
    attributes:
      - name: startInstant
        type: instant-millis
        required: true
        constraints: 開始日時のその秒の 0 ミリ秒
      - name: endInstant
        type: instant-millis
        required: true
        constraints: 終了日時のその秒の 999 ミリ秒（終了はその秒の終わりまで含む、FR3.5）
      - name: inputTimeZone
        type: enum
        allowed_values: [UTC]
        default: UTC
        constraints: U1 は UTC 固定。U4 で Local を足す
    constraints:
      - startInstant < endInstant
    relationships: []

  - name: ConnectionTarget
    owner: CloudWatchLogsGateway
    description: 1 回の取得で使う接続先。リージョンはプロファイルの既定から決まる（Q1）
    attributes:
      - name: profileName
        type: text
        required: false
        constraints: 空なら SDK の既定の設定
      - name: region
        type: text
        required: true
        constraints: プロファイル（または SDK の既定の設定）の既定のリージョン。見つからなければ取得を始めずエラー
    constraints:
      - 認証情報そのもの（シークレットアクセスキー・セッショントークン・SSO トークン・アクセスキー ID）は持たない
    relationships: []

  - name: LogEvent
    owner: EventTimeline
    description: 取得した 1 件のログ（U1 は 1 ストリーム分を取得した順に持つ）
    attributes:
      - name: timestamp
        type: instant-millis
        required: true
      - name: ingestionTime
        type: instant-millis
        required: false
      - name: message
        type: text
        required: true
        constraints: 取得したまま保持する（改行を含みうる）
      - name: logStreamName
        type: text
        required: true
      - name: sequence
        type: integer
        required: true
        min: 0
        constraints: 取得した順の通し番号。同じ時刻の並び順を決める
    constraints:
      - "識別子は (logStreamName, sequence)"
    relationships:
      - target: FetchOutcome
        cardinality: 0..*
        direction: 1 回の取得の結果が複数の LogEvent を持つ

  - name: PageCursor
    owner: EventFetcher
    description: ページングの進み具合
    attributes:
      - name: sentToken
        type: text
        required: false
        constraints: 最初の呼び出しでは空
      - name: receivedToken
        type: text
        required: false
      - name: pageCount
        type: integer
        required: true
        min: 0
    constraints:
      - receivedToken が sentToken と等しいとき、ページの終わり（FR4.4）
    relationships: []

  - name: FetchOutcome
    owner: FetchCoordinator
    description: 1 回の取得の結果（U1 の最小版）
    attributes:
      - name: status
        type: enum
        allowed_values: [Completed, Failed]
        required: true
      - name: eventCount
        type: integer
        required: true
        min: 0
      - name: pageCount
        type: integer
        required: true
        min: 0
      - name: failure
        type: reference
        references: ApiFailure
        required: false
        constraints: status が Failed のときだけ持つ
    constraints:
      - Failed でも、取得できたページの LogEvent は保持する（Q3）
    relationships:
      - target: LogEvent
        cardinality: 0..*
        direction: FetchOutcome が持つ
      - target: ApiFailure
        cardinality: 0..1
        direction: FetchOutcome が参照する

  - name: ApiFailure
    owner: CloudWatchLogsGateway
    description: 安全な形に変換したエラー
    attributes:
      - name: kind
        type: enum
        allowed_values: [AuthRequired, AccessDenied, Throttled, Network, NotFound, InvalidInput, RegionMissing, Other]
        required: true
      - name: safeDetail
        type: text
        required: true
        constraints: 秘密の認証情報とアクセスキー ID を含まない。プロファイル名・ロール ARN・アカウント ID は含めてよい（FR8.3）
      - name: retryable
        type: boolean
        required: true
        default: false
    constraints: []
    relationships: []

  - name: SessionState
    owner: AppSession
    description: 画面の状態（U1 の最小版）
    attributes:
      - name: phase
        type: enum
        allowed_values: [Idle, Fetching, Done, Failed]
        default: Idle
        required: true
      - name: request
        type: reference
        references: FetchRequest
        required: true
      - name: validationErrors
        type: list
        required: true
        constraints: "[Fetch] を押せない理由（文言キーの一覧）"
      - name: lastOutcome
        type: reference
        references: FetchOutcome
        required: false
    constraints:
      - phase が Fetching の間は、request を変更できない
    relationships:
      - target: FetchRequest
        cardinality: 1..1
        direction: SessionState が持つ
      - target: FetchOutcome
        cardinality: 0..1
        direction: SessionState が参照する

  - name: MessageCatalog
    owner: DesktopUi
    description: 画面の文言を英語・日本語で引く仕組み（U1 で土台を作り、各単位が文言を足す）
    attributes:
      - name: key
        type: text
        required: true
        unique: true
      - name: en
        type: text
        required: true
      - name: ja
        type: text
        required: true
    constraints:
      - 画面に出す文字列は、すべて key を通して引く（直書きしない）
      - OS の言語が日本語なら ja、それ以外は en
    relationships: []
```

## まとめ

| エンティティ | 持ち主 | 役割 | U2 以降で変わること |
|--------------|--------|------|---------------------|
| FetchRequest | AppSession | 手入力した取得条件 | U2 で一覧からの選択、U3 でストリーム名をなくしロググループ全体に |
| TimeRange | TimeRangeModel | ミリ秒範囲（終了はその秒の終わりまで） | U4 でローカルタイムゾーンと夏時間を足す |
| ConnectionTarget | CloudWatchLogsGateway | 接続先（プロファイルと既定リージョン） | U2 でリージョンを選べるように |
| LogEvent | EventTimeline | 取得したログ 1 件 | U3 で複数ストリームを時刻順に |
| PageCursor | EventFetcher | ページングの進み具合 | U3 で再試行の情報を足す |
| FetchOutcome | FetchCoordinator | 1 回の取得の結果 | U3 でストリーム単位の失敗と中断を足す |
| ApiFailure | CloudWatchLogsGateway | 安全な形のエラー | 種類は U3 以降も同じ。文は U7 で仕上げる |
| SessionState | AppSession | 画面の状態 | 各単位が状態を足す |
| MessageCatalog | DesktopUi | 英日の文言 | 各単位が文言を足す |
