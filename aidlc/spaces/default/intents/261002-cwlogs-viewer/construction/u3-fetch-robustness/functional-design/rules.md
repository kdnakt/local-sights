# Business Rules — U3 取得の作り込み（u3-fetch-robustness）

上流の成果物：`inception/requirements-analysis/requirements.md`（U3 の FR：FR4.1、FR4.2、FR4.5、FR4.7、FR4.8、FR4.10、FR4.11、NFR2〜NFR4）、`inception/units-generation/unit-of-work.md`、`inception/domain-design/components.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U1・U2 のルールは、ここで置き換えると書いたもの以外はそのまま有効。U1・U2 とルールの番号が重なるため、前の単位のルールは `U1:BR3.2` のように作業単位名を前に付けて書く。前に何も付けない `BRx.y` は、この文書（U3）のルールを指す。部品とエンティティの名前は components.md と entities.md のものを使う（U1 の rules.md の BR3.2・BR4.1 の applies_to にある古い名前 PageCursor・FetchOutcome は、それぞれ EventFetcher の中のページング（StreamFetchOutcome）と FetchJob・EventTimeline を指す。U1 の機能設計のレビュー R-08 の持ち越し）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: ストリームは、最後のイベント時刻の新しい順に DescribeLogStreams で列挙する
    category: constraint
    applies_to: StreamPlanner
    trigger: 取得を始めたとき
    logic: DescribeLogStreams を、選んだロググループ・最後のイベント時刻の順・新しい順を指定して呼び、次のトークンをたどる。ページの終わりは「次のトークンがない」か「送ったトークンと同じトークンが返った」とき（U1:BR3.2 と同じ判定）。ストリーム名の前方一致などの条件は渡さない
    violation: なし
    source: FR4.1、Q2
  - id: BR1.2
    statement: 最後のイベント時刻が「範囲の開始 − 1 時間」より古いストリームが出たら、列挙をやめる
    category: policy
    applies_to: StreamPlan.listingStatus
    trigger: 列挙のページを受けたとき
    logic: IF ページの中に lastEventTimestamp を持ち、かつ lastEventTimestamp < TimeRange.startInstant − 3,600,000 ミリ秒のストリームがある THEN そのストリームとそれより後ろ（同じページの残りを含む）は対象にせず、次のページを呼ばずに listingStatus = StoppedEarly とする。lastEventTimestamp を持たないストリームでは打ち切らない
    violation: なし
    source: FR4.2、Q2
  - id: BR1.3
    statement: 範囲と重なるストリームと、時刻を持たないストリームだけを取得対象にする
    category: calculation
    applies_to: StreamPlan.selectedStreams
    trigger: 列挙のページを受けたとき
    logic: 列挙で得たストリームのうち、IF firstEventTimestamp と lastEventTimestamp の両方を持つ THEN（lastEventTimestamp ≥ startInstant − 3,600,000）かつ（firstEventTimestamp ≤ endInstant）のときだけ対象にする。IF どちらか一方でも持たない THEN 対象にする（取りこぼしを避ける）。対象の並びは列挙で得た順のまま
    violation: なし
    source: FR4.2、Q2
  - id: BR1.4
    statement: 呼ぶ API に DescribeLogStreams を足す。読み取り 3 API 以外は呼ばない
    category: constraint
    applies_to: CloudWatchLogsGateway
    trigger: 常に
    logic: AWS への呼び出しの境界に DescribeLogStreams を足す。渡すのはロググループ名・並び順（最後のイベント時刻）・新しい順・次のトークンだけ
    violation: なし（それ以外の API を呼ぶ手段を作らない）
    source: NFR6、project.md Forbidden
  - id: BR1.5
    statement: ストリームの列挙が途中でエラーになったら、それまでに選んだストリームだけを取得し、列挙が途中までであることを結果に残す
    category: policy
    applies_to: StreamPlan
    trigger: DescribeLogStreams が再試行（BR2.1）の後もエラーを返したとき
    logic: それ以上列挙せず、listingStatus = Partial とし、ApiFailure を listingFailure に付ける。それまでに選んだストリームは取得する。最初のページでエラーなら対象は 0 件になる
    violation: なし
    source: FR4.10、NFR4

  - id: BR2.1
    statement: スロットリングと通信のエラーは、倍々に待って最大 5 回まで再試行する
    category: policy
    applies_to: RetryPolicy
    trigger: DescribeLogStreams または GetLogEvents がエラーを返したとき
    logic: IF エラーの種類が Throttled か Network THEN n 回目の再試行の前に baseDelaysSeconds[n−1] 秒（1・2・4・8・16 秒）× (1 + 乱数 −0.2〜+0.2) だけ待ち、同じ要求（同じ次のトークン）を送り直す。5 回再試行しても失敗したら、その要求を失敗とする。IF それ以外の種類 THEN 再試行せずにすぐ失敗とする
    violation: 失敗の扱いは BR1.5（列挙）と BR3.3（ストリーム）
    source: FR4.5、NFR4、Q1
  - id: BR2.2
    statement: 再試行の回数は、要求（1 回の API 呼び出し）ごとに数える
    category: calculation
    applies_to: RetryPolicy
    trigger: 再試行するとき
    logic: 成功したら次の要求の再試行の回数は 0 から数え直す。StreamFetchOutcome.retryCount には、そのストリームで行った再試行の回数の合計を入れる
    violation: なし
    source: FR4.5、Q1
  - id: BR2.3
    statement: 再試行の待ちの間に中断を指示されたら、待たずにやめる
    category: constraint
    applies_to: RetryPolicy
    trigger: 中断の指示（BR5.5）を受けたとき
    logic: 待ちを打ち切り、送り直さずに取得を止める
    violation: なし
    source: FR4.9（U7 の確認ダイアログから使う中断）、components.md の EventFetcher

  - id: BR3.1
    statement: 対象のストリームを 1 つずつ順に取得する
    category: policy
    applies_to: EventFetcher
    trigger: StreamPlan が決まったとき
    logic: selectedStreams の順に 1 ストリームずつ取得する。同時に複数のストリームは取得しない（並列化は MVP の後、IB-07）
    violation: なし
    source: FR4.1、units-generation の Q1
  - id: BR3.2
    statement: 各ストリームの取得は、U1 のページングのルールに従う
    category: constraint
    applies_to: EventFetcher
    trigger: 各ストリームを取得するとき
    logic: U1:BR3.1（開始・終了時刻と古い順を毎回指定）、U1:BR3.2（ページの終わりの判定と pageCount）、U1:BR3.3（件数の上限なし）をストリームごとに適用する。sequence はストリームごとに 0 から振る
    violation: なし
    source: FR4.3、FR4.4、FR4.6
  - id: BR3.3
    statement: 再試行しても失敗したストリームは、取得できたページを残して失敗とし、次のストリームに進む
    category: policy
    applies_to: StreamFetchOutcome
    trigger: GetLogEvents が再試行（BR2.1）の後もエラーを返したとき
    logic: そのストリームの取得をやめ、status = Failed とし、ApiFailure を付ける。それまでに EventTimeline に足した LogEvent は残す。次のストリームの取得に進む（U1:BR4.1 を複数ストリームに広げる）
    violation: なし
    source: FR4.10

  - id: BR4.1
    statement: ログは、時刻 → ストリーム名 → ストリームの中の順で並べる
    category: calculation
    applies_to: LogEvent
    trigger: LogEvent を EventTimeline に足すとき
    logic: 並べ替えのキーは (timestamp, logStreamName, sequence) の昇順。logStreamName は文字列の昇順で比べる（U1:BR5.1 の (timestamp, sequence) を置き換える）
    violation: なし
    source: FR4.7、Q3
  - id: BR4.2
    statement: 取得したログは、取得したものから逐次、時刻順を保ったまま足す
    category: policy
    applies_to: EventTimeline
    trigger: ページを受けるたび
    logic: 受けたページの LogEvent を BR4.1 の順の正しい位置に入れる。入れた後も、全体が常に BR4.1 の順に並んでいる。件数の上限は設けない
    violation: なし
    source: FR4.7、FR4.6、NFR2
  - id: BR4.3
    statement: 表示範囲の行だけを、位置と件数で取り出せる
    category: calculation
    applies_to: RowWindow
    trigger: 画面が行を求めたとき
    logic: offset から最大 limit 件の LogEvent を BR4.1 の順で返す。offset が保持件数以上なら空。取り出した時点の totalCount と timelineVersion を一緒に返す。100 万件を保持していても、1 回の取り出しは画面のスクロールが 1 秒以内に応えられる速さで返す
    violation: なし
    source: NFR2、NFR3、unit-of-work のログの置き場所（レビュー R-01）
  - id: BR4.4
    statement: (logStreamName, sequence) で 1 件のログを特定でき、その位置を求められる
    category: calculation
    applies_to: EventTimeline
    trigger: 画面が見ている位置を保つとき
    logic: (logStreamName, sequence) を受けたら、そのログのいまの位置（何番目か）を返す。ないときはないと返す
    violation: なし
    source: Q4
  - id: BR4.5
    statement: 新しい取得を始めた時点と中断した時点で、保持ログを破棄する
    category: policy
    applies_to: EventTimeline
    trigger: 取得の開始時（BR5.6）と中断時（BR5.5）
    logic: 保持ログをすべて捨て、timelineVersion を増やす
    violation: なし
    source: components.md の EventTimeline（レビュー R-06）、U1 のコード生成のレビュー R-06

  - id: BR5.1
    statement: 1 回の取得は「列挙と選定 → ストリームごとの取得 → 逐次の追加」の順に進める
    category: policy
    applies_to: FetchCoordinator
    trigger: "[Fetch] を押したとき"
    logic: 新しい FetchJob（status = Running）を作り、BR5.6 を行い、StreamPlanner で StreamPlan を作り（BR1.1〜BR1.5）、EventFetcher で各ストリームを取得し（BR3.1〜BR3.3）、受けたページを EventTimeline に足す（BR4.2）。キャッシュの分岐は U6 で足す
    violation: なし
    source: FR4.1、FR4.7、components.md の FetchCoordinator
  - id: BR5.2
    statement: 取得の結果の状態は、失敗の有無で決める
    category: calculation
    applies_to: FetchJob.status
    trigger: 取得が終わったとき
    logic: IF 中断した THEN Aborted。ELSE IF 列挙が最初のページで失敗し対象が 0 件 THEN Failed。ELSE IF 失敗したストリームがある、または listingStatus = Partial THEN CompletedWithFailures。ELSE Completed（対象が 0 件でも Completed）
    violation: なし
    source: FR4.10、FR4.11
  - id: BR5.3
    statement: 失敗したストリームは、名前と安全な詳細をまとめる
    category: policy
    applies_to: FailedStream
    trigger: ストリームが失敗したとき
    logic: FailedStream（logStreamName、ApiFailure）を FetchJob.failedStreams に足す。ApiFailure の詳細は U1:BR4.3 の安全な詳細だけ（秘密の認証情報とアクセスキー ID を含まない）
    violation: なし
    source: FR4.10、Q5、project.md Forbidden
  - id: BR5.4
    statement: 取得の進み具合を、開始時に渡された受け口に順に届ける
    category: policy
    applies_to: FetchCoordinator
    trigger: 取得の開始から終わりまで
    logic: 受け口に、開始 → 計画したストリーム数 → ページごとの追加（追加件数と累計件数）→ ストリームごとの終了（成功・失敗）→ 終了（FetchJob）の順で届ける（U1:BR4.5 を広げる）。FetchCoordinator は呼び出し元を知らない（ADR-007）
    violation: なし
    source: FR4.7、NFR3、ADR-007
  - id: BR5.5
    statement: 中断を指示されたら、次の API を呼ばずに止め、取得途中の結果を捨てる
    category: policy
    applies_to: FetchCoordinator
    trigger: 呼び出し元が中断を指示したとき（ウィンドウを閉じるとき。確認ダイアログは U7）
    logic: 進行中の待ち（BR2.3）を打ち切り、次の DescribeLogStreams・GetLogEvents を呼ばない。FetchJob.status = Aborted とし、EventTimeline の保持ログを破棄する（BR4.5）。利用者向けの中断の操作は作らない
    violation: なし
    source: FR4.9、components.md の FetchCoordinator（IB-08）
  - id: BR5.6
    statement: 新しい取得を始めた時点で、前回の行・件数・結果を消す
    category: policy
    applies_to: SessionState
    trigger: "[Fetch] を押したとき"
    logic: 取得の最初の API を呼ぶ前に、EventTimeline の保持ログを破棄し（BR4.5）、画面に出している前回の行・件数・失敗の一覧・直近の結果を消す。画面には「取得中」と 0 件を出す
    violation: なし
    source: U1 のコード生成のレビュー R-06

  - id: BR6.1
    statement: ストリーム名の手入力をなくし、[Fetch] を押せる条件からストリーム名を外す
    category: validation
    applies_to: FetchRequest
    trigger: 選択または入力が変わったとき
    logic: "[Fetch] は、プロファイル・リージョン・ロググループが選ばれ（U2:BR2.7）、日時が U1:BR1.2・U1:BR1.3 の検証を通るときだけ押せる。共通の検証（U1:BR1.8）からストリーム名が空でないことの確認を外し、ロググループ名が空でないことの確認は残す"
    violation: "[Fetch] を無効にし、理由の文言キーを validationErrors に入れる"
    source: FR4.1、U1:BR1.1・U1:BR1.8、U2:BR2.7
  - id: BR6.2
    statement: ステータス行に、取得中の進み具合と、取得後の件数・失敗の数を出す
    category: policy
    applies_to: DesktopUi
    trigger: 進み具合を受けたとき
    logic: 取得中は「取得中」と、終えたストリーム数／計画したストリーム数、累計件数を出す。終わったら件数を出し、0 件なら 0 件と文字で出す。失敗したストリームがあれば「N ストリームで失敗」、列挙が途中までなら「ストリームの列挙は途中まで」を出す。Failed なら種類名と安全な詳細を出す（U1:BR4.4 の暫定表示）。Aborted は表示しない（ウィンドウを閉じるときだけ起きるため）
    violation: なし
    source: FR4.8、FR4.10、FR4.11、project.md Corrections（最低限の状態表示）
  - id: BR6.3
    statement: 失敗の数を押すと、失敗したストリームの一覧を出す
    category: policy
    applies_to: SessionState.failureListOpen
    trigger: 失敗の数を押したとき、またはキーボードで開いたとき
    logic: 失敗したストリームの名前と、エラーの種類名・安全な詳細の一覧を出す。列挙の失敗があれば、その種類名・安全な詳細も一覧の先頭に出す。一覧は Escape か [閉じる] で閉じる。取得し直すと一覧は閉じて中身も消える（BR5.6）
    violation: なし
    source: FR4.10、Q5
  - id: BR6.4
    statement: ログ一覧は「時刻 / ストリーム名 / メッセージ」の列で、表示範囲の行だけを描く
    category: policy
    applies_to: DesktopUi
    trigger: 一覧を描くとき
    logic: 画面は保持ログを持たず、表示範囲の行だけを RowWindow で取り寄せて描く（BR4.3）。各行は 1 行の高さで、時刻（U1:BR5.3）、ストリーム名、メッセージ（U1:BR5.2 の 1 行表示）を出す。スクロールの範囲は totalCount から決める
    violation: なし
    source: FR4.7、NFR2、NFR3、unit-of-work のログの置き場所
  - id: BR6.5
    statement: 取得中に行が増えても、画面の一番上に見えている行を動かさない
    category: policy
    applies_to: ViewportAnchor
    trigger: timelineVersion が変わったとき
    logic: IF 一番上までスクロールしている THEN 一番上のままにする。ELSE 変わる前に一番上に見えていた行の (logStreamName, sequence) の新しい位置を BR4.4 で求め、その行が同じ高さに見えるようにスクロールの位置を合わせる
    violation: なし
    source: FR4.7、Q4
  - id: BR6.6
    statement: 取得中は、取得条件と [Fetch] を変えられない
    category: constraint
    applies_to: SessionState
    trigger: phase が Fetching の間
    logic: プロファイル・リージョン・ロググループの選択、日時の入力、[再読み込み]、[Fetch] を無効にし、ステータス行に取得中であることを出す（U1:BR1.4・U2:BR2.6 のとおり）。ロググループ一覧の絞り込みと、ログ一覧のスクロールは使える
    violation: 操作を受け付けない
    source: FR4.8、NFR3
  - id: BR6.7
    statement: 画面からの操作は、受け取った順に処理する
    category: constraint
    applies_to: AppSession
    trigger: 画面から操作が届いたとき
    logic: ロググループ一覧の絞り込みとロググループの選択は、ほかの処理を待たずに受け取った順に処理し、その結果を画面に返す。保持ログに触れる操作（接続の変更・取得の開始）だけが、保持ログの処理を待つ
    violation: なし
    source: U2 のコード生成のレビュー R-06
  - id: BR6.8
    statement: U3 の画面の文字列も文言キーから英日を引き、キーボードだけで操作できる
    category: policy
    applies_to: DesktopUi
    trigger: 常に
    logic: 列の見出し、進み具合、失敗の数、列挙が途中までの知らせ、失敗の一覧と [閉じる] の文言キーを英日で足す（U1:BR6.1）。ログ一覧は上下の矢印キー・Page Up・Page Down・Home・End でスクロールでき、失敗の数は Tab で選んで Enter かスペースで一覧を開ける。一覧を開いたときのフォーカスは [閉じる]（U1:BR6.2）
    violation: なし
    source: NFR12、NFR13
  - id: BR6.9
    statement: 確認用プログラムも、ロググループ全体を取得する
    category: policy
    applies_to: 確認用プログラム
    trigger: 手元で実行したとき
    logic: 引数から --stream をなくす。取得はこの単位の流れ（BR5.1）を使う。1 件 1 行で「UTC の時刻（ミリ秒まで）＋タブ＋ストリーム名＋タブ＋メッセージ」を BR4.1 の順に標準出力に出し、最後に件数・ストリーム数・失敗の数を標準エラーに出す。失敗したストリームがあれば、名前と種類・安全な詳細を標準エラーに出し、終了コード 1 で終わる。自動テストと CI からは実行しない（U1:BR7.2）
    violation: 引数の誤りは使い方を表示して終了コード 2
    source: U1:BR7.1、team.md Walking Skeleton
```

## まとめ

| ID | ルール | 種類 | 出典 |
|----|--------|------|------|
| BR1.1 | 最後のイベント時刻の新しい順に列挙 | constraint | FR4.1、Q2 |
| BR1.2 | 「開始 − 1 時間」より古いストリームで列挙をやめる | policy | FR4.2、Q2 |
| BR1.3 | 範囲と重なるストリームと時刻なしを対象にする | calculation | FR4.2 |
| BR1.4 | DescribeLogStreams を足す、読み取り 3 API だけ | constraint | NFR6 |
| BR1.5 | 列挙の途中のエラーは、選んだ分だけ取得 | policy | FR4.10 |
| BR2.1 | スロットリング・通信のエラーは倍々に待って最大 5 回 | policy | FR4.5、Q1 |
| BR2.2 | 再試行は要求ごとに数える | calculation | FR4.5 |
| BR2.3 | 待ちの間の中断ですぐやめる | constraint | FR4.9 |
| BR3.1 | ストリームを 1 つずつ順に取得 | policy | FR4.1 |
| BR3.2 | 各ストリームは U1 のページングのルール | constraint | FR4.3、FR4.4 |
| BR3.3 | 失敗したストリームは取得分を残して次へ | policy | FR4.10 |
| BR4.1 | 時刻 → ストリーム名 → ストリームの中の順 | calculation | FR4.7、Q3 |
| BR4.2 | 逐次、時刻順を保って足す | policy | FR4.7 |
| BR4.3 | 表示範囲の行だけを取り出す | calculation | NFR2 |
| BR4.4 | 1 件の位置を求める | calculation | Q4 |
| BR4.5 | 取得の開始と中断で保持ログを破棄 | policy | R-06 |
| BR5.1 | 列挙と選定 → 取得 → 追加の順 | policy | FR4.1 |
| BR5.2 | 結果の状態の決め方 | calculation | FR4.10、FR4.11 |
| BR5.3 | 失敗したストリームのまとめ | policy | FR4.10、Q5 |
| BR5.4 | 進み具合を受け口に届ける | policy | FR4.7、ADR-007 |
| BR5.5 | 中断で止めて結果を捨てる | policy | FR4.9 |
| BR5.6 | 取得の開始時に前回の行と件数を消す | policy | U1 の R-06 |
| BR6.1 | ストリーム名の手入力をなくす | validation | FR4.1 |
| BR6.2 | ステータス行の進み具合・件数・失敗 | policy | FR4.8、FR4.10、FR4.11 |
| BR6.3 | 失敗したストリームの一覧 | policy | FR4.10、Q5 |
| BR6.4 | 3 列で表示範囲だけ描く | policy | FR4.7、NFR2 |
| BR6.5 | 一番上に見えている行を動かさない | policy | Q4 |
| BR6.6 | 取得中は取得条件を変えられない | constraint | FR4.8 |
| BR6.7 | 画面からの操作は受け取った順に処理 | constraint | U2 の R-06 |
| BR6.8 | 文言とキーボード操作 | policy | NFR12、NFR13 |
| BR6.9 | 確認用プログラムもロググループ全体 | policy | U1:BR7.1 |
