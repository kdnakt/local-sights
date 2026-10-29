# Functional Design Questions — U1 薄い一本（u1-stats-skeleton）

## Sources

- `inception/units-generation/unit-of-work.md`（U1 の責務・確かめ方・実装の注意、レビューの持ち越しの表）、`unit-of-work-story-map.md`
- `inception/requirements-analysis/requirements.md`（FR1.1・FR1.2（6 つの書き方）・FR1.3・FR1.5・FR1.6（仮の決まり）・FR1.8、FR2.1〜FR2.3、FR3.1〜FR3.6、FR6.3、NFR2・NFR7・NFR9・NFR10）、そのレビュー R-04（0 件のときの `by` なし）・R-05（鍵に書ける式）
- `inception/domain-design/components.md`・`decisions.md`（ADR-001 形つきの QueryResult、ADR-002 StatsAggregator、ADR-006）、そのレビュー R-01（行は写し）・R-05（時刻のセルは表示時に整形）・R-09
- AWS の資料「CloudWatch Logs Insights query syntax」（awsdocs の `CWL_QuerySyntax.md`、2026-10-26 に参照）：`stats` は「ログの項目の値から集計の統計を計算する」。集計の関数：`avg(数値の項目)`・`count()` / `count(*)` / `count(項目)`・`count_distinct(項目)`（非常に多い値では近似）・`max(項目)`（結果は項目の値の型）・`min(項目)`（同）・`pct(項目, 百分位)`・`stddev(数値の項目)`・`sum(数値の項目)`。集計以外の関数：`earliest(項目)`・`latest(項目)`・`sortsFirst(項目)`・`sortsLast(項目)`。`bin(period)`：「`@timestamp` の値を与えた期間に丸めて切り捨てる」、期間は「数と `m`（分）または `h`（時）」。`as` の別名は `fields`・`stats`・`sort` で使える。例：`stats count(*) as exceptionCount by bin(1h) | sort exceptionCount desc`。項目のないイベントや数値でない値の扱いは書かれていない
- 今のコード：`QueryResult { run_id, evaluated_count, rows: Vec<ResultRow { event, log_group_name }>, columns: Vec<FieldRef>, matched_count, truncated_at }`、`QueryPlan::columns() -> Vec<FieldRef>`、`EvalOutput.rows: Vec<usize>`（保持の位置）→ `QueryStore::copy_rows` で写し、`QueryRow`（`LogEvent` を平たく含む＋`display_time`＋`values`）、`ResultSummary.columns: Vec<String>`、`ExportSnapshotSource::LogAnalytics { columns, rows, zone }`、`FieldValue`（Missing / Number(f64) / Text / Boolean）、`EventFields::display`（JSON の数は serde_json の書き方、真偽値は `1` / `0`）、`compat/compare.rs` は列の名前と行の文字で比べる

決まっていて聞き直さないこと：U1 の関数は 6 つの書き方（`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`、UG Q2 A）。`stats` は U1 では最後の命令で、あとに命令があれば「対応していない」（U2 で `sort`・`limit`）。結果は 1 つの QueryResult に形（イベントの行／集計の行）を持たせ、行はすべて写し（ADR-001、DD R-01）。空のビンの行は出さない（FR2.2）。時刻のセルはミリ秒で持ち、表示と書き出しのときに画面のタイムゾーンで整形する（FR2.3、DD R-05）。中断の扱い・途中結果は U3。

## Q1. `bin()` の期間の書き方は、U1 でどこまで受け付けますか？（FR2.1、OQ2）

背景：AWS の資料には「数と `m` または `h`」とあり、コンソールでは `bin(30s)`・`bin(1d)` も通ることが知られています。区切りは「`@timestamp` を期間で丸めて切り捨てる」＝UTC のエポック（1970-01-01 00:00:00 UTC）からの倍数に切り捨てる、と読みます（A8）。

- A. `s`（秒）・`m`（分）・`h`（時）・`d`（日）を受け付け、どれもエポックの倍数に切り捨てる（`1d` は UTC の日付の境目）。`ms`・`w`・`mo`・`y` は「対応していない」のエラー（推奨。資料とコンソールの両方で確かめられる範囲）
- B. 資料どおり `m` と `h` だけ
- C. `ms`・`s`・`m`・`h`・`d`・`w` まで受け付ける（`w` の境目は木曜 0:00 UTC＝エポックの倍数）
- D. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q2. `by` を書かない `stats` で、集計するイベントが 0 件のとき（取得が 0 件、`filter` で全部落ちた）、結果は何行にしますか？（RA レビュー R-04、FR1.3・FR3.2）

背景：AWS の資料に書かれていません。SQL の `SELECT count(*) FROM ...` は 0 の 1 行を返し、コンソールも「0 件のときに count(*) が 0 の 1 行」を出すと見込みますが、確かめられるのは U4 の互換の確かめです。`by` を書いたときは 0 行（FR3.2）で、こちらは迷いがありません。

- A. 1 行を出す：`count(*)`・`count(項目)` は 0、`sum`・`avg`・`min`・`max` は値なし（空のセル）。U4 でコンソールと比べ、違えば直す（推奨。SQL と同じ）
- B. 0 行にし、「結果は 0 行」の表示（FR3.2）にそろえる
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. `by` の鍵の項目を持たないイベントは、どう扱いますか？（FR1.6）

背景：例：`stats count(*) by level` で、`level` を持たないイベント（JSON でないメッセージなど）。AWS の資料には書かれていません。SQL の `GROUP BY` は NULL を 1 つのグループにまとめます。

- A. 鍵の値が「なし」のグループを 1 つ作り、その行の鍵のセルは空にして出す（SQL と同じ。複数の鍵のときは鍵ごとに「なし」を区別する）。U4 でコンソールと比べる（推奨）
- B. 鍵の項目を 1 つでも持たないイベントは集計から外す（行に出ない）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. `sum`・`avg` の対象が数値でない値、`min`・`max` の対象が文字列のときは、どう扱いますか？（FR1.6）

背景：資料では `sum`・`avg` は「数値の項目」、`min`・`max` は「項目の値（型を問わない）」、`count(項目)` は「項目を持つイベントの数」です。JSON の値は数・文字列・真偽値があり、文字列の `"200"` のような数に読める値もあります。

- A. `sum`・`avg`：数（JSON の数か、数に読める文字列）だけを足し、それ以外と項目のないイベントは無視する（無視したイベントしかなければ値なし）。`min`・`max`：数は数として、文字列は文字列として比べ、数と文字列が混ざるときは `sort` と同じ順（数でない値 → 数、文字は自然順）で比べる。`count(項目)`：項目を持つイベント（値が空の文字列でも）を数える。U4 でコンソールと比べる（推奨）
- B. `sum`・`avg` は JSON の数だけを足し、数に読める文字列は無視する。ほかは A と同じ
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q5. 集計値の表示の文字（小数の桁と丸め）は、どうしますか？（FR1.5、A7）

背景：`avg` と、`sum` の対象が小数のときに小数が出ます。コンソールは `avg` を小数 4 桁で表示する（例：`200.3333`）と見込みますが、資料には書かれていません。表と CSV / Markdown は同じ文字にします（A7）。

- A. 整数に収まる値は整数のまま（`3`、`600`）、小数は小数点以下 4 桁に丸め（四捨五入）、末尾の 0 は落とす（`200.3333`、`1.5`）。`count`・`count(項目)` は常に整数。U4 でコンソールと比べ、違えば直す（推奨）
- B. 丸めず、浮動小数の最短の表記で出す（`200.33333333333334`）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q6. `stats` の引き数と `by` の鍵に書けるものは、U1 でどこまでにしますか？（FR1.3、RA レビュー R-05）

背景：FR1.3 は「決まった項目、JSON の項目、`bin()`、`as` で名前を付けた式」と書きます。前の作業で関数は `toMillis(@timestamp)` だけに対応しています。

- A. 引き数：`*`（`count` だけ）、決まった項目（`@timestamp`・`@message`・`@logStream`・`@log`）、JSON の項目、`toMillis(@timestamp)`。鍵：決まった項目、JSON の項目、`bin(期間)`。どちらも `as 名前` で列の名前を付けられる。演算（`+` など）や他の関数は「対応していない」のエラー（FR6.3）（推奨）
- B. A から `toMillis(@timestamp)` を外す（引き数は項目だけ）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A
