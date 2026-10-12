<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

- 2026-10-12T00:00:00Z — 「上限の半分程度」は東京の 25 回の半分、1 秒あたり 12 回前後と読み替えた。ストリームの一覧の呼び出しは順にしか読めないため、この割合の対象はログを読む呼び出しとした

## Deviations
- 2026-10-12T00:00:00Z — 規制・既存システム・技術スタックの質問は前回の作業で確定済みのため聞かなかった。代わりにリージョン、使ってよい量、中断後の表示、再試行の決まり、計測環境を聞いた
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
