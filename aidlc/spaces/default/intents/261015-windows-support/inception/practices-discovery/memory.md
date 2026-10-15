<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-15T13:45:00Z — 下書きは「v0.1.2 の次」を前提にしていたが、Q5 で取得の高速化（PR #22）と Windows 版を合わせて v0.2.0 で配ると決まったため、team.md の「次の版のタグは v0.1.2」を v0.2.0 に改める。v0.1.2 のタグは付けない

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-15T13:45:00Z — Windows 11 の Smart App Control がオンだと、SmartScreen を越えても未署名のアプリが止められる。開発者と会社の PC での設定は未確認で、要件分析で確かめる
