# Code Summary — U3 取得の作り込み（u3-fetch-robustness）

計画：`code-generation-plan.md`（Plan Approval 済み）。単位テストの手順：`unit-test-instructions.md`。書いたファイルの一覧は `source-manifest.json`、ルールと要件の対応は `traceability.json`。

## 作ったもの・変えたもの

| 場所 | 中身 |
|------|------|
| `crates/local-sights-core/src/streams/mod.rs`（新規） | StreamPlanner の純粋なロジック。LogStream・StreamPlan・StreamListingStatus、範囲との重なり（BR1.3）、「開始 − 1 時間」での打ち切り（BR1.2、時刻なしは残す）、ページの終わり（BR1.1）、エラーでの Partial（BR1.5） |
| `crates/local-sights-core/src/streams/planner.rs`（新規） | DescribeLogStreams のページをたどって StreamPlan を作る流れ。ページごとに列挙の途中経過（見た数・対象の数）を返す。失敗は再試行し、それでも失敗したら Partial |
| `crates/local-sights-core/src/retry.rs`（新規） | RetryPolicy（1・2・4・8・16 秒、±20%、最大 5 回、続けての使い切り 3）、要求ごとの回数（RetryCounter）、続けての使い切りの数（ExhaustionStreak）、差し替えられるばらつき（JitterSource：本番 RandomJitter（fastrand）、テスト FixedJitter）、中断（AbortHandle・AbortSignal、`tokio::sync::watch`）、待ちと中断（`wait_or_abort`）、1 要求の再試行の実行（Retrier） |
| `crates/local-sights-core/src/timeline.rs` | ソート済みの `Vec` へのマージでの挿入、ストリームごとの sequence → timestamp の索引（R-10）、`rows(offset, limit)` → RowWindow、`position_of(stream, sequence)`（索引で時刻を引いてから二分探索）、追加・破棄ごとの timelineVersion |
| `crates/local-sights-core/src/event.rs` | 並べ替えのキーを (timestamp, logStreamName, sequence) に変更（U1:BR5.1 の置き換え） |
| `crates/local-sights-core/src/fetcher.rs` | ストリーム名を引数で受け、再試行し、retryCount を数え、sequence をストリームごとに 0 から振る。終わり方（Completed・Failed{exhausted}・Aborted）を返す。呼ばずに失敗とするストリームの StreamFetchOutcome（R-09） |
| `crates/local-sights-core/src/coordinator.rs` | ロググループ全体の取得の流れ（破棄 → 列挙 → 1 ストリームずつ取得 → ページごとに追加 → 結果の状態）。FetchJob（jobId は増える整数、listingStatus の写し、failedStreams、outcomes）、FailedStream、`decide_job_status`（BR5.2）、受け口 FetchSink（開始 → 列挙の途中経過 → 計画 → 追加 → ストリームの終了 → 終了）、保持ログの受け口 TimelineStore（`Mutex<EventTimeline>` をページごとに短く握る）、中断 |
| `crates/local-sights-core/src/gateway/` | trait に `describe_log_streams` を追加（並び順と降順は AWS の実装の中で固定）。AWS SDK の実装とエラーの分類・安全な詳細 |
| `crates/local-sights-core/src/request.rs` | 取得の要求と入力からストリーム名を削除（BR6.1） |
| `crates/local-sights-core/src/session.rs` | 取得の開始時の消去（BR5.6）、進み具合 FetchProgress、失敗したストリーム・列挙の失敗・失敗の一覧の開閉（BR6.3）、timelineVersion、接続先の世代番号と StaleGeneration（BR6.7） |
| `crates/local-sights-core/tests/` | 偽物の gateway に DescribeLogStreams とストリームごとの GetLogEvents の台本・呼び出しの記録を追加。共通の記録用の受け口 `support/recording_sink.rs`。結合テスト `u3_fetch_flow.rs`。`u1_fetch_flow.rs` を新しい流れに合わせて修正 |
| `crates/local-sights-core/examples/fetch_check.rs` | `--stream` を削除し、ロググループ全体を取得。時刻順に「時刻＋タブ＋ストリーム名＋タブ＋メッセージ」、標準エラーに件数・ストリーム数・失敗の数と失敗の詳細 |
| `src-tauri/` | 保持ログを `std::sync::Mutex` に変更、`log-batch` イベントを廃止、コマンド `get_rows`・`find_row_position`・`set_failure_list_open` を追加、`start_fetch`・`reload_log_groups`・`select_log_group` に世代番号、ウィンドウを閉じるときの中断、権限 |
| `src/` | `virtualScroll.ts`（純粋な計算）、`hooks/useRowWindow.ts`、`LogTable`（3 列・表示範囲だけ描く・位置を保つ・キーボード）、`StatusLine`（列挙中・取得中・失敗の数・列挙が途中まで）、`FailureList`（新規）、`FetchForm` からストリーム名の欄を削除、`App` が世代番号を付ける、英日の文言 |
| `README.md` | 3 つの API と IAM 権限、`--stream` の削除、U3 の手元の確認項目 |

## 主な判断

- 保持ログは 1 本のソート済みの `Vec` に、受けたページを挿入位置から後ろだけマージして入れる。ストリームごとの sequence → timestamp の索引から時刻を引き、キー全体で二分探索して位置を求める（R-10）。リリースビルドで 100 万件のとき、行の取り出し 約 20 µs、位置の問い合わせ 約 11 µs（上限 10 ms）。
- 保持ログは取得の流れが 1 ページごとに短くロックするだけにし、画面は取得中でも `get_rows` で行を読める（NFR3）。ロックの順は常に「セッション → 保持ログ」。接続先の変更での破棄は、セッションのロックを握ったまま行い、新しい timelineVersion をセッションに書いてから画面に知らせる（遅れた破棄が次の取得のログを消さない）。
- 取得の開始時の破棄で増えた timelineVersion も `FetchSink::on_started` で受け取り、セッションに書く。画面はその時点で一覧を一番上に戻す。
- 中身のない GetLogEvents のページは保持ログに足さず、受け口にも追加として届けない（timelineVersion も増やさない）。pageCount には数える（U1:BR3.2）。
- 再試行を使い切ったかどうか（exhausted）は Retrier が返し、使い切らない失敗（AccessDenied など）や成功で続けての数を 0 に戻す。呼ばずに失敗とするストリームは pageCount 0・retryCount 0・最後のストリームと同じ ApiFailure（R-09）。
- 列挙の失敗は再試行の後に Partial。最初のページ（受けたページ 0）で対象 0 件のときだけ Failed。FetchJob の `failure()` は取得全体の失敗（列挙の失敗）を指す。
- jobId はアプリの実行の中で増える整数（静的な原子カウンタ）。
- 中断（Aborted）はウィンドウを閉じるときだけ起きるため、SessionState では Idle に戻し、件数を 0 にする。
- 世代番号の違う `select_log_group`・`reload_log_groups`・`start_fetch` は、コアが StaleGeneration で拒み、Tauri のコマンドは何も返さずに成功として終える（画面には何も出さない）。
- 仮想スクロールは自前。行の高さ 22 px 固定、スクロールの高さの上限 1,000 万 px。上限を超える件数では、スクロールの位置を全体の位置に比例で写す。位置を保つ処理は、変更前のスクロールの位置と、そのとき使っていた件数から一番上の行を求め、最後に受けた行のまとまりからその (logStreamName, sequence) を得て `find_row_position` で新しい位置を聞く。
- 一覧は `<table>` ではなく ARIA の role（table・row・columnheader・cell）を付けた要素で作る（行を絶対位置で描くため）。
- 列挙が途中までで失敗したストリームがないときも、一覧（列挙の失敗を先頭に出す）を開けるよう、ステータス行に「失敗の詳細を見る」ボタンを出す。
- `get_rows` が 1 回に返す行は最大 1,000 行に制限する。
- 秘密情報：失敗の一覧・ステータス行・標準エラー・診断ログに出すのは U1:BR4.3 の安全な詳細だけ。DescribeLogStreams の安全な詳細は API 名・リクエスト ID・プロファイル名・ロググループ名。

## テストの結果

| コマンド | 結果 |
|----------|------|
| `cargo test --workspace` | 成功（lib 162 件成功・1 件 ignored、u1_fetch_flow 11、u2_log_group_listing 9、u3_fetch_flow 14）。レビュー 1 の修正後は下の表 |
| `cargo test -p local-sights-core --release --lib -- timeline:: --ignored` | 成功（100 万件：rows 約 20.5 µs、position 約 10.9 µs） |
| `npx vitest run` | 11 ファイル 75 件成功 |
| `cargo fmt --check`・`cargo clippy --workspace --all-targets` | 成功・警告 0 |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `npm audit` | 脆弱性 0 件 |
| `cargo build -p local-sights` | 成功 |

U3 のライブラリ側の件数：retry 11、streams 7、timeline 8（うち 1 件は `#[ignore]` の 100 万件の速さ）、coordinator 5、event 3（書き直し）、session +11（合計 36）、gateway::aws +3、request（2 件を書き直し）、結合テスト u3_fetch_flow 14、u1_fetch_flow 11（新しい流れに合わせて書き直し）。画面側：virtualScroll 7、LogTable 7、StatusLine 8、FailureList 4、FetchForm 10、App 7、messages 6。U2 の時点の 124（lib）・59（Vitest）から、lib 162（+ignored 1）・Vitest 75。

## テスト先行の証拠（Red）

Testing Contract の `ordering` のとおり、純粋なロジックはテストを先に書き、実行して失敗を確かめてから実装した。

1. `cargo test -p local-sights-core --lib -- streams:: retry:: timeline:: coordinator:: session:: event::`（実装は `todo!()` の骨組み）→ `39 passed; 23 failed; 1 ignored`。失敗の内訳：streams 7、timeline 7、coordinator 5、session 3、event 1。主な出力：`not yet implemented: U3 Step 4`（streams・timeline・coordinator）、`session.rs:1307 left: 0 right: 1`（世代番号が増えない）、`session.rs:1348 left: Ok(()) right: Err(StaleGeneration)`（古い世代を拒まない）、`event.rs:65 left: Less right: Greater`（同じ時刻でストリーム名の順にならない）。成功した 39 件は既存のテスト・retry（下の 2 を参照）・「同じ世代の操作は受け取った順に反映される」（骨組みが世代番号を無視するため最初から通る）。
2. `cargo test -p local-sights-core --lib -- retry::`（retry.rs の関数の中身を一時的に `todo!("red")` に差し替え）→ `0 passed; 11 failed`。その後、元の実装に戻した。
3. `npx vitest run src/virtualScroll.test.ts`（`virtualScroll.ts` がない状態）→ `Failed to resolve import "./virtualScroll"`。

AWS 接続の層（DescribeLogStreams の SDK の実装、偽物を使った列挙と取得の流れ）と GUI の層（AppSession のつなぎ、Tauri、画面）は、実装してからテストを書いて実行した。

## 計画との違い

1. retry.rs は、テストと実装を同じ編集で書いてしまった。テスト先行の確認は、実装の中身を一時的に `todo!()` に差し替えて 11 件すべての失敗を確かめ（上の Red の 2）、元に戻す形で行った。ほかの純粋なロジックは骨組みとテストを先に書いて失敗を確かめてから実装した。
2. `tokio` をライブラリ側の通常の依存にした（機能 `time`。待ちと中断に使う）。U2 までは dev-dependencies だけだったが、AWS SDK がすでに tokio に依存しているため、ビルドに入るクレートは増えない。`Cargo.lock` の差分は `local-sights-core` の依存に `fastrand` が 1 行増えただけ。
3. `request.rs` からのストリーム名の削除（Step 7）は、`fetcher.rs` をストリーム名を引数で受ける形にするために Step 6 の途中で行った（順序だけの違い）。
4. `FetchSink::on_started` に、開始時の破棄の後の timelineVersion を渡す引数を足した（画面が取得の開始で一覧を一番上に戻せるようにするため）。
5. U2 の `timelineGeneration` は、保持ログの `timelineVersion` に置き換えた（接続先の変更での破棄も timelineVersion を増やす、BR4.5）。
6. 接続まわりの Tauri コマンドは、非同期のロックをやめたため同期のコマンドに戻した。取得の中断のハンドルは取得ごとの番号付きで持ち、終わった取得が次の取得のハンドルを消さないようにした。
7. 確認用プログラムは、時刻順に出すため、取得がすべて終わってから標準出力に書く。取得中は標準エラーに「listing streams...」「fetching N streams...」を出す。終了コードは Completed 以外（失敗したストリームがある、列挙が途中まで、取得の失敗）で 1。
8. 中身のない GetLogEvents のページは受け口に届けない（U1 では届けていた）。

## まだ確かめていないこと・未解決

- 実際の AWS での動作と GUI の目視（複数ストリームの時刻順、100 万件に近い件数でのスクロール、取得中に一番上の行が動かない、失敗の一覧、列挙中の表示）は、開発者本人の手元で行う（`README.md` の「Whole-group fetch (U3)」）。
- BR1.2 の仮定（時刻を持たないストリームが列挙のどこに並ぶか）は、手元で確認用プログラムを使って確かめる（rules.md の「前提と手元の確認の項目」）。
- NFR2 のうち「絞り込みの結果が 100 秒以内」と NFR3 の「行の展開」は、この単位の範囲外（後の単位）。U3 で確かめたのは、ライブラリ側の 100 万件での行の取り出しと位置の問い合わせの速さ（自動テスト）で、画面のスクロールの体感は手元の目視で確かめる。
- 取得中のページごとの送信は、レビュー 1 の R-05 で軽い `fetch-progress` イベントに分けた（下の「レビュー 1 の指摘への対応」）。
- Tauri の同期コマンドはメインスレッドで動く。`get_rows` は、取得の流れが大きなページをマージしている間だけ待つことがある。レビュー 1 の R-04 で測ったところ、100 万件を保持した状態で時刻の重なる約 1 万件のページを 1 つ足すのに約 163 ms かかった（リリースビルド）。NFR2 の「スクロールに 1 秒以内に反応」には収まるが、取得中の引っかかりは手元の目視で確かめる。

## レビュー 1 の指摘への対応

レビュー 1（READY、Minor 6 件）の指摘は、project.md の決まりでは次の作業単位に回してよいものだったが、人間の判断で 6 件ともこの単位で直した。

| ID | 対応 | 主な変更 |
|----|------|----------|
| R-01 | 位置を保つ基準の行（logStreamName・sequence・pixelOffset）を `anchorRef` に持ち、`find_row_position` の返答を適用するまで取り直さない。返答の前に次の timelineVersion が来たら、同じ基準で問い合わせ直し、古い返答は捨てる。利用者がスクロール・キー操作をしたら保留中の基準を捨てる（こちらで設定したスクロール位置の反響は無視する） | `src/components/LogTable.tsx`、テスト `LogTable.test.tsx`「keeps the same anchor row when a second version arrives before the reply」（修正前の LogTable では失敗し、修正後に通ることを確かめた） |
| R-02 | `FetchSink::on_finished` に終了後の timelineVersion を渡す（中断のときは破棄の後の版）。`AppSession::finish_fetch(job, timeline_version)` が受け付けた終了で版を書く（Aborted の分岐でも書く） | `coordinator.rs`、`session.rs`、`src-tauri/src/lib.rs`、`examples/fetch_check.rs`、`tests/support/recording_sink.rs`。テスト `session::tests::an_aborted_finish_takes_the_version_of_the_discard`、`u3_fetch_flow` の中断のテストに版の確認を追加 |
| R-03 | AppSession に取得の番号（`begin_fetch` のたびに増える、`fetch_number()`）を持たせ、`abort_fetch_with_failure(fetch_number, failure)` は番号が今の取得と一致するときだけ失敗に移す。Tauri の監視タスクは自分の番号を渡す。中断のハンドルの番号もセッションの番号を使い、アプリ側の独自のカウンタはなくした | `session.rs`、`src-tauri/src/lib.rs`。テスト `session::tests::recovery_only_fails_the_fetch_it_belongs_to` |
| R-04 | `index()` で既存のストリームは `get_mut` で引き、ストリーム名の複製を新しいストリームのときだけにした。リリースビルドの `#[ignore]` テストで、100 万件を保持した状態に時刻の重なる 1 万件のページを足す時間を測った：約 163 ms（同じ実行で rows 約 23 µs、position 約 11 µs）。設計に上限がないため、アサーションはおおまかな上限（1 秒）だけにした | `timeline.rs`、テスト `timeline::tests::one_overlapping_page_is_merged_into_one_million_events` |
| R-05 | 取得中の列挙のページごと・追加のページごとには、SessionView 全体ではなく軽い `fetch-progress` イベント（jobId・進み具合・eventCount・timelineVersion）を送る。開始・計画・ストリームの終了・終了では、これまでどおり `session-changed` で全体を送る。画面は実行中の取得の jobId と一致するときだけ反映する（終了の後に届いた古い進み具合は無視する） | `session.rs`（`FetchProgressUpdate`・`progress_update()`）、`src-tauri/src/lib.rs`、`src/api.ts`（`onFetchProgress`・`withProgress`）、`src/App.tsx`。テスト `session::tests::the_progress_update_carries_only_the_running_job_progress`、`App.test.tsx`「applies the light progress messages of the running job only」 |
| R-06 | 対応先を実装のファイルに変えた：BR5.5・FR4.10・BR4.5 → `crates/local-sights-core/src/coordinator.rs`、BR6.5・BR6.8 → `src/components/LogTable.tsx`（1 つの ID に 1 つの既存ファイル） | `traceability.json` |

修正後の結果：

| コマンド | 結果 |
|----------|------|
| `cargo test --workspace` | 成功（lib 165 件成功・2 件 ignored、u1_fetch_flow 11、u2_log_group_listing 9、u3_fetch_flow 14） |
| `cargo test -p local-sights-core --release --lib -- timeline:: --ignored` | 2 件成功（rows 約 23 µs、position 約 11 µs、重なる 1 万件の append 約 163 ms） |
| `npx vitest run` | 11 ファイル 77 件成功 |
| `cargo fmt --check`・`cargo clippy --workspace --all-targets` | 成功・警告 0 |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `cargo build -p local-sights` | 成功 |

ファイルの集合は変わらないため、`source-manifest.json` は変えていない。計画と `unit-test-instructions.md` も変えていない。
