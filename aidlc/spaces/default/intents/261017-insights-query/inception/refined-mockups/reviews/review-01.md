## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-10-17T11:53:38Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/design-system-mapping.md > CodeEditor について、および interaction-spec.md > QueryEditor | クエリ欄を Cloudscape の `CodeEditor` で作る前提が、次の点で確かめられていない。(1) 独自の言語「Logs Insights のクエリ」を載せられるか（`CodeEditor` の言語指定は決まった言語の一覧から選ぶ作りのはずで、独自の Ace モードを登録できるか未確認）。(2) 設定画面を出さない（`preferences` を固定する）ことが標準部品のままでできるか。(3) エディタの中で Tab を字下げに使わず次の部品へ移す動きが設定できるか。(4) Ace を Tauri の画面（CSP やワーカーの読み込み）で動かせるか。Q3 の B は「色分けと補完をそのまま載せられる」を理由に選ばれており、この前提が崩れると画面 2・7・8 と NFR10・NFR11 の書き方が変わる。 | 作る段階の前に、上の 4 点が標準の `CodeEditor` で成り立つかを確かめるか、確かめるべき前提として明記する。成り立たないときの逃げ道（例：等幅の `Textarea` ＋ 直下のエラー文だけに下げ、色分け・補完・欄内の印は Should として次の版に回す）を design-system-mapping.md に 1 段落で書く。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 画面 2、画面 3、interaction-spec.md > LogGroupMultiselect と 2 つの画面の間の約束 | 接続（プロファイル・リージョン）が未選択のときのクエリの画面が決まっていない。FR1.6 の最初の条件は「接続が選ばれている」で、アプリは起動時に何も選ばない（プロジェクトの既定の決まり）。この状態でロググループの欄・期間・[Run query] がどう見えるか、理由をどこに出すかが画面 2・3 にない。また A2 は「接続を変えたときは Log events と同じ」とするが、クエリの画面に結果・入力済みのクエリがあるとき確認ダイアログを出すか、実行中に接続を変えたらどうなるかが書かれていない（選択を空に戻す、とだけある）。 | 画面 3 に「接続未選択」の状態（欄を無効にして理由を文字で出す、など）を足す。接続変更時の扱いを interaction-spec.md に書く：確認ダイアログの有無と条件、実行中は変更不可か中止してから変えるか、結果・期間・クエリの文を残すか消すか。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/interaction-spec.md > 状態の移り変わり（前の結果の扱い）と mockups.md > 画面 4 | 前の結果の扱いが食い違う。interaction-spec は Fetching・Evaluating の間は前の結果を残すとするが、画面 4 の結果の囲みは StatusIndicator だけで前の結果が見えない。QueryError では前の結果を残す一方、FetchFailed では「違う条件の結果を残すと取り違える」として消しており、QueryError は文が書き直されたあとも古い結果が表示され続けて FR7.6（最後に実行したクエリの結果だけを出す）と筋が通らない。 | 前の結果を (a) 実行中・QueryError でも残す（どの条件の結果かを見出しに添える）、(b) 実行を始めた時点で消す、のどちらかに決めて、画面 4 の図と状態の表の両方をそろえる。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 追跡、interaction-spec.md > 文言表 | 追跡表に FR3〜FR5（命令・項目・演算子の画面上の見え方）、FR7.5、FR7.6、FR8、SM2 の行がない。また対応外の関数・演算子（FR5.5）を書いたときの文は「使える命令：…」だけで、関数・演算子の一覧への案内がない。文言表にも、画面 6b の本文、「Show failed streams」、「Couldn't load log groups」/「Retry」、「No log groups」、トークンを外すラベル、ステータス行の「scanned」の文、画面切り替えの読み上げ文がなく、NFR9（英日両方）を満たす材料が足りない。 | 追跡表に抜けている要件を足し、UI に関わらないものは「画面なし」と書く。対応外の関数・演算子のときの案内文を決める。文言表に上の文言を足して英日の対を持たせる。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 画面 8（例のクエリ） | [Examples] は「欄に何か書いてあるとき」に置き換えの確認を出すが、初回は例のクエリが入っているため、手を付けていない既定のクエリでも毎回確認が出る。 | 確認を出す条件を「既定・直前に選んだ例から変えている」ときに限るか、既定のままなら黙って置き換えると書く。 | New |

### Summary

質問票の回答 Q1〜Q5 は画面・部品・文言・アクセシビリティに一貫して反映され、主な流れ・エラー・空・実行中・一部失敗・中止の状態もそろっているため、実装に進める水準である。ただし、`CodeEditor` に載せる前提（独自の言語・設定画面・Tab の動き・Ace の読み込み）の確認と、接続未選択・接続変更時の状態の決めは、承認の前に人間が重みを判断してほしい（どちらも逃げ道を書いておけば手戻りは小さい）。
