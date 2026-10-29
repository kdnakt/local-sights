### Review — functional-design / u1-stats-skeleton (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-29T04:42:23Z
**Iteration:** 1

### Summary
質問票 Q1〜Q6（すべて A）と ADR-001・ADR-002・ADR-006、C1（Logs Insights の API を呼ばない）、project.md の Forbidden には沿っており、矛盾はない。traceability.json も、OK の対象はすべて rules.md に実在する BR で、孤立した BR もない。Critical はない。ただし Major が 3 件ある。いまのコードに当てると、(1) 評価の段階の機械が集計の行をどう持つか、(2) 書き出しの列を名前の文字列にすると events の行の文字が作れなくなること、(3) 字句解析器が `bin(5m)` を読めないこと、の 3 点でコード生成が推測を迫られる。この依頼の基準（Critical がなければ READY）に従い判定は READY とするが、Code Generation の前（または冒頭）にこの 3 件を直すことを強く勧める。Minor 6 件は直さなくても進められる。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | rules.md > BR2.9・BR3.1・BR3.2、functional-spec.md > 2 章 UC1 手順 5・8 | 「いまの評価の段階の機械（`Evaluation` / `Stage`）に stats の段を足す」とあるが、足し方が書かれていない。実際のコードでは次の点がすべて未定のままである。(a) `Evaluation.rows` は `Vec<usize>`（保持の位置）で、集計の行（型つきのセル）を持つ場所がない。`EvalOutput` も位置しか運べない。(b) `QueryRunner` は `output.rows` を `copy_rows` に渡して `QueryResult::from_output` を作る（runner.rs の終わり）。集計の行をこの経路に載せるのか、runner が形で分岐するのかが決まっていない。UC1 手順 8 は「QueryEvaluator が QueryResult を作る」と書くが、いまは runner が作っている。(c) `finish_commands` の既定の 10,000 件の打ち切り（`!plan.has_row_limit()` で `rows.truncate`）と `Columns` の段（空の列を落とす）を、aggregates でどう避けるか。BR3.2 は Columns を行わないとだけ書き、既定の打ち切りには触れていない。(d) 進み具合 `evaluated()` は `is_chunked` / `passes_planned` に依存するため、stats の段が chunked に数えられないと進み具合が 0 のままになる。(e) 塊のあとの「行を作って鍵の昇順に並べる」仕上げ（BR2.6・BR2.7）は塊に分かれず、止める信号を確かめられない。BR3.1 の「QueryRunner は型に追従するだけ」は、(b) のとおり成り立たない。 | 機能設計に次を書き足す。stats の段の状態（例：`Pass::Stats { next, aggregator }`）、`EvalOutput` が運ぶ集計の行（または `EvalOutput` を形つきの列挙型にする）、runner が形で分岐して `copy_rows` を呼ばないこと、既定の 10,000 件の打ち切りと Columns の段を aggregates で行わないこと、`is_chunked` / `passes_planned` に stats を数えること、仕上げの並べ替えが 1 つの止められない段であること（鍵の種類の数が多いときの時間は NFR1 の測定に含める）。これに合わせて BR3.1 の「追従するだけ」を直す。 | New |
| R-02 | Major | entities.md > ExportSnapshotSource.LogAnalytics.columns（`list<string>`）・QueryResult.columns、rules.md > BR3.1・BR4.4 | 書き出しの `columns` と `QueryResult.columns` を「列の名前の文字列」に変えるとしているが、events の行の文字（BR4.4「events は今のとおり」）は `row_cells(columns: &[FieldRef], …)` で作っている。`FieldRef` は JSON の項目（`JsonPath` の keys とバッククォート付きの名前）や `toMillis(@timestamp)` を区別する。名前の文字列から `FieldRef` を復元すると壊れる（`user.id` と `` `user.id` `` が別の項目、など）。`into_snapshot` と `display_rows` の events の道は、このままでは作れない。BR5.1（events の結果は前と同じ）を守れなくなる。 | 列を `ColumnSet` のような列挙型にする（events は `Vec<FieldRef>`、aggregates は `Vec<String>`）。または `QueryResult` に名前の一覧に加えて events 用の `FieldRef` の並びを残す。`ExportSnapshotSource::LogAnalytics` も同じ形にそろえる。どちらを採るかを entities.md に書く。 | New |
| R-03 | Major | rules.md > BR1.5（と BR1.4・BR1.7） | BR1.5 は `bin(` の中身を「`<整数><単位>`」と読むとするが、いまの字句解析器 `number()` は数字に英数字が続く語（`5m`、`30s`、`1d`、`5ms`）を字句の段階で `INVALID_NUMBER`（Syntax）にする（parser.rs の `glued` の判定）。そのため `bin(5m)` はパーサに届く前にエラーになり、BR1.5 の `ms` / `w` / `mo` を Unsupported にする分岐にも到達できない。BR1.4 の既定の列の名前（「`stats` のあとの文字をそのまま」）も、トークンを連結するのか元の文を切り出すのかが書かれていない。いずれもコード生成が推測することになる。 | 字句解析器の変更を決める。例：`bin(` の中だけは数と単位をまとめて 1 つの語として読む。または `INVALID_NUMBER` を出さずに元の文字を保つ。そのうえで、`ms` を Unsupported（Q1 A）、`0` と形の違いを Syntax にする分岐が、どのトークンに対して働くかを書く。既定の列の名前は元の文の切り出しであること、改行・コメント・空白を含むときの扱いも 1 行書く。 | New |
| R-04 | Minor | rules.md > BR2.1・BR2.4、entities.md > CellValue | 鍵の値と min / max の値の扱いに食い違いと未定が残る。(a) BR2.1 は Boolean を `1` / `0` として扱うが、BR2.4 の min / max は `NaturalSort` の `SortValue` を使い、Boolean は `true` / `false` の文字になる。同じ JSON の値が鍵では `1`、min では `true` と出る。(b) 鍵は「型と値」で区別するため、JSON の数 200 と文字列 `"200"` は別のグループになるが、表では同じ `200` の行が 2 つ並ぶ。(c) `by @timestamp` や `sum(@timestamp)` のセルの種類（timestamp か integer か）と、f64 の鍵の等価（`-0` と `0`）が書かれていない。 | 表に出す文字が同じになるグループは 1 つにまとめるか、別にするかを 1 行で決める。Boolean の文字は鍵と min / max でそろえる。`by @timestamp` のセルを timestamp にするかを BR2.1 に足す。 | New |
| R-05 | Minor | rules.md > BR2.8・BR1.5、entities.md > CellValue | 数値の文字と期間の境界が実装者に委ねられている。(a) 小数 4 桁の丸めは f64 の `{:.4}` 書式と `(x*1e4).round()/1e4` で結果が異なる値がある。「half away from zero」の実装を 1 つに決め、`-0.00001` が `-0` ではなく `0` と出ること、2^53 を超える整数、`inf` を書く。(b) `sum` の結果が integer か decimal かが明記されていない（「整数に収まるかは表示で決める」だけ）。(c) 丸めは UC1 手順 7 では「セルの値を決めるとき」、BR2.8 では「文字にするとき」と読め、components.md の「ここで 1 回だけ決める」とも揃っていない。(d) `bin()` の期間の上限がなく、`count × millis` が i64 を超えうる。 | 丸めを 1 つの純粋な関数（置き場所は中核）に決めて UC1・BR2.8 の書き方をそろえる。テスト用に境界の値の表（`1.00005`、`-0.00001`、`2^53+1`、`1e20`）を足す。`bin()` の `count` に上限（例：オーバーフローしない範囲）を置き、超えたら Syntax にする。 | New |
| R-06 | Minor | rules.md > 先頭の「U1 だけの暫定の決まり」・BR2.9、functional-spec.md > 8 章 | 鍵の種類の数（グループ数）に上限がなく、U1 のメモリの見積りがない。FR1.7（件数の上限）は U2 に回されているので扱いは妥当だが、`stats count(*) by @message` のように 100 万件で 100 万グループになる場合、`AggregateGroup` と結果の写しの両方が全件分メモリに載る。7 章の「取得済み 100 万件」の測定がこのケースを含むかも書かれていない。 | 暫定の決まりに「U1 はグループ数の上限なし。U2 で FR1.7 に合わせる」と明記し、7 章の測定に鍵の種類が多いクエリを 1 本含める。 | New |
| R-07 | Minor | functional-spec.md > 1 章の部品の表・7 章の 2、rules.md | 薄い一本の確かめ（Walking Skeleton の 2）で使う `fetch_check`（確認用プログラム）の変更が、1 章の部品の表にも rules.md にもない。集計の行をどう文字にして出すか（BR2.8 の「同じ関数」を使うか）、列の見出しと行の数をどう出すか、100 万件の測定をどの区間で測るかが決まっていない。`compat/format.rs` の `app_row` も `ResultRow` の列挙型化で影響を受けるが「型だけ合わせる」としか書かれていない。 | 1 章の表に FetchCheck を足し、出力の形（見出し、行、件数、経過時間）と測定の区間を数行で書く。 | New |
| R-08 | Minor | functional-spec.md > 全体、rules.md > BR1.2、unit-of-work.md > U1 実装の注意 | U1 に割り当てられた持ち越しのうち 2 件が決着していない。(a) DD R-09（QueryEvaluator と QueryStore の依存の向き）は、どこにも解決が書かれていない。BR3.1 の「追従するだけ」では答えにならない。(b) OQ1「集計の関数の確定した一覧」は、U1 の機能設計で「引き数・結果の型・表示の形」まで起こして U2 に渡す決まりだが、BR1.2 は残りの関数の名前を並べるだけで、`pct` の 2 つ目の引き数や `earliest` / `latest` の結果の型などがない。 | R-09 は 1 行（例：「QueryStore が評価器の型を使う。評価器は QueryStore を知らない」）で解決を書く。OQ1 は関数ごとの引き数の数・結果の型・表示の形の表を `rules.md` に足すか、「U2 の機能設計で決める」と持ち越しの表を明記して unit-of-work の記述を改める。 | New |
| R-09 | Minor | rules.md > BR4.1・BR4.5、src/api.ts の QueryRow、src/components/QueryResultTable.tsx の `cellText` | 画面側の型の変更（`QueryRow extends LogEvent` を任意の項目にする）は妥当で、`event` を `Option` にしても serde の `flatten` は動く。ただし `QueryResultTable` の `cellText` と `columnClass` は列の名前が `@message` / `@timestamp` かどうかで表示を変えているため、aggregates の鍵の列（`by @message` など）が 1 行目だけに切られる。BR4.5 は「しない」と書くが、画面に形を伝える方法（`ResultSummary.shape` を表の部品まで渡す）が書かれていない。 | 表の部品が見出しの `shape` を受け取って分岐することを BR4.5 に 1 行足し、`by @message` のテスト項目を 7 章に加える。 | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
