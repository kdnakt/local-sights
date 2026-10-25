<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-10-25T12:40:00Z — 依頼文の「count(*)、sum、avg、min、max、bin() など」の「など」は、利用者の答え（Q1 C）でコンソールの stats の集計関数すべてと読んだ。関数の確定した一覧は AWS の資料から Functional Design で起こす。
- 2026-10-25T12:40:00Z — 空のビンはコンソールと同じく出さない（Q2 A）。stats のあとの sort・limit もコンソールと同じ（Q3 A）。細部（by の鍵が欠けた行、文字列の sum/avg、集計値の型）は前の作業の決まりどおり「コンソールと同じ」と書き、Functional Design で確かめる。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
- 2026-10-25T12:40:00Z — 前の作業の FR2.6（中断したら評価しない）を、利用者の答え（Q6 A）で「止めた時点までの途中結果を『一部のログだけの結果（中断）』の印つきで残す」に改めた。
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-10-25T12:40:00Z — 途中結果はどのクエリにも出す（Q5 A）。sort・limit・stats を含むクエリは届いた分で作り直して差し替えるため、画面の並び順や集計値が実行中に変わることを受け入れた。
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
