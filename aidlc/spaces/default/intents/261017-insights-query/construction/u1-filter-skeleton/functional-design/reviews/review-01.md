## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T01:23:50Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/functional-design/functional-spec.md > 2 章 UC1 の 10 と UC4、rules.md > BR2.7 | [Cancel] を押しても、取得が Cancelled で終わるとは限らない。今の run_fetch は、中止の信号が最後のストリームを読み終えたあとに届いた場合や、キャッシュから全件を返す場合に、未完了のストリームがなければ Completed を返す（run_fetch の説明にある FR5.9 / NFR4.5 と serve_from_cache の「cancel は全件を見せる」）。UC1 の 10 は取得の終わりの状態だけで分岐するため、利用者が [Cancel] を押したのに評価に進んで結果が Done になりうる。BR2.7 の「評価しない」と食い違う。 | InsightsSession に「[Cancel] が押された」印を持たせ、取得が Completed や CompletedWithFailures で終わっても、印があれば評価せずに Cancelled にする、と UC1 の 10・UC4・BR2.7 に書く。状態の移り変わりの図にも、その分岐（Fetching から Cancelled）を足す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/functional-design/rules.md > BR2.3、BR2.8、BR7.3、functional-spec.md > 1 章の AppSession の行 | 取得の 1 本化の決まりが、今の AppSession の一部の判定にしか広げられていない。今のコードでは can_open_settings・open_settings・is_busy_with_disk（設定の画面でキャッシュを消す・置き場所を変える）、is_busy_for_sso_login、ensure_can_change、fetch_cancel_available、confirm_close の abort_fetch が、すべて Phase::Fetching だけを見ている。クエリの画面の取得はログ管理の画面の Phase を使わないため、取得中に設定の画面を開いてキャッシュの削除や置き場所の変更ができ、同じキャッシュを読み書き中のクエリの取得とぶつかる。SSO のログインも走らせられる。また「閉じる」を確認したときに取得を止める abort_fetch が false になり、クエリの取得が止まらないまま閉じる。BR1.5 の (8) は「設定が開いていれば Run query を押せない」という一方向だけで、逆の向き（取得中に設定を開けない）がない。 | BR2.3 と BR2.8 に、Phase::Fetching を見ている上の判定すべてを「FetchOwner が None でない」にそろえる、と書き足す（設定の画面を開く・SSO のログイン・閉じる確認で取得を止める・[Cancel] の押せる条件）。クエリの取得にもキャッシュ書き込み中は [Cancel] を出さない（cache_saving と同じ）決まりを置くか、置かないと決める。テストの対象に、取得中は設定の画面とログインを開けないことを加える。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/functional-design/rules.md > BR5.9 | 「`YYYY-MM-DD HH:MM:SS` は今の parse_in_zone と同じ」とあるが、今の parse_in_zone（parse_wall_clock）は `yyyy/mm/dd hh:mm:ss` のスラッシュ区切りしか読めず、ハイフン区切りは形式エラーで、ミリ秒も読まない。再利用できるのは曖昧な時刻と存在しない時刻の扱いだけで、形式の読み取りは新しく書く必要がある。 | 「曖昧・存在しない時刻の扱いだけ parse_in_zone と同じ。ハイフン区切りとミリ秒の読み取りは新規」と書き分ける。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/functional-design/functional-spec.md > 2 章 UC1 の 2〜7 | 実行可否の確認（2）から FetchOwner の取得（6）・段階を Fetching にする（7）までに複数の手順があり、1 つのロックの中で行うかが書かれていない。今の begin_fetch は確認から Phase の更新までを 1 回の呼び出しで行っている。分かれていると、続けて 2 回押したときに 2 本の実行が通る余地がある。 | 2〜7 を AppSession の 1 回の排他の中での 1 つの遷移とする、と書く（読み取りと日時の変換はその前に済ませてもよい）。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/functional-design/entities.md > QueryResult.rows | rows の説明が「並びは QueryHoldings の時刻順（BR7.3）」と、関係のない BR7.3（接続の変更）を指している。正しくは BR6.3。あわせて、行ごとにストリーム名の文字列を持つ形は、100 万件規模でメモリが大きくなる（epoch で守られているため、位置の整数でも足りる）。 | 参照を BR6.3 に直す。行のキーを位置の整数にするか、今のキーの形のままにするかを、作る段階の判断として書き残す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| sensor-traceability (traceability.json) | PASS: gaps、orphans、invalid entries はすべて 0 | 決まりと上流の対応に抜けはない |
| sensor-upstream-coverage (functional-spec.md) | PASS: consumes がなく、対象は 0 件 | 検査する上流の宣言がなかったため、この結果は何も保証しない。上流との対応は人の目で確かめた |

### Summary

構造と上流との対応は整っており、FR5.6 の追加や R-01 から R-04 の反映とも矛盾しない。気をつけたいのは Major の 2 件で、[Cancel] が Completed で終わった取得を止められないことと、今の AppSession が Phase::Fetching だけを見ている判定（設定の画面・SSO のログイン・閉じる確認）をクエリの取得に広げていないことである。どちらも作る段階で気づけば直せる範囲なので、承認の前に設計書へ書き足すか、承認時にその旨を決めておくことを勧める。
