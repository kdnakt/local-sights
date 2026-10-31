# Business Rules — U1 薄い一本（u1-stats-skeleton）

上流：`inception/units-generation/unit-of-work.md`（U1）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1.1〜FR1.3・FR1.5・FR1.6・FR1.8、FR2.1〜FR2.3、FR3.1〜FR3.6、FR6.3、NFR2・NFR7・NFR9・NFR10・NFR11、C3・C5）とそのレビュー（RA R-04・R-05）、`inception/domain-design/components.md`・`decisions.md`（ADR-001・ADR-002）とそのレビュー（DD R-01・R-05・R-09）、質問票 `functional-design-questions.md`（Q1〜Q6）、AWS の資料「CloudWatch Logs Insights query syntax」（`stats` の関数の表、`bin()` の説明、`as` の別名）、今のコード（`parser.rs` の command・function・OTHER_COMMANDS、`evaluator.rs` の Stage・EVALUATION_CHUNK、`natural.rs` の NaturalSort、`fields.rs` の FieldValue・display、`insights_session.rs` の display_rows・display_row_detail・export_source、`compat/compare.rs`）。

## U1 だけの暫定の決まりと、後の作業単位に渡すもの

- **関数の数**：U1 は 6 つの書き方（`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`）。資料にあるほかの関数（`count_distinct`・`pct`・`stddev`・`earliest`・`latest`・`sortsFirst`・`sortsLast`）は U1 では「対応していない」の問題として場所つきで返し（BR1.2）、U2 で足す。
- **`stats` のあとの命令**：U1 では `stats` を最後の命令とし、あとに命令があれば「対応していない」（BR1.6）。U2 で `sort`・`limit` の後置と、`stats` を含むクエリの件数の上限（FR1.7）を決める。
- **コンソールの細部のうち U1 で仮に決めるもの**：鍵のないイベント（BR2.1、Q3 A）、数値でない値（BR2.3・BR2.4、Q4 A）、0 件のときの `by` なしの 1 行（BR2.6、Q2 A）、小数の桁（BR2.8、Q5 A）、`sort` を書かないときのグループの並び（BR2.7）は、資料に書かれていないため SQL と一般的な理解で仮に決め、U4 の確認用プログラムでコンソールと比べて直す。
- **グループの数**：U1 ではグループの数に上限を設けない（`by @message` で 100 万件なら 100 万グループになりうる）。`stats` を含むクエリの件数の上限（FR1.7）とメモリの扱いは U2 で決める。U1 の測定（functional-spec 7 章）には鍵の種類が多いクエリを 1 本含める（FR レビュー R-06）。
- **一覧の一元化**：U1 は今の `OTHER_COMMANDS`・`function()`・補完の固定の候補に `stats`・`by`・`bin`・6 つの関数を足すだけ（暫定の振り分け、units R-02）。QueryCatalog への置き換えと、色分け・エラー文言・README は U2。
- **QueryEvaluator と QueryStore の依存の向き（DD R-09）**：今のコードのまま、QueryStore の evaluate_step が QueryEvaluator の Evaluation を受け取って view の上で 1 ステップ進める（QueryStore → QueryEvaluator の型）。QueryEvaluator は QueryStore を知らず、項目の読み手（EvalItems）だけを受け取る。U1 は向きを変えない。
- **関数の確定した一覧（OQ1）**：U1 の 6 つの書き方は AWS の資料の表（`count()`/`count(*)`/`count(項目)`・`sum(数値の項目)`・`avg(数値の項目)`・`min(項目)`・`max(項目)`、結果は number か項目の値）で確かめた。残りの関数の引き数・結果の型・表示の形は U2 の機能設計で決める（unit-of-work の U1 の注記を改める）。
- **途中結果と中断**：U3。U1 の評価と結果の差し替えは今までどおり取得がすべて終わってから 1 回。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: "`stats <集計>[, <集計>...] [by <鍵>[, <鍵>...]]` を 1 つの命令として読む。集計は `関数(引き数)[ as 名前]`、鍵は `項目[ as 名前]` か `bin(期間)[ as 名前]`"
    category: validation
    applies_to: QueryParser、QueryPlan
    trigger: 命令の先頭の語が stats のとき
    logic: IF 語が `stats`（大文字・小文字は前の作業の決まりどおり区別しない）THEN `,` で区切った集計の並びを読み、`by` があればそのあとに `,` で区切った鍵の並びを読む。読んだものを CommandKind::Stats（StatsCommand）として計画に置く。`by` は命令の語と同じく大文字・小文字を区別しない。関数名の大文字・小文字は資料の書き方（小文字）だけを受け付け、違えば Unsupported の問題（U2 の OQ2 で確かめる）
    violation: 形に合わない語があれば、その語の場所で書き間違い（Syntax）の問題を返す（BR1.7）
    source: FR1.1、FR1.3
  - id: BR1.2
    statement: U1 で読める集計の関数は count(*)・count(項目)・sum・avg・min・max の 6 つの書き方。資料にあるほかの集計の関数は「対応していない」とする
    category: validation
    applies_to: QueryParser
    trigger: 集計の関数名を読んだとき
    logic: IF 関数名が count・sum・avg・min・max THEN 引き数を読む（BR1.3）。ELSE IF 関数名が count_distinct・pct・stddev・earliest・latest・sortsFirst・sortsLast THEN Unsupported の問題（関数名の場所、文言は「この関数にはまだ対応していない」の鍵）。ELSE Unsupported の問題（今の UNSUPPORTED_FUNCTION）
    violation: 問題を返し、取得に入らない（今の読み取りのエラーの流れ）
    source: FR1.2、UG Q2 A、FR6.3
  - id: BR1.3
    statement: 集計の引き数は `*`（count だけ）、決まった項目、JSON の項目、toMillis(@timestamp)。by の鍵は決まった項目、JSON の項目、bin(期間)。演算やほかの関数は「対応していない」
    category: validation
    applies_to: QueryParser
    trigger: 引き数か鍵を読んだとき
    logic: IF count の引き数が `*` THEN AggregateArgument(all)。IF 引き数が項目か toMillis(@timestamp) THEN AggregateArgument(field)。IF sum・avg・min・max の引き数が `*` THEN Syntax の問題（引き数の場所）。IF 鍵が bin( THEN BR1.5 で期間を読む。IF 鍵が toMillis(...) やほかの関数、演算子 THEN Unsupported の問題（その語の場所）
    violation: 問題を返す
    source: FR1.3、Q6 A、RA レビュー R-05
  - id: BR1.4
    statement: 列の名前は `as` があればその名前、なければ書いたとおりの文字。同じ名前が 2 つあれば読み取りのエラー
    category: validation
    applies_to: QueryParser、QueryPlan
    trigger: 集計と鍵を読み終えたとき
    logic: 名前は、`as` の次の語（識別子。前の作業の `fields ... as` は Unsupported のままで、`stats` の `as` だけを読む）か、`stats` のあとの文字を空白をそろえずそのまま（例 `count(*)`、`avg(latency)`、`bin(5m)`、`toMillis(@timestamp)` を含むなら `sum(toMillis(@timestamp))`）。鍵の列の名前と集計の列の名前を合わせて同じものが 2 つあれば Syntax の問題（2 つ目の場所）
    violation: 問題を返す
    source: FR1.1、FR3.1
  - id: BR1.5
    statement: bin() の期間は 1 以上の整数と単位 s・m・h・d。ほかの単位と 0 は受け付けない
    category: validation
    applies_to: QueryParser（BinPeriod）
    trigger: bin( を読んだとき
    logic: IF 中身が `<整数><単位>` で単位が s・m・h・d かつ整数が 1 以上 THEN BinPeriod（millis = 整数 × 1000 / 60,000 / 3,600,000 / 86,400,000）。IF 単位が ms・w・mo・q・y THEN Unsupported の問題（期間の場所）。IF 整数が 0 か形が違う THEN Syntax の問題。閉じ括弧がなければ UNCLOSED_PAREN
    violation: 問題を返す
    source: FR2.1、Q1 A
  - id: BR1.6
    statement: U1 では stats は 1 つだけで、最後の命令。stats のあとに命令があれば「対応していない」
    category: validation
    applies_to: QueryParser
    trigger: stats を読み終えて `|` が続くとき、または 2 つ目の stats を読んだとき
    logic: IF stats のあとに命令がある THEN Unsupported の問題（その命令の語の場所、文言は「stats のあとの命令にはまだ対応していない」の鍵）。stats の前の fields・filter・sort・limit は今までどおり読む（sort・limit が stats の前にあるときの扱いはコンソールに合わせて U2 で決める。U1 ではそのまま読み、評価では stats が sort・limit の結果のイベントを集計する）
    violation: 問題を返す
    source: FR1.4（U1 の範囲）、UG Q2
  - id: BR1.7
    statement: stats の書き間違い（by のあとに何もない、閉じていない括弧、引き数の数の違い、`,` のあとに何もない）は、取得の前に場所つきのエラーにする
    category: validation
    applies_to: QueryParser
    trigger: stats を読んでいるとき
    logic: IF `by` のあとに鍵がない THEN Syntax（by の場所、文言「by のあとに鍵がない」）。IF 括弧が閉じていない THEN 今の UNCLOSED_PAREN。IF count 以外で引き数がない・2 つ以上 THEN Syntax（括弧の場所、文言「引き数の数が違う」）。IF `,` のあとに何もない THEN Syntax（`,` の場所）。IF 集計が 1 つもない（`stats by level`）THEN Syntax（stats の場所）
    violation: 問題を返し、エラーの場所にカーソルを動かす（今の読み取りのエラーの流れ）
    source: FR6.3
  - id: BR1.8
    statement: bin( の中だけ、数と単位を 1 つの語として読む。ほかの場所の数の読み方（5m などを INVALID_NUMBER にする今の決まり）は変えない
    category: validation
    applies_to: QueryParser（字句解析）
    trigger: 鍵の位置で `bin` と `(` を読んだとき
    logic: 今の字句解析器 number() は数字に英字が続く語（5m・30s・1d・5ms）を字句の段で INVALID_NUMBER にするため、BR1.5 に届かない（FR レビュー R-03）。`bin(` の直後だけ、閉じ括弧までの文字を 1 語として取り出し、BR1.5 で数と単位に分ける。単位が ms・w・mo・q・y なら Unsupported、数が 0・空・数でない・単位がない・知らない単位なら Syntax（INVALID_BIN_PERIOD の鍵）。数の上限は 1,000,000（超えたら Syntax。count × millis が i64 を超えないため、FR レビュー R-05）。`filter @timestamp > 5m` の今のエラーは変えない
    violation: 問題を返す
    source: FR2.1、Q1 A、FR レビュー R-03・R-05
  - id: BR1.9
    statement: as を付けない列の名前は、元の文から切り出した文字（関数名から閉じ括弧まで、または鍵の語）。改行と連続する空白は 1 つの空白にし、コメントは含めない
    category: validation
    applies_to: QueryParser
    trigger: 集計と鍵の列の名前を決めるとき（BR1.4）
    logic: 語の span を使って元の文から切り出す（トークンの連結ではない）。中の改行・タブ・連続する空白は 1 つの空白に、前後の空白は落とす。`#` のコメントは字句の段で落ちているため含まれない。例：`count( * )` は `count( * )`、`avg(latency)` は `avg(latency)`
    violation: なし
    source: FR1.1、FR3.1、FR レビュー R-03
  - id: BR3.5
    statement: 評価の段階の機械に Aggregate の段を足す。stats の前の命令を今の段で適用したあと、残った位置を 10,000 行ずつ集計し、止められない短い仕上げで集計の行を作る
    category: constraint
    applies_to: QueryEvaluator（Evaluation、Stage、EvalOutput）、QueryRunner
    trigger: 計画に Stats があるとき
    logic: Stage に `Aggregate { next, aggregator }` を足す（stats の前の命令がすべて終わったら入る）。各ステップで `rows` の次の 10,000 位置のイベントを aggregator に渡し、ステップの間で今の on_evaluation_progress（続けるか）を聞く。残りがなくなったら `aggregator.finish()` で行を作り Done に移る（グループの並べ替えと文字の丸めはこの 1 段で、途中で止めない。グループ数に比例する短い処理）。aggregates のときは finish_commands の既定の 10,000 件の打ち切りと Columns の段（空の列を落とす）を行わない。passes_planned に Aggregate を 1 つの塊の段として数え、進み具合が 0 のままにならないようにする。EvalOutput は `Events { rows: Vec<usize>, columns: Vec<FieldRef>, ... }` か `Aggregates { columns: Vec<String>, rows: Vec<Vec<CellValue>>, ... }` の列挙型にする。QueryRunner は Events なら今のとおり copy_rows で写しを作って QueryResult にし、Aggregates なら行をそのまま QueryResult にする（copy_rows を呼ばない）。そのため QueryRunner の変更は「形による分岐 1 つ」であり、流れの順番は変えない（BR3.1 を改める）
    violation: なし
    source: FR1.8、FR3.6、NFR2、ADR-001・002、FR レビュー R-01
  - id: BR3.6
    statement: 結果の列は列挙型で持つ。events は今の FieldRef の並び、aggregates は列の名前の並び
    category: policy
    applies_to: QueryResult、ExportSnapshotSource.LogAnalytics、InsightsSession
    trigger: 結果を作るとき、行の文字を作るとき
    logic: `ResultColumns::Events(Vec<FieldRef>)` / `ResultColumns::Aggregates(Vec<String>)`。events の行の文字は今の row_cells(&[FieldRef], ...) のまま作る（JSON の項目と toMillis(@timestamp) を名前から復元しない）。見出しの文字（ResultSummary.columns）は両方とも名前の文字列にする。ExportSnapshotSource::LogAnalytics の columns もこの列挙型にする
    violation: なし
    source: FR3.4、BR5.1、FR レビュー R-02
  - id: BR2.1
    statement: filter で残ったイベントを by の鍵の値の組ごとにグループにする。鍵の項目を持たないイベントは「なし」の鍵として 1 つのグループにまとめる
    category: calculation
    applies_to: StatsAggregator
    trigger: 評価で stats の命令に達し、イベントを塊ごとに受け取ったとき
    logic: 鍵ごとに値を読む：決まった項目と JSON の項目は EventFieldReader の値（Missing / Number / Text / Boolean）、bin は BR2.5 の区切り。鍵の比べ方は「表に出す文字」で決める：鍵の値を BR2.8 の文字（Number は整数か 4 桁、Boolean は今の display と同じ `1` / `0`、Text はそのまま）にし、文字が同じイベントは同じ AggregateGroup に入れる（数の 200 と文字列の "200" は同じグループ。-0 と 0 も同じ。FR レビュー R-04）。鍵のセルの種類は最初に見た値の種類。`by @timestamp` の鍵は timestamp のセル（表示はタイムゾーンで整形、BR4.2）、`by @logStream`・`@log`・`@message` は text。Missing はほかのどの値とも違い、Missing どうしは同じ（複数の鍵のときは鍵ごとに区別）。by がないときは全体で 1 つのグループ
    violation: なし（イベントを落とさない）
    source: FR1.3、FR1.6、Q3 A
  - id: BR2.2
    statement: count(*) はグループのイベントの数、count(項目) はその項目を持つイベントの数
    category: calculation
    applies_to: StatsAggregator（Accumulator）
    trigger: イベントをグループに入れたとき
    logic: count(*) は 1 足す。count(項目) は項目の値が Missing でなければ 1 足す（空の文字列も数える）。結果は integer
    violation: なし
    source: FR1.2、Q4 A
  - id: BR2.3
    statement: sum・avg は数に読める値だけを足し、それ以外と項目のないイベントは無視する。足せた値がなければ値なし
    category: calculation
    applies_to: StatsAggregator（Accumulator）
    trigger: イベントをグループに入れたとき
    logic: 値が Number ならそのまま、Text なら前の作業の数の読み方（10 進の整数・小数、前後の空白なし）で読めれば数として足し、読めなければ無視する。Boolean と Missing は無視する。sum は合計、avg は合計 ÷ 足した個数。足した個数が 0 なら CellValue(missing)。値は f64 で持ち、整数に収まるかは表示で決める（BR2.8）
    violation: なし（エラーにしない）
    source: FR1.6、Q4 A
  - id: BR2.4
    statement: min・max は数は数として、文字列は文字列として比べ、数と文字列が混ざるときは sort と同じ順（数でない値 → 数、文字は自然順）で比べる
    category: calculation
    applies_to: StatsAggregator（Accumulator）
    trigger: イベントをグループに入れたとき
    logic: 値が Missing なら無視する。それ以外は今の NaturalSort の順（SortValue）で今までの最小・最大と比べ、小さい（大きい）方を残す。同じ順位の値は先に見た方を残す。結果の CellValue は残った値の型（Number は integer か decimal、Text は text、Boolean は鍵と同じ `1` / `0` の text、@timestamp の項目なら timestamp、toMillis(@timestamp) なら integer）。値が 1 つもなければ missing
    violation: なし
    source: FR1.6、Q4 A
  - id: BR2.5
    statement: bin(期間) の区切りは、@timestamp のミリ秒を期間のミリ秒で切り捨てたもの（UTC のエポックの倍数）。イベントのない区切りの行は出さない
    category: calculation
    applies_to: StatsAggregator
    trigger: 鍵に bin があるとき
    logic: 区切りの始まり = floor(timestamp ÷ millis) × millis（負の時刻でも床で切り捨てる）。これを鍵の値（CellValue(timestamp)）にする。1d の境目は UTC の 0:00 で、画面のタイムゾーンには依存しない（FR2.3）。グループはイベントがあった区切りだけに作られるため、空の区切りの行は自然に出ない
    violation: なし
    source: FR2.1、FR2.2、FR2.3、A8、Q1 A
  - id: BR2.6
    statement: by を書かないとき、イベントが 0 件でも 1 行を出す（count は 0、ほかは値なし）。by を書いたときは 0 行
    category: calculation
    applies_to: StatsAggregator
    trigger: 評価が終わったとき
    logic: IF groupKeys が空 AND グループが 1 つもない THEN 空の AggregateGroup を 1 つ作って行にする（count(*)・count(項目) は 0、sum・avg・min・max は missing）。IF groupKeys がある THEN グループの数だけ行にする（0 なら 0 行、FR3.2）
    violation: なし
    source: FR1.3、FR3.2、RA レビュー R-04、Q2 A
  - id: BR2.7
    statement: 集計の行の並びは、鍵の値の昇順（鍵が複数なら書いた順に比べる）。「なし」の鍵は最後。by がなければ 1 行
    category: policy
    applies_to: StatsAggregator
    trigger: 評価が終わって行を作るとき
    logic: 鍵の値を今の NaturalSort の順で昇順に並べる（timestamp と数は数として、文字は自然順。Missing は最後）。同じ値はない（グループの鍵は一意）。U1 の仮の決まり（資料にない）。U2 で sort の後置を足したときは sort が優先し、U4 でコンソールの既定と比べる
    violation: なし
    source: FR3.1（並びは要件にないため仮）、OQ2
  - id: BR2.8
    statement: 集計値の文字は、整数に収まる値は整数のまま、小数は小数点以下 4 桁に四捨五入して末尾の 0 を落とす。count は常に整数
    category: calculation
    applies_to: StatsAggregator（CellValue）、InsightsSession（文字にするとき）
    trigger: 結果を作るとき、表示・書き出しの文字を作るとき
    logic: 丸めは 1 つの純粋な関数 format_aggregate(f64) で行い、セルの値（f64）は丸めずに持って、文字にするときだけ丸める。手順：(1) 値が有限でなければ値なし（null。inf・NaN は sum の桁あふれなどでしか起きない）。(2) r = (x × 10,000).round() ÷ 10,000（Rust の f64::round は half away from zero）。(3) r が -0 なら 0。(4) r が整数で |r| < 2^53 なら整数の文字（`600`）、そうでなければ小数点以下 4 桁の文字から末尾の 0 と不要な小数点を落とす（200.3333、1.5）。|r| ≥ 2^53 の整数は f64 の最短の表記。境界の値（0.00005、-0.00004、2^53、1e20、0.1 + 0.2）はテストの表に入れる（FR レビュー R-05）。sum・avg は常に decimal として持ち、整数に収まれば整数の文字になる。count は integer。text はそのまま。timestamp は BR4.2。missing は値なし（null）。表・行の詳細・書き出し・確認用プログラムは同じ関数で文字を作る（A7）
    violation: なし
    source: FR1.5、A7、Q5 A
  - id: BR2.9
    statement: 集計はイベントを塊ごとに受け取って累積し、塊の間で止める信号を確かめる
    category: constraint
    applies_to: StatsAggregator、QueryEvaluator
    trigger: 評価の stats の段
    logic: 今の EVALUATION_CHUNK（10,000 行）ごとに、filter で残った位置のイベントを StatsAggregator に渡す。塊ごとに on_evaluation_progress で続けるか聞き、止める信号があれば結果を捨てる（今の流れのまま）。グループの数・累積の中身はメモリの中だけに置く
    violation: なし
    source: NFR2、FR1.8
  - id: BR3.1
    statement: 結果は 1 つの QueryResult で、形（events / aggregates）、列の名前の一覧、形つきの行の写しを持つ。計画に stats があれば aggregates、なければ events
    category: policy
    applies_to: QueryEvaluator、QueryStore（copy_rows）、QueryRunner、TauriSink
    trigger: 評価が終わって結果を作るとき
    logic: IF 計画に Stats がある THEN shape = aggregates、columns = BR3.3（BR3.6 の Aggregates）、rows = 集計の行（cells）。ELSE shape = events、columns = 今の columns()（BR3.6 の Events。空の列を落とす今の決まりのまま）、rows = 今の copy_rows のイベントの写し。行はどちらも写しで、保持を位置で指さない。QueryStore・TauriSink・CompatCheck は型の変更に追従するだけ、QueryRunner は形による分岐を 1 つ足す（BR3.5）。どれも流れの順番は変えない（units R-01、FR レビュー R-01）
    violation: なし
    source: FR3.6、ADR-001、DD レビュー R-01、C3
  - id: BR3.2
    statement: 計画に stats があるとき、filter（と stats の前の sort・limit）で残ったイベントを集計の入力にし、fields は結果の列に効かない
    category: policy
    applies_to: QueryEvaluator
    trigger: 評価で命令を順に適用するとき
    logic: 今の流れ（Ordering → 命令を書いた順）で stats の前の命令を適用し、stats に達したら残った位置のイベントを BR2.9 で StatsAggregator に渡す。fields で選んだ列は集計の行には出さない（集計の行の列は鍵と集計値だけ、FR1.5）。空の列を落とす Columns の段は aggregates では行わない（鍵が全部「なし」でも列は残る）
    violation: なし
    source: FR1.4、FR1.5、FR3.1
  - id: BR3.3
    statement: 集計の結果の列は、by の鍵の列をクエリに書いた順、次に集計の列を書いた順。名前は BR1.4
    category: policy
    applies_to: QueryPlan、QueryEvaluator
    trigger: 結果の列を決めるとき
    logic: columns = groupKeys の columnName の並び ++ aggregates の columnName の並び。行の cells はこの順
    violation: なし
    source: FR3.1
  - id: BR3.4
    statement: aggregates の件数は、matchedCount = 行（グループ）の数、evaluatedCount = 評価したイベントの数、truncatedAt = なし（U1）
    category: policy
    applies_to: QueryEvaluator、InsightsSession（ResultSummary）
    trigger: 結果を作るとき
    logic: 見出しの件数の文は「n 行」を形に合わせて出す（events は今の「n 件」のまま）。打ち切りの注記は aggregates では出ない（上限は U2、FR1.7）
    violation: なし
    source: FR3.2、FR1.7（U1 の範囲外）
  - id: BR4.1
    statement: 画面に渡す見出し（ResultSummary）は形を持ち、行（QueryRow）は values を必ず持ち、イベントの項目は events のときだけ持つ
    category: policy
    applies_to: InsightsSession、TauriApp、ApiBridge
    trigger: 見出しと行を読み出すとき（今の get_query_rows / get_query_row_detail）
    logic: ResultSummary.shape = QueryResult.shape。display_rows は events なら今のとおり（event・displayTime・logGroupName・values）、aggregates なら values（BR2.8・BR4.2 の文字）だけで event・displayTime・logGroupName は持たない。コマンドの名前と引き数（runId, offset, limit / runId, index）は変えない。古い runId の答えを捨てる決まりは今のまま
    violation: なし
    source: FR3.1、FR3.6、ADR-001、ADR-004（U1 の範囲）
  - id: BR4.2
    statement: 時刻のセル（bin の鍵、@timestamp の min・max）の文字は、表示と書き出しのときに画面のタイムゾーンで、今の @timestamp の表示と同じ形に整形する
    category: calculation
    applies_to: InsightsSession（display_rows・display_row_detail・export_source）
    trigger: 文字を作るとき
    logic: CellValue(timestamp) は今の format_display_in_zone(ミリ秒, zone)。タイムゾーンを切り替えると文字だけが変わり、行の数と集計値は変わらない。toMillis(@timestamp) の集計（integer）は数のまま
    violation: なし
    source: FR1.5、FR2.3、DD レビュー R-05
  - id: BR4.3
    statement: 集計の行を開いたときの詳細は、列の順に（列の名前, セルの文字）だけ。イベントの全文（@message の整形）は出ない
    category: policy
    applies_to: InsightsSession（display_row_detail）、QueryResultTable
    trigger: 行を開いたとき
    logic: aggregates なら fields = columns と cells を順に組にしたもの（値なしは null）。events は今のとおり
    violation: なし
    source: FR3.3
  - id: BR4.4
    statement: 集計の結果も CSV / Markdown に書き出せる。列は表の見出し、セルは表と同じ文字
    category: policy
    applies_to: InsightsSession（export_source）、ExportJob、ExportFormatter（変えない）
    trigger: ［Export］を押したとき
    logic: export_source は形つきの行と列の名前と zone を ExportSnapshotSource::LogAnalytics に渡す。書き出しの文字は events なら今のとおり（@message は全文）、aggregates なら BR2.8・BR4.2 の文字（表と同じ関数）。既定のファイル名・形式・出し方・通知・可否（実行中は Running、結果がなければ NoResult）は変えない
    violation: なし
    source: FR3.4
  - id: BR4.5
    statement: 結果の表は 2 つの形を同じ場所に出す。見出し・行・詳細は受け取った見出しの形に従い、形を利用者が選ぶ操作はない
    category: policy
    applies_to: QueryResultTable、LogAnalyticsPage
    trigger: 見出しを受け取ったとき
    logic: 表の部品は見出しの shape を受け取り、それで分岐する。columns をそのまま見出しに、values をそのままセルに出す。events は今のとおり（列の名前が `@message` なら 1 行目だけ、`@timestamp` なら見出しにタイムゾーンを付ける：cellText・columnClass）。aggregates では列の名前で表示を変えない（`by @message` の鍵の列も全文、`by @timestamp` の見出しにもタイムゾーンを付けない。セルの文字は中核が整形済み）。列が多いときは表を横にスクロールできる（今の表の作りで足りなければ Cloudscape の Table の設定で、FR3.5）。0 行は「結果は 0 行」の表示（今の 0 件の表示を形に合わせた文言に）。aggregates で 1 行（BR2.6）のときは普通の 1 行
    violation: なし
    source: FR3.5、FR3.6、FR3.2、NFR11
  - id: BR4.6
    statement: 新しい文言（集計の 0 行、stats の読み取りのエラー、行の見出しの「n 行」）は英語と日本語を用意し、読み上げの名前は今の表と同じ水準で付ける
    category: policy
    applies_to: MessageCatalog、QueryResultTable
    trigger: 文言を出すとき
    logic: MessageCatalog に鍵を足す。QueryParser の問題の message_key は今の keys と同じ仕組みで画面側の鍵に写す
    violation: なし
    source: NFR9、NFR10
  - id: BR5.1
    statement: filter・fields・sort・limit だけのクエリの結果（列・行・並び・件数・書き出し）は、前と同じ
    category: constraint
    applies_to: QueryEvaluator、InsightsSession、QueryResultTable、ExportJob、CompatCheck
    trigger: 計画に stats がないとき
    logic: shape = events の道は今のコードのまま。既存のテストは型の変更に追従するだけで期待値を変えない。CompatCheck は events の結果が今までどおり比べられるように型だけ合わせる
    violation: 既存のテストが落ちたら直す（期待値を変えない）
    source: C5、FR1.8
  - id: BR5.2
    statement: 集計は手元で行い、AWS の呼び出し（DescribeLogGroups・DescribeLogStreams・GetLogEvents の取得の流れ）は変えない
    category: constraint
    applies_to: QueryRunner、FetchCoordinator（変えない）
    trigger: 常に
    logic: 取得の流れ・足りない分だけの取得・キャッシュは今のまま。Logs Insights の API は呼ばない
    violation: なし
    source: FR1.8、C1、NFR5
```

## ルールの要約

| ID | 分類 | 要点 | 出典 |
|---|---|---|---|
| BR1.1 | validation | `stats 集計, ... by 鍵, ...` の読み方 | FR1.1 |
| BR1.2 | validation | U1 の 6 つの書き方。ほかの関数は Unsupported | FR1.2 |
| BR1.3 | validation | 引き数は `*`・項目・toMillis、鍵は項目・bin | FR1.3、Q6 |
| BR1.4 | validation | 列の名前（as か書いたとおり）、重複はエラー | FR3.1 |
| BR1.5 | validation | bin の期間は 1 以上の s・m・h・d | FR2.1、Q1 |
| BR1.6 | validation | stats は最後の命令で 1 つ（U1） | FR1.4 |
| BR1.7 | validation | 書き間違いの場所つきのエラー | FR6.3 |
| BR1.8 | validation | bin( の中だけ数と単位を 1 語で読む、上限 | FR2.1 |
| BR1.9 | validation | as なしの列の名前は元の文の切り出し | FR3.1 |
| BR2.1 | calculation | 鍵の値の組でグループ化、鍵なしは 1 グループ | FR1.6、Q3 |
| BR2.2 | calculation | count(*) と count(項目) | FR1.2 |
| BR2.3 | calculation | sum・avg は数に読める値だけ | FR1.6、Q4 |
| BR2.4 | calculation | min・max は sort と同じ順で比べる | FR1.6、Q4 |
| BR2.5 | calculation | bin はエポックの倍数に切り捨て、空のビンは出ない | FR2.1〜2.3 |
| BR2.6 | calculation | by なしの 0 件は 1 行、by ありは 0 行 | FR3.2、Q2 |
| BR2.7 | policy | 行の並びは鍵の昇順（仮） | OQ2 |
| BR2.8 | calculation | 小数 4 桁の丸め、整数はそのまま | FR1.5、Q5 |
| BR2.9 | constraint | 塊ごとの累積と止める信号 | NFR2 |
| BR3.1 | policy | 形つきの QueryResult、行は写し | ADR-001 |
| BR3.2 | policy | stats は filter の残りを集計、fields は列に効かない | FR1.4 |
| BR3.3 | policy | 列は鍵 then 集計、書いた順 | FR3.1 |
| BR3.4 | policy | 件数の意味（行の数）、打ち切りなし | FR3.2 |
| BR3.5 | constraint | 評価の機械の Aggregate の段、EvalOutput の形、runner の分岐 | FR1.8、FR3.6 |
| BR3.6 | policy | 結果の列は列挙型（FieldRef か名前） | FR3.4 |
| BR4.1 | policy | 見出しに形、行は values 必須 | FR3.6 |
| BR4.2 | calculation | 時刻のセルはタイムゾーンで整形 | FR2.3 |
| BR4.3 | policy | 集計の行の詳細は鍵と集計値だけ | FR3.3 |
| BR4.4 | policy | 集計の行の書き出し | FR3.4 |
| BR4.5 | policy | 同じ表に 2 つの形、横スクロール、0 行 | FR3.5、FR3.6 |
| BR4.6 | policy | 文言の対訳と読み上げ | NFR9、NFR10 |
| BR5.1 | constraint | stats のないクエリは前と同じ | C5 |
| BR5.2 | constraint | AWS の呼び出しは変えない | FR1.8、C1 |
