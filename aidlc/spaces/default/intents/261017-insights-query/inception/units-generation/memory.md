<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T13:21:55Z — この計画は refactor を元にしており骨組みのチェックポイントはないが、team.md の Walking Skeleton にならって U1 を画面から AWS まで通る薄い一本にした; 確かめ方も team.md の 3 つにそろえた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T13:21:55Z — 確認用プログラムは利用者の答え（言語がそろったあとに独立）に合わせて 4 つ目の作業単位 U4 にした; 選んだ 3 つの案には置き場所がなかったため、追加の質問で確かめた。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T13:21:55Z — U1 は今の取得の流れ（捨てて取り直す）を使うため、U3 までの間はクエリの画面で取得すると Log Management の表示中のログも入れ替わる; 最初の一本を薄く保つ代わりに、その期間の振る舞いの違いを受け入れた。
- 2026-10-18T00:05:00Z — [insights-query] 要件の見直し（RA Q3b）に合わせ、U1 は QueryStore の最小版（1 つのロググループ、実行のたびにクエリの画面の保持だけを捨てて取り直す）で取得し、範囲の記録と取り足し・キャッシュの一部の利用・複数のロググループを U3 に残した（Q2a）; 前の版の「U3 までログ管理の画面のログが入れ替わる」暫定の動きをなくし、U3 は QueryStore と取得の段の内側を広げるだけにするため。U3 のディレクトリ名 u3-shared-store は記録の続きのため変えなかった。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T13:21:55Z — @log とロググループ名を含む同順位の決まりを U1・U2 でどう扱うか（LogEvent の変更は U3）は、U1 の機能設計で決める（レビューの Major）。
