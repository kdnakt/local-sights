# Functional Design Questions — U1 薄い一本（u1-filter-skeleton）

## Sources

- `inception/units-generation/unit-of-work.md`（U1 の責務・確かめ方・実装の注意）、`unit-of-work-story-map.md`
- `inception/requirements-analysis/requirements.md`（FR1、FR3.1・FR3.2、FR5、FR6.3、FR7.2、FR7.6、NFR5）
- `inception/domain-design/components.md`・`decisions.md`（ADR-002 正規表現の重さへの備え、ADR-005 取得は同時に 1 つ）
- AWS の資料「filter」（CWL_QuerySyntax-Filter）：`like`・`not like`・`=~` は大文字・小文字を区別する。`(?i)` で区別しない一致にできる。`in` は文字列全体が一致するものだけ。正規表現は `/.../` で書く
- AWS の資料「Boolean, comparison, numeric ... functions」：比較は `=`・`!=`・`<`・`>`・`<=`・`>=`
- AWS の資料「Datetime functions」の注記と `toMillis(@timestamp)` の例、AWS re:Post の質問（日時の文字列との比較が動かなかった報告）
- 今のコード：接続を変えるとき、表示中のログがあれば確認ダイアログを出す（261002 U2 の Q6）

## Q1. U1 では、クエリの画面で取得すると Log Management の画面の表示中のログも入れ替わります（U3 までの暫定の動き）。これを利用者にどう示しますか？

背景：U1 の取得は今の流れ（保持中を捨てて取り直す）を使うため、Log Management の画面で見ていたログが、クエリの実行で別のロググループ・期間のログに入れ替わります。U3 で共有の保持に載せ替えると、この入れ替わりはなくなります。

A. Log Management の画面に表示中のログがあるときだけ、クエリの実行の前に確認ダイアログを出す（「Log Management の画面のログが入れ替わります」）。接続を変えるときの確認と同じ作り（推奨）
B. 確認は出さず、Log Management の画面の上に「Log Analytics で取得したログを表示しています」と示す
C. 何も示さない（U3 までの短い期間のため）
D. Not yet defined
X. Other (please specify)

[Answer]: C. 何も示さない（U3 までの短い期間のため）

## Q2. `filter` で `@timestamp` を比べるとき、どんな書き方を受け付けますか？

背景：FR1.5 で、文の中の `@timestamp` の条件は結果を絞ると決めています。利用者の依頼で調べた結果は次のとおりです。

- AWS の資料（Datetime functions）：「CloudWatch Logs Insights は、人が読める形の日時でログを絞ることには今は対応していない」。公式の例は `filter toMillis(@timestamp) >= (now() * 1000 - 7200000)` のように、`toMillis` でミリ秒の数値にしてから比べている
- AWS re:Post：`filter @timestamp >= '2020-06-01T00:00:00.000+00:00'` は動かなかったという報告がある
- `@timestamp` をそのまま数値と比べる書き方（`@timestamp > 1760659200000`）は、資料に書かれていない

A. `toMillis(@timestamp)` と数値の比較だけを受け付ける（公式の例と同じ書き方。FR5.5 の「関数は対応しない」の例外として `toMillis` の 1 つだけを足す。`now()` は対応しない）（推奨）
B. A に加えて、`@timestamp` をそのまま数値と比べる書き方も受け付ける（コンソールで動くかは U4 で確かめる）
C. U1 では `@timestamp` の比較を作らず、後の作業単位に回す（期間の欄で足りる）
D. Not yet defined
X. Other (please specify)

[Answer]:

## Q3. U1 では、結果の行の件数に上限を設けますか？

背景：`limit` と、`limit` を書かないときの上限（コンソールと同じ数、OQ1）は U2 で作ります。U1 では、取得したログのうち条件に合うものをすべて表に出すか、先に上限を入れるかを決めます。今の Log Management の画面は 100 万件まで表示できます。

A. U1 では上限を設けず、条件に合うものをすべて出す。上限は U2 で入れる（推奨）
B. U1 から、コンソールと同じ上限で打ち切る
C. Not yet defined
X. Other (please specify)

[Answer]:

## Q4. 重すぎるクエリへの備え（ADR-002）として、クエリの文の長さに上限を設けますか？

背景：正規表現は利用者の文から作ります。このアプリが使う正規表現の仕組みは、入力の長さに比例する時間で一致を調べるため、極端に遅くなる書き方は起きません。残る心配は、とても長い文や大きな正規表現でメモリを多く使うことです。コンソールに渡せるクエリの文は 10,000 文字までです（StartQuery の queryString の上限）。

A. コンソールと同じく 10,000 文字までにする。超えたら実行を押せなくし、理由を示す。正規表現の大きさの上限は仕組みの既定のままにし、超えたらエラーにする（推奨）
B. 文の長さの上限は設けず、正規表現の大きさの上限だけにする
C. Not yet defined
X. Other (please specify)

[Answer]:
