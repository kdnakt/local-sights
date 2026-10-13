<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-10-13T00:10:00Z — [u1-concurrent-skeleton] NFR1 の偽物の条件を「1 回 200 ミリ秒、100 本・各 1 回、東京（12 回／秒）」と決め、要件分析のレビューの指摘（偽物の時間が未定）を片付けた
- 2026-10-13T00:10:00Z — [u1-concurrent-skeleton] 受け渡しの通り道の上限は同時に読む本数（8）の 2 倍の 16 件とした

- 2026-10-13T00:40:00Z — [u1-concurrent-skeleton] レビューを受け、チャネルをやめて「1 ストリーム 1 ページずつ」を FuturesUnordered で並べる形に変えた（渡し待ちは最大 8 件、16 件の通り道はなくした）。SDK の再試行を止めるのは GetLogEvents だけにした

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
