<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] 終了日時はその秒の 999 ミリ秒まで含むと解釈した（FR3.5）; 画面の入力が秒単位のため、終了の秒に記録されたログを取りこぼさない。
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] ページングの終わりは「送ったトークンと同じトークンが返る」か「次のトークンがない」で判定し、最後の応答もページ数に数えた（BR3.2）。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] unit-of-work.md では EventTimeline の主な単位は U3 だが、U1 で最小版（取得順の追加・全件の読み出し・破棄）を置いた; LogEvent の持ち主を components.md のとおり EventTimeline に保つため（レビュー R-01、R-02、R-08）。承認済みの上流ファイルは書き換えていない。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] 途中でエラーが起きても取得できたページは残して表示する（Q3）; 再試行は U3 まで入らないが、薄い一本の確認で取得結果を失わないことを優先した。
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] U1 では件数の上限を設けず全件を取得・表示する（Q4）; 大量件数での性能は U3 以降に確かめる。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] プロファイルに既定のリージョンがない場合は RegionMissing を返す（Q1）。リージョンの選択画面は U2 で扱うかを U2 の機能設計で確かめる。
