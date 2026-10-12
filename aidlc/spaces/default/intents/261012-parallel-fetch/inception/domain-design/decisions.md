# Architecture Decisions — ロググループ全体の取得の高速化

上流：`domain-design-questions.md`（Q1〜Q5）、`components.md`、`inception/requirements-analysis/requirements.md`、`codekb/local-sights/component-inventory.md`。

## ADR-001: 呼び出しの速さを抑える仕組みを新しい部品 CallPacer にする

- **Context** — 同時に読むと GetLogEvents の呼び出しが増え、アカウントとリージョンで共有の上限（東京 1 秒 25 回、引き上げ不可）を他の利用者と取り合う。要件は「上限の半分程度に抑える」「再試行も数える」「制限を受けたら下げて戻す」（FR2.1〜FR2.4、NFR3）。今の RetryPolicy は 1 回の要求の再試行の待ちだけを持ち、取得全体の速さを扱う場所がない（Q1）。
- **Decision** — 新しい純粋な部品 CallPacer を作る。取得 1 回につき 1 つ作って全ストリームの読み取りで共有し、EventFetcher が GetLogEvents の各試行（再試行を含む）の前に許可を待つ。リージョンごとの上限の表と、下げ方・戻し方を持つ。時計は差し替えられる形にする。
- **Consequences** — 良い点：速さの決まりが 1 か所にまとまり、時間を進めるテストで先に書ける。再試行の前も通るので数え漏れがない。悪い点：EventFetcher に許可待ちの手順が 1 つ増える。SDK の中の再試行は CallPacer を通らないため、AwsCloudWatchLogsGateway 側で止めるか数える必要がある（機能設計）。
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
- **Consequences** — 良い点：止め方のテストと実装が 1 つで済み、EventFetcher・StreamPlanner・CallPacer の待ちが同じ信号を見るだけでよい。悪い点：今の閉じる動きの経路にも手が入るため、閉じるときの今の動き（FR5.10）を回帰テストで守る。中断の後に閉じるが来たときの扱いを決める必要がある（機能設計）。
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
- **Decision** — 数えの持ち主を StreamSchedule にする。RetryPolicy は 1 回の要求の再試行と止める信号だけを持つ。
- **Consequences** — 良い点：1 つのデータの持ち主が 1 つの部品になる。悪い点：今の ExhaustionStreak のテストを StreamSchedule 側に移す必要がある。
- **Security / compliance** — 影響なし。
- **Alternatives Rejected** — RetryPolicy に残し、StreamSchedule から呼ぶ：数えの決まりとストリームの状態が 2 つの部品に分かれ、終わった順の扱いを 2 か所で合わせることになる。
