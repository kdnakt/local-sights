# Contract Design 質問票

上流：`inception/units-generation/unit-of-work.md`・`unit-of-work-dependency.md`（U1〜U5、U2〜U5 はどれも U1 に依存）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。

このアプリには外から呼ばれる API はない（ローカルのデスクトップアプリ。AWS への呼び出しは読み取り 3 つだけで、その形は既存の CloudWatchLogsGateway のまま変えない）。そのため、ここで決める約束は次の 2 種類だけ：

- **作業単位の間の約束（ライブラリの中）**：U1 が作り、U2〜U5 が使うもの。CallPacer の許可待ち、StreamSchedule の状態、FetchStatus（JobStatus）、FetchSink と FetchJob（unit-of-work-dependency の「つなぎ目」）。
- **アプリと画面の間の約束**：Tauri のコマンドとイベント（`session-changed`、`fetch-progress`）で運ぶ SessionView などの形。U4（経過）と U5（中断のコマンド、注意）が足す。Rust と TypeScript の両方で同じ形を守る必要がある。

## Q1. 画面に新しく渡すもの（経過、中断中の段階、中断の注意）は、どう運びますか？

背景：今の画面は `session-changed`（SessionView 全体）と `fetch-progress`（取得中の軽い進み具合）の 2 つのイベントで状態を受け取ります。経過は始まりと終わりの値だけ送り、秒の進みは画面が数えます（ADR-005）。

A. 今の 2 つのイベントの形に項目を足すだけにする：SessionView に経過の始まり・最終の値、中断の注意を足し、段階（Phase）に「中断中」を足す。新しいイベントは作らない。画面は知らない項目を無視するので、足すだけなら古い画面も壊れない
B. 中断と経過のための新しいイベントを作る：今のイベントには手を入れないが、画面が受け取るイベントが 3 つになり、どれが最新かの合わせ方が増える
X. Other (please specify)

[Answer]: A. 今の 2 つのイベントの形に項目を足すだけにする：SessionView に経過の始まり・最終の値、中断の注意を足し、段階（Phase）に「中断中」を足す。新しいイベントは作らない。画面は知らない項目を無視するので、足すだけなら古い画面も壊れない

## Q2. 約束を変えるときの決まりは、どうしますか？

背景：開発者は 1 人で、ライブラリ・アプリ・画面は同じリポジトリにあり、同じ `.dmg` で一緒に配ります（team-practices）。外から使う人はいません。

A. 版の番号は付けない：約束を壊す変更（名前の変更、項目の削除、意味の変更）は、使う側もすべて同じ PR で直す。足すだけの変更は、使う側が知らない項目を無視する前提で、別の PR に分けてもよい。CI（Rust のテスト、画面の型検査とテスト）が通ることを確かめの手段にする
B. 約束ごとに版の番号を付ける：変えるたびに番号を上げ、古い形も一定期間残す。今の配り方では使う人がいないため、手間に見合わない
X. Other (please specify)

[Answer]: A. 版の番号は付けない：約束を壊す変更（名前の変更、項目の削除、意味の変更）は、使う側もすべて同じ PR で直す。足すだけの変更は、使う側が知らない項目を無視する前提で、別の PR に分けてもよい。CI（Rust のテスト、画面の型検査とテスト）が通ることを確かめの手段にする

## Q3. 取得していないときに［中断］のコマンドが届いたら、どうしますか？

背景：［中断］は取得中にしか押せませんが、押した瞬間に取得が終わる、二度押しが届くなど、取得していないときにコマンドが届くことがあります（FR5.2、FR5.9）。

A. 何もせずに成功として返す：取得していなければ何もしない。すでに中断中でも何もしない（同じことを何度送っても結果は同じ）。画面はエラーを出さない
B. エラーとして返す：画面がエラーを受け取り、表示するか捨てるかを決める。押した瞬間に終わった場合にもエラーになりうる
X. Other (please specify)

[Answer]: A. 何もせずに成功として返す：取得していなければ何もしない。すでに中断中でも何もしない（同じことを何度送っても結果は同じ）。画面はエラーを出さない

## Consolidated Summary Confirmation

- 約束の範囲：外から呼ばれる API はない。決めるのは、U1 が作り U2〜U5 が使うライブラリの中の約束（CallPacer の許可待ち、StreamSchedule、FetchStatus、FetchSink と FetchJob）と、アプリと画面の間の約束（Tauri のコマンドとイベント）
- 画面へ渡すもの：今の `session-changed`（SessionView）と `fetch-progress` に項目を足すだけ。経過の始まりと最終の値、中断の注意、段階「中断中」を足す。新しいイベントは作らない（Q1）
- 変更の決まり：版の番号は付けない。壊す変更は使う側も同じ PR で直す。足すだけの変更は知らない項目を無視する前提で分けてよい。CI で確かめる（Q2）
- 中断のコマンド：取得していない、すでに中断中のときは何もせずに成功として返す。何度送っても結果は同じ（Q3）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
