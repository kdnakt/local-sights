# Decision Log — Logs Insights 互換のクエリ（IB-11）

アイデア整理フェーズで決めたことの記録。上流：`ideation/intent-capture/intent-statement.md`（intent-statement）、`ideation/intent-capture/stakeholder-map.md`（stakeholder-map）、`ideation/scope-definition/scope-document.md`（scope-document）、`ideation/scope-definition/intent-backlog.md`（intent-backlog）。

## 意図の把握（Intent Capture）

| ID | 決めたこと | 出典 |
|----|------------|------|
| D1 | いちばんの困りごとは、絞り込みで条件を組み合わせられないこと | intent-statement（意図 Q1） |
| D2 | 利用者は開発者本人・同じ会社の同僚・OSS の利用者 | intent-statement、stakeholder-map（意図 Q2） |
| D3 | 成功の目安は、コンソールの Logs Insights で書いていたクエリが、対応する範囲ではそのまま書けて同じ結果になること | intent-statement（意図 Q3） |
| D4 | 範囲と優先順位は開発者本人が決め、進捗の共有は PR と README 程度とする | stakeholder-map（意図 Q5） |
| D5 | 命令は `fields`・`filter`・`sort`・`limit` を含め、集計（`stats`）は次の版以降とする | intent-statement（意図 Q6・Q9） |
| D6 | 今の部分一致の絞り込みは残し、クエリは別に書けるようにする | intent-statement（意図 Q7） |
| D7 | クエリの保存・履歴と、結果の書き出しは作らない | intent-statement（意図 Q8） |
| D8 | クエリの中で期間とロググループを指定でき、足りない分だけ追加で取得してから手元で実行する | intent-statement（意図 Q10・Q11） |

## 範囲の定義（Scope Definition）

| ID | 決めたこと | 出典 |
|----|------------|------|
| D9 | 使える項目は、決まった項目（`@timestamp`・`@message`・`@logStream`・`@log`）と JSON の項目（入れ子は `a.b`） | scope-document の B3 |
| D10 | ロググループは複数指定できる | scope-document の B5 |
| D11 | B1〜B7 を Must、B8〜B10（補完・色分け・例のクエリ）を Should とし、時間が足りなければ Should を次の版に回してよい | scope-document、intent-backlog |
| D12 | クエリの書き方の説明は書かず、対応する範囲の一覧（B6）だけを README に書く | scope-document の「範囲外」 |
| D13 | 期限はなく、この作業も `v0.2.0` に含めて、終わるまでタグを待つ | scope-document の「期限と配る版」 |
| D14 | 進める順は価値が先で、最初の一本は取得済みのログに対する `filter`（B1）の実行と表示まで | scope-document の「進める順」、intent-backlog |

## 承認と引き継ぎ（Approval & Handoff）

| ID | 決めたこと | 出典 |
|----|------------|------|
| D15 | 速さの基準が未確認であること、追加の取得で待ちが延びること、`v0.2.0` の配布が遅れることを、対策込みで受け入れる | approval-handoff-questions Q1 |
| D16 | コンソールとの互換の確かめ方が未決のままでは進めず、この段で決める | approval-handoff-questions Q1a |
| D17 | 互換を確かめるクエリの組は、AWS の資料にあるクエリの例のうち対応する範囲のものとする | approval-handoff-questions Q4 |
| D18 | 「同じ結果」は、結果に出る行の集まりが同じこと。並び順は `sort` を書いたときだけ比べる | approval-handoff-questions Q5 |
| D19 | コンソールで書き出した結果とアプリの結果を、手元の確認用プログラムで自動で比べる（自動テストと CI からは AWS に接続しない） | approval-handoff-questions Q6 |
| D20 | team.md の Deployment の `v0.2.0` の記述に、この作業を含めることを書き足す（この段の「次のために覚えておくこと」で行う） | approval-handoff-questions Q2 |
| D21 | Go：次のフェーズに進める | approval-handoff-questions Q3 |

## Assumptions & Open Questions

None.
