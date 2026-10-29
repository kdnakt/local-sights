### Review — functional-design / u2-stats-complete (iteration 1)

**Reviewer:** aidlc-architecture-reviewer-agent
**Verdict:** READY
**Date:** 2026-10-29T12:46:18Z
**Iteration:** 1

### Summary
Critical はなく、人間の回答（Q1 A・Q2 A・Q3 A・Q4 B・Q5 A）との矛盾も見つからなかった。トレーサビリティは健全（OK の対象の BR はすべて実在し、U2 担当の FR1.2・FR1.4・FR1.6・FR1.7・FR6.1・FR6.2 はすべて OK、孤立した BR もない）。`sortsFirst = min` も U1 BR2.4（sort と同じ順）に照らして筋が通る。ただし Major が 2 つある。1 つ目は「結果の列」を指す条件の部品（ColumnRef）が entities に名前だけ出て定義がなく、既存の Condition が FieldRef しか持たないことと、CellValue の比べ方も未定義であること。2 つ目は README の一致のテストが、今の README の散文の表のままでは作れないこと。どちらもゲートで直してから Code Generation に進むことを勧める。

**Prior findings**

| ID | Severity | Location | Status |
|---|---|---|---|
| — | — | — | （初回のため なし） |

**New findings**

| ID | Severity | Location | Finding | Recommendation | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/entities.md > AfterStatsCommand.condition（「ColumnRef」）／rules.md > BR1.4・BR3.1 | entities.md は「項目の参照は集計の結果の列の名前（ColumnRef）だけ」と書くが、ColumnRef というエンティティはどこにも定義されていない。今の `Condition` / `ConditionKind`（Compare・CompareFields・Like・Match・In）は `FieldRef`（Timestamp・Message・LogStream・Log・Json）しか持たず、評価も `LogEvent` / `FieldValue` に対して行われる。一方、stats のあとの行は `CellValue`（missing・integer・decimal・text・timestamp）で、BR3.1 の「今の filter の比べ方で調べる」では次が決まらず、実装者が推測することになる。(a) 列の参照を `FieldRef` の新しい種類にするのか、別の型にするのか。(b) CellValue から FieldValue への対応（integer・decimal → Number、text → Text、timestamp → Number、missing → Missing か）。(c) timestamp の列への like / `=~`（今は FieldRef が時刻のとき禁止）と、時刻の列を日時の文字と比べる拡張を許すか。(d) `a = b` のような列どうしの比較（CompareFields）を許すか。(e) 名前のない書き方（`a.b`、`count(*)` など記号を含む名前）を、列の名前の文字との一致で引くのか。U1 BR1.4 で列の名前の重複はエラーなので曖昧さは出ない。 | entities.md に ColumnRef（列の名前と位置）を定義し、Condition の項目の参照を FieldRef か ColumnRef のどちらかにする型を決める。BR1.4 に (a)〜(e) を書く。BR3.1 に CellValue から比べる値への対応表を 1 つ足す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR4.4／README.md > 「Log Analytics queries」の Supported / Not supported の表 | 一致のテストが今の README の形では作れない。README の表は 2 つの表に散文の行が並ぶだけで、1 行の中に命令・項目・演算子・`toMillis` が混ざる（例：「Commands `fields`, `filter`, `sort` (`asc`, `desc`) and `limit` …」）。Not supported の行には `ispresent`・`isempty` のように QueryCatalog に載せない語がある。大文字・小文字もずれている（README は `filterIndex`、BR4.1 の一覧は `filterindex`）。さらに BR4.4 は「表の形は今のまま（行の追加と移動だけ）」と言うため、バッククォートの語をどう取り出し、どの種類の語と比べるのかが決まらない。 | テストが比べる範囲を決める（例：命令の行と関数の行だけをバッククォートの語で取り出し、種類が command・aggregateFunction・function の語と、大文字・小文字を区別せずに比べる）。README に命令用・関数用の専用の行を設けるなら BR4.4 の「形は今のまま」を直す。`filterIndex` と `filterindex` の扱いも決める。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR3.2 | 打ち切りの意味が既存の events の決まりと食い違う余地がある。今のコードは「行が残っていて、かつ matched より少ないときだけ truncatedAt を出す。`limit 0` は 0 行で打ち切りなしと表示する」（evaluator.rs の p08）。BR3.2 は「limit を書いたらその数で切り、残りがあれば truncatedAt = その数」とだけ書くので、`limit 0` が truncatedAt = 0 になりうる。`limit` が 2 つあるとき（`limit 5 | limit 3`）や limit のあとに filter が来るときの matchedCount がどの時点の行の数かも書かれていない。 | BR3.2 に「events と同じ判定（行が 0 なら truncatedAt なし）」と、matchedCount を数える時点（最後の命令の前）を書く。`limit 0` の表を足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR4.1・BR4.3、BR5.1 | 「パーサの KEYWORDS と補完の固定の候補を一覧から作る」と書くが、今の `KEYWORDS`（and・or・not・like・in）は `is_keyword` で filter の裸の語を予約語として扱う。一覧の keyword 種に by・as・asc・desc を入れ、そのまま KEYWORDS を作ると、`desc` や `by` という名前の JSON の項目を裸で書いたクエリが前と違う結果になる。BR5.1 の「変えない」に反する。また、補完の固定の項目に今ある `toMillis(@timestamp)` が、一覧では関数の `toMillis` になって候補の文字が変わる。 | 一覧の語に「予約語か」を表す印を持たせるか、パーサ用の部分集合を決める。補完で `toMillis(@timestamp)` を残すか、変えるかを書く。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR2.2 | pct の順位を「⌈p/100 × n⌉」と書いている。浮動小数でそのまま計算すると誤差で 1 つずれる（例：p = 7、n = 100 では 0.07 × 100 が 7.000000000000001 になり、順位が 8 になる）。p は小数（99.9 など）も許すので、整数の計算だけでは足りない。 | p × n ÷ 100 を、小数 1 桁・2 桁などの p も含めて正確に天井にする書き方（p を 10 の冪倍した整数で計算するなど）を決め、境界の表（p = 7・n = 100、p = 95・n = 20、p = 0、p = 100、n = 1）をテストの先に書く。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR2.6、BR3.1／functional-spec.md > 4. 状態、7. 確かめ方 | U2 の機能設計に任された DD R-10（値を保持する関数のメモリの上限の扱い）は、「上限なしで測る」までで、合否と、測って足りなかったときの扱いが決まっていない。「U1 より大きく遅くならない」は測れる数にもなっていない。pct は全値の一覧、count_distinct は値の文字そのもの（`@message` なら全文）をグループごとに持つので、100 万グループでは小さな割り当てが大量にできる。また、Finishing は止めない 1 段だが、U2 では正規表現つきの filter と NaturalSort の sort が加わり、グループが多いと NFR2 の［Cancel］に応じる要求と合わなくなりうる。 | 測定の合格の数（時間とメモリ）を、U1 の測定値に対する比として書く。足りなかったときの手（count_distinct は値の 128 ビットのハッシュを持つ、仕上げの filter・sort の途中で止める信号を確かめる）を残した問いとして書く。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/rules.md > BR2.5／entities.md > EventOrderKey | EventOrderKey は logGroupName を持つが、U1 の `add(event, 項目の読み手)` が受け取る `LogEvent` はロググループ名を持たない（stream と sequence はある）。U2 では 1 つのロググループなので定数だが、入口にどう渡すか、あるいは U2 では比べないのかが書かれていない。U3 の複数ロググループでも同じ決まりを使うなら、入口の変更が U3 と重なる。 | U2 は（timestamp, stream, sequence）で比べ、ロググループ名は定数のため入れないと書くか、add に渡す手段を決める。 | New |
| R-08 | Minor | aidlc/spaces/default/intents/261025-insights-stats/construction/u2-stats-complete/functional-design/functional-spec.md > 3. コマンドとイベント（get_query_catalog）／8. 残した問い | get_query_catalog は session の状態を使わない定数の答えだが、そのことが書かれておらず、AppState のロックを取るかも決まらない。「今の権限の一覧の一致のテスト（3 か所）に足す」とあるが、一致のテスト自体は数を決めず、build.rs・generate_handler・capability に足せば通る。名前を明示する一覧のテスト（generate_handler_build_rs_and_the_capability_list_the_query_commands）に足す、という書き方が正確。また FR1.7 の「コンソールと同じ」上限（既定 10,000）は仮の決まりだが、残した問いの U4 の比較の項目に入っていない。 | コマンドをロックなしの定数の答えと書き、テストへの足し方を直す。10,000 行の打ち切りも U4 で比べる項目に足す。 | New |

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
