# Business Rules — U2 クエリの言語をそろえる（u2-query-language）

上流：`inception/units-generation/unit-of-work.md`（U2）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR3.3〜FR3.8、FR4.2〜FR4.5、FR6.1・FR6.2・FR6.4、FR7.1・FR7.3〜FR7.5、FR8、FR9、NFR1・NFR2・NFR9・NFR10）、`inception/domain-design/components.md`・`decisions.md`、`inception/refined-mockups/interaction-spec.md`・`mockups.md`（画面 5・7、QueryEditor、QueryResultTable）、質問票 `functional-design-questions.md`（Q1・Q1a・Q1b）、AWS の資料（filter・fields・sort・limit、Boolean・comparison 演算子、Structure types のバッククォート）、U1 の機能設計（rules.md の BR3.3・BR3.4・BR5.1・BR6.3 の仮の決まり）と U1 のコードの見直し（R-02〜R-04）。

## U1 の仮の決まりの扱い

U1 で仮に決めた細部は、コンソールで確かめた結果（Q1a・Q1b）に合わせてここで決め直す。決め直したものは U1 の決まりより優先する。

| U1 の決まり | U2 での扱い | 根拠 |
|---|---|---|
| BR3.3 言葉の大文字・小文字を区別しない | そのまま（確かめた） | Q1a の 3a・3c・3d |
| BR3.4 優先順位 not > and > or | and > or は確かめた。`not` は括弧で囲んだ条件の前にだけ書ける（BR2.4）に改める | Q1a の 4a〜4f、3b、5e |
| BR5.1 文字列の項目と数値は満たさない | 数字だけの文字列は数として比べる（BR1.6）に改める | Q1a の 6b〜6h |
| BR6.3 結果は時刻の昇順、決まった列 | sort がないときは @timestamp の降順（BR4.4）、列は fields（BR3.3） | Q1b の 1d、Q1a の 2a |

U1 のコードの見直しで回した 3 つ（R-02：最後の区切りのあとの [Cancel]、R-03：評価の異常終了の表示、R-04：条件の木の場所）も、この作業単位で直す（BR10.1〜BR10.3）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 項目の名前は、決まった項目（@timestamp・@message・@logStream・@log）か JSON の項目として読む
    category: validation
    applies_to: QueryParser
    trigger: 項目の位置の言葉を読んだとき
    logic: '@ で始まる名前は、決まった 4 つのどれかなら決まった項目。ほかの @ の名前（@ingestionTime など）は Unsupported。@ で始まらない名前は JSON の項目として読む（U1 の BR3.6 の「Unsupported」をこれに置き換える）。名前の文字は英数字・_・- で、. で区切って入れ子を書く'
    violation: QueryProblem（Unsupported、言葉と場所つき）
    source: FR4.1、FR4.2
  - id: BR1.2
    statement: JSON の項目は、メッセージが JSON のオブジェクトとして読めるイベントからだけ取り出す
    category: calculation
    applies_to: EventFieldReader
    trigger: JSON の項目を評価するとき
    logic: メッセージ全体（前後の空白は除く）を JSON として読み、オブジェクトのときだけ項目を持つ。読めない・壊れている・配列・文字列や数だけのときは、JSON の項目をどれも持たない（Missing）。エラーにはせず、ほかのイベントの評価を続ける。同じイベントを 1 回の評価の中で何度も読み直さない（読んだ結果を、そのイベントの評価が終わるまで持つ）
    violation: なし
    source: FR4.2、FR4.3
  - id: BR1.3
    statement: 入れ子の項目は `a.b` の形で指定し、名前に . などを含むときはバッククォートで囲む
    category: validation
    applies_to: QueryParser、EventFieldReader
    trigger: JSON の項目を読むとき
    logic: '`http.status` は http の中の status。`` `user.id` `` はキー user.id そのもの。`` a.`b.c`.d `` のように混ぜてよい。途中の値がオブジェクトでなければ Missing'
    violation: 閉じていないバッククォートは Syntax
    source: FR4.2、AWS の資料（Structure types の attribute のバッククォート）
  - id: BR1.4
    statement: JSON の値は、文字列・数・真偽値をそのまま、null とオブジェクトと配列は値なし（Missing）として扱う
    category: calculation
    applies_to: EventFieldReader
    trigger: JSON の項目の値を読むとき
    logic: 文字列は Text、数は Number、true・false は Boolean。null・オブジェクト・配列は Missing。配列の中の要素を指す書き方（`a.0`、`a[0]`）は U2 では読まない（Unsupported）
    violation: なし（配列の書き方は QueryProblem の Unsupported）
    source: FR4.2、FR4.5、OQ2
  - id: BR1.5
    statement: 値がない項目は、= と < <= > >= と like と =~ と in を満たさず、!= と not like と not in を満たす
    category: calculation
    applies_to: QueryEvaluator
    trigger: 条件を評価するとき、項目が Missing のとき
    logic: Missing の項目の Compare は、演算子が != のときだけ満たす。Like は not like のときだけ、In は not in のときだけ満たす。Match（=~）は満たさない
    violation: なし
    source: FR4.4、Q1a の 5c・5d・5f・5g
  - id: BR1.6
    statement: 型の違う値は、数字だけの文字列なら数として比べ、それ以外は = と < <= > >= を満たさず != を満たす
    category: calculation
    applies_to: QueryEvaluator
    trigger: Compare・In を評価するとき
    logic: '両方が数か、片方が数で他方が数字だけの文字列（前後の空白なし、- と . を含んでよい）なら、数として比べる。両方が文字列で、両方とも数字だけなら数として比べ、そうでなければ文字の順（Unicode のスカラー値の順）で比べる。数と、数字だけでない文字列は、= < <= > >= を満たさず、!= を満たす。真偽値は文字列 "true"・"false" と同じに扱う。@timestamp と日時の文字列は U1 の BR5.9 のとおり'
    violation: なし
    source: FR4.5、Q1a の 6b〜6h（U1 の BR5.1 を置き換える）
  - id: BR1.7
    statement: 取得済みのログに出てくる JSON の項目の名前を集め、補完の候補にする
    category: calculation
    applies_to: EventFieldReader、QueryRunner
    trigger: クエリの画面の取得が終わったとき
    logic: QueryStore の中のイベントのうち、先頭から 10,000 件までのメッセージを JSON として読み、オブジェクトの項目の名前を入れ子は a.b の形で集める（名前は 500 個まで、名前の順）。メッセージの値は集めない
    violation: なし
    source: FR9.1（Should）

  - id: BR2.1
    statement: 命令は fields・filter・sort・limit の 4 つを読み、ほかの命令は対応外とする
    category: validation
    applies_to: QueryParser
    trigger: 命令の位置の言葉を読んだとき
    logic: U1 の BR3.2 の対応外の一覧から fields・sort・limit を外す。`limit any`・`fields ... as`・`display`・`stats` などは Unsupported（言葉と場所つき）
    violation: QueryProblem（Unsupported）
    source: FR3.1、FR6.1
  - id: BR2.2
    statement: 書き間違いと対応外は、別の文言で、行と列と長さつきで返す
    category: validation
    applies_to: QueryParser、QueryEditor
    trigger: 読み取りでエラーになったとき
    logic: Syntax は「書き方の誤り」の文言、Unsupported は「この版では対応していない」の文言とその言葉。場所は言葉か記号の始まりの行・列と長さ。閉じていない括弧・引用符・正規表現は開いた場所を示す。命令の後ろに何もないときは命令の言葉の場所を示す
    violation: なし
    source: FR6.1、FR6.2
  - id: BR2.3
    statement: エラーの文言は英語と日本語を用意し、クエリの言葉と場所だけを出す
    category: policy
    applies_to: QueryParser、i18n
    trigger: エラーを出すとき
    logic: 文言は OS の言語設定に合わせる。文言にはクエリから取った言葉と行・列だけを入れ、認証情報やアクセスキー ID は入れない
    violation: なし
    source: FR6.4、NFR9、project.md Forbidden
  - id: BR2.4
    statement: not は、括弧で囲んだ条件の前にだけ書ける
    category: validation
    applies_to: QueryParser
    trigger: not を読んだとき
    logic: 'IF not の次が ( THEN 括弧の中の条件の否定。ELSE IF not の次が like・in（not like・not in） THEN それらの演算子。ELSE Syntax（「not の後ろは括弧で囲む」）。U1 の BR3.4 の「not a and b」を読む決まりを置き換える'
    violation: QueryProblem（Syntax、not の場所）
    source: FR5.2、Q1a の 3b・4d・4e・4f・5e
  - id: BR2.5
    statement: 優先順位は and が or より強く、括弧で変えられる
    category: validation
    applies_to: QueryParser
    trigger: 条件を読んだとき
    logic: '`a or b and c` は `a or (b and c)`。比べる・like・=~・in は and・or より強い'
    violation: なし
    source: FR5.2、Q1a の 4a〜4c

  - id: BR3.1
    statement: 命令は文に現れた順に、前の命令の結果に当てはめる
    category: calculation
    applies_to: QueryEvaluator
    trigger: 評価するとき
    logic: 最初の並びは BR4.4 の並び（@timestamp の降順）。filter は残す行を絞り、sort は並べ替え、limit は先頭から切る、fields は出す列を足す。たとえば `limit 5 | filter …` は新しい 5 件の中から絞る
    violation: なし
    source: FR3.1
  - id: BR3.2
    statement: fields は出す列を選ぶ。何度書いても、書いた項目をすべて出す
    category: calculation
    applies_to: QueryEvaluator
    trigger: fields を評価するとき
    logic: 列は fields に現れた順に、同じ項目は 1 つにまとめて並べる。行は減らさない
    violation: なし
    source: FR3.3、AWS の資料（fields）、Q1a の 2c
  - id: BR3.3
    statement: fields を書かないときの列は @timestamp と @message
    category: calculation
    applies_to: QueryEvaluator
    trigger: QueryPlan に fields がないとき
    logic: 列を @timestamp・@message の順にする
    violation: なし
    source: FR3.3、Q1a の 2a・2b
  - id: BR3.4
    statement: 結果のどの行にも値がない列は出さない
    category: calculation
    applies_to: QueryEvaluator
    trigger: 結果を作るとき
    logic: fields で選んだ列のうち、結果の行のすべてで Missing のものは列から外す
    violation: なし
    source: Q1a の 5j

  - id: BR4.1
    statement: sort は項目と向き（asc・desc）の並びで並べ、向きを書かないときは昇順
    category: calculation
    applies_to: QueryEvaluator
    trigger: sort を評価するとき
    logic: 先に書いた項目で比べ、同じなら次の項目で比べる。向きの言葉は大文字・小文字を区別しない
    violation: 向きに asc・desc 以外を書いたら Syntax
    source: FR3.4、Q1b の 1a
  - id: BR4.2
    statement: sort の値の比べ方は、AWS の資料の自然順とする
    category: calculation
    applies_to: QueryEvaluator
    trigger: sort の値を比べるとき
    logic: 昇順では、数でない値がすべて数の値より前。数の値どうしは数として比べる。数でない値どうしは、数字の続きと文字の続きの塊に分け、文字の塊は Unicode の順、数字の塊は桁数→値の順で比べる。真偽値は文字列として扱う。降順はその逆
    violation: なし
    source: FR3.4、AWS の資料（sort）
  - id: BR4.3
    statement: sort で値がない行は、いちばん小さい値として並ぶ
    category: calculation
    applies_to: QueryEvaluator
    trigger: sort の項目が Missing のとき
    logic: 昇順では値のある行より前、降順では後ろ
    violation: なし
    source: FR4.4、Q1a の 5h・5i
  - id: BR4.4
    statement: sort を書かないときと、sort の値が同じ行は、決まった並びで並ぶ
    category: calculation
    applies_to: QueryEvaluator
    trigger: 並びを決めるとき
    logic: 決まった並びは、@timestamp の降順、同じ時刻ならロググループ名、ストリーム名、ストリーム内の順番の昇順。sort はこの並びを崩さない安定な並べ替えにする
    violation: なし
    source: FR3.4、FR3.5、Q1b の 1d

  - id: BR5.1
    statement: limit は結果を先頭から指定の件数までにする
    category: calculation
    applies_to: QueryEvaluator
    trigger: limit を評価するとき
    logic: 1〜100,000 はその件数まで。0 は件数を絞らない。100,001 以上は Syntax（「limit は 100,000 まで」）。負の数・小数も Syntax
    violation: QueryProblem（Syntax）
    source: FR3.6、AWS の資料（limit）、Q1a の 7a〜7c
  - id: BR5.2
    statement: limit を書かないときは、結果を 10,000 件までにする
    category: calculation
    applies_to: QueryEvaluator
    trigger: QueryPlan に 1〜100,000 の limit がないとき
    logic: すべての命令を当てはめたあと、先頭から 10,000 件で切る（limit 0 だけのときも同じ）
    violation: なし
    source: FR3.7、OQ1、AWS の資料（limit）
  - id: BR5.3
    statement: 打ち切ったときは、打ち切ったことと件数を示す
    category: policy
    applies_to: QueryResultTable
    trigger: BR5.1・BR5.2 で行を切ったとき
    logic: 見出しの件数の横に「Showing N」（何件に切ったか）と、条件に合った件数を出す
    violation: なし
    source: FR3.7、FR7.3、interaction-spec の truncated

  - id: BR6.1
    statement: エラーのときは、クエリ欄の中の該当の言葉に下線を引き、行番号の横に印を出す
    category: policy
    applies_to: QueryEditor
    trigger: QueryError のとき
    logic: Span の範囲に下線、その行の行番号の横に印。欄の直下に文言を出し、焦点をクエリ欄に置いてカーソルを場所に動かす（U1 の BR3.9 に下線と印を足す）。文を書き換えたら下線と印は消す
    violation: なし
    source: FR6.1、FR6.2、interaction-spec の QueryEditor の error

  - id: BR7.1
    statement: 結果の表の列は QueryResult の列で、時刻の列は選んでいるタイムゾーンで出す
    category: policy
    applies_to: QueryResultTable
    trigger: Done のとき
    logic: '@timestamp は今のログ管理の画面と同じ書き方。@message は 1 行目。JSON の項目は文字列はそのまま、数は書かれたとおり、真偽値は true・false。値がないセルは空'
    violation: なし
    source: FR7.1
  - id: BR7.2
    statement: 結果の件数を見出しに出す
    category: policy
    applies_to: QueryResultTable
    trigger: Done のとき
    logic: 出している行の数を見出しに出す。打ち切ったときは BR5.3
    violation: なし
    source: FR7.3
  - id: BR7.3
    statement: 行を開くと、項目と値の一覧（決まった項目、JSON の項目の名前の順）とメッセージ全体を出す
    category: policy
    applies_to: QueryResultTable
    trigger: 行を開いたとき
    logic: 決まった 4 つ、続けて JSON の項目（入れ子は a.b、名前の順）。値は BR7.1 と同じ書き方
    violation: なし
    source: FR7.4、interaction-spec の QueryResultTable
  - id: BR7.4
    statement: 結果の表は見えている行だけを読み、スクロールは今のログの画面と同じに滑らかにする
    category: constraint
    applies_to: QueryResultTable、TauriApp
    trigger: スクロールしたとき
    logic: U1 の BR6.4（runId・epoch・先頭・件数での読み出し）に、列の値を足して返す。行の値はそのとき読み、結果の中に写して持たない
    violation: なし
    source: FR7.5、NFR2

  - id: BR8.1
    statement: README に対応範囲の一覧を短く書き、今の動きと食い違う記述を直す
    category: policy
    applies_to: README.md
    trigger: U2 を作るとき
    logic: '対応する命令（fields・filter・sort・limit）、項目（決まった 4 つ、JSON の項目と a.b、バッククォート）、演算子（比べる・and・or・not(…)・like・not like・=~・in・not in）、toMillis(@timestamp) と日時の文字列の比べ方（アプリだけの拡張）、対応しないもの（stats・parse・dedup など、関数、limit any、fields … as、配列の要素）を書く。クエリの書き方の説明は書かない。「streams are read one after another」など、今の動きと違う記述を直す'
    violation: なし
    source: FR8.1、FR8.2

  - id: BR9.1
    statement: 入力の補完で、命令・言葉・決まった項目・取得済みのログの JSON の項目の名前を候補に出す
    category: policy
    applies_to: QueryEditor
    trigger: Ctrl+Space を押したとき、命令や項目を書き始めたとき
    logic: 候補は CompletionCandidate。JSON の項目の名前は BR1.7 で集めたもの（まだ取得していなければ出さない）
    violation: なし
    source: FR9.1（Should）
  - id: BR9.2
    statement: 構文を色で分けるが、色だけで意味を伝えない
    category: policy
    applies_to: QueryEditor
    trigger: クエリを書いているとき
    logic: 命令・項目・文字列・数・正規表現を色で分ける。エラーは下線と印と文言でも示す（BR6.1）
    violation: なし
    source: FR9.2（Should）、NFR10
  - id: BR9.3
    statement: 例のクエリを選んでクエリ欄に入れられる
    category: policy
    applies_to: LogAnalyticsPage、QueryEditor
    trigger: 例のクエリの一覧から選んだとき
    logic: 対応する範囲の例を 5 つほど用意する（filter の組み合わせ、JSON の項目、fields と sort と limit）。選ぶとクエリ欄の中身を置き換える。書きかけの文があるときは置き換える前に確かめる
    violation: なし
    source: FR9.3（Should）

  - id: BR10.1
    statement: 評価の最後の区切りのあとに届いた [Cancel] でも、結果を出さずに Cancelled にする
    category: policy
    applies_to: InsightsSession（評価の終わりの反映）
    trigger: 評価の終わりを反映するとき
    logic: IF [Cancel] の印が立っている THEN 結果を捨てて Cancelled。保持が 0 件で区切りが 1 回もないときも同じ
    violation: なし
    source: U1 のコードの見直し R-02、FR2.6
  - id: BR10.2
    statement: 評価が異常終了したときは、取得の失敗ではなく評価の失敗として示す
    category: policy
    applies_to: QueryRunner、InsightsSession、TauriApp
    trigger: 評価の区切りが異常終了したとき
    logic: 段階を FetchFailed とは別の EvaluationFailed にし、「クエリを評価できなかった」の文言を出す。理由に API の名前を付けない
    violation: なし
    source: U1 のコードの見直し R-03
  - id: BR10.3
    statement: 条件の木のどの部分にも場所（Span）を持たせる
    category: constraint
    applies_to: QueryParser
    trigger: 条件を読んだとき
    logic: And・Or・Not・Compare・Like・Match・In のそれぞれに、その部分の始まりと長さを付ける。日時の文字列のエラー、型の誤り、正規表現のエラーはこの場所を使う
    violation: なし
    source: U1 のコードの見直し R-04、FR6.1
```

## まとめ

| ID | 決まり | 出典 |
|---|---|---|
| BR1.1〜BR1.7 | 項目：JSON の項目の読み方、入れ子とバッククォート、値の型、ない項目、型の違う値、補完の候補 | FR4.1〜FR4.5、FR9.1、Q1a |
| BR2.1〜BR2.5 | 読み取り：4 つの命令、エラーの場所と文言、not は括弧の前だけ、and > or | FR3.1、FR5.2、FR6.1〜FR6.4、Q1a |
| BR3.1〜BR3.4 | 命令の順と fields：順に当てはめる、fields の列、既定の列、値のない列は出さない | FR3.1、FR3.3、Q1a |
| BR4.1〜BR4.4 | sort：既定は昇順、自然順、ない値は最小、sort なしは @timestamp の降順 | FR3.4、FR3.5、Q1b |
| BR5.1〜BR5.3 | limit：100,000 まで、0 は絞らない、既定は 10,000、打ち切りの表示 | FR3.6、FR3.7、FR7.3、OQ1 |
| BR6.1 | エラーの下線と印 | FR6.1、FR6.2 |
| BR7.1〜BR7.4 | 結果の表：列、件数、行を開いた中身、見えている行だけ | FR7.1、FR7.3〜FR7.5 |
| BR8.1 | README の対応範囲の一覧 | FR8.1、FR8.2 |
| BR9.1〜BR9.3 | 書く手助け（Should）：補完、色分け、例のクエリ | FR9.1〜FR9.3 |
| BR10.1〜BR10.3 | U1 のコードの見直しから回した直し | U1 R-02〜R-04 |
