<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-17T09:35:00Z — 古いコード知識ベース（parallel-fetch のとき）は Windows 対応などの後で古くなっていたため、利用者の選んだ全面の読み直しで 9 つの文書を作り直した; 前回の文書は向きを知るためだけに読ませ、中身はこの回の走査から組み立てた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-17T09:35:00Z — 一部のモジュール（fetcher・pacing・schedule・sso_cli など）は説明と公開項目の一覧だけを見て、浅い記録（shallow.paths）に残した; この作業（クエリ）に関わらないため。全面の読み直しの扱い（kind: full）は変えていない。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-17T09:35:00Z — 走査の中では、テスト・ビルド・lint を実行しなかった; 文書づくりに必要な情報は設定とテストのコードから読めるため。テスト数や速さのテストの有無は、コードを数えた値として記録した。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-17T09:35:00Z — 複数のロググループを扱うときのストリーム名の衝突と、取り足しで sequence が振り直される件は、要件分析と設計で扱いを決める。README の「streams are read one after another」は今の同時取得と食い違っており、B6 で README を直すときに合わせて直すか決める。
