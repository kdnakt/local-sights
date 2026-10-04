# Entities — U1 薄い一本（u1-walking-skeleton）

上流の成果物：`inception/domain-design/components.md`（エンティティの持ち主）、`inception/units-generation/unit-of-work.md`（U1 の範囲）、`inception/requirements-analysis/requirements.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U1 で扱うエンティティだけを、論理的な型と制約のレベルで定める。名前と持ち主は components.md のエンティティに合わせ、U1 で使う属性だけを持つ（U2 以降で属性を足す）。components.md にないものは、U1 の作業のための補助として持ち主と対応を明記する。

## エンティティ（機械可読）

```yaml
entities:
  - name: FetchRequest
    owner: AppSession
    components_md: SessionState の入力部分（selectedProfile・selectedLogGroup と TimeRange の入力）を U1 の手入力向けにまとめた補助
    description: 利用者（画面）または確認用プログラムが渡す 1 回分の取得条件。U1 は 1 ストリームだけ
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
      - 検証（BR1.1〜BR1.3）は画面にも確認用プログラムにも依存しない共通の検証で行う（BR1.8）
    relationships:
      - target: TimeRange
        cardinality: 1..1
        direction: FetchRequest から TimeRange を 1 つ算出する
      - target: ConnectionTarget
        cardinality: 1..1
        direction: FetchRequest の profileName から ConnectionTarget を 1 つ決める

  - name: TimeRange
    owner: TimeRangeModel
    components_md: TimeRange（同名）
    description: 取得に使うミリ秒単位の範囲（U1 は UTC だけ。タイムゾーン切替は U4 で足す）
    identifier: [startInstant, endInstant]
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
    components_md: ConnectionProfile（ConnectionCatalog が持つ）への参照を、U1 では CloudWatchLogsGateway が接続のために解決した結果として持つ補助。U2 で ConnectionCatalog の ConnectionProfile を使う形に置き換える
    description: 1 回の取得で使う接続先。CloudWatchLogsGateway が解決する。リージョンはプロファイルの既定から決まる（Q1）
    attributes:
      - name: profileName
        type: text
        required: false
        constraints: 空なら SDK の既定の設定
      - name: region
        type: text
        required: true
        constraints: プロファイル（または SDK の既定の設定）の既定のリージョン。見つからなければ CloudWatchLogsGateway が RegionMissing を返す
    constraints:
      - 認証情報そのもの（シークレットアクセスキー・セッショントークン・SSO トークン・アクセスキー ID）は持たない
    relationships: []

  - name: LogEvent
    owner: EventTimeline
    components_md: LogEvent（同名）
    description: 取得した 1 件のログ。U1 では EventTimeline の最小版（取得順に追加・全件の読み出し・破棄だけ）が持つ
    identifier: [logStreamName, sequence]
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
        constraints: 1 回の取得の中で、API が返した順に 0 から振る通し番号
    constraints:
      - 並び順は timestamp の昇順、同じ timestamp は sequence の昇順（BR5.1）
    relationships: []

  - name: StreamFetchOutcome
    owner: EventFetcher
    components_md: StreamFetchOutcome（同名）。U1 では 1 ストリーム分だけ
    description: 1 ストリームの取得の結果とページングの進み具合
    identifier: [logStreamName]
    attributes:
      - name: logStreamName
        type: text
        required: true
      - name: eventCount
        type: integer
        required: true
        min: 0
      - name: pageCount
        type: integer
        required: true
        min: 0
        constraints: GetLogEvents を呼んで応答を受けた回数。最後の「同じトークンが返った」応答も 1 回に数える（BR3.2）
      - name: status
        type: enum
        allowed_values: [Completed, Failed]
        required: true
      - name: failure
        type: reference
        references: ApiFailure
        required: false
        constraints: status が Failed のときだけ持つ
    constraints:
      - 取得中のトークン（送ったトークン・受け取ったトークン）は EventFetcher の内部の状態で、結果には残さない
    relationships:
      - target: ApiFailure
        cardinality: 0..1
        direction: StreamFetchOutcome が参照する

  - name: FetchJob
    owner: FetchCoordinator
    components_md: FetchJob（同名）。U1 では 1 ストリーム分の最小版
    description: 1 回の取得の状態と結果
    identifier: [jobId]
    attributes:
      - name: jobId
        type: identifier
        required: true
        unique: true
      - name: status
        type: enum
        allowed_values: [Running, Completed, Failed]
        required: true
      - name: eventCount
        type: integer
        required: true
        min: 0
      - name: failedStreamCount
        type: integer
        required: true
        min: 0
        max: 1
        constraints: U1 は 0 か 1
    constraints:
      - Failed でも、取得できたページの LogEvent は EventTimeline に残す（Q3）
    relationships:
      - target: StreamFetchOutcome
        cardinality: 1..1
        direction: FetchJob が持つ（U1 は 1 ストリーム）
      - target: TimeRange
        cardinality: 1..1
        direction: FetchJob が参照する

  - name: ApiFailure
    owner: CloudWatchLogsGateway
    components_md: ApiFailure（同名）
    description: 安全な形に変換したエラー
    identifier: [kind]
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
    components_md: SessionState（同名）。U1 の最小版
    description: 画面の状態
    identifier: [sessionId]
    attributes:
      - name: sessionId
        type: identifier
        required: true
        unique: true
      - name: phase
        type: enum
        allowed_values: [Idle, Fetching, Done, Failed]
        default: Idle
        required: true
      - name: validationErrors
        type: list
        required: true
        constraints: "[Fetch] を押せない理由（文言キーの一覧）。空なら押せる"
    constraints:
      - phase が Fetching の間は、FetchRequest を変更できない
    relationships:
      - target: FetchRequest
        cardinality: 1..1
        direction: SessionState が持つ（入力中の条件）
      - target: FetchJob
        cardinality: 0..1
        direction: SessionState が直近の取得を参照する

  - name: MessageCatalog
    owner: DesktopUi
    components_md: components.md にはないが、DesktopUi の責任「表示言語の切り替え」のための補助
    description: 画面の文言を英語・日本語で引く仕組み（U1 で土台を作り、各単位が文言を足す）
    identifier: [key]
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

| エンティティ | 持ち主 | components.md との対応 | U2 以降で変わること |
|--------------|--------|------------------------|---------------------|
| FetchRequest | AppSession | SessionState の入力部分を U1 向けにまとめた補助 | U2 で一覧からの選択、U3 でストリーム名をなくしロググループ全体に |
| TimeRange | TimeRangeModel | 同名 | U4 でローカルタイムゾーンと夏時間を足す |
| ConnectionTarget | CloudWatchLogsGateway | ConnectionProfile の参照を U1 で解決した補助 | U2 で ConnectionCatalog の ConnectionProfile に置き換える |
| LogEvent | EventTimeline | 同名 | U3 で複数ストリームを時刻順に |
| StreamFetchOutcome | EventFetcher | 同名 | U3 で再試行の情報を足す |
| FetchJob | FetchCoordinator | 同名 | U3 で複数ストリーム・中断を、U6 でキャッシュを足す |
| ApiFailure | CloudWatchLogsGateway | 同名 | 種類は U3 以降も同じ。文は U7 で仕上げる |
| SessionState | AppSession | 同名 | 各単位が状態を足す |
| MessageCatalog | DesktopUi | DesktopUi の責任のための補助 | 各単位が文言を足す |

U1 では EventTimeline を最小版（取得順の追加・全件の読み出し・破棄）として作る。unit-of-work では EventTimeline の主な単位は U3 だが、LogEvent の持ち主を components.md のとおり EventTimeline に保つため、U1 から最小版を置く（レビュー R-01、R-02）。
