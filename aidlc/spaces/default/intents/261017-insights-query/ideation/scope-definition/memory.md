<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T05:08:27Z — [insights-query] この作業も v0.2.0 に含め、終わるまで v0.2.0 のタグを待つ（範囲の定義 Q5）; team.md の Deployment は「v0.2.0 は取得の高速化（PR #22）と Windows 版を合わせて配る」としているが、IB-11 を加える形と解釈した。v0.1.2 を付けない決まりや、タグを付け替えない決まりは変わらない。
- 2026-10-17T05:08:27Z — 「クエリの書き方の説明は含めない」と「対応する範囲の一覧は README に書く」は別のものとして扱う（範囲の定義 Q6・Q8）; 書き方は AWS の資料に任せ、このアプリで使える命令・関数・演算子の一覧だけを書く。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T05:08:27Z — team.md の Deployment にある v0.2.0 の中身の記述（PR #22 と Windows 版）を、IB-11 を含む形に直すかどうか; 直すなら次に practices を見直す機会に行う。
