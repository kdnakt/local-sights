## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-design/security-design.md > 2.2 新しい取得の見分け方 / reliability-design.md > NFR6.6 | 「running のまま別の取得が始まる経路はない」と言い切っているが、画面が新しい取得を見分ける根拠は、画面が直前に見た起点が running かどうかだけである。src-tauri/src/lib.rs では SessionView を鍵の中で作り、鍵を放した後で emit している（344 行、380 行、679 行など）。そのため別のスレッドの emit が順番を追い越すことはありうる。また src/App.tsx は getSession と updateInput の応答を setSession(view) でそのまま置き換えており、順序の保護がない。React が終わりの view と次の取得の view を 1 回の描き直しにまとめた場合も、途中の running false を見ないので、新しい取得の 0 に古い表示が保持される。終わりの view の後に古い running の view が遅れて届くと、止まらない時計が動き出す。段階も古いままになる点は今のコードにもある弱点だが、時計が止まらなくなる分は U4 が新しく足す。 | 見分けを「見た順番」に頼らず、ElapsedView に取得の番号（AppSession が持つ fetch_number を使える）を足して、番号が変わったら保持を解く形を検討する。足さない場合は、「経路はない」を「通常は running false か空を挟む。順序が崩れる経路は今の emit の作りに残る」と書き直し、コード生成の計画に確認を残す。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-design/reliability-design.md > NFR6.7 | 保持した値は「次の合わせ直しまでにライブラリの値に追い越される」と書いているが、画面もライブラリも 1 秒に 1 秒ずつ進むので追い越しは起きない。baseMs を max(incoming, shown) で置き直すたびに、見た中でいちばん大きい送受信の遅れの分だけ先に出た状態が取得の終わりまで残る（棘のように戻らない）。ずれは小さい（遅れのばらつき分）ので 1 秒以内の目標は守れるが、書かれた理屈と、それを確かめるテストの狙い（次の合わせ直しで追いつく）は成り立たない。 | NFR6.7 の説明を「先に出た分は遅れのばらつき分に収まり、終わりの running false で最終の値に置き換わる」に直す。テストは「保持の直後と取得中の最大のずれが 1 秒を超えない」を確かめる形にし、追いつくことを期待する書き方をやめる。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-design/security-design.md > 2.2 時計のフック（useElapsed） | 起点を持つ場所（ref か state）と、nextAnchor を呼ぶ時点（描画中か effect の中か）が決まっていない。performance.now() を描画中に読むと純粋でなくなり（StrictMode の二重実行でも値が揺れる）、effect の中で起点を置くと、新しい取得の始まりで最終の値 00:31 を running の見た目（aria-hidden 付き）で 1 回描いてから 00:00 に直る。NFR5.3 の描画回数のテストも、この選び方で数が変わる。 | useElapsed の内部を一つに決めて書く。例：elapsed のオブジェクトが変わった描画の中で、純粋な nextAnchor に nowMs を渡して useState の起点を更新する（描画中の setState の形）。または useRef に置き、描画回数の期待値を明記する。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-design/security-design.md > 2.1 画面へ渡す形 / inception/contract-design/contract-summary.md > C5 elapsed の meaning | 契約の C5 は「null は一度も取得していない」と書くが、この設計は Aborted と接続の変更でも null にする（NFR6.10、BR4.3、BR4.4）。契約の文言と設計が食い違ったまま残っている。 | 契約の meaning を「null は、表示する経過がない（一度も取得していない、閉じて止めた、接続の変更で前の結果が消えた）」に直す依頼を出すか、設計側に「契約の文言を拡張する」と明記する。U5 が同じ欄を読むので、早めにそろえる。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-design/logical-components.md > 部品の表 | src/test/fixtures.ts の SessionView の見本と、SessionView の型を使う既存テスト（App.test.tsx、StatusLine.test.tsx）が部品に入っていない。SessionView.elapsed を必須の項目として足すと、見本に項目がないと tsc が通らず、実行時に undefined が来ると nextAnchor が null と区別できず落ちうる。 | 部品の表に fixtures.ts の更新を足す。nextAnchor と useElapsed は undefined も null と同じに扱うか、型を必須にして見本を全部直す、と書く。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この段に検証ツールの指定はない） | - | コードとの照合は手で行った。AppSession は Debug と Clone のみ（PartialEq なし）なので SessionClock を足せる。begin_fetch は Fetching で Busy を返し、受け入れ後に phase を設定する。finish_fetch と abort_fetch_with_failure の分岐、apply_connection_change が Done と Failed のときだけ Idle に戻すこと、select_log_group と reload_log_groups が記録を触らないこと、on_finished がキャッシュ保存の後に呼ばれることを確かめ、設計の記述と合っていた。withProgress と withFilter と mergeSessionView は {...view} か {...next} で elapsed のオブジェクトを引き継ぐので、合わせ直しをオブジェクトの入れ替わりで見分ける方法は成り立つ。 |

### Summary

U4 の設計は、現在のコード（begin_fetch、finish_fetch、abort_fetch_with_failure、apply_connection_change、画面側の合成関数）と矛盾せず、実装に進める。Critical と Major はなく、残る指摘は、新しい取得の見分けを順序に頼っていること、保持の理屈の誤り、フックの内部の未決定、契約の null の意味、見本の更新漏れの Minor 5 件である。
