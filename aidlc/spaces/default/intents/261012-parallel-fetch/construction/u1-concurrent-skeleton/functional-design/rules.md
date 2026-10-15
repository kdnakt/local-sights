# Business Rules — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`inception/requirements-analysis/requirements.md`（U1 の FR：FR1.1〜FR1.5、FR2.1〜FR2.3、NFR1、NFR3、NFR5、NFR8）、`inception/units-generation/unit-of-work.md`、`inception/contract-design/contract-summary.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1・Q2。アーキテクチャレビュー（1 回目）の指摘を反映済み。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: アプリの呼び出しの上限は、リージョンの GetLogEvents の既定の上限の半分（小数は切り捨て）
    category: calculation
    applies_to: RegionCallLimit, PacingState.ceilingPerSecond
    trigger: 取得を始めるとき
    logic: IF リージョンの既定の上限が 25 THEN 12、IF 10（us-west-2、ap-northeast-3、ap-southeast-3、ca-west-1、eu-central-1、eu-west-1、il-central-1）THEN 5、IF 30（eu-west-3）THEN 15
    violation: なし（決まった値を使う）
    source: FR2.1
  - id: BR1.2
    statement: 表にないリージョンと、リージョンの指定が無いときは 1 秒 5 回
    category: policy
    applies_to: PacingState.ceilingPerSecond
    trigger: 取得を始めるとき
    logic: IF リージョンが表にない OR 取得の条件にリージョンが無い THEN 上限は 5
    violation: なし
    source: FR2.2、ドメイン設計のレビュー R-06
  - id: BR1.3
    statement: 許可は一定の間隔で 1 つずつ出す
    category: constraint
    applies_to: PacingState
    trigger: 試行の前に許可を求められたとき
    logic: 許可を求められたら、ロックの中で枠を予約してから待つ。slot = max(今, nextPermitAt)、nextPermitAt = slot + minInterval をロックの中で決め、ロックを放してから slot の時刻まで待つ（待つ前に予約するので、同時に求めた 8 本が同じ時刻を受け取ることはない）。minInterval はナノ秒単位で切り上げる（1,000,000,000 ÷ 上限 を切り上げ。東京なら 83,333,334 ナノ秒）
    violation: なし（待たせる）。許可の時刻の間は必ず minInterval 以上あくので、どの 1 秒の区間 [t, t+1 秒) をとっても許可の数は上限以下になる
    source: FR2.1、NFR3
  - id: BR1.4
    statement: GetLogEvents のすべての試行（再試行を含む）の前に許可を待つ
    category: constraint
    applies_to: EventFetcher、Retrier の 1 回の試行
    trigger: GetLogEvents を呼ぶ前
    logic: IF これから GetLogEvents を呼ぶ（最初の試行でも再試行でも）THEN 先に CallPacer の許可を待つ
    violation: 許可なしでは呼ばない
    source: FR2.3、約束 C1
  - id: BR1.5
    statement: 許可待ちの最中に止める信号が来たら、AWS を呼ばずに「止めた」で終える
    category: policy
    applies_to: CallPacer、Retrier
    trigger: 許可を待っている間に止める信号が来たとき
    logic: IF 許可待ちの間に止める信号が来た THEN 許可を出さず、その試行は Aborted として終わる（今の再試行の待ちの間の中止と同じ道）
    violation: なし
    source: NFR4、ドメイン設計のレビュー R-12、約束 C1・C7
  - id: BR1.6
    statement: GetLogEvents では AWS SDK の既定の再試行を止め、試行 1 回を実際の呼び出し 1 回にする。DescribeLogGroups・DescribeLogStreams は今のまま
    category: constraint
    applies_to: AwsCloudWatchLogsGateway
    trigger: GetLogEvents を呼ぶとき
    logic: IF GetLogEvents を呼ぶ THEN その呼び出しだけ SDK の再試行の回数を 1（再試行なし）に上書きする。DescribeLogGroups（ロググループ一覧。今は Retrier を通らない）と DescribeLogStreams は SDK の既定の再試行を残し、今の動きを変えない
    violation: なし。GetLogEvents で SDK が吸収していた一時的なサーバー側の失敗（5xx）は、今の分け方では Other になり再試行されない。これはそのまま受け入れ、失敗として一覧する（FR2.5 の再試行の決まりは変えない）
    source: NFR3、ADR-008
  - id: BR1.7
    statement: 同時に読むストリームは最大 8 本
    category: constraint
    applies_to: StreamSlot.state
    trigger: ストリームを読み始めるとき
    logic: IF InFlight の数 < 8 AND 未着手がある THEN 次の未着手を読み始める、ELSE どれかが終わるまで待つ
    violation: 9 本目は始めない
    source: FR1.1、functional-design Q1
  - id: BR1.8
    statement: 読み始める順は選ばれた順（最終イベントの新しい順）
    category: policy
    applies_to: StreamSlot.order
    trigger: 空きができたとき
    logic: IF 空きができた THEN order が最も小さい未着手を読み始める
    violation: なし
    source: FR1.4
  - id: BR1.9
    statement: ストリームの一覧を読み終えてから、ストリームを読み始める（今と同じ）。選ぶ決まりも今と同じ
    category: policy
    applies_to: FetchCoordinator、StreamPlanner
    trigger: 取得を始めるとき
    logic: IF 一覧が終わった（全部、または途中までで Partial）THEN 選ばれたストリームを読み始める、IF 一覧の最初のページで失敗して何も選べない THEN 読まずに Failed で終える
    violation: なし
    source: FR1.4、functional-design Q2
  - id: BR1.10
    statement: 届いたページは 1 つずつ、届いた順に受け口と時刻順の保持先に渡す。あるストリームのページは、すべてそのストリームの終わりの知らせより先に渡す
    category: constraint
    applies_to: PageStep、FetchSink
    trigger: あるストリームの「次の 1 ページ」の読み取りが終わったとき
    logic: IF Page が返った THEN 保持先に混ぜ込み、on_batch を呼んでから、そのストリームの次の 1 ページの読み取りを始める。IF Finished が返った THEN on_stream_finished を呼ぶ。1 本のストリームは 1 ページずつ順にしか進まないので、Page は必ず Finished より先になる
    violation: なし（受け口は同時には呼ばれない）
    source: FR1.3、ADR-003、約束 C4
  - id: BR1.11
    statement: 渡し待ちのページは最大 8 件（読み途中のストリームごとに 1 ページ）
    category: constraint
    applies_to: PageStep
    trigger: ページが返ったとき
    logic: あるストリームの次の 1 ページの読み取りは、前のページを受け口と保持先に渡し終えてから始める。そのため渡し待ちのページは読み途中の本数（最大 8）を超えない
    violation: なし（メモリの使いすぎを防ぐ。別の通り道や上限の数は持たない）
    source: ドメイン設計のレビュー R-07、NFR5
  - id: BR1.12
    statement: 一覧の並びは届いた順ではなく、時刻・ストリーム名・ストリーム内の順番で決まる（今と同じ）
    category: constraint
    applies_to: EventTimeline（LogView）
    trigger: ページを混ぜ込むとき
    logic: IF 古い時刻のページが後から届いた THEN 並びのキーで入る位置に混ぜ込む
    violation: なし
    source: FR1.2、NFR5
  - id: BR1.13
    statement: あきらめは暫定として、今の決まり（使い切り 3 本）をストリームが終わった順に当てはめる
    category: policy
    applies_to: StreamSchedule、ScheduleOutcome.exhaustionStreak
    trigger: ストリームが終わったとき
    logic: IF 使い切りで失敗 THEN 数を 1 増やす、IF 成功 OR 使い切り以外の失敗 THEN 0 に戻す。IF 数が 3 になった THEN 未着手はすべて呼ばずに失敗（Skipped）とし、失敗の内容は最後に使い切ったストリームのものを写す。読み途中は最後まで読む
    violation: なし
    source: 作業単位の切り分けのレビュー R-02、FR3（U3 で決まりを整える）
  - id: BR1.14
    statement: 一部のストリームが失敗しても、ほかのストリームは読み続け、取れたログは残す。失敗したストリームは今と同じく一覧する
    category: policy
    applies_to: FetchJob.failedStreams
    trigger: ストリームが失敗で終わったとき
    logic: IF ストリームが失敗した THEN failedStreams に足し、ほかの読み取りは続ける
    violation: なし
    source: FR1.5
  - id: BR1.15
    statement: 取得 1 回の終わりの状態は、ストリームを読む段の結果と一覧の段の結果を合わせて FetchCoordinator が決める（今の決め方と同じ）
    category: calculation
    applies_to: JobStatus、FetchJob.status
    trigger: すべてのストリームが終わったとき、または閉じるとき
    logic: IF 閉じる THEN Aborted、IF 一覧の最初のページで失敗 THEN Failed、IF 一覧が途中まで OR 失敗したストリームがある THEN CompletedWithFailures、ELSE Completed
    violation: なし
    source: ADR-009、約束 C3、ドメイン設計のレビュー R-09
  - id: BR1.16
    statement: ウィンドウを閉じるとき（止める理由 Close）は、新しいストリームも新しい呼び出しも始めず、取れたログを捨てて Aborted で終える（今と同じ）
    category: policy
    applies_to: FetchCoordinator、StopReason
    trigger: 止める信号 Close が来たとき
    logic: IF Close THEN 未着手は始めない、次の 1 ページの読み取りも始めない。読み途中は次の試行の前（許可待ち・再試行の待ちを含む）で止まる。出ている呼び出しの応答は待つが、その後に返ったページは受け口にも保持先にも渡さない。すべて止まったら保持先を捨てて Aborted。最後のストリームが終わった後でも、on_finished より前に Close が来ていれば Aborted（今の decide_job_status と同じ）
    violation: なし
    source: FR5.10、ADR-004
  - id: BR1.17
    statement: キャッシュに書くのは Completed かつ失敗 0 の取得だけ（今と同じ）
    category: policy
    applies_to: CachePlan、JobStatus
    trigger: 取得が終わったとき
    logic: IF status = Completed AND 失敗 0 THEN 書く、ELSE 書かない
    violation: なし
    source: 約束 C3（JobStatus の置き場所を FetchStatus に移しても判断は変えない）
  - id: BR1.18
    statement: 失敗の詳細は今の許可リストを通した安全な詳細だけを出す
    category: authorization
    applies_to: FetchJob.failedStreams、ApiFailure
    trigger: 失敗を記録・表示するとき
    logic: IF 失敗を記録する THEN 秘密の認証情報とアクセスキー ID を含まない安全な詳細だけを持つ
    violation: なし（今の許可リストの決まり）
    source: NFR8、project.md Forbidden
  - id: BR1.20
    statement: 止める理由は Close が常に Cancel より強い（約束 C7）
    category: policy
    applies_to: StopReason、StopHandle
    trigger: 止める信号を送るとき
    logic: IF まだ止めていない THEN 送った理由にする、IF 今が Cancel で Close が来た THEN Close にする、IF 今が Close THEN 何が来ても Close のまま。同じ理由を何度送っても結果は同じ
    violation: なし。U1 では Close だけを送るが、型と強さの決まりは U1 で作ってテストする
    source: 約束 C7、ADR-004
  - id: BR1.19
    statement: 読み終えた本数は、ストリームが終わった順に 1 ずつ増やす
    category: calculation
    applies_to: FetchJob.finishedStreams
    trigger: ストリームが終わったとき（成功・失敗・Skipped）
    logic: IF Finished を受け取った OR Skipped とした THEN finishedStreams を 1 増やす。failedStreams と outcomes も終わった順に並ぶ（今の「読んだ順」から変わる）
    violation: なし
    source: FR1.1（状態表示の n/m ストリームは今のまま）
```

## まとめ

| ID | 内容 | 分類 |
|----|------|------|
| BR1.1 | 上限はリージョンの上限の半分（東京 12） | calculation |
| BR1.2 | 表にない・指定なしは 5 | policy |
| BR1.3 | 許可は一定の間隔で 1 つずつ | constraint |
| BR1.4 | すべての試行の前に許可を待つ | constraint |
| BR1.5 | 許可待ちで止められたら AWS を呼ばずに終える | policy |
| BR1.6 | GetLogEvents だけ SDK の再試行を止める | constraint |
| BR1.7 | 同時に読むのは最大 8 本 | constraint |
| BR1.8 | 読む順は選ばれた順 | policy |
| BR1.9 | 一覧を読み終えてから読む | policy |
| BR1.10 | ページは 1 つずつ、ストリームの終わりより先に渡す | constraint |
| BR1.11 | 渡し待ちは最大 8 件 | constraint |
| BR1.12 | 一覧の並びは時刻順（今と同じ） | constraint |
| BR1.13 | あきらめは今の決まりを終わった順に（暫定） | policy |
| BR1.14 | 一部失敗でも続ける | policy |
| BR1.15 | 終わりの状態の決め方（今と同じ） | calculation |
| BR1.16 | 閉じるときは捨てて Aborted | policy |
| BR1.17 | キャッシュに書く条件（今と同じ） | policy |
| BR1.18 | 失敗の詳細は安全な詳細だけ | authorization |
| BR1.19 | 読み終えた本数は終わった順に数える | calculation |
| BR1.20 | Close は Cancel より強い | policy |

## Assumptions & Open Questions

None.
