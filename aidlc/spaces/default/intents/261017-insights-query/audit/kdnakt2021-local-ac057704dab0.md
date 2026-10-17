# AI-DLC Audit Log

## Decision Recorded
**Timestamp**: 2026-10-17T04:24:25Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q1. クエリで使える項目（フィールド）は、最初の版でどこまでにしますか？
**Options**: A. 決まった項目だけ,B. A に加えて JSON の項目を自動で取り出す,C. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T04:42:58Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["B. JSON の項目も使える"]

---

## Question Replied
**Timestamp**: 2026-10-17T04:42:58Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q1. クエリで使える項目（フィールド）は、最初の版でどこまでにしますか？\n\nLogs Insights では @timestamp・@message・@logStream・@log といった決まった項目に加えて、JSON 形式のログから項目（例：level、status）を自動で取り出して filter や sort に使えます。コンソールとの互換をどこまで求めるかで、作る範囲が大きく変わります。
**Reply**: B. JSON の項目も使える

---

## Question Answered
**Timestamp**: 2026-10-17T04:43:09Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q1: B
**Picker Note**: Not what the person picked in the picker ("B. JSON の項目も使える").

---

## Decision Recorded
**Timestamp**: 2026-10-17T04:43:10Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q2. 1 回のクエリで対象にするロググループは、いくつまでにしますか？
**Options**: A. 1 つだけ,B. 複数,C. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T04:56:16Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["B. 複数"]

---

## Question Replied
**Timestamp**: 2026-10-17T04:56:17Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q2. 1 回のクエリで対象にするロググループは、いくつまでにしますか？\n\n意図の把握（Q11）で、クエリの中でロググループを指定できると決めました。今の画面はロググループを 1 つ選んで取得します。Logs Insights では 1 回のクエリで複数のロググループを対象にできます。
**Reply**: B. 複数

---

## Question Answered
**Timestamp**: 2026-10-17T04:56:26Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q2: B
**Picker Note**: Not what the person picked in the picker ("B. 複数").

---

## Decision Recorded
**Timestamp**: 2026-10-17T04:56:27Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q3. 決まっている機能のうち、最初の版に欠かせないもの（Must）はどれですか？
**Options**: A. すべて Must,B. (1)〜(3) が Must、(4) は Should,C. (1) だけが Must,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T04:59:27Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["A. すべて Must"]

---

## Question Replied
**Timestamp**: 2026-10-17T04:59:27Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q3. 決まっている機能のうち、最初の版に欠かせないもの（Must）はどれですか？\n\n最初の版に入れる機能は、(1) 条件を組み合わせて絞り込む filter、(2) 表示する項目を選ぶ fields（JSON の項目を含む）、(3) 並べ替えの sort と件数の limit、(4) クエリの中で期間やロググループ（複数可）を指定し、足りない分だけ追加で取得すること、です。どれも入れる予定ですが、時間が足りないときに後回しにしてよいもの（Should）を決めておきます。
**Reply**: A. すべて Must

---

## Question Answered
**Timestamp**: 2026-10-17T04:59:33Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q3: A
**Picker Note**: Not what the person picked in the picker ("A. すべて Must").

---

## Decision Recorded
**Timestamp**: 2026-10-17T04:59:33Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q4. どの順番で作りますか？
**Options**: A. 価値を先に,B. 危険を先に,C. 依存を先に,D. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:00:48Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["A. 価値を先に"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:00:49Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q4. どの順番で作りますか？\n\n最初の作業単位では、端から端まで通る薄い一本を作ります（team.md の Walking Skeleton）。何を最初の一本にするかで、早く確かめられる不安が変わります。
**Reply**: A. 価値を先に

---

## Question Answered
**Timestamp**: 2026-10-17T05:00:55Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q4: A
**Picker Note**: Not what the person picked in the picker ("A. 価値を先に").

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:00:55Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q5. 期限や、どの版で配るかの決まりはありますか？
**Options**: A. v0.2.0 を先に出し、この作業は v0.3.0,B. v0.2.0 に含める,C. 期限がある,D. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:03:08Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["B. v0.2.0 に含める"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:03:09Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q5. 期限や、どの版で配るかの決まりはありますか？\n\nteam.md の Deployment では、次の版のタグ v0.2.0 は取得の高速化（PR #22）と Windows 版を合わせて配る版としていて、まだタグを付けていません。この作業をどの版に入れるかで、v0.2.0 を出す時期が変わります。
**Reply**: B. v0.2.0 に含める

---

## Question Answered
**Timestamp**: 2026-10-17T05:03:16Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q5: B
**Picker Note**: Not what the person picked in the picker ("B. v0.2.0 に含める").

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:03:17Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q6. クエリを書くための手助けのうち、最初の版に含めないものはどれですか？（複数選択）
**Options**: A. 入力の補完,B. 構文の色分け,C. クエリの書き方の説明,D. 例のクエリ,E. None,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:04:04Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["C. 書き方の説明"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:04:04Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q6. クエリを書くための手助けのうち、最初の版に含めないものはどれですか？（複数選べます）\n\nコンソールの Logs Insights には、入力の補完や構文の色分けがあります。選ばなかった項目は、含めるかどうかを後で確かめます。どれも含める場合は「Type something」で「None」と書いてください。
**Reply**: C. 書き方の説明

---

## Question Answered
**Timestamp**: 2026-10-17T05:04:15Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q6: C
**Picker Note**: Not what the person picked in the picker ("C. 書き方の説明").

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:04:16Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q7. 入力の補完・構文の色分け・例のクエリは、最初の版に含めますか？（複数選択）
**Options**: A. 入力の補完,B. 構文の色分け,C. 例のクエリ,D. None,E. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:05:18Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["A. 入力の補完, B. 構文の色分け, C. 例のクエリ"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:05:19Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q7. 入力の補完・構文の色分け・例のクエリは、最初の版に含めますか？（含めるものを複数選べます）\n\nQ6 では、クエリの書き方の説明だけを「含めないもの」に選びました。選ばれなかった 3 つは、含めるとも含めないとも決めつけずに確かめます。要件分析で決める場合は「Type something」でそう書いてください。
**Reply**: A. 入力の補完, B. 構文の色分け, C. 例のクエリ

---

## Question Answered
**Timestamp**: 2026-10-17T05:05:38Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q7: A, B, C
**Picker Note**: Not what the person picked in the picker ("A. 入力の補完, B. 構文の色分け, C. 例のクエリ").

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:05:38Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q8. 対応する範囲（使える命令・関数・演算子）は、利用者にどう伝えますか？
**Options**: A. 対応範囲の一覧だけ README に書く,B. 画面のエラーで分かるようにする,C. A と B の両方,D. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:07:07Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["C. A と B の両方"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:07:07Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q8. 対応する範囲（使える命令・関数・演算子）は、利用者にどう伝えますか？（Q6 の確認）\n\nQ6 で、クエリの書き方の説明を手順書や README に書くことは含めないと決めました。一方、このアプリが対応するのは Logs Insights の一部です。書き方そのものは AWS の資料に任せるとしても、どこまで使えるかを利用者が知る手段を確かめます。
**Reply**: C. A と B の両方

---

## Question Answered
**Timestamp**: 2026-10-17T05:07:14Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q8: C
**Picker Note**: Not what the person picked in the picker ("C. A と B の両方").

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:07:41Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Q9. 書く手助けと対応範囲の伝え方は Must か Should か
**Options**: A. すべて Must,B. 伝え方は Must・手助けは Should,C. すべて Should,D. Not yet defined,X. Other

---

## Human Turn
**Timestamp**: 2026-10-17T05:08:21Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["B. 伝え方だけ Must"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:08:22Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: Q9. クエリを書く手助け（入力の補完・構文の色分け・例のクエリ）と、対応範囲の伝え方（README の一覧・画面のエラー）は、Must ですか、Should ですか？\n\nQ3 では filter・fields・sort／limit・クエリの中での期間とロググループの指定（追加の取得を含む）をすべて Must としました。Q7・Q8 で含めると決めた手助けと伝え方は、優先度をまだ聞いていません。時間が足りないときに後回しにしてよいかを確かめます。
**Reply**: B. 伝え方だけ Must

---

## Question Answered
**Timestamp**: 2026-10-17T05:08:27Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Q9: B
**Picker Note**: Not what the person picked in the picker ("B. 伝え方だけ Must").

---

## Artifact Updated
**Timestamp**: 2026-10-17T05:08:40Z
**Event**: ARTIFACT_UPDATED
**Tool**: Edit
**File**: <project-dir>/aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/memory.md
**Context**: ideation > scope-definition > memory.md

---

## Artifact Updated
**Timestamp**: 2026-10-17T05:08:42Z
**Event**: ARTIFACT_UPDATED
**Tool**: Edit
**File**: <project-dir>/aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/memory.md
**Context**: ideation > scope-definition > memory.md

---

## Artifact Created
**Timestamp**: 2026-10-17T05:09:15Z
**Event**: ARTIFACT_CREATED
**Tool**: Write
**File**: <project-dir>/aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-document.md
**Context**: ideation > scope-definition > scope-document.md

---

## Artifact Created
**Timestamp**: 2026-10-17T05:09:17Z
**Event**: ARTIFACT_CREATED
**Tool**: Write
**File**: <project-dir>/aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/intent-backlog.md
**Context**: ideation > scope-definition > intent-backlog.md

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:09:30Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Which notes to keep for next time?
**Options**: Keep none of these,c1,c2

---

## Decision Recorded
**Timestamp**: 2026-10-17T05:09:30Z
**Event**: DECISION_RECORDED
**Stage**: scope-definition
**Decision**: Anything to add for next time?
**Options**: Nothing to add,Add a note

---

## Human Turn
**Timestamp**: 2026-10-17T05:12:11Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["Keep none of these","Nothing to add"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:12:11Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: 次に役立ちそうなメモのうち、残すものを選んでください（複数選べます）。残したものは project.md の Corrections に書きます。チーム全体の決まりにしたい場合は「Type something」でそう書いてください。
**Reply**: Keep none of these

---

## Question Replied
**Timestamp**: 2026-10-17T05:12:11Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: ほかに、次のために残しておきたいことはありますか？
**Reply**: Nothing to add

---

## Question Answered
**Timestamp**: 2026-10-17T05:12:16Z
**Event**: QUESTION_ANSWERED
**Stage**: scope-definition
**Details**: Keep none of these; Nothing to add

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:16Z
**Event**: SENSOR_FIRED
**Fire id**: e7c86847
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-document.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:17Z
**Event**: SENSOR_PASSED
**Fire id**: e7c86847
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-document.md
**Duration ms**: 254

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:17Z
**Event**: SENSOR_FIRED
**Fire id**: d27cebfc
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/intent-backlog.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:17Z
**Event**: SENSOR_PASSED
**Fire id**: d27cebfc
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/intent-backlog.md
**Duration ms**: 246

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:18Z
**Event**: SENSOR_FIRED
**Fire id**: 5a1503cd
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-definition-questions.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:18Z
**Event**: SENSOR_PASSED
**Fire id**: 5a1503cd
**Sensor ID**: required-sections
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-definition-questions.md
**Duration ms**: 245

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:18Z
**Event**: SENSOR_FIRED
**Fire id**: 5977711c
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-document.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:18Z
**Event**: SENSOR_PASSED
**Fire id**: 5977711c
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-document.md
**Duration ms**: 249

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:19Z
**Event**: SENSOR_FIRED
**Fire id**: 5475de5a
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/intent-backlog.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:19Z
**Event**: SENSOR_PASSED
**Fire id**: 5475de5a
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/intent-backlog.md
**Duration ms**: 250

---

## Sensor Fired
**Timestamp**: 2026-10-17T05:12:19Z
**Event**: SENSOR_FIRED
**Fire id**: e5560663
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-definition-questions.md

---

## Sensor Passed
**Timestamp**: 2026-10-17T05:12:20Z
**Event**: SENSOR_PASSED
**Fire id**: e5560663
**Sensor ID**: upstream-coverage
**Stage slug**: scope-definition
**Output path**: aidlc/spaces/default/intents/261017-insights-query/ideation/scope-definition/scope-definition-questions.md
**Duration ms**: 252

---

## Stage Awaiting Approval
**Timestamp**: 2026-10-17T05:12:20Z
**Event**: STAGE_AWAITING_APPROVAL
**Stage**: scope-definition

---

## Human Turn
**Timestamp**: 2026-10-17T05:12:58Z
**Event**: HUMAN_TURN
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Picked**: ["承認するけど、ここからはクラウド環境で再開したい。変更をプッシュして欲しい。後引き継ぎ用のメッセージをお願い。"]

---

## Question Replied
**Timestamp**: 2026-10-17T05:12:58Z
**Event**: QUESTION_REPLIED
**Session**: cfee0cf0-fdc3-4933-89bc-ab5bbddd3e0e
**Question**: 範囲の定義ができました。どう進めますか？
**Reply**: 承認するけど、ここからはクラウド環境で再開したい。変更をプッシュして欲しい。後引き継ぎ用のメッセージをお願い。

---

## Gate Approved
**Timestamp**: 2026-10-17T05:13:05Z
**Event**: GATE_APPROVED
**Stage**: scope-definition
**User Input**: Approve
**Person Reply**: 承認するけど、ここからはクラウド環境で再開したい。変更をプッシュして欲しい。後引き継ぎ用のメッセージをお願い。

---

## Stage Completion
**Timestamp**: 2026-10-17T05:13:05Z
**Event**: STAGE_COMPLETED
**Stage**: scope-definition
**Validation Basis**: {"graphContract":"sha256:f507bca6811bab5a3fbe73663d1debe5d0de707829c0a8a0d3c77b97f91a29c7","inputs":[{"artifact":"intent-statement","contentHash":"sha256:b3a2a7fc17fa18e615bcd53d272d0c7c26b9001b26fed5f0b81b068f73ef61b8","instanceCount":1,"presentCount":1,"producer":"intent-capture","required":true,"structureHash":"sha256:d04cc45348373c989b64b74a7aace6151bb5ede10e689c0f5e6fa2041f14a2ac"}],"outputs":[{"artifact":"intent-backlog","contentHash":"sha256:89be5f9fc9dd25a85fc905ff4dd6575642e3b394f88531a5f95168b117581a6a","instanceCount":1,"presentCount":1,"producer":"scope-definition","required":true,"structureHash":"sha256:16ad43c81f25a7c9b4f42acc82a0ee8978b8fe9d79255ea991ba0c63e2df96b7"},{"artifact":"scope-definition-questions","contentHash":"sha256:4c768a48ad353210e609cb154030e4c9b122b220a377b3d8a3207038dfd6efbb","instanceCount":1,"presentCount":1,"producer":"scope-definition","required":true,"structureHash":"sha256:4b50e056126188feb47bba27862f6559c8fff27531ab9aad3553ad9957f7bb8b"},{"artifact":"scope-document","contentHash":"sha256:2c98163eed4f63c1294f737fd8b2310db9cd917fe73937f66a67647f9629a978","instanceCount":1,"presentCount":1,"producer":"scope-definition","required":true,"structureHash":"sha256:c6a97dede25241a12283fa2df32c31fda608978a32e5f996207c64366380c00a"}],"projectType":"brownfield","schema":3}
**Details**: Stage Scope Definition approved by gate
**Tokens In**: 90
**Tokens Out**: 33871
**Cache Read**: 8316187
**Cache Write**: 129467
**Cost USD**: 3.38
**By Model**: opus-5-5=3.38
**By Agent**: main=3.38
**Tokens By Model**: opus-5-5=90/33.9k/8.3M/129.5k
**Tokens By Agent**: main=90/33.9k/8.3M/129.5k

---

## Stage Start
**Timestamp**: 2026-10-17T05:13:05Z
**Event**: STAGE_STARTED
**Stage**: approval-handoff
**Agent**: aidlc-delivery-agent
**Answer Mode**: guide (reused from intent-capture)

---

## Workflow Parked
**Timestamp**: 2026-10-17T05:13:05Z
**Event**: WORKFLOW_PARKED
**Stage**: approval-handoff

---
