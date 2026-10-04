## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T22:19:27Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/rules.md > BR1.2 と BR1.3、functional-spec.md > UC2 手順 2 | BR1.2 は「同じページの残りを含め、古いストリームより後ろは対象にしない」とするが、BR1.3 と FR4.2 と Q2 は「時刻を持たないストリームは含める」としている。打ち切りのあるページで、古いストリームの後ろに時刻なしストリームが並ぶと、BR1.2 がそれを捨てる。また DescribeLogStreams の LastEventTime 降順で lastEventTimestamp を持たないストリームがどこに並ぶかは、AWS の仕様として設計のどこにも根拠がない。末尾に並ぶ場合、作成直後で lastEventTimestamp がまだ入っていない最新のストリーム（受け入れ基準「時刻を持たないストリームにも GetLogEvents を呼ぶ」の対象）は、最初の古いストリームでの打ち切りの後ろに回り、一度も呼ばれない。Q2 の「列挙の途中で見つかった分」という言い回しだけでは、実装者はどちらの挙動か決められない | (1) BR1.2 の「それより後ろ」から時刻なしのストリームを除き、打ち切りのページ内でも時刻なしは BR1.3 で対象にすると書く。(2) 時刻なしのストリームの並び位置に関する前提（降順での位置は未確認で、打ち切りより後ろにあれば取得できない）を rules.md か functional-spec.md に明記する。(3) その前提を薄い一本の実 AWS 確認で確かめる項目に足すか、取りこぼしを避ける代替（例：時刻なしかつ作成時刻が範囲に近いものの扱い）をこの単位の範囲で決める | New |
| R-02 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/rules.md > BR6.7、functional-design-questions.md > 持ち越し分（U2 のコード生成のレビュー R-06） | 持ち越し項目は「同期のコマンドに戻すか、連番を付ける」ことを求めているが、BR6.7 は「受け取った順に処理する」と述べるだけで、順序を保証する仕組み（連番、古い結果の破棄、直列化のどれか）を定めていない。さらに「絞り込みとロググループの選択はほかの処理を待たず、接続の変更と取得の開始だけが保持ログの処理を待つ」と分けているため、接続の変更（待つ）の直後のロググループの選択（待たない）が先に処理される追い越しが規則の上で許される。U2 の R-06 が問題にした到着順の弱まりそのものが残る | BR6.7 に、順序を保つ具体的な決まり（例：画面からの操作に単調増加の連番を付け、古い連番の結果は捨てる、または接続の変更・選択・取得の開始を同じ直列の列で処理する）を書く。接続の変更とロググループの選択の追い越しが起きない形にし、受け入れの観点（接続変更→選択の連続操作）を functional-spec.md のテスト方針に足す | New |
| R-03 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/rules.md > BR2.1、BR3.3 | 再試行は要求ごとに最大約 31 秒（±20% で最大約 37 秒）待つ。通信断や持続的なスロットリングのとき、失敗するストリームごとにこの待ちが繰り返され、数百ストリームでは数時間かかりうる。利用者向けの中断の操作はなく、中断はウィンドウを閉じるときだけ。Q1 の回答（要求ごと 5 回）は満たしているが、全体の上限や連続失敗での打ち切りがない | 取得全体の連続失敗の上限（例：Network が連続して N ストリーム失敗したら残りを失敗扱いにして止める）か、全体の待ち時間の上限を足すか、受け入れたリスクとして記録する | New |
| R-04 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/inception/requirements-analysis/requirements.md > 未解決の事項（スロットリングの受け入れ基準を FR4.10 に足す）と functional-spec.md | 要件の未解決事項は、再試行を決めたときにスロットリングによる失敗の受け入れ基準を FR4.10 に足すことを Functional Design に割り当てている。U3 の成果物はこの基準（例：再試行を上限まで繰り返しても Throttled が続くストリームは失敗に数え、取得できた分は表示する）を提案していない | functional-spec.md か rules.md に、FR4.10 へ足す受け入れ基準の文面を提案として書き、traceability.json で BR2.1、BR3.3 と結びつける | New |
| R-05 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/entities.md > FetchJob の relationships、functional-spec.md > 5. ER 図 | ER 図は「entities.md から写したもの」とあるが、FetchJob と StreamFetchOutcome の関係と SessionState と FetchJob の関係は entities.md の relationships にない。FetchJob と StreamPlan は 1 対 1 とされるが、UC1 手順 3 で FetchJob は計画の前に作られ（中断や列挙前は計画がない）、FetchJob に StreamPlan を指す属性もない。identifier の jobId が属性として定義されていない。listingFailure が StreamPlan と FetchJob の両方にあり、持ち主が曖昧 | entities.md と ER 図をそろえる（関係を両方に書く、StreamPlan は 0..1 にする、jobId を属性に足す、listingFailure の正を 1 か所に決める） | New |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/functional-spec.md > UC1 手順 4、rules.md > BR6.2 | 列挙は全ページを終えてから取得に入り、列挙中の進み具合の表示がない（「計画したストリーム数」は列挙の後に決まる）。取得範囲が過去で、新しいストリームが数万あるロググループでは、範囲に届くまで多数のページをたどるため、「取得中」だけが長く続く。FR4.8 の最低限は満たすが、利用者には止まって見える | 列挙中のページ数や見つけたストリーム数を受け口に届け、ステータス行に出すか、出さないことを判断として明記する | New |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/rules.md > BR4.4、BR4.1、BR6.4 | BR4.4 は (logStreamName, sequence) から位置を求めるが、並び替えのキーは (timestamp, logStreamName, sequence) で、その行の timestamp を引く索引が前提になる。100 万件でページごと（timelineVersion が変わるたび）に位置を求める計算量の要求が BR4.3 のような時間の制約として書かれていない。列幅など Functional Design に割り当てられた未解決事項（ログ一覧の列幅）も触れていない | BR4.4 に、位置を求める計算量の目標（100 万件で 1 回のページ追加ごとに画面が固まらない）を足し、列幅は固定か最小限の既定かを決めて書く | New |
| R-08 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u3-fetch-robustness/functional-design/traceability.json > upstream_ids と coverage | upstream_ids と coverage が FR だけで、U3 の範囲に明記された NFR2、NFR3、NFR4、NFR6、NFR12、NFR13 がない。BR4.3、BR6.8 などは理由欄に NFR を挙げるが、機械的な対応表では追えない。また BR4.5 の trigger は接続の変更時の破棄（U2:BR2.4）を含まず、その際の timelineVersion の増加が定められていない | NFR の項目を coverage に足す。接続の変更による破棄でも timelineVersion が増えることを BR4.5 に足す | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| traceability の機械照合（手作業） | OK の対象 BR はすべて rules.md に存在する。BR2.3、BR3.2、BR4.3〜BR4.5、BR5.5、BR5.6、BR6.7〜BR6.9 は reverse に載り、孤立した BR はない | 構造上は問題なし。NFR が coverage にない点は R-08 |
| mermaid の構文確認（目視） | graph LR、stateDiagram-v2 3 本、erDiagram はいずれも構文が正しく、テキストの代替も付いている | 問題なし。ER 図の内容と entities.md のずれは R-05 |
| 技術非依存の確認 | 成果物に特定の言語やフレームワークの名前はない | 問題なし |
| 回答の反映（Q1〜Q5） | Q1 の 1・2・4・8・16 秒、±20%、最大 5 回、Throttled と Network は BR2.1 と RetryPolicy に一致。Q3 は BR4.1、Q4 は BR6.5 と BR4.4、Q5 は BR6.3 と BR6.8 に一致。Q2 は BR1.2 と BR1.3 に反映されているが時刻なしの扱いに矛盾がある | Q2 の部分は R-01 |
| 受け入れ基準 | 2 時間前は BR1.2 で呼ばない、30 分前は BR1.3 で呼ぶ、10 ストリーム中 2 失敗は BR3.3 と BR6.2 で 8 ストリーム分表示と「2 ストリームで失敗」、逐次表示は BR4.2 と BR5.4 が対応。時刻なしの基準は R-01 の条件付き | R-01 を参照 |
| 持ち越し分 | U1 の R-06 は BR5.6 と BR4.5、U1 の R-08 は rules.md 冒頭と名前の置き換え、U2 の R-06 は BR6.7 | U2 の R-06 は仕組みが未定（R-02） |

### Summary

Q1〜Q5 は概ね忠実に反映され、状態遷移、読み取り 3 API の制約、秘密情報の扱い、技術非依存、traceability の整合は妥当である。残る主な懸念は、ストリームの打ち切り規則が時刻なしストリームの取り込みと両立するかが曖昧な点（R-01）と、U2 から持ち越した順序保証の仕組みが未定義な点（R-02）で、いずれも Major だが 2 件にとどまるため、実装者への確認は要るものの READY とする。
