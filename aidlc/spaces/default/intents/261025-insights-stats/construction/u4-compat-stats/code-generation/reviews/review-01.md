### Review — code-generation（u4-compat-stats）

**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-30
**Iteration:** 1
**Verdict:** READY

### 所見
- R-01（重さ：低）`crates/local-sights-core/examples/compat/compare.rs:68`・`:359-362`。小数の判定に足した読み取りの誤差 `READING_SLACK = 1e-12` は絶対誤差にだけ足されている（相対誤差には足していない）。表の小数は 4 桁なので 1e-12 は 8 桁下で、実際に範囲が広がるのは「差が 0.0001 を 1e-12 以内で超える」ごく狭い場合だけ。0.5001 − 0.5 が f64 で 0.0001 をわずかに超える問題を直すのに必要で、BR2.2 の範囲を意味のある形で広げてはいない。受け入れてよい。直すなら、`code-summary.md` の「ずれ」に 1e-12 を足したことを一行足すだけでよい（すでに「実装の主な決め方」に書いてある）。
- R-02（重さ：低）`compare.rs:533-548`（`group_by_key`）と `:515-520`（`match_rows` の `position`）。緩めのある列を持つクエリで、鍵の組が数千になると、グループを線形に探すため 2 乗の時間がかかる。緩めのあるクエリは数の組の少数グループのものだけ（s05・s14・n01〜n06）で、今の組では問題にならない。将来、緩めのある列を持つ大きなグループ数のクエリを足すときに HashMap にすればよい。直さなくてよい。
- R-03（重さ：低）`compare.rs:458`（`CompareMode::KeyOrder if !spec.key_columns.is_empty()`）。keyorder のクエリで鍵の列が空（`by` のない stats）だと、順番の比べを黙って飛ばし Same になる。組の s15 は `by eventSource` があるので影響はない。keyorder を書いたのに比べていない状態を報告に出す（または組の読み込みで by のない keyorder を拒む）と安全。今の組では不要。
- R-04（重さ：低）`src/components/QueryResultTable.tsx:199-205`（R-02 の直し）。`findQueryRow` が失敗すると `openedState` の版が古いままなので、その版の間は開いた行が「読み込み中」のまま残る（エラーは `onError` で出る）。次の版が来れば新しい `find_query_row` が走って戻る。`onError`（App の `reportError`）は `useCallback([])` で安定しており、依存の変化で取り消されたまま再実行されない、という経路はない。許せる範囲。`code-summary.md` が既知の制限として書いているとおりで、新しい不具合とは言えない。

### 確かめたこと（問題なし）
1. BR1.3 の 5 つ目の欄：`parse_line` は `splitn(5)` で、4 欄の行は従来どおり読める（テストあり）。5 つ目は `parse_tolerances` で重複・空の列・未知の種類を行のエラーにする。クエリの文にタブは書けない（冒頭の注記）。
2. BR2.2・BR2.3：小数は `|a−b| ≤ 0.0001(+1e-12)` または `≤ 1e-6×max`、近似は整数の `|a−b|×50 ≤ max` で 2%。完全に同じ文字は先に Exact になる。数に読めなければ文字で比べる。
3. BR2.6：緩めのない列の (名前, 値) を鍵に対にし、同じ鍵は緩めのある列の昇順で順に対にする。片側にしかない鍵は `only_in` に入る。緩めがなければ従来の multiset のまま。Row は並べ替え済みなので鍵の一致が列の順に左右されない。
4. BR2.8：`KeyOrder` は行の集まりを比べたあとに、鍵の列の値の並びを出た順のまま比べ、最初の食い違いの位置を出す。
5. BR2.4・BR2.7：sort のない集計が切られたら Skipped、sort のあるものは最後の行と同じ並べ替えの値の行を両側から除いて件数を比べる。sort・limit は stats のあとだけから取り、stats の前の limit は切り取りに数えない（テスト `a3-before`）。bin の鍵は期間の最初と最後の区切りを求め、違いを「期間の端の違い」として分けて出すが、同じとはみなさない。
6. BR2.5・BR1.4：範囲内の違いは `Same (within decimal tolerance: n cells)` などで最初の 5 つを出し、終了コードは違いがあるときだけ 1。`# provisional:` の注記は組にない id を行のエラーにし、報告は matches / differs / notChecked にまとめる。組の注記は設計の表（BR1.4）と一致している（U2:BR3.2 は notChecked）。
7. 組：s01〜s16 は BR1.1 の一覧を満たし（s16 は追加）、n01〜n06 は BR1.2 を満たす。@log を鍵にするクエリも日時の文字との比較もない（BR3.2）。
8. events の比べ方：`tie_at_cut` と `first_order_mismatch` が `same_sort_values` に変わったが、緩めがないときは完全一致と同じ。`only_in` の件数制限は `split_edges` に移っただけで、events の出力は変わらない。events の期待値は変えずに通った。`cargo test -p local-sights-core --example compat_check` は 85 件が通った。
9. 計画からのずれ 1（`insights-export.sh` の 2 か所）：設計の「変えない」が誤りだった。直さないと、keyorder の行は「bad mode」で止まり、5 つ目の欄までクエリとしてコンソールに送られる。直しは最小で、4 欄の行の動きは変わらない（`query=${rest%%TAB*}`）。偽の CLI のテストもある。受け入れる。
10. 持ち越し R-02：開いた行に版を持たせ、`placedHere` が真になるまで詳細を取りに行かない。複数の版が続けて来ても、最後の版だけが `openedState` を置き換える。開いた行がなく先頭の行の鍵もないときは何もせず、次に開いた時点で現在の版になる。テスト 2 本は意味がある（古い位置を読まない、見つからなければ読まない）。持ち越し R-05：`handleExport` が書き出しを始めたときの stage を `isPartialResultExport` で固定し、次の書き出しまで変えない。ログの画面・クエリの画面の両方が `handleExport` を通る。
11. 守ること：自動テストと CI は AWS に接続せず（確認用プログラムは手元だけ、テストは偽のゲートウェイ）、アプリが呼ぶ API の追加はない（`start-query` などの記述は `scripts/compat` と README のみ）。12 桁の数・ARN・アクセスキーの形は組・スクリプト・README になかった。テスト以外のコードに `unwrap()` / `expect()` / `panic!` はない。
12. `traceability.json`：target はすべて `source-manifest.json` のパスに実在し、各 ID に対応するテストがある（例：BR2.6 の `rows_are_paired_by_the_columns_without_a_tolerance`、BR2.7(5) の `a_difference_in_the_first_or_last_bin_is_listed_as_a_period_edge`、BR2.8 の `keyorder_compares_...`）。意味のないテスト（常に通るもの）は見つからなかった。

### まとめ
上流の BR1.1〜BR3.2 に反する不具合は見つからなかった。5 つ目の欄の読み方、緩めの範囲、緩めのない列での対にする比べ方、keyorder、打ち切りと期間の端、報告と終了コード、仮の決まりの対応はいずれも設計どおりで、events の比べ方も変わっていない。`insights-export.sh` の 2 か所の直しは必要で最小なので受け入れてよい。U3 から持ち越した R-02・R-05 の直しも正しく、`findQueryRow` の失敗で読み込み中が残る点は既知の許せる制限である。見つかったのは低い重さの 4 件だけで、どれも直さずに進めてよい。残る作業は、開発者の手元でのコンソールとの比較（UC1・UC2）の結果を `code-summary.md` の表に埋めることである。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
