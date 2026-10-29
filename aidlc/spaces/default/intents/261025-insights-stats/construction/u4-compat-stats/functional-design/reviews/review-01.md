### Review — functional-design / u4-compat-stats (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-29T15:06:58Z
**Iteration:** 1

### Summary

U4 の機能設計は、FR7.1〜FR7.3 を既存 BR に対応づけており（孤立した BR はなし）、アプリが呼ぶ API は読み取りの 3 つだけで、書き出しスクリプトの `start-query` も手元専用で NFR4 と project.md の Forbidden に反しない。Critical はなく、Q1〜Q4 の決定とも矛盾しないため READY とする。ただし Major が 3 件ある。行の対にする方法（許容誤差つき）、`compat_check.rs` の集計用の経路、仮の決まりを確かめるクエリの欠落である。これらは Code Generation が推測で作ることになるため、承認の前に直すことを勧める（助言レビューであり、承認の場で Request Changes にすれば改訂になる）。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | rules.md > BR2.2・BR2.3（functional-spec.md > UC1 手順 3）／今のコード compare.rs `only_in`・`Row` | BR2.2 は「並べてから 1 つずつ対にする」とあるが、今の `Row` は列の名前順に並べた (列, 値) の並びで、並べ替えるとアルファベットで先頭の列が主キーになる。たとえば `count_distinct(eventName) by eventSource` では `count_distinct(...)` が `eventSource` より先、`avg(latency) by bin(5m)` では `avg(latency)` が `bin(5m)` より先に来る。つまり許容誤差のある列（文字列としての数）が並びの主キーになり、丸めや近似の差で並びが入れ替わって、別の鍵の行どうしを対にして誤って Different にする。今の `only_in` は完全一致の `HashMap` による multiset で、許容誤差を入れる方法は設計にない | 対の作り方を明記する。許容誤差のない列（鍵）を完全一致の鍵として行を対にし（鍵の組が同じ行どうし）、対になった行の許容誤差のある列だけを BR2.2・BR2.3 で比べる。鍵が重複する行（鍵の列がない `stats count(*)` など）や、片側にしかない鍵の行の扱いも書く。count_distinct のように許容誤差のある列しか持たない行（by なしの集計）は行数が 1 なので、そのまま対にする | New |
| R-02 | Major | functional-spec.md > §1 部品の表（`compat_check.rs` が載っていない）／rules.md > BR2.1・BR2.4／今のコード compat_check.rs `evaluate_query`・`visible_sort_keys`・`next_row_after_cut`・`row_cap`・`in_boundary_second` | BR2.1 は集計の Row の作り方だけを定めるが、集計の結果をアプリ側の `AppRows` にする経路は設計にない。今の `visible_sort_keys` は `plan.columns()`（FieldRef）で並べ替えの項目を探すため、`as n` の別名（`sort n desc`）は見つからず空になる。すると BR1.1 の `stats count(*) as n by eventSource \| sort n desc \| limit 10`（s02、order）は、切られたとき「並べ替えの項目が列にない」で Skipped になり、切られなくても順番を比べない。`next_row_after_cut`（打ち切りの同点の段）は `store.copy_rows` を使うイベント用で、集計の行の写しでは動かない。`boundary_events`（期間の端の 1 秒のマーク、R-03）も集計の行にはイベントがなく、`count(*) by bin(5m)` で端のイベントがコンソールとの差の原因になるのを見分けられない | spec の部品の表に `compat_check.rs` を加え、集計の経路を決める。(1) 並べ替えの項目は `as` の別名と鍵を含めて集計の列の名前から求める。(2) 打ち切り・同点の段は集計の行の写しで動くようにする（または s02 を「切られない大きさ」で保証する）。(3) 端の 1 秒は集計の入力のイベントで判定する。(4) `limit` が stats の前か後かで `row_cap` の意味が変わる点を書く | New |
| R-03 | Major | rules.md > BR1.1・BR1.4／U1:BR2.7（鍵の昇順）・U2 の関数名の大文字・小文字・U2:BR3.2 | BR1.4 は「仮の決まりごとにクエリを対応づける」とするが、次の仮の決まりを確かめるクエリが BR1.1・BR1.2 にない。(a) 鍵の昇順（U1:BR2.7）：sort のない stats は set で比べるため行の順が見られず、order にしても今の順番の段は `sorted && sort_keys 非空` が条件なので比べない。(b) 関数名の大文字・小文字（U2 の BR1 付近の `sortsfirst` は Unsupported とした決まり）：BR1.1 にあるのは命令の `STATS` だけで、`COUNT(*)`・`Count_Distinct` のような関数名の大文字・小文字がない。(c) 既定の 10,000 行の打ち切り（U2:BR3.2）は「行が少なければ打ち切られないことだけ」と自認しており、決まりそのものは確かめられない | (a) sort なしの stats で、by の鍵の列の並びを比べる方法（例：order で sort がないときは鍵の列で行の並びを比べる）を BR2 に足し、それを使うクエリを s に足す。(b) 関数名の大文字小文字のクエリ（拒否の一致か不一致かを確かめる）を足す。(c) BR1.4 の対応で、10,000 行の決まりを「確かめられない（notChecked）」と明記する。あわせて BR1.4 の対応表（仮の決まり → クエリ id）を spec か rules に実際に書く | New |
| R-04 | Minor | rules.md > BR1.3／今のコード queries.rs `parse_line`（`splitn(4, '\t')`） | 5 つ目の欄は、今のパーサだと 4 つ目（クエリ）に取り込まれる（`splitn(4)` は残りをすべて 4 つ目に入れる）。「省略できる 5 つ目の欄を足す」は、今の 4 欄の組との互換を保つなら `splitn(5)` に変え、4 欄の行がそのまま読めること、クエリに tab が混ざらないことをテストで守る必要がある。設計にその注意がない | BR1.3 に、`splitn(5)` に変えること、4 欄の行は従来どおり読めること、5 つ目の欄の書式（`;` と `=` を列の名前に含む場合の扱い）を書く | New |
| R-05 | Minor | rules.md > BR1.2（n04 `sum(latency) as s by status \| sort s desc`）、BR2.2／今のコード compare.rs `first_order_mismatch`・`tie_at_cut` | 並べ替えの項目が小数の列（許容誤差のある列）のとき、順番の段と打ち切りの同点の段は文字の完全一致で比べる。n04 の `s` が小数なら、丸めの差で同じ順位の行が入れ替わって見える／同点が見分けられない。BR2.2 は行の集まりの段でしか許容誤差を使わない | 順番の段・同点の段でも、許容誤差のある列は BR2.2 の「同じ」とみなす比べ方を使うと書く（または n04 の `s` は整数になる条件にする） | New |
| R-06 | Minor | rules.md > BR3.1／unit-of-work.md > U4（含む部品は CompatCheck のみ、責務は「食い違いを記録する」、大きさ S） | BR3.1 は U4 の中で中核（StatsAggregator・QueryParser・QueryEvaluator）を直し、U1・U2 の `rules.md` の注記も直すとする。unit-of-work の U4 は部品が CompatCheck だけで「記録する」までで、大きさも S のため、範囲が広がる。さらに承認済みの U1・U2 の設計文書を後から編集すると、project.md の「レビュー後の修正は次へ回す」やチェックポイントの一致の確認と衝突しうる | 直す対象と範囲を unit-of-work の U4 の責務に追記するか、U1・U2 の `rules.md` は編集せず U4 の作る段階の記録だけに残す（注記は U4 側に置き、U1・U2 の BR を参照する）と変える。大きさ S の前提を見直す旨も書く | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
