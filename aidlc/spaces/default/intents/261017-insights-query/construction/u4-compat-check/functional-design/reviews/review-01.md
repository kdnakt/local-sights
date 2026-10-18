## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T08:23:34Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/rules.md > BR2.5、BR2.3、およびfunctional-spec.md > 3. クエリの組 q09-sort-natural・q10-sort-missing（q01・q02 も同じ） | BR2.5 は「sort を書いたクエリは並びが決まるため、件数の上限に当たっても比べる」としているが、sort の値が同じ行が上限の境目にまたがると、どの行が残るかは決まらない。q09 は `sort eventName asc \| limit 200` で、CloudTrail では同じ eventName が数百〜数千件あるのが普通なので、先頭 200 件の中身は Logs Insights とアプリで別になりうる。q10（`sort errorCode desc \| limit 50`、errorCode がない行は最小値として並ぶ）も同様。q01・q02 も同じミリ秒のイベントが境目にあれば起きる。BR2.3 の「同順位は違いとみなさない」は並びの比べだけで、BR2.1 の行の集まりの比べには効かないため、アプリの誤りではない Different が出て、BR2.6 の終了コード 1 が常に出る恐れがある | sort と limit（または既定の 10,000 件）で切られたクエリの扱いを決め直す。例：境目の sort の値と同じ値を持つ行は集まりの比べから外す。または、境目の値が同じ行がある場合は Skipped とし、理由を報告する。クエリの組では、q09・q10 の sort の項目に一意に近い項目（@timestamp など）を足すか、limit を境目で値が変わる大きさにする | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/rules.md > BR2.4、BR2.6、entities.md > QueryComparison.verdict、InsightsExport.status | InsightsExport.status は Complete・Failed・Cancelled・Timeout・Unknown・Error を取りうるが、BR2.4 が扱うのは Error（start-query が断った）だけ。Failed・Cancelled・Timeout・Unknown のときの verdict が決まっていない。また、書き出しのファイルがないクエリは Skipped とされ、BR2.6 では Skipped は失敗にならない。そのため、書き出しの失敗や欠けが続くと、ほぼ全部が Skipped でも終了コード 0（互換）で終わり、SM1 を確かめていないのに通ったように見える | status が Complete 以外のときの扱いを決める。例：Failed・Cancelled・Timeout・Unknown は動かせなかった（終了コード 2）とする。書き出しのファイルがないことも同様にする。さらに、Skipped の割合や、実際に比べた本数が 0 のときは成功にしない決まりを BR2.6 に足す | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/rules.md > BR1.3、entities.md > RunSettings.startSeconds・endSeconds、functional-spec.md > UC1・UC2 | スクリプトは秒単位の startSeconds・endSeconds を start-query に渡し、確認用プログラムはそれを同じ期間としてアプリの取得（終了日時はその秒の 999 ミリ秒まで含む、FR3.5）に渡す。Logs Insights が終わりの秒をどこまで含めるか、期間の始まりと終わりの境目のイベントの扱いが、両側で同じになることが書かれていない。30 分の余裕（BR1.3）は遅れて届くログの対策で、境目の違いは防がない | 期間の境目（始まり・終わりの秒の扱い）を機能設計に 1 文で決める。決められなければ、境目の 1 秒に当たるイベントで Different が出たときの見立ての手順を UC3 に足す | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/functional-spec.md > 3. クエリの組 p09-json-number、rules.md > BR2.2 | p09 は「JSON の数と文字列の数」を確かめる目的だが、CloudTrail の eventVersion は文字列なので、`eventVersion = "1.08"` は JSON の数の比べを確かめない。U2 の R-01（JSON の数と "200" の比べ）は CloudTrail のロググループでは実際には確かめられないままになる。BR2.2 の「JSON の数は書かれた文字列のまま」も、Logs Insights が数をその形で返すか確かめる組がない | CloudTrail で JSON の数になる項目を探してクエリに入れるか、確かめられないことを BR3.3 の記録と README に残す。p09 の目的の名前も実態に合わせて直す | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/rules.md > BR1.2、BR1.4 | BR1.2 は「リポジトリの最上位のフォルダ」を基準にするが、確認用プログラムがそれをどう求めるか（git の最上位か、ビルド時の場所か）と、Windows での場所の比べ方（canonicalize の `\?` 付きの形）が書かれていない。BR1.4 の「違う条件を渡したらエラー」も、UC2 の確認用プログラムは `--export-dir` しか受け取らないので、実際に起こる場面がない | リポジトリの最上位の求め方を決める。BR1.4 は「条件は settings.json からだけ取る」と言い換え、実行時に条件を渡せないことを明記する | New |
| R-06 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/functional-design/rules.md > BR2.1、BR2.2 | 選んだ列のすべてが空のイベントを、Logs Insights がどう返すか（@ptr だけの行か、行が出ないか）と、アプリの空の行をどう合わせるかが書かれていない。q10 のように多くの行で列が空になるクエリで、件数の違いの原因になりうる | 全部の列が空の行の扱いを決める。例：@ptr だけの行は、全部空の行として数える | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS：gaps、orphans、missing_from_table、invalid_entries、invalid_targets がすべて 0 | 上流の ID（SM1、NFR3、NFR4、OQ5 など）との対応は取れている。上の指摘は対応の欠けではなく、決まりの中身の問題 |

### Summary

構成（CLI のスクリプトで書き出し、例のプログラムでアプリの取得と評価を使って比べる）は、NFR4 と Forbidden を守っていて筋が通っている。ただし、sort と limit の境目の同順位（q09・q10）と、書き出しの失敗が通ってしまう終了コードの 2 点は、手元で流したときに誤った Different や誤った成功を出すため、承認の前に見直すのがよい。
