# Scope Definition 質問票

上流の成果物：
- `intent-statement`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/intent-capture/intent-statement.md`）
- `feasibility-assessment`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/feasibility/feasibility-assessment.md`）
- `constraint-register`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/feasibility/constraint-register.md`）

確定済みの前提（再質問しない）：MVP は 1 週間以内。MVP は利用者が選んだロググループ・ストリーム（または時間範囲）だけを取得・表示・フィルタし、ロググループ全体の横断取得は MVP 後（feasibility Q5、Q9）。原因調査は対象外（feasibility Q8）。

## Q1. MVP の取得・表示まわりで必須なのはどれですか？（select all that apply）

背景：1 週間で作る範囲を決めるため、取得・表示の機能を必須とそれ以外に分けます。対象環境は複数アカウント・複数リージョンです（feasibility Q1）。

A. プロファイル・リージョンの切り替え
B. ロググループ・ストリームを一覧から選ぶ
C. 時間範囲（開始・終了）を指定して取得する
D. 複数のストリームを選び、時刻順にまとめて表示する
E. None（ストリーム名を直接指定できれば十分）
X. Other (please specify)

[Answer]: A, C, X. Other: プロファイルとリージョン切り替えはやりたい。かつ、その権限で取得できるロググループを一覧から1つ選んで時間範囲を指定して、ログストリームを跨いだ結果を閲覧したい。

## Q2. MVP のフィルターはどのレベルが必要ですか？

背景：フィルターの表現力で作業量が大きく変わります。将来の Insights 風クエリとは別に、MVP で最低限必要なものを決めます。

A. 文字列の部分一致のみ
B. 部分一致＋正規表現
C. CloudWatch のフィルターパターン互換（例：`?ERROR ?WARN`、JSON フィールド条件）
D. 部分一致の複数条件（AND／OR／NOT）
E. Not yet defined
X. Other (please specify)

[Answer]: A. 文字列の部分一致のみ

## Q3. 次のうち MVP に含めたいものはありますか？（select all that apply）

背景：あると便利だが必須ではない候補です。含めるほど 1 週間の期限が厳しくなります。

A. 表示中の結果をファイルに書き出す（テキスト／JSON）
B. 取得済みログのディスクキャッシュ（任意・既定は無効。feasibility Q3）
C. 取得の進み具合の表示と、途中での中断
D. JSON 形式のログメッセージを整形して表示する
E. None（どれも MVP 後でよい）
X. Other (please specify)

[Answer]: B. 取得済みログのディスクキャッシュ（任意・既定は無効）

## Q4. MVP の後、どちらを先に進めたいですか？

背景：意図整理では最終目標に「Logs Insights のような複雑なクエリ」がありました。実現性評価では「ロググループ全体の横断取得」を MVP 後に回しました。

A. ロググループ全体の横断取得（大量ストリームの取得の高速化・キャッシュ）を先に
B. Insights 風クエリを先に
C. 両方を並行して少しずつ
D. Not yet defined
X. Other (please specify)

[Answer]: A. ロググループ全体の横断取得（大量ストリームの取得の高速化・キャッシュ）を先に

## Q5. 将来の Insights 風クエリは、どこまでを目指しますか？

背景：今回のワークフローで作るのは MVP ですが、後の設計で拡張しやすくするため、目指す範囲を確認します。

A. 項目の抽出・絞り込み・並べ替え・件数制限（fields / filter / sort / limit 程度）
B. A に加えて集計（stats、count、時間ごとの集計）
C. Logs Insights のクエリ構文との互換を目指す
D. Not yet defined
X. Other (please specify)

[Answer]: C. Logs Insights のクエリ構文との互換を目指す

## Q6. 明示的に対象外にするものはどれですか？（select all that apply）

背景：スコープの境界をはっきりさせ、後から範囲が膨らまないようにします。

A. リアルタイムの追従表示（tail）
B. ログの書き込み・削除・保持期間変更などの変更操作
C. Logs Insights や FilterLogEvents API の利用
D. Web ブラウザ版やサーバー版
E. None
X. Other (please specify)

[Answer]: A, B, C, D

## Q7. （追加質問）時間範囲が広く、対象ストリームが多い場合、MVP ではどう振る舞えばよいですか？

背景：MVP はロググループを 1 つ選び、時間範囲を指定してストリームを跨いだ結果を表示します（Q1）。対象のロググループはストリーム数千以上（feasibility Q6）で、GetLogEvents はストリーム単位・毎秒 25 回までです。時間範囲が広いと、実質的にロググループ全体の横断取得（MVP 後の項目、feasibility Q9）と同じ負荷になります。一方、進捗表示と中断は MVP に含めていません（Q3）。

A. そのまますべて取得する（時間がかかってもよい）
B. 対象ストリーム数や件数に上限を設け、超えたら警告して時間範囲を狭めてもらう
C. 上限は設けないが、進捗表示と中断だけは MVP に入れる
D. Not yet defined
X. Other (please specify)

[Answer]: A. そのまますべて取得する（時間がかかってもよい）

## Consolidated Summary Confirmation

回答のまとめ：

- MVP の取得・表示：プロファイルとリージョンを切り替え、その権限で取得できるロググループを一覧から 1 つ選び、時間範囲を指定して、ログストリームを跨いだ結果を閲覧する（Q1）
- MVP のフィルター：文字列の部分一致のみ（Q2）
- MVP に含める任意機能：取得済みログのディスクキャッシュ（任意・既定は無効）（Q3）
- MVP 後の優先：ロググループ全体の横断取得（大量ストリームの取得の高速化・キャッシュ）を先に（Q4）
- 将来の Insights 風クエリ：Logs Insights のクエリ構文との互換を目指す（Q5）
- 明示的に対象外：リアルタイム追従（tail）、変更操作、Logs Insights／FilterLogEvents API の利用、Web 版・サーバー版（Q6）
- 時間範囲が広い場合の MVP の振る舞い：上限を設けず、そのまますべて取得する（時間がかかってもよい）（Q7）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
