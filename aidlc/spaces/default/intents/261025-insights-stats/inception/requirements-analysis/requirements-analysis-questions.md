# Requirements Analysis — 質問

元にしたもの：依頼文（「Logs Insights 互換クエリの残りを実装する：stats による集計（count(*)、sum、avg、min、max、bin() などを by でグループ化）と、クエリ実行中の途中結果の表示（取得が進むにつれて表の行が増えていく）。261022-small-features の依頼文で「次の別の作業に回す」とした 2 つ。」）、前の作業 261017-insights-query の要件（FR1〜FR9、NFR1〜NFR12、Out of scope に `stats` を次の版以降と記した）、今のコードの知識ベース（`codekb/local-sights/` の business-overview・architecture・code-structure。`stats` はパーサで拒まれる、結果の行は「1 行 = 1 イベントの写し」で固定、評価は取得がすべて終わってから 1 回だけ）。

前の作業で決まっていて聞き直さないこと：クエリの画面の構成（ロググループ・期間・クエリ欄・実行ボタン・結果の表）、使える命令 `fields`・`filter`・`sort`・`limit` の書き方、取得は足りない分だけ、読み取りの 3 API だけ、コンソールの細部は「コンソールと同じ」と書いて Functional Design で AWS の資料から確かめる（project.md Corrections）、書き出し（CSV / Markdown）はクエリの結果の表の列をそのまま出す。

## Q1. `stats` で使える集計の関数は、どこまで入れますか？

依頼文には count(*)、sum、avg、min、max と bin() が挙がっています。コンソールの Logs Insights にはほかに count_distinct、earliest / latest、pct（パーセンタイル）、stddev、sortsFirst / sortsLast などがあります。関数が増えるほど、評価器と互換の確かめの量が増えます。

- A. 依頼文の 5 つだけ：`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`（`bin()` は `by` の区切りに使う）
- B. A に `count_distinct`・`earliest`・`latest` を足す（よく使う 3 つ）
- C. コンソールの stats の集計関数をすべて（`pct`・`stddev`・`sortsFirst`・`sortsLast` など）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: C

## Q2. `bin()` で時刻ごとに区切ったとき、イベントのない区切り（空のビン）はどうしますか？

例：`stats count(*) by bin(5m)` で、10:05〜10:10 にイベントが 1 件もないとき。コンソールはその区切りの行を出しません（0 の行が出ない）。グラフのように眺めたいときは 0 の行があると読みやすい一方、コンソールと結果が食い違います。

- A. コンソールと同じ。イベントのない区切りの行は出さない
- B. 期間の中の区切りをすべて出し、イベントのない区切りは 0 で出す（コンソールと食い違うため、互換の確かめの組には入れない）
- C. 既定は A で、画面の切替で B にもできる
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. `stats` のあとに `sort` と `limit` を書けるようにしますか？

コンソールでは `stats count(*) as n by level | sort n desc | limit 10` のように、集計の結果の列で並べ替えたり件数を切ったりできます。`fields` と `filter` は集計の前（元のイベントに対して）だけ意味を持ちます。

- A. コンソールと同じ。`stats` のあとに `sort`・`limit` を書け、集計の結果の列（`as` で付けた名前を含む）で並べ替えと件数の制限ができる。`stats` の前には今までどおり `fields`・`filter` を書ける
- B. この版では `stats` は最後の命令だけとし、そのあとに命令を書いたら「対応していない」のエラーにする（並び順は区切りの鍵の順で固定）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. 集計の結果（stats の行）も、CSV / Markdown の書き出しの対象にしますか？

前の作業で作った書き出しは、クエリの結果の表の列をそのまま出す作りです。集計の行には元のイベントがないため、行を開いたときの全文表示や `@message` の全文を入れる決まりはあてはまりません。

- A. 対象にする。表に出ている列（区切りの鍵と集計値）をそのまま CSV / Markdown に書く
- B. この版では対象外。`stats` を含むクエリの結果では［Export］を無効にし、理由を示す
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q5. 途中結果（取得が進むにつれて表の行が増える）は、どのクエリに適用しますか？

今は取得がすべて終わってから評価し、結果を 1 回で出します。`filter` と `fields` だけのクエリは、届いたイベントをそのまま評価して行を足していけます。`sort`・`limit`・`stats` を含むクエリは、新しいイベントが届くたびに並び順や件数や集計値が変わるため、途中結果を出すなら「届いた分で作り直して差し替える」形になります。

- A. どのクエリも途中結果を出す。`filter`・`fields` だけなら行が末尾に増え、`sort`・`limit`・`stats` を含むなら届いた分で作り直した結果に差し替える（集計値が更新され、並び順が入れ替わることがある）
- B. `filter`・`fields` だけのクエリは途中結果を出し、`sort`・`limit`・`stats` を含むクエリは今までどおり取得が終わってからまとめて出す（途中は件数の進み具合だけ）
- C. 途中結果は出さず、実行中の件数の進み具合だけを今より細かく見せる（この依頼の「途中結果」を見送る）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q6. 取得の途中で［Cancel］を押したとき、そこまでの途中結果を残しますか？

前の作業の決まり（FR2.6）では、中断するとクエリを評価せず「止めた」とだけ示します。途中結果が出るようになると、コンソールの「クエリを停止」のように、止めた時点までの結果を残す選び方もできます。

- A. 残す。止めた時点までに取得した分で評価した結果を、「一部のログだけの結果（中断）」の印つきで最終結果として出す
- B. 残さない。前の決まりのとおり、中断すると途中結果も消し、止めたことだけを示す
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q7. コンソールとの互換の確かめ（手元の確認用プログラム）に、`stats` のクエリも足しますか？

前の作業では、AWS の資料の例のうち対応する範囲のクエリを組にして、コンソールの結果とこのアプリの結果を手元の確認用プログラム（`compat_check`）で比べました。`stats` の結果は行の形が違うため、比べ方（集計値の一致、`bin()` の区切りの一致）を足す必要があります。

- A. 足す。`count(*) by bin(5m)` などの集計のクエリも組に入れ、集計の行の集まりが一致することを比べる
- B. 足さない。`stats` は手元で目で確かめるだけにする
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A
