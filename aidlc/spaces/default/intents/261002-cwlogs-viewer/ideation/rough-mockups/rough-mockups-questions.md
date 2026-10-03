# Rough Mockups 質問票

上流の成果物：
- `intent-statement`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/intent-capture/intent-statement.md`）
- `scope-document`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/scope-definition/scope-document.md`）
- `intent-backlog`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/scope-definition/intent-backlog.md`）

確定済みの MVP の流れ（再質問しない）：プロファイル・リージョンを切り替える → ロググループを一覧から 1 つ選ぶ → 時間範囲を指定する → ストリームを跨いだ結果を時刻順に表示する → 部分一致で絞り込む。ディスクキャッシュは任意・既定は無効（scope-document）。

## Q1. アプリの形はどれにしますか？

背景：画面の作り方が根本的に変わる決定です。利用者は開発者本人と OSS 利用者です（intent-statement）。

A. ターミナル上で動く対話型画面（TUI）
B. デスクトップ GUI アプリ（ウィンドウで操作する）
C. コマンドライン（CLI）：引数で指定して結果を標準出力に出す。パイプで他のコマンドに渡せる
D. TUI と CLI の両方（同じ機能を対話でも引数でも使える）
E. Not yet defined
X. Other (please specify)

[Answer]: B. デスクトップ GUI アプリ（ウィンドウで操作する）

## Q2. 対応する OS はどれですか？（select all that apply）

背景：OSS として公開するため、対応 OS の範囲を決めます。

A. macOS
B. Linux
C. Windows
D. Not yet defined
X. Other (please specify)

[Answer]: A, C

## Q3. 画面の構成はどちらが好みですか？

背景：ロググループの選択・時間範囲の指定・ログ表示をどう並べるかの方針です（Q1 で TUI または GUI を選んだ場合に使います）。

A. 1 画面に並べる：左にロググループ一覧、上に時間範囲とフィルター、右に大きくログ表示
B. 段階的に画面を切り替える：プロファイル選択 → ロググループ選択 → 時間範囲 → ログ表示
C. ログ表示を全画面にし、選択や指定はポップアップで行う
D. Not applicable（CLI のみの場合）
X. Other (please specify)

[Answer]: A. 1 画面に並べる：左にロググループ一覧、上に時間範囲とフィルター、右に大きくログ表示

## Q4. 時間範囲はどのように指定したいですか？

背景：古いログの調査では、特定の日時を指定することが多いと考えられます。

A. 絶対日時のみ（例：2024-03-01 10:00 〜 2024-03-01 12:00）
B. 相対指定のみ（例：直近 1 時間、直近 7 日）
C. 絶対日時と相対指定の両方
D. Not yet defined
X. Other (please specify)

[Answer]: A. 絶対日時のみ

## Q5. 時刻はどのタイムゾーンで表示・入力しますか？

背景：CloudWatch のイベント時刻は UTC 基準です。表示と入力のタイムゾーンを決めます。

A. ローカル時刻（端末のタイムゾーン）
B. UTC
C. 切り替えられるようにする（既定はローカル時刻）
D. 切り替えられるようにする（既定は UTC）
E. Not yet defined
X. Other (please specify)

[Answer]: C. 切り替えられるようにする（既定はローカル時刻）

## Q6. 操作方法とアクセシビリティの要件はありますか？（select all that apply）

背景：キーボード操作だけで完結するか、色だけに頼らない表示にするかなどを決めます。

A. キーボードだけですべて操作できる
B. マウスでも操作できる
C. 色だけで意味を伝えない（色が使えない端末でも分かる表示）
D. 特に要件はない
X. Other (please specify)

[Answer]: B, C

## Q7. （追加質問）デスクトップ GUI と 1 週間の期限の折り合いをどうつけますか？

背景：アプリの形はデスクトップ GUI（Q1）、対応 OS は macOS と Windows（Q2）になりました。一般に GUI は TUI や CLI より画面部品・OS ごとの動作確認・配布の手間が大きく、実現性評価で「MVP は 1 週間以内に作れる見込み」とした前提（feasibility の RAID ログ A-02）は、アプリの形が未定の時点のものです。

A. GUI のまま、見た目は最小限（標準的な部品だけ、デザインの作り込みなし）にして 1 週間を目指す
B. GUI のまま、1 週間は目安とし、延びてもよい
C. 1 週間を優先し、MVP は TUI で作る（GUI は MVP 後）
D. Not yet defined
X. Other (please specify)

[Answer]: A. GUI のまま、見た目は最小限（標準的な部品だけ、デザインの作り込みなし）にして 1 週間を目指す

## Consolidated Summary Confirmation

回答のまとめ：

- アプリの形：デスクトップ GUI アプリ（Q1）
- 対応 OS：macOS と Windows（Q2）
- 画面構成：1 画面に並べる。左にロググループ一覧、上に時間範囲とフィルター、右に大きくログ表示（Q3）
- 時間範囲の指定：絶対日時のみ（Q4）
- タイムゾーン：表示・入力のタイムゾーンを切り替えられる。既定はローカル時刻（Q5）
- 操作とアクセシビリティ：マウスでも操作できる。色だけで意味を伝えない（Q6）
- GUI と期限：GUI のまま、見た目は最小限（標準的な部品だけ）にして 1 週間を目指す（Q7）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
