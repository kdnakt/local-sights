# Entities — U6 ディスクキャッシュ（u6-disk-cache）

上流の成果物：`inception/domain-design/components.md`（LogCache の CacheEntry、FetchCoordinator の FetchJob、AppSession の SessionState）、`inception/units-generation/unit-of-work.md`（U6 の範囲）、`inception/requirements-analysis/requirements.md`（FR7.1〜FR7.9、NFR5、NFR8）。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U6 で新しく扱うエンティティと、U1〜U5 のエンティティに足す・変える属性だけを、論理的な型と制約のレベルで定める。ファイルの具体的な書式（JSON・圧縮の有無など）は Code Generation で決める。前の単位のルールを指すときは `U3:BRx.y` のように作業単位名を前に付ける。前に何も付けない `BRx.y` はこの単位（U6）の rules.md のルール。

## エンティティ（機械可読）

```yaml
entities:
  - name: CacheSettings
    owner: LogCache
    components_md: LogCache の責務「キャッシュの有効・無効と保存場所の管理」を表す補助（components.md には名前がない）
    description: キャッシュの有効・無効の設定。アプリを閉じても覚えておく（Q1）
    identifier: []
    attributes:
      - name: cacheEnabled
        type: boolean
        required: true
        default: false
        constraints: 既定は無効（FR7.1）。設定ファイルがない・読めない・解釈できないときも無効として扱う（BR1.2）
      - name: settingsPath
        type: text
        required: true
        constraints: OS のアプリ設定用フォルダの中のアプリ専用の設定ファイル（macOS では `~/Library/Application Support/` 配下）。保存するのは cacheEnabled の 1 項目と書式の版だけ（BR1.1）
      - name: cacheDirectory
        type: text
        required: true
        constraints: OS のキャッシュ用フォルダの中のアプリ専用フォルダ（macOS では `~/Library/Caches/` 配下）。設定ダイアログに表示する（FR7.3）
    constraints:
      - 秘密の認証情報・アクセスキー ID・プロファイル名は設定ファイルに書かない（project.md Forbidden、BR1.1）

  - name: CacheKey
    owner: LogCache
    components_md: CacheEntry の cacheKey
    description: キャッシュを引くための組み合わせ
    identifier: [profileKey, region, logGroupName]
    attributes:
      - name: profileKey
        type: enum-or-text
        required: true
        constraints: U2 の ConnectionProfile をそのまま写す。kind が SdkDefault なら「SDK の既定」という印だけ、Named ならプロファイル名（BR2.1）
      - name: region
        type: text
        required: true
        constraints: 選択中のリージョンのコード（例 ap-northeast-1）
      - name: logGroupName
        type: text
        required: true
        constraints: 選択中のロググループの名前
    constraints:
      - 3 つすべてが一致したときだけ同じキーとみなす（FR7.4）
      - キャッシュのファイル名にはキーをそのまま使わず、キーから求めたハッシュを使う（BR2.2）。キー自体はファイルの中に持つ

  - name: CacheEntry
    owner: LogCache
    components_md: CacheEntry（同名）
    description: 1 つのキーのキャッシュ。同じキーの取得はすべてここにまとまる（Q3）
    identifier: [cacheKey]
    attributes:
      - name: cacheKey
        type: CacheKey
        required: true
      - name: formatVersion
        type: integer
        required: true
        constraints: ファイルの書式の版。読めない版は壊れたものとして扱う（BR4.1）
      - name: coveredRanges
        type: list of CoveredRange
        required: true
        constraints: 重なりも隣り合いもない区間を開始の早い順に並べたもの（BR3.3）
      - name: events
        type: list of CachedEvent
        required: true
        constraints: coveredRanges のどれかに入る時刻のイベントだけを持つ（BR3.2）。並びは U3:BR4.1 の (timestamp, logStreamName, sequence) の順
      - name: storagePath
        type: text
        required: true
        constraints: cacheDirectory の直下のファイル。アプリ専用フォルダの外には書かない
    constraints:
      - 秘密の認証情報とアクセスキー ID は持たない（project.md Forbidden、NFR5）
      - ファイルは本人だけが読み書きできる権限で作る（BR3.6）

  - name: CoveredRange
    owner: LogCache
    components_md: CacheEntry の coveredRanges の要素
    description: キャッシュ済みの時間範囲
    identifier: []
    attributes:
      - name: startMs
        type: integer
        required: true
        constraints: 開始のエポックミリ秒（含む）
      - name: endMs
        type: integer
        required: true
        constraints: 終了のエポックミリ秒（含む）。startMs 以上。取得を始めた時刻の 5 分前より新しくはしない（BR3.1）

  - name: CachedEvent
    owner: LogCache
    components_md: CacheEntry の中身。U3 の LogEvent をディスクに写したもの
    description: キャッシュに保存した 1 件のイベント
    identifier: [logStreamName, timestamp, sequence]
    attributes:
      - name: timestamp
        type: integer
        required: true
      - name: ingestionTime
        type: integer
        required: false
      - name: message
        type: text
        required: true
        constraints: API が返したまま（改行を含む）
      - name: logStreamName
        type: text
        required: true
      - name: sequence
        type: integer
        required: true
        constraints: ストリームの中での並び順。読み出すときに U3:BR3.2 のとおり 0 から振り直す（BR4.3）

  - name: FetchJob
    owner: FetchCoordinator
    components_md: FetchJob（U3 で定義、U6 で属性を足す）
    description: U3 の FetchJob に、キャッシュから出したかどうかと、キャッシュの扱いの結果を足す
    identifier: [jobId]
    attributes:
      - name: servedFromCache
        type: boolean
        required: true
        default: false
        constraints: キャッシュだけで表示し、AWS の API を呼ばなかったとき true（FR7.4、BR2.3）
      - name: cacheOutcome
        type: enum
        required: true
        allowed_values: [NotUsed, Hit, Saved, NotSaved, SaveFailed, ReadFailed]
        default: NotUsed
        constraints: ステータス行の知らせに使う（BR5.3）。NotUsed は無効のとき、Hit はキャッシュから出したとき、Saved は書き込んだとき、NotSaved は失敗したストリームがあった・途中で終わった・記録できる範囲がなかったとき、SaveFailed は書き込みに失敗したとき、ReadFailed は読めずに AWS から取り直したとき（Q5）

  - name: SessionState
    owner: AppSession
    components_md: SessionState（U1〜U5 で定義、U6 で属性を足す）
    description: 設定ダイアログの状態とキャッシュの知らせを足す
    identifier: [sessionId]
    attributes:
      - name: settingsDialog
        type: enum
        required: true
        allowed_values: [Closed, Open]
        default: Closed
        constraints: 取得中は開けない（BR5.1）
      - name: cacheEnabled
        type: boolean
        required: true
        constraints: CacheSettings.cacheEnabled を写したもの。画面に出すために持つ
      - name: cacheNotice
        type: enum
        required: false
        allowed_values: [Hit, ReadFailed, SaveFailed]
        constraints: 直近の取得の FetchJob.cacheOutcome のうち、ステータス行に出すもの。次の取得の開始で消える（BR5.3）
```

## まとめ

- **CacheSettings** はアプリを閉じても覚える有効・無効の 1 項目（Q1）。読めなければ無効（BR1.2）。
- **CacheEntry** は（プロファイル・リージョン・ロググループ）ごとに 1 つ。範囲とイベントを 1 つにまとめる（Q3）。
- **CoveredRange** の終わりは、取得を始めた時刻の 5 分前より新しくしない（Q2）。
- **FetchJob** に servedFromCache と cacheOutcome を、**SessionState** に設定ダイアログとキャッシュの知らせを足す。
