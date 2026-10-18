# Business Rules — U3 クエリの画面の取り足しと複数のロググループ（u3-shared-store）

上流：`inception/units-generation/unit-of-work.md`（U3）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1.3、FR2.1〜FR2.8、NFR2）、`inception/domain-design/components.md`・`decisions.md`（ADR-001・ADR-004〜ADR-008）、`inception/refined-mockups/interaction-spec.md`（LogGroupMultiselect、画面 4・6c）、質問票 `functional-design-questions.md`（質問にしなかったことの表）、U1・U2 の機能設計とコード、U2 のコードの見直し（R-01〜R-05）。

## U1・U2 の決まりの扱い

- U1 の BR1.3（ロググループは 1 つだけ）を BR4.1（50 個まで）に置き換える。
- U1 の BR2.1（実行のたびに捨てて取り直す）と BR2.9（取得に入るときに前の結果を消す）を、BR1.1〜BR1.5 と BR3.4 に置き換える。interaction-spec のとおり、次の実行の取得と評価の間は前の結果を残す。
- U1 の BR2.2（期間全体の取得）を BR2.1〜BR2.4 に置き換える。
- U2 のコードの見直しで回した 2 つ（R-01 入れ子の深さ、R-02 補完の名前が残る）を BR5.1・BR5.2 として直す。R-03（並べ替えのメモリ）は BR5.3 で測る。R-04（本物の Ace での目視）と R-05（記録の誤り）は、確かめ方と記録で扱う（functional-spec の 7 章）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 足りない範囲は、期間から取得済みの範囲を引いたものとし、取得済みの範囲とミリ秒で重ならない
    category: calculation
    applies_to: CachePlan（足りない範囲の計算）、QueryStore
    trigger: 実行を始めるとき、ロググループごとに
    logic: 期間を閉区間 [開始, 終了（999 ms まで）] とし、取得済みの範囲（閉区間の並び）を引く。結果は重ならない閉区間の並び。取得済みの範囲の終わりが t なら、足りない範囲は t+1 ms から始まる。期間がすべて取得済みなら空
    violation: なし
    source: FR2.2、FR2.3、ADR-004
  - id: BR1.2
    statement: 取得済みの範囲に加えるのは、そのロググループの取得がすべてのストリームで終わった範囲だけで、取得を始めた時刻の 5 分前より新しい部分は加えない
    category: policy
    applies_to: QueryStore
    trigger: ロググループの 1 つの足りない範囲の取得が終わったとき
    logic: 取得が Completed のとき、その範囲から「取得を始めた時刻の 5 分前」より後を除いた部分を取得済みの範囲に加え、隣り合う・重なる範囲とつなぐ（今のキャッシュの record_range と merge_ranges と同じ）。除いた部分のイベントは committed に入れず staging に入れる（結果には使い、次の実行で取り直す）。CompletedWithFailures・Failed・Cancelled のときは範囲に加えない
    violation: なし
    source: FR2.5、FR2.6、ADR-004、質問票の表（範囲の新しさ）
  - id: BR1.3
    statement: 取得済みとみなさない取得の分（一部失敗・中止・新しすぎる部分）は、その実行の結果には使い、次の実行を始めるときに捨てる
    category: policy
    applies_to: QueryStore（staging）
    trigger: 実行を始めるとき
    logic: すべてのロググループの staging を捨て、epoch を 1 増やす。そのあと足りない範囲を計算する（staging の分は足りない範囲に入っているため、取り直される）。同じイベントを 2 回持たない
    violation: なし
    source: FR2.5〜FR2.7、ADR-004
  - id: BR1.4
    statement: ストリーム内の順番は、QueryStore に足すときに、ロググループのストリームごとに続きの番号を振り直す
    category: calculation
    applies_to: QueryStore
    trigger: ページ（AWS かキャッシュから）を足すとき
    logic: ロググループのストリームごとに次の順番（nextSequence、0 から）を持ち、足すイベントに 1 ずつ振る。committed と staging で同じ番号の列を使う（staging を捨てても番号は戻さない）。違うロググループの同じ名前のストリームは別の列にする
    violation: なし
    source: ADR-004、FR2.8、ドメイン設計の R-05
  - id: BR1.5
    statement: 評価の並びは、選んだロググループの、期間に入るイベント（committed と staging）で作る
    category: calculation
    applies_to: QueryStore、QueryRunner
    trigger: 取得の段が終わって評価に入るとき
    logic: 選んだロググループごとに、committed と staging のうち期間に入るイベントを集め、（イベント, ロググループ名）の並びにする。並びの初めの順は U2 の BR4.4（@timestamp の降順、同じ時刻はロググループ名・ストリーム名・順番の昇順）のとおり。評価の間は、並びの中の位置で行を指し、epoch で守る（結果の行の持ち方は BR3.2）
    violation: なし
    source: FR2.1、FR2.8、FR4.1、U2:BR4.4
  - id: BR1.6
    statement: QueryStore を捨てるのは、接続を変えたときとアプリを閉じたときだけ
    category: policy
    applies_to: QueryStore、AppSession
    trigger: 接続を変えたとき
    logic: すべてのロググループの保持と範囲を捨て、epoch を 1 増やす。ロググループの選択を変えても、期間を変えても捨てない（選ばなくなったロググループの保持も残す）。手で捨てる手段は作らない
    violation: なし
    source: ADR-008、domain Q9

  - id: BR2.1
    statement: ロググループごとに、足りない範囲のうちディスクキャッシュの記録範囲と重なる部分は、キャッシュから読む
    category: policy
    applies_to: LogCache、QueryRunner
    trigger: ディスクキャッシュが有効で、足りない範囲があるとき
    logic: そのロググループのキャッシュの記録範囲のうち、足りない範囲と重なる部分を求め、その部分に入るイベントだけをキャッシュから読んで committed に足し、その部分を取得済みの範囲に加える（キャッシュの記録範囲は取得の直前 5 分を除いて書かれているため、BR1.2 の新しさの決まりは満たしている）。ログ管理の画面がキャッシュに書いた範囲も使う。キャッシュが読めない・壊れているときは今と同じ扱いで、その部分は AWS から取得する。そのあと足りない範囲を計算し直す
    violation: なし
    source: FR2.2、ドメイン設計の R-02
  - id: BR2.2
    statement: キャッシュで埋まらなかった足りない範囲は、ロググループ 1 つずつ、範囲 1 つずつ、今の取得の流れで AWS から取得する
    category: policy
    applies_to: FetchCoordinator、QueryRunner
    trigger: BR2.1 のあと、足りない範囲が残っているとき
    logic: 選んだ順のロググループ、時刻の順の範囲ごとに、今の取得の流れ（ストリームの選び方、8 本まで同時、呼び出しの速さ、再試行、あきらめ）で 1 回の取得をする。ページは QueryStore のそのロググループの staging に足し、取得が終わったら BR1.2 で committed に移すか決める。キャッシュへの書き込みは今の取得と同じ決まりで、その範囲について行う
    violation: なし
    source: FR2.2、FR2.4、ドメイン設計の R-03
  - id: BR2.3
    statement: 期間がすべて取得済みのロググループは、AWS もキャッシュも使わない
    category: policy
    applies_to: QueryRunner
    trigger: 足りない範囲が空のとき
    logic: そのロググループの取得の結果を NotNeeded にする。すべてのロググループが NotNeeded か FromCache なら、取得の段で AWS を呼ばない
    violation: なし
    source: FR2.3
  - id: BR2.4
    statement: 取得は、2 つの画面を合わせて同時に 1 つ、という決まりの中で、1 回の実行の取得すべてを 1 つの取得として扱う
    category: constraint
    applies_to: AppSession（FetchOwner）、QueryRunner
    trigger: 実行の取得の段
    logic: 取得の段の始めに FetchOwner を LogAnalytics にし、すべてのロググループの取得が終わってから None に戻す（ロググループの間で戻さない）。[Cancel] と閉じる確認は、今走っているロググループの取得を止め、残りのロググループは始めない
    violation: なし
    source: ADR-005、U1:BR2.3
  - id: BR2.5
    statement: 実行の取得の終わりは、ロググループごとの結果から決める
    category: policy
    applies_to: QueryRunner、InsightsSession
    trigger: 取得の段が終わったとき
    logic: '[Cancel] の印があれば Cancelled（評価しない、U1:BR2.7）。どれかのロググループが Failed なら、実行を FetchFailed にして評価しない（取得全体が失敗したロググループがあれば評価しない、components.md の QueryRunner）。CompletedWithFailures のロググループがあれば、評価して一部失敗の印を立てる。そのほかは評価する'
    violation: なし
    source: FR2.5、FR2.6、components.md QueryRunner
  - id: BR2.6
    statement: 失敗したストリームの一覧と取得のエラーには、ロググループの名前を添える
    category: policy
    applies_to: QueryRunner、InsightsSession、LogAnalyticsPage
    trigger: 一部失敗・取得全体の失敗を示すとき
    logic: 失敗の一覧の行は「ロググループ名 / ストリーム名」と安全な理由。取得全体の失敗のアラートには、失敗したロググループの名前と理由を出す。認証情報とアクセスキー ID は出さない
    violation: なし
    source: FR2.5、FR2.8、ドメイン設計の R-04、project.md Forbidden
  - id: BR2.7
    statement: 取得中の進み具合は、ロググループの数と、今のロググループの中のストリームの数で示す
    category: policy
    applies_to: InsightsSession、LogAnalyticsPage
    trigger: Fetching の間
    logic: 「Fetching logs（ロググループ i / n）」と、今のロググループの読み終えたストリームの数／計画した数、取得した件数、経過時間。キャッシュから読んでいる間は「Reading cache」
    violation: なし
    source: FR2.4

  - id: BR3.1
    statement: 評価は、取得済みのイベントが後から増えても自動ではやり直さない
    category: policy
    applies_to: InsightsSession
    trigger: 実行を終えたあと
    logic: 結果を作り直すのは [Run query] を押したときだけ
    violation: なし
    source: A5、FR1.7
  - id: BR3.2
    statement: 結果の行は、評価が終わったときに、そのイベント（とロググループ名）を写して結果の中に持つ
    category: constraint
    applies_to: QueryEvaluator、QueryStore、TauriApp
    trigger: 評価が終わったとき、行を読み出すとき
    logic: 結果の行は limit（100,000 まで）か既定の上限（10,000）で切ったあとの行だけなので、写しても多くて 100,000 件になる。写した行は QueryStore が後で変わっても（取り足し、staging を捨てる）読める。表のスクロールと行を開いたときは、runId が今の結果と同じなら結果の中の写しから読み、違えば空を返す（U1 の BR6.4 の epoch の確かめを、runId の確かめに置き換える）
    violation: なし
    source: FR7.5、FR7.6、interaction-spec（前の結果を残す）
  - id: BR3.3
    statement: 補完の候補の JSON の項目の名前は、選んだロググループの、取得済みのログから集める
    category: calculation
    applies_to: QueryStore、QueryRunner
    trigger: 実行の取得が終わったとき
    logic: U2 の BR1.7 の集め方を、評価の並びのイベント（先頭から 10,000 件）に当てはめる
    violation: なし
    source: FR9.1（Should）、U2:BR1.7
  - id: BR3.4
    statement: 次の実行の取得と評価の間は、前の結果を残して表示する
    category: policy
    applies_to: InsightsSession、QueryResultTable
    trigger: 実行を始めたとき
    logic: 前の結果は写しを持っている（BR3.2）ため、実行を始めて QueryStore の staging を捨てても、取り足しで保持が増えても、そのまま表示できる。新しい結果が出たら置き換える。FetchFailed・EvaluationFailed のときは前の結果を消す（interaction-spec のとおり）
    violation: なし
    source: interaction-spec（前の結果の扱い）、U1:BR2.9 の置き換え

  - id: BR4.1
    statement: ロググループは、今の接続の一覧から 50 個まで選べる
    category: validation
    applies_to: LogGroupMultiselect、InsightsSession
    trigger: ロググループを選んだとき
    logic: 選んだものは欄の下にトークンで出し、× で外せる。50 個選んだら、それ以上は選べず、上限に達したことを文で示す。接続を変えたら選択を空に戻す
    violation: 51 個目は受け付けない
    source: FR1.3、ADR-006、interaction-spec の LogGroupMultiselect
  - id: BR4.2
    statement: '[Run query] の押せる条件の「ロググループが選ばれている」は、1 個以上 50 個以下とする'
    category: constraint
    applies_to: InsightsSession
    trigger: 画面の状態を作るとき
    logic: U1 の BR1.5 の (2) を、選んだ数が 1 以上 50 以下に広げる
    violation: 押せない（理由の文を出す）
    source: FR1.6

  - id: BR5.1
    statement: 条件の括弧と not の入れ子は 100 段までとし、超えたら書き間違いにする
    category: validation
    applies_to: QueryParser、QueryEvaluator
    trigger: 条件を読むとき
    logic: 読み取りの途中で入れ子の深さを数え、101 段目で Syntax（「入れ子が深すぎる」、開いた括弧の場所）を返す。評価は読み取りが通った条件だけを受け取るため、評価の深さも 100 段までになる
    violation: QueryProblem（Syntax）
    source: U2 のコードの見直し R-01、ADR-002（重すぎるクエリへの備え）
  - id: BR5.2
    statement: 補完の候補の JSON の項目の名前は、実行を始めるときに消す
    category: policy
    applies_to: InsightsSession、QueryStore
    trigger: 実行を始めたとき、接続を変えたとき
    logic: 前の実行の名前を消し、新しい実行の取得が終わったら集め直す（BR3.3）。取得が失敗したら空のまま
    violation: なし
    source: U2 のコードの見直し R-02
  - id: BR5.3
    statement: 取得済み 100 万件の保持と評価で、アプリが落ちず、ピークのメモリを測って記録する
    category: constraint
    applies_to: QueryStore、QueryEvaluator
    trigger: 作る段階の確かめ
    logic: 100 万件を 2 つのロググループに分けて保持し、JSON の項目の sort を含むクエリを評価したときのピークのメモリを測る。メモリの上限は設けない（NFR2 で確かめる）
    violation: なし
    source: NFR2、U2 のコードの見直し R-03
```

## まとめ

| ID | 決まり | 出典 |
|---|---|---|
| BR1.1〜BR1.6 | 保持：足りない範囲、範囲に加える条件と新しさ、staging を次の実行で捨てる、順番の振り直し、評価の並び、捨てるとき | FR2.1〜FR2.3、FR2.5〜FR2.8、ADR-004、ADR-008 |
| BR2.1〜BR2.7 | 取得：キャッシュの重なる部分、ロググループと範囲ごとの取得、取得済みは呼ばない、取得の 1 本化、終わりの決め方、失敗にロググループ名、進み具合 | FR2.2〜FR2.6、ADR-005 |
| BR3.1〜BR3.4 | 結果：自動ではやり直さない、行の読み出し、補完の名前、前の結果を残す | A5、FR7.5、FR7.6、FR9.1、interaction-spec |
| BR4.1・BR4.2 | 選択：50 個まで | FR1.3、FR1.6、ADR-006 |
| BR5.1〜BR5.3 | U2 から回した直し：入れ子の上限、補完の名前を消す、メモリを測る | U2 の見直し R-01〜R-03 |
