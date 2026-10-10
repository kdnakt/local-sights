## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-08T14:52:17Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.1、BR2.2、BR2.4、BR1.5 / entities.md > FilterResult | 反復 1 の指摘（走査と、ページの途中差し込み・破棄の整合）。結果を行のキー（matchedKeys）で持ち、scanCursor・timelineEpoch・filterId による古い結果の破棄を入れた。ページが scanCursor をまたぐ場合は、cursor 以下のキーだけをその場で判定し、それより大きいキーは走査に任せるため、取りこぼしも重複もない。キー（timestamp、logStreamName、sequence）は一意で差し込みでずれない。成果物は前回から変わっておらず、設計として解消した状態が保たれている。実装上の残りは R-02 と R-07 で扱う | なし（残りは R-02、R-07） | Resolved |
| R-02 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.5、BR2.1、BR2.2 / 既存コード crates/local-sights-core/src/coordinator.rs の TimelineStore（add・discard）と src-tauri/src/lib.rs | BR2.5 は「ページの追加と逐次の判定を、保持ログと絞り込み結果の両方のロックを持ったまま続けて行う」と定めるが、既存の追加・破棄の経路（取得タスクが保持ログのロックだけで add / discard し、そのあとで on_batch がセッションのロックを取る）とは合わない。この隙間に走査の 1 回分が入ると、追加済みの行まで走査して scanCursor を進めたあと、on_batch が同じ行を key <= scanCursor として再判定し、matchedKeys に同じキーが二重に入る。破棄でも、保持ログが空になってから timelineEpoch が進むまでの間に古い結果が読まれうる。設計は TimelineStore の入れ替えが必要なことを書いていない。成果物は前回から変わっておらず、未解消のまま。project.md の方針（レビュー回数の上限後の指摘はその単位で直さず次へ回す）に従い、この最終反復では設計を変えず、コード生成への申し送りとする | 追加・破棄の経路を、セッション → 保持ログ → 絞り込み結果の順のロックの中で、追加または破棄と判定または世代更新を一続きにする形に変えること、および matchedKeys への挿入はキーが既にあれば何もしない（冪等）ことを、コード生成で対応する。申し送りとして記録する | Unresolved |
| R-03 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.6 / entities.md > FilterResult.resultVersion、SessionState.filterSummary | resultVersion、filterSummary（session-changed と fetch-progress の両方）、取得と関係なく結果が変わったときの軽い知らせ、版が古い行の破棄がそろい、件数が同じでも取り寄せ直す経路が定まっている。再確認でも抜けは見つからない | なし | Resolved |
| R-04 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.2 | 絞り込みの変更・解除で一番上に戻ることが、意図した割り切りとして BR3.2 に書かれている | なし | Resolved |
| R-05 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/entities.md > FilterResult.allCount、FilterCondition | allCount への名前の変更と、空の filterText は条件なしと同じ（components.md の 0..1）という扱いが entities.md・rules.md・functional-spec.md でそろっている。旧名（matchedPositions、totalCount）の取り残しはない（RowWindow.totalCount は U3 の既存属性で意味の説明が付いている） | なし | Resolved |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR2.1 | AppSession が届いたページの行を FilterEngine に渡し、更新はページの行数と合う行数に比例するとした。渡し方の意味は定まっている。ロック順の実装可能性は R-02 に集約 | なし | Resolved |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR1.5、BR2.1 / entities.md > FilterResult.scanCursor | 走査の再開位置の決め方が書かれていない。走査の合間に cursor より小さいキーの行が差し込まれると位置（添字）がずれるため、再開は「scanCursor より大きい最初のキー」を二分探索で見つける形が必要。また、BR1.5 は開始時の Filtering を「scanCursor なし」とするが、entities.md は「Filtering の間だけ持つ」とし、BR2.1 は cursor 以下だけ判定するため、開始直後（まだ走査していない）の cursor なしの読み方が未定義。成果物は前回から変わっておらず未解消 | 走査の再開をキーによる二分探索で行うこと、Filtering で scanCursor が無いときは「何も走査していない（どの行も判定しない）」と読むことを、コード生成で対応する（次の単位への申し送り） | Unresolved |
| R-08 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/rules.md > BR3.2 | 絞り込み中の位置の問い合わせ（U3:BR4.4）の入力は（ストリーム名、sequence）だが、matchedKeys の二分探索には timestamp を含むキーが要る。保持ログの索引から引けば実装はできるが、設計は触れておらず、matchedKeys に無いキーを問い合わせたときの戻り値も未定義。成果物は前回から変わっておらず未解消 | timestamp は保持ログの索引から引くこと、matchedKeys に無ければ「位置なし」を返すことを、コード生成で対応する（次の単位への申し送り） | Unresolved |
| R-09 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u5-filter/functional-design/entities.md > FilterResult.matchedKeys | キーにストリーム名の文字列を含めると、合う行が 100 万件に近いときに文字列の複製でメモリが膨らむ。上限の見積りがない。成果物は前回から変わっておらず未解消。ブロックしない | キーの持ち方（ストリームの識別番号や共有参照にする等）をコード生成で選ぶ前提として一言残す | Unresolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| ID の整合（rules.md の BR ID と traceability.json、entities.md、functional-spec.md の参照の照合） | PASS | BR1.1〜BR3.6 は functional-spec.md の要約表に載り、traceability.json の coverage（FR6〜FR6.5、NFR1、NFR2、NFR12、NFR13）と reverse（BR1.3、BR1.4、BR3.2、BR3.5、BR2.4、BR2.5）に割り当てられている。upstream_ids の FR6.1〜FR6.5、NFR1、NFR2 は requirements.md に実在し、FilterEngine・FilterCondition・FilterResult は components.md と unit-of-work.md の U5 の範囲と矛盾しない（matchedPositions → matchedKeys の変更は entities.md に理由付きで記載） |
| 上流契約との照合（unit-of-work.md の U5、components.md、requirements.md） | PASS | FilterEngine → EventTimeline の依存の向き、AppSession → FilterEngine の呼び出し、SessionState が条件を 0 か 1 つ参照する点（空の文字列＝条件なし）は一貫している。循環依存はない |
| 前回記録された既存コードとの照合（coordinator.rs、timeline.rs、src-tauri/src/lib.rs） | 差異あり（今回は再実行せず前回の記録を引き継ぐ） | 追加・破棄の経路が BR2.5 のロック順と合わない点は R-02 のとおり。この単位の読み取り範囲を超えるため、今回の判断の根拠は設計文書の内部整合に置いた |

### Summary

成果物は前回のレビューから変わっておらず、再実行でも結論は同じ。Critical はなく、Major は R-02 の 1 件（追加・破棄の既存経路と BR2.5 のロック順の食い違い）だけで、2 件以下のため READY とする。R-02 と R-07 から R-09 は、project.md の方針（レビュー回数の上限後の指摘はその単位で直さず次へ回す）に従い、コード生成への申し送りとして残す。
