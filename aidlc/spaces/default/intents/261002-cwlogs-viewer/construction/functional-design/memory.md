<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] 終了日時はその秒の 999 ミリ秒まで含むと解釈した（FR3.5）; 画面の入力が秒単位のため、終了の秒に記録されたログを取りこぼさない。
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] ページングの終わりは「送ったトークンと同じトークンが返る」か「次のトークンがない」で判定し、最後の応答もページ数に数えた（BR3.2）。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] unit-of-work.md では EventTimeline の主な単位は U3 だが、U1 で最小版（取得順の追加・全件の読み出し・破棄）を置いた; LogEvent の持ち主を components.md のとおり EventTimeline に保つため（レビュー R-01、R-02、R-08）。承認済みの上流ファイルは書き換えていない。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] 途中でエラーが起きても取得できたページは残して表示する（Q3）; 再試行は U3 まで入らないが、薄い一本の確認で取得結果を失わないことを優先した。
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] U1 では件数の上限を設けず全件を取得・表示する（Q4）; 大量件数での性能は U3 以降に確かめる。
- 2026-10-04T12:30:00Z — [u2-connection-selection] 接続を変えるとき、表示中のログがあるときだけ確認ダイアログを出す（Q6 の利用者の追加要望）; 誤操作でログを失うのを防ぐ代わりに、操作が 1 手増える。
- 2026-10-04T12:30:00Z — [u2-connection-selection] 起動時は何も選ばず、前回の選択も覚えない（Q2）; 設定ファイルを保存しない単純さを取り、起動のたびに選ぶ手間を受け入れた。
- 2026-10-04T15:00:00Z — [u3-fetch-robustness] ストリームは 1 つずつ順に取得し、再試行は要求ごとに最大 5 回、使い切りが 3 ストリーム続いたら残りを失敗とする（Q1、レビュー R-03）; 並列化しない単純さとクォータの安全を取り、取得時間の長さを受け入れた。
- 2026-10-04T15:00:00Z — [u3-fetch-robustness] 列挙は「開始 − 1 時間」より古いストリームで打ち切る（Q2）; API の呼び出しを減らす代わりに、最後のイベント時刻の更新遅れが 1 時間を超えると取りこぼしうる。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-04T06:19:17Z — [u1-walking-skeleton] プロファイルに既定のリージョンがない場合は RegionMissing を返す（Q1）。リージョンの選択画面は U2 で扱うかを U2 の機能設計で確かめる。
- 2026-10-04T12:30:00Z — [u2-connection-selection] 再レビュー（上限の 2 回目）の R-11（確認用プログラムのプロファイル名の引数から ConnectionProfile を作る方法）と R-12（BR1.3 の知らせの内容の書き方の食い違い）は、project.md の「レビュー上限後の指摘は次に回す」に従い機能設計では直さない。U2 のコード生成の計画で扱い方を決める（R-11：引数なし＝SdkDefault、引数あり＝その名前の Named。R-12：知らせにはファイルの種類だけを出す）。
- 2026-10-04T12:30:00Z — [u2-connection-selection] U1 から持ち越した疑問（リージョンの選択画面）は U2 で解決した（Q3：未選択でうながす）。
- 2026-10-04T15:00:00Z — [u3-fetch-robustness] 再レビュー（上限の 2 回目）の R-09（FetchJob に listingStatus がない。上限で呼ばずに失敗としたストリームを finishedStreamCount と StreamFetchOutcome に数えるか）、R-10（BR4.4 の二分探索には (logStreamName, sequence) から timestamp を引く索引が要る）、R-11（UC8 の並び順）は、project.md の「レビュー上限後の指摘は次に回す」に従い機能設計では直さず、U3 のコード生成の計画で扱う。
- 2026-10-04T15:00:00Z — [u3-fetch-robustness] 時刻を持たないストリームが最後のイベント時刻の順でどこに並ぶか（BR1.2 の仮定）は API の文書で確かめられない。開発者本人が実際の AWS で確かめる。
