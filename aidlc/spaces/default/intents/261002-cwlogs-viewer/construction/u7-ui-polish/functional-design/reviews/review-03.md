## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-08T14:54:10Z
**Iteration:** 2

### Findings

成果物（functional-spec.md、frontend-components.md、traceability.json）は前回の iteration 2 から変わっていない。前回の R-01〜R-14 を同じ ID で再掲し、成果物の現状で再判定した。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u7-ui-polish/functional-design/functional-spec.md > §6 BR1.4・BR1.7・BR1.8、frontend-components.md > §2 | 展開行の位置の取得手段（`row_positions(keys)` の一括取得、版つき、同時に 1 呼び出し）、累積高さと二分探索による逆引き、`rowLayout.ts` の型、アンカーを保つ手順（BR1.8）が定義され、実装者が推測せずに作れる形になった。縮小の式の欠陥は R-12 で扱う。 | なし | Resolved |
| R-02 | Major | functional-spec.md > §6 BR3.1・BR3.2、§3 終了の確認の状態遷移 | 閉じる経路の表（CloseRequested／ExitRequested／Destroyed）、取得中だけ止めて closeConfirmation を Pending にすること、中断を [Close] と Destroyed に限ること、`confirm_close`／`cancel_close`、終えてよい印による再入防止、Cmd+Q も確認することが定義され、SessionView を正とする方針に揃った。 | なし | Resolved |
| R-03 | Major | functional-spec.md > §6 BR1.6、§5 保持ログの破棄の世代 | 展開を閉じる条件が「保持ログが捨てられたとき」に絞られ、取得中の追加では保つことが規則と状態遷移図に書かれた。方針は正しい。ただし検出に使う discardGeneration の持ち主が §5 と BR1.7 で矛盾したままである（R-11）。 | R-11 を直すことで解消する。 | Unresolved |
| R-04 | Major | functional-spec.md > §6 BR1.5 | U3 のキー操作の置き換え、PageUp・PageDown、キーと位置の両方で持つこと、窓にない行での Enter・Space は何もしないこと、展開部分を選択の単位にしないこと、キーが消えたときの選び直し、破棄で一番上、がそろった。 | なし | Resolved |
| R-05 | Major | functional-spec.md > §6 BR1.10、frontend-components.md > §5 | role=grid、aria-rowcount／aria-rowindex、activedescendant、行の中の全幅セル（aria-colspan=3）、スクロールする展開部分の入力位置、event.target によるキー処理の振り分けが定義された。細部の食い違いは R-13 に分ける。 | R-13 で扱う。 | Resolved |
| R-06 | Major | functional-spec.md > §6 BR1.2・BR1.9 | 20 行ぶん＝372 px の数値定義、折り返しの CSS、見積もりの式、ResizeObserver による実測、幅が変わったときの再測定、BR1.8 によるアンカーの補正が書かれた。 | なし | Resolved |
| R-07 | Minor | functional-spec.md > §6 BR2.1・BR2.4 | 場面は画面が表示の場所から決め、ライブラリは変えないことに揃った。ステータス行の詳細の行を外すことも明記された。 | なし | Resolved |
| R-08 | Minor | functional-spec.md > §6 BR2.2 | {retry} を場面ごとに [Fetch]／[再読み込み] へ差し込む形になり、文言の正本が 1 つの表にまとまった。起きない組み合わせは Other の文を使うことも決まった。 | なし | Resolved |
| R-09 | Minor | functional-spec.md > §6 BR4.1、§9 | tauri.conf.json の minWidth／minHeight の変更と 1024×640 の手元確認、書き込み中の [Close] の確認が明記された。 | なし | Resolved |
| R-10 | Minor | functional-design/traceability.json、functional-spec.md > §6 BR1.2 | NFR2 が upstream_ids と coverage に入り、全文をテキストノードだけで出すこと、長さの上限を設けない理由も書かれた。 | なし | Resolved |
| R-11 | Major | functional-spec.md > §5「保持ログの破棄の世代」「行の位置をまとめて引くコマンド」、§6 BR1.6・BR1.7、§1 の部品のつながりの図 | discardGeneration の持ち主が文書内で矛盾している。§5 は持ち主を「ライブラリ（AppSession）→ SessionView」とし、AppSession が「取得の開始、接続の変更、中断」のたびに 1 増やすと書く（BR1.6 も同じ）。一方 BR1.7 は `row_positions` が「LogView の 1 回のロックの中で」timelineVersion・resultVersion・discardGeneration を返すとする。保持ログを実際に空にするのは LogView 側の破棄であり、AppSession が別の契機で増やすと、実際の破棄との順序が保証されず、`row_positions` の答えの世代と SessionView の世代がずれうる。(logStreamName, sequence) は次の取得で別の行を指すため（BR1.6 自身の理由）、世代が遅れると別の行を展開したまま、あるいは選んだままにする。取得の失敗・中断・キャッシュ読み込みのときに保持ログが残るのか捨てられるのか、世代が増えるのかも書かれていない。 | discardGeneration の持ち主を LogView に一本化する（実際に空にした時点で 1 増やし、timelineVersion と同じロックで読み、`row_positions` と SessionView に載せる）。増える条件を「LogView が実際に空にされたとき」と定義し、取得の開始・接続の変更・中断のうち実際に空にする契機を列挙し、失敗・キャッシュ読み込みの扱いも書く。§5 の持ち主の列と BR1.6 の文を揃える。画面は「`row_positions` の答えと SessionView のうち新しい世代」を正にする。 | Unresolved |
| R-12 | Major | functional-spec.md > §6 BR1.4、frontend-components.md > §2 `scale`・`scrollTopForAnchor`・「展開がないとき U3 と同じ答え」 | 縮小の式が最大のスクロール位置を考慮していない。BR1.4 は s = 1,000 万 / H、実際のスクロール位置 = 仮想の位置 × s としている。しかしスクロール領域の高さは min(H, 1,000 万) で、動ける最大のスクロール位置は「領域の高さ − 画面の高さ vp」である。最後の行を先頭に出すのに必要な位置は (H − vp) × s = 1,000 万 − vp × s となり、最大のスクロール位置（1,000 万 − vp）を超えるため、末尾の約 vp × (1/s − 1) 仮想 px が到達できない。100万件（H = 2,200 万 px、s ≈ 0.4545、vp = 600 px）では末尾の約 33 行が、スクロールでも End キーでも画面に出ない。100万件は NFR2 の対象規模である。また frontend-components.md は「展開がないとき U3 の `virtualScroll.ts` と同じ答え」とするが、U3 は (H − vp) と (領域の高さ − vp) で正規化する対応づけのため、1,000 万 px を超える範囲では同じ答えにならない。`scale`・`rowAt`・`scrollTopForAnchor`・`visibleRange` の入出力に画面の高さがないことも、この式の欠陥の表れである。 | 縮小を、最大の仮想の上端 = H − vp と最大のスクロール位置 = min(H, 1,000 万) − vp から求める正規化に直す（実際のスクロール位置 = 仮想の上端 × 最大のスクロール位置 / 最大の仮想の上端）。`rowLayout.ts` の各関数の入出力に画面の高さを加える。「U3 のテストをそのまま通す」を残すなら、その式で成り立つことを確かめる。末尾の行に届くこと（100万件で End）を `rowLayout.test.ts` の観点に加える。 | Unresolved |
| R-13 | Minor | functional-spec.md > §6 BR1.10、BR4.4、frontend-components.md > §4・§5 | スクロールする展開部分の入力位置の規則が食い違う。BR1.10 は「スクロールするときは展開部分に入力位置を受けさせる（tabIndex=0）」とする一方、「選んでいる行の展開部分がスクロールするときは Tab でその中に入る」とも書く。前者のままだと選んでいない行の展開部分も Tab の順に入り、BR4.4 の「ログ一覧（→ 選んでいる行のスクロールする展開部分）」と合わない。role=grid を付ける要素と入力位置を受ける要素が同じか、見出し行をその中に含めるか（aria-rowindex=1 を成り立たせるため）も定まっていない。 | tabIndex=0 は「選んでいる行の、スクロールする展開部分」だけに付け、ほかは -1 にする。role=grid と tabIndex=0 を付ける要素を 1 つに決め、見出し行をその中に入れるかを書く。 | Unresolved |
| R-14 | Minor | functional-spec.md > §5 展開している行、BR1.9、§8 / inception/domain-design/components.md SessionState.expandedRows | (1) 展開の集合を components.md の SessionState.expandedRows から画面の中へ移すことは §5・§8 に理由つきで書かれているが、共有契約の components.md は更新されず食い違ったまま残る。(2) BR1.9 の見積もりは等幅の文字 1 つの幅で数えるため、全角文字が多いログでは行数を少なく見積もる（実測で直るので害は小さい）。 | (1) 変更を「意図した契約の変更」として前提に明記し、Code Generation／Build and Test の確認項目に入れる。(2) 全角文字を 2 倍で数えるか、見積もりの限界として §8 に書く。 | Unresolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この段階で割り当てられた検証ツールは dispatch に無い） | 実行せず | 代わりに traceability.json の upstream_ids と coverage、reverse の BR 参照を functional-spec.md と照合し、整合を確認した。R-11・R-12 は成果物の本文（§5、BR1.4、BR1.6、BR1.7）の記述そのものから確認した。作業単位のコードは既にコード生成で変わっており、設計の評価には使っていない。他の作業単位の設計ファイルは開いていない。 |

### Summary

前回と同じ結論である。展開・エラー・終了確認の設計は実装できる形にほぼ達しているが、discardGeneration の持ち主が §5 と BR1.7 で矛盾していること（R-11、R-03 も未解消）と、縮小の式が最大のスクロール位置を考慮せず 100万件で末尾に届かないこと（R-12）が Major として残り、Major が 3 件（R-03、R-11、R-12）のため NOT-READY とする。修正は小さい（持ち主を LogView に一本化、U3 の正規化にそろえる）。レビュー回数の上限に達しているため、直すかどうかは運用ルール（project.md の Corrections）に従い人間が判断する。
