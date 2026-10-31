# Functional Design Questions — U2 `stats` をそろえる（u2-stats-complete）

## Sources

- `inception/units-generation/unit-of-work.md`（U2 の責務、U3 との重なる部品の分担の表、レビューの持ち越しの表：DD R-10 のメモリは U2）、`unit-of-work-story-map.md`（FR1.2・FR1.4・FR1.6・FR1.7・FR6.1・FR6.2）
- `inception/requirements-analysis/requirements.md`（FR1.2・FR1.4・FR1.6・FR1.7、FR6.1〜FR6.3、NFR2・NFR7）
- `inception/domain-design/components.md`・`decisions.md`（ADR-002 StatsAggregator、ADR-005 QueryCatalog）
- U1 の機能設計（`construction/u1-stats-skeleton/functional-design/`）：U1 の 6 つの書き方、StatsAggregator の入口（new / add / finish）、BR2.1〜BR2.9 の仮の決まり、BR3.5 の Aggregate の段、BR1.6（U1 では stats は最後の命令）
- AWS の資料「CloudWatch Logs Insights query syntax」（awsdocs の `CWL_QuerySyntax.md`）：`count_distinct(項目)` は「項目の一意の値の数。非常に多い値では近似」、`pct(項目, 百分位)`（例 `pct(@duration, 95)`）、`stddev(数値の項目)` は「標準偏差」、`earliest(項目)` / `latest(項目)` は「最も早い（遅い）@timestamp のイベントの項目の値」、`sortsFirst(項目)` / `sortsLast(項目)` は「並べたときに最初（最後）に来る値」。例 `stats count(*) as exceptionCount by bin(1h) | sort exceptionCount desc`
- AWS の告知（2023-11）：1 つのクエリに `stats` を 2 つまで書ける
- 今のコード：`DEFAULT_RESULT_LIMIT` = 10,000、`MAX_LIMIT` = 100,000、`sort` は NaturalSort、`OTHER_COMMANDS`・`KEYWORDS`・`function()`、`src/analytics.ts` の色分け、`messages.ts` の `insights.error.supportedList`、README の対応表、`InsightsSession::completion_candidates`

決まっていて聞き直さないこと：関数はコンソールの集計関数すべて（RA Q1 C）。`stats` のあとに `sort`・`limit` を書ける（RA Q3 A）。一覧は中核の QueryCatalog に 1 つ持ち、README はテストで一致を確かめる（DD Q6 A）。U1 の仮の決まり（鍵なし・数でない値・0 件の 1 行・4 桁・鍵の昇順）は U4 でコンソールと比べる。

## Q1. `pct`・`stddev`・`count_distinct` の計算の方法は、どうしますか？（FR1.2、OQ1）

背景：資料は名前と意味だけで、計算の細部（百分位の補間、標準偏差の分母、近似）は書かれていません。コンソールと完全に同じにできるかは U4 で比べます。

- A. `pct(項目, p)`：p は 0〜100 の数（範囲外は読み取りのエラー）、数に読める値を昇順に並べて「最も近い順位」（nearest-rank：⌈p/100 × n⌉ 番目、p = 0 は最小）を返す。`stddev`：標本標準偏差（n − 1 で割る。値が 1 つなら 0）。`count_distinct`：手元では正確に数える（近似しない。値の文字が同じものを 1 つと数える、U1 の鍵と同じ比べ方）（推奨）
- B. `pct` は線形補間（Excel の PERCENTILE.INC と同じ）、`stddev` は母標準偏差（n で割る）、`count_distinct` は正確に数える
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q2. `stats` を含むクエリの結果の件数の上限（`limit` を書かないとき）は、どうしますか？（FR1.7）

背景：今の `fields`・`filter` だけのクエリは既定で 10,000 行で打ち切り、打ち切ったことを示します（前の作業）。コンソールの `stats` の結果も 10,000 行までと見込みますが、資料には書かれていません。`by @message` のように鍵の種類が多いと、グループの数がイベントの数と同じになりえます（DD R-10）。

- A. 表に出す行は今と同じく既定 10,000 行で打ち切り（`limit` を書けば最大 100,000）、打ち切ったことを示す。集計そのものはすべてのグループで行う（打ち切りは並べたあと）。グループの数の上限は設けず、100 万件・100 万グループでも落ちないことを NFR2 で測る（推奨）
- B. A に加えて、グループの数が 100 万を超えたら集計を止めて「グループが多すぎる」のエラーにする
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. 1 つのクエリに `stats` を 2 つ書く（例：`stats count(*) as n by bin(1m) | stats avg(n)`）書き方は、この作業で入れますか？（FR1.4、OQ2）

背景：AWS は 2023 年から 1 つのクエリに `stats` を 2 つまで書けるようにしています。2 つ目の `stats` は 1 つ目の集計の行を入力にするため、集計の行に対する項目の読み方（`n` を項目として読む）と、2 つ目の前の `filter`・`fields` の扱いが要ります。

- A. 入れない。2 つ目の `stats` は「対応していない」のエラーのまま（場所つき）にし、次の版以降に回す（推奨。依頼文の範囲は 1 つの `stats`）
- B. 入れる。`stats` を 2 つまで書け、2 つ目は 1 つ目の行を入力にする。間に書けるのは `filter`・`sort`・`limit`
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. `stats` のあとに `fields`・`filter` を書いたときと、`stats` の前に `sort`・`limit` を書いたときは、どう扱いますか？（FR1.4、OQ2）

背景：コンソールでは `stats ... | filter n > 10` のように集計の列で絞れることが知られていますが、資料には例がありません。`sort`・`limit` を `stats` の前に書くと、U1 ではそのまま適用してから集計しています（`limit 100 | stats count(*)` は 100 件を数える）。

- A. `stats` のあとに書けるのは `sort`・`limit` だけ（RA Q3 A の範囲）。`fields`・`filter` は「対応していない」のエラー。`stats` の前の `sort`・`limit` は U1 のとおり集計の前に適用する（推奨）
- B. `stats` のあとに `filter`（集計の列に対する条件）も書けるようにする。`fields` は「対応していない」
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: B

## Q5. `earliest`・`latest`・`sortsFirst`・`sortsLast` で、同じ順位の値が複数あるとき（同じ時刻のイベント、同じ値）は、どれを返しますか？（FR1.2）

背景：資料には書かれていません。同じミリ秒のイベントは珍しくありません。今の結果の並び（前の作業の FR3.5）は、同じ時刻なら（ロググループ, ストリーム, ストリーム内の順番）で決まっています。

- A. `earliest`・`latest`：同じ時刻なら、今の結果の並びの決まり（ロググループ, ストリーム, ストリーム内の順番）で先（`latest` は後）のイベントの値。`sortsFirst`・`sortsLast`：値は同じなので、どれを返しても同じ文字になる。項目を持たないイベントは無視する（推奨）
- B. 同じ時刻なら最初に見たイベント（取得の順）の値
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A
