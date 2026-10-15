<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
-  — 品質目標の ID は作業単位の頭文字をつけて区別した（U1-NFR2.1 など）; 同じ NFR 番号が作業単位ごとに別の中身を持つため。nfr-design は新しい目標を持たず、tech-stack-decisions の NFR も目標に含めた。
- 2026-10-15T11:10:34Z — 品質目標の ID は作業単位の頭文字をつけて区別した（U1-NFR2.1 など）; 同じ NFR 番号が作業単位ごとに別の中身を持つため。nfr-design は新しい目標を持たず、tech-stack-decisions の NFR も目標に含めた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
-  — U4-NFR9.2・U5-NFR9.4 の確かめ方から VoiceOver の確認を外した; 要件（NFR9・NFR10）に読み上げの条件がなく、利用者が Q2 で A を選んだため。画面のテストの結果で Met とした。
- 2026-10-15T11:10:34Z — U4-NFR9.2・U5-NFR9.4 の確かめ方から VoiceOver の確認を外した; 要件（NFR9・NFR10）に読み上げの条件がなく、利用者が Q2 で A を選んだため。画面のテストの結果で Met とした。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
-  — 要件の対応表で直接の行がない 45 件は、code-generation の traceability.json を直さず指摘として承認のときに示した; レビュー済みの作業単位の成果物は直さない決まり（project.md の Corrections）を優先したため。
- 2026-10-15T11:10:34Z — 要件の対応表で直接の行がない 45 件は、code-generation の traceability.json を直さず指摘として承認のときに示した; レビュー済みの作業単位の成果物は直さない決まり（project.md の Corrections）を優先したため。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
