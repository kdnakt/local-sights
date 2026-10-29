# Business Rules — U2 `stats` をそろえる（u2-stats-complete）

上流：`inception/units-generation/unit-of-work.md`（U2）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1.2・FR1.4・FR1.6・FR1.7、FR6.1〜FR6.3、NFR2・NFR7）、`inception/domain-design/components.md`・`decisions.md`（ADR-002・ADR-005）とそのレビュー（DD R-10）、U1 の機能設計（BR1.1〜BR1.9、BR2.1〜BR2.9、BR3.1〜BR3.6）、質問票 `functional-design-questions.md`（Q1〜Q5）、AWS の資料（`stats` の集計の関数と集計以外の関数の表）、今のコード（`parser.rs` の OTHER_COMMANDS・KEYWORDS・function()、`insights_session.rs` の completion_candidates・COMMAND_WORDS・KEYWORD_WORDS、`src/analytics.ts`、`src/i18n/messages.ts` の supportedList、README の対応表、`plan.rs` の DEFAULT_RESULT_LIMIT・MAX_LIMIT）。

## U2 の範囲と、決めたこと

- **関数**：資料の表にある 12 の名前（13 の書き方。count は `*` と項目の 2 通り）をすべて読む（RA Q1 C）。計算の細部は Q1 A・Q5 A のとおり仮に決め、U4 でコンソールと比べる。
- **stats のあと**：`filter`・`sort`・`limit` を書ける（Q4 B、RA Q3 A）。`fields` と 2 つ目の `stats` は「対応していない」（Q3 A・Q4 B）。`stats` の前の `sort`・`limit` は U1 のとおり集計の前に適用する。
- **件数の上限**：表に出す行は既定 10,000 行、`limit` で最大 100,000（Q2 A）。集計はすべてのグループで行う。グループの数の上限は設けない。
- **一覧**：QueryCatalog に 1 つ持ち、パーサ・補完・色分け・エラー文言はそこから作り、README はテストで一致を確かめる（DD Q6 A）。U1 の暫定の振り分けの表を置き換える。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: stats で読める集計の関数は count・count_distinct・sum・avg・min・max・pct・stddev・earliest・latest・sortsFirst・sortsLast
    category: validation
    applies_to: QueryParser、QueryCatalog
    trigger: stats の中で関数名を読んだとき
    logic: IF 名前が QueryCatalog の aggregateFunction で supported THEN 引き数を読む。U1 の BR1.2 の「U2 の関数は Unsupported」を外す。名前の大文字・小文字は資料の書き方と同じものだけ（`sortsfirst` は Unsupported の問題。コンソールの区別は U4 で確かめる）
    violation: 一覧にない名前は今の UNSUPPORTED_FUNCTION
    source: FR1.2、RA Q1 C
  - id: BR1.2
    statement: pct は 2 つの引き数（項目, 百分位）をとり、百分位は 0〜100 の数。ほかの関数は 1 つの引き数
    category: validation
    applies_to: QueryParser
    trigger: pct( を読んだとき
    logic: IF 引き数が 2 つで 2 つ目が 0 以上 100 以下の数（整数か小数）THEN AggregateSpec(pct, percentile)。IF 範囲外 THEN Syntax（百分位の場所、PCT_OUT_OF_RANGE の鍵）。IF 引き数の数が違う THEN U1 の BR1.7 の「引き数の数が違う」。count_distinct・stddev・earliest・latest・sortsFirst・sortsLast は 1 つ（`*` は受けない）
    violation: 問題を返す
    source: FR1.2、Q1 A
  - id: BR1.3
    statement: as を付けない pct の列の名前は、元の文の切り出し（例 `pct(latency, 95)`）
    category: validation
    applies_to: QueryParser
    trigger: 列の名前を決めるとき
    logic: U1 の BR1.9 のとおり（空白をそろえた元の文の切り出し）
    violation: なし
    source: FR3.1
  - id: BR1.4
    statement: stats のあとに filter・sort・limit を書け、書いた順に集計の結果の行に適用する。項目の参照は集計の結果の列の名前だけ
    category: validation
    applies_to: QueryParser、QueryPlan
    trigger: stats のあとに `|` が続くとき
    logic: stats のあとの命令の項目は、イベントの項目ではなく結果の列の参照（ColumnRef）として読む（FR レビュー R-01）。(a) 型：FieldRef に `Column(index)` を足し、読み取りの時に名前を結果の列の位置に解く。(b) 名前の引き方：書いた語（バッククォートは外す。`a.b` は 1 つの名前 `a.b` として引く）が結果の列の名前（as の名前か、BR1.9 の切り出しの文字）と大文字・小文字まで一致するもの。IF 一致する列がない THEN Syntax（その語の場所、UNKNOWN_RESULT_COLUMN の鍵）。`count(*)` のような名前はバッククォートで囲んで書く（`` `count(*)` ``）。(c) 比べ方：BR3.1 の対応表で CellValue を今の FieldValue に写し、今の filter の比べ方（比較・and/or/not・括弧・like・正規表現・in、数と文字の比較の決まり、Missing はどの比較にも合わない）をそのまま使う。timestamp の列は @timestamp と同じ扱い（数としてのミリ秒、今の日時の文字との比較の拡張も効く）。(d) 列どうしの比較（`n > m`）も今の CompareFields と同じく書ける。(e) sort の鍵も同じ引き方。limit の書き方は今と同じ（limit 0 は 0 行、最大 100,000）
    violation: 問題を返す
    source: FR1.4、Q4 B、RA Q3 A
  - id: BR1.5
    statement: stats のあとの fields と、2 つ目の stats は「対応していない」
    category: validation
    applies_to: QueryParser
    trigger: stats のあとに fields か stats を読んだとき
    logic: Unsupported の問題（その語の場所。fields は FIELDS_AFTER_STATS、2 つ目の stats は SECOND_STATS の鍵）。U1 の BR1.6 の「stats は最後の命令」を、この 2 つだけに狭める
    violation: 問題を返す
    source: FR1.4、Q3 A、Q4 B
  - id: BR2.1
    statement: count_distinct は値の文字が同じものを 1 つとして、項目を持つイベントの値の種類を正確に数える
    category: calculation
    applies_to: StatsAggregator
    trigger: イベントをグループに入れたとき
    logic: 値が Missing なら無視する。値を U1 の鍵と同じ文字（BR2.1 の比べ方：数は丸めた文字、真偽値は 1 / 0）にして集まりに足す。結果は集まりの大きさ（integer）。資料の「非常に多い値では近似」は使わず、手元では正確に数える
    violation: なし
    source: FR1.2、Q1 A
  - id: BR2.2
    statement: pct(項目, p) は、数に読めた値を昇順に並べた最も近い順位の値（⌈p/100 × n⌉ 番目。p = 0 は最小）
    category: calculation
    applies_to: StatsAggregator
    trigger: finish のとき
    logic: 値は sum と同じく数に読めるものだけ（U1 の BR2.3）。n = 0 なら値なし。並べるのはグループごとに finish のとき 1 回。順位は浮動小数を使わず整数で正確に求める（FR レビュー R-05）：p を読み取りの時に 10 進の文字から（分子 a, 小数の桁 d）として持ち、k = max(1, ceil(a × n ÷ (100 × 10^d)))（u128 の整数の割り算）。k 番目（1 始まり）の値を返す。境界の表（p = 0・7・50・99.9・100、n = 1・100・101）をテストに先に書く。結果は decimal（U1 の BR2.8 の文字）
    violation: なし
    source: FR1.2、Q1 A
  - id: BR2.3
    statement: stddev は数に読めた値の標本標準偏差（n − 1 で割る）。値が 1 つなら 0、0 個なら値なし
    category: calculation
    applies_to: StatsAggregator
    trigger: イベントをグループに入れたとき、finish のとき
    logic: Welford の方法で逐次に平均と 2 乗和を更新する（桁落ちを避ける）。finish で n ≥ 2 なら sqrt(m2 / (n − 1))、n = 1 なら 0、n = 0 なら値なし。結果は decimal
    violation: なし
    source: FR1.2、Q1 A
  - id: BR2.4
    statement: sortsFirst・sortsLast は、項目の値を sort と同じ順（U1 の BR2.4）に並べたときの最初・最後の値
    category: calculation
    applies_to: StatsAggregator
    trigger: イベントをグループに入れたとき
    logic: U1 の min・max と同じ比べ方（NaturalSort）。sortsFirst は min、sortsLast は max と同じ値を返す（min・max との違いがコンソールにあるかは U4 で確かめる）。同じ順位の値は同じ文字なので、どれを返しても同じ
    violation: なし
    source: FR1.2、Q5 A
  - id: BR2.5
    statement: earliest・latest は、項目を持つイベントのうち @timestamp が最も早い（遅い）イベントの値。同じ時刻なら結果の並びの決まりで先（後）のもの
    category: calculation
    applies_to: StatsAggregator
    trigger: イベントをグループに入れたとき
    logic: 項目が Missing のイベントは無視する。評価の入力の view は、今の結果の並びの決まり（時刻、ロググループ、ストリーム、ストリーム内の順番）で並んでいるため、view の中の位置で比べる（FR レビュー R-07）：earliest は view で最初に見た値、latest は最後に見た値を残す（StatsAggregator の add は view の順に呼ばれる）。値の型は U1 の min・max と同じ（CellValue）
    violation: なし
    source: FR1.2、Q5 A
  - id: BR2.6
    statement: 集計の途中の状態はメモリの中だけに置き、グループの数に上限を設けない。pct と count_distinct は値を保持するため、メモリを NFR2 で測る
    category: constraint
    applies_to: StatsAggregator
    trigger: 常に
    logic: 塊ごとの累積と止める信号は U1 の BR2.9 のまま（仕上げも BR3.1 のとおり止められる）。100 万件のイベントで `stats count_distinct(@message), pct(latency, 99) by @logStream` のような重いクエリを評価し、落ちないこと、最終結果までの時間が同じ条件の U1 の `stats count(*) by bin(5m)` の 3 倍以内であることを確かめる（DD R-10、FR レビュー R-06）。満たさないときの手（count_distinct を値の文字のハッシュで持つ、pct を並べる前に値を数の配列だけにする）は作る段階で試し、それでも満たさなければ記録して利用者に諮る
    violation: なし
    source: NFR2、Q2 A、DD レビュー R-10
  - id: BR3.1
    statement: stats のあとの命令は、集計の行（鍵の昇順に並んだもの、U1 の BR2.7）に書いた順に適用する
    category: policy
    applies_to: QueryEvaluator（Aggregate の段のあと）
    trigger: 集計の行ができたあと
    logic: filter は行ごとに、条件の列の値を次の対応表で FieldValue に写して今の filter の比べ方で調べ、残す：integer・decimal → Number、text → Text、timestamp → Number（ミリ秒）、missing → Missing。sort は今の NaturalSort の順で並べる（同じ値は前の並びのまま）。limit は先頭からその数だけ残す。集計の行の数はグループの数に比例し、正規表現の filter や sort が加わると長くなりうるため、仕上げも 10,000 行ごとに止める信号を確かめ、止まったら結果を捨てる（U1 の BR3.5 の「止められない仕上げ」を改める、FR レビュー R-06）
    violation: なし
    source: FR1.4、Q4 B
  - id: BR3.2
    statement: aggregates の結果も、limit を書かないときは既定 10,000 行で打ち切り、打ち切ったことを示す
    category: policy
    applies_to: QueryEvaluator、InsightsSession、QueryResultTable
    trigger: 集計の行を仕上げたとき
    logic: events の今の決まり（evaluator.rs の matched_count・truncated_at、Loop-back 3 の p08）と同じ判定にする（FR レビュー R-03）：matchedCount = 最後の limit と既定の打ち切りの前に残っていた行の数（limit のあとに filter があるときは、その filter のあとで数え直さない）。truncatedAt は、行が切られたとき（limit が残りの行より小さい、または limit がなく 10,000 を超えた）だけ、表に出した行の数。limit 0 は 0 行で truncatedAt なし（今の events と同じ）、行が 0 のときも truncatedAt なし。limit が複数あれば書いた順にすべて適用する。表の打ち切りの注記は今の文言を「行」に合わせて出す。U1 の BR3.4 の「truncatedAt はなし」を改める
    violation: なし
    source: FR1.7、Q2 A
  - id: BR4.1
    statement: 命令・集計の関数・ほかの関数・キーワード・決まった項目の一覧は、中核の QueryCatalog に 1 つ持つ
    category: policy
    applies_to: QueryCatalog、QueryParser、InsightsSession
    trigger: 常に
    logic: QueryCatalog は QueryVocabulary の固定の一覧（命令：fields・filter・sort・limit・stats は supported、parse・dedup・display・pattern・diff・unmask・unnest・lookup・filterIndex・source は unsupported。名前は AWS の資料の書き方で持ち、命令の読み取りは今のとおり大文字・小文字を区別しない。集計の関数：BR1.1 の 12。ほかの関数：toMillis・bin。キーワード：by・as・and・or・not・like・in・asc・desc。決まった項目：@timestamp・@message・@logStream・@log）。キーワードは reserved（どこでも項目の名前として読まない）の印を持ち、reserved は今と同じ and・or・not・like・in の 5 つだけにする。by・as・asc・desc は今と同じく置かれる位置でだけキーワードとして読み、`desc` や `by` という名前の JSON の項目は裸で書いても今のとおり項目として読む（BR5.1、FR レビュー R-04）。QueryParser の OTHER_COMMANDS・KEYWORDS（reserved の語）・関数の振り分けと、InsightsSession の補完の固定の候補（COMMAND_WORDS・KEYWORD_WORDS・FIXED_FIELD_WORDS）は、この一覧から作る。補完の候補の文字は今の文字を保つ（`toMillis(@timestamp)` などの今の候補はそのまま）。U1 の暫定の表は消す
    violation: なし
    source: FR6.2、ADR-005、DD Q6 A
  - id: BR4.2
    statement: 画面は QueryCatalog の一覧をコマンドで受け取り、色分けの規則と「使える命令」のエラー文言をそこから作る
    category: policy
    applies_to: TauriApp、ApiBridge、AnalyticsHelpers、QueryEditor、MessageCatalog、LogAnalyticsPage
    trigger: 画面を開いたとき（一覧は起動中に変わらないため 1 回だけ）
    logic: 新しいコマンド get_query_catalog は一覧を返す。一覧は固定の定数で、AppState の鍵を取らずに返す。capabilities と permissions に足し、今の権限の一致のテスト（数は固定せず 3 か所の一覧の一致を確かめるもの）には、名前を明示する一覧にこのコマンドを足す（FR レビュー R-08）。AnalyticsHelpers は命令（supported と unsupported の両方）・関数・キーワードの語から色分けの規則を作り、一覧が届くまでは色分けなしで動く。MessageCatalog の supportedList は「使える命令：{commands}」の差し込みの文にし、LogAnalyticsPage が supported の命令を並べて差し込む
    violation: 取れなければ色分けなしで動き、エラー文言は命令の一覧なしの文を出す
    source: FR6.2、ADR-005
  - id: BR4.3
    statement: 補完の候補に stats・by・as と集計の関数を入れる
    category: policy
    applies_to: InsightsSession
    trigger: 補完の候補を作るとき
    logic: BR4.1 の一覧の supported の命令・キーワード・集計の関数・ほかの関数・決まった項目、と今の JSON の項目の名前。種類（CompletionKind）に Function を足す
    violation: なし
    source: FR6.2
  - id: BR4.4
    statement: README の対応する命令・関数の表は、QueryCatalog の一覧と一致することをテストで確かめる
    category: constraint
    applies_to: README、QueryCatalog のテスト
    trigger: cargo test のとき
    logic: 比べるのは命令・集計の関数・ほかの関数（toMillis・bin）の 3 種類だけ（キーワード・決まった項目・演算子は比べない、FR レビュー R-02）。README の Supported の表に「Commands」の行（今の行を `stats` を足して直す）と、新しく「Aggregate functions in `stats`」の行（12 の関数と `bin(…)`）を置き、Not supported の表の最初の行（「The other commands」）から `stats` を外す。テストは、README の 3 つの行（Supported の Commands の行、Supported の Aggregate functions の行、Not supported の The other commands の行）をその行の先頭の語で見つけ、行の中のバッククォートで囲んだ語のうち括弧と引き数を外した名前の集まりを取り出し、QueryCatalog の supported の命令・supported の集計の関数と bin・unsupported の命令の集まりと、大文字・小文字まで一致することを確かめる。`toMillis` は今の行（toMillis の行）にそのまま残し、比べる対象に入れない。`ispresent` など、関数の行に例として書いた一覧にない語は比べない。表のそのほかの行の形は今のまま
    violation: 一致しなければテストが落ちる
    source: FR6.1、NFR7
  - id: BR5.1
    statement: stats を含まないクエリと、U1 の stats のクエリの結果は変えない（stats のあとの命令と関数を足すだけ）
    category: constraint
    applies_to: QueryParser、QueryEvaluator、StatsAggregator
    trigger: 常に
    logic: U1 のテストは期待値を変えずに通る。ただし U1 で「対応していない」だった書き方（U2 の関数、stats のあとの filter・sort・limit）のテストは、U2 の振る舞いに合わせて直す
    violation: 既存のテストが落ちたら直す
    source: C5
```

## ルールの要約

| ID | 分類 | 要点 | 出典 |
|---|---|---|---|
| BR1.1 | validation | 12 の関数名を読む | FR1.2 |
| BR1.2 | validation | pct の百分位は 0〜100 | FR1.2、Q1 |
| BR1.3 | validation | pct の列の名前 | FR3.1 |
| BR1.4 | validation | stats のあとの filter・sort・limit、列の名前で参照 | FR1.4、Q4 |
| BR1.5 | validation | stats のあとの fields と 2 つ目の stats は Unsupported | FR1.4、Q3 |
| BR2.1 | calculation | count_distinct は正確に数える | FR1.2、Q1 |
| BR2.2 | calculation | pct は最も近い順位 | FR1.2、Q1 |
| BR2.3 | calculation | stddev は標本標準偏差（Welford） | FR1.2、Q1 |
| BR2.4 | calculation | sortsFirst・sortsLast は sort の順 | FR1.2、Q5 |
| BR2.5 | calculation | earliest・latest と同じ時刻の決まり | FR1.2、Q5 |
| BR2.6 | constraint | 上限なし、メモリを測る | NFR2、Q2 |
| BR3.1 | policy | stats のあとの命令を書いた順に適用 | FR1.4 |
| BR3.2 | policy | 既定 10,000 行の打ち切り | FR1.7、Q2 |
| BR4.1 | policy | QueryCatalog に一覧を 1 つ | FR6.2 |
| BR4.2 | policy | 画面は一覧から色分けと文言を作る | FR6.2 |
| BR4.3 | policy | 補完の候補 | FR6.2 |
| BR4.4 | constraint | README と一覧の一致のテスト | FR6.1 |
| BR5.1 | constraint | 前と同じ結果を保つ | C5 |
