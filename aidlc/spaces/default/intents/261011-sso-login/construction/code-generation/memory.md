<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-12T03:04:16Z — CLI が自分で正常に終わったときはプロセスグループを止めない；CLI が開いたブラウザまで閉じてしまわないため。止めるのはキャンセル・時間切れ・アプリ終了のときだけにした。
- 2026-10-12T03:04:16Z — 成功の知らせの位置「右ペインの上」を、条件エリアの下・ログ一覧の上と解釈した；Cloudscape のレイアウトでは右ペインの最上部が条件エリアのため。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-12T03:04:16Z — 計画は Mac のパス（/Users/kdnakt/.nvm の node）を前提にしていたが、Linux のコンテナで PATH 上の Node v22 を使い、GTK／WebKit の開発ライブラリを入れて検査した；単位テストの手順に書くコマンドは、環境に依存しない形（PATH 上の node）にしておくほうがよい。
- 2026-10-12T03:04:16Z — 文言表にない i18n key を 4 つ足した（ssoLogin.failedToStart、ssoLogin.openFailed、session.ssoLoginInProgress、session.ssoLoginUnavailable）；起動失敗・URL を開けない・ログイン中の拒否の表示に必要だったため。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-12T03:04:16Z — CI は cargo test -p local-sights を実行しないので、sso_cli のテストは CI で動かない；CI に足すか（macOS ランナーか、Linux で GTK を入れるか）を決める。
- 2026-10-12T03:04:16Z — Dock の「終了」で RunEvent::Exit が届き CLI が止まるかは Mac の実機で確かめる必要がある。
