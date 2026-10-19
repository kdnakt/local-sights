# Business Rules — U4 互換の確認用プログラム（u4-compat-check）

上流：`inception/units-generation/unit-of-work.md`（U4）、`inception/requirements-analysis/requirements.md`（SM1、NFR3、NFR4、OQ5、D17〜D19）、質問票 `functional-design-questions.md`（Q1 = A）、U2 の `code-summary.md`（U4 に引き継ぐ仮の決まり：型の比べ方、AND・OR・NOT・NOT LIKE・NOT IN の大文字、`limit 0` と 10,000 件を超えるログ、sort がないときの並び）、U1 の決まり（`@log` の値はロググループ名だけ）、U2 の Q1a・Q1b（CloudTrail のロググループで CLI から流せた）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: Logs Insights の結果は、開発者の手元で CLI のスクリプトから流して用意する。アプリと確認用プログラムは Logs Insights の API を呼ばない
    category: policy
    applies_to: insights-export.sh（スクリプト）、compat_check（確認用プログラム）
    trigger: 比べる前
    logic: スクリプトはクエリの組のファイルを 1 行ずつ読み、aws logs start-query と get-query-results で流して、クエリごとに 1 つの JSON のファイルに保存する（U2 の console-check.sh と同じ流し方）。確認用プログラムが呼ぶ AWS API は、今のアプリと同じ DescribeLogStreams・GetLogEvents だけ
    violation: なし
    source: Q1、NFR5、project.md Forbidden
  - id: BR1.2
    statement: 書き出しのファイルはリポジトリの外のフォルダに置き、リポジトリの中のフォルダを指したら動かない
    category: constraint
    applies_to: insights-export.sh、compat_check
    trigger: 書き出しのフォルダを受け取ったとき
    logic: フォルダの実際の場所（シンボリックリンクを解いたもの）が、リポジトリの最上位のフォルダの中なら、スクリプトも確認用プログラムもエラーで止まる。報告に出すのはクエリの id・件数・違いの行の要約（BR3.2）だけで、書き出しの全体は出さない
    violation: エラーで止まる
    source: NFR4
  - id: BR1.3
    statement: 比べる期間の終わりは、流す時刻の 30 分以上前にする
    category: validation
    applies_to: insights-export.sh、compat_check
    trigger: 期間を受け取ったとき
    logic: 遅れて届くログで、Logs Insights とアプリの見るイベントがずれないようにする。終わりが 30 分以内なら、どちらもエラーで止まる
    violation: エラーで止まる
    source: SM1（同じロググループ・同じ期間）
  - id: BR1.4
    statement: スクリプトと確認用プログラムは、同じ接続・ロググループ・期間・フォルダを受け取る
    category: constraint
    applies_to: insights-export.sh、compat_check
    trigger: 実行のとき
    logic: スクリプトは条件を書き出しのフォルダに settings.json として残し、確認用プログラムはそれを読んで同じ条件でアプリの取得をする。違う条件を渡したらエラー
    violation: エラーで止まる
    source: SM1

  - id: BR2.1
    statement: 行の集まりは、両方にある列の値で比べ、@ptr は比べない
    category: calculation
    applies_to: compat_check
    trigger: 1 本のクエリを比べるとき
    logic: Logs Insights の行は {field, value} の並び。アプリの行は QueryResult の列と値。@ptr を除いた列の名前の集まりが同じかを先に確かめ、違えば Different（列の違いとして報告）。同じなら、各行を「列の名前と値」の組にして、多重集合として比べる
    violation: なし
    source: SM1、D17
  - id: BR2.2
    statement: 値は、Logs Insights の書き方に合わせて文字列にしてから比べる
    category: calculation
    applies_to: compat_check
    trigger: アプリの行の値を比べるとき
    logic: '@timestamp は UTC の `YYYY-MM-DD HH:MM:SS.sss`（Q1b の出力と同じ形）。JSON の数は JSON に書かれた文字列のまま、真偽値は true・false、文字列はそのまま。値がない列は、Logs Insights がその行にその列を返さないのと同じに扱う'
    violation: なし
    source: SM1、U2 の Q1a・Q1b の出力
  - id: BR2.3
    statement: 並び順は、sort を書いたクエリだけで比べる
    category: calculation
    applies_to: compat_check
    trigger: compareOrder のクエリ
    logic: 行の集まりが同じとき、先頭から 1 行ずつ比べ、最初に違った番号を報告する。sort の値が同じ行どうしの順番は、比べた sort の項目の値が同じなら違いとみなさない（同順位の並びはアプリの決まりで決めているため、FR3.4）
    violation: なし
    source: D18、FR3.4、FR3.5
  - id: BR2.4
    statement: 片方だけが書き方を断ったら違いとし、両方が断ったら同じとみなす
    category: policy
    applies_to: compat_check
    trigger: Logs Insights が Error か、アプリが QueryProblem を返したとき
    logic: 両方が断った → BothRejected（違いとしない）。アプリだけ → OnlyAppRejected、Logs Insights だけ → OnlyInsightsRejected（どちらも違いとして報告し、断った文を出す）
    violation: なし
    source: SM2、FR6
  - id: BR2.5
    statement: どちらかが件数の上限に当たったクエリは、比べずに Skipped とする
    category: policy
    applies_to: compat_check
    trigger: 件数が 10,000（limit を書かないとき）か limit の数に当たったとき
    logic: 上限で切られた行は、どのイベントが残るかが両方の並べ方に左右されるため、集まりで比べられない。ただし sort を書いたクエリは並びが決まるため比べる。Skipped の理由を報告する
    violation: なし
    source: FR3.7、OQ1
  - id: BR2.6
    statement: すべてのクエリが Same か BothRejected なら成功、1 つでも違いがあれば失敗の終了コードで終わる
    category: policy
    applies_to: compat_check
    trigger: すべてのクエリを比べたあと
    logic: 終了コード 0 は互換、1 は違いあり、2 は動かせなかった（条件の誤り、取得の失敗）。Skipped は数を報告するが、それだけでは失敗にしない
    violation: なし
    source: SM1

  - id: BR3.1
    statement: クエリの組は、AWS の資料の例を手元のログに合わせたものと、U2 の仮の決まりを確かめるものからなる
    category: policy
    applies_to: compat_queries.txt（クエリの組のファイル）
    trigger: クエリの組を作るとき
    logic: AWS の資料の filter・fields・sort・limit の例（CWL_QuerySyntax-Filter・-Fields・-Sort・-Limit）を、手元の CloudTrail のロググループの項目（eventSource、eventName、errorCode、userIdentity.type、readOnly、eventVersion など）に合わせる。仮の決まりの組として、数字だけの文字列と数の < >、数字でない文字列どうしの順、真偽値、JSON の数と文字列、AND・OR・NOT・NOT LIKE・NOT IN の大文字、sort がないときの並び（集まりだけを比べる）、ない値の並び、limit 0 を入れる。@log の値と、日時の文字列との比べは入れない
    violation: なし
    source: NFR3、OQ5、U2 の code-summary.md、U1 の決まり
  - id: BR3.2
    statement: 報告には、クエリごとの結果と、違いの行の要約だけを出す
    category: policy
    applies_to: compat_check
    trigger: 報告を出すとき
    logic: クエリごとに id・結果・両方の件数を 1 行で出し、違いがあれば片方にだけある行を最初の 5 つまで、時刻と、列の名前と値の先頭 40 文字で出す。最後に Same・Different・Skipped などの数を出す。認証情報は出さない
    violation: なし
    source: NFR4、project.md Forbidden
  - id: BR3.3
    statement: 比べの結果は、作る段階の記録に残す
    category: policy
    applies_to: 開発者、code-summary.md
    trigger: 手元で比べたあと
    logic: 報告の要約（ロググループの種類、期間の長さ、件数、結果の数、違いのあったクエリと原因の見立て）を U4 の code-summary.md と Build and Test の記録に残す。ログの中身は残さない。違いがあれば、アプリを直すか、違いを受け入れて README に書くかを決める
    violation: なし
    source: unit-of-work.md の U4 の確かめ方、SM1
```

## まとめ

| ID | 決まり | 出典 |
|---|---|---|
| BR1.1〜BR1.4 | 用意：CLI のスクリプトで結果を用意する、書き出しはリポジトリの外、期間は 30 分以上前に終わる、同じ条件を使う | Q1、NFR4、NFR5、SM1 |
| BR2.1〜BR2.6 | 比べ方：列と値の多重集合、Logs Insights の書き方に合わせる、並びは sort のときだけ、断り方、上限に当たったら比べない、終了コード | SM1、SM2、D17、D18 |
| BR3.1〜BR3.3 | クエリの組と報告：資料の例と仮の決まり、要約だけを出す、記録に残す | NFR3、OQ5 |
