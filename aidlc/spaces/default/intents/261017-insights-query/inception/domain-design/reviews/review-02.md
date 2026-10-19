## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-17T23:59:02Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > component QueryStore の behaviour と entity GroupHoldings（coveredRanges）、decisions.md > ADR-004 の「悪い点」 | 取得済みの範囲 coveredRanges は（プロファイル, リージョン, ロググループ）ごとの 1 組で、ストリームの次元がない。一方 behaviour は「そのストリームの取得が終わった範囲だけを取得済みに加える」と書く。一部のストリームが失敗した取得や、中止で途中までの取得では、成功したストリームのイベントと途中までのページは EventTimeline に入るのに、その範囲は取得済みに加わらない。ADR-004 は「次の実行で取り直すため重なる読み取りは起きない」と書くが、取り直すと同じストリームのイベントが再度足される。今のコードでは EventFetcher が取得のたびに順番を 0 から振り（fetcher.rs の next_sequence）、EventTimeline.append は順番が違えば同じイベントでも別のものとして足すので、重複として残る。FR2.7（同じイベントを 2 回数えない）と FR2.5・FR2.6 が同時には成り立たない。 | 取得済みの範囲の粒度を決めて部品とエンティティに書く。例：(a) ストリームごとに取得済みの範囲を持つ、(b) 範囲は全ストリームが成功したときだけ加え、失敗・中止で足したページは取り消す（その取得で足した分を外せる形にする）、(c) 失敗や中止の分は次の実行の前に捨てる。選んだ形に合わせて GroupHoldings の属性と ADR-004 の説明を直す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > component LogCache の behaviour、component QueryRunner の behaviour | LogCache の新しい照会は「要求に重なる記録範囲のイベントを返し、重なった範囲を取得済みとして知らせる」とある。記録範囲は要求の範囲より広いことがあり、QueryStore がすでに持つ範囲（取得済み）とも重なりうる。QueryRunner の流れは「足りない範囲の問い合わせ → キャッシュの重なる記録範囲を足す → 残りの取得」で、照会に渡すのが全期間なのか足りない範囲だけなのかを決めていない。全期間のまま照会すると、取得済みの範囲のイベントがもう一度足され、FR2.7 を破る。また、キャッシュの範囲を足したあとの「残り」を求め直す手順（CachePlan の再計算）が流れにない。 | LogCache の照会は QueryStore が答えた足りない範囲だけを受け取り、返すイベントとその範囲をそこに切り詰めると書く。QueryRunner の流れに、キャッシュを足した後に足りない範囲を求め直してから取得に渡す手順を足す。範囲の境界（ミリ秒の閉区間）の切り詰めも ADR-004 の境界テストの対象に含める。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > component FetchCoordinator の behaviour、Open Questions の 4 つ目 | 今の run_fetch は、1 回の呼び出しで 1 つのロググループ・1 つの期間を扱い、冒頭の discard()、FetchJob の件数、呼び出しの速さの管理（CallPacer）、中止の信号、abort() での discard を持つ（coordinator.rs）。新しい入口は（ロググループ, 範囲）の並びを 1 回の取得として扱うとあるが、並びを 1 つの FetchJob にまとめるのか、run_fetch を順に呼ぶのかが決まっていない。順に呼ぶと CallPacer と GIVE_UP の数え方が範囲ごとに作り直される。さらに abort() の discard と on_started の「捨てた直後」の意味は、QueryStore に使うと合わない（アプリを閉じるときの捨てる動作の扱いも未定）。 | 機能設計で、1 回の取得として束ねる単位（速さの管理・あきらめの数え方・中止・進み具合）を先に決め、discard を持たない版を run_fetch と共通の中身にする。QueryStore に対する on_started・abort の扱いも書く。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > entity InsightsRun の failedStreams、entity QueryResult | 失敗したストリームを持つ failedStreams にロググループの次元がない。FR2.8 は、違うロググループの同じ名前のストリームを別に扱うと決めているので、画面に失敗を示すとき、どのロググループのものか区別できない。今の FailedStream（coordinator.rs）もロググループ名を持たない。 | failedStreams をロググループ名とストリーム名の組で持つ、と属性に書く。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > component QueryStore の behaviour（順番の振り直し）、Open Questions の 1 つ目 | EventTimeline の per-stream の索引（timestamps）は順番を Vec の添字に使う密な配列で、隙間は埋め草で埋まる（timeline.rs の index）。範囲をまたいで一意に振り直すなら、ストリームごとの次の順番を QueryStore が持つ必要がある。キャッシュから足すイベントにも同じ振り直しが要るが、そこまでは書かれていない。未解決の問いとして記録済みだが、密な配列では時刻順の取り足しでも隙間が出ない形（ストリームごとの連番の払い出し）に限られることは、設計の制約として先に書いたほうがよい。 | Open Questions に、順番はストリームごとの連番で払い出し、取得の取り足しとキャッシュの追加の両方に同じ払い出しを使う、という制約を書き足す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| sensor-traceability（domain-design） | PASS（gaps・orphans・invalid の各 0） | 構造上の問題はない。 |
| 手元での YAML 確認（名前の一意性、depends_on と dependents の対称性、エンティティの持ち主、循環） | 問題なし | 18 部品の名前は一意。依存と被依存はすべて対称。エンティティの持ち主はそれぞれ 1 つ。依存の向きは非循環（AppSession と FetchCoordinator は QueryStore に向かい、QueryStore は CachePlan だけに依存する）。 |
| 共有の名残の確認 | 問題なし | LogStore の名前や、ログ管理の画面と取得済みのログを共有する記述は残っていない。LogCache のディスクキャッシュだけが共有される点は FR2.1・FR2.2 の例外として明記されている。ADR-001・004・005・007・008 と Q6〜Q9 は要件と整合している。 |

### Summary

要件の見直し（共有しない）への追従は一貫しており、カタログの形式も整っている。残る大きな懸念は、取得済みの範囲の粒度とキャッシュの照会の切り詰めで、このままでは取り足しで同じイベントが 2 回入りうる（FR2.7）。2 件とも機能設計で解決できる範囲の穴なので、承認の際に機能設計への申し送りとして確認してほしい。
