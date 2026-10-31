# Domain Design — 質問

元にしたもの：要件（`inception/requirements-analysis/requirements.md` の FR1〜FR7、NFR1〜NFR12、C1〜C5、A1〜A8、OQ3・OQ5）、要件のレビュー（`reviews/review-01.md` の R-01・R-02・R-04・R-05・R-07）、今のコードの知識ベース（`codekb/local-sights/` の component-inventory・architecture・code-quality-assessment）、前の作業 261017-insights-query の部品の一覧（`QueryPlanner`・`QueryEvaluator`・`QueryRunner`・`QueryStore`・`InsightsSession`・`TauriQuerySink`・画面の `InsightsView`・`ResultBody`）。

決まっていて聞き直さないこと：中核（判断）・Tauri アプリ（配線）・画面（表示だけ）の分け方を保つ（C3）。AWS 呼び出しは今の `FetchCoordinator` と読み取りの 3 API だけ（C1）。`stats` の集計は手元で評価する（FR1.8）。集計関数の確定した一覧とコンソールの細部は Functional Design で AWS の資料から起こす（OQ1・OQ2）。

## Q1. 集計の結果の行を、結果の型の中でどう表しますか？（A2、R-02、FR3.6）

今の結果は `QueryResult { columns: Vec<FieldRef>, rows: Vec<ResultRow { event, log_group_name }> }` で、1 行 = 1 イベントの写しに固定されています。画面の表・行の詳細・書き出し（`ExportSnapshotSource::LogAnalytics { columns, rows }`）・確認用プログラム（`compat_check`）はすべてこの形を前提にしています。集計の行は元のイベントを持たず、列も `by` の鍵と集計値になります。

- A. 1 つの `QueryResult` のまま、`columns` を「列の名前の一覧」に広げ、`rows` を列挙型（イベントの行 ／ 集計の行＝セルの値の列）にする。画面・書き出し・確認用プログラムはその区別で分岐する（結果を受け取る場所は 1 つのまま。形が切り替わる時点も 1 か所で決まる）
- B. 集計の結果は別の型 `StatsResult` にし、Tauri の値も画面の表の部品も別に作る（イベントの結果の部品に手を入れない）
- C. イベントの行も集計の行も「セルの値の列」に統一し、イベントの行だけ行の詳細のために元イベントへの参照を添える（表と書き出しは 1 つの形だけを扱う）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q2. `stats` の集計の評価は、どの部品が受け持ちますか？（FR1.2〜FR1.7、NFR7）

今の `QueryEvaluator` は 1 イベントずつ `filter` の式を評価し、残った行の添字を返し、`sort`・`limit` を行に対して行います。集計は、関数ごとの累積（count・sum・avg・pct・stddev・earliest・latest・sortsFirst・sortsLast など 13 種類以上）、`bin()` の区切り、`by` の鍵によるグループ化と、独立した純粋なロジックの塊です。

- A. 集計の中身（関数ごとの累積、`bin()` の区切り、グループ化）を新しい部品 `StatsAggregator` に切り出す。`QueryEvaluator` は `filter` で残ったイベントを `StatsAggregator` に渡し、できた集計の行に対して `sort`・`limit` を行う（関数の種類が多いためテストを部品の中で閉じられる）
- B. `QueryEvaluator` を広げ、`filter` → `stats` → `sort` → `limit` を 1 つの部品の中で扱う（部品は増やさない）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. 途中結果の評価は、何を元に、どのやり方で行いますか？（OQ5、A5、FR4.8、NFR1・NFR2）

今は取得中のイベントを範囲ごとの保持（`Arc<Mutex<EventTimeline>>`）にため、範囲が取得し終わったら `QueryStore::settle_range` で確定し、取得がすべて終わってから `build_view` → `evaluate_step`（10,000 行ずつ）→ `copy_rows` を 1 回だけ行います。最終結果で 2 回数えない・落とさない決まり（FR4.8）は、この「確定」の仕組みで守られています。

- A. 確定の仕組みは変えない。一定の間隔ごとに、`QueryStore` の確定済みの分と取得中の保持の写しを合わせた「途中の view」を作り、同じ評価器で全体を評価し直して差し替える（`QueryStore` に途中の view を作る入口を足す。最終結果は今までどおり確定済みの分だけから作るので FR4.8 はそのまま守られる）
- B. 届いたページごとに `QueryStore` に段階的に確定し（範囲の途中までの状態を `QueryStore` が持つ）、評価はいつも確定済みの全体に対して行う（保持の写しは要らないが、`QueryStore` の確定と「取得済みの範囲」の記録を分ける作り直しが要る）
- C. 増分評価：`filter`・`fields` だけのクエリは届いた分だけを評価して行を足し、`sort`・`limit`・`stats` を含むクエリは A と同じく全体を評価し直す（評価器に 2 つの入口。速いが、2 つの道を保つ）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. 途中結果は、中核からアプリ、画面へどう届けますか？（FR4.4、FR4.5、A4）

ログの画面では、取得の進み具合を `FetchSink::on_batch` → `fetch-progress` イベント（`timelineVersion` つき）→ 画面が `get_rows` で取りに行く、という流れです。クエリの画面の `TauriQuerySink` は進み具合を 250 ms ごとに押し出しますが、途中結果の入口はありません。

- A. 中核の `QuerySink` に途中結果の通知（版の番号と結果）を足し、`TauriQuerySink` が間隔を絞って（250 ms を起点に設計で決める）`query-partial-result` のようなイベントで画面に送る。画面は版の番号が新しいときだけ差し替える（A4）
- B. 中核の `QuerySink` に途中結果の通知を足し、Tauri 側は「版が進んだ」ことだけをイベントで知らせ、画面が command で結果を取りに行く（ログの画面の `fetch-progress` → `get_rows` と同じ形。結果が大きいときイベントに載せない）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: B

## Q5. `filter`・`fields` だけのクエリの途中結果の並び順は、どちらにしますか？（R-01、FR4.2、前の FR3.5）

要件のレビュー R-01 の指摘：FR4.2 は「新しい行を末尾に足し、既に出ている行は動かない」と「時刻順の並びは最終結果と同じ」の両方を書いていますが、範囲は時刻の古い側にも新しい側にも並行して取得されるため、両立しません。設計でどちらかに決めます。

- A. 途中も最終結果と同じ並び（前の FR3.5 の決まり）に保つ。新しい行が途中に挿入され、既に出ている行が動くことを許す（Q3 で全体を評価し直す作りと同じ道を使える。利用者の操作の保持は FR4.4 のとおりできるだけ行う）
- B. 途中は到着順に末尾に足し、既に出ている行は動かさない。最終結果になった時点で 1 回だけ並べ直す（途中は読みやすいが、最後に並びが変わる。表示済みの行を覚える仕組みが要る）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q6. 対応する命令・関数の一覧は、どこに 1 つ持ちますか？（FR6.2、NFR7）

今は、パーサ（`parser.rs` の `OTHER_COMMANDS`・`KEYWORDS`・`function()`）、画面の色分け（`src/analytics.ts`）、エラー文言（`messages.ts` の `insights.error.supportedList`）、補完の候補（`InsightsSession::completion_candidates`）、README の表と、4〜5 か所に別々にあります。

- A. 中核に 1 つの一覧（`QueryCatalog`：命令・集計関数・その他の関数・キーワード）を置き、パーサと補完はそれを参照し、Tauri の command で画面に渡して色分けとエラー文言もそこから作る。README の表はテストで一覧との一致を確かめる
- B. 今のまま複数の場所に持ち、すべての場所の一致をテストで確かめるだけ（部品は増やさないが、変えるときは 4〜5 か所を触る）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q7. 前回の結果と今回の結果の形が違うとき（例：前回は 1 行 = 1 イベント、今回は `stats`）、表はいつ切り替えますか？（R-02、FR4.7、A3）

FR4.7 では、実行を始めても前回の結果を「前回の結果」の印つきで最初の途中結果が届くまで残します。形が違う 2 つの結果が表に混ざらないように、切り替える時点を 1 つ決めます。

- A. 前回の結果は前回の形のまま印つきで残し、最初の途中結果が届いた時点で、見出し・行の詳細・件数表示・［Export］の列をまとめて今回の形に切り替える（結果の型（Q1）が形を持つので、画面は受け取った結果の形に従うだけ）
- B. 形が違う実行のときだけ、実行を始めた時点で前回の結果を消して空の表にし、途中結果を待つ（形が同じときは A と同じ）
- C. 実行を始めた時点で常に前回の結果を消す（FR4.7 の「前回の結果を残す」をやめる。要件の見直しになる）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A
