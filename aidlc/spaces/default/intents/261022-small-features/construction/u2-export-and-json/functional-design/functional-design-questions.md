# Functional Design Questions — U2 書き出しと JSON の整形（u2-export-and-json）

上流：`inception/units-generation/unit-of-work.md`（U2 の責務・実装の注意）、`inception/requirements-analysis/requirements.md`（FR2〜FR5、OQ1〜OQ3）、`inception/domain-design/components.md`・`decisions.md`（ADR-001〜ADR-004、ADR-006）。

要件で機能設計に送られた 3 つの点（OQ1〜OQ3）と、書き出した表の見出しの文字を聞く。ほかの細部（写しの持ち方、通知の出し方、処理の順、二重実行の防止、JSON の整形の字下げ幅など）は設計の判断で決め、`rules.md` の「決めたこと」に書く。

## Q1 ログの画面の書き出しの見出し（FR2.2、FR4.2.1、FR4.3.1）

ログの画面の 3 列（時刻・ストリーム・メッセージ）を CSV と Markdown に書くとき、1 行目の見出しは何にしますか？ クエリの画面はクエリの `fields` の名前（`@timestamp`、`@message` など）がそのまま見出しになるため、ログの画面もそれに合わせるのが推奨です。

A. `@timestamp`・`@logStream`・`@message`（推奨：マネジメントコンソールの項目名と同じ。言語に依存せず、クエリの画面の見出しとも形がそろう）
B. 画面の列名を表示の言語で（「時刻（JST）」「ストリーム名」「メッセージ」／"Time (JST)"…）
C. 英語の固定の語（`timestamp`・`stream`・`message`）
X. Other (please specify)

[Answer]: A

## Q2 Markdown の値の中の改行の置き換え（FR4.3.2、OQ2）

Markdown の表は 1 行が 1 レコードなので、メッセージの中の改行をそのままは書けません。何に置き換えますか？

A. `<br>` に置き換える（推奨：GitHub・多くの Markdown の表示器が表のセルの中の `<br>` を改行として描く。元の改行の位置が残る）
B. 半角の空白 1 つに置き換える（どこでも崩れないが、改行の位置は分からなくなる）
C. `\n` の 2 文字に置き換える（文字として残るが、表示では改行されない）
X. Other (please specify)

[Answer]: A

## Q3 保存するファイルの既定の名前（FR4.4.1、OQ3）

保存ダイアログに最初に出す名前の形を決めてください。ロググループ名と日時を含めるのは要件どおりで、`/` などファイル名に使えない文字は `_` に置き換えます。

A. `<ロググループ名>_<yyyyMMdd-HHmmss>.<csv|md>`（推奨：例 `_aws_lambda_my-func_20261024-213045.csv`。短く、同じロググループの書き出しが名前順で時刻順に並ぶ。日時は選んでいるタイムゾーンの時刻）
B. `local-sights_<ロググループ名>_<yyyyMMdd-HHmmss>.<csv|md>`（アプリ名を先頭に付け、ほかのファイルと区別しやすい）
C. `<yyyyMMdd-HHmmss>_<ロググループ名>.<csv|md>`（日時を先頭にし、ロググループをまたいで時刻順に並ぶ）
X. Other (please specify)

[Answer]: A

## Q4 メッセージの中の JSON とみなす条件（FR5.2、OQ1）

行を開いたときの全文の表示で、どこまでを整形しますか？ どの案でも、JSON として読み切れない部分（閉じていない・壊れている）は整形せずそのまま出します（FR5.3）。

A. 最初に見つかった完全な JSON（`{…}` か `[…]`）だけを整形し、その前と後ろの文字はそのまま残す（推奨：ログの大半は「前置き＋JSON 1 つ」の形。後ろに文字が続いても壊さない。実装と確認が簡単）
B. メッセージの中の完全な JSON をすべて見つけて、それぞれ整形する（JSON が 2 つ以上並ぶログにも効くが、間の文字の扱いが複雑になる）
C. メッセージ全体が 1 つの JSON のときだけ整形する（前置きつきのログは整形しない。要件 FR5.1 の「途中から始まる JSON も整形する」と合わないため非推奨）
X. Other (please specify)

[Answer]: A
