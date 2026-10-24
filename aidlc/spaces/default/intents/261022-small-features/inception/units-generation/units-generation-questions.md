# Units Generation — 質問

元にしたもの：部品の一覧（`inception/domain-design/components.md`：新規 5・変更 9）、設計判断（`decisions.md` ADR-001〜006）、要件（`inception/requirements-analysis/requirements.md` FR1〜FR5）。3 つの機能は互いに独立で、どの作業単位も今の 1 つのデスクトップアプリに足す（作業単位ごとに別々に配ることはない。team.md Deployment）。ここでは「どう切るか」と「最初の一本をどれにするか」を決めます。どれから作るかの経済的な順番は、作る段階の計画で決めます。

## Q1. 作業単位の分け方は、どれにしますか？

部品の一覧では、言語切替（AppSettings・AppSession・MessageCatalog・AppShell・SettingsDialog）、書き出し（ExportFormatter・LogExporter・ExportBoundary・AppSession・InsightsSession・TauriApp・ApiBridge・ExportMenu）、JSON の整形（JsonPrettyPrinter・ExpandedMessage）が、ほぼ重ならない 3 つの群になっています。重なるのは AppSession（言語と書き出し）と TauriApp・ApiBridge（書き出しと設定コマンド）だけです。

- A. 機能ごとに 3 つ：U1 言語切替／U2 書き出し（保存・コピー・CSV・Markdown・両画面）／U3 JSON の整形（おすすめ：機能どうしの依存がなく、1 つずつ PR にしやすい）
- B. 4 つ：書き出しを「ログ画面の CSV 保存まで（薄い一本）」と「Markdown・コピー・クエリ画面」の 2 つに割り、言語切替と JSON の整形を 1 つずつ
- C. 2 つ：書き出しを 1 つ、言語切替と JSON の整形を合わせて 1 つ（どちらも画面側の変更が主）
- D. 1 つ：3 つの機能をまとめて 1 つの作業単位にする
- X. Other (please specify)

[Answer]: D

## Q2. 最初の作業単位（薄い一本）は、どれにしますか？

チームの決まり（team.md Walking Skeleton）では、最初の作業単位で「新しく加える振る舞いが画面から端まで通る最小の版」を作って動くことを確かめます。この作業では AWS を新しく呼ばないため、「端」は設定ファイル・保存ダイアログ・クリップボードです。

- A. 言語切替を最初に作る（おすすめ：いちばん小さく、設定ファイル → 中核 → 画面の経路を一通り通る。しかも先に入ると、残りの機能の英語の画面を起動し直さずに確かめられる）
- B. 書き出しを最初に作る（いちばん大きく、新しい依存（公式プラグイン）と OS の境界を早めに通してつなぎ込みの不安を潰す）
- C. JSON の整形を最初に作る（画面側だけで最小。ただし新しい経路を通らない）
- D. Not yet defined（作る段階の計画で決める）
- X. Other (please specify)

[Answer]: A

## Q3. 「1 つの作業単位」と「言語切替を最初の一本に」のすり合わせ

Q1 で「3 つをまとめて 1 つの作業単位」、Q2 で「言語切替を最初の一本に」を選びました。1 つの作業単位の中には「最初の作業単位」がないため、どちらの読みかを確かめます。

- A. 作業単位は 1 つ。その中の作る順番として言語切替を最初に実装し、動くことを確かめてから書き出しと JSON の整形に進む（途中の人間の確認は 1 つの作業単位の中の節目として行い、別の作業単位にはしない）
- B. 作業単位は 2 つ：U1 言語切替（薄い一本として独立）、U2 書き出しと JSON の整形
- C. やはり 3 つ（U1 言語切替／U2 書き出し／U3 JSON の整形）
- X. Other (please specify)

[Answer]: B
