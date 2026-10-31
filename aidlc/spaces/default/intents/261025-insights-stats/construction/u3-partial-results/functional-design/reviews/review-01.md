### Review — functional-design / u3-partial-results (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-29T12:57:40Z
**Iteration:** 1

### Summary
要件の読み替え（FR4.2・C5・R-08）、質問票 Q1〜Q4 の反映、トレーサビリティ（FR4.x・FR5.x はすべて OK、BR の孤児なし、存在しない BR を指す行なし）は整っている。一方、今のコードと突き合わせると、FR4.8（2 回数えない・落とさない）を守る仕組み、途中の評価と `&mut sink` の関係、2 回目の［Cancel］の信号、Cancelling を足したときの既存の呼び出し側の 4 点で、作る段階で設計を補わないと正しく作れない箇所がある（Major 5 件）。Critical はないため判定は READY とするが、承認の前に Major の修正（Request Changes）を勧める。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | rules.md > BR1.2・BR1.3、functional-spec.md > UC1 手順 4・9 | FR4.8 を守る仕組みが書ききれていない。`fetch_range` は取得後に holder から `take_events()` し、そのあと別の blocking タスクで `settle_range` を呼ぶ。この間、イベントは holder にも確定済みの分にもなく、途中の view が落とす。逆に `settle_range` のあとで登録を外すと 2 回入る。BR1.2 の「settle_range に渡すのと同じ鍵の中で外す」は、どの鍵（store の鍵か、InflightRegistry 自身の鍵か）かも、`take_events` をどこへ動かすかも決めていない。InflightRegistry を持つ鍵と store の鍵の取る順番も未定。 | 「holder から取り出す・settle_range・登録から外す」を store の鍵 1 回の中で行う（取り出しも鍵の中へ動かす）と書く。build_partial_view は store の鍵の中で registry を読む、と順番を固定する。テストの題材として「take_events と settle_range の間に view を作っても 1 回ずつ入る」を足す。 | New |
| R-02 | Major | entities.md > PartialView.entries、rules.md > BR1.3・BR1.4 | 確定済みの分の ViewEntry は `(group, source, position)` で保持の中の位置を指す。`EventTimeline::append` は時刻順に途中へ差し込むため、途中の評価の最中に別の範囲の `settle_range` が走ると位置がずれ、`evaluate_step` と `copy_rows` が違うイベントを読む。BR1.4 の `(epoch, viewVersion)` の照合は view の作り直しだけを見分け、`settle_range` による保持の変化は見分けられない。取得中の写しを指す ViewEntry の種類（今の `Holding` は Committed / Staging の 2 つだけ）も未定義。 | 次のどちらかを決める。(a) `settle_range`（と `add_cached`・`begin_run`）も viewVersion を進め、食い違った評価は捨てて作り直す。(b) 途中の view は確定済みの分も写して持つ。あわせて ViewEntry に取得中の写しを指す種類を足すと明記する。 | New |
| R-03 | Major | functional-spec.md > UC1 手順 3〜8 とその疑似コード、rules.md > BR1.1・BR2.1 | 途中の評価を取得とは別のタスクで走らせ、そこから `sink.on_partial_result` を呼ぶと書くが、今の `run_query` は `sink: &mut S` を取得の流れ（`DeferredFinishSink` 経由）が独り占めしている。`QuerySink` は `Send` だけで、複製も共有もできない。別タスクからどう知らせるか（チャンネルで本流に渡すか、`select!`／`join!` で同じ future の中で回すか、sink を Mutex で共有するか）と、「取得が終わったら動いている途中の評価の終わりを待ち、その結果の通知を先に済ませてから最終の view を作る」順番の保ち方が決まっていない。「最初のページが届いた」「取得済みの件数」を途中のタスクがどこから知るかも未定。 | 並行の作り方を 1 つ選んで書く（例：途中のタスクは結果をチャンネルに送り、`run_query` 本流が取得の await と併せて受け取って sink へ渡す）。あわせて、最初のページと取得済み件数の共有の方法（例：InflightRegistry か atomic の件数）を書く。 | New |
| R-04 | Major | rules.md > BR3.1・BR3.2、functional-spec.md > UC3・UC4・§4 状態図 | 2 回目の［Cancel］が今の作りでは通らない。`request_cancel` は `cancel_available` が `!cancel_requested` を要求し、`cancel_running_query` が止める `StopHandle` は 1 回目で Cancel 済み、`retrier.is_stopped()` は 1 回目から真のままである。BR3.1 は「`is_stopped` などを中断後の評価では使わない」とだけ書き、2 回目の信号の運び方（新しい StopSignal か、session の旗を runner が読むか）、`cancel_available`／`request_cancel` の変更、Cancelling のうち「取得が止まるのを待つ間」の 2 回目（評価がまだ始まっていない）の扱いが決まっていない。BR3.2 の ELSE は result を消すと書くが、BR3.3 は「今の Cancelled の表示」（前回の結果を残す。既存のテストが示すとおり、今の中断は前回の結果を残す）で食い違う。 | 2 回目の信号の運び方と `cancel_available` の新しい条件（Cancelling で 1 回だけ押せる）を書く。取得が止まる前の 2 回目は「評価を始めない」と決める。BR3.2 と BR3.3 の ELSE を、前回の結果を残すかどうかも含めて 1 つにそろえる。 | New |
| R-05 | Major | entities.md > InsightsRun.phase、rules.md > BR3.1、functional-spec.md > §1 表・§4 | Cancelling を足したときに既存の呼び出し側がどう変わるかが書かれていない。コアの `is_running`（Fetching か Evaluating）、`is_fetching`、`finish_fetch`（Cancel なら Cancelled にして false を返す）、`begin` の実行中の判定、`canExport`、画面側では `LogAnalyticsPage.tsx` の `running`（3 か所）、`previous={running}`、`StatusLine`、`api.ts` の phase の型、`messages.ts` が Cancelling を知らない。このままでは Cancelling の間、［Run query］と欄が有効になり、［Cancel］と進み具合が消える。`finish_fetch` が「取得は止まった → Cancelling に移って評価する」に変わることも手順に出てこない。 | Cancelling を running に含めるか、含めた場合の各判定（Run・Cancel・Export・欄のロック・進み具合）を表にして書く。`finish_fetch`／`on_fetch_ended` の戻り値の意味を変える（Cancel でも評価へ進む）と明記する。 | New |
| R-06 | Major | rules.md > BR1.2・BR3.1・BR3.6 | 「失敗・中断で終わった範囲は確定せず、登録から外して捨てる」は今のコードと食い違う。今は取得が完了しなかった範囲でも `settle_range(completed=false)` が全イベントを staging に残し（確定の範囲の記録には入れない）、最終結果と BR3.6（「今の決まりのまま」）が使う。そのまま捨てると、ストリームが一部失敗した範囲のイベントが最終結果から落ち、U1 の「取得できたページは残して表示する」とも合わなくなる。BR3.1 の「写しを使ってから捨てる」も、取得が止まった時点で holder はすでに staging へ渡っている。 | 「確定せず」を「取得済みの範囲の記録に入れない（staging には残す）」に直し、「捨てる」を削る。中断後の view は、取得が止まったあとの staging を含む通常の view と同じものだと整理し、UC3 手順 2 の「取得中の分も含める」を実態に合わせる。 | New |
| R-07 | Minor | rules.md > BR4.2、entities.md > RowIdentity | 「ResultRow のイベントの写しにはストリーム内の順番がない」は誤り。`ResultRow.event` は `LogEvent`（`sequence` を持つ）で、`QueryRow` は `#[serde(flatten)]` でそのまま出す。`copy_rows` で ViewEntry の番号を写す作業は不要で、読んだ人が余計な項目を足しかねない。一方で、取得中の写しに BR1.3 で仮に振る番号は、`StreamSequences` を変えずに「続きの番号」を数える（peek する）作りが要る。committed と staging の振り分けは `settle_range` が時刻で分けるため、写しの番号は holder の並びの順に振れば最終と一致する。範囲外のイベントを `settle_range` と同じく除く点も書かれていない。 | BR4.2 の誤った前提を削り、rowKey は `event.sequence` から作ると書く。BR1.3 に「StreamSequences は読むだけで進めない」「範囲外のイベントは除く」を足す。 | New |
| R-08 | Minor | functional-spec.md > §7 の 3、§8 の 1 行目、rules.md > BR1.1 | NFR1 の合否の書き方が揺れている。§7 は「10% 以上遅くならない」を合否として書くが、§8 とBR1.1 は「数値は作る段階で測って決める」としている。測る条件（機材、100 万件の作り方。CI から AWS へ繋げないため偽の取得元か手元の実 AWS か）と、「途中の評価が取得を待たせない」ことをどのテストで確かめるかが決まっていない。 | 10% は暫定の目安と書くか、決める。測る条件（偽の取得元を使う自動テストと、手元の実測の役割分担）を 1 行足す。 | New |
| R-09 | Minor | rules.md > BR2.4・BR2.8、functional-spec.md > §2 UC1 手順 7 | 画面側の読み直しの細部が足りない。`QueryResultTable` の行詳細の取得は `${runId}:${timeZone}:${position}` で重複を避けており、版が進むと位置が指す行が変わるため、版もこの鍵に入れて開き直す必要がある。`getQueryRows` の効果の依存に version を足すことも書かれていない。DD R-10 の U3 分（評価の中断の確認）がこの文書のどこで扱われているか（U2 の仕上げの `stats` の長い 1 手を中断で止める確認）も追えない。 | 詳細の取得の鍵と効果の依存に version を含めると書く。DD R-10 の中断の確認を BR3.2／BR3.3 のテストの題材に足すか、U2 側の扱いだと明記する。 | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
