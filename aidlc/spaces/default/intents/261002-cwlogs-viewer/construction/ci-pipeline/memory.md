<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-10T05:41:51Z — `npm audit` のゲートは high 以上だけを失敗にした; team.md は「脆弱性の検査」としか言っておらず、low・moderate の偶発的な勧告で PR を止めるより、Dependabot の更新 PR で解消する方が流れを止めない。厳しくしたいときは `--audit-level` を変えるだけ。
- 2026-10-10T05:41:51Z — ライセンスの許可リストに `Apache-2.0 WITH LLVM-exception` を足した; 質問票 Q3 の一覧にはなかったが、その式を単独で持つクレートが 1 つあり、許可しないと現状の依存で `cargo deny check` が落ちる。Apache-2.0 の例外付きなので趣旨は同じ。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-10T05:41:51Z — Tauri アプリの組み立ては `rust-core`・`frontend` が通ってから走らせた（`needs`）; macOS ランナーは分数を多く消費するため、整形やテストで落ちる PR では走らせない。並行にすれば数分早いが、費用を優先した。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-10T05:41:51Z — Action の版はメジャー版（v4・v2）で指定し、コミット SHA での固定はしなかった; SHA 固定は改ざんに強いが、Dependabot の更新 PR が増える。個人開発の MVP では読みやすさと更新の手間を優先した。SHA 固定は後から変えられる。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-10T05:41:51Z — ブランチ保護・secret scanning・Dependabot security updates はリポジトリの設定画面で手で行う必要がある; ワークフローからは設定できないため、`ci-config.md` に手順を書いた。実際に設定したかは次の作業で確かめる。
