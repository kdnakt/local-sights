# Security Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

この文書は U5 の設計の正本も兼ねる（ほかの設計の文書はここを参照する）。機能設計と食い違うところは、この文書と非機能要件を正とする。

## 1. 守りの設計

| 要件 | 仕組み |
|------|--------|
| NFR8.13 | 中断の注意は本数（数値）と決まった文だけ。SessionView の `cancelNotice` は `{ unfinishedStreams: number \| null } \| null`。要約の 1 行の status は `JobStatus::as_str` の決まった語（Cancelled を足す） |
| NFR8.14 | AWS の API・依存・外への送信は足さない。呼ぶのは読み取り 3 つのまま |
| NFR8.15 | 権限は `cancel_fetch` だけを `src-tauri/build.rs` のコマンドの一覧、`src-tauri/capabilities/default.json` の allow、`generate_handler` に足す。入力を取らない |
| NFR6.13 | 新しいテストは FakeGateway と止めた時計（tokio の start_paused）だけを使う |

## 2. 設計の正本

### 2.1 ライブラリ（crates/local-sights-core）

- **`fetch_status.rs`**：`JobStatus::Cancelled` を足す（serde の名前 "Cancelled"、`ALL` は 6 つ、`as_str`）。網羅の検査で漏れる match（CachePlan、session.rs、fetch_check、src-tauri）を同じ変更で直す
- **`coordinator.rs` の打ち切り（NFR4.3、追加の質問 Q1）**：
  - ReadStage::run：ループの中で `stop.stopped()` と `reads.next()` を `tokio::select!` の `biased;` で、止めを先にして並べて待つ（retry.rs の wait_or_stop と同じ）。止める信号が来たら（理由を問わず）`reads` を落としてループを抜ける。さらに `reads.next()` の枝でも、受け取った直後に `stop.is_stopped()` を見て、止めていたら処理せずに抜ける。止めた時点で届いていても処理していない応答は捨て、そのストリームは読み途中のまま
  - 止める信号が来る前に処理した応答は今と同じに扱う（ページは足す、終わりは finish）
  - N の渡し方：ループを抜けた後、ReadStage は `schedule.outcome().unfinished_streams` を `self.job.unfinished_stream_count` に直接入れる（止めていなければ 0 のまま）。打ち切りでループを抜けると finish は呼ばれないので、止めた後にあきらめ（Skipped）は起きない。StreamSchedule に新しいメソッドは足さない
  - N に入るもの：止めた時点の読み途中（全ページを足し終えて終わりの処理だけが残っていたものも入る）とまだ始めていないもの
  - plan_streams（一覧の段）も、一覧の呼び出しと `stop.stopped()` を並べて待ち、来たら PlanOutcome::Aborted を返す（今の型のまま）
- **`schedule.rs`**：変更しない。`outcome().unfinished_streams`（今の項目：読み途中＋まだ始めていない）を N に使う
- **終わりの状態（BR5.5）**：
  - `decide_job_status(stop: Option<StopReason>, plan: Option<&StreamPlan>, failed_stream_count: usize, unfinished: usize) -> JobStatus`：Close → Aborted。Cancel で plan が無い（一覧の前） → Cancelled。Cancel で unfinished > 0 → Cancelled。それ以外 → 今の決め方
  - `FetchJob` に `unfinished_stream_count: usize`（中断でなければ 0）と `cancelled_before_listing: bool` を足す（serde camelCase）
  - run_fetch：一覧の段が Aborted を返したとき、理由が Close なら今の abort()、Cancel なら job を Cancelled・cancelled_before_listing = true にして on_finished（ログは 0 件、タイムラインは開始時に捨て済み）。読む段の後：理由が Close なら今の abort()、それ以外は decide_job_status で決めて今の終わりの流れ（要約、on_finished、キャッシュの判定）に進む
  - serve_from_cache：理由が Close のときだけ今の abort()。Cancel は無視して最後まで表示し Completed
- **キャッシュ**：decide_write は変えない（`== Completed` かつ失敗 0 だけ書くので、Cancelled を足しても直すところはない）
- **要約の 1 行**：status に Cancelled が入る。打ち切った呼び出しは、試行の数え（counted_call の await の後）に入らないので、calls と throttled に数えない
- **網羅の検査で直す match**：JobStatus を match しているところ（session.rs、src-tauri、examples/fetch_check.rs、テスト）はコンパイルが教える。CachePlan は `==` なので直さない

### 2.2 AppSession（session.rs）

- 項目：`cancelling: bool`、`cancel_notice: Option<CancelNoticeView>`（`CancelNoticeView { unfinished_streams: Option<usize> }`）
- `fetch_cancel_available()` = 段階が Fetching && !cancelling && !cache_saving && close_confirmation が Pending でない
- `request_cancel(&mut self) -> Option<u64>`：押せるなら cancelling = true にして今の fetch_number を返す。押せないなら None
- `begin_fetch`：cancelling = false、cancel_notice = None
- `finish_fetch`：どの終わり方でも cancelling = false。Cancelled なら段階 Done、件数・失敗・一覧の状態・last_job を今の終わりと同じく残し、cancel_notice = Some（cancelled_before_listing なら None の本数、そうでなければ N）。Cancelled のときは cache_notices を空にする（キャッシュに書かず、状態行もキャッシュの通知を出さないため）
- `abort_fetch_with_failure`：cancelling = false
- `apply_connection_change`：cancel_notice = None
- `select_log_group`：選んだ名前が今と違うときだけ cancel_notice = None
- SessionView：`cancelling`、`fetch_cancel_available`、`cancel_notice` を足す（camelCase）

### 2.3 アプリ（src-tauri/src/lib.rs）

- `begin_and_spawn_fetch`：session のロックを持っている間に `stop_pair()` を作り、`AppState.abort` に `(fetch_number, handle)` を登録してからロックを放す（ロックの順は session → abort）
- 新しいコマンド `cancel_fetch`：session のロックを取り、`request_cancel()` が Some(n) なら、abort のロックを取って持ち手の番号が n なら `handle.stop(StopReason::Cancel)`。view を作り、session のロックを持ったまま session-changed を送る（emit は待たない操作なので、ロックの中で送ってよい）。取得の終わりの view は、取得のタスクが session のロックを取った後に作るので、必ず cancel_fetch の view より後に送られ、画面が古い［中断中…］で上書きされることはない。返り値は常に成功。request_cancel が None のときは何も送らない
- 閉じるときの今の abort_fetch（Close）は abort のロックだけを取る今のまま
- build.rs・capabilities・generate_handler に `cancel_fetch`

### 2.4 画面（src/）

- `api.ts`：`JobStatus` に "Cancelled"、SessionView に `cancelling`、`fetchCancelAvailable`、`cancelNotice`。`cancelFetch()`（invoke("cancel_fetch")）
- `ConditionArea.tsx`：取得のボタンは 1 つの Button。段階が Fetching でなければ［取得］（今のまま）。Fetching で cancelling なら `loading` の［中断中…］（Cloudscape の loading は要素を無効にしないので、フォーカスは残る）。fetchCancelAvailable なら主ボタンの［中断］で押すと cancelFetch。それ以外（キャッシュに保存中）は `disabled` の［中断］
  - R-06 の effect（押せない［取得］からフォーカスを絞り込み欄へ移す）の条件を、canSubmit から「ボタンが要素として無効か」に変える：取得中でないときは !canSubmit、取得中は「cancelling でも fetchCancelAvailable でもない」とき（保存中）だけ。［取得］を押して［中断］になったとき、［中断中…］の間は、フォーカスはボタンに残る
  - 日付・時刻の欄の Enter（submit）は今と同じく取得中は何もしない。中断はボタンを押したときだけ
- 新しい `CancelNotice.tsx`：Cloudscape の Alert（type="warning"、閉じるボタンなし）を `role="status"` の囲みに入れる。文は `cancel.notice.afterListing`（{n}）か `cancel.notice.beforeListing`
- `App.tsx`：FetchErrorBanner と同じ場所に、cancelNotice があれば CancelNotice を出す（Cancelled は失敗ではないので同時には出ない）
- `StatusLine.tsx`：段階が Done で lastJob.status が Cancelled なら「中断」と件数（0 件でも範囲にログがない文は出さない）、失敗のリンク、経過。キャッシュの通知は出さない
- `messages.ts`：`fetch.cancel`（Cancel／中断）、`fetch.cancelling`（Cancelling…／中断中…）、`status.cancelled`（Cancelled／中断）、注意 2 種（refined-mockups の文言の一覧）
- `src/test/fixtures.ts`：見本に新しい項目

## 3. 確かめること

| 確かめること | どこで | 要件 |
|--------------|--------|------|
| 中断の後に始まる呼び出しが 0 回。再試行の待ち・許可待ち・応答待ち・応答を返さない偽物で、時計を進めずに終わる | 結合テスト（FakeGateway、start_paused） | NFR4.2、NFR4.3 |
| 応答待ちの 5 本を打ち切ると N に入り、そのページが一覧にない | 結合テスト | NFR4.4 |
| 3 節の例を小さくした N（止めた＋まだ始めていない、失敗は入らない） | StreamSchedule の単体テスト（テスト先行）、1,000 本の例 | BR5.4、NFR4.6 |
| 打ち切りで抜けた後に finish・あきらめが起きない、N が job に入る、全ページを足し終えて終わりの処理だけ残っていたものも N に入る | 結合テスト | BR5.4 |
| decide_job_status の表 | 単体テスト（テスト先行） | BR5.5 |
| 一覧の途中の中断で Cancelled・一覧の前の印・0 件・キャッシュに書かない | 結合テスト | BR5.5、BR5.7 |
| 全部読み終えた後・終わる前の中断で Completed、キャッシュに書く | 結合テスト | NFR4.5、NFR6.18 |
| キャッシュを引いている間の中断（当たり Completed、外れ一覧の前の Cancelled） | 結合テスト | NFR6.19 |
| 中断の後に閉じるで Aborted、閉じるの今の回帰テスト | 結合テストと既存のテスト | NFR6.15 |
| AppSession：request_cancel、どの終わり方でも cancelling が戻る、注意が出て消える | session.rs の単体テスト（押せるかの判定はテスト先行） | BR5.1、BR5.2、BR5.8、BR5.9、NFR6.14、NFR6.16 |
| 持ち手の登録と中断（始める直後の中断が新しい持ち手に届く、持ち手が無いときは何もしない） | src-tauri のテスト（登録と状態の更新をまとめた関数） | NFR6.17 |
| ［取得］を押した後も［中断］・［中断中…］の間はフォーカスがボタンに残る、保存中だけ絞り込み欄へ移る | 画面のテスト | NFR10.1、BR5.10 |
| cancel_fetch の view と取得の終わりの view の順（stop の直後に終わっても最後に届くのは終わりの view） | src-tauri のテスト（終わりの処理を同じ session で続けて呼ぶ） | NFR6.16、NFR6.17 |
| ボタンの 4 つの見た目、注意の文と role と閉じるボタンなし、状態行の「中断」と 0 件、文言の英日 | 画面のテスト | BR5.10〜BR5.13、NFR9.3、NFR9.4、NFR10.1 |
| 中断の後に絞り込みが使える | 画面のテスト | NFR5.5 |
| 権限の 3 か所 | 差分の確認 | NFR8.15 |

## Assumptions & Open Questions

None.
