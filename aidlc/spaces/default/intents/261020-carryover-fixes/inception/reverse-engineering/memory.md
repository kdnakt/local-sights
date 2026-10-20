<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-20T08:29:17Z — Run Record の Commit は走査時の HEAD（記録だけを足した `8bb3759`）ではなく、コードが同じ `main` の `3208713` とした；知識ベースはコードの状態を指すものなので、記録の追加で変わらない方の印を残す。
- 2026-10-20T07:58:47Z — 知識ベースが STALE（IB-11 の実装と依存関係の更新で `./` が変わった）だったため、開発者の選択で全面の読み直しにした；前回（insights-query）の 9 文書は向きをつかむためだけに読み、成果物はこの回のコードから書き直す。協力者（architect）は off なので、開発者 1 人が走査と合成の両方を行う。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-20T08:29:17Z — 依頼 (3) の「`getSession` の番号付けの古い説明」はコードに該当の文言がなく、parallel-fetch U4 のレビュー R-01（`src/elapsed.ts` の `nextAnchor` が取得番号を等しいかどうかだけで比べる点の注記が未記載）を指すと読んだ；要件分析で開発者に確かめる。
- 2026-10-20T08:29:17Z — 依頼 (4) の権限のテストの固定値は 26 ではなく今は 32（`src-tauri/src/lib.rs` のコメントに 26→30→32 の経緯）；直し方（固定値をやめて登録の一覧と突き合わせるか）を要件分析で決める。
- 2026-10-20T08:29:17Z — 依頼 (5) のリージョン名は、コードのどこにも名前の表がない（`catalog/regions.rs` はコード 34 個だけ）；読み取り 3 API 以外は呼べないため静的な日英の表を新しく持つことになる。表示言語と出どころを要件分析で決める。
