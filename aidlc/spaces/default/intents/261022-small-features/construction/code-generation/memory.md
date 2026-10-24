<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-10-24T11:56:43Z [unit u1-language-switch] — Cloudscape の `I18nProvider` の `locale`/`messages` を描画中に差し替えても開いている部品は崩れなかったため、`key={locale}` による作り直しはどの部品にも入れなかった（functional-spec の仮定が成立）。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
- 2026-10-24T11:56:43Z [unit u1-language-switch] — 計画の手順を局所的に入れ替えた。`save_settings` の引数が 3 つになると `session.rs` が旧シグネチャで呼んでいてクレートがコンパイルできないため、`session.rs` の実装（Step 4.1）を Step 3.3 の前に行った。純粋なロジックはテスト先行、境界は実装後にテストという順序は保った。
- 2026-10-24T11:56:43Z [unit u1-language-switch] — `App.tsx` の `reportError` を「鍵と引数で持ち、描くときに翻訳する」形に直した（計画に明記なし）。`t` に依存したままだと言語の切り替えでイベント購読の `useEffect` が再実行され `getSession` が 2 回呼ばれていたため。表示中のエラー文も言語に追従するようになった。
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
