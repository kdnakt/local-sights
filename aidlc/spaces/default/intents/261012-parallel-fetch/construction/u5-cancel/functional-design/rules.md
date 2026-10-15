# Business Rules — U5 中断（u5-cancel）

上流：`inception/units-generation/unit-of-work.md`（U5）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR4、NFR8〜NFR10）、`inception/user-stories/stories.md`（US3.1、US4.1、US4.2）、`inception/domain-design/components.md`・`decisions.md`（ADR-004、ADR-006、ADR-009）、`inception/contract-design/contract-summary.md`（C2〜C7）、`inception/refined-mockups/`（画面 A〜D、CancelNotice、文言の一覧）、今のコード（`coordinator.rs` の run_fetch・ReadStage・serve_from_cache・abort、`schedule.rs`、`src-tauri/src/lib.rs` の begin_and_spawn_fetch）。質問票は `functional-design-questions.md`（まとめの確認）。

## 上流の約束との違い

契約 C2 と ADR-009 は、読む段の終わり（Cancelled を含む）を StreamSchedule の outcome が決めるとしている。今のコードの FetchCoordinator は ScheduleOutcome の終わりの状態を使わず、decide_job_status で決めているため、U5 も Cancelled を FetchCoordinator が決める（BR5.5）。StreamSchedule は N の材料（止めた本数とまだ始めていない本数）を出すだけで、ScheduleOutcome の終わりの状態の値は増やさない。

## ルール（機械可読）

```yaml
rules:
  - id: BR5.1
    statement: ［中断］の操作を受け入れたら、アプリ（AppState）が止める信号に理由「中断」を送り、AppSession を中断中にする
    category: policy
    applies_to: TauriApp（cancel_fetch、AppState の止める信号の持ち手）、AppSession
    trigger: cancel_fetch を受けたとき
    logic: IF AppSession の fetchCancelAvailable が true THEN AppState が持つ今の取得の持ち手に Cancel を送り、AppSession の cancelling を true にして session-changed を送る。ELSE 何もせず成功を返す（何度送っても結果は同じ）。止める信号の持ち手は、取得を始める操作の中で session のロックを持ったまま AppState に登録する（持ち手がまだ無いのに cancelling だけ立つことを防ぐ）
    violation: なし（エラーを返さない）
    source: FR5.1、C6、AC3.1.2
  - id: BR5.2
    statement: ［中断］を押せるのは、取得中で、中断中でなく、キャッシュに保存中でないときだけ
    category: constraint
    applies_to: AppSession.fetchCancelAvailable
    trigger: SessionView を作るとき
    logic: fetchCancelAvailable = 段階が Fetching AND NOT cancelling AND NOT cache_saving（今の on_saving で立つ項目） AND 閉じる確認の途中でない。キャッシュから表示する道（serve_from_cache）は on_started から on_finished まで 1 回の処理で進むため、画面が押せる見た目になる間はない。キャッシュを引いている間（look_up の待ち）は押せ、その結果は BR5.5 の表に従う
    violation: 押せない（ボタンは押せない見た目）
    source: FR5.2、refined Q2、C5
  - id: BR5.3
    statement: 止める信号が来たら、新しい呼び出しを始めない。出ている呼び出しの応答は待ち、待ちの間ならすぐに止まる
    category: constraint
    applies_to: FetchCoordinator、EventFetcher、StreamPlanner、CallPacer、Retrier
    trigger: 止める信号（理由を問わない）
    logic: IF 再試行の待ち・許可待ちの最中 THEN すぐ「止めた」で戻る。IF 呼び出しの応答待ち THEN 応答を待つ。止めた後は新しいストリームも新しい呼び出し（次のページ・再試行）も始めない。一覧（DescribeLogStreams）も同じ。止めた後に届いた応答の扱いは BR5.4
    violation: なし
    source: NFR4、ADR-004、AC3.1.3、AC3.1.4
  - id: BR5.4
    statement: 止めた後に届いた応答は、理由が「中断」なら次のとおり記録し、「閉じる」なら今のとおり捨てる。読み終えていない本数 N は、途中で止めたストリームとまだ始めていないストリームの合計
    category: calculation
    applies_to: ReadStage（coordinator.rs）、StreamSchedule、FetchJob.unfinished_stream_count
    trigger: 止めた後に読む段へ応答が届いたとき、および読む段が止まったとき
    logic: 理由 Close：今と同じく届いたものを捨て、schedule には渡さない。理由 Cancel：最後でないページ → ページは時刻順の一覧に足して on_batch で渡し、次のページは頼まず、そのストリームを「止めた」として schedule に記録する（今の止めたの終わり方で finish する）。最後のページ（読み終えた） → 今と同じく Finished として finish する。失敗の応答 → 今と同じく失敗として記録する（失敗の一覧と件数に入る）。止めた後は schedule のあきらめ（使い切りの数え）を止め、新しい Skipped を作らない。止めた時点で再試行の待ち・許可待ちだったストリームも「止めた」。N = 「止めた」の本数 ＋ まだ始めていない本数。Finished・失敗・止める前の Skipped は入れない
    violation: なし
    source: FR5.4、C2 の残した問い、AC4.1.2
  - id: BR5.5
    statement: 止める信号で止まった取得の終わりの状態は、FetchCoordinator が止めた理由と N と一覧の進み具合から決める
    category: policy
    applies_to: FetchCoordinator（run_fetch の止めたときの分岐、decide_job_status、serve_from_cache、abort）
    trigger: 取得の終わり
    logic: 今の「止めていたら理由を見ずに abort()（Aborted）」の分岐を理由つきに置き換える。理由 Close → abort()（Aborted、今と同じ）。理由 Cancel で一覧を読み終えていない → Cancelled、cancelled_before_listing = true、unfinished_stream_count = 0、ログは 0 件。理由 Cancel で一覧の後、N > 0 → Cancelled、unfinished_stream_count = N。理由 Cancel で N = 0 → 止めていないときと同じ決め方（Completed／CompletedWithFailures）で on_finished に進み、キャッシュに書くかの判定（BR5.7）も今のとおり行う。キャッシュを引いている間に Cancel：当たったら serve_from_cache を最後まで通して Completed（読み終えていないものがないため。Close のときは今と同じく Aborted）。当たらなければ run_fetch に進み、一覧の前の Cancel として上の表に従う。decide_job_status の入力は、止めた理由（無し・Cancel・Close）、一覧の結果、失敗の本数、N とする
    violation: なし
    source: FR5.6、FR5.9、ADR-009、AC3.1.6、AC4.1.4
  - id: BR5.6
    statement: 閉じるは常に中断より強い
    category: policy
    applies_to: StopHandle、FetchCoordinator、AppSession
    trigger: 止める信号に Close が届いたとき
    logic: 中断の後に閉じるが来たら理由は Close になり、終わりは Aborted（取れたログを捨てて取得前に戻る、注意は出さない）。閉じるときの確認は今と同じ
    violation: なし
    source: FR5.10、ADR-004、AC3.1.7
  - id: BR5.7
    statement: 中断した取得はキャッシュに書かない
    category: constraint
    applies_to: CachePlan
    trigger: 取得の終わり
    logic: キャッシュに書くのは Completed かつ失敗 0 だけ（今のまま）。Cancelled は書かない。N = 0 で終わった扱いになったものは今の決まりに従う
    violation: なし
    source: FR5.8、C3、AC4.2.1
  - id: BR5.8
    statement: どの終わり方でも中断中を戻し、Cancelled なら取れたログを表示したまま終わった段階に移り、注意を持つ
    category: policy
    applies_to: AppSession
    trigger: 取得の終わり（finish_fetch のすべての終わり方と abort_fetch_with_failure）
    logic: どの終わり方でも cancelling = false。Cancelled なら段階 = Done、件数・失敗の一覧・一覧の状態・最後の結果は今の終わりと同じく残し、cancelNotice の unfinishedStreams は cancelled_before_listing なら null、そうでなければ N。0 件でも「この範囲にログはありません」は出さない。Cancelled 以外の終わりでは cancelNotice は null のまま
    violation: なし
    source: FR5.3、FR5.4、FR5.5、FR5.6、ADR-009、refined Q3、AC4.1.1、AC4.1.3
  - id: BR5.9
    statement: 中断の注意は、次の取得を始めたとき、接続を変えたとき、別のロググループを選んだときに消す
    category: policy
    applies_to: AppSession.cancelNotice
    trigger: begin_fetch の受け入れ、接続の変更、select_log_group
    logic: IF 取得を始める OR 接続を変える OR 選んだロググループの名前が今と違う THEN cancelNotice = null。同じ名前を選び直しただけなら残す
    violation: なし
    source: FR5.7、AC4.1.5
  - id: BR5.10
    statement: 取得のボタンは同じ場所で［取得］→［中断］→［中断中…］→［取得］と変わり、フォーカスは動かない
    category: policy
    applies_to: ConditionArea（ボタン）
    trigger: SessionView を受け取ったとき
    logic: 段階が Fetching でない → ［取得］。Fetching かつ cancelling → 読み込み中の［中断中…］（押しても何も起きない）。Fetching かつ fetchCancelAvailable → ［中断］（主ボタン）。Fetching かつ押せない → 押せない［中断］。どの状態も同じ 1 つのボタンで、タブの順とフォーカスは変わらない
    violation: なし
    source: FR5.1、FR5.2、NFR10、AC3.1.1、AC3.1.5、AC3.1.8
  - id: BR5.11
    statement: 中断の注意はログの表の上に warning の注意として出し、アイコンと文で伝え、一度だけ読み上げる
    category: policy
    applies_to: CancelNotice、AppShell
    trigger: cancelNotice が null でないとき
    logic: 今のエラーの表示（FetchErrorBanner）と同じ場所に出す。2 つは同時に出ない（Cancelled は失敗ではない）。閉じるボタンは付けない。role=status の囲みに入れ、出たときに一度だけ読み上げる。フォーカスは動かさない
    violation: なし
    source: FR5.11、ADR-006、AC4.1.6
  - id: BR5.12
    statement: Cancelled で終わったときの状態行は「中断」・件数・（あれば）失敗の件数とリンク・最終の経過
    category: policy
    applies_to: StatusLine
    trigger: 段階が Done で最後の結果が Cancelled のとき
    logic: 「中断」、件数（0 件でも「0 件」とだけ出し、範囲にログがない文は出さない）、失敗があれば今と同じ件数とリンク、キャッシュの通知は出さない、その後ろに U4 の経過
    violation: なし
    source: FR5.5、refined 画面 C、AC4.1.3
  - id: BR5.13
    statement: 新しい文言は英語と日本語を用意する
    category: constraint
    applies_to: MessageCatalog
    trigger: 画面を描くとき
    logic: Cancel／中断、Cancelling…／中断中…、Cancelled／中断、注意 2 種（refined-mockups の文言の一覧のとおり）
    violation: なし
    source: NFR9
  - id: BR5.14
    statement: Cancelled と N を使う側をすべて追従させる
    category: constraint
    applies_to: FetchStats、FetchCheck、RecordingSink、ApiBridge、TauriApp の権限
    trigger: 取得の終わり、画面へ渡すとき
    logic: 要約の 1 行の status に Cancelled が入りうる（決まった語だけ）。確認用プログラムは中断を送らない（止まらない信号）が、終わりの状態の分岐に Cancelled を足す。テスト用の RecordingSink は Cancelled・N・一覧の前の印を記録する。画面の型（src/api.ts）に JobStatus の Cancelled、cancelling、fetchCancelAvailable、cancelNotice を足す。Tauri の権限に cancel_fetch を足す
    violation: なし
    source: C3、C5、C6、unit-of-work U5 の部品
```

## まとめ

| ID | 内容 | 種類 |
|----|------|------|
| BR5.1 | ［中断］の操作を受け入れたら、アプリ（AppState）が止める信号に理由「中断」を送り、AppSession を中断中にする | policy |
| BR5.2 | ［中断］を押せるのは、取得中で、中断中でなく、キャッシュに保存中でないときだけ | constraint |
| BR5.3 | 止める信号が来たら、新しい呼び出しを始めない。出ている呼び出しの応答は待ち、待ちの間ならすぐに止まる | constraint |
| BR5.4 | 止めた後に届いた応答は、理由が「中断」なら次のとおり記録し、「閉じる」なら今のとおり捨てる。読み終えていない本数 N は、途中で止めたストリームとまだ始めていないストリームの合計 | calculation |
| BR5.5 | 止める信号で止まった取得の終わりの状態は、FetchCoordinator が止めた理由と N と一覧の進み具合から決める | policy |
| BR5.6 | 閉じるは常に中断より強い | policy |
| BR5.7 | 中断した取得はキャッシュに書かない | constraint |
| BR5.8 | どの終わり方でも中断中を戻し、Cancelled なら取れたログを表示したまま終わった段階に移り、注意を持つ | policy |
| BR5.9 | 中断の注意は、次の取得を始めたとき、接続を変えたとき、別のロググループを選んだときに消す | policy |
| BR5.10 | 取得のボタンは同じ場所で［取得］→［中断］→［中断中…］→［取得］と変わり、フォーカスは動かない | policy |
| BR5.11 | 中断の注意はログの表の上に warning の注意として出し、アイコンと文で伝え、一度だけ読み上げる | policy |
| BR5.12 | Cancelled で終わったときの状態行は「中断」・件数・（あれば）失敗の件数とリンク・最終の経過 | policy |
| BR5.13 | 新しい文言は英語と日本語を用意する | constraint |
| BR5.14 | Cancelled と N を使う側をすべて追従させる | constraint |

## Assumptions & Open Questions

None.
