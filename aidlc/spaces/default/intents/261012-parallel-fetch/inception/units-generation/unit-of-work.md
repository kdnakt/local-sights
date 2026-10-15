# Unit of Work — ロググループ全体の取得の高速化

上流：`inception/domain-design/components.md`（25 部品）・`decisions.md`（ADR-001〜ADR-009）、`inception/requirements-analysis/requirements.md`、`inception/user-stories/stories.md`、質問票 `units-generation-questions.md`（Q1〜Q3）。アーキテクチャレビューの指摘 R-01〜R-03 をゲートでの Request Changes を受けて反映済み（R-04 の U1 の大きさは Delivery Planning で見る）。

分け方は「利用者に見える機能ごと」。どの作業単位も、今の 1 つのデスクトップアプリ（GUI に依存しないライブラリ＋Tauri アプリ＋画面、1 つの `.dmg`）に足していく。作業単位ごとに別々に配ることはない（team-practices Deployment）。どれから作るかは Delivery Planning で決める。

## 作業単位の一覧

| Unit ID | Directory | 名前 | 種類 | 配置 | 大きさ | ストーリー |
|---------|-----------|------|------|------|--------|------------|
| U1 | u1-concurrent-skeleton | 薄い一本・同時取得 | service | アプリに組み込み | L | US1.1、US1.2、US1.3 |
| U2 | u2-call-pacing | 呼び出しの速さ（引き下げと戻し） | library | ライブラリに組み込み | M | US2.2 |
| U3 | u3-give-up-rule | あきらめの決まり | library | ライブラリに組み込み | S | US2.3 |
| U4 | u4-elapsed-time | かかった時間 | service | アプリに組み込み | S | US2.1 |
| U5 | u5-cancel | 中断 | service | アプリに組み込み | M | US3.1、US4.1、US4.2 |

下準備（CI のカバレッジの検査と cargo-deny のコメントの修正、FR6.1〜FR6.3）は作業単位にしない。U1 の前に小さな PR で入れる（requirements FR6.4、RA Q6）。

## U1 薄い一本・同時取得（u1-concurrent-skeleton）

- **説明**：複数のストリームを同時に読み、届いた分から時刻順に画面に出す、端から端まで通る最小の一本（team-practices Walking Skeleton）。手元で実際の AWS に対して試しても同じアカウントの他の利用者に迷惑をかけないよう、固定の速さの上限を入れる（Q1）。
- **責務**：
  - StreamSchedule（ストリームごとの状態と次に読むもの。あきらめの決まりは U3）
  - FetchCoordinator の同時の読み取りと、ページを 1 列に集めて FetchSink に渡すこと（ADR-003）
  - CallPacer の固定の上限：リージョンの上限の表、半分程度の上限、表にないリージョンとリージョン未指定は 1 秒 5 回程度（ADR-001）。制限での引き下げと戻しは U2
  - 再試行の前も含めて、すべての試行の前に許可を待つこと（FR2.3）。許可待ちの最中に止める信号が来たら「止めた」を返して AWS を呼ばずに終える道（今の `Retrier::call` の形を変える。ドメイン設計のレビュー R-12）。これで U1 の時点でもウィンドウを閉じるときに許可待ちで止まらず、NFR3 の自動テストも再試行を含めて成り立つ（レビュー R-01）
  - あきらめの暫定の動き：今の RetryPolicy の ExhaustionStreak（3 本）を、ストリームが終わった順にそのまま当てはめる。3 本続いたら未着手は呼ばずに失敗とし、読み途中は最後まで読む。StreamSchedule への移動と FR3.1〜FR3.4 のテストをそろえるのは U3（レビュー R-02）
  - AwsCloudWatchLogsGateway で SDK の既定の再試行を止める（ADR-008）
  - FetchStatus の切り出し（JobStatus の移動、ADR-009）。Cancelled の値は U5 で足す
  - FakeGateway（呼び出しの時間と時刻の記録）、RecordingSink、FetchCheck（リージョン必須）の追従
- **含む部品**：StreamSchedule、FetchCoordinator、EventFetcher、CallPacer（固定の上限まで）、FetchStatus、AwsCloudWatchLogsGateway、CachePlan（取り込み先の変更）、AppSession（取り込み先の変更）、TauriSink、FakeGateway、RecordingSink、FetchCheck
- **確かめ方**：`cargo test` がすべて通る。FetchCheck で実際の AWS から同時に取得して表示する。GUI を起動して取得結果が表示されることを目で確かめる（team-practices Walking Skeleton の 3 つ）。NFR1（偽物で 100 本が半分以下の時間）と NFR3（固定の上限を超えない）を自動テストで確かめ、NFR5 は既存の速度測定テストで確かめる。
- **実装の注意**：
  - 今の取得の結果（一部失敗、一覧が途中まで、キャッシュの読み書き）は変えない（FR1.4、FR1.5）。
  - ドメイン設計の 3 回目のレビューで残った指摘のうち、R-12（許可待ちから「止めた」を返す道）、R-13（ADR-009 の見出しの言い回し）、R-14 の一部（一覧の段の状態を FetchJob のどの属性で持つか、終わりの状態を確定する表の置き場所）、R-15（FetchCheck・RecordingSink の FetchStatus への依存）は U1 の機能設計で片付ける。
  - 同時に読む本数、ページの受け渡しの通り道を上限のある形にするかは、U1 の機能設計で決める。

## U2 呼び出しの速さ（u2-call-pacing）

- **説明**：制限（スロットリング）を受けたら速さを下げ、しばらく制限を受けなければ上限まで戻す。
- **責務**：CallPacer の引き下げと戻し（FR2.4）、制限を受けたことを EventFetcher から CallPacer に知らせること、引き下げていることを画面に出さないこと（FR2.6）。再試行の前の許可待ち（FR2.3）は U1 で済んでいる。
- **含む部品**：CallPacer、EventFetcher、FakeGateway
- **確かめ方**：偽物のゲートウェイで制限を返し、時間を進めるテストで呼び出しの間隔が広がり、制限が止むと戻ることを確かめる（NFR3）。
- **実装の注意**：下げ方・戻し方の数値は U2 の機能設計で決める。

## U3 あきらめの決まり（u3-give-up-rule）

- **説明**：同時に読むときの「使い切りが続いたらあきらめる」決まり。終わった順に数え、成功で 0 に戻し、3 本で未着手を呼ばずに失敗にする。読み途中は最後まで読む。
- **責務**：StreamSchedule の使い切りの数え、本数の上限、終わり方の分け方（ExhaustionStreak・max_consecutive_exhausted・StreamEnding を RetryPolicy から移す、ADR-007）。
- **含む部品**：StreamSchedule、RetryPolicy、EventFetcher（StreamEnding を使わなくする）、FetchCheck
- **確かめ方**：StreamSchedule の純粋なテストを先に書き、終わる順を入れ替えた場合を確かめる（FR3.1〜FR3.4）。
- **実装の注意**：U1 の StreamSchedule に足す形で作り、U1 の暫定の動き（今の ExhaustionStreak を終わった順に当てはめる）を置き換える。今の動きとの違いは、使い切り以外の失敗で数え直すこと（FR3.4）を含めて、テストで確かめる。

## U4 かかった時間（u4-elapsed-time）

- **説明**：取得にかかった時間を状態行に出す。取得中は 1 秒ごとに進み、終わったら最終の値を残す。キャッシュから表示したときも出す。
- **責務**：AppSession の始まり・終わり・最終の値（戻らない時計）、画面の時計のフックと表記（mm:ss、h:mm:ss）、状態行の経過、英日の文言、秒ごとに読み上げないこと（FR4、ADR-005）。
- **含む部品**：AppSession、ApiBridge、ScreenHooks、StatusLine、MessageCatalog
- **確かめ方**：表記の関数は Vitest でテストを先に書く。AppSession の最終の値は Rust のテストで確かめる。GUI で経過が進むことを目で確かめる。

## U5 中断（u5-cancel）

- **説明**：取得中は［取得］が同じ場所で［中断］に変わり、押すと取れた分を残して止まる。読み終えていない本数を注意で知らせる。中断した取得はキャッシュに書かない。
- **責務**：理由つきの止める信号（閉じるが常に勝つ、ADR-004）、Cancelled の追加（ADR-009）、中断のコマンドとその権限、［中断］／中断中…のボタン、中断の注意（CancelNotice）、読み終えていない本数 N、最後が終わった後の中断は終わった扱い、キャッシュに書かない（FR5）。
- **含む部品**：RetryPolicy、FetchStatus、FetchCoordinator、StreamSchedule、StreamPlanner、CallPacer（許可待ちの最中の中断の理由の扱い）、CachePlan、AppSession、TauriApp、ApiBridge、AppShell、ConditionArea、StatusLine、CancelNotice、MessageCatalog、RecordingSink（Cancelled と読み終えていない本数 N の記録）、FetchCheck（止める信号の理由への追従）（レビュー R-03）
- **確かめ方**：止め方（新しい呼び出しを始めない、待ちの間ならすぐ止まる、NFR4）と N の数え方を自動テストで確かめる。閉じるときの今の動き（FR5.10）は回帰テストで守る。GUI で中断と注意を目で確かめる。
- **実装の注意**：ドメイン設計のレビュー R-14 の一部（「止めた」で終えたストリームの状態と、N・FR5.9 の判定への影響）は U5 の機能設計で決める。

## Assumptions & Open Questions

- 仮の決め：U2〜U5 は同じファイル（FetchCoordinator、AppSession、StreamSchedule など）に手を入れることがあるが、コードの上で互いを必要とはしないため依存にしない（Q3）。同じファイルを触る順は Delivery Planning で決める。
- 仮の決め：U1 の CallPacer は固定の上限と、すべての試行の前の許可待ちを持ち、U2 がそこに引き下げと戻しを足す。
