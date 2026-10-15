<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-15T11:49:09Z — CI の道具・枝の進め方・ゲート・成果物の置き場所は team.md で決まっているため質問せず、記録から答えた; 質問は CI の変更（確認用プログラムのテスト）と phase-check の扱いの 2 つに絞った。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-15T11:49:09Z — phase-check では、直接の対応で FAIL の要件対応表を利用者の判断（Q2=A）で受け入れて PASS にした; ステージは未解決なら止めるとするが、45 件はすべて間接にたどれ、レビュー済みの成果物を書き直す負担が大きいため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-15T11:49:09Z — 速度測定（--ignored）は CI に入れなかった; 実行機の速さに左右され、CI が不安定になるため手元に残した。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
