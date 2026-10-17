<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T11:38:32Z — 範囲の定義の B5「クエリの中で期間とロググループを指定」を、クエリの画面に置く専用の欄で指定することと読み替えた（A1）; 利用者がコンソールと同じく文の外で指定することを選んだため（RA Q1）。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T11:38:32Z — コンソールの振る舞いの細部（fields の既定の列、sort の既定、型の違う値の比較など）は要件では「コンソールと同じ」とだけ書き、中身は Functional Design で AWS の資料から確かめることにした（OQ2）; 資料を確かめないまま細部を決めると互換を外すおそれがあるため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T23:20:00Z — [insights-query] 2 つの画面は取得済みのログを共有しないと見直した（RA Q3b、Q3a の見直し）; 共有するとログの画面の取得（保持中を捨てて取り直す）を作り替える必要があり、U1〜U3 の間に暫定の動きが要ると U1 の機能設計で分かった。利用者は普段は片方の画面で済むと答え、同じログを 2 回取得しメモリも 2 つ分持つことを受け入れた。
- 2026-10-17T11:38:32Z — クエリの速さの合否の数値は、利用者の答え（作る段階で決める）に従って要件では決めなかった（NFR1）; 今の基準より遅くてよいことだけを書き、測ってから決めた数値は下げない、とした。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T11:38:32Z — ログの画面で取り直したとき（今は保持中を捨てる）に、共有した取得済みの範囲をどう扱うか、2 つの画面から同じロググループを同時に取得したときの扱いは Domain Design で決める（OQ4、レビューの指摘）。
