# Scope Definition Questions — Logs Insights 互換のクエリ（IB-11）

## Sources

- [desc] Initial description: 依頼文（`project-description.json`）。クエリは取得済みのログに対して手元で実行する前提、速さの基準（10 万件で 10 秒以内・100 万件で 100 秒以内）、macOS と Windows の両方で動くこと。
- [scope] Workflow-selected scope: `refactor` を元にした計画 `local-insights-query`。
- 意図の把握と整理の回答（`ideation/intent-capture/intent-capture-questions.md`）：Q1（条件の組み合わせ）、Q2（利用者は全員）、Q3（コンソールとの互換）、Q6（`fields`・`filter`・`sort`・`limit`）、Q7（今の絞り込みは残す）、Q8（保存・履歴と書き出しは含めない）、Q9（`stats` は次の版以降）、Q10・Q11（クエリの中で期間やロググループを指定し、足りない分だけ追加で取得する）。
- [memory:M1] `aidlc/spaces/default/memory/team.md#Deployment`：次の版のタグは `v0.2.0` とし、取得の高速化（PR #22）と Windows 版を合わせてこの版で配る。
- 261002-cwlogs-viewer の `ideation/scope-definition/intent-backlog.md`：IB-10（JSON 形式のログメッセージを整形して表示する、Could）、IB-11（Logs Insights 互換のクエリ）。

## Q1. クエリで使える項目（フィールド）は、最初の版でどこまでにしますか？

背景：Logs Insights では、`@timestamp`・`@message`・`@logStream`・`@log` といった決まった項目に加えて、JSON 形式のログから項目（例：`level`、`status`）を自動で取り出して `filter` や `sort` に使えます。コンソールとの互換（意図の把握 Q3）をどこまで求めるかで、作る範囲が大きく変わります。

A. 決まった項目だけ（`@timestamp`・`@message`・`@logStream`・`@log`）
B. A に加えて、JSON 形式のログの項目を自動で取り出して使える（入れ子は `a.b` の形で指定）
C. Not yet defined（要件分析で決める）
D. Not applicable
E. None
X. Other (please specify)

[Answer]: B

## Q2. 1 回のクエリで対象にするロググループは、いくつまでにしますか？

背景：意図の把握（Q11）で、クエリの中でロググループを指定できると決めました。今の画面はロググループを 1 つ選んで取得します。Logs Insights では 1 回のクエリで複数のロググループを対象にできます。

A. 1 つだけ（今の画面と同じ）
B. 複数（Logs Insights と同じように、いくつかのロググループをまとめて対象にする）
C. Not yet defined（要件分析で決める）
D. Not applicable
E. None
X. Other (please specify)

[Answer]: B

## Q3. 決まっている機能のうち、最初の版に欠かせないもの（Must）はどれですか？

背景：最初の版に入れる機能は、(1) 条件を組み合わせて絞り込む `filter`、(2) 表示する項目を選ぶ `fields`、(3) 並べ替えの `sort` と件数の `limit`、(4) クエリの中で期間やロググループを指定し、足りない分だけ追加で取得すること、です。どれも入れる予定ですが、時間が足りないときに後回しにしてよいもの（Should）を決めておきます。

A. すべて Must（どれが欠けても最初の版として出さない）
B. (1)〜(3) が Must、(4) の追加の取得は Should（間に合わなければ次の版に回す）
C. (1) だけが Must、ほかは Should
D. 上のいくつか（具体的に書いてください）
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q4. どの順番で作りますか？

背景：最初の作業単位では、端から端まで通る薄い一本を作ります（team.md の Walking Skeleton）。何を最初の一本にするかで、早く確かめられる不安が変わります。

A. 価値を先に：条件を組み合わせた `filter` を、取得済みのログに対して手元で実行して表示するところを最初の一本にする
B. 危険を先に：クエリの中で期間やロググループを指定し、足りない分だけ追加で取得してからクエリを実行するところを最初の一本にする
C. 依存を先に：クエリの書き方の読み取り（構文の解析）を全命令分そろえてから、画面につなぐ
D. Not yet defined（作業単位の分割で決める）
E. Not applicable
X. Other (please specify)

[Answer]: A

## Q5. 期限や、どの版で配るかの決まりはありますか？

背景：team.md の Deployment では、次の版のタグ `v0.2.0` は取得の高速化（PR #22）と Windows 版を合わせて配る版としていて、まだタグを付けていません。この作業をどの版に入れるかで、`v0.2.0` を出す時期が変わります。

A. `v0.2.0` は先に今のまま出し、この作業はその次の版（`v0.3.0`）で配る。期限はない
B. この作業も `v0.2.0` に含める（終わるまで `v0.2.0` のタグを待つ）
C. 期限がある（日付を書いてください）
D. Not yet defined
E. Not applicable
X. Other (please specify)

[Answer]: B

## Q6. クエリを書くための手助けのうち、最初の版に含めないものはどれですか？（select all that apply）

背景：コンソールの Logs Insights には、入力の補完や構文の色分けがあります。最初の版でどこまで用意するかを決めます。選ばなかった項目は、含めるかどうかを後で確かめます。

A. 入力の補完（命令や項目の名前の候補を出す）
B. 構文の色分け
C. クエリの書き方の説明（手順書や README への記載）
D. 例のクエリを選んで入れられる仕組み
E. None（どれも含める）
X. Other (please specify)

[Answer]: C

## Q7. 入力の補完・構文の色分け・例のクエリは、最初の版に含めますか？（Q6 で選ばれなかった項目の確認、select all that apply）

背景：Q6 では、クエリの書き方の説明（手順書や README への記載）だけを「含めないもの」に選びました。選ばれなかった 3 つは、含めるとも含めないとも決めつけずに確かめます。含めるものを選んでください。

A. 入力の補完（命令や項目の名前の候補を出す）を含める
B. 構文の色分けを含める
C. 例のクエリを選んで入れられる仕組みを含める
D. None（3 つとも最初の版には含めない）
E. Not yet defined（要件分析で決める）
X. Other (please specify)

[Answer]: A, B, C

## Q8. 対応する範囲（使える命令・関数・演算子）は、利用者にどう伝えますか？（Q6 の確認）

背景：Q6 で、クエリの書き方の説明を手順書や README に書くことは最初の版に含めないと決めました。一方、このアプリが対応するのは Logs Insights の一部（`fields`・`filter`・`sort`・`limit` と、要件分析で決める関数や演算子）です。書き方そのものは AWS の資料に任せるとしても、どこまで使えるかを利用者が知る手段を確かめます。

A. 対応する範囲の一覧だけは README に短く書く（書き方の説明は AWS の資料に任せる）
B. 文書には何も書かず、対応していない命令や関数を書いたときに、画面のエラーでそれと分かるようにする
C. A と B の両方
D. Not yet defined（要件分析で決める）
E. Not applicable
X. Other (please specify)

[Answer]: C

## Q9. クエリを書く手助け（入力の補完・構文の色分け・例のクエリ）と、対応範囲の伝え方（README の一覧・画面のエラー）は、Must ですか、Should ですか？（Q3・Q7・Q8 の確認）

背景：Q3 では、`filter`・`fields`・`sort`／`limit`・クエリの中での期間とロググループの指定（追加の取得を含む）をすべて Must としました。Q7・Q8 で最初の版に含めると決めた手助けと伝え方は、Q3 では優先度を聞いていません。時間が足りないときに後回しにしてよいかを確かめます。

A. すべて Must
B. 対応範囲の伝え方（README の一覧・画面のエラー）は Must、書く手助け（補完・色分け・例のクエリ）は Should
C. すべて Should
D. Not yet defined
E. Not applicable
X. Other (please specify)

[Answer]: B
