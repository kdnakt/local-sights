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
    logic: 並行の作り方（FR レビュー R-03）：run_query の本流は今のとおり取得の流れを進め、sink（&mut S）は本流だけが持つ。途中の評価は別のタスク（blocking スレッド）で行い、できた QueryResult を本流へのチャンネル（tokio の mpsc）に送る。本流は取得の future とチャンネルの受け取りを select! で待ち、受け取った途中結果を sink.on_partial_result に渡す（sink を別のタスクと共有しない）。取得の進み具合（最初のページが届いたか、取得した件数）は、本流が今の FetchSink::on_batch で数えている値を Arc の原子的な数（AtomicBool・AtomicU64）にも書き、途中のタスクの起動の判断はそれを読む。起動の条件：最初のページが届いた、動いている途中の評価がない、前の途中の評価が終わってから PARTIAL_EVALUATION_INTERVAL（1 秒。作る段階で測って調整してよい）たった、取得した件数が前の途中結果のときより増えた。最初のページのあとの最初の途中の評価はすぐ始める（RA R-06）。評価に 1 秒より長くかかったら、終わり次第すぐ次。取得の終わりや中断で取得の future が終わったら、本流は途中のタスクに止める信号を出し、終わるのを待ってから先へ進む（BR1.4）。足りない範囲がなく AWS を呼ばないとき（今の fetch_missing が取得を 1 つも始めないとき、FR4.9）は途中の評価をしない
    violation: なし
    source: FR4.1、FR4.4、FR4.9、NFR1・NFR2、Q1 A、RA レビュー R-06、FR レビュー R-03
  - id: BR1.2
    statement: 取得中の範囲の保持の登録先は QueryStore が持ち、取り出し・確定・登録外しを QueryStore の鍵 1 回の中で行う
    category: constraint
    applies_to: QueryStore、QueryRunner（fetch_range）
    trigger: 範囲の取得を始めたとき、範囲の取得が終わったとき
    logic: QueryStore に登録先（InflightRegistry）を持たせる。fetch_range は今のとおり範囲ごとの holder を作り、取得を始める前に store.register_inflight(key, range, holder) で登録する。取得が終わったら、今の「holder から take_events → 別の blocking タスクで settle_range」を、1 つの store.settle_inflight(id, completed, started) にまとめる：QueryStore の鍵を取り、その中で holder の鍵を取って take_events し、settle_range と同じ確定を行い、登録から外す。こうして、イベントが holder にも確定済みにもない瞬間と、両方にある瞬間をなくす（FR4.8、FR レビュー R-01）。鍵の順番は「QueryStore → holder」に固定し、取得の流れは holder の鍵だけを取って QueryStore の鍵は取らない（デッドロックしない）。失敗・中断で終わった範囲も今のとおり settle_range(completed = false) と同じに staging に残し、取得済みの範囲の記録にだけ入れない（FR5.3、FR レビュー R-06）
    violation: なし
    source: FR4.8、FR5.3、DD レビュー R-02、FR レビュー R-01・R-06
  - id: BR1.3
    statement: 途中の view は、確定済みの分と取得中の分の写しを、結果の並びの順に合わせて作る
    category: calculation
    applies_to: QueryStore（build_partial_view、ViewEntry）
    trigger: 途中の評価を始めるとき
    logic: QueryStore の鍵を取り、その中で登録された各 holder の鍵を 1 つずつ短く取ってイベントを写す（BR1.2 の鍵の順番。評価は鍵の外で行う、NFR2）。写しには、その範囲の StreamSequences を進めずに読み、続きの番号を仮に振る（確定のときに振る番号と同じ規則で、並びが最終結果と食い違わない）。期間の外のイベントは今の build_view と同じく除く（FR レビュー R-07）。ViewEntry に今の Committed・Staging の 2 つに加えて Inflight（view が持つ写しの位置）の種類を足す（FR レビュー R-02）。確定済みの分の ViewEntry と写しの ViewEntry を合わせ、（時刻, ロググループ, ストリーム, ストリーム内の順番）の順に並べる。キャッシュから読んだ分は add_cached で確定済み（staging）に入るため、確定済みの分として含まれる
    violation: なし
    source: FR4.1、FR4.8、ADR-003、DD レビュー R-02、FR レビュー R-02・R-07
  - id: BR1.4
    statement: view は epoch とは別の viewVersion を持ち、保持が変わるたびに進める。評価と写しは (epoch, viewVersion) が今と同じときだけ進め、食い違えばその途中結果を捨てる
    category: constraint
    applies_to: QueryStore（build_view・build_partial_view・settle_inflight・add_cached・begin_run・evaluate_step・copy_rows）
    trigger: view を作るとき、保持が変わるとき、評価と写しのとき
    logic: ViewEntry は保持の中の位置を指し、EventTimeline の append は時刻順に途中へ差し込むため、確定（settle_inflight）とキャッシュの追加（add_cached）で位置がずれる（FR レビュー R-02）。そこで viewVersion を、view を作るときだけでなく、settle_inflight・add_cached・begin_run・discard_all のたびにも進める。evaluate_step と copy_rows は (epoch, viewVersion) が食い違えば何もせず失敗を返し、途中の評価はその回の結果を捨てる（次の起動で作り直す）。範囲の確定はロググループごとの範囲の終わりにしか起きないため、途中の評価は確定の間の時間で終わる。取得がすべて終わったら、本流は途中のタスクに止める信号を出して終わるのを待ち（BR1.1）、そのあとで最終の view を作る。最終の評価の間は保持が変わらないため、今のとおり食い違わない（DD R-03）
    violation: 食い違った評価はその途中結果を捨てる
    source: FR4.8、ADR-003、DD レビュー R-03、FR レビュー R-02
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
    logic: 中核は QueryRow に rowKey（イベントの行は ロググループ名・ストリーム名・ストリーム内の順番、集計の行は鍵のセルの文字をつないだもの）を足す。画面は開いた行を rowKey で覚え（今の位置で覚える作りを改める）、新しい版で同じ rowKey の行を開いたままにし、なければ閉じる。スクロールは、差し替えの前に先頭に見えていた行の rowKey を新しい版で探し（中核の新しいコマンド find_query_row(runId, version, rowKey) が位置を返す）、見つかればその行が先頭になるように位置を合わせ、なければ今の位置のまま。開いた行の詳細の取得の鍵と、行の取得の効果の依存には version を含める（版が進んだら読み直す。FR レビュー R-09）。選択（範囲の選択の文字）は保たない
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
    statement: 取得中に［Cancel］を押したら、取得を止め、止めた時点までに取得した分（確定済み。取得の途中で止まった範囲は staging に入った分）で 1 回評価し、「一部のログだけの結果（中断）」の最終結果にする
    category: policy
    applies_to: QueryRunner、InsightsSession、QueryStore
    trigger: Fetching の間に［Cancel］を押したとき
    logic: 段階を Cancelling にし、取得に止める合図を出す（今のまま）。取得の future が終わるとき、止まった範囲も BR1.2 のとおり settle_inflight で staging に入るため、取得中の分の写しは要らない（FR レビュー R-06）。本流は途中のタスクを止めて待ち（BR1.1）、今の最終の view（staging を含む）を作って評価する。この評価には中断の合図（StopHandle の cancel）ではなく、2 回目の［Cancel］の合図（abandon、BR3.2）だけを見せる。今の finish_fetch・on_fetch_ended は「中断なら評価しない」を返すが、これを「評価する（中断の結果として）」を返すように変える（戻り値を bool から enum の Evaluate / EvaluateCancelled / Skip に）。今の is_stopped・on_evaluation_progress・finish_evaluation の「中断なら結果を捨てる」は、中断後の評価では使わない。評価が終わったら stage = cancelledPartial の最終結果にし、段階を Cancelled にする。取得の一部のストリームが失敗していれば、失敗したストリームも示す（FR5.4）
    violation: なし
    source: FR5.1、FR5.3、FR5.4、ADR-006、DD レビュー R-04、FR レビュー R-04・R-06
  - id: BR3.2
    statement: Cancelling の間にもう一度［Cancel］を押したら、中断後の評価を止める（まだ始まっていなければ始めない）。表に今の実行の途中結果があればそれを中断の印つきで残し、なければ今の中断と同じ表示にする
    category: policy
    applies_to: QueryRunner、InsightsSession、StopHandle、LogAnalyticsPage
    trigger: Cancelling の間に［Cancel］を押したとき
    logic: StopHandle に 2 つ目の合図 abandon を足す。cancel_available を「is_running かつ (cancel_requested でない、または段階が Cancelling で abandon_requested でない) かつ（今の cache_saving・close_pending の条件）」に改める（FR レビュー R-04）。2 回目の［Cancel］で abandon を立てる。IF 取得がまだ止まっていない THEN 取得が止まったあとに中断後の評価を始めない。IF 評価中 THEN 次の塊の間で止め、結果を捨てる。どちらでも、IF result が今の実行の partial THEN その stage を cancelledPartial に変えて残す（版は進めない）。ELSE 今の Cancelled の表示（止めた。前回の結果が「前回の結果」の印つきで残っていればそのまま）。段階を Cancelled にする。3 回目の［Cancel］は押せない（cancel_available が偽）
    violation: なし
    source: FR5.1、Q3 A、DD レビュー R-04、FR レビュー R-04
  - id: BR3.3
    statement: Evaluating の間（取得が終わって最終の評価をしている間）に［Cancel］を押したら、今のとおり評価を止める。表に今の実行の途中結果があれば中断の印つきで残し、なければ今の中断と同じ表示にする
    category: policy
    applies_to: QueryRunner、InsightsSession
    trigger: Evaluating の間に［Cancel］を押したとき
    logic: 取得はもう終わっているため、最終の評価を止める（今のまま）。IF result が今の実行の partial THEN cancelledPartial にして残す。ELSE 今の Cancelled の表示（BR3.2 の ELSE と同じ）
    violation: なし
    source: FR5.1、FR レビュー R-04
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
    statement: 止める前に取得し終えた範囲は取得済みとして残し、途中までの範囲は取得済みの範囲の記録に入れない（イベントは staging に残り、その実行の結果には使う）
    category: constraint
    applies_to: QueryRunner、QueryStore
    trigger: 中断のとき
    logic: 今の決まり（前の FR2.6・FR2.5、settle_range の completed = false）のまま。次の実行の begin_run で staging は捨てられ、足りない範囲として取り直す
    violation: なし
    source: FR5.3、FR レビュー R-06
  - id: BR3.7
    statement: Cancelling は実行中として扱い、各判定をそろえる
    category: constraint
    applies_to: InsightsSession、TauriApp、ApiBridge、LogAnalyticsPage、StatusLine、MessageCatalog
    trigger: 段階が Cancelling のとき
    logic: is_running は Fetching・Evaluating・Cancelling で真（FR レビュー R-05）。そのため［Run query］は押せない、欄は変えられない、［Export］は押せない（Running）、接続は変えられない（今の実行中と同じ）。［Cancel］は BR3.2 の条件で 1 回だけ押せる。進み具合（RunProgress）は「結果を作っています」と評価の件数を出す。api.ts の phase の型、LogAnalyticsPage の running の判定（3 か所）、StatusLine、messages.ts に Cancelling を足す。begin は Cancelling の間は呼ばれない（Run が押せないため）
    violation: なし
    source: FR5.1、FR4.10、FR レビュー R-05
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
    logic: イベントの行は ロググループ名・ストリーム名・ストリーム内の順番（ResultRow.event の LogEvent が持つ sequence）を区切りの文字（U+001F）でつないだもの。集計の行は鍵のセルの文字（U1 の鍵の比べ方の文字）を同じ区切りでつないだもの。by なしの 1 行は空の文字
    violation: なし
    source: FR4.4、Q4 A、FR レビュー R-07
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
| BR3.6 | constraint | 取得済みの範囲の扱い（staging に残す） | FR5.3 |
| BR3.7 | constraint | Cancelling を実行中として扱う判定の表 | FR5.1 |
| BR4.1 | policy | 見出しと読み出しに版 | ADR-004 |
| BR4.2 | policy | 行の見分けの鍵 | FR4.4 |
| BR5.1 | constraint | ログ管理の画面と取得の速さは変えない | C5 |
