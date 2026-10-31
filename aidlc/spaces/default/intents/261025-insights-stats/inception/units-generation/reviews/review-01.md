### Review — units-generation (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-26T00:15:03Z
**Iteration:** 1

### Summary
YAML の辺（宣言済みの 4 単位、循環なし、kind は許可値）、story map と traceability の対応（44 個の ID すべてが 1 つの Unit に割り当てられ、対象の Unit ID が story map の行と一致）、Q1〜Q3 の回答（A/A/A）への沿い方は整合していて、Critical は見つからなかった。一方、U1 を「画面から AWS まで通る薄い一本」として成り立たせるには結果の型の変更が波及する部品（QueryStore・QueryRunner・TauriSink・CompatCheck）が U1 の「含む部品」から漏れており、機能設計で手戻りになる。そのほか、DD レビューの持ち越し事項の対応先、U2/U3 の重なりの書き方、レビュー ID の曖昧さを Major/Minor として次の段へ持ち越す。助言のみの 1 回の確認のため READY とする。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | unit-of-work.md > U1「含む部品」「責務」、unit-of-work-dependency.md > 作業単位の間のつなぎ目 | U1 は ADR-001 で QueryResult の行を列挙型（イベントの行／集計の行）に変えるが、この型を使う QueryStore（`evaluate_step`・`copy_rows` が `Evaluation`／`ResultRow` を使う。DD レビュー R-09）、QueryRunner（`copy_rows`・`on_evaluated` で結果を QuerySink に渡す）、TauriSink（結果の写し先）、CompatCheck（U1 の責務に「型だけ合わせる」とある）が U1 の「含む部品」にない。QueryStore と QueryRunner は U3 にしか現れず、U3 が U1 に依存するため、U1 の時点では型が合わずビルドが通らない／画面から AWS まで通る薄い一本が成立しない。さらに、QueryRunner を通さないと「実際の AWS から取得して集計の結果を出す」確認も GUI の実行もできない。 | U1 の「含む部品」と責務に、結果の型の変更に追従するだけの最小の変更として QueryStore・QueryRunner・TauriSink・CompatCheck（型合わせのみ）を明記し、U3 が足す部分（途中の view・版・中断後の評価）との境目を書く。つなぎ目の表にも「結果の型を使う側の追従は U1」と足す。 | New |
| R-02 | Major | unit-of-work.md > U1「責務」(QueryParser)、unit-of-work-dependency.md > つなぎ目の表「命令・関数の一覧（QueryCatalog）」、components.md > QueryParser の depends_on | components.md では QueryParser が QueryCatalog に依存するが、U1 は QueryParser を含み QueryCatalog を含まない（QueryCatalog は U2 が作る）。つなぎ目の表は「U1 は今の `OTHER_COMMANDS`／`function()` に 5 つの関数を足す」と補っているが、U1 の責務や U2 の「実装の注意」にこの暫定の持ち方を U2 でどう置き換えるか（U2 で QueryParser の振り分けを作り直す）が書かれていない。U2 の機能設計が、U1 で入れた暫定の表の扱い（残すか捨てるか）を決められない。 | U1 の責務に「QueryCatalog なしの暫定の振り分け」であることを明記し、U2 の責務に「U1 の暫定の表を QueryCatalog に置き換える」を足す。 | New |
| R-03 | Major | unit-of-work.md > U2「含む部品」と「実装の注意」、unit-of-work-dependency.md > 並べて進められる組み合わせ | 「U2 と U3 は並べて進められる（Q3 A）」の根拠は「U2 は列・関数・一覧、U3 は版・途中の view・印を触る」だが、次の点がこの主張と合わない。(1) U2 の「実装の注意」は QueryResultTable が U3 と重なるとするのに、U2 の「含む部品」に QueryResultTable がない。(2) 重なる部品に InsightsSession・TauriApp・ApiBridge の `SessionView`／コマンド（U2 は一覧のコマンド、U3 は版つきの見出し・イベント）があり、同じ型・同じファイルを両方が変える。(3) U3 の FR4.3「`sort`・`limit`・`stats` を含むクエリの差し替え」は `stats` のあとの `sort`／`limit`（U2）を含むため、U3 の確かめ方にその組み合わせを入れるなら U2 が先に必要になる。どの条件でどれが先に要るかが書かれていない。 | 重なる部品ごとに、U2 と U3 が触る関数・型・コマンドを表にして書く（特に SessionView と TauriApp のコマンドの分担）。U2 の「含む部品」に QueryResultTable を足すか、触らない理由を書く。U3 の確かめ方は U1 だけで足りるクエリ（`stats` のあとに命令がないもの）に限るか、`stats ... | sort` の確認は U2 の後と書く（作る順の指定ではなく依存の条件として）。 | New |
| R-04 | Major | unit-of-work.md > U3「実装の注意」、U1「実装の注意」、U2「実装の注意」（DD レビューの持ち越し事項） | DD レビュー R-01〜R-10 を、どの作業単位の機能設計が解くかの対応が一部しか書かれていない。U1 の「(R-05)」「(R-04)」は RA レビューの番号（引き数と鍵の式、0 件の `by` なし）だが、DD レビューにも R-04（中断後の評価）・R-05（タイムゾーンと表示の文字）があり、ID が衝突して読み分けられない。DD の R-05（セルを型つきで持ち、時刻だけ表示のときに整形）は U1 の責務（StatsAggregator・InsightsSession）に実質含まれるが、R-ID で対応づけられておらず、R-01（行は写し）、R-06（references の owner）、R-09（QueryStore↔QueryEvaluator の依存）もどの単位の機能設計かが書かれていない。 | 「RA R-xx」「DD R-xx」と接頭辞をそろえ、DD R-01〜R-10 と RA の持ち越しを 1 つの表（持ち越し ID → 解く単位）にまとめる。R-01（行は写し）・R-05（タイムゾーン）・R-09 は U1、R-02・R-03・R-04・R-08 は U3、R-10 は U2 と明示する。 | New |
| R-05 | Minor | unit-of-work.md > 一覧の表と U1「責務」、unit-of-work-story-map.md > FR1.2 | 「5 つの関数」と書きながら `count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max` の 6 つを並べている（質問票 Q2 A の文言もそう）。`count(*)` と `count(項目)` を 1 つと数えているのか 6 つなのかが U1 の範囲の境目として曖昧。 | 「5 つ（count は 2 つの書き方）」か「6 つ」かに文言をそろえ、U1 のテストの数え方に合わせる。 | New |
| R-06 | Minor | unit-of-work.md > U1「確かめ方」、U4「含む部品」 | U1 の確かめ方は「今の FetchCheck を広げるか、`compat_check` の評価の部分を使う」とあるが、確認用プログラムは U1 の「含む部品」に載っておらず、`compat_check` は U4 の部品である。U1 のウォーキングスケルトンの確認が U4 の成果物に依存するように読める（U4 は U2 に依存するため、U1 の確認が U4 を待つと矛盾する）。 | U1 では FetchCheck（または U1 で最小限広げた確認用プログラム）を使うと決め、`compat_check` の `stats` の比較は U4 とはっきり分ける。 | New |
| R-07 | Minor | unit-of-work.md > U1・U2 の確かめ方、unit-of-work-story-map.md > 複数にまたがる要件「NFR1・NFR2」 | NFR1・NFR2 の「集計の速さは U1・U2 で測る」とあるが、U1・U2 の確かめ方に集計の速さの測定がなく、測る条件・数値の決め方（OQ3、project.md の「クエリの速さの合否の数値は作る段階で決める」）がどの単位かが宙に浮く。 | U1 または U2 の確かめ方に集計の速さの測定（10,000 行ずつ進める流れでの所要時間）を 1 行足すか、測る単位を U3 に一本化して理由を書く。 | New |
| R-08 | Minor | unit-of-work-story-map.md > 作業単位の中の作る順（目安） | 単位の中の作る順は、作業単位をまたぐ順序の推奨ではないが、「どれから作るかは作る段階の計画で決める」という本文の方針と並ぶと、段階の計画の領域に踏み込んだ書き方に見える（例：U1 の中で「テスト先行」を含む順序の指定）。stage の契約は単位間の作る順と最短経路を示さないことを求めるので、単位内の目安であることを強調しておくのが安全。 | 見出しを「単位の中の主な流れ（目安。単位どうしの順ではない）」のように改め、作る段階の計画が変えてよいと書く。 | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
