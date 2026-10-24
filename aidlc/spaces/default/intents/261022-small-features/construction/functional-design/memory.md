<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations

- 2026-10-24T10:24:44Z [unit u1-language-switch] — U1 の機能設計は質問を設けなかった。残る細部（値の綴り、formatVersion、選択肢の表示名、最初の描画、起動中の OS の言語の変化）は利用者の世界の事実ではなく設計の判断で決められるため、rules.md の「決めたこと」に書いた。
- 2026-10-24T10:24:44Z [unit u1-language-switch] — 選択肢の表示名は選択肢の言語で固定（「日本語」「English」）とし、「OS の設定に合わせる」だけを訳すことにした。読めない言語に切り替えても戻れるようにするため。
- 2026-10-24T10:24:44Z [unit u1-language-switch] — formatVersion は 1 のままにして language を省略可能な項目として足すことにした。前の版のファイルも、前の版のアプリも読めるため。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
