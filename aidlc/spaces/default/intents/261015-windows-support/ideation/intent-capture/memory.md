<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-15T13:00:00Z — Q3 の「macOS 版と同じ」は、今の macOS 版にある機能（取得・表示・絞り込み・キャッシュ・SSO ログイン・中断・経過時間）が Windows でも同じように使えることと解釈した。見た目の細部（メニューの位置など）は OS の慣習に合わせてよいかは、要件分析で決める
- 2026-10-15T13:00:00Z — 「なぜ今か」は単独の質問にせず、依頼文（IB-11 の前にやる）と Q1（会社の PC で使いたい）から書いた

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-15T13:00:00Z — Q1（会社の PC で使う）と Q6（署名しない）が食い違う恐れがあったため、追加の Q8 で確かめ、未署名のアプリを入れてよいと確認した
