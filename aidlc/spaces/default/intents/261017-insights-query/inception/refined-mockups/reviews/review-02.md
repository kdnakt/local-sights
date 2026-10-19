## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-10-17T23:19:38Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/interaction-spec.md > LogGroupMultiselect（接続を変えたら…）、「2 つの画面の間の約束」 | 要件 A2 は、接続を変えたらクエリの画面の取得済みのログと結果も捨てると定めている。しかし設計に書いてあるのは、選んだロググループを空に戻すことだけである。結果と取得済みを捨てることも、Log Management にある「表示中のログがあるときだけ出す確認ダイアログ」を結果の表示中にも出すかどうかも、どこにも書かれていない。画面 1 の「接続を変えたときの扱いは今と同じ」は、共有をやめた今では、どちらの画面の何が捨てられるのかが読み取れない。 | 接続を変えたときの、クエリの画面での扱いを書く。結果の表と取得済みのログを捨てること、結果が出ている間に確認を出すか出さないか、取得中や実行中に接続を変えたときの扱いの 3 点を明示する。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 追跡 | 追跡の表に、今回の見直しの中心である FR2.1（共有しない）が載っていない。FR2.7、FR2.8、FR8 も載っていない。FR2.1 の受け入れ基準（ログの画面の表示中のログと絞り込みがそのまま残る）が、どの画面のどの振る舞いで確かめられるのかが追えない。 | 追跡に FR2.1 の行を足し、画面 4 の注記と「2 つの画面の間の約束」を指す。FR2.7、FR2.8、FR8 が画面に現れないのであれば、画面の対象外とその理由を 1 行で示す。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 画面 4 の注記（実行中に Log Management へ切り替えてもよい） | 実行中に Log Management へ移ると、ステータス行は Log Management の内容になる。バックグラウンドでクエリの取得や評価が動いていることが、どこにも出ない。共有しないため、同じログの取得が裏で走り、呼び出しの速さの上限も使う。それなのに、止められるのはクエリの画面に戻ったときだけである。 | 他方の画面で実行中であることを示す最低限の表示（ナビゲーション項目の印など）を設けるか、設けない理由を OQ4 の決定事項として Domain Design に引き継ぐと明記する。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/refined-mockups/mockups.md > 画面 4・画面 5 の状態の文言 | ディスクキャッシュが切のとき、ログの画面で見たばかりの期間でも、クエリの画面は「Fetching missing logs」から始まる（FR2.1 の受け入れ基準のとおり）。設計には、これが利用者の期待とずれる場面だという説明や、取得が二度目であることへの手がかりがない。 | 文言か手順書のどちらかに、取得済みを共有しないことを短く説明する一文を入れるか、入れないと決めた理由を記録する。 | New |

### Summary

共有の言及は、interaction-spec の「2 つの画面の間の約束」、mockups の画面 4 と補完の候補元（画面 8）、design-system-mapping の補完の候補元のいずれでも FR2.1 と矛盾せず、共有を前提とした記述は残っていなかった。ただし接続を変えたときのクエリの画面の扱い（A2）が未記述で、設計の抜けとして 1 点ある。承認時に判断してほしい。実装を止めるほどではなく、Domain Design か Functional Design で補える範囲である。
