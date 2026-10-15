# NFR Design 質問票 — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1：中断したら出ている呼び出しを待たずに打ち切る）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちの部分は非機能要件の NFR4.3〜NFR4.5 で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード（`coordinator.rs` の run_fetch・ReadStage・plan_streams・serve_from_cache、`schedule.rs`、`fetch_status.rs`、`session.rs`、`src-tauri/src/lib.rs` の begin_and_spawn_fetch、`src/components/`）。

U5 の非機能の決まりは前の段で利用者と決め終えており、ここで決めるのは実現の形だけである。新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **打ち切り方**：読む段は、積んだ読み（FuturesUnordered）と止める信号を並べて待ち、信号が来たら読みをまとめて捨てる。一覧の段も、一覧の呼び出しと止める信号を並べて待つ。理由が中断でも閉じるでも同じ打ち切り方にする（閉じるときも今より早く終わる。閉じるときの確認と、取れたログを捨てて終わる動きは今のまま）。
- **N の出どころ**：止めた時点で読み途中だったものとまだ始めていないものを、StreamSchedule が数える（今の ScheduleOutcome.unfinished_streams を使う）。止めた後は StreamSchedule にあきらめをさせない。
- **終わりの状態**：decide_job_status に止めた理由・N・一覧を読み終えたかを渡す。閉じるは今の abort()、中断は表のとおり。
- **持ち手の登録とロックの順**：取得を始める操作の中で、session のロックを持ったまま止める信号の持ち手を作って登録する。`cancel_fetch` も session → 持ち手の順でロックを取る。
- **質問票の前半の古い決め**：非機能要件の質問票の前半（応答を待って止まる、最後のページかを次を求めて決める）は、追加の質問 Q1 で置き換わった（非機能要件のレビュー 2 回目の指摘）。この段では Q1 の決めだけを使う。

## Consolidated Summary Confirmation

- 中断でも閉じるでも、出ている呼び出しは待たずに打ち切る。閉じるときの確認と、ログを捨てて終わる動きは今のまま
- 読み終えていない本数は StreamSchedule が数え、止めた後はあきらめを起こさない。終わりの状態は FetchCoordinator が止めた理由・本数・一覧の進み具合から決める
- アプリは取得を始めるときに止める信号の持ち手を session のロックの中で登録し、中断の操作も同じ順でロックを取る。権限に中断の操作を足す
- 画面：ボタンは同じ 1 つで［取得］［中断］［中断中…］を切り替え、中断の注意は今のエラーの表示と同じ場所に warning で出す。状態行に「中断」を足す。英日の文言を足す
- 新しい依存・API・外への送信は足さない。注意と要約に足すのは本数と決まった語だけ

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
