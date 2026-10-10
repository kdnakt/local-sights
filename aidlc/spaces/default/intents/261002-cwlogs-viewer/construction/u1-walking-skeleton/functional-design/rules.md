# Business Rules — U1 薄い一本（u1-walking-skeleton）

上流の成果物：`inception/requirements-analysis/requirements.md`（U1 の FR：FR1.2、FR3.1、FR3.5、FR4.3、FR4.4、FR4.6、FR5.1、FR8.3）、`inception/units-generation/unit-of-work.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: ロググループ名とストリーム名は必須
    category: validation
    applies_to: FetchRequest
    trigger: 入力が変わったとき
    logic: IF 前後の空白を除いた logGroupName または logStreamName が空 THEN [Fetch] を押せず、空の項目を理由として示す
    violation: "[Fetch] を無効にし、理由の文言キーを validationErrors に入れる"
    source: FR3.4（U1 の最小版）、unit-of-work U1
  - id: BR1.2
    statement: 開始・終了日時は yyyy-mm-dd hh:mm:ss 形式で、UTC として解釈する
    category: validation
    applies_to: FetchRequest.startText, FetchRequest.endText
    trigger: 入力が変わったとき
    logic: IF 形式に合わない、または存在しない日付（例 2024-13-01、2024-02-30）THEN 形式の誤りとする
    violation: "[Fetch] を無効にし、形式の誤りを示す"
    source: FR3.1
  - id: BR1.3
    statement: 開始日時は終了日時より前
    category: validation
    applies_to: FetchRequest
    trigger: 入力が変わったとき
    logic: IF 開始 >= 終了（秒単位で比較）THEN 範囲の誤りとする
    violation: "[Fetch] を無効にし、範囲の誤りを示す"
    source: FR3.4（U1 の最小版）
  - id: BR1.4
    statement: 取得中は条件を変更できず、[Fetch] も押せない
    category: constraint
    applies_to: SessionState
    trigger: phase が Fetching の間
    logic: IF phase = Fetching THEN 入力欄と [Fetch] を無効にし、取得中であることを表示する
    violation: 操作を受け付けない
    source: FR4.8（U1 の最小版）、project.md Corrections（最低限の状態表示）
  - id: BR1.5
    statement: プロファイル名が空欄なら AWS SDK の既定の設定を使う
    category: policy
    applies_to: ConnectionTarget.profileName
    trigger: 取得を始めるとき
    logic: IF profileName が空 THEN プロファイルを指定せずに接続する（default プロファイル・環境変数などを SDK が解決する）
    violation: なし
    source: Q2、FR1.2
  - id: BR1.6
    statement: リージョンはプロファイル（または SDK の既定の設定）の既定のリージョンを使う
    category: policy
    applies_to: ConnectionTarget.region
    trigger: 取得を始めるとき
    logic: CloudWatchLogsGateway が接続先（ConnectionTarget）を解決する。IF 既定のリージョンが見つからない THEN GetLogEvents を呼ばずに RegionMissing を返す
    violation: FetchCoordinator は FetchJob を Failed にし、AppSession は phase を Failed にしてエラーを表示する
    source: Q1
  - id: BR1.7
    statement: 認証方式は AWS SDK に任せ、SSO・アクセスキー・AssumeRole・環境変数のいずれのプロファイルでも接続できる
    category: policy
    applies_to: ConnectionTarget
    trigger: 取得を始めるとき
    logic: 認証情報の解決は SDK の仕組みに任せ、アプリは認証情報を読み出さず保存もしない
    violation: SDK が認証に失敗したら AuthRequired のエラーとする
    source: FR1.2、NFR5

  - id: BR1.8
    statement: 取得条件の検証と範囲の算出は、画面にも確認用プログラムにも依存しない共通の検証で行う
    category: constraint
    applies_to: FetchRequest
    trigger: 取得を始める前
    logic: BR1.1〜BR1.3 の検証と BR2.1・BR2.2 の範囲の算出を、ライブラリの共通の検証（FetchRequest の検証と TimeRangeModel）にまとめる。AppSession は入力が変わるたびにこれを呼び、確認用プログラムは引数を受けたときにこれを呼ぶ。FetchCoordinator は検証済みの条件と TimeRange だけを受け取る
    violation: 確認用プログラムは使い方と理由を標準エラーに出し、0 以外の終了コードで終わる
    source: レビュー R-06

  - id: BR2.1
    statement: 開始のミリ秒は、開始日時のその秒の 0 ミリ秒
    category: calculation
    applies_to: TimeRange.startInstant
    trigger: 取得を始めるとき
    logic: startInstant = 開始日時（UTC）を 1970-01-01T00:00:00Z からのミリ秒にしたもの
    violation: なし
    source: FR3.1
  - id: BR2.2
    statement: 終了日時は、その秒の終わり（999 ミリ秒）までを含む
    category: calculation
    applies_to: TimeRange.endInstant
    trigger: 取得を始めるとき
    logic: endInstant = 終了日時（UTC）のミリ秒 + 999。GetLogEvents の終了時刻（その時刻ちょうどは含まない）には endInstant + 1 を渡す
    violation: なし
    source: FR3.5

  - id: BR3.1
    statement: GetLogEvents は開始時刻と終了時刻を必ず指定し、古い順に読む
    category: constraint
    applies_to: EventFetcher
    trigger: 各ページを取得するとき
    logic: 毎回 startTime = startInstant、endTime = endInstant + 1、startFromHead = true を渡す
    violation: なし（指定しない呼び出しは作らない）
    source: FR4.3、constraint-register C-T4
  - id: BR3.2
    statement: ページの終わりは、渡したトークンと同じトークンが返ったときだけ
    category: constraint
    applies_to: PageCursor
    trigger: 各ページの応答を受けたとき
    logic: IF 最初の呼び出しではなく、かつ受け取ったトークン = 送ったトークン THEN 終わる。IF 応答に次のトークンがない THEN それ以上たどれないため終わる（防御のための扱い）。ELSE 受け取ったトークンを次に送って続ける。空のページや件数の少ないページでは終わらない。pageCount は応答を受けた回数で、最後の同じトークンの応答も数える
    violation: なし
    source: FR4.4、constraint-register C-T2・C-T3
  - id: BR3.3
    statement: 件数の上限を設けない
    category: policy
    applies_to: EventFetcher
    trigger: 取得中
    logic: ページの終わりまで取得し、件数で打ち切らない
    violation: なし
    source: FR4.6、D-14
  - id: BR3.4
    statement: 呼ぶ API は GetLogEvents だけ（U1）
    category: constraint
    applies_to: CloudWatchLogsGateway
    trigger: 常に
    logic: AWS への呼び出しの境界は読み取り 3 API だけを持ち、U1 では GetLogEvents だけを使う
    violation: なし（それ以外の API を呼ぶ手段を作らない）
    source: NFR6、project.md Forbidden

  - id: BR4.1
    statement: 取得の途中でエラーになっても、取得できたページの分は表示したままにする
    category: policy
    applies_to: FetchOutcome
    trigger: GetLogEvents がエラーを返したとき
    logic: IF エラー THEN それまでの LogEvent を残し、status = Failed とし、ApiFailure を付ける
    violation: なし
    source: Q3
  - id: BR4.2
    statement: AWS のエラーを種類に分類する
    category: calculation
    applies_to: ApiFailure.kind
    trigger: AWS SDK がエラーを返したとき
    logic: 認証切れ・認証情報なし → AuthRequired、権限なし → AccessDenied、スロットリング → Throttled、接続・タイムアウト → Network、ロググループ・ストリームが存在しない → NotFound、パラメータの誤り → InvalidInput、それ以外 → Other
    violation: なし
    source: ADR-006
  - id: BR4.3
    statement: エラーの詳細と診断ログに、秘密の認証情報とアクセスキー ID を出さない
    category: constraint
    applies_to: ApiFailure.safeDetail、診断ログ
    trigger: エラーを表示・記録するとき
    logic: safeDetail には種類・API 名・リクエスト ID・プロファイル名・ロール ARN・アカウント ID・ロググループ名・ストリーム名だけを入れ、SDK の生のエラーメッセージはそのまま出さない。シークレットアクセスキー・セッショントークン・SSO トークン・アクセスキー ID（AKIA／ASIA で始まる 20 文字）に当たる文字列は伏せ字にする
    violation: なし
    source: FR8.3、project.md Forbidden
  - id: BR4.4
    statement: U1 のエラー表示は、種類名と安全な詳細をそのまま出す暫定の表示
    category: policy
    applies_to: DesktopUi
    trigger: phase が Failed のとき
    logic: 種類名（文言キーで英日を引く）と safeDetail をステータス行に出す。文としての仕上げは U7
    violation: なし
    source: unit-of-work（エラー表示の暫定扱い）

  - id: BR4.5
    statement: 取得の進み具合とログは、取得開始時に呼び出し元が渡した受け口に届け、ログは EventTimeline が持つ
    category: policy
    applies_to: FetchCoordinator、EventTimeline
    trigger: 取得の開始から終わりまで
    logic: FetchCoordinator は取得の開始時に EventTimeline の保持ログを破棄し、受け口に「開始」を届ける。各ページを受け取るたびに、その LogEvent を EventTimeline に追加し、受け口に「追加された LogEvent のまとまり（ページ単位）と累計件数」を届ける。終わったら FetchJob（status・件数・失敗）を受け口に届ける。画面はページのまとまりを受けるたびに一覧を更新してよく、確認用プログラムは受け取ったまとまりを順に標準出力に出す
    violation: なし
    source: レビュー R-01、ADR-007

  - id: BR5.1
    statement: 一覧は timestamp の昇順、同じ timestamp は sequence（API が返した順）の昇順で、取得した全件を並べる
    category: policy
    applies_to: LogEvent
    trigger: 取得が終わったとき
    logic: 並べ替えのキーは (timestamp, sequence)。U1 は 1 ストリームで API が古い順に返すため、通常は取得順と一致する。件数で打ち切らない
    violation: なし
    source: Q4、FR4.6
  - id: BR5.2
    statement: 一覧のメッセージは 1 行分だけ表示する
    category: policy
    applies_to: DesktopUi
    trigger: 一覧を描くとき
    logic: 改行を空白に置き換えて 1 行で表示し、収まらない分は省略記号で切る。保持しているメッセージそのものは変えない
    violation: なし
    source: FR5.1
  - id: BR5.3
    statement: 時刻は UTC の yyyy-mm-dd hh:mm:ss.mmm で表示する
    category: calculation
    applies_to: DesktopUi
    trigger: 一覧を描くとき
    logic: timestamp を UTC に直し、ミリ秒まで表示する
    violation: なし
    source: FR3.1（U1 は UTC だけ）
  - id: BR5.4
    statement: 取得が終わったら件数を表示し、0 件なら 0 件であることを文字で示す
    category: policy
    applies_to: SessionState
    trigger: phase が Done または Failed になったとき
    logic: eventCount を表示する。0 なら「0 件」の文言を出す
    violation: なし
    source: FR4.11（U1 の最小版）

  - id: BR6.1
    statement: 画面の文字列はすべて文言キーから英日を引き、OS の言語が日本語なら日本語、それ以外は英語にする
    category: policy
    applies_to: MessageCatalog
    trigger: 画面を描くとき
    logic: キーが en・ja の両方を持つことをテストで確かめる。画面のコードに文字列を直書きしない
    violation: なし
    source: NFR12（U1 の土台）
  - id: BR6.2
    statement: キーボードだけで入力から取得まで操作できる
    category: policy
    applies_to: DesktopUi
    trigger: 常に
    logic: Tab で入力欄と [Fetch] を順に移動でき、Enter で [Fetch] を押せる（押せる条件を満たすとき）。Escape で開いているダイアログを閉じる仕組みを用意する（U1 にはダイアログがない）
    violation: なし
    source: NFR13（U1 の土台）

  - id: BR7.1
    statement: 確認用プログラムは、引数で条件を受け取り、結果をテキストで出す
    category: policy
    applies_to: 確認用プログラム
    trigger: 手元で実行したとき
    logic: 引数は --profile（省略可）・--log-group・--stream・--start・--end（UTC）。時刻（UTC、ミリ秒まで）とメッセージを 1 件 1 行で標準出力に、最後に件数とページ数を標準エラーに出す。エラーは種類と安全な詳細を標準エラーに出し、0 以外の終了コードで終わる
    violation: 引数の誤りは使い方を表示して 0 以外の終了コードで終わる
    source: Q5、team.md Walking Skeleton
  - id: BR7.2
    statement: 確認用プログラムは、自動テストと CI からは実行しない
    category: constraint
    applies_to: 確認用プログラム
    trigger: 常に
    logic: 開発用の例として置き、cargo test の対象にしない。配布するアプリには含めない
    violation: なし
    source: project.md Forbidden、team.md Walking Skeleton
```

## まとめ

| ID | ルール | 種類 | 出典 |
|----|--------|------|------|
| BR1.1 | ロググループ名・ストリーム名は必須 | validation | FR3.4 |
| BR1.2 | 日時は yyyy-mm-dd hh:mm:ss（UTC） | validation | FR3.1 |
| BR1.3 | 開始 < 終了 | validation | FR3.4 |
| BR1.4 | 取得中は変更不可、取得中の表示 | constraint | FR4.8 |
| BR1.5 | プロファイル空欄は SDK の既定 | policy | Q2、FR1.2 |
| BR1.6 | リージョンはプロファイルの既定、なければエラー | policy | Q1 |
| BR1.7 | 認証は SDK に任せる | policy | FR1.2、NFR5 |
| BR1.8 | 検証と範囲の算出は画面・確認用プログラム共通 | constraint | レビュー R-06 |
| BR2.1 | 開始はその秒の 0 ミリ秒 | calculation | FR3.1 |
| BR2.2 | 終了はその秒の 999 ミリ秒まで含む | calculation | FR3.5 |
| BR3.1 | 開始・終了を必ず指定、古い順 | constraint | FR4.3 |
| BR3.2 | 同じトークンが返ったら終わり（次のトークンがなくても終わり） | constraint | FR4.4 |
| BR3.3 | 件数の上限なし | policy | FR4.6 |
| BR3.4 | 呼ぶ API は GetLogEvents だけ | constraint | NFR6 |
| BR4.1 | エラーでも取得できた分は表示 | policy | Q3 |
| BR4.2 | エラーの種類の分類 | calculation | ADR-006 |
| BR4.3 | 秘密とアクセスキー ID を出さない | constraint | FR8.3 |
| BR4.4 | U1 のエラーは暫定表示 | policy | unit-of-work |
| BR4.5 | 進み具合とログは受け口へ、ログは EventTimeline が持つ | policy | レビュー R-01、ADR-007 |
| BR5.1 | (timestamp, sequence) の昇順で全件並べる | policy | Q4、FR4.6 |
| BR5.2 | メッセージは 1 行分 | policy | FR5.1 |
| BR5.3 | 時刻は UTC・ミリ秒まで | calculation | FR3.1 |
| BR5.4 | 件数と 0 件の表示 | policy | FR4.11 |
| BR6.1 | 文言はキーから英日を引く | policy | NFR12 |
| BR6.2 | キーボード操作の土台 | policy | NFR13 |
| BR7.1 | 確認用プログラムの入出力 | policy | Q5 |
| BR7.2 | 確認用プログラムはテスト・CI で実行しない | constraint | project.md Forbidden |
