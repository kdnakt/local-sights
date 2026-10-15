# Domain Design 質問票

上流の成果物：`inception/requirements-analysis/requirements.md`（FR1〜FR6、NFR1〜NFR10）、`inception/user-stories/stories.md`、`inception/refined-mockups/interaction-spec.md`、コードの知識ベース `codekb/local-sights/component-inventory.md`・`architecture.md`、`inception/practices-discovery/team-practices.md`。

このステージでは、今回書くコードの「部品（コンポーネント）」の分け方を決める。部品は書くコードだけとし、AWS SDK・Tauri・React・Cloudscape・ファイルシステムは外部依存として扱う（project.md Corrections）。同時に読む本数、速さの数値、下げ方・戻し方、SDK の再試行の扱いは機能設計で決める（requirements §7）。

今ある部品のうち、今回大きく変わる見込みのもの（component-inventory の「今回：高」）：FetchCoordinator、EventFetcher、RetryPolicy、AppSession、TauriApp、TauriSink、ApiBridge、ConditionArea、StatusLine、MessageCatalog、FakeGateway。

## Q1. 呼び出しの速さを抑える仕組み（FR2.1〜FR2.4、NFR3）は、どの部品に置きますか？

背景：同時に読む全ストリームで 1 つの速さの上限（東京なら 1 秒 12 回程度）を共有し、再試行も数に入れ、制限を受けたら下げてしばらくして戻す必要があります。リージョンごとの上限の表も要ります。今の RetryPolicy は「1 回の要求の再試行の待ち」だけを持っています。

A. 新しい部品（CallPacer）にする：リージョンの上限の表、全体で共有する速さの上限、制限を受けたときの下げ方・戻し方を持つ。EventFetcher が GetLogEvents を呼ぶ前（再試行の前も）に毎回これを通る。時計を差し替えられる純粋なロジックとしてテスト先行で作れる
B. ゲートウェイを包む部品にする：CloudWatchLogsGateway と同じ形の包み（RateLimitedGateway）を本物の手前に置き、呼ぶ側は何も知らない。ただし「制限を受けた」の判断と再試行の数え方が RetryPolicy と包みの 2 か所に分かれる
C. RetryPolicy を広げる：再試行の決まりと速さの上限を 1 つの部品にまとめる。部品は増えないが、RetryPolicy の役割が大きくなる
X. Other (please specify)

[Answer]: A. 新しい部品（CallPacer）にする：リージョンの上限の表、全体で共有する速さの上限、制限を受けたときの下げ方・戻し方を持つ。EventFetcher が GetLogEvents を呼ぶ前（再試行の前も）に毎回これを通る。時計を差し替えられる純粋なロジックとしてテスト先行で作れる

## Q2. 同時に読むときの「どのストリームを次に読むか・あきらめの数え方・中断時の本数 N」は、どこに置きますか？

背景：FR3（終わった順に使い切りを数え、3 本で未着手を失敗にする）、FR5.4（N＝読み途中＋未着手）、FR5.9（最後が終わった後の中断は「終わった」）は、どれもストリームごとの状態（未着手・読み途中・成功・失敗）の数え方です。今は FetchCoordinator の直列の `for` の中に埋まっています。

A. 新しい純粋な部品（StreamSchedule）にする：ストリームごとの状態、次に読むもの、あきらめの決まり、中断時の N、終わりの状態の判断を持つ。非同期のつなぎ（同時に走らせる、結果を受け取る）は FetchCoordinator に残す。テスト先行で作れる
B. FetchCoordinator の中に置く：部品は増えないが、非同期の処理と数え方が混ざり、テストは偽のゲートウェイ経由になる
X. Other (please specify)

[Answer]: A. 新しい純粋な部品（StreamSchedule）にする：ストリームごとの状態、次に読むもの、あきらめの決まり、中断時の N、終わりの状態の判断を持つ。非同期のつなぎ（同時に走らせる、結果を受け取る）は FetchCoordinator に残す。テスト先行で作れる

## Q3. 同時に届くページを、画面への受け口（FetchSink）にどう渡しますか？

背景：今の受け口は `&mut self` で 1 本ずつ呼ばれる前提です（TauriSink は呼ばれるたびにセッションのロックを取って画面に送る）。同時に読むと、複数のストリームからページが同時に届きます。

A. 1 か所に集める：各ストリームの読み取りはページを FetchCoordinator に送るだけにし、FetchCoordinator が 1 つずつ受け口に渡す。受け口の形（`&mut self`）と TauriSink はほぼそのまま。届いた順が 1 列に決まり、テストしやすい
B. 受け口を同時に呼べる形（`&self` と内部のロック）に変える：各ストリームが直接受け口を呼ぶ。集める段が要らないが、TauriSink・RecordingSink・確認用プログラムの受け口をすべて直す
X. Other (please specify)

[Answer]: A. 1 か所に集める：各ストリームの読み取りはページを FetchCoordinator に送るだけにし、FetchCoordinator が 1 つずつ受け口に渡す。受け口の形（`&mut self`）と TauriSink はほぼそのまま。届いた順が 1 列に決まり、テストしやすい

## Q4. 利用者の［中断］（ログを残す）と、ウィンドウを閉じるときの中止（ログを捨てる）は、同じ仕組みにしますか？

背景：今はウィンドウを閉じるときだけ中止があり（AbortHandle／AbortSignal）、取れたログを捨てて `Aborted` になります。今回の［中断］は取れたログを残し、キャッシュには書かず、注意を出します（FR5.3・FR5.8）。

A. 同じ信号に「理由」を持たせる：中止の信号を 1 つにし、理由（中断＝残す／閉じる＝捨てる）で終わり方を分ける。止め方（新しい呼び出しを始めない、待ちの間ならすぐ止まる）は 1 か所で済む
B. 別の信号にする：［中断］用の信号を新しく作り、閉じるときの信号はそのまま。今の閉じる動きに手を入れないが、止め方を 2 か所で守る
X. Other (please specify)

[Answer]: A. 同じ信号に「理由」を持たせる：中止の信号を 1 つにし、理由（中断＝残す／閉じる＝捨てる）で終わり方を分ける。止め方（新しい呼び出しを始めない、待ちの間ならすぐ止まる）は 1 か所で済む

## Q5. かかった時間（FR4）は、どの部品が数えますか？

背景：取得中は 1 秒ごとに進め、再試行の待ちの間も進む必要があります（FR4.2）。終わったら最終の時間を残し、キャッシュから表示したときも出します（FR4.3・FR4.4）。時計は戻らない時計で数えます（interaction-spec）。

A. 始まりと終わりはライブラリの画面の状態（AppSession）が戻らない時計で記録し、最終の時間を持つ。取得中の 1 秒ごとの表示は画面側（新しい時計のフック）が進める。秒ごとに画面へ送るものは増えない
B. 画面側だけで数える：［取得］を押した時と終わりを受け取った時の差を画面が数える。ライブラリは何も持たないが、最終の時間がライブラリのテストで確かめられない
C. ライブラリが 1 秒ごとに経過を画面に送る：数えるのは 1 か所だが、取得中ずっと 1 秒ごとの送信が増える
X. Other (please specify)

[Answer]: A. 始まりと終わりはライブラリの画面の状態（AppSession）が戻らない時計で記録し、最終の時間を持つ。取得中の 1 秒ごとの表示は画面側（新しい時計のフック）が進める。秒ごとに画面へ送るものは増えない

## Consolidated Summary Confirmation

- 呼び出しの速さ：新しい部品 CallPacer が、リージョンの上限の表・全体で共有する速さの上限・制限を受けたときの下げ方と戻し方を持つ。EventFetcher は GetLogEvents の各試行（再試行を含む）の前にこれを通る（Q1）
- ストリームの状態：新しい純粋な部品 StreamSchedule が、ストリームごとの状態、次に読むもの、使い切り 3 本のあきらめ、中断時の本数 N、終わりの状態を決める。FetchCoordinator は同時に走らせて結果を受け取る非同期のつなぎに専念する（Q2）
- 受け口：各ストリームはページを FetchCoordinator に送り、FetchCoordinator が 1 つずつ FetchSink に渡す。FetchSink（`&mut self`）と TauriSink の形はほぼそのまま（Q3）
- 中断：今の中止の信号に理由（中断＝残す／閉じる＝捨てる）を持たせ、止め方は 1 か所にまとめる（Q4）
- 経過：AppSession が戻らない時計で始まりと終わりを記録し最終の時間を持つ。取得中の 1 秒ごとの表示は画面の新しい時計のフックが進める（Q5）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
