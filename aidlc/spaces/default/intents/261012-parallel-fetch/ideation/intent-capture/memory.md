<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-10-12T00:00:00Z — Q3 の「半分以下」は選択肢 A の例として示したもので、確定した数値ではないと解釈した。測る条件と数値は後のステージで決める
- 2026-10-12T00:00:00Z — Q7 で選ばれなかった項目（進捗表示と中断、キャッシュ）は除外とも要件ともみなさず、Q8・Q9 の追加質問で確かめた

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-10-12T00:00:00Z — project.md の Corrections にある「ストリームは 1 つずつ順に取得する」（u3-fetch-robustness）は今回の作業と食い違うため、後のステージでこの決まりをどう扱うか決める必要がある
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
