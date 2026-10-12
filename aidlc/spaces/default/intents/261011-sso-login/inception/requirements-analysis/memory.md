<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-12T00:40:00Z — RA Q2 で利用者に「おすすめは？」と聞かれ、エラーのところだけ（A）を勧めた；起動時に一覧を自動で読むので、切れていれば必ず左ペインにエラーと入口が出るため、上部バーの入口は効果が薄いと判断した。
- 2026-10-12T00:40:00Z — RA Q6 で確認コードと URL を画面に出すことになった；確認コードはトークンではないため禁止ルール（秘密情報を画面に出さない）には当たらないと解釈したが、ログやキャッシュには書かない要件（NFR3）を足した。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
