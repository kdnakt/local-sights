# Business Rules — U2 接続とロググループの選択（u2-connection-selection）

上流の成果物：`inception/requirements-analysis/requirements.md`（U2 の FR：FR1.1、FR1.3、FR1.4、FR2.1〜FR2.3）、`inception/units-generation/unit-of-work.md`、`inception/domain-design/components.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q6。U1 のルール（`construction/u1-walking-skeleton/functional-design/rules.md`）はそのまま有効で、ここでは U2 で足すルールと、U1 のルールを置き換えるところだけを書く。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: プロファイルの一覧は、先頭の「既定の設定（SDK に任せる）」と、設定ファイルに定義されたプロファイル名の昇順で作る
    category: calculation
    applies_to: ConnectionProfile
    trigger: アプリの起動時
    logic: AWS 共有設定ファイル（config の [profile 名前] と [default]、credentials の [名前]）のセクション名を集め、重複を除いて名前の昇順に並べる。先頭に kind = SdkDefault の 1 行を置く。設定ファイルの場所は AWS CLI と同じ（環境変数 AWS_CONFIG_FILE・AWS_SHARED_CREDENTIALS_FILE があればそれを使う）
    violation: なし
    source: FR1.1、Q1
  - id: BR1.2
    statement: 設定ファイルから読むのはセクション名と region だけで、認証情報の値は読み出さず保持もしない
    category: constraint
    applies_to: ConnectionCatalog
    trigger: 設定ファイルを読むとき
    logic: セクション名（プロファイル名）と、そのセクションの region の値だけを取り出す。aws_access_key_id・aws_secret_access_key・aws_session_token・sso の各項目などの値は、読み取りの結果に含めない
    violation: なし（取り出す手段を作らない）
    source: NFR5、project.md Forbidden
  - id: BR1.3
    statement: 設定ファイルがない・読めない・形式が崩れているときも、アプリは止めずに「既定の設定」だけで一覧を作る
    category: policy
    applies_to: ConnectionCatalog
    trigger: 設定ファイルを読むとき
    logic: IF ファイルがない THEN そのファイルからのプロファイルは 0 件として扱う。IF 読めない、または解釈できない THEN そのファイルからのプロファイルは 0 件とし、読めなかったことを文言キーで画面に知らせる（ファイルの中身は出さない）
    violation: 一覧は「既定の設定」と、読めた方のファイルのプロファイルだけになる
    source: FR1.1、NFR5
  - id: BR1.4
    statement: リージョンの選択肢は、AWS SDK が持つ公開リージョンの一覧を、コードの昇順に並べたもの
    category: calculation
    applies_to: RegionOption
    trigger: アプリの起動時
    logic: SDK が持つ公開リージョン（標準のパーティション）の一覧からコードを取り出して昇順に並べる。SDK から一覧を得られない場合は、アプリに組み込んだ同じ内容の一覧を使う
    violation: なし
    source: FR1.3、レビュー R-06（requirements）
  - id: BR1.5
    statement: プロファイルの既定のリージョンは、設定ファイルのそのプロファイルの region。「既定の設定」は SDK と同じ順で決める
    category: calculation
    applies_to: ConnectionProfile.defaultRegion
    trigger: プロファイルの一覧を作るとき
    logic: kind が Named なら、config のそのプロファイルのセクションの region。kind が SdkDefault なら、環境変数 AWS_REGION、次に AWS_DEFAULT_REGION、次に config の [default] の region の順で最初に見つかったもの。見つからなければ持たない。IF 既定のリージョンが BR1.4 の選択肢にない THEN そのコードを選択肢に足す
    violation: なし
    source: FR1.3

  - id: BR2.1
    statement: 起動時は、プロファイルもリージョンも未選択で、ロググループ一覧を取らない
    category: policy
    applies_to: ConnectionSelection
    trigger: アプリの起動時
    logic: profile と regionCode を未選択にする。ロググループの一覧の欄には、プロファイルを選ぶようにうながす文言を出す
    violation: なし
    source: Q2
  - id: BR2.2
    statement: プロファイルを選ぶと、そのプロファイルの既定のリージョンをリージョンの初期値にする。既定がなければ未選択にして選ぶようにうながす
    category: policy
    applies_to: ConnectionSelection.regionCode
    trigger: プロファイルを選んだとき
    logic: IF 選んだプロファイルに defaultRegion がある THEN regionCode = defaultRegion。ELSE regionCode を未選択にし、リージョンを選ぶようにうながす文言を出す
    violation: なし
    source: FR1.3、Q3
  - id: BR2.3
    statement: プロファイルとリージョンの両方が決まったら、ロググループ一覧を取り直す
    category: policy
    applies_to: LogGroupListing
    trigger: プロファイルかリージョンが変わり、両方が選ばれた状態になったとき
    logic: IF profile と regionCode の両方が選ばれている THEN 新しい LogGroupListing（status = Loading）を始める。どちらかが未選択なら一覧を取らず、一覧の欄を空にする
    violation: なし
    source: FR1.4、Q2、Q3
  - id: BR2.4
    statement: 接続を変えると、ロググループの選択を外し、表示中のログ・件数・直近の取得の結果も消す
    category: policy
    applies_to: SessionState
    trigger: 接続の変更を適用するとき
    logic: selectedLogGroupName を外す。EventTimeline の保持ログを破棄し、件数と直近の FetchJob の要約を消す。その後 BR2.3 に従う
    violation: なし
    source: FR1.4、Q6
  - id: BR2.5
    statement: 表示中のログがあるときは、接続を変える前に確認ダイアログを出す
    category: policy
    applies_to: PendingConnectionChange
    trigger: プロファイルかリージョンを変えようとしたとき
    logic: IF EventTimeline に 1 件以上のログがある THEN 変更をすぐには適用せず PendingConnectionChange に入れ、「表示中のログが消えます。変えてよいですか？」の確認ダイアログを出す。[変える] なら BR2.4 で適用し、[キャンセル]（Escape キーでも同じ）なら変更を捨てて元の選択に戻す。ELSE 確認なしで BR2.4 で適用する。確認を待っている間は、ほかの接続の変更・ロググループの選択・[Fetch] を受け付けない
    violation: 確認を待っている間の操作は受け付けない
    source: Q6
  - id: BR2.6
    statement: 取得中と確認を待っている間は、プロファイル・リージョン・ロググループの選択と一覧の再読み込みを変えられない
    category: constraint
    applies_to: SessionState
    trigger: phase が Fetching の間、または PendingConnectionChange があるとき
    logic: IF phase = Fetching、または PendingConnectionChange がある THEN プロファイル・リージョン・ロググループの選択と [再読み込み] を無効にする（U1 の BR1.4 を広げる）。確認を待っている間に使えるのは確認ダイアログの [変える]・[キャンセル] だけ
    violation: 操作を受け付けない
    source: FR4.8（U1 の BR1.4）、Q6
  - id: BR2.7
    statement: "[Fetch] を押せるのは、プロファイル・リージョン・ロググループが選ばれ、ストリーム名と日時が U1 の検証を通るとき"
    category: validation
    applies_to: FetchRequest
    trigger: 選択または入力が変わったとき
    logic: IF プロファイル・リージョン・ロググループのどれかが未選択 THEN [Fetch] を押せず、未選択の項目を理由（文言キー）として示す。ストリーム名と日時は U1 の BR1.1〜BR1.3 のまま。ロググループ名の手入力の検証（U1 の BR1.1 のロググループ名の部分）は、一覧からの選択に置き換える
    violation: "[Fetch] を無効にし、理由の文言キーを validationErrors に入れる"
    source: FR2.3、U1 の BR1.1・BR1.8
  - id: BR2.8
    statement: 画面からの取得は、選んだプロファイルとリージョンで接続する
    category: policy
    applies_to: FetchRequest.regionCode
    trigger: 取得を始めるとき
    logic: 画面からの FetchRequest は、選んだ profile と regionCode を必ず持つ。CloudWatchLogsGateway はそのリージョンで接続する（U1 の BR1.6 のプロファイルの既定のリージョンによる解決は、regionCode を省略した確認用プログラムの取得でだけ使う）。kind が SdkDefault なら、プロファイルを指定せずに接続する（U1 の BR1.5）
    violation: なし
    source: FR1.3、FR1.4、U1 の BR1.5・BR1.6

  - id: BR3.1
    statement: ロググループ一覧は、DescribeLogGroups のページをすべてたどって作る
    category: constraint
    applies_to: LogGroupListing
    trigger: 一覧の取得中
    logic: 次のトークンを付けて DescribeLogGroups を呼び続け、応答に次のトークンがなくなったら status = Complete にする。IF 受け取ったトークンが直前に送ったトークンと同じ THEN それ以上たどれないため Complete にする（防御のための扱い）。件数で打ち切らない
    violation: なし
    source: FR2.1
  - id: BR3.2
    statement: 一覧はページを受け取るたびに更新し、取得中でもロググループを選べる
    category: policy
    applies_to: LogGroupListing
    trigger: ページを受け取ったとき
    logic: 受け取ったロググループを groups に足し、BR3.3 の順に並べて画面に届ける。status が Loading の間は「読み込み中」と表示するが、表示されているロググループは選べる
    violation: なし
    source: FR2.1
  - id: BR3.3
    statement: 一覧はロググループ名の昇順に並べる
    category: calculation
    applies_to: LogGroupListing.groups
    trigger: ページを受け取ったとき
    logic: logGroupName の文字列の昇順で並べる
    violation: なし
    source: FR2.1
  - id: BR3.4
    statement: 一覧の取得の途中でエラーになったら、取得できた分を残し、途中までであることとエラーを表示する
    category: policy
    applies_to: LogGroupListing
    trigger: DescribeLogGroups がエラーを返したとき
    logic: それ以上たどらず、status = Partial にし、それまでの groups を残し、ApiFailure（U1 の BR4.2 の種類と BR4.3 の安全な詳細）を付ける。画面には「一覧が途中までであること」と種類名・安全な詳細を出す（U1 の BR4.4 の暫定表示）。最初のページでエラーなら groups は空のまま Partial になる
    violation: なし
    source: FR2 の受け入れ基準、Q4
  - id: BR3.5
    statement: "[再読み込み] は、いまの接続で一覧を取り直し、ロググループの選択と表示中のログは変えない"
    category: policy
    applies_to: LogGroupListing
    trigger: "[再読み込み] を押したとき"
    logic: 接続が決まっているときだけ押せる。新しい LogGroupListing（status = Loading）を始める。selectedLogGroupName と EventTimeline は変えない
    violation: 接続が決まっていなければ押せない
    source: Q4
  - id: BR3.6
    statement: 古い一覧の取得の応答は捨てる
    category: constraint
    applies_to: LogGroupListing.listingId
    trigger: 一覧の取得の応答を受けたとき
    logic: IF 応答の listingId が、いまの LogGroupListing の listingId と違う THEN その応答を捨てる（接続の変更や再読み込みの後に、前の取得の応答が遅れて届いた場合）。古い取得は、次のページを呼ばずにやめる
    violation: なし
    source: FR1.4
  - id: BR3.7
    statement: ロググループ名の絞り込みは、大文字と小文字を区別しない部分一致
    category: calculation
    applies_to: LogGroupFilter
    trigger: 絞り込みの文字列か一覧が変わったとき
    logic: 前後の空白を除いた filterText が空なら全件を出す。ELSE logGroupName と filterText をどちらも小文字にそろえ、logGroupName に filterText を含むものだけを出す。絞り込みは取得済みの一覧に対して行い、AWS の API は呼ばない。絞り込みの文字列は、接続の変更・再読み込みでも消さない。選んでいるロググループが絞り込みで隠れても、選択は外さない
    violation: なし
    source: FR2.2、Q5
  - id: BR3.8
    statement: ロググループは一度に 1 つだけ選べる
    category: constraint
    applies_to: SessionState.selectedLogGroupName
    trigger: 一覧の行を選んだとき
    logic: 新しく選んだら、それまでの選択は外れる。選び直しても表示中のログは消さない（次の [Fetch] で置き換わる）
    violation: なし
    source: FR2.3
  - id: BR3.9
    statement: 呼ぶ API に DescribeLogGroups を足す。読み取り 3 API 以外は呼ばない
    category: constraint
    applies_to: CloudWatchLogsGateway
    trigger: 常に
    logic: AWS への呼び出しの境界に DescribeLogGroups を足す。名前による絞り込みは画面側の取得済みの一覧で行うため、DescribeLogGroups には次のトークン以外の条件を渡さない
    violation: なし（それ以外の API を呼ぶ手段を作らない）
    source: NFR6、project.md Forbidden
  - id: BR3.10
    statement: ロググループが 0 件、または絞り込みで 0 件のときは、そのことを文字で示す
    category: policy
    applies_to: LogGroupListing
    trigger: 一覧を描くとき
    logic: IF status = Complete で groups が 0 件 THEN 「ロググループがありません」の文言を出す。IF 絞り込みの結果が 0 件 THEN 「一致するロググループがありません」の文言を出す
    violation: なし
    source: FR2.1、FR2.2

  - id: BR4.1
    statement: 認証の失敗や権限不足で一覧を取れないときも、アプリは止めずにエラーを表示する
    category: policy
    applies_to: LogGroupListing
    trigger: 一覧の取得で AuthRequired・AccessDenied などが返ったとき
    logic: BR3.4 のとおり Partial にして種類名と安全な詳細を出す。プロファイルとリージョンは選び直せる。「何が起きたか」と「次の行動」の文への仕上げは U7（FR1.5）
    violation: なし
    source: FR1 と FR2 の受け入れ基準、U1 の BR4.4

  - id: BR5.1
    statement: U2 の画面の文字列も、すべて文言キーから英日を引く
    category: policy
    applies_to: MessageCatalog
    trigger: 画面を描くとき
    logic: プロファイル・リージョンの欄、「既定の設定（SDK に任せる）」、うながしの文言、一覧の読み込み中・途中まで・0 件の文言、絞り込みの欄、[再読み込み]、確認ダイアログの文言とボタンの文言キーを足し、英日の両方をそろえる（U1 の BR6.1）
    violation: なし
    source: NFR12（U1 の BR6.1）
  - id: BR5.2
    statement: U2 の画面もキーボードだけで操作できる
    category: policy
    applies_to: DesktopUi
    trigger: 常に
    logic: Tab でプロファイル・リージョン・絞り込み・ロググループ一覧・[再読み込み] を順に移動できる。一覧は上下の矢印キーで移動し、Enter かスペースで選べる。確認ダイアログは開いたときに [キャンセル] にフォーカスを置き、Escape で閉じる（[キャンセル] と同じ）
    violation: なし
    source: NFR13（U1 の BR6.2）、Q6
```

## まとめ

| ID | ルール | 種類 | 出典 |
|----|--------|------|------|
| BR1.1 | プロファイル一覧は「既定の設定」＋設定ファイルのプロファイル名の昇順 | calculation | FR1.1、Q1 |
| BR1.2 | 設定ファイルからはセクション名と region だけを読む | constraint | NFR5 |
| BR1.3 | 設定ファイルが読めなくても止めない | policy | FR1.1 |
| BR1.4 | リージョンの選択肢は SDK の公開リージョンの一覧 | calculation | FR1.3 |
| BR1.5 | プロファイルの既定のリージョンの決め方 | calculation | FR1.3 |
| BR2.1 | 起動時は未選択で一覧を取らない | policy | Q2 |
| BR2.2 | プロファイルを選ぶとリージョンに既定を入れる、なければうながす | policy | FR1.3、Q3 |
| BR2.3 | 接続が決まったら一覧を取り直す | policy | FR1.4 |
| BR2.4 | 接続を変えたら選択と表示中のログを消す | policy | FR1.4、Q6 |
| BR2.5 | 表示中のログがあれば確認ダイアログ | policy | Q6 |
| BR2.6 | 取得中・確認中は選択を変えられない | constraint | FR4.8 |
| BR2.7 | [Fetch] を押せる条件に選択を足す | validation | FR2.3 |
| BR2.8 | 画面からの取得は選んだプロファイルとリージョンで接続 | policy | FR1.3、FR1.4 |
| BR3.1 | 一覧は DescribeLogGroups の全ページ | constraint | FR2.1 |
| BR3.2 | ページごとに一覧を更新し、取得中も選べる | policy | FR2.1 |
| BR3.3 | 一覧は名前の昇順 | calculation | FR2.1 |
| BR3.4 | 途中のエラーは取得できた分を残して表示 | policy | FR2 の受け入れ基準、Q4 |
| BR3.5 | [再読み込み] は選択とログを変えない | policy | Q4 |
| BR3.6 | 古い一覧の応答は捨てる | constraint | FR1.4 |
| BR3.7 | 絞り込みは大文字・小文字を区別しない部分一致 | calculation | FR2.2、Q5 |
| BR3.8 | ロググループは 1 つだけ選べる | constraint | FR2.3 |
| BR3.9 | DescribeLogGroups を足す、読み取り 3 API だけ | constraint | NFR6 |
| BR3.10 | 0 件のときの表示 | policy | FR2.1、FR2.2 |
| BR4.1 | 認証失敗・権限不足でも止めずに表示 | policy | FR1・FR2 の受け入れ基準 |
| BR5.1 | 文言はキーから英日 | policy | NFR12 |
| BR5.2 | キーボード操作 | policy | NFR13、Q6 |
