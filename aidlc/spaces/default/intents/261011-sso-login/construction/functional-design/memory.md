<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-12T01:20:00Z — FR3.3 の「CLI が出した失敗の理由を出す」を、終了コードと代わりの手順だけを出す形に変えた；画面に出す CLI の出力を許可リスト（URL と確認コード）に限ったため、自由な文章の理由は出せない。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-12T01:20:00Z — ログイン成功後は、SDK がトークンを読み直すかに頼らず、キャッシュしている SDK のクライアントを全部捨てる形にした；プロファイルごとに選んで捨てるより単純で、再作成の費用は小さい。
- 2026-10-12T01:20:00Z — FD Q2 で AssumeRole のプロファイルも対象になったため、設定ファイルから source_profile の名前も残すことにした；名前は秘密情報ではなく、SSO の値（開始 URL など）は読まずに捨てる方針は保った。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
