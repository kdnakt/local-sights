# Contract Summary — ロググループ全体の取得の高速化

上流：`inception/units-generation/unit-of-work.md`・`unit-of-work-dependency.md`（U1〜U5。U2〜U5 はどれも U1 に依存）、`inception/domain-design/components.md`・`decisions.md`、`inception/requirements-analysis/requirements.md`、質問票 `contract-design-questions.md`（Q1〜Q3）。

このアプリはローカルのデスクトップアプリで、外から呼ばれる API はない。AWS への呼び出しは読み取り 3 つだけで、その境界（CloudWatchLogsGateway）の形は変えない（components.md）。ここで決めるのは、作業単位の間のライブラリの中の約束と、アプリと画面の間の約束だけである。

下の型の書き方は、項目と意味を決めるためのもので、Rust・TypeScript の正確な書き方（名前の大文字・小文字、所有の仕方など）は各作業単位の機能設計とコード生成で決める。ただし、ここに書いた項目・値・意味は変えない（変えるときは下の「約束の決まり」に従う）。画面との間で運ぶ項目名は、今と同じく camelCase で送る。

## 約束の一覧

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---------------|----------|-----------|-------|
| C1 | U1 | U2、U5 | ライブラリの中の関数呼び出し（CallPacer の許可待ち） | U1 |
| C2 | U1 | U3、U5 | ライブラリの中の関数呼び出し（StreamSchedule） | U1 |
| C3 | U1 | U5、U3（終わりの状態） | ライブラリの中の共有の型（FetchStatus の JobStatus） | U1 |
| C4 | U1 | U4、U5、TauriSink・RecordingSink | ライブラリとアプリの間の trait（FetchSink と FetchJob） | U1 |
| C5 | 既存（アプリ） | U4、U5（画面） | Tauri のイベント `session-changed`・`fetch-progress`（項目を足す） | U4・U5（足す項目ごと） |
| C6 | U5（アプリ） | U5（画面） | Tauri のコマンド `cancel_fetch` | U5 |

## C1 CallPacer の許可待ち（U1 → U2、U5）

EventFetcher は、GetLogEvents のすべての試行（再試行を含む）の前に許可を待つ（FR2.3、ADR-001）。許可待ちの最中に止める信号が来たら「止めた」を返し、その試行は AWS を呼ばずに終わる（NFR4、ドメイン設計のレビュー R-12 を U1 で片付ける）。U2 は「制限を受けた」の報告と、それによる速さの引き下げと戻しを足す（FR2.4）。

```yaml
# shared-schema: ライブラリの中の約束（Rust）
CallPacer:
  created_by: FetchCoordinator   # 取得 1 回につき 1 つ。全ストリームの読み取りで共有する
  new:
    inputs:
      region: "Option<regionCode>"   # 取得の条件のリージョン。None は表にないリージョンと同じ扱い
    rule: "上限 = リージョンの GetLogEvents の上限の表の半分程度。表にない・None は 1 秒 5 回程度（FR2.1、FR2.2）"
  acquire:                         # 各試行の前に呼ぶ（U1）
    inputs:
      stop: "StopSignal の受け口"
    returns:
      - Permitted                  # 試行してよい
      - Stopped                    # 止める信号が来た。AWS を呼ばずに試行を終える
    guarantees:
      - "Stopped は止める信号が来てから待たずに返る（NFR4）"
      - "Permitted の回数は、どの 1 秒をとっても上限を超えない（NFR3、時計を差し替えてテストする）"
  report_throttled:                # U2 で足す。制限を受けた試行の後に呼ぶ
    inputs: {}
    effect: "速さを下げる。しばらく制限を受けなければ上限まで戻す（FR2.4）。画面には何も知らせない（FR2.6）"
  errors: "なし（許可待ちは失敗しない。AWS のエラーは EventFetcher と RetryPolicy が扱う）"
```

## C2 StreamSchedule（U1 → U3、U5）

ストリームごとの状態と、次に読むものを決める純粋な部品（ADR-002）。U3 はあきらめの決まり、U5 は中断のときの本数 N と「最後が終わった後の中断は終わった扱い」を足す。

```yaml
# shared-schema: ライブラリの中の約束（Rust、純粋なロジック。非同期も入出力も持たない）
StreamSchedule:
  new:
    inputs:
      streams: "選ばれたストリーム名の並び（最終イベントの新しい順。FR1.4）"
  next:                            # 空きが出たら呼ぶ（U1）
    returns: "Option<logStreamName>   # 次に読むもの。未着手が無ければ None"
    effect: "返したストリームを『読み途中』にする"
  finish:                          # ストリームが終わったら呼ぶ（U1）
    inputs:
      stream: logStreamName
      outcome: "StreamFetchOutcome（EventFetcher の型をそのまま受け取る。ADR-007）"
    returns:
      skipped: "呼ばずに失敗とした未着手のストリーム（U3 のあきらめ。U1 では今の ExhaustionStreak の暫定の動き）"
  stream_state:
    values: [Unstarted, InFlight, Succeeded, Failed]   # 「止めた」で終えたものの扱いは U5 の機能設計で決める（レビュー R-14）
  unfinished_count:                # U5 で足す
    returns: "読み途中＋未着手の本数。失敗は含めない（FR5.4）"
  outcome:                         # ストリームを読む段の終わりの状態
    returns: "Completed | CompletedWithFailures | Cancelled（Cancelled は U5 で足す）"
    rule: "すべて終わった後に中断が届いたら Cancelled にしない（FR5.9、U5）"
  errors: "なし（不正な呼び出し順はテストで防ぐ。テスト以外で unwrap/expect を使わない）"
```

## C3 FetchStatus の JobStatus（U1 → U5、U3）

```yaml
# shared-schema: ライブラリの中の共有の型（Rust。画面へは C5 の SessionView の中で送る）
JobStatus:
  values:
    - Running
    - Completed               # すべてのストリームが成功
    - CompletedWithFailures   # 一部のストリームが失敗、または一覧が途中まで
    - Failed                  # 一覧の最初のページで失敗して何も選べなかった
    - Aborted                 # ウィンドウを閉じるときの中止。取れたログは捨てる（今と同じ）
    - Cancelled               # 利用者の中断。取れたログを残す（U5 で足す。ADR-009）
  decided_by: "ストリームを読む段は StreamSchedule、取得 1 回の確定は FetchCoordinator（一覧の段の結果と合わせる。ADR-009）"
  cache_rule: "キャッシュに書くのは Completed かつ失敗 0 だけ。Cancelled は書かない（FR5.8）"
```

## C4 FetchSink と FetchJob（U1 → U4、U5、受け口の実装）

今の FetchSink（`&mut self`、届いた順に 1 つずつ呼ばれる）の形は変えない（ADR-003）。同時に読んでも、FetchCoordinator が 1 列に集めてから呼ぶ。

```yaml
# shared-schema: ライブラリとアプリの間の trait（Rust）
FetchSink:
  call_order: "on_started → on_listing_progress* → on_planned → (on_batch | on_stream_finished)* → on_saving? → on_finished"
  guarantees:
    - "どのメソッドも同時には呼ばれない（1 列。U1 で変えない）"
    - "on_batch の順は届いた順。表示の並びは LogView が時刻順に決める（FR1.2）"
    - "on_finished はちょうど 1 回。job.status は Running 以外（C3）"
  methods: [on_started, on_listing_progress, on_planned, on_batch, on_stream_finished, on_saving, on_finished]   # 今と同じ
FetchJob:                          # 今の項目に足すもの
  existing: [job_id, status, range, planned_stream_count, finished_stream_count, failed_streams, listing_status, listing_failure, event_count, outcomes]
  added:
    - name: unfinished_stream_count   # U5。中断のときの N（読み途中＋未着手）。中断でなければ 0
    - name: cancelled_before_listing  # U5。一覧を読み終える前に中断したら true（FR5.6）
```

U4 の始まりと終わりの時刻は FetchJob には足さない。AppSession が on_started と on_finished を受けた時点を戻らない時計で記録する（ADR-005）。

## C5 画面へのイベント（`session-changed`・`fetch-progress`）に足す項目（U4、U5）

今の 2 つのイベントの形に項目を足すだけにする。新しいイベントは作らない（Q1）。画面は知らない項目を無視する。

```yaml
# shared-schema: アプリ → 画面（Tauri のイベント。項目名は camelCase）
SessionView:                       # session-changed で送る。今の項目はそのまま
  phase:
    values: [Idle, Fetching, Cancelling, Done, Failed]   # Cancelling を U5 で足す
    note: "中断した取得は Done（取れたログを表示したまま終わった段階）。Aborted は今と同じく Idle（ADR-009）"
  added:
    - name: fetchCancelAvailable   # U5。［中断］を押せるか。キャッシュに保存中・キャッシュから表示中は false
      type: boolean
    - name: elapsed                # U4
      type: "{ running: boolean, finalMs: number | null } | null"
      meaning: "null は一度も取得していない。running が true の間は画面の時計で 1 秒ごとに進める。終わったら finalMs を出す（ADR-005）"
    - name: cancelNotice           # U5
      type: "{ unfinishedStreams: number | null } | null"
      meaning: "null は出さない。unfinishedStreams が null なら『一覧の前の中断』の文を出す（FR5.4、FR5.6）。次の取得、接続・ロググループの変更で null に戻る（FR5.7）"
  last_job_status_values: "JobSummary.status に Cancelled が入りうる（C3）"
FetchProgressUpdate:               # fetch-progress で送る。今の項目はそのまま
  added: []                        # 経過は秒ごとには送らない（ADR-005）
secrets: "どの項目にも秘密の認証情報・アクセスキー ID を入れない。失敗の詳細は今の許可リストを通す（NFR8）"
```

## C6 中断のコマンド `cancel_fetch`（U5）

```yaml
# shared-schema: 画面 → アプリ（Tauri のコマンド）
cancel_fetch:
  inputs: {}
  returns: "成功（値なし）"
  behaviour:
    - "取得中なら、止める信号に理由『中断』を送り、SessionView.phase を Cancelling にして session-changed を送る"
    - "取得していない、すでに中断中、閉じる途中のときは何もせずに成功として返す（何度送っても結果は同じ。Q3）"
    - "閉じるときの理由『閉じる』は常に『中断』より強い（ADR-004）"
  errors: "返さない（画面はエラーを出さない）"
  permission: "Tauri の capabilities に cancel_fetch を足す。このコマンドは取得を止めるだけで、新しい API 呼び出しも外部への送信もしない"
```

## 約束の決まり

- 持ち主：C1〜C4 は U1（ライブラリの中の約束）、C5 は項目を足す作業単位（経過は U4、段階・中断の注意・中断を押せるかは U5）、C6 は U5。
- 版の番号は付けない（Q2）。ライブラリ・アプリ・画面は同じリポジトリで、同じ `.dmg` で一緒に配る。
- 壊す変更（名前の変更、項目や値の削除、意味の変更、呼ばれ方の保証の変更）は、使う側もすべて同じ PR で直す。
- 足すだけの変更は、使う側が知らない項目を無視する前提で、別の PR に分けてよい。Rust の `match` で網羅を求める enum（JobStatus、Phase など）に値を足すときは、使う側の分岐も同じ PR で直す（コンパイルで漏れが見つかる）。
- 確かめの手段は CI（Rust のテスト、画面の型検査とテスト）。自動テストと CI は実際の AWS に接続しない。

## Open questions

| Contract | Question | Blocks |
|----------|----------|--------|
| C1 | 今の `Retrier::call` の形をどう変えて「止めた」を返す道を作るか | U1 |
| C1 | 制限を受けたときの下げ方と戻し方の数値 | U2 |
| C2 | 「止めた」で終えたストリームの状態（InFlight のままか、別の値か）と、N・FR5.9 の判定への影響 | U5 |
| C4 | 同時に読む本数と、各ストリームから FetchCoordinator への受け渡しの通り道を上限のある形にするか | U1 |
| C5 | `elapsed` の画面の時計の起点（`session-changed` を受け取った時点か、［取得］を押した時点か）と、最終の値に置き換えたときのずれの扱い | U4 |

## Assumptions & Open Questions

- 仮の決め：C5 の型の書き方（`elapsed`、`cancelNotice` の入れ子）は項目と意味を決めるためのもので、TypeScript の型の正確な形は U4・U5 の機能設計で決める。
- 未解決の事項は上の「Open questions」の表のとおり。
