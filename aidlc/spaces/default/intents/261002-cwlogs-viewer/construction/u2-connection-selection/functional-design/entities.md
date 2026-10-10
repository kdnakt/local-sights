# Entities — U2 接続とロググループの選択（u2-connection-selection）

上流の成果物：`inception/domain-design/components.md`（ConnectionCatalog の ConnectionProfile・RegionOption、LogGroupBrowser の LogGroup）、`inception/units-generation/unit-of-work.md`（U2 の範囲）、`inception/requirements-analysis/requirements.md`（FR1.1、FR1.3、FR1.4、FR2.1〜FR2.3）。質問票の回答は `functional-design-questions.md` の Q1〜Q6。

U2 で新しく扱うエンティティと、U1 のエンティティに足す属性だけを、論理的な型と制約のレベルで定める。名前と持ち主は components.md に合わせる。components.md にないものは、U2 の作業のための補助として持ち主と対応を明記する。

## エンティティ（機械可読）

```yaml
entities:
  - name: ConnectionProfile
    owner: ConnectionCatalog
    components_md: ConnectionProfile（同名）。U1 の補助 ConnectionTarget の「プロファイルの部分」を置き換える
    description: プロファイルの一覧の 1 行。先頭の「既定の設定（SDK に任せる）」と、設定ファイルに定義されたプロファイルがある
    identifier: [kind, profileName]
    attributes:
      - name: kind
        type: enum
        allowed_values: [SdkDefault, Named]
        required: true
        constraints: SdkDefault は一覧に 1 つだけで、常に先頭（Q1）
      - name: profileName
        type: text
        required: false
        constraints: kind が Named のときだけ持つ。設定ファイルのセクション名から取ったプロファイル名。kind が SdkDefault のときは持たない
      - name: defaultRegion
        type: text
        required: false
        constraints: そのプロファイルの既定のリージョン（BR1.5）。なければ持たない
    constraints:
      - 認証情報そのもの（シークレットアクセスキー・セッショントークン・SSO トークン・アクセスキー ID）は持たない（NFR5、project.md Forbidden）
      - kind が Named の profileName は一覧の中で重複しない
    relationships: []

  - name: RegionOption
    owner: ConnectionCatalog
    components_md: RegionOption（同名）
    description: リージョンの選択肢の 1 つ
    identifier: [regionCode]
    attributes:
      - name: regionCode
        type: text
        required: true
        unique: true
        constraints: 例 ap-northeast-1。AWS SDK が持つ公開リージョンの一覧から作る（BR1.4）
    constraints: []
    relationships: []

  - name: ConnectionSelection
    owner: AppSession
    components_md: SessionState の selectedProfile と、リージョンの選択を U2 向けにまとめた補助
    description: いま選んでいる接続（プロファイルとリージョン）
    attributes:
      - name: profile
        type: reference
        references: ConnectionProfile
        required: false
        constraints: 起動時は未選択（Q2）
      - name: regionCode
        type: text
        required: false
        constraints: 未選択がありうる（Q3）。選ぶと RegionOption の regionCode のどれか
    constraints:
      - profile と regionCode の両方が選ばれているときだけ「接続が決まっている」とする（BR2.3）
    relationships:
      - target: ConnectionProfile
        cardinality: 0..1
        direction: ConnectionSelection が参照する
      - target: RegionOption
        cardinality: 0..1
        direction: ConnectionSelection が参照する

  - name: LogGroup
    owner: LogGroupBrowser
    components_md: LogGroup（同名）
    description: ロググループ一覧の 1 行
    identifier: [logGroupName]
    attributes:
      - name: logGroupName
        type: text
        required: true
        unique: true
      - name: arn
        type: text
        required: false
        constraints: 表示には使わない。アカウント ID を含むが、リポジトリには置かない（project.md Forbidden）
      - name: creationTime
        type: instant-millis
        required: false
    constraints: []
    relationships: []

  - name: LogGroupListing
    owner: LogGroupBrowser
    components_md: components.md にはないが、LogGroupBrowser の責任「ロググループの全件列挙」の進み具合と結果を表す補助
    description: 1 つの接続に対する、ロググループ一覧の取得の状態と結果
    identifier: [listingId]
    attributes:
      - name: listingId
        type: identifier
        required: true
        unique: true
        constraints: 取得を始めるたびに新しくする。古い取得の応答を捨てるために使う（BR3.6）
      - name: profile
        type: reference
        references: ConnectionProfile
        required: true
      - name: regionCode
        type: text
        required: true
      - name: status
        type: enum
        allowed_values: [Loading, Complete, Partial]
        required: true
        constraints: Partial は途中でエラーになり、取得できた分だけを持つ状態（Q4）
      - name: groups
        type: list
        required: true
        constraints: LogGroup の一覧。logGroupName の昇順（BR3.3）
      - name: failure
        type: reference
        references: ApiFailure
        required: false
        constraints: status が Partial のときだけ持つ
    constraints:
      - status が Complete のとき、DescribeLogGroups のすべてのページを取得し終えている（BR3.1）
    relationships:
      - target: LogGroup
        cardinality: 0..*
        direction: LogGroupListing が持つ
      - target: ApiFailure
        cardinality: 0..1
        direction: LogGroupListing が参照する

  - name: LogGroupFilter
    owner: LogGroupBrowser
    components_md: components.md にはないが、LogGroupBrowser の責任「ロググループ名での絞り込み」の条件を表す補助
    description: ロググループ一覧の絞り込みの文字列
    attributes:
      - name: filterText
        type: text
        required: true
        default: ""
        constraints: 空なら絞り込まない。前後の空白は取り除いて使う（BR3.7）
    constraints: []
    relationships: []

  - name: PendingConnectionChange
    owner: AppSession
    components_md: components.md にはないが、AppSession の責任「接続を変えるときの確認」のための補助（Q6）
    description: 表示中のログがあるときに、利用者の確認を待っている接続の変更
    attributes:
      - name: proposedProfile
        type: reference
        references: ConnectionProfile
        required: false
      - name: proposedRegionCode
        type: text
        required: false
    constraints:
      - proposedProfile と proposedRegionCode の少なくとも一方を持つ（プロファイルを変えるなら proposedProfile、リージョンだけを変えるなら proposedRegionCode）
      - プロファイルを変える場合のリージョンは持たず、適用するときに BR2.2 で決める
      - 確認を待っている間は、ほかの接続の変更と [Fetch] を受け付けない（BR2.5）
      - 同時に 1 つだけ
    relationships: []

  - name: SessionState
    owner: AppSession
    components_md: SessionState（同名）。U1 の最小版に属性を足す
    description: 画面の状態（U1 の属性に、U2 の分を足す）
    attributes:
      - name: connection
        type: reference
        references: ConnectionSelection
        required: true
      - name: selectedLogGroupName
        type: text
        required: false
        constraints: 一度に 1 つだけ（FR2.3）。接続を変えると外れる（BR2.4）
      - name: pendingConnectionChange
        type: reference
        references: PendingConnectionChange
        required: false
      - name: currentListing
        type: reference
        references: LogGroupListing
        required: false
        constraints: いまの接続のロググループ一覧。接続が未決のときは持たない（BR2.3、BR3.6）
      - name: logGroupFilter
        type: reference
        references: LogGroupFilter
        required: true
        constraints: 接続の変更・再読み込みでも消さない（BR3.7）
    constraints:
      - phase が Fetching の間は、connection・selectedLogGroupName を変えられない（BR2.6）
      - 接続の変更を適用したら phase を Idle に戻す（BR2.4）
    relationships:
      - target: ConnectionSelection
        cardinality: 1..1
        direction: SessionState が持つ
      - target: LogGroupListing
        cardinality: 0..1
        direction: SessionState が直近の一覧を参照する
      - target: LogGroupFilter
        cardinality: 1..1
        direction: SessionState が持つ（絞り込みの条件は LogGroupBrowser が解釈する）

  - name: FetchRequest
    owner: AppSession
    components_md: U1 の補助 FetchRequest に属性を足す
    description: 1 回分の取得条件（U1 の属性に、U2 の分を足す）
    attributes:
      - name: profile
        type: reference
        references: ConnectionProfile
        required: true
        constraints: U1 の profileName（手入力）を置き換える。SdkDefault なら SDK の既定の設定を使う
      - name: regionCode
        type: text
        required: false
        constraints: 画面からの取得では必ず持つ（BR2.8）。確認用プログラムでは省略でき、そのときは U1 と同じくプロファイルの既定のリージョンを使う
      - name: logGroupName
        type: text
        required: true
        constraints: 画面ではロググループ一覧で選んだもの（FR2.3）。確認用プログラムでは U1 と同じく引数
    constraints:
      - logStreamName・startText・endText は U1 のまま（ストリーム名は U3 まで手入力）
    relationships: []
```

## まとめ

| エンティティ | 持ち主 | components.md との対応 | 備考 |
|--------------|--------|------------------------|------|
| ConnectionProfile | ConnectionCatalog | 同名 | 先頭は「既定の設定（SDK に任せる）」（Q1） |
| RegionOption | ConnectionCatalog | 同名 | SDK の公開リージョンの一覧から作る |
| ConnectionSelection | AppSession | SessionState の接続の選択をまとめた補助 | 起動時は未選択（Q2、Q3） |
| LogGroup | LogGroupBrowser | 同名 | |
| LogGroupListing | LogGroupBrowser | 全件列挙の進み具合と結果の補助 | 途中のエラーでは Partial（Q4） |
| LogGroupFilter | LogGroupBrowser | 絞り込みの条件の補助 | 部分一致、大文字・小文字を区別しない（Q5） |
| PendingConnectionChange | AppSession | 接続を変えるときの確認の補助 | 表示中のログがあるときだけ（Q6） |
| SessionState | AppSession | 同名（属性を足す） | 選んだロググループ、確認待ちの変更 |
| FetchRequest | AppSession | U1 の補助（属性を足す） | プロファイルとリージョンを一覧の選択から受け取る |

U1 の補助 ConnectionTarget は、U2 で ConnectionProfile とリージョンの選択に置き換える（U1 の entities.md の「U2 以降で変わること」のとおり）。接続先の解決（プロファイル・リージョンで AWS に接続すること）は、引き続き CloudWatchLogsGateway が受け持つ。
