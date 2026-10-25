<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-10-25T11:40:00Z — 既存の知識ベース（2026-10-20、全面）は言語切替・書き出し・JSON 整形の 3 コミットで STALE になった。利用者は焦点を絞った読み直しを選び、深く読む範囲は中核 src・Tauri src・画面 src・マニフェスト・docs に限った（tests/examples/scripts/.github は流し読み）。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-10-25T12:12:00Z — 焦点を絞った読み直しのため、前回の全面の深い読み（37 パス、取得・キャッシュ・SSO・画面の部品など）は再確認できず浅い読みに格下げになった（compare は NARROWER）。記述そのものは保たれており、今回の作業（クエリの評価器・実行・結果の表・書き出し）に関わる範囲は深く読み直した。
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-10-25T12:12:00Z — 開発者の発見：stats はパーサで拒まれ命令一覧が 4〜5 か所に分散、結果の行は「1 行 = 1 イベント」で固定（集計行の型がない）、評価は取得完了後に 1 回だけで途中結果の経路がない（Log Management の fetch-progress → get_rows が手本）。要件と部品の設計で扱う。
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
