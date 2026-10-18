<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] @log の値はロググループ名だけとした; コンソールの「アカウント ID:ロググループ名」にするには読み取りの 3 つ以外の API が要り、Forbidden で呼べないため。@log の値を比べるクエリは互換の確かめの組に入れない。
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] 言葉の大文字・小文字、優先順位（not > and > or）、文字列の項目と数値の比較、sort がないときの並び順は、AWS の資料に書かれていないため U1 では仮の決まりにした; U2 の機能設計（OQ2）と U4 でコンソールに合わせる。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] U1 では取得の段に入るときに前の結果を消す（interaction-spec は前の結果を残す）; QueryStore の最小版が実行のたびに捨てて取り直すため、前の結果の行が指す先がなくなる。U3 で interaction-spec のとおりに戻す。
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] 作業単位の分割のレビューの指摘 R-01〜R-04 と Q8 に合わせて、上流の unit-of-work・story-map・dependency・traceability を直し、Q2a・Q5・Q6 の答えを要件の FR5.6 として書き足した; 決まった項目の取り出し・取得中の表示と中止・接続を変えるときの確認を U1 に移した。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] U1 の取得は今の run_fetch_with_cache を、行き先に QueryStore を渡して使う; 取得の流れを変えずに済み、期間全体のキャッシュの利用と書き込みも今のとおりになる。範囲の一部の利用と取り足しは U3。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-18T01:30:00Z — [u1-filter-skeleton] 評価の区切りの件数（10,000 件）は、作る段階で NFR1・NFR2 を測って決め直す。
