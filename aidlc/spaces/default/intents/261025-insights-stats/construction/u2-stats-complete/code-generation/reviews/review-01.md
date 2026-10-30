### Review — code-generation（u2-stats-complete）

**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-30
**Iteration:** 1
**Verdict:** READY

### 所見
- R-01（重さ：低）……`crates/local-sights-core/src/query/plan.rs`（`MAX_PERCENTILE_DECIMALS`、`Percentile::parse`）。pct の百分位の小数を 15 桁までにした上限は、設計（BR1.2・BR2.2）にない新しい制限だが、u128 の順位の計算（最大 100×10^15 の分子 × 件数）を安全に保つための必要な上限で、許せる。超えると PCT_OUT_OF_RANGE になる。エラー文言（`query.syntax.pctOutOfRange`）には「小数は 15 桁まで」と書いてあるので、利用者にも伝わる。直すとすれば、README の pct の説明（「p は 0 から 100」）にも 15 桁を一言足し、U2 の決まりとして残すこと。
- R-02（重さ：低）……`crates/local-sights-core/src/query/evaluator.rs`（`a_missing_cell_meets_a_value_as_a_missing_field_does`）、`code-summary.md` ずれ 2。BR1.4 (c) の本文（今の filter の比べ方をそのまま使う）と括弧書き（Missing はどの比較にも合わない）は設計の中で食い違っていた。実装は本文に従い、値のない列の `!=` は成り立つ（イベントの項目と同じ。README の既存の説明「値と比べるときの値のない項目は `!=` を満たす」とも一致）。`=`・`<`・`>`・like・列どうしの比較は成り立たない。これは許せる。ただし README の stats の節には、集計の行でも同じと書かれていない。直すなら README に一文足し、U4 でコンソールと比べる項目に「値のない列の `!=`」を入れる。
- R-03（重さ：低）……`code-summary.md` ずれ 3。打ち切りの注記を QueryResultTable ではなく LogAnalyticsPage の結果の見出しに出した。unit-of-work.md の分担表は QueryResultTable としているが、既存の events の注記が LogAnalyticsPage にあり、文言を「行」に合わせて出すだけなので、利用者に見える結果は同じ。許せる。U3 が QueryResultTable と LogAnalyticsPage の両方を触るため、U3 の計画で「注記は LogAnalyticsPage にある」と引き継ぐこと。
- R-04（重さ：低）……`code-generation/traceability.json`。各 target のファイルは実在し、BR の ID は対象のファイルに書かれている（BR1.3・BR1.5・BR2.6・BR3.1・BR4.3・BR5.1 などを確認）。一方、FR1.2・FR1.4・FR1.6・FR1.7・FR3.1・FR6.2・FR6.3 は、target のファイルに ID の文字そのものがなく、BR を通してだけたどれる（FR6.1・NFR2・NFR7 は書かれている）。BR2.6 が定める NFR2 のメモリの測定も、時間の測定（`the_heavy_stats_takes_at_most_three_times_the_sum_by_stream`、2.14〜2.37 倍）と「落ちないこと」だけで、メモリの数値は記録されていない。BR2.6 の判定条件（落ちない・3 倍以内）は満たしているので、ブロックしない。次の作業（U3・U4）で、1,000,000 件の重いクエリのメモリの目安を測って記録するとよい。
- R-05（重さ：低）……`src/components/QueryEditor.tsx`（`coloured` の `useMemo`）。`registerQueryMode` という副作用を `useMemo` の中で実行している。同じ規則で何度呼んでも結果は同じ（上書きするだけ）で、テストも通るので不具合ではないが、React の作法では `useEffect` が望ましい。`catalog.rs` の行カバレッジは 88%（手順書の目安 90% を少し下回る）だが、拾えていないのはテストの中の panic の分岐とのことで、許せる。

### 確かめたこと（問題なし）
- **BR2.2 pct**：`percentile_rank` は u128 の整数だけで `max(1, ceil(numerator × n ÷ (100×10^scale)))` を求め、`count` で頭打ちにする。p = 0・7・50・99.9・100 × n = 1・100・101 の表と、10 億件規模の値のテストがある。小数は `Percentile::parse` が文字から（分子, 桁）を取るので、浮動小数を使っていない。負の数・101・`.5`・`5.`・`1e2` は拒否される。
- **BR2.3 Welford**：`mean += delta / n`、`m2 += delta × (x − mean)`。n = 0 は値なし、n = 1 は 0、n ≥ 2 は sqrt(m2/(n−1))。1e9 付近の値のテストで桁落ちがないことを確かめている。
- **BR2.5 earliest・latest**：設計は view の位置で比べるとしていたが、実装は（時刻、ロググループ、ストリーム、ストリーム内の順番）の鍵で比べる。add の呼ばれる順に左右されないので、設計より頑丈で、結果は同じ。同じ時刻のテストあり。
- **BR1.4・BR3.1**：stats のあとの項目は `FieldRef::Column` に解かれ、書いた名前（バッククォートは外す）が列の名前と大文字・小文字まで一致するものだけを引く。名前が `(` に続く場合と、一致しない場合は UNKNOWN_RESULT_COLUMN。`cell_value` の写し方は設計の表のとおり（integer・decimal・timestamp は Number、text は Text、missing は Missing）。時刻の列は `instant.rs` で日時の文字との比較も効く。
- **BR3.2**：`Finishing::cut` の打ち切りと件数の数え方は events の `into_output` と同じ形（filter のあとで数え直す点も、events の `matched_does_not_count…` のテストと同じ）。limit 0 は 0 行で注記なし、行が 0 のときも注記なし。10,000 行と `limit` 付きのテストあり。
- **止める信号**：filter は 10,000 行ずつ、sort・limit は 1 段ずつ進め、`a_stop_while_the_rows_are_finished_drops_everything` で止まったら捨てることを確かめている。
- **BR4.1・BR4.3**：`catalog.rs` は 42 語（命令 15、集計の関数 12、`toMillis`・`bin`、キーワード 9、決まった項目 4）。reserved は and・or・not・like・in の 5 語だけ。`by`・`as`・`asc`・`desc` は reserved でなく、`filter desc = 'x' | sort by desc` のテストで項目として読めることを確かめている。補完の候補は一覧から作り、今の文字（`toMillis(@timestamp)` など）を保つ。
- **BR4.2**：`get_query_catalog` は引き数なし、AppState の鍵を取らず、固定の一覧を返すだけ。権限は `allow-get-query-catalog` だけが capabilities に足され、`build.rs` の一覧、生成された toml、一致のテストがそろっている。新しい AWS の呼び出しはない。
- **BR4.4**：README の 3 つの行を見つけて catalog と比べるテストがあり、通る。
- **BR5.1・計画からずれ 4**：直した既存のテストは、U1 で「対応していない」だった書き方（関数・stats のあとの命令）、BR3.2 が明示して改めた「aggregates は打ち切らない」、BR4.3 で形が変わった補完、鍵の数 54 → 57、`supportedList` の差し込みの形に限られる。どれも BR5.1 とそれぞれの BR が認める範囲で、stats を含まないクエリや U1 の stats の結果の期待値は変わっていない。
- **決まりごと**：テスト以外のコードで `unwrap()` / `expect()` / `panic!` は使われていない（各ファイルの `#[cfg(test)]` より前を確認）。CloudWatch Logs の読み取りの 3 つの API 以外の呼び出し、秘密の情報の書き込み、新しい依存は見当たらない。
- **範囲**：`runner.rs`・`store.rs`・compat 系のファイルは変えておらず、途中結果・中断（U3）やコンソールとの比較（U4）には踏み込んでいない。pct・stats の細部が仮であることは README にも書かれている。
- **テストの実行**：`cargo test -p local-sights-core --lib query::` は 237 件が通る（無視 5）。関連する Vitest 5 ファイル（104 件）も通る。
- **速さ（Q1 A・Q2 A）**：基準を `stats sum(latency) by @logStream` に替え、count_distinct を正確に数える形に戻している。`#[ignore]` のテストは release で 3 倍以内を確かめる形になっており、利用者の決めたとおり。

### まとめ
実装は BR1.1〜BR5.1 に合っており、pct の整数の順位、Welford、earliest・latest、stats のあとの列の参照、打ち切りの数え方、QueryCatalog、`get_query_catalog` の最小の権限、README の一致のテストのどれにも、直さないとまずい不具合は見つからなかった。「計画からずれたところ」の 1〜4 はどれも許せる。上の所見はすべて低で、README への一文の追記（R-01・R-02）、U3 への引き継ぎ（R-03）、メモリの目安の記録（R-04）が主なもの。判定は READY。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
