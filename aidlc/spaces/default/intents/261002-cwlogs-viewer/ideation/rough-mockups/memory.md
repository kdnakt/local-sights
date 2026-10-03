<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-03T02:51:25Z — 進捗表示は MVP 対象外だが、取得中であることを示す「Fetching」表示とボタン無効化は必要と判断した; 進捗率や中断ではなく、システム状態の可視化の最低限として wireframes に含めた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-03T02:51:25Z — デスクトップ GUI（macOS・Windows）を 1 週間で作るため、見た目は標準部品のみ・作り込みなしとした; 利用者の選択（Q7）。期限超過のリスクは承認・引き継ぎで再確認する。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
