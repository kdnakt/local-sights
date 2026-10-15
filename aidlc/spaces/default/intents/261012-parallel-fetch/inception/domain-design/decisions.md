# Architecture Decisions — ロググループ全体の取得の高速化

上流：`domain-design-questions.md`（Q1〜Q5）、アーキテクチャレビューの指摘 R-01〜R-07 と 2 回目の R-08〜R-11（いずれもゲートでの Request Changes を受けて反映）、`components.md`、`inception/requirements-analysis/requirements.md`、`codekb/local-sights/component-inventory.md`。

## ADR-001: 呼び出しの速さを抑える仕組みを新しい部品 CallPacer にする

- **Context** — 同時に読むと GetLogEvents の呼び出しが増え、アカウントとリージョンで共有の上限（東京 1 秒 25 回、引き上げ不可）を他の利用者と取り合う。要件は「上限の半分程度に抑える」「再試行も数える」「制限を受けたら下げて戻す」（FR2.1〜FR2.4、NFR3）。今の RetryPolicy は 1 回の要求の再試行の待ちだけを持ち、取得全体の速さを扱う場所がない（Q1）。
- **Decision** — 新しい純粋な部品 CallPacer を作る。取得 1 回につき 1 つ作って全ストリームの読み取りで共有し、EventFetcher が GetLogEvents の各試行（再試行を含む）の前に許可を待つ。リージョンごとの上限の表と、下げ方・戻し方を持つ。時計は差し替えられる形にする。許可待ちは EventFetcher が Retrier に渡す 1 回の試行の先頭に置き、止める信号（RetryPolicy）が来たら許可を出さずに「止めた」を返す（CallPacer → RetryPolicy の依存）。
- **Consequences** — 良い点：速さの決まりが 1 か所にまとまり、時間を進めるテストで先に書ける。再試行の前も通るので数え漏れがない。悪い点：EventFetcher に許可待ちの手順が 1 つ増える。SDK の中の再試行は CallPacer を通らないため、SDK の既定の再試行は止め、再試行はアプリ側に一本化する（ADR-008）。リージョンが取得の条件に無いときは、表にないリージョンと同じ 1 秒 5 回程度になる（手元の確認用プログラムではリージョンを必ず指定する）。
- **Security / compliance** — 呼ぶ API は増えない（読み取り 3 つのまま）。速さを抑えることは同じアカウントの他の利用者への影響を減らす方向に働く。
- **Alternatives Rejected** — (B) ゲートウェイを包む部品：呼ぶ側は何も知らずに済むが、「制限を受けた」の判断と再試行の数え方が RetryPolicy と包みの 2 か所に分かれる。(C) RetryPolicy を広げる：部品は増えないが、「1 回の要求」と「取得全体」の 2 つの関心が 1 つの部品に混ざる。

## ADR-002: ストリームごとの状態の数え方を新しい純粋な部品 StreamSchedule にする

- **Context** — 同時に読むと、「次に読むストリーム」「使い切りが 3 本続いたらあきらめる」（FR3）「中断時の本数 N＝読み途中＋未着手」（FR5.4）「最後が終わった後の中断は終わった扱い」（FR5.9）が、終わる順が決まらない中で正しく数えられる必要がある。今はこれらが FetchCoordinator の直列の `for` に埋まっている（Q2）。
- **Decision** — 新しい純粋な部品 StreamSchedule を作り、ストリームごとの状態（未着手・読み途中・成功・失敗）と上の決まりをすべて持たせる。非同期のつなぎは FetchCoordinator に残す。
- **Consequences** — 良い点：利用者に見える決まりを、入出力なしでテスト先行に作れる（team-practices の Ordering）。終わる順を自由に並べたテストが書ける。悪い点：FetchCoordinator と StreamSchedule の間でストリームの状態を正しく伝え合う必要があり、伝え忘れは結合のテストで確かめる。
- **Security / compliance** — 影響なし。失敗の内容は今の ApiFailure（安全な詳細）をそのまま写す。
- **Alternatives Rejected** — (B) FetchCoordinator の中に置く：部品は増えないが、非同期の処理と数え方が混ざり、テストが偽のゲートウェイ経由の重いものになる。

## ADR-003: 同時に届くページは FetchCoordinator に集め、FetchSink には 1 つずつ渡す

- **Context** — 今の FetchSink は `&mut self` で 1 本ずつ呼ばれる前提で、TauriSink は呼ばれるたびにセッションのロックを取って画面に送る。同時に読むとページが複数のストリームから同時に届く（Q3）。
- **Decision** — 各ストリームの読み取りはページを FetchCoordinator に送るだけにし、FetchCoordinator が届いた順に 1 つずつ FetchSink と LogView に渡す。FetchSink の形は変えない。
- **Consequences** — 良い点：TauriSink・RecordingSink・確認用プログラム（FetchCheck）の受け口がほぼそのまま使える。画面とログの保持先に入る順が 1 列に決まり、テストで再現しやすい。悪い点：集める段が 1 つの通り道になり、そこが詰まると全体が遅れる。ページごとの処理（混ぜ込み、画面への送信）が速いことが前提で、NFR5 の速度測定テストで確かめる。
- **Security / compliance** — 影響なし。
- **Alternatives Rejected** — (B) 受け口を同時に呼べる形（`&self` と内部のロック）に変える：集める段は要らないが、受け口の実装をすべて直し、ロックの取り合いが画面側に移る。

## ADR-004: 利用者の中断とウィンドウを閉じるときの中止を、理由つきの 1 つの信号にする

- **Context** — 今はウィンドウを閉じるときだけ中止があり（AbortHandle／AbortSignal）、取れたログを捨てて `Aborted` になる。今回の中断は取れたログを残し、キャッシュに書かず、注意を出す（FR5.3、FR5.8）。止め方（新しい呼び出しを始めない、待ちの間ならすぐ止まる、NFR4）は同じ（Q4）。
- **Decision** — RetryPolicy の止める信号に理由（中断＝残す／閉じる＝捨てる）を持たせる。止め方は 1 か所のまま、終わり方だけを理由で分ける。
- **Consequences** — 良い点：止め方のテストと実装が 1 つで済み、EventFetcher・StreamPlanner・CallPacer の待ちが同じ信号を見るだけでよい。悪い点：今の閉じる動きの経路にも手が入るため、閉じるときの今の動き（FR5.10）を回帰テストで守る。理由が重なったときは「閉じる」が常に勝つ。中断の後に閉じるが来たら「閉じる」に強め、閉じるの後に中断が来ても変えない（ウィンドウを閉じるときは取れたログを残す意味がないため）。
- **Security / compliance** — 中断のコマンドを画面から呼べるよう Tauri の権限（capabilities）に足す。このコマンドは走っている取得を止めるだけで、新しい API 呼び出しも外部への送信もしない。
- **Alternatives Rejected** — (B) 中断用の別の信号：今の閉じる動きには手を入れずに済むが、止め方を 2 か所で守ることになり、片方だけ直す漏れが起きやすい。

## ADR-005: かかった時間の値はライブラリ（AppSession）が持ち、秒ごとの進みは画面が数える

- **Context** — 経過は取得中 1 秒ごとに進み（再試行の待ちの間も）、終わったら最終の値を残し、キャッシュから表示した場合も出す（FR4.1〜FR4.4）。時計は戻らない時計で数える（interaction-spec）（Q5）。
- **Decision** — AppSession が戻らない時計で始まりと終わりを記録し、最終の値を持つ。取得中の 1 秒ごとの表示は画面の新しい時計のフック（ScreenHooks）が、始まりを受け取った時点から進める。終わりを受け取ったら AppSession の最終の値に置き換える。
- **Consequences** — 良い点：秒ごとの送信が増えない。最終の値をライブラリのテストで確かめられる。悪い点：取得中の値（画面の時計）と最終の値（ライブラリの時計）の間に、送受信の分のわずかなずれが出る。終わったときに数字が少し戻ったり飛んだりしうるが、秒単位の表示では目立たない。
- **Security / compliance** — 影響なし。
- **Alternatives Rejected** — (B) 画面だけで数える：ライブラリは何も持たずに済むが、最終の値をライブラリのテストで確かめられない。(C) ライブラリが 1 秒ごとに送る：数えるのは 1 か所だが、取得中ずっと送信が増え、画面の描き直しが増える。

## ADR-006: 中断の注意は新しい画面部品 CancelNotice にする

- **Context** — 中断の注意は、ログの表の上、今のエラーの表示（FetchErrorBanner）と同じ場所に出す（refined-mockups）。ただし中断は失敗ではなく、文・色（warning）・読み上げ（一度だけ、role="status"）・閉じるボタンなしが、今のエラーの表示と違う。
- **Decision** — 新しい画面部品 CancelNotice を作り、AppShell が FetchErrorBanner と同じ場所に置く。2 つは同時には出ない（中断した取得は「失敗」にならない）。
- **Consequences** — 良い点：エラーの表示の決まり（何が起きたか・次に何をするか・詳細、SSO のボタン）を変えずに済み、それぞれ単独でテストできる。悪い点：同じ場所に出す部品が 2 つになり、出し分けは AppShell で守る。
- **Security / compliance** — 注意の文は本数だけを出し、ストリーム名や認証の情報は出さない。
- **Alternatives Rejected** — FetchErrorBanner に中断の型を足す：部品は増えないが、エラーの表示の決まりと中断の決まりが 1 つの部品に混ざる。

## ADR-007: 連続使い切りの数え（ExhaustionStreak）の持ち主を RetryPolicy から StreamSchedule に移す

- **Context** — 今は RetryPolicy が ExhaustionStreak（既定 3）を持ち、直列の取得の中で使われている。同時に読むと「終わった順に数える」「成功で 0 に戻す」「3 本で未着手を失敗にする」が、ストリームの状態と切り離せない（FR3、ADR-002）。
- **Decision** — 数えの持ち主を StreamSchedule にする。あきらめるまでの本数（今の `RetryPolicy.max_consecutive_exhausted`、既定 3）と、ストリームの終わり方の分け方（今の `StreamEnding` と `streak_ending` の対応）も StreamSchedule に移す。RetryPolicy は 1 回の要求の再試行と止める信号だけを持つ。EventFetcher は自分の終わり方（StreamFetchOutcome の endReason）だけを返し、それを StreamEnding に分けるのは StreamSchedule が行う。依存は StreamSchedule → EventFetcher の片方向で、EventFetcher は StreamEnding を使わない（レビュー R-08：両方が互いの型を使う輪を避ける）。
- **Consequences** — 良い点：1 つのデータの持ち主が 1 つの部品になる。悪い点：今の ExhaustionStreak のテストを StreamSchedule 側に移す必要があり、RetryPolicy の設定の項目が 1 つ減るため、それを使う呼び出し側（確認用プログラムを含む）も直す。
- **Security / compliance** — 影響なし。
- **Alternatives Rejected** — RetryPolicy に残し、StreamSchedule から呼ぶ：数えの決まりとストリームの状態が 2 つの部品に分かれ、終わった順の扱いを 2 か所で合わせることになる。

## ADR-008: AWS SDK の既定の再試行を止め、再試行はアプリ側に一本化する

- **Context** — 今の `gateway/aws.rs` は再試行の設定を持たず、SDK の既定の再試行が効いている。スロットリングを受けると、アプリの 1 回の GetLogEvents の裏で SDK が複数回の呼び出しを出しうる。CallPacer は境界（`CloudWatchLogsGateway`）の手前にあるため、その内側の再試行は数えられない。数えるためにゲートウェイが CallPacer を受け取ると、ADR-001 で退けた「ゲートウェイを包む」形と同じ構造になる（レビュー R-01）。要件は再試行を含めて上限の半分程度を守ること（FR2.3、NFR3）と、今と同じ再試行の待ちと回数（FR2.5）。
- **Decision** — AwsCloudWatchLogsGateway で SDK の既定の再試行を止め（試行 1 回＝実際の呼び出し 1 回）、再試行はアプリの RetryPolicy（Retrier）だけで行う。具体的な設定の書き方は機能設計で決める。
- **Consequences** — 良い点：CallPacer が数える回数と実際の呼び出しの回数が一致し、NFR3 を偽のゲートウェイのテストで確かめられる。使い切りの数え（FR3）もずれない。依存の形は変わらない（ゲートウェイは CallPacer を知らない）。悪い点：SDK が自動で吸収していた一時的な失敗（接続の一時的な切れなど）も、アプリの再試行の決まりで扱う必要がある。今の決まり（スロットリングと通信失敗を再試行）で足りるかは機能設計で確かめる。ロググループの一覧（DescribeLogGroups）など他の呼び出しの再試行も同じくアプリ側だけになる。
- **Security / compliance** — 呼ぶ API は増えない。呼び出しの量が見積もりどおりになり、同じアカウントの他の利用者への影響を抑えやすくなる。
- **Alternatives Rejected** — SDK の再試行を残し、ゲートウェイが CallPacer を受け取って数える：ADR-001 の案 B と同じ構造になり、制限の判断が 2 か所に分かれる。SDK の再試行を残して数えない：上限の半分程度を守れているかを確かめられない。

## ADR-009: 終わりの状態の型を FetchStatus に切り出し、決めるのは StreamSchedule だけにする

- **Context** — 今の `cache/plan.rs` は FetchCoordinator の `JobStatus` を取り込み、FetchCoordinator も CachePlan を使うため、相互に依存している。AppSession も `FetchJob` と `JobStatus` を使う。今の `JobStatus` には「中断（取れたログを残す）」に当たる値がなく、`Aborted` は「捨てる」で、AppSession は `Aborted` を受けると Idle に戻す。また、終わりの状態が FetchJob.status・ScheduleOutcome.endState・画面の段階の 3 か所に重なって書かれていた（レビュー R-02、R-04）。
- **Decision** — `JobStatus` を小さな共有の型の部品 FetchStatus に切り出し、新しい値「Cancelled」（利用者の中断。取れたログを残す）を足す。「Aborted」はウィンドウを閉じるときの中止（捨てる）のまま。ストリームを読む段の終わりの状態（Completed／CompletedWithFailures／Cancelled）は StreamSchedule が決める。取得 1 回の終わりの状態（FetchJob.status）を確定するのは FetchCoordinator で、一覧の段の結果と合わせる：一覧が最初のページで失敗して何も選べなければ Failed、一覧の前に止める信号が来たら Cancelled（一覧の前）か Aborted、一覧が途中までなら StreamSchedule の結果が Completed でも CompletedWithFailures、それ以外は StreamSchedule の結果をそのまま使う。閉じるときは常に Aborted（レビュー R-09：一覧の段は StreamSchedule ができる前に起きるため）。AppSession は FetchJob.status を見て、「Cancelled」なら取れたログを表示したまま終わった段階に移し、「Aborted」は今と同じく Idle に戻す。画面の段階（phase）は表示のための状態で、終わりの状態の持ち主ではない。
- **Consequences** — 良い点：CachePlan↔FetchCoordinator の相互の依存が解ける。終わりの状態を決める場所が段ごとに 1 つずつになり、「Cancelled はキャッシュに書かない」（Completed ではないため）が今の判断のまま成り立つ。悪い点：`JobStatus` を使う箇所（CachePlan、AppSession、TauriApp、確認用プログラム、テスト）の取り込み先が変わる。`JobStatus` の値が増えるため、それを網羅して分ける箇所はすべて直す必要がある（Rust の網羅の検査で漏れは見つかる）。
- **Security / compliance** — 影響なし。
- **Alternatives Rejected** — `JobStatus` を FetchCoordinator に置いたままにする：相互の依存が残る。中断を `Aborted` に理由を足して表す：「捨てる」と「残す」が同じ値になり、CachePlan と AppSession が理由まで見て分ける必要が出る。

