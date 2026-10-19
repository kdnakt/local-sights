# Business Rules — U1 薄い一本（u1-filter-skeleton）

上流：`inception/units-generation/unit-of-work.md`（U1）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1、FR2.1・FR2.4〜FR2.6、FR3.1・FR3.2、FR4.1、FR5.1〜FR5.6、FR6.3・FR6.4、FR7.2、FR7.6、NFR2・NFR5）、`inception/domain-design/components.md`・`decisions.md`（ADR-001・ADR-002・ADR-005・ADR-007・ADR-008）、`inception/refined-mockups/interaction-spec.md`・`mockups.md`、質問票 `functional-design-questions.md`（Q2〜Q6、Q8）、AWS の資料（filter、Boolean・comparison 演算子、Datetime functions）、今のコード（`coordinator.rs` の run_fetch・run_fetch_with_cache・decide_job_status、`session.rs` の can_fetch・can_change_connection・has_logs、`time_range.rs` の parse_in_zone）。

## 上流との違いと、U1 だけの暫定の動き

- **実行を始めると前の結果を消す**：interaction-spec は「前の結果は次の実行の Fetching・Evaluating の間も残す」としている。U1 の QueryStore は実行のたびに中身を捨てて取り直す（units Q2a）ため、前の結果の行が指すイベントがなくなる。U1 では取得の段に入るときに前の結果を消し、結果の囲みに進み具合を出す（BR2.9）。U3 で取得済みの範囲を覚えて捨てなくなったら、interaction-spec のとおり前の結果を残す形に戻す。読み取りのエラー（QueryError）のときは取得に入らないため、前の結果を残す（interaction-spec のとおり）。
- **コンソールの細部のうち U1 で仮に決めるもの**：言葉の大文字・小文字（BR3.3）、優先順位（BR3.4）、文字列の項目と数値の比較（BR5.1）、`sort` がないときの並び順（BR6.3）は、AWS の資料に書かれていない。U1 では一般的な決まりで仮に作り、U2 の機能設計（OQ2）と U4 の確認用プログラムでコンソールに合わせる。
- **`@log` の値**：コンソールの `@log` は「アカウント ID:ロググループ名」の形だが、アカウント ID を知るには読み取りの 3 つ以外の API（GetCallerIdentity）が要り、呼べない（NFR5、project.md Forbidden）。`@log` の値はロググループ名だけとする（BR4.1）。`@log` の値を比べるクエリは、互換の確かめ（SM1）の組に入れない。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 左端のナビゲーションで Log Management と Log Analytics を切り替える。起動時は Log Management で、Log Management の画面の中身と振る舞いは変えない
    category: policy
    applies_to: AppNavigation、AppShell
    trigger: ナビゲーションの項目を選んだとき、Ctrl+1／Ctrl+2（macOS は Cmd+1／Cmd+2）を押したとき
    logic: IF 項目を選んだ THEN その画面を出し、行き先の画面の最初の欄（Log Management はロググループの検索、Log Analytics はロググループの選択）に焦点を移して、画面名を読み上げる。ナビゲーションの開閉の状態はアプリを閉じるまで覚える。取得中・評価中でも切り替えられ、切り替えても取得と評価は止まらない
    violation: なし
    source: FR1.1、FR1.8、refined-mockups の SideNavigation
  - id: BR1.2
    statement: クエリの画面の入力（ロググループ、期間、クエリの文）と実行の状態は、ログ管理の画面とは別に持ち、画面を行き来しても残す
    category: policy
    applies_to: InsightsSession（AnalyticsInput、InsightsRun）
    trigger: 画面を切り替えたとき
    logic: 画面の切り替えでは何も捨てない。クエリの画面の入力はログ管理の画面の入力を変えず、ログ管理の画面の入力もクエリの画面の入力を変えない。タイムゾーンの選択だけはアプリに 1 つで、両方の画面に効く
    violation: なし
    source: FR1.2、RA Q3b、refined-mockups の 2 つの画面の約束
  - id: BR1.3
    statement: ロググループは、今の接続の一覧にあるものから 1 つだけ選べる（U1）
    category: validation
    applies_to: LogGroupMultiselect、InsightsSession
    trigger: ロググループを選んだとき
    logic: IF 選んだ名前が今の接続の一覧にある THEN それを選んだロググループにする（すでに 1 つ選んでいれば置き換える）。一覧は Log Management の左ペインと同じもの（今の LogGroupListing）を使う。50 個までの複数選択は U3
    violation: 一覧にない名前は受け付けない（今の UnknownLogGroup と同じ）
    source: FR1.3、ADR-006
  - id: BR1.4
    statement: 期間は、今のログ管理の画面と同じ決まりで入れて検証する
    category: validation
    applies_to: InsightsSession（今の DateTimeInput・TimeRangeModel を使う）
    trigger: 開始・終了を変えたとき、タイムゾーンを変えたとき
    logic: 絶対日時、秒まで、選んでいるタイムゾーンで読む、終了はその秒の 999 ms まで含む、開始は終了より前。曖昧な時刻と存在しない時刻の扱いも今の parse_in_zone のとおり（曖昧なら早い方、存在しなければ不備）
    violation: 欄の直下に理由を出し、[Run query] を押せない（BR1.5）
    source: FR1.4、261002 の FR3
  - id: BR1.5
    statement: '[Run query] は次のすべてを満たすときだけ押せる。満たさないときは押せない状態で、最初に当てはまる理由を文字で示す'
    category: constraint
    applies_to: InsightsSession、AppSession
    trigger: 画面の状態を作るとき
    logic: 理由を次の順に調べる。(1) 接続が選ばれていない (2) ロググループが選ばれていない (3) 期間に不備がある (4) クエリの文が空白だけ (5) クエリの文が 10,000 文字を超える (6) クエリの画面で実行中（Fetching・Evaluating） (7) ログ管理の画面で取得中（BR2.3） (8) 接続の変更の確認待ち、設定の画面が開いている、SSO のログインが走っている（今の can_change_connection と同じ）。どれにも当たらなければ押せる。押した時点でもう一度同じ順で調べ、当たれば実行しない
    violation: 押せない。理由の文を出す
    source: FR1.6、Q4、ADR-005
  - id: BR1.6
    statement: クエリは [Run query] か Cmd+Enter（Windows は Ctrl+Enter）を押したときだけ実行する
    category: policy
    applies_to: QueryEditor、LogAnalyticsPage
    trigger: クエリの文を変えたとき、キーを押したとき
    logic: 文を変えても実行しない。IF キーの組み合わせを押した AND BR1.5 で押せる THEN [Run query] と同じく実行する。押せないときは何もしない
    violation: なし
    source: FR1.7、RA Q4
  - id: BR1.7
    statement: クエリの画面の実行は、ログ管理の画面の保持・表示・部分一致の絞り込みを変えない
    category: constraint
    applies_to: QueryRunner、QueryStore、FetchCoordinator、AppSession
    trigger: クエリの画面で実行したとき
    logic: クエリの画面の取得の行き先は QueryStore だけ。ログ管理の画面の EventTimeline・LogView・FilterEngine・状態（Phase、件数、失敗の一覧、中止の知らせ）には書かない。ログ管理の画面の部分一致の文字列は、クエリの評価に使わない
    violation: なし（作りで守る。テストで確かめる）
    source: FR1.8、FR2.1、C5

  - id: BR2.1
    statement: クエリの画面の取得済みのログは QueryStore だけが持ち、実行のたびにクエリの画面の分だけを捨てて取り直す（U1）
    category: policy
    applies_to: QueryStore、QueryRunner
    trigger: 読み取りが通り、取得の段に入るとき
    logic: QueryHoldings の中身を捨て、epoch を 1 増やし、key を（今のプロファイル, 今のリージョン, 選んだロググループ）にしてから、期間全体を取得する。取得済みの範囲は覚えない（U3 で覚える）
    violation: なし
    source: FR2.1、units Q2a、ADR-001
  - id: BR2.2
    statement: クエリの画面の取得は、今のログ管理の画面の取得と同じ流れと決まりで行い、行き先だけを QueryStore にする
    category: policy
    applies_to: FetchCoordinator（今の run_fetch_with_cache を、行き先に QueryStore を渡して使う）
    trigger: 取得の段
    logic: ストリームの選び方、同時に読む本数（8 本）、呼び出しの速さの上限と引き下げ、再試行、あきらめの決まりは今のまま。ディスクキャッシュが有効なら、今と同じく、期間全体がキャッシュの記録範囲に入るときはキャッシュから読み、取得したログは今の決まりでキャッシュに書く。記録範囲の一部だけを使う照会は U3
    violation: なし
    source: FR2.2（期間全体の場合）、FR2.4、units Q2a
  - id: BR2.3
    statement: 取得は、2 つの画面を合わせて同時に 1 つだけ走る
    category: constraint
    applies_to: AppSession（FetchOwner）
    trigger: どちらかの画面で取得を始めるとき
    logic: IF FetchOwner が None THEN 始めた画面を FetchOwner にして始める。ELSE 始めない。FetchOwner が LogAnalytics の間は、ログ管理の画面の [Fetch] を押せず「Log Analytics でクエリを実行中」と理由を出す。LogManagement の間は、クエリの画面の [Run query] を押せず「Log Management で取得中」と理由を出す。取得が終わったら（どの終わり方でも）None に戻す。評価の段は AWS を呼ばないため FetchOwner を持たないが、クエリの画面の実行中（BR1.5 の (6)）として扱う
    violation: 押せない（理由の文を出す）。押せない状態で届いた取得の頼みは断る（今の SessionError と同じ扱い）
    source: ADR-005、domain Q7
  - id: BR2.4
    statement: 取得中と評価中は、処理中であることと進み具合を結果の囲みに示し、[Cancel] で止められる
    category: policy
    applies_to: InsightsSession、LogAnalyticsPage、QueryResultTable
    trigger: Fetching・Evaluating の間
    logic: Fetching は「Fetching logs」と、読み終えたストリームの数／計画した数（一覧の前は一覧のページ数）と取得した件数、経過時間を出す。Evaluating は「Running query」と、評価を終えた件数／全体の件数を出す。[Run query] は押せず、[Cancel] を出す。ロググループ・期間・クエリの欄は書き換えられない（クエリの欄は読むだけ）
    violation: なし
    source: FR2.4、project.md Corrections（最低限の状態表示）、interaction-spec の RunBar
  - id: BR2.5
    statement: 一部のストリームが失敗したときは、取得できた分で評価し、一部のログだけの結果であることと失敗したストリームを示す
    category: policy
    applies_to: QueryRunner、InsightsSession、LogAnalyticsPage
    trigger: 取得が CompletedWithFailures で終わったとき（今の decide_job_status のとおり。一覧が途中で失敗したときを含む）
    logic: 評価に進み、partial を true にする。結果の囲みの上に警告を出し、失敗したストリームの一覧（ストリーム名と、安全な理由だけ）を開けるようにする。失敗の一覧の形は Log Management の失敗の一覧と同じ
    violation: なし
    source: FR2.5、interaction-spec の画面 6c
  - id: BR2.6
    statement: 取得全体が失敗したときは評価せず、エラーを示し、前の結果を消す
    category: policy
    applies_to: QueryRunner、InsightsSession
    trigger: 取得が Failed で終わったとき
    logic: 段階を FetchFailed にし、結果の囲みの上にエラーのアラートを出す。理由は今のログ管理の画面と同じ ApiFailure の種類と安全な詳細だけで、秘密の認証情報とアクセスキー ID は出さない。権限の不足・SSO の期限切れの案内も今と同じ
    violation: なし
    source: interaction-spec の画面 6b、FR6.4、project.md Forbidden
  - id: BR2.7
    statement: '[Cancel] は取得と評価の両方を止め、評価しない。止めたことを示す'
    category: policy
    applies_to: QueryRunner、FetchCoordinator、QueryEvaluator、InsightsSession
    trigger: Fetching・Evaluating の間に [Cancel] を押したとき
    logic: Fetching の間は、今のログ管理の画面の [Cancel] と同じ止め方（新しい呼び出しを始めず、出ている応答は待つ）で取得を止め、評価に進まない。Evaluating の間は、次の区切り（BR5.8）で評価を止め、途中までの結果は捨てる。どちらも段階を Cancelled にし、結果の囲みに「Query cancelled」と出す。止める前に取得した分は QueryStore に残るが、U1 では次の実行で捨てて取り直す（取得済みとして使うのは U3）
    violation: なし
    source: FR2.6、interaction-spec の画面 6d
  - id: BR2.8
    statement: クエリの画面の取得中にアプリを閉じようとしたときは、今と同じ閉じる確認を出す
    category: policy
    applies_to: AppSession（request_close）
    trigger: アプリを閉じようとしたとき
    logic: IF FetchOwner が None でない（どちらの画面の取得でも） THEN 今の閉じる確認を出す。閉じたら取得を Close で止める（今と同じく Aborted）。評価だけが走っているときは確認を出さずに閉じる（手元の処理だけで、失うのは画面の結果だけのため）
    violation: なし
    source: ADR-005、今の閉じる確認（261012-parallel-fetch U5）
  - id: BR2.9
    statement: U1 では、取得の段に入るときに前の結果を消す
    category: policy
    applies_to: InsightsSession、QueryResultTable
    trigger: 読み取りが通り、取得の段に入るとき
    logic: 前の結果（行・件数・一部失敗の警告）を消し、結果の囲みに BR2.4 の進み具合を出す。読み取りのエラーで取得に入らないときは前の結果を残す
    violation: なし
    source: units Q2a（実行のたびに捨てて取り直す）、interaction-spec からの暫定の違い（この文書の冒頭）

  - id: BR3.1
    statement: クエリの文は、`|` でつないだ命令の並びとして読む
    category: validation
    applies_to: QueryParser
    trigger: 実行したとき
    logic: 空白・改行は言葉の区切りとしてだけ扱う。`#` から行の終わりまでは、引用符と正規表現の外にあるときだけ注記として読み飛ばす。`|` の前後に命令がない（先頭・末尾・`||`）ときは Syntax
    violation: QueryProblem（Syntax、場所つき）
    source: FR3.1
  - id: BR3.2
    statement: U1 で読める命令は `filter` だけ。filter を 2 つ以上つないだときは、すべての条件を満たすものだけを残す
    category: validation
    applies_to: QueryParser
    trigger: 命令の位置の言葉を読んだとき
    logic: IF 言葉が filter THEN 後ろの条件を読む（条件がなければ Syntax）。ELSE IF コンソールの命令の名前（fields、sort、limit、stats、parse、dedup、display、pattern、diff、unmask、unnest、lookup、filterIndex、SOURCE など）THEN Unsupported（言葉と場所つき。fields・sort・limit は U2 で読めるようにする）。ELSE Syntax（命令として読めない）
    violation: QueryProblem
    source: FR3.1、FR3.2、FR6.1（U1 は対応外の検出だけ）、範囲外の表
  - id: BR3.3
    statement: 命令の名前と、and・or・not・like・in の言葉は、大文字・小文字を区別せずに読む（U1 の仮の決まり）
    category: validation
    applies_to: QueryParser
    trigger: 言葉を読んだとき
    logic: filter・FILTER・Filter をどれも filter と読む。AWS の資料に `IN` の例があるため区別しない側にそろえた。項目の名前（@timestamp など）は区別する。コンソールとの違いは U2（FR3.8、OQ2）と U4 で確かめて合わせる
    violation: なし
    source: FR3.8（U2）、AWS の資料（filter の Indexed fields の節）
  - id: BR3.4
    statement: 条件の優先順位は、強い順に「比べる・like・=~・in」「not」「and」「or」とし、括弧で変えられる（U1 の仮の決まり）
    category: validation
    applies_to: QueryParser
    trigger: 条件を読んだとき
    logic: '`not a and b` は `(not a) and b`、`a or b and c` は `a or (b and c)` と読む。閉じていない括弧、対応のない閉じ括弧は Syntax（開いた括弧の場所を示す）。コンソールとの違いは U2（OQ2）と U4 で確かめる'
    violation: QueryProblem（Syntax）
    source: FR5.2
  - id: BR3.5
    statement: 値は、文字列・数値・正規表現・値の並びの 4 つの書き方で読む
    category: validation
    applies_to: QueryParser
    trigger: 値の位置を読んだとき
    logic: 文字列は "..." か '...' で、中の \" \' \\ は 1 文字として解く。数値は整数か小数で、前に - を付けてよい。正規表現は /.../ で、中の \/ は / として解く。値の並びは [値, 値, ...] で 1 つ以上（in の後ろだけ）。閉じていない引用符・/・[ は Syntax（開いた場所を示す）
    violation: QueryProblem（Syntax）
    source: FR5.3、FR5.4、AWS の資料（filter）
  - id: BR3.6
    statement: U1 の条件に書ける項目は @timestamp・@message・@logStream・@log と toMillis(@timestamp) だけ
    category: validation
    applies_to: QueryParser
    trigger: 項目の位置を読んだとき
    logic: 項目の名前は大文字・小文字を区別する。IF 4 つのどれか THEN その項目。ELSE IF @ で始まるほかの名前（@ingestionTime など）か、@ で始まらない名前（JSON の項目）THEN Unsupported（U1 ではまだ使えない項目。JSON の項目は U2）
    violation: QueryProblem（Unsupported）
    source: FR4.1、units レビュー R-01
  - id: BR3.7
    statement: 関数は toMillis(@timestamp) だけを読み、ほかの関数は対応外とする
    category: validation
    applies_to: QueryParser
    trigger: 名前のすぐ後ろに ( が来たとき
    logic: IF 名前が toMillis（大文字・小文字を区別する）AND 引数が @timestamp の 1 つ THEN TimestampMillis の項目。ELSE IF 名前が toMillis で引数が違う THEN Unsupported（toMillis は @timestamp だけ）。ELSE Unsupported（関数の名前と場所つき。ispresent・isempty・now など）
    violation: QueryProblem（Unsupported）
    source: FR5.5、Q2
  - id: BR3.8
    statement: 正規表現は読み取りのときに作り、作れなければ書き間違いとする
    category: validation
    applies_to: QueryParser
    trigger: 正規表現か like の値を読んだとき
    logic: 入力の長さに比例する時間で一致を調べる正規表現の仕組みで作る。書き方の誤り、仕組みが対応しない書き方（後方参照・先読みなど）、大きさの上限（仕組みの既定）を超えたものは Syntax とし、正規表現の場所を示す
    violation: QueryProblem（Syntax）
    source: ADR-002、Q4
  - id: BR3.9
    statement: 読み取りのエラーのときは、取得も評価もせず、クエリの文と前の結果を残して、場所を示す
    category: policy
    applies_to: QueryRunner、InsightsSession、QueryEditor
    trigger: QueryParser が QueryProblem を返したとき
    logic: 段階を QueryError にして QueryProblem を持つ。AWS は呼ばない。クエリの文は消さない。前の結果は残す。クエリ欄の直下にエラーの文（対応外か書き間違いかで文言を分ける）と行・列を出し、焦点をクエリ欄に置いてカーソルをエラーの場所に動かす。文言は英語と日本語で、クエリの言葉と場所だけを出す
    violation: なし
    source: FR6.1（U1 の範囲）、FR6.2（U1 の範囲）、FR6.3、FR6.4

  - id: BR4.1
    statement: 決まった項目は、イベントと評価の入口が渡すロググループ名から読む
    category: calculation
    applies_to: EventFieldReader（決まった項目の部分）
    trigger: 条件を評価するとき
    logic: '@timestamp と toMillis(@timestamp) はイベントの時刻（エポックからのミリ秒の数値）。@message はメッセージ全体（改行を含む、取得したままの文字列）。@logStream はストリーム名。@log は EvalItem のロググループ名（コンソールの「アカウント ID:」は付けない）。U1 の 4 つの項目は、どのイベントにも必ず値がある'
    violation: なし
    source: FR4.1、NFR5（アカウント ID を取る API を呼ばない）

  - id: BR5.1
    statement: 比べる演算子（= != < <= > >=）は、項目の値と値の型で次のとおり比べる
    category: calculation
    applies_to: QueryEvaluator
    trigger: Compare の条件を評価するとき
    logic: '@timestamp（数値）と数値：数として比べる。@timestamp と日時の文字列：BR5.9 でミリ秒に直した値と数として比べる。文字列の項目（@message・@logStream・@log）と文字列：= と != は文字列全体が一致するか（大文字・小文字を区別）、< <= > >= は文字の並びの順（Unicode のスカラー値の順）で比べる。文字列の項目と数値：U1 では条件を満たさないとする（!= は満たす）。型の違う値の比べ方は U2 でコンソールに合わせる（FR4.5、OQ2）。値が正規表現のときは Syntax（読み取りのときに返す）'
    violation: なし（評価ではエラーにしない）
    source: FR5.1、FR4.5（U2）
  - id: BR5.2
    statement: and・or・not は、真偽の論理で組み合わせる
    category: calculation
    applies_to: QueryEvaluator
    trigger: And・Or・Not の条件を評価するとき
    logic: And は両方を満たすとき、Or はどちらかを満たすとき、Not は満たさないときに満たす。左から評価し、結果が決まったら右は評価しない
    violation: なし
    source: FR5.2
  - id: BR5.3
    statement: like と not like に文字列を書いたときは、その文字列を含むかで判定する
    category: calculation
    applies_to: QueryEvaluator
    trigger: Like の条件（値が文字列）を評価するとき
    logic: 項目の値（文字列）に値の文字列が含まれていれば満たす。大文字・小文字を区別する。not like は逆。@timestamp には使えない（BR5.6）
    violation: なし
    source: FR5.3、AWS の資料（filter：3 つの例はどれも大文字・小文字を区別）
  - id: BR5.4
    statement: like /…/・not like /…/・=~ /…/ は、正規表現に一致する部分があるかで判定する
    category: calculation
    applies_to: QueryEvaluator、QueryParser
    trigger: Like（値が正規表現）・Match の条件を評価するとき
    logic: 項目の値のどこかに一致すれば満たす（全体の一致は求めない）。大文字・小文字は区別し、(?i) を前に書けば区別しない。=~ の後ろは正規表現だけで、文字列を書いたら Syntax。not like は逆
    violation: QueryProblem（Syntax、=~ に文字列のとき）
    source: FR5.3、AWS の資料（filter の Match substrings）
  - id: BR5.5
    statement: in [値, ...] は、項目の値が並びのどれかと等しいときに満たし、not in は逆
    category: calculation
    applies_to: QueryEvaluator
    trigger: In の条件を評価するとき
    logic: 並びのそれぞれと BR5.1 の = で比べ、1 つでも等しければ満たす。文字列は全体の一致だけ（部分の一致はしない）。並びに数値と文字列が混じってよい
    violation: なし
    source: FR5.4、AWS の資料（in は文字列全体が一致するもの）
  - id: BR5.6
    statement: '@timestamp と toMillis(@timestamp) には、like・not like・=~ を使えない'
    category: validation
    applies_to: QueryParser
    trigger: '@timestamp に文字の一致の演算子を書いたとき'
    logic: Unsupported（演算子と場所つき）とし、比べる演算子か in を使うように示す
    violation: QueryProblem（Unsupported）
    source: FR5.6
  - id: BR5.7
    statement: 対応外の演算子は対応外として返す
    category: validation
    applies_to: QueryParser
    trigger: 条件の中で演算子の位置に読めない記号や言葉が来たとき
    logic: 'IF 算術の演算子（+ - * / ^ %）か、コンソールにあってアプリにない比べ方 THEN Unsupported（演算子と場所つき）。ELSE Syntax'
    violation: QueryProblem
    source: FR6.1（U1 の範囲）、SM2
  - id: BR5.8
    statement: 評価は決まった件数ずつ区切って進め、区切りごとに止める信号と古さを確かめる
    category: constraint
    applies_to: QueryEvaluator、QueryRunner
    trigger: 評価の段
    logic: QueryHoldings の時刻順に、1 区切り 10,000 件ずつ評価する（数は作る段階で NFR1・NFR2 を測って変えてよい）。区切りの間に、[Cancel]（BR2.7）、新しい実行・接続の変更（runId と epoch が変わった）を確かめ、当たれば止めて途中までの結果を捨てる。区切りの間は QueryStore を握り続けず、画面は操作に反応し続ける。進み具合（BR2.4）は区切りごとに知らせる
    violation: なし
    source: NFR2、FR7.6、components.md の QueryEvaluator（今の FilterEngine と同じ形）
  - id: BR5.9
    statement: '@timestamp と比べる文字列は、決まった 2 つの書き方だけをミリ秒に直す'
    category: calculation
    applies_to: QueryRunner（実行の時点のタイムゾーンで直す）、QueryParser（書き方を確かめる）
    trigger: '@timestamp・toMillis(@timestamp) と文字列を比べる条件（Compare と In）を読んだとき'
    logic: '`YYYY-MM-DD HH:MM:SS` と `YYYY-MM-DD HH:MM:SS.sss` は、実行した時点で選んでいるタイムゾーンで読む（曖昧な時刻は早い方、存在しない時刻は Syntax。今の parse_in_zone と同じ）。`YYYY-MM-DDTHH:MM:SSZ` と `YYYY-MM-DDTHH:MM:SS.sssZ` は UTC で読む（Z は大文字だけ）。ミリ秒を省いたときはその秒の 0 ミリ秒。ミリ秒は 3 桁だけ。月日・時分秒が範囲の外（13 月、25 時など）か、ほかの書き方（yesterday、2025/08/24、+09:00 付きなど）のときは Syntax とし、文字列の場所と、受け付ける 2 つの書き方を示す。期間の欄とは違い、終了の 999 ms の決まりは当てはめない'
    violation: QueryProblem（Syntax）
    source: FR5.6、Q2a、Q5、Q6
  - id: BR5.10
    statement: '@timestamp はミリ秒の数値とそのまま比べられる'
    category: calculation
    applies_to: QueryEvaluator
    trigger: '@timestamp・toMillis(@timestamp) と数値を比べる条件を評価するとき'
    logic: '`@timestamp > 1756053806000` と `toMillis(@timestamp) > 1756053806000` は同じ結果になる。小数の数値とも数として比べる'
    violation: なし
    source: FR5.6、Q2（timestamp-check.sh で両方の書き方が同じ件数になることを確かめた）

  - id: BR6.1
    statement: 結果が 0 件のときは 0 件であることを示し、取得や評価の失敗と区別する
    category: policy
    applies_to: QueryResultTable、InsightsSession
    trigger: Done で行が 0 件のとき
    logic: 結果の囲みに「No matching events」を出す。まだ実行していないとき（No results yet）、処理中（BR2.4）、取得の失敗（BR2.6）、読み取りのエラー（BR3.9）、止めた（BR2.7）とは別の表示にする。一部失敗で 0 件のときは、警告（BR2.5）と 0 件の両方を出す
    violation: なし
    source: FR7.2
  - id: BR6.2
    statement: 結果は最後に始めた実行のものだけを出す
    category: constraint
    applies_to: InsightsSession、QueryRunner、TauriApp
    trigger: 進み具合・結果の知らせが届いたとき
    logic: 知らせに runId を付け、今の InsightsRun の runId と違えば捨てる。実行中は新しい実行を始められない（BR1.5 の (6)）
    violation: なし（古い知らせは黙って捨てる）
    source: FR7.6
  - id: BR6.3
    statement: U1 の結果の表は決まった列で、取得したログの時刻順に出す。件数の上限は設けない
    category: policy
    applies_to: QueryResultTable、QueryEvaluator
    trigger: Done のとき
    logic: 列は @timestamp（選んでいるタイムゾーン、今のログ管理の画面と同じ書き方）と @message（1 行目、今のログ管理の画面と同じ省き方）。行を開くと、メッセージ全体と @logStream・@log を出す。並びは QueryHoldings の時刻順（時刻、ストリーム名、ストリーム内の順番の昇順）。結果の件数を見出しに出す。条件に合うものはすべて出す。fields の列、sort がないときのコンソールの並び順、limit と既定の上限は U2
    violation: なし
    source: FR7.2、Q3、FR3.5・FR3.7・FR7.1（U2）
  - id: BR6.4
    statement: 結果の行は、見えている分だけを runId と epoch を添えて読み出す
    category: constraint
    applies_to: TauriApp、ApiBridge、QueryResultTable
    trigger: 結果の表をスクロールしたとき
    logic: 画面は（runId, epoch, 先頭, 件数）で行を頼み、アプリは今の QueryResult と QueryHoldings の epoch が一致するときだけ、その範囲の行（時刻・メッセージ・ストリーム名・ロググループ名）を返す。一致しなければ空を返し、画面は新しい状態を待つ。スクロールの決まり（見えている行だけを読む、↑↓ で行を選ぶ）はログ管理の画面と同じ
    violation: なし
    source: FR7.5（U2 の前提）、FR7.6

  - id: BR7.1
    statement: 接続を変えるときは、ログ管理の画面に表示中のログがあるか、クエリの画面に結果の行があるときに確認を出す
    category: policy
    applies_to: AppSession
    trigger: プロファイルかリージョンを選んだとき
    logic: IF ログ管理の画面の件数が 1 件以上（今の has_logs）OR クエリの画面が Done で行が 1 件以上 THEN 確認ダイアログを出し、両方の画面のログと結果が消えることを文で示す。ELSE すぐに変える。確認の作り（変える・やめる）は今と同じ
    violation: なし
    source: ADR-007、Q8、units レビュー R-03
  - id: BR7.2
    statement: 接続を変えたら、クエリの画面の保持と選択と結果を捨て、クエリの文と期間は残す
    category: policy
    applies_to: AppSession、QueryStore、InsightsSession
    trigger: 接続の変更が決まったとき（確認で変えるを選んだときを含む）
    logic: ログ管理の画面は今と同じく捨てる。QueryStore の中身を捨てて epoch を 1 増やし、key をなくす。クエリの画面のロググループの選択を外し、InsightsRun を Idle に戻して結果・エラー・一部失敗の警告を消す。クエリの文と期間の入力は残す
    violation: なし
    source: A2、ADR-008
  - id: BR7.3
    statement: どちらかの画面の取得中・評価中は、接続を変えられない
    category: constraint
    applies_to: AppSession
    trigger: プロファイルかリージョンを選ぼうとしたとき
    logic: 今の can_change_connection の「取得中は変えられない」を、FetchOwner が None でないとき、またはクエリの画面が Evaluating のときに広げる
    violation: 選べない（今と同じく押せない見た目）
    source: ADR-005、ADR-007
```

## まとめ

| ID | 決まり | 種類 | 出典 |
|---|---|---|---|
| BR1.1 | ナビゲーションで 2 つの画面を切り替える | policy | FR1.1、FR1.8 |
| BR1.2 | クエリの画面の入力と状態は別に持つ | policy | FR1.2、RA Q3b |
| BR1.3 | ロググループは 1 つだけ（U1） | validation | FR1.3 |
| BR1.4 | 期間は今と同じ決まり | validation | FR1.4 |
| BR1.5 | [Run query] を押せる条件と理由 | constraint | FR1.6、Q4 |
| BR1.6 | 実行はボタンとキーだけ | policy | FR1.7 |
| BR1.7 | ログ管理の画面を変えない | constraint | FR1.8、FR2.1 |
| BR2.1 | QueryStore を実行のたびに捨てて取り直す | policy | FR2.1、units Q2a |
| BR2.2 | 取得は今と同じ流れ、行き先だけ QueryStore | policy | FR2.2、FR2.4 |
| BR2.3 | 取得は 2 つの画面で同時に 1 つ | constraint | ADR-005 |
| BR2.4 | 処理中の表示と [Cancel] | policy | FR2.4 |
| BR2.5 | 一部失敗は取得できた分で評価して示す | policy | FR2.5 |
| BR2.6 | 全体の失敗は評価せずエラー | policy | interaction-spec |
| BR2.7 | [Cancel] は評価しない | policy | FR2.6 |
| BR2.8 | 閉じる確認は今と同じ | policy | ADR-005 |
| BR2.9 | 取得に入るときに前の結果を消す（U1 の暫定） | policy | units Q2a |
| BR3.1〜BR3.9 | クエリの文の読み取りと、読み取りのエラー | validation・policy | FR3.1、FR3.2、FR4.1、FR5.2〜FR5.5、FR6.1〜FR6.4 |
| BR4.1 | 決まった項目の値 | calculation | FR4.1 |
| BR5.1〜BR5.10 | filter の演算子と評価、@timestamp の比べ方 | calculation・validation・constraint | FR5.1〜FR5.6、NFR2 |
| BR6.1〜BR6.4 | 結果の表示（0 件、古い結果、列と並び、行の読み出し） | policy・constraint | FR7.2、FR7.6 |
| BR7.1〜BR7.3 | 接続の変更（確認、捨てるもの、変えられないとき） | policy・constraint | A2、ADR-007、Q8 |
