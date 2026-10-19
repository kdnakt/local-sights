<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T08:06:40Z — Q1 で A（互換の確かめ方が未決）が選ばれなかったので、受け入れないと決めつけず追加の質問（Q1a）で確かめた; 回答は「この段で決める」で、Q4〜Q6 で確かめ方を決めた。複数選択で選ばれなかった項目は、受け入れたとも受け入れないともみなさない。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T08:06:40Z — 関係者の合意・予算・体制・市場調査・画面設計の質問を省いた; この計画では該当の段を行っておらず、関係者は開発者本人だけのため（stakeholder-map）。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T08:06:40Z — 互換を確かめる比べ方は、目視でも自動テストでもなく、手元の確認用プログラムで自動で比べる形を選んだ（Q6）; 自動テストと CI から AWS に接続しない決まりを守りつつ、目視より確かに比べられるため。コンソールで書き出した結果はログの中身を含むので、リポジトリには置かない前提とした。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T08:06:40Z — AWS の資料のクエリの例は、特定のログの形（Lambda や VPC フローログなど）を前提にするものが多い。開発者の手元にあるロググループで使える例をどう選ぶか、見本のログをどう用意するかは要件分析で決める。
