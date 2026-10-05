## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-05T23:37:46Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.1、BR2.2、BR2.4、BR1.5 / entities.md > FilterResult | 位置を行のキー（matchedKeys）に変え、scanCursor・timelineEpoch・filterId による古い結果の破棄を入れた。ページが scanCursor をまたぐ場合は行ごとにキーで分けるため、cursor 以下は即判定、それより大きいものは走査に任せる形で取りこぼしも重複もしない。キー（timestamp、stream、sequence）は一意なので、同じキーの行の扱いは問題にならない。設計の意図としては解消した。ただし実装可能にするための残りは R-02（ページ追加のロック）と R-07（走査の再開）に分けて扱う | なし（残りは R-02、R-07） | Resolved |
| R-02 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.5、BR2.1、BR2.2 / 既存コード crates/local-sights-core/src/coordinator.rs の TimelineStore（add・discard）と src-tauri/src/lib.rs | BR2.5 は「ページの追加と逐次の判定を、保持ログと絞り込み結果の両方のロックを持ったまま続けて行う」と定めたが、既存コードでは取得タスクが TimelineStore::add で保持ログのロックだけを取って追加し、そのあとで sink.on_batch がセッションのロックを取る。discard も取得開始時に同じ形で行う（lib.rs の「セッション → 保持ログ」の順は get_rows などの読み側だけ）。この隙間に走査の 1 回分が入ると、追加済みのページの行まで走査して scanCursor を進めたあと、on_batch が同じ行を key <= scanCursor として再判定し、matchedKeys に同じキーが二重に入る（matchedCount の水増し）。破棄でも、保持ログが空になってから timelineEpoch が進むまでの間に古い結果が読まれる。設計は TimelineStore の入れ替えが必要なことを書いておらず、このままでは BR2.5 は守れない | 追加・破棄の経路（TimelineStore の実装、または on_batch の中での追加）を、セッション → 保持ログ → 絞り込み結果の順のロックの中で、追加または破棄と判定または世代更新を一続きにする形に変えることを、BR2.5 か functional-spec に明記する。あわせて、matchedKeys への挿入はキーが既にあれば何もしない（冪等）と定め、取りこぼしや二重を防ぐ最後の守りにする。この最終反復ではレビュー済みの内容を変えず、コード生成で対応する（project.md の方針）ため、次の単位への申し送りとして記録する | Unresolved |
| R-03 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.6 / entities.md > FilterResult.resultVersion、SessionState.filterSummary | resultVersion、filterSummary（session-changed と fetch-progress の両方）、走査の進みや Ready への変化での軽い知らせ、版が古い行の破棄が入り、件数が同じでも取り寄せ直す経路がそろった。走査 1 回分ごとの知らせは 100 万件でも数百回程度で負荷にならない | なし | Resolved |
| R-04 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.2 | 絞り込みの変更・解除で一番上に戻ることを、意図した割り切りとして書いた | なし | Resolved |
| R-05 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/entities.md > FilterResult.allCount、FilterCondition | allCount への名前の変更と、空の filterText は条件なしと同じ（components.md の 0..1）という扱いが entities.md・rules.md・functional-spec.md でそろっている。この文書内に matchedPositions と totalCount の旧名の取り残しはない（RowWindow.totalCount は U3 の既存属性で、意味の説明が付いている） | なし | Resolved |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.1 | AppSession が届いたページの行を FilterEngine に渡し、更新はページの行数と合う行数に比例するとした。渡し方の意味は整った。ロック順の実装可能性は R-02 に集約した | なし | Resolved |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR1.5、BR2.1 / entities.md > FilterResult.scanCursor | 走査の再開位置の決め方が書かれていない。走査の合間に、cursor より小さいキーの行（別ストリームのページ）が保持ログに差し込まれて位置（添字）がずれるため、再開は「scanCursor より大きい最初のキー」を二分探索で見つける形でなければ、行を飛ばしたり二重に見たりする。また、BR1.5 は開始時の Filtering を「scanCursor なし」とするが、entities.md は「Filtering の間だけ持つ」とし、BR2.1 は cursor 以下だけ判定するため、開始直後（まだ 1 回も走査していない）と Ready を、cursor が無い状態で区別できない | 走査の再開はキーによる二分探索で行うこと、Filtering で scanCursor が無いときは「何も走査していない（どの行も判定しない）」と読むことを、BR1.5 と entities.md に一文で足す。コード生成で対応する | New |
| R-08 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.2 | 絞り込み中の位置の問い合わせ（U3:BR4.4）は入力が（ストリーム名、sequence）だが、matchedKeys の二分探索には timestamp を含むキーが要る。既存の EventTimeline には、ストリームと sequence から timestamp を引く索引があるので実装はできるが、設計は触れていない。キーが matchedKeys にない場合（絞り込み中に該当しない行）の戻り値も未定義 | BR3.2 に、timestamp は保持ログの索引から引くこと、matchedKeys に無ければ「位置なし」を返すことを足す。コード生成で対応する | New |
| R-09 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/entities.md > FilterResult.matchedKeys | キーに stream 名の文字列を含めると、合う行が 100 万件に近いときに文字列の複製でメモリが膨らむ。上限の見積りがない | キーの持ち方（保持ログへの添字を使わずに、ストリームの識別番号や共有参照にする等）を、コード生成で選ぶ前提として一言残す。ブロックしない | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| ID の整合（rules.md の BR ID と traceability.json、entities.md、functional-spec.md の参照の照合） | PASS | BR1.1〜BR3.6 は functional-spec.md の要約表に載り、traceability.json の forward（FR6〜FR6.5、NFR1、NFR2、NFR12、NFR13）と reverse（BR1.3、BR1.4、BR3.2、BR3.5、BR2.4、BR2.5）に割り当てられている。BR2.3、BR3.6 は forward 側に載っている |
| 既存コードとの照合（timeline.rs、coordinator.rs、src-tauri/src/lib.rs） | 差異あり | EventTimeline のキーは一意で、追加はキー順の挿入のため設計の前提と合う。一方、追加・破棄の経路は保持ログのロックだけを取る形で、BR2.5 とは合わない（R-02） |

### Summary

反復 1 の R-01（Critical）は設計として解消し、R-03 から R-06 も解消した。残る Major は R-02 の 1 件（追加・破棄の既存経路が BR2.5 のロックの順と合わず、走査との競合で二重登録が起こりうる）だけで、Critical は無く Major は 2 件以下のため READY とする。R-02、R-07 から R-09 は、project.md の方針どおりこの単位では直さず、コード生成の申し送りとする。

READY
