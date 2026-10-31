### Review — code-generation（u1-stats-skeleton）

**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-30
**Iteration:** 1
**Verdict:** READY

### 所見

- R-01（重さ：低）……`aidlc/spaces/default/intents/261025-insights-stats/construction/u1-stats-skeleton/code-generation/traceability.json` の coverage のうち、FR1.1・FR1.2・FR1.3・FR1.4・FR1.5・FR1.6・FR1.8・FR2.1・FR2.2・FR3.3・FR3.4・FR3.5・FR3.6・FR6.3 の 14 行は、target のファイルは実在し、その要件を実装している場所として正しいが、ファイルの中にその FR の ID の文字がない（コードのコメントは BR の ID だけを書いている）。BR1.1〜BR5.2 の全 target は、実在し、ID の文字も入っている。直し方：必須ではない。対応表を ID の文字の検索で機械的にたどる運用なら、各 target の先頭のコメントに FR の ID を 1 行足す。
- R-02（重さ：低）……`README.md`（対応範囲の表と、`stats` の結果の説明 1 段落）。unit-of-work.md と rules.md は、README の更新を U2 の担当と書いている。U1 は「暫定」と明記して 1 段落と表を足した。内容は実装と合っていて、「Not supported」からも `stats` を外してあるので、実害はない。ただし U2 の README と QueryCatalog の一致のテストを作るとき、この暫定の文面が書き直しの対象になる。直し方：U2 の計画に「U1 の暫定の README の文面を置き換える」と 1 行書いておく。
- R-03（重さ：低）……`code-summary.md` の「計画からずれたところ」2。Step 10 で evaluator.rs のテスト補助関数を `python3` で書き換えた。変えたのはテストのコードだけで、`cargo test` も通っており、成果物への影響はない。記録は正直。直し方：不要。次からファイル用の道具を使う。
- R-04（重さ：低、情報）……`crates/local-sights-core/src/query/stats.rs` の `key_text`（BR2.1）。小数の鍵は 4 桁に丸めた文字で同じグループになるため、1.00001 と 1.00002 は 1 グループにまとまる。また sum・avg は f64 で足すので、2^53 を超える整数の和は桁が落ちる。どちらも BR2.1・BR2.8 の書いたとおりで、「仮の決まり、U4 でコンソールと比べる」の範囲にある。直し方：U4 の比べる組に、小数の鍵のクエリを 1 本入れる。

### 確かめたこと

1. **BR への適合**
   - 丸め（BR2.8）：`format_aggregate` は、有限でなければ値なし、2^53 以上はそのまま全桁、整数は掛けずにそのまま（桁を落とさない）、小数は ×10,000 の四捨五入（0 から遠い方へ）、-0 は 0、末尾の 0 を落とす、の順で手順どおり。境界の表（0.00005、-0.00005、-0.00004、2^53、2^53-1、1e20、0.1+0.2、f64::MAX、1e-9）のテストがある。
   - グループと並び（BR2.1・BR2.7）：鍵の文字で見分け（数の 200 と文字列の "200" は 1 つ）、鍵のないイベントは「なし」の鍵で 1 グループ、並びは NaturalSort で昇順、「なし」は最後、書いた順に比べる。それぞれテストがある。
   - bin の床（BR2.5）：`div_euclid` で負の時刻も床で切り捨てる。`bin(1s)` で -1 → -1000 のテストがある。1d の境目は UTC の 0:00 で、画面のタイムゾーンに依存しない。
   - by なしの 0 件（BR2.6）：by なしは空のグループを 1 つ作って 1 行、by ありは 0 行。両方のテストがある。
   - aggregates で打ち切りをしない（BR3.2）：`aggregate()` は `Aggregated` に移り、既定の 1 万件の打ち切りと Columns の段を通らない。`aggregates_are_not_cut_at_ten_thousand_and_keep_columns_without_values` が確かめている。
   - 時刻のセル（BR4.2）：`CellValue::Timestamp` は型で持ち、表示・詳細・書き出しの 1 か所（`result_row_cells` と `cell_text`）で `format_time` を通して文字にする。UTC と Tokyo のテストがあり、切り替えても行の数は変わらない。
   - 構文（BR1.x）：`bin(` の中だけ 1 語で読み（BR1.8）、数は 1〜1,000,000、単位は s・m・h・d、ms・w・mo・q・y は Unsupported、そのほかは Syntax。列の名前の重複はエラー。`stats` のあとの命令は Unsupported。
   - ほかの層：ResultSummary.shape、集計の行は values だけ、書き出しは同じ関数、画面は shape で分岐、の各項目が BR4.1〜BR4.5 のとおり。
2. **BR5.1（stats のない道が変わらないこと）**
   - 計画に stats がなければ `EvalOutput::Events` の道で、`finish_commands` と `copy_rows` は変わらない。`a_plan_without_stats_still_gives_events` と `an_events_result_is_still_written_from_its_fields` がある。
   - 入力の見本を替えた 4 つのテストは、許されると判断する。理由は 3 つある。(a) BR5.1 が守るのは「stats を含まないクエリ」で、これらのテストは `stats` を「対応していない命令の見本」に使っていた。U1 が `stats` を読むようにするのは U1 の目的そのものなので、入力に `stats` を使うテストが落ちるのは設計どおり。(b) 見本を `parse @message 'a*b' as x` に替えただけで、問題の鍵（`UNSUPPORTED_COMMAND`）・種類・場所（2 行目 3 列、長さ 5）・語の期待値は同じ。`parse` は今も未対応なので、確かめている中身は弱まっていない。(c) 差分で確認した。`stats` が読まれること自体は別の新しいテストが受け持つ。`messages.test.ts` の鍵の数 42→54 も、新しい鍵 12 個を足したことに合っている。
3. **範囲**：U2〜U4 の仕事には入っていない。`stats` の後の `sort`・`limit`、残りの関数、QueryCatalog、途中結果、コンソールとの比べ（compat は集計を「U4 で比べる」と報告するだけ）のいずれも入れていない。補完の固定の候補への追加は rules.md が U1 に許している。README だけが R-02 のとおり。
4. **チームの決まり**
   - テスト以外のコードに `unwrap()` / `expect()` はない。差分の中の該当行を、`#[cfg(test)]` の位置と突き合わせて確かめた。`panic!` と `unwrap` はテストの中だけ。
   - AWS の呼び出しは、取得の流れを変えていない（BR5.2）。Logs Insights の API などは呼んでいない。runner の差分は結果の分岐だけ。
   - 秘密の情報を書く処理は足されていない。依存も足されていない（Cargo.lock・package-lock.json は変わっていない）。
5. **テストの中身**：`cargo test -p local-sights-core --lib query::` を実行し、187 件が通った（無視 4 件は既存のもの）。stats.rs のテストは、期待値を値そのもので書き、0 件・値なし・境界の値・負の時刻・文字と数の混在を確かめていて、いつも通るだけのものではない。

### まとめ

BR1.1〜BR5.2 は、丸め・グループと並び・bin の床・by なしの 0 件・打ち切りなし・時刻の整形のいずれも実装とテストが合っている。stats のない道は変わっておらず、4 つのテストの見本替えは U1 の目的から出る変更で BR5.1 に反しない。直さないとまずい不具合はなく、所見はいずれも低（対応表の FR の ID、U1 での README の暫定の更新、記録済みの作法違反、仮の決まりの境目）なので、READY とする。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
