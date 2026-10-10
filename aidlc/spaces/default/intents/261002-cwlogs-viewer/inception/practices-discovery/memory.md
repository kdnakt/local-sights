<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-03T05:13:13Z — Q10 の認証情報ルールは、利用者の回答「どこまでかわからない」を受けて秘密情報と識別子に分けて追加で確認した（F1）; アクセスキー ID は識別子に含めずログ出力も禁止とし、プロファイル名・ロール ARN・アカウント ID はログ・エラー表示に出してよいとした。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-03T05:13:13Z — Q1 の当初回答「main に直接 push」を、組織の既定（短命ブランチ→PR→スクワッシュマージ）との矛盾として追加質問（F2）で確認し、既定に合わせた; 組織の既定に矛盾するチームルールは登録時に拒否されるため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-03T05:13:13Z — テストは全面 TDD ではなく、純粋ロジックだけテスト先行・AWS 接続と GUI は実装後（custom）を選んだ; 期限 1 週間の中で、壊れやすいロジック（時刻順マージ・ページ終端判定など）の品質を優先するため。数値のカバレッジ下限は設けず、GUI 非依存層の公開関数にはテスト必須とした。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-03T05:13:13Z — GUI フレームワークが未選定のため、Code Style（Tauri なら JS/TS 側のツール）と非同期 SDK と GUI のつなぎ方は技術選定後に見直しが要る。スコープ local-tool は skeleton を宣言していないため、骨組みのチェックポイントの扱いを Construction 開始時に確認する。
