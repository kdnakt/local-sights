# Functional Design 質問票 — U4 かかった時間（u4-elapsed-time）

上流：`inception/units-generation/unit-of-work.md`（U4）、`unit-of-work-story-map.md`（US2.1）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR6、NFR9）、`inception/user-stories/stories.md`（AC2.1.1〜AC2.1.7）、`inception/domain-design/components.md`・`decisions.md`（ADR-005）、`inception/contract-design/contract-summary.md`（C4、C5 の `elapsed`）、`inception/refined-mockups/`（mockups.md、interaction-spec.md の ElapsedTime、accessibility-checklist.md）、今のコード（`crates/local-sights-core/src/session.rs` の AppSession、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。

U4 は、取得にかかった時間を状態行の末尾に「経過 mm:ss」として出す。値の持ち主はライブラリの AppSession、取得中の 1 秒ごとの進みは画面の時計、と分ける（ADR-005）。

## 前提として決めたこと（質問にしない技術的な決め）

- **始まり**：［取得］の押下を AppSession が受けて取得中に移る時点（今の `begin_fetch`）を、戻らない時計で記録する。キャッシュから表示する取得も同じ流れを通る（FR4.2、FR4.4）。約束 C4 は「on_started を受けた時点」と書いているが、押した時点（FR4.2）にいちばん近いのは `begin_fetch` で、その差は同じ操作の中の処理の分だけである
- **終わり**：取得の終わり（成功・一部失敗・失敗・中断）を AppSession が受けた時点を記録し、最終の値を決める。キャッシュへの保存は終わりの前に行われるので、保存の時間も経過に入る（refined-mockups 画面 E）。取得の作業が異常に止まったとき（今の `abort_fetch_with_failure`）も、その時点で最終の値を決める
- **画面への渡し方**：`session-changed` で送る SessionView に `elapsed`（running と elapsedMs、一度も取得していなければ空）を足す。秒ごとには送らない（C5、ADR-005）
- **画面の時計**：running の間、画面は SessionView を受け取った時点の自分の戻らない時計を起点に、1 秒ごとに描き直す。次の SessionView を受け取ったら合わせ直す。終わったら最終の値に置き換える（ADR-005 で認めた小さなずれは残る）
- **表記**：秒未満は切り捨てる。1 時間未満は `mm:ss`、1 時間以上は `h:mm:ss`（時の桁は必要なだけ）。文言は「経過 {time}」／「Elapsed {time}」（FR4.5、NFR9）
- **閉じる（Aborted）**：今と同じく取得前の状態（Idle）に戻り、経過も出さない。アプリは閉じるので見えることはない
- **テスト**：表記の関数は Vitest でテストを先に書く。最終の値は Rust のテストで、時計を差し替えて確かめる（NFR6）

## Q1. 画面の読み上げで、取得が終わったときの最終の経過を読み上げますか？

背景：状態行は今、`role="status"`（変化を控えめに読み上げる）です。取得中に経過の秒が進むたびに読み上げると邪魔になるため、取得中の経過は読み上げの対象から外します（FR4.6、AC2.1.7）。決まっていないのは、取得が終わって数字が止まった後の扱いです。interaction-spec は「経過の部分は読み上げの対象から外す」「終わったときの状態表示の変化は今と同じく読み上げる」としています。

A. 取得中だけ外し、終わったら最終の経過も一緒に読み上げる（推奨）：終わったときの「12,345 件 経過 00:31」がそのまま読まれる。画面を見られない人も、速くなったかを知ることができる
B. 取得中も終わった後も、経過はいつも読み上げの対象から外す：読み上げは今とまったく同じ。経過は目で見るだけの情報になる
X. Other (please specify)

[Answer]: A. 取得中だけ外し、終わったら最終の経過も一緒に読み上げる（推奨）：終わったときの「12,345 件 経過 00:31」がそのまま読まれる。画面を見られない人も、速くなったかを知ることができる

## Q2. 前の取得の結果が消える操作（接続やロググループを変える）をしたとき、経過はどうしますか？

背景：FR4.3 は「最終の時間を、次の取得を始めるまで残す」としています。今のアプリでは、接続やロググループを変えると表示中のログ・件数・前の結果が消え、状態行は取得前の表示に戻ります。

A. 前の結果と一緒に経過も消す（推奨）：状態行は取得前の表示に戻り、経過は次に［取得］を押したときに 00:00 から出る。消えた結果の時間だけが残ることがない
B. 経過だけは次の取得を始めるまで残す：FR4.3 の文どおりだが、ログも件数もないのに前の取得の時間だけが出る
X. Other (please specify)

[Answer]: A. 前の結果と一緒に経過も消す（推奨）：状態行は取得前の表示に戻り、経過は次に［取得］を押したときに 00:00 から出る。消えた結果の時間だけが残ることがない

## Consolidated Summary Confirmation

- 経過は状態行の今の内容の末尾に「経過 mm:ss」／「Elapsed mm:ss」として出す。1 時間以上は h:mm:ss、秒未満は切り捨て（FR4.1、FR4.5、NFR9）
- 始まりは［取得］を押して取得中に移った時点、終わりは取得の終わり（成功・一部失敗・失敗・中断、取得の作業が異常に止まったときも）を受けた時点。どちらも AppSession が戻らない時計で記録し、最終の値を持つ。キャッシュへの保存の時間も含み、キャッシュから表示したときも同じに出す（FR4.2〜FR4.4、ADR-005）
- 取得中は画面が 1 秒ごとに進める（再試行の待ちの間も）。画面へは状態が変わるときに送る SessionView の `elapsed` で渡し、秒ごとには送らない（C5）
- 最終の経過は次の取得を始めるまで残す。ただし接続（プロファイル・リージョン）を変えて前の結果が消えるときは、経過も一緒に消し、状態行は取得前の表示に戻る。同じ接続でロググループを選び直すだけなら、今と同じくログも経過も残る（Q2）
- 閉じる（Aborted）は今と同じく取得前の状態に戻り、経過も出さない。一度も取得していないときは出さない
- 読み上げ：取得中の経過の秒の進みは読み上げない。取得が終わったときは、状態表示の変化と一緒に最終の経過も読み上げる（Q1、FR4.6、AC2.1.7）
- 確かめ：表記の関数は Vitest でテストを先に書く。最終の値は時計を差し替えた Rust のテストで確かめる。GUI で経過が進み、終わると残ることを目で確かめる（NFR6）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
