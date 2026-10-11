<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-10T12:57:56Z — Q4 で利用者が「時刻が空なら自動で補う」を選んだため、要件の前提 AS1（不正扱い）は画面案で置き換えた；承認済みの requirements.md は編集せず、mockups.md の冒頭の表に置き換えを明記した。
- 2026-10-10T12:57:56Z — コアとのやり取りは「開始・終了それぞれ 1 つの文字列」を変えず、画面側で日付と時刻を結合・分割する方針にした；コアの変更を書式の置き換えだけに抑え、Rust 側のテストの影響を小さくするため。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-10T12:57:56Z — 前回ワークフローの wireframes と user-flow は今回の記録に存在しないため読まず、添付スクリーンショットと既存コードから画面案を起こした；stage の「存在しない成果物の内容を作らない」に従った。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-10T12:57:56Z — Q3 で利用者は推奨（自前の仮想スクロール）ではなく「Cloudscape の表で測ってから判断」を選んだ；画面案は両方で同じ見た目になるよう描き、判断の基準と戻し先を interaction-spec の §4 に書いた。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-10T12:57:56Z — Cloudscape の DatePicker が `yyyy/mm/dd` の入力マスクと ja ロケールの組み合わせで期待どおり動くかは、実装の最初に確かめる。
