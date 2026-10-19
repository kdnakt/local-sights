<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T11:54:21Z — ラフな画面設計がないため、要件と今の画面（261010 の mockups と今のコード）から画面を起こした; 新しい内容を作ったのではなく、今の Log events の部品と文言の決まりをクエリの画面に当てはめた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T11:54:21Z — 期間とロググループの選択は 2 つの画面で別々に持つことにした（取得済みのログだけを共有）; 要件は取得済みの範囲の共有だけを決めており、入力まで共有すると一方の画面の操作がもう一方を黙って変えてしまうため。
- 2026-10-17T23:30:00Z — [insights-query] 要件の見直し（RA Q3b：取得済みのログを共有しない）に合わせ、画面設計は「2 つの画面の間の約束」・画面 4 の注記・補完の候補元だけを直し、画面の構成は変えなかった; 共有の有無は画面の見た目には出ず、どちらの画面の取得がもう一方を変えないことを書けば足りるため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T11:54:21Z — クエリ欄は利用者の選択どおり最初から CodeEditor（Ace）にした; 色分け・補完・エラーの印をそのまま載せられる代わりに、依存と配布物が増え、Tauri 上での動作は作る段階で確かめる必要がある（レビューの指摘）。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T11:54:21Z — 接続を変えたときと未選択のときのクエリの画面の扱い、選べるロググループの数の上限、2 つの画面から同じロググループを同時に取得するときの扱いは Domain Design 以降で決める（レビューの指摘）。
