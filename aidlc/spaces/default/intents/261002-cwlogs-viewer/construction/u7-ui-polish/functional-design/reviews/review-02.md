## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-08T04:00:47Z
**Iteration:** 2

### Findings

前回の R-01〜R-10 は carry-forward として同じ ID・同じ位置づけで再掲し、Status を判定した。新しい指摘は R-11 から。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | construction/u7-ui-polish/functional-design/functional-spec.md > §6 BR1.4・BR1.7・BR1.8、frontend-components.md > §2 | 前回の指摘は、展開行の位置の取得手段、累積高さによる逆引き、比例の縮小との組み合わせ、アンカーを保つ手順が未定義、というものだった。修正で `row_positions(keys)`（一括、版つき、同時に 1 呼び出し）、累積と二分探索、`rowLayout.ts` の型、アンカーの保ち方（BR1.8）が定義され、実装者が推測せずに作れる形になった。ただし縮小の式に新しい欠陥がある（R-12）。 | R-12 で扱う。 | Resolved |
| R-02 | Major | functional-spec.md > §6 BR3.1・BR3.2、§3 終了の確認の状態遷移 | 閉じる経路の表（CloseRequested／ExitRequested／Destroyed）、取得中だけ `prevent_close`／`prevent_exit` して closeConfirmation を Pending にすること、中断を [Close] と Destroyed に限ること、`confirm_close`／`cancel_close`、終えてよい印による再入防止、Cmd+Q も確認することが書かれ、SessionView を正とする方針（イベントは合図のみ）に揃った。`ExitRequested` を受けるにはコード生成で `Builder::run` を `build` ＋ `run` のコールバックに変える必要があるが、これは実装の手順で設計の欠落ではない。 | なし（コード生成の計画に「`build()` ＋ `RunEvent::ExitRequested`」を明記するとよい） | Resolved |
| R-03 | Major | functional-spec.md > §6 BR1.6、§5 保持ログの破棄の世代 | 閉じる条件が「保持ログが捨てられたとき」に絞られ、取得中の追加では展開を保つことが規則・状態遷移図に書かれた。方針は正しい。一方で、検出に使う discardGeneration の持ち主が矛盾している（R-11）。 | R-11 で扱う。 | Unresolved |
| R-04 | Major | functional-spec.md > §6 BR1.5 | U3 のキー操作の置き換え、PageUp・PageDown、キーと位置の両方で持つこと、窓にない行での Enter・Space は何もしないこと、展開部分を選択の単位にしないこと、キーが消えたときの選び直し、破棄で一番上、がそろった。 | なし | Resolved |
| R-05 | Major | functional-spec.md > §6 BR1.10、frontend-components.md > §5 | role=grid、aria-rowcount／aria-rowindex、activedescendant、展開部分を行の中の全幅セル（aria-colspan=3）に置くこと、スクロールする展開部分の入力位置、event.target によるキー処理の振り分け、が定義された。細部の食い違いは Minor として R-13 に分ける。 | R-13 で扱う。 | Resolved |
| R-06 | Major | functional-spec.md > §6 BR1.2・BR1.9 | 20 行ぶん＝372 px の数値定義、折り返しの CSS（pre-wrap と overflow-wrap: anywhere）、見積もりの式、ResizeObserver による実測、幅が変わったときの再測定、BR1.8 によるアンカーの補正、が書かれた。 | なし | Resolved |
| R-07 | Minor | functional-spec.md > §6 BR2.1・BR2.4 | 場面は画面が表示の場所から決め、ライブラリは変えないことに揃った。ステータス行の詳細の行（status-line-detail）を外すことも明記された。 | なし | Resolved |
| R-08 | Minor | functional-spec.md > §6 BR2.2 | {retry} を場面ごとに [Fetch]／[再読み込み] へ差し込む形になり、文言の正本が 1 つの表にまとまった。起きない組み合わせは Other の文を使うことも決まった。 | なし | Resolved |
| R-09 | Minor | functional-spec.md > §6 BR4.1、§9 | tauri.conf.json の minWidth／minHeight を 800×500 から 1024×640 に変え、起動時の大きさもそれ以上にすることが明記された（現状は 1200×800 で満たす）。1024×640 の確認と書き込み中の [Close] の確認も手元の確認項目に入った。 | なし | Resolved |
| R-10 | Minor | functional-design/traceability.json、functional-spec.md > §6 BR1.2 | NFR2 が upstream_ids と coverage に入った。全文を HTML として解釈せずテキストノードで出すこと、長さの上限を設けない理由（1 件 256 KB まで）も書かれた。 | なし | Resolved |
| R-11 | Major | functional-spec.md > §5 保持ログの破棄の世代・行の位置をまとめて引くコマンド、§6 BR1.6・BR1.7 / crates/local-sights-core/src/log_view.rs（`clear`）、src-tauri/src/lib.rs（`TauriSink`、`begin_and_spawn_fetch`） | discardGeneration の持ち主が食い違っている。§5 は「ライブラリ（AppSession）→ SessionView」で、AppSession が「取得の開始、接続の変更、中断」のたびに 1 増やすとする。一方 BR1.7 は `row_positions` が「LogView の 1 回のロックの中で」timelineVersion・resultVersion・discardGeneration を返すとする。LogView（log_view.rs）は世代を持たず、AppSession のロックも取らない（lib.rs の `find_row_position` も LogView のロックだけ）。実際の破棄は、取得の開始では coordinator が `TimelineStore::discard`（`LogView::clear`）で後から行い、AppSession は `on_job_started`／`finish_fetch` で版を受け取るだけである。したがって AppSession が `begin_fetch` で世代を増やすと実際の破棄より前に増え、`row_positions` の答えの世代と、`session-changed` の世代の順序・一致が保証されない。(logStreamName, sequence) は次の取得で別の行を指すため（BR1.6 自身の理由）、世代が遅れると別の行を展開したまま、あるいは選んだままにしてしまう。取得の失敗・中断のときに保持ログが残るのか捨てられるのかも書かれていない。 | discardGeneration の持ち主を LogView に一本化する（`clear()` の中で 1 増やし、timelineVersion と同じロックで読めるようにして、`row_positions`・SessionView（`set_timeline_version` と同じ受け取り方）に載せる）。世代が増える条件を「LogView が実際に空にされたとき」と定義し、取得の開始・接続の変更・中断のうち実際に `clear` が走るものを列挙して、失敗・キャッシュ読み込みのときの扱いも書く。画面は「`row_positions` の答えと SessionView のうち新しい世代」を正にする。 | New |
| R-12 | Major | functional-spec.md > §6 BR1.4、frontend-components.md > §2 `scale`・`scrollTopForAnchor`・「展開がないとき U3 と同じ答え」/ src/virtualScroll.ts（`toVirtual`・`fromVirtual`・`maxScrollTop`） | 縮小の式が U3 の対応づけと違い、100万件で末尾が見えなくなる。BR1.4 は s = 1,000 万 / H、実際のスクロール位置 = 仮想の位置 × s としている。しかしスクロール領域の高さは min(H, 1,000 万) で、実際に動ける最大のスクロール位置は「領域の高さ − 画面の高さ」＝ 1,000 万 − vp である。この式では最後の行を先頭に出すのに必要な位置が (H − vp) × s ＝ 1,000 万 − vp × s となり、最大のスクロール位置を超える。結果、末尾の約 vp × (1/s − 1) 仮想 px は到達できない。たとえば 100万件（H = 2,200 万 px、s ≈ 0.4545、vp = 600 px）では末尾の約 33 行が、スクロールでも End キーでも画面に出ない。U3 の `toVirtual`／`fromVirtual` は (H − vp) と (領域の高さ − vp) で正規化していて、この問題を避けている。また frontend-components は「展開がないとき U3 の `virtualScroll.ts` と同じ答え（U3 のテストをそのまま通す）」とするが、上の違いのため、1,000 万 px を超える範囲では同じ答えにならず、U3 のテストを通らない（`offsetY` も変わる）。100万件は NFR2 の対象規模である。 | 縮小を U3 と同じ正規化にそろえる：最大の仮想の上端 = H − vp、最大のスクロール位置 = min(H, 1,000 万) − vp とし、実際のスクロール位置 = 仮想の上端 × (最大のスクロール位置 / 最大の仮想の上端) と書く。`scale()` を比ではなく、この 2 つの最大値から求める形に直し、`rowAt`・`scrollTopForAnchor`・`visibleRange` の入出力に画面の高さを含める。「U3 のテストをそのまま通す」を残すなら、その式で成り立つことを確かめる。末尾の行に届くこと（100万件で End）を `rowLayout.test.ts` の観点に加える。 | New |
| R-13 | Minor | functional-spec.md > §6 BR1.10、BR4.4、frontend-components.md > §4 | スクロールする展開部分の入力位置の規則が食い違う。BR1.10 は「スクロールするときは展開部分に入力位置を受けさせる（tabIndex=0）」とする一方、「選んでいる行の展開部分がスクロールするときは Tab でその中に入る」とも書く。前者のままだと、選んでいない行の展開部分も Tab の順に入り、BR4.4 の「ログ一覧（→ 選んでいる行のスクロールする展開部分）」と合わない。また role=grid を付ける要素と、入力位置を受ける要素（今は見出しの外にあるスクロールの viewport、tabIndex=0）が同じか、見出し行を含むかも定まっていない（aria-activedescendant は入力位置を持つ要素に付ける）。 | tabIndex=0 は「選んでいる行の、スクロールする展開部分」だけに付け、ほかは -1 にする。role=grid と tabIndex=0 を付ける要素を 1 つに決め、見出し行をその中に入れるか（aria-rowindex=1 を成り立たせるため）を書く。 | New |
| R-14 | Minor | functional-spec.md > §5 展開している行、BR1.9 / inception/domain-design/components.md SessionState.expandedRows | (1) 展開の集合を components.md の SessionState.expandedRows から画面の中に移すことは §5・§8 に理由つきで書かれているが、上流の components.md は更新されず、共有契約と食い違ったまま残る。(2) BR1.9 の見積もりは等幅の文字 1 つの幅で数えるため、日本語の全角文字（半角の約 2 倍）が多いログでは行数を少なく見積もる。実測で直るので害は小さい。 | (1) 変更を components.md の差分として記録するか、設計の前提に「意図した契約の変更」と明記して Code Generation／Build and Test の確認項目に入れる。(2) 全角文字は 2 倍で数える、または見積もりの限界として §8 に書く。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この段階で割り当てられた検証ツールは dispatch に無い） | 実行せず | 代わりに python3 で traceability.json を機械的に照合した：upstream_ids と coverage の ID が一致し、coverage の BR 参照は functional-spec.md に存在する（`BR5.5` は `U3:BR5.5` の参照）。既存コード（src-tauri/src/lib.rs、src/components/LogTable.tsx、src/virtualScroll.ts、src/hooks/useRowWindow.ts、log_view.rs、session.rs、tauri.conf.json）と照合して R-11・R-12 を確認した。他の作業単位の設計ファイルは開いていない。 |

### Summary

前回の R-01、R-02、R-04〜R-10 は解消され、設計は実装できる形にかなり近づいた。一方、修正で入った 2 点が Major として残る：discardGeneration の持ち主が AppSession と LogView で矛盾していること（R-11、R-03 は部分的な解消にとどまる）と、縮小の式が U3 の正規化と違うため 100万件で末尾の約 33 行に届かないこと（R-12）。どちらも修正は小さく、R-11 は持ち主を LogView に一本化、R-12 は U3 の正規化にそろえればよい。最大 2 回のレビューに達したため、運用ルール（project.md の Corrections）に従い、直すかどうかは人間が判断する。
