<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-10T05:03:06Z — 品質目標の棚卸しの出所に `inception/requirements-analysis/requirements.md` の NFR 表を加えた; このスコープでは nfr-requirements・nfr-design のステージを実行しておらず、ステージの手順が挙げる出所は Testing Contract だけになる。要件の NFR 表がこのプロジェクトの実際の測れる目標なので、そこから目標 ID（NFR1〜NFR16）を取った。
- 2026-10-10T05:03:06Z — Test Strategy は Standard だが手順書は 4 種類（ビルド・統合・性能・セキュリティ）とも作った; ステージの成果物の一覧に 4 つとも挙がっており、要件に性能（NFR1〜NFR3）とセキュリティ（NFR5〜NFR8）の目標があるため、手順書を分けておく方が手元の確認に使いやすい。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-10T05:03:06Z — Tauri アプリの組み立て（`cargo build -p local-sights`）とワークスペース全体の clippy はこの環境で実行できず、「未確認」として記録した; この環境（Linux のコンテナ）に WebKitGTK・GDK がなく、apt でも入れられない。利用者の判断（Q1）で手順書だけを書き、手元の確認は後に回した。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-10T05:03:06Z — 速さを測る `#[ignore]` のテストはこの環境のリリースビルドで実行し参考値として記録した; 要件の計測機は開発者の Mac で、ここでの値では達成の判定をしない。実行しない選択肢もあったが、桁が合うかを早めに知る価値を取った（Q2）。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-10T05:03:06Z — NFR7（外部に送らない）・NFR9（macOS で動く）・NFR14（テストしやすい構成）はどの単位の traceability.json にも対応がなく、FR7.8 は U6 で N/A（選択肢であり機能のルールではない）; このステージで静的に確かめた結果を cross-unit-traceability.md に書いた。traceability.json への追記をどのステージで行うかは承認の場で確認する。
