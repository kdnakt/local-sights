# Business Rules — U3 途中結果と中断後の結果（u3-partial-results）

上流：`inception/units-generation/unit-of-work.md`（U3）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.10、FR5.1〜FR5.4、NFR1・NFR2・NFR9・NFR10、C5、A3・A4・A5）とそのレビュー（RA R-01・R-06・R-07・R-08）、`inception/domain-design/components.md`・`decisions.md`（ADR-003・ADR-004・ADR-006）とそのレビュー（DD R-02・R-03・R-04・R-08）、U1 の機能設計（BR3.1・BR3.5）、U2 の機能設計（BR3.1 の仕上げも止められる）、質問票 `functional-design-questions.md`（Q1〜Q4）、今のコード（`runner.rs`・`store.rs`・`insights_session.rs`・`src-tauri/src/lib.rs` の TauriQuerySink、`src/components/QueryResultTable.tsx`・`LogAnalyticsPage.tsx`、`src/hooks/expansionState.ts`）。

## 要件の読み替え

- **FR4.2（RA R-01）**：要件の「新しい行を末尾に足し、既に出ている行は動かない」は、ADR-003（DD Q5 A）に従い「途中も最終結果と同じ並びに保ち、新しい行が途中に入ることがある」と読み替える（BR2.2）。要件の受け入れ基準「2 つ目・3 つ目の範囲で行が末尾に増える」は「2 つ目・3 つ目の範囲を取得するにつれて行が増え、並びはいつも最終結果と同じ決まり」と読み替える。
- **C5 の例外（RA R-07）**：クエリの画面の中断で評価しない前の決まり（前の FR2.6）は、FR5.1 で改める（BR3.1）。ログ管理の画面の中断は変えない。
- **中断の印を書き出しに含めない理由（RA R-08）**：FR5.2 のとおり印はファイルに含めない。コンソールの書き出しにも印がなく、書き出しの形を前の作業とそろえるため。代わりに、中断後の結果の書き出しの完了の通知に「一部のログだけの結果です」と添える（BR3.5）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 取得中、前の途中の評価が終わってから最低 1 秒あけて、次の途中結果の評価を始める
    category: policy
    applies_to: QueryRunner
    trigger: 取得の最中（最初のページが届いたあと）
    logic: 途中の評価は取得の流れとは別のタスク（blocking スレッド）で行い、取得を待たせない（NFR1）。前の途中の評価が終わった時刻から 1 秒（PARTIAL_EVALUATION_INTERVAL。作る段階で測って調整してよい）たち、かつ取得済みの件数がその途中結果のときより増えていれば、次を始める。評価に 1 秒より長くかかったら、終わり次第すぐ次。最初のページが届いたあと、最初の途中の評価はすぐ始める（RA R-06：最初のページのあと、1 回の評価の時間で最初の途中結果が出る）。取得済みがすべて確定済みで AWS を呼ばないとき（FR4.9）は途中の評価をしない
    violation: なし
    source: FR4.1、FR4.4、FR4.9、NFR1・NFR2、Q1 A、RA レビュー R-06
  - id: BR1.2
    statement: 取得中の範囲の保持は、実行の間だけの登録先（InflightRegistry）に登録し、確定したら外す
    category: constraint
    applies_to: QueryRunner、QueryStore
    trigger: 範囲の取得を始めたとき、settle_range に渡すとき
    logic: fetch_range が今のように範囲ごとの holder を作るときに、InflightRegistry にも登録する。取得が終わったら、settle_range に渡すのと同じ鍵の中で登録から外す（外す前と後で、同じイベントが途中の view に 2 回入ったり落ちたりしない、FR4.8）。失敗・中断で終わった範囲は確定せず（FR5.3）、登録から外して捨てる（中断後の評価のときは BR3.1 で写しを使ってから捨てる）
    violation: なし
    source: FR4.8、FR5.3、DD レビュー R-02
  - id: BR1.3
    statement: 途中の view は、確定済みの分と取得中の分の写しを、結果の並びの順に合わせて作る
    category: calculation
    applies_to: QueryStore
    trigger: 途中の評価を始めるとき
    logic: 取得中の各 holder の鍵を 1 つずつ短く取り、中のイベントを写す（鍵を持ったまま評価しない、NFR2）。写しのイベントには、その範囲の StreamSequences の続きの番号を仮に振る（確定のときに振る番号と同じ規則なので、並びが最終結果と食い違わない）。確定済みの分の ViewEntry と写しの ViewEntry を合わせ、（時刻, ロググループ, ストリーム, ストリーム内の順番）の順に並べる。キャッシュから読んだ分は add_cached で確定済みに入るため、確定済みの分として含まれる
    violation: なし
    source: FR4.1、FR4.8、ADR-003、DD レビュー R-02
  - id: BR1.4
    statement: view は epoch とは別の viewVersion を持ち、評価と写しは (epoch, viewVersion) が今の view と同じときだけ進める
    category: constraint
    applies_to: QueryStore（build_view・build_partial_view・evaluate_step・copy_rows）
    trigger: view を作るとき、評価と写しのとき
    logic: view を作るたびに viewVersion を 1 つ進める（最終の view も）。evaluate_step と copy_rows は (epoch, viewVersion) が食い違えば何もせず失敗を返す（今の epoch だけの照合を広げる）。取得が終わったとき・中断したときは、動いている途中の評価に止める信号を出して終わるのを待ってから、最終の view を作る（DD R-03）
    violation: 食い違った評価はその途中結果を捨てる（次の view で作り直す）
    source: FR4.8、ADR-003、DD レビュー R-03
  - id: BR2.1
    statement: 途中の評価は、最終結果と同じ評価器を途中の view に走らせ、できた QueryResult を版つきで知らせる
    category: policy
    applies_to: QueryRunner、QuerySink
    trigger: 途中の評価が終わったとき
    logic: QuerySink に on_partial_result(runId, result, coverage) を足す。QueryRunner は評価が最後まで進んだときだけ知らせる（途中で止めたものは知らせない）。版の番号は InsightsSession が受け取ったときに進める（BR2.4）。U2 の stats のあとの命令と打ち切りも、途中の評価で同じく行う
    violation: なし
    source: FR4.1、FR4.3、A5、ADR-003・ADR-004
  - id: BR2.2
    statement: 途中結果も最終結果と同じ並びに保つ。新しい行が途中に入ることがあり、最終結果になったときに並びが急に変わることはない
    category: policy
    applies_to: QueryEvaluator（変えない）、QueryResultTable
    trigger: 途中結果を出すとき
    logic: 途中の view は結果の並びの順（BR1.3）で、評価器は今の決まりで並べるため、filter・fields だけのクエリも sort・limit・stats を含むクエリも、途中結果の並びはいつもそのときの view に対する最終結果と同じ。行の数と集計値は取得が進むにつれて変わる（FR4.3）
    violation: なし
    source: FR4.2（読み替え）、FR4.3、ADR-003、RA レビュー R-01
  - id: BR2.3
    statement: TauriQuerySink は途中結果を受けて InsightsSession の結果を差し替え、「結果の版が進んだ」ことだけを画面に知らせる
    category: policy
    applies_to: TauriSink、TauriApp
    trigger: on_partial_result、on_evaluated、中断後の結果を受けたとき
    logic: InsightsSession の結果を差し替えたあと、session-changed（今のイベント）で SessionView の ResultSummary（runId・版・stage・coverage を足したもの）を送る。結果の行はイベントに載せない（ADR-004）。最終結果と中断後の結果は今の進み具合の押し出しの間隔（250 ms）に関わらずすぐ送る。途中結果も、BR1.1 の 1 秒の間隔で来るため、間隔を絞らず送る
    violation: なし
    source: FR4.4、ADR-004
  - id: BR2.4
    statement: 古い版と、別の実行の結果は捨てる
    category: constraint
    applies_to: InsightsSession、QueryResultTable、ApiBridge
    trigger: 結果を受け取ったとき、画面が行を読んだ答えが届いたとき
    logic: InsightsSession は runId が今の実行と同じで、版が今の版より新しいものだけを受け取り、版を 1 つ進める。画面は get_query_rows・get_query_row_detail に (runId, version) を渡し、中核は今の版と違えば空の答えを返す。画面は答えの (runId, version) が今の見出しと違えば捨てる。実行中に［Run query］は押せない（今のまま）ため、実行中の再実行は起きない。画面を切り替えても途中結果の受け取りは止まらない（今の実行と同じ）
    violation: なし
    source: A4、RA レビュー R-07、FR4.10
  - id: BR2.5
    statement: 実行を始めたら前回の結果を「前回の結果」の印つきで残し、最初の途中結果（または途中結果を経ない最終結果）を受け取った時点で、見出し・行・詳細・件数・書き出しの列をまとめて差し替える
    category: policy
    applies_to: InsightsSession、LogAnalyticsPage、QueryResultTable
    trigger: 実行を始めたとき、最初の版を受け取ったとき
    logic: begin で previous = true のまま前回の result を持ち越す（今のまま）。最初の版を受け取ったら previous = false にして差し替える。形（events / aggregates）が前回と違っても、画面は受け取った見出しの shape に従うだけ（U1 の BR4.5）。読み取りのエラー・取得全体の失敗・評価の失敗のときの前回の結果の扱いは今のまま
    violation: なし
    source: FR4.7、A3、ADR-006
  - id: BR2.6
    statement: 途中結果の印は、表の見出しの横に、途中であること・ロググループの何番目か・その途中結果に含めた件数を文字で出す。最終結果で消す
    category: policy
    applies_to: InsightsSession（coverage）、QueryResultTable、MessageCatalog
    trigger: 見出しを描くとき
    logic: stage = partial なら「途中の結果（取得中：ロググループ {n} / {m}、{count} 件まで）」（日英の文言）。ロググループが 1 つなら「ロググループ」の部分を省く。stage = final なら印なし（今の件数の見出し）。stage = cancelledPartial なら BR3.4 の印。今の RunProgress（結果の容器の外の進み具合）は変えない。色だけで示さず、Cloudscape の標準の部品（StatusIndicator など）と文字で示す（NFR10・NFR11）
    violation: なし
    source: FR4.5、FR4.6、Q2 A、NFR10
  - id: BR2.7
    statement: 途中結果の差し替えでは、読み上げに状態の変化だけを知らせ、版が進むたびには知らせない
    category: policy
    applies_to: QueryResultTable、LogAnalyticsPage
    trigger: 版が進んだとき
    logic: 読み上げの領域（aria-live）には、途中結果が初めて出たとき・最終結果になったとき・中断の結果になったときだけ文を出す。途中の版の差し替えでは出さない
    violation: なし
    source: NFR10、RA レビュー R-06
  - id: BR2.8
    statement: 差し替えのあとも、開いている行と、スクロールの先頭に見えている行を、行の見分けで保つ
    category: policy
    applies_to: QueryResultTable、ScreenHooks（expansionState）
    trigger: 版が進んで行を読み直したとき
    logic: 中核は QueryRow に rowKey（イベントの行は ロググループ名・ストリーム名・ストリーム内の順番、集計の行は鍵のセルの文字をつないだもの）を足す。画面は開いた行を rowKey で覚え（今の位置で覚える作りを改める）、新しい版で同じ rowKey の行を開いたままにし、なければ閉じる。スクロールは、差し替えの前に先頭に見えていた行の rowKey を新しい版で探し（中核の新しいコマンド find_query_row(runId, version, rowKey) が位置を返す）、見つかればその行が先頭になるように位置を合わせ、なければ今の位置のまま。選択（範囲の選択の文字）は保たない
    violation: なし
    source: FR4.4、Q4 A
  - id: BR2.9
    statement: 途中結果が出ている間も、画面の切り替え・スクロール・行の開閉ができ、［Export］は実行が終わるまで押せない
    category: constraint
    applies_to: LogAnalyticsPage、ExportJob（変えない）
    trigger: 実行中
    logic: 今の可否（実行中は Running）のまま。途中結果でも表を描く（今の「result があるときだけ描く」の条件は result が版つきになっただけで変わらない）
    violation: なし
    source: FR4.10
  - id: BR3.1
    statement: 取得中に［Cancel］を押したら、取得を止め、止めた時点の確定済みの分と取得中の分の写しで 1 回評価し、「一部のログだけの結果（中断）」の最終結果にする
    category: policy
    applies_to: QueryRunner、InsightsSession、QueryStore
    trigger: Fetching の間に［Cancel］を押したとき
    logic: 取得に止める合図を出す（今のまま）。動いている途中の評価を止める（BR1.4）。段階を Cancelling にする。取得が止まったら、BR1.3 で途中の view を作り（取得中の範囲の分も含める。確定はしない、FR5.3）、評価する。この評価は最初の中断の合図では止めない（今の is_stopped・on_evaluation_progress・finish_evaluation の「中断なら捨てる」を、中断後の評価では使わない）。評価が終わったら stage = cancelledPartial の最終結果にし、段階を Cancelled にする。取得の一部のストリームが失敗していれば、失敗したストリームも示す（FR5.4）
    violation: なし
    source: FR5.1、FR5.3、FR5.4、ADR-006、DD レビュー R-04
  - id: BR3.2
    statement: 中断後の評価の最中にもう一度［Cancel］を押したら、その評価も止める。そのとき表に出ていた途中結果があれば中断の印つきで残し、なければ「止めた」とだけ示す
    category: policy
    applies_to: QueryRunner、InsightsSession
    trigger: Cancelling の間に［Cancel］を押したとき
    logic: 2 回目の合図で中断後の評価に止める信号を出し、結果は捨てる。IF そのとき result が今の実行の partial THEN その stage を cancelledPartial に変えて残す（版は進めない）。ELSE result をなくし、今の Cancelled の表示（止めた）にする。段階を Cancelled にする
    violation: なし
    source: FR5.1、Q3 A、DD レビュー R-04
  - id: BR3.3
    statement: Evaluating の間（取得が終わって最終の評価をしている間）に［Cancel］を押したら、今のとおり評価を止める。表に途中結果があれば中断の印つきで残す
    category: policy
    applies_to: QueryRunner、InsightsSession
    trigger: Evaluating の間に［Cancel］を押したとき
    logic: 取得はもう終わっているため、最終の評価を止める（今のまま）。IF result が今の実行の partial THEN cancelledPartial にして残す。ELSE 今の Cancelled の表示
    violation: なし
    source: FR5.1
  - id: BR3.4
    statement: 中断の結果の印は「一部のログだけの結果（中断）」。失敗したストリームもあれば、両方を示す
    category: policy
    applies_to: QueryResultTable、LogAnalyticsPage、MessageCatalog
    trigger: stage = cancelledPartial の見出しを描くとき
    logic: 表の見出しの横に印（日英の文言）。失敗したストリームの一覧は今の表示（FailureList）のまま並べて出す。中断の表示（CancelNotice）は「止めた」から「一部のログだけの結果を出しました」に変える（結果がないときは今の「止めた」）
    violation: なし
    source: FR5.1、FR5.4、NFR9
  - id: BR3.5
    statement: 中断の結果も書き出せる。印はファイルに含めず、完了の通知に「一部のログだけの結果です」と添える
    category: policy
    applies_to: ExportJob、InsightsSession、ExportNoticeAlert
    trigger: Cancelled で cancelledPartial の結果があるときに［Export］を押したとき
    logic: 可否は今のとおり（実行中でなく、結果があれば書き出せる）。ファイルの中身は表と同じ（U1 の BR4.4）。完了の通知（Saved / Copied）の文に、結果が cancelledPartial のときだけ「一部のログだけの結果です」を足す
    violation: なし
    source: FR5.2、RA レビュー R-08
  - id: BR3.6
    statement: 止める前に取得し終えた範囲は取得済みとして残し、途中までの範囲は取得済みにしない
    category: constraint
    applies_to: QueryRunner、QueryStore
    trigger: 中断のとき
    logic: 今の決まり（前の FR2.6・FR2.5）のまま。中断後の評価に使った取得中の分の写しは、評価が終わったら捨てる
    violation: なし
    source: FR5.3
  - id: BR4.1
    statement: 結果の見出し（ResultSummary）に版・stage・coverage を足し、行の読み出しに版を足す
    category: policy
    applies_to: InsightsSession、TauriApp、ApiBridge
    trigger: 見出しと行を読み出すとき
    logic: ResultSummary に version・stage（partial / final / cancelledPartial）・coverage（partial のとき）を足す。get_query_rows・get_query_row_detail の引き数に version を足す（BR2.4）。find_query_row(runId, version, rowKey) を足す（BR2.8）。新しいコマンドは capabilities と permissions と、権限の一致のテストの名前の一覧に足す
    violation: なし
    source: ADR-004、FR4.4
  - id: BR4.2
    statement: QueryRow に rowKey を足す
    category: policy
    applies_to: InsightsSession（display_rows）
    trigger: 行を読み出すとき
    logic: イベントの行は ロググループ名・ストリーム名・ストリーム内の順番を区切りの文字（U+001F）でつないだもの。集計の行は鍵のセルの文字（U1 の鍵の比べ方の文字）を同じ区切りでつないだもの。by なしの 1 行は空の文字。ResultRow のイベントの写しにはストリーム内の順番がないため、copy_rows で写すときに ViewEntry の番号も写す
    violation: なし
    source: FR4.4、Q4 A
  - id: BR5.1
    statement: ログ管理の画面の取得・表示・中断と、取得の流れの速さは変えない
    category: constraint
    applies_to: FetchCoordinator（変えない）、LogView（変えない）、QueryRunner
    trigger: 常に
    logic: InflightRegistry への登録は holder の Arc を 1 つ増やすだけで、取得の流れは変えない。途中の評価は取得と別のタスクで行う。既存の速さのテスト（10 万件 10 秒・100 万件 100 秒）は通ったまま。前の作業のクエリの中断のテスト（評価しない）は FR5.1 に合わせて直す
    violation: 既存のテストが落ちたら直す（中断のテストを除き期待値を変えない）
    source: C5、NFR1
```

## ルールの要約

| ID | 分類 | 要点 | 出典 |
|---|---|---|---|
| BR1.1 | policy | 前の評価から最低 1 秒、最初はすぐ | FR4.4、Q1 |
| BR1.2 | constraint | 取得中の保持の登録先 | FR4.8、DD R-02 |
| BR1.3 | calculation | 途中の view の作り方 | FR4.1、FR4.8 |
| BR1.4 | constraint | viewVersion で食い違いを見分ける | DD R-03 |
| BR2.1 | policy | 同じ評価器、版つきで知らせる | FR4.1、FR4.3 |
| BR2.2 | policy | 途中も最終結果と同じ並び | FR4.2（読み替え） |
| BR2.3 | policy | 版の進みだけを画面に | ADR-004 |
| BR2.4 | constraint | 古い版・別の実行を捨てる | A4 |
| BR2.5 | policy | 前回の結果と最初の版での差し替え | FR4.7 |
| BR2.6 | policy | 途中の印（何番目・何件） | FR4.5、Q2 |
| BR2.7 | policy | 読み上げは状態の変化だけ | NFR10 |
| BR2.8 | policy | 行の見分けで開いた行とスクロールを保つ | FR4.4、Q4 |
| BR2.9 | constraint | 実行中の操作と書き出し不可 | FR4.10 |
| BR3.1 | policy | 中断後の 1 回の評価 | FR5.1 |
| BR3.2 | policy | 2 回目の［Cancel］ | FR5.1、Q3 |
| BR3.3 | policy | Evaluating 中の［Cancel］ | FR5.1 |
| BR3.4 | policy | 中断の印と失敗の両方 | FR5.4 |
| BR3.5 | policy | 中断の結果の書き出しと通知 | FR5.2 |
| BR3.6 | constraint | 取得済みの範囲の扱い | FR5.3 |
| BR4.1 | policy | 見出しと読み出しに版 | ADR-004 |
| BR4.2 | policy | 行の見分けの鍵 | FR4.4 |
| BR5.1 | constraint | ログ管理の画面と取得の速さは変えない | C5 |
