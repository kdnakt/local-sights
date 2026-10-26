# Units Generation Questions — `stats` による集計と途中結果の表示（261025-insights-stats）

## Sources

- `inception/domain-design/components.md`（新規 2 部品：QueryCatalog・StatsAggregator、変更 16 部品）・`decisions.md`（ADR-001〜ADR-006）
- `inception/requirements-analysis/requirements.md`（FR1〜FR7、NFR1〜NFR12、C1〜C5）
- 依頼文の 2 つの柱：`stats` による集計（FR1〜FR3、FR6、FR7）と、クエリ実行中の途中結果の表示（FR4、FR5）
- team.md Walking Skeleton：各作業の最初の作業単位で、端から端まで通る薄い一本を作る（この計画は refactor を元にしており骨組みのチェックポイントの儀式はないが、前の作業 261017 と同じく U1 を薄い一本の形にする）
- どの作業単位も今の 1 つのアプリ（中核ライブラリ＋Tauri＋画面）に足し、作業単位ごとに別々に配ることはない（team.md Deployment、C4：配布は次の版）
- 前の作業 261017 の分け方（U1 薄い一本／U2 言語をそろえる／U3 取り足し／U4 互換の確認用プログラム）

## Q1. 作業単位の分け方は、どれにしますか？

背景：依頼の 2 つの柱（`stats` と途中結果）は、ドメイン設計では別々の部品の変更にあたります。`stats` は QueryParser・QueryPlan・StatsAggregator・QueryEvaluator・結果の形（ADR-001）と画面の表・書き出し、途中結果は QueryStore の途中の view・QueryRunner の版・TauriSink・画面の差し替えです。両方が結果の型（形つきの QueryResult）に乗るため、結果の型を最初に決める作業単位が要ります。どれから作るかは作る段階の計画で決めます。

A. 4 つに分ける（推奨）：U1 薄い一本（`stats` の最小版を画面から AWS まで通す。形つきの QueryResult、StatsAggregator の一部の関数、`by` と `bin()`、表・行の詳細・書き出しの形の分岐）／U2 `stats` をそろえる（コンソールの集計関数すべて、`stats` のあとの `sort`・`limit`、QueryCatalog による一覧の一元化、補完・色分け・エラー文言・README）／U3 途中結果（途中の view、版、QuerySink の通知、画面の差し替えと印、中断後の結果）／U4 互換の確かめ（`compat_check` に `stats` の比較を足す）
B. 3 つに分ける：U1 `stats` 全部（関数すべて・一覧の一元化・補完・README・書き出しを含む）／U2 途中結果／U3 互換の確かめ
C. 2 つに分ける：U1 `stats` 全部（互換の確かめを含む）／U2 途中結果
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. 最初の一本（U1）の `stats` には、どの集計の関数を入れますか？

背景：薄い一本は、結果の形の変更（中核・Tauri・画面・書き出しをそろえて変える、C3）が通ることを確かめるのがねらいです。関数の数は少なくても通ります。残りの関数は次の作業単位で足します（Q1 で B・C を選んだときは、この答えは U1 の中の作る順の目安になります）。

A. 依頼文の 5 つ：`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max` と、`by` の鍵（項目と `bin()`）（推奨。依頼文の例がそのまま U1 で動く）
B. `count(*)` と `by` の鍵（項目と `bin()`）だけ（最も薄い）
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. 途中結果の作業単位（U3）は、`stats` をそろえる作業単位（U2）を待ちますか？

背景：途中結果は結果の形（U1 で決まる）に乗り、`stats` の関数の数には依存しません。ドメイン設計では、`stats` を含むクエリも含まないクエリも同じ道（全体の評価し直し）で途中結果を作ります（ADR-003）。U1 のあと U2 と U3 を並べて進められるかは、重なる部品（QueryEvaluator・QueryResultTable・InsightsSession・TauriApp）をどう扱うかで決まります。

A. 待たない。U2 と U3 は U1 にだけ依存し、並べて進められる（重なる部品は U1 で決めた形の内側の変更にとどめる）（推奨）
B. 待つ。U3 は U2 のあと（`stats` がそろった結果に対して途中結果を確かめる）
C. Not yet defined
X. Other (please specify)

[Answer]: A
