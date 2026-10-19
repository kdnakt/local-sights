<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T13:12:48Z — 部品の一覧には新しく作る部品と変える部品だけを載せ、変えない部品（FilterEngine など）は codekb の一覧のままとした; 前回の作業（parallel-fetch）の components.md と同じ書き方にそろえた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T13:12:48Z — 同じイベントを 2 回持たない方法（範囲を重ねない）と正規表現の重さへの備えは、利用者に聞かずに ADR として決めた; どちらも要件（FR2.7）を満たす技術上の判断で、利用者の世界の事実に依らないため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T13:12:48Z — 部品カタログの YAML は手書きせず、依存の一覧から dependents を作る小さなスクリプトで作った; 対称性と循環のなさを機械で確かめるため。
- 2026-10-17T23:45:00Z — [insights-query] 要件の見直し（RA Q3b）で共有の保持（LogStore）をやめ、クエリの画面専用の QueryStore を作り、取得は今の FetchCoordinator に「捨てずに足す」入口を足して使うことにした（Q6）; 取得の決まりを 1 か所に保ち、ログ管理の画面の LogView・EventTimeline・取得の流れを変えずに済むため。LogEvent の型も変えず、ロググループ名は QueryStore のキーから添える。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T13:12:48Z — 取得済みの範囲の新しさ（終了が今に近い期間の取り直し）と、取り足し後のキャッシュへの書き込みの単位は、機能設計で決める必要がある（レビューの Major 2 件）。
