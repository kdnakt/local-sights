### Review — domain-design (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-25T23:36:26Z
**Iteration:** 1

### Summary

部品の分け方、Q1〜Q7 の回答への沿い方、depends_on と dependents の対称性、図・表・YAML・traceability の対応は一貫していて、Critical に当たるもの（持ち主のない実体、人間の回答との矛盾、C1/C2/NFR12 の違反）は見つからなかった。一方で、実コード（`store.rs`・`runner.rs`・`insights_session.rs`・`exporter.rs`）に当てると、途中結果の作り方（保持の写し、途中の view の識別、取得との並行）、中断後の評価、集計セルの値の持ち方（タイムゾーン）、結果の行の持ち方で、機能設計の前に文言を直しておかないと手戻りになる点がある。これらを Major として次の段へ持ち越す。助言のみの 1 回の確認のため、Major と Minor は機能設計で解消する前提で READY とする。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | components.md > QueryEvaluator の `QueryResult.references`（GroupHoldings）と behaviour（行の列挙型）、decisions.md > ADR-001 | 「イベントの行＝保持の位置とロググループ名」「イベントの行は QueryStore が持つイベントを位置で指す」と書かれているが、今のコードの `ResultRow { event, log_group_name }` は `copy_rows` でイベントを複製した写しで、結果は保持に依存しない（`QueryResult` のコメント、U3:BR3.2、261020 FR2.3.1、`a_result_keeps_its_rows_after_the_next_run_discards_the_staging_events` のテスト）。位置で指す形にすると、途中結果の版を差し替える間に `settle_range` や次の途中の view 作りで位置がずれ、画面が `get_query_rows` で取りに行く行や書き出しの行が別のイベントを指しうる。 | `QueryResult` の `references` から GroupHoldings を外し、イベントの行は今と同じくイベントの写し（`ResultRow`）を持つ、と ADR-001 と behaviour の文言を直す。集計の行も写し（セルの列）であることを明記する。 | New |
| R-02 | Major | components.md > QueryRunner と QueryStore の behaviour（途中結果）、Open Questions の「保持の写しの取り方」 | 途中結果を取得と並行して作る仕組みの持ち主が決まっていない。(1) 取得中の保持（`Arc<Mutex<EventTimeline>>`）は `fetch_range` の中のローカル変数で、`take_events` したあと `settle_range` するまでの間は、どこにもない。写しを取る側が見られる場所（取得中の保持の登録先）の持ち主がない。(2) `run_query` は `&mut S` のシンクを持ったまま `fetch_missing` を順に await する 1 本の流れで、`select!` の腕の中で評価を await すると取得の future が進まず「取得を待たせない」（NFR1）を破る。評価を別タスクにするとき、シンクへの通知をどの側が行うかが決まっていない。(3) 写しの中のイベントには、確定時に `StreamSequences` が付ける順序の番号がなく、`build_view` の整列キー（時刻・ロググループ・ストリーム・番号）をそのまま使えない。また `ViewEntry` は `(group, Committed/Staging, position)` でしか指せず、写しを指す手段がない。 | 機能設計の前提として、(a) 取得中の保持の登録先と、take→settle の間の扱い（途中の view から一時的に落ちるのは許すか、二重に数えないか）の持ち主、(b) 評価を取得と並行に動かす並行モデル（別タスクと、通知の送り方）、(c) 写しのイベントの順序の付け方と、view が写しを指す方法（新しい保持の種類か、view 自身が写しを持つか）を、QueryRunner と QueryStore のどちらの責任かを分けて書く。 | New |
| R-03 | Major | components.md > QueryStore（途中の view）、QueryRunner（最終の view と途中の view） | 途中の view は「世代（epoch）を固定した一覧」とあるが、`QueryStore` は `view` と `view_epoch` の 1 つの枠しか持たず、途中の view・次の途中の view・最終の view が同じ epoch を共有する。`evaluate_step` と `copy_rows` の食い違いの検出は epoch だけなので、途中の評価が動いている間に別の view が作り直されても検出できず、ある view の位置を別の view に当てて別の行を複製しうる（最終の `build_view` が取得終了の直後や中断の直後に走る場合に特に）。「前の評価が終わるまで次を始めない」は途中どうしの重なりしか防がない。 | view に epoch とは別の識別（版）を持たせ、`evaluate_step`・`copy_rows`・`json_field_names` がその識別も照合する、または評価が view を自分で持つ、のどちらかを決める。取得の終了・中断のとき、動いている途中の評価を止めてから最終の view を作る順番を QueryRunner の責任として書く。 | New |
| R-04 | Major | components.md > QueryRunner（［Cancel］では…評価し）、InsightsSession（結果の持ち方）、decisions.md > ADR-006 | 今のコードでは、中断の合図が立つと評価は必ず止まる（`evaluate_held` が `retrier.is_stopped()` を見る、`on_evaluation_progress` が `cancel_requested` で false を返す、`finish_evaluation` が `cancel_requested` なら結果を捨てる、`finish_fetch` は中断で `Cancelled` に直行する）。FR5.1 は「止めた時点の分で 1 回評価した結果を最終結果にする」ため、これらの停止の判断をすべて逆にする必要があるが、設計は部品の責任の文言だけで、評価を止める合図と中断後の評価を分ける方法、`AnalyticsPhase` の遷移（取得中に途中結果が来る、中断後に結果を持つ終わり方）、中断後の評価の最中にもう一度［Cancel］が押されたとき、が書かれていない。InsightsSession の属性にも `cancelledPartial` があるだけで、相の変更が出ていない。 | 機能設計に回す問いとして Open Questions に明記したうえで、ADR-006 に「中断後の評価は中断の合図で止めない」「その評価の最中の［Cancel］は無効にする、または評価を打ち切って取得済みの分で出す」のどちらかを方針として書く。InsightsSession の相（取得中・評価中・完了・中断）のどこで途中結果と中断後の結果を受けるかも、責任に書き足す。 | New |
| R-05 | Major | components.md > StatsAggregator（集計値の表示の文字はここで 1 回だけ決め）、QueryEvaluator（集計の行＝セルの値の列）、traceability.json > FR2.3 | FR1.5・FR2.3 は `bin()` の列の時刻を画面で選んだタイムゾーンで表示し、切り替えたときに行数と集計値は変わらず文字だけが変わる（受け入れ基準）とする。一方、設計は StatsAggregator（純粋で、タイムゾーンを知らない）が「表示の文字を 1 回だけ決め」、表と書き出しが「同じ文字をそのまま使う」としている。評価の時点で文字にすると、タイムゾーンの切り替えで文字が変わらない。`display_rows(zone)` と `export_source(zone)` は表示のときに zone を渡して整形している今の作りとも合わない。traceability の FR2.3 も StatsAggregator と QueryResultTable だけで、時刻を整形する InsightsSession と ExportJob が入っていない。 | セルの値を型つき（数値・文字列・時刻の値など）で持ち、時刻の列だけは表示と書き出しのときに zone で整形する、数値の桁と丸めは StatsAggregator が決める、と責任を分けて書く。traceability の FR2.3 に InsightsSession・ExportJob を足す。 | New |
| R-06 | Major | components.md > QueryEvaluator の YAML `references`（QueryPlan の `owner`）と Entity Ownership の注記 | `QueryResult.references` の QueryPlan は `owner: QueryParser` だが、Entity Ownership の表と注記は QueryPlan の持ち主を QueryPlan の部品としている（QueryPlan の `entities` もそう）。機械可読の参照が持ち主に解決しない。 | `references` の `owner` を `QueryPlan` に直す。 | New |
| R-07 | Minor | components.md > InsightsSession と ExportJob の責任・依存 | 今のコードで `ExportSnapshotSource::LogAnalytics` を作っているのは `InsightsSession::export_source` だが、設計は「書き出しの元の組み立て」を ExportJob の責任にし、InsightsSession には書き出しの可否しか書かれていない。InsightsSession が ExportJob の型を使う依存も書かれていない。また InsightsSession は QueryPlan（列と形の導出）を使うと QueryPlan の behaviour に書かれているのに、depends_on と QueryPlan の dependents に入っていない。 | 元の組み立てを InsightsSession の責任に直し、InsightsSession の depends_on に ExportJob（型）と QueryPlan を足す（QueryPlan の dependents にも InsightsSession を足す）。または ExportJob に組み立てを移すなら、その旨をコードの移動として書く。 | New |
| R-08 | Minor | components.md > QueryStore の behaviour（最終結果は確定済みの分だけから作る）、Rationale、decisions.md > ADR-003 | 「確定済み」は、今のコードの `Holding::Committed`（確定した分）とも、`settle_range` を通った分（committed と staging の両方）とも読める。今の最終の view は committed と staging の両方から作る（直近 5 分は staging）。前者に読むと、最終結果から直近 5 分が落ち、C5 と前の作業のテストに反する。 | 用語を分ける（例：「`settle_range` を通った分（committed と staging）」と書き、`Committed` と区別する）。FR4.8・FR5.3 の説明も同じ語でそろえる。 | New |
| R-09 | Minor | components.md > QueryEvaluator と QueryStore の依存 | QueryEvaluator の behaviour は「評価の入力は QueryStore の view」としており、`QueryStore::evaluate_step` は評価器の `Evaluation`・`ResultRow` を使う。両方とも一覧に載る部品どうしだが、どちらの depends_on にも相手がなく、結合が図に出ていない。 | 結合の向き（QueryStore が評価器の型を使う）を depends_on に書くか、載せない理由を一覧の範囲の説明に書く。 | New |
| R-10 | Minor | components.md > StatsAggregator と QueryRunner（NFR1・NFR2） | `pct`・`count_distinct`・`sortsFirst` などは値を保持して累積するため、取得済み 100 万件・鍵の種類が多い `by` で、メモリが評価のたび（途中結果は取得中に何度も）に作り直される。途中結果の評価は毎回全体の再評価（A5）で、取得済みが増えるほど 1 回が重くなり、更新の間隔が実際には間引かれる。設計は「前の評価が終わるまで次は始めない」だけで、グループ数や保持量の上限、評価の最中の中断の確認（StatsAggregator が塊ごとに戻ること）を書いていない。 | StatsAggregator に、塊ごとに戻って中断を確認できること、グループ数・保持量が大きいときの扱い（コンソールの件数の上限に合わせる、または打ち切る）を機能設計の問いとして Open Questions に足す。 | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
