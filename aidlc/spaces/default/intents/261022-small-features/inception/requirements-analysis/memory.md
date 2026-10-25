<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations

- 2026-10-24T02:48:40Z — 書き出しの行数に上限を置かない（Q3 A）答えに合わせ、処理時間の目標値は要件に書かず、画面が固まらないこと（処理中の表示・二重実行の防止・ほかの操作に応じる）だけを NFR にした。前の作業（insights-query NFR1）と同じ扱い。
- 2026-10-24T02:48:40Z — スプレッドシートの式として解釈される値（`=` 始まりなど）の加工は、利用者に聞かずに「しない」を前提（AS3）にした。社内の利用者が自分のログを自分で開く用途で、「値はそのまま」（FR4.1.4）を優先したため。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
