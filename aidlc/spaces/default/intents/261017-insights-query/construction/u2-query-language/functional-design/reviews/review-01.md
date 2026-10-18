## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T05:19:33Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR1.6、functional-design-questions.md > Q1a の「読み取り」（型の違う値） | BR1.6 は「数字だけの文字列と数は = < <= > >= のすべてで数として比べる」「文字列どうしは数字だけでなければ Unicode の順」「真偽値は文字列 true・false と同じ」と言い切っているが、コンソールで確かめたのは次だけである。6b は文字列の項目と数のリテラルの `=`、6f は数字だけの文字列どうしの `>`、6c〜6e・6g・6h は数字でない文字列と数の比べ。数字だけの文字列と数のリテラルの `<`・`>` は流していない。6a（`"10" as a, 9 as b` の `a > b`）は、文字列の項目と数の項目の `>` がコンソールで Type Mismatch になった唯一の例で、読み取りは 6a を「文字の値どうし」と書いていて実際の式と合わない。6f は「数字だけなら数として比べる」とも「自然順で比べる」とも読めるため、`v10 > v9` のような数字でない文字列の `>` が Unicode の順でよいかは分からない。さらに 6 組は全部が定数か `@logStream` で、実際の JSON の項目の値（JSON の数、JSON の文字列 "200"）では 1 本も流していない。型の比べ方は FR4.5 の互換の中心なので、設計が確かめた範囲を超えて断定している | BR1.6 を、確かめた組（6b・6c〜6e・6f・6g・6h）と、推測で決めた組（数字だけの文字列と数の順序の比べ、数字でない文字列どうしの順序、真偽値、JSON の項目の実値）に分けて書き、推測の組は「仮の決まり」と明記する。6a の読み取りを「文字列の項目と数の項目の比べがコンソールでは型の誤りになるが、アプリは項目どうしの比べを作らないため起きない」に直す。推測の組は U4 の比べるクエリの組か、`console-check.sh` に JSON の項目を使う 2〜3 本を足して、この作業単位の作る段階までに確かめる | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR1.2、BR3.4、BR4.2、BR7.4、functional-spec.md > 2. 流れ UC1 の 4・6 | JSON の項目を使う評価の費用と中断の設計が足りない。(1) BR1.2 は「同じイベントを 1 回の評価の中で何度も読み直さない」と「読んだ結果をそのイベントの評価が終わるまで持つ」を同時に書いており、`filter a=1 \| sort b \| fields c` のように命令をまたぐと両立しない。(2) sort は全件の並べ替えの鍵が要るが、鍵を 1 度だけ取り出して持つのか、比べるたびに JSON を読み直すのかが書かれていない。後者だと 100 万件で比べの回数 × JSON の解析になる。(3) UC1 の 4 は sort の前と後でしか止める信号を確かめないため、100 万件の自然順の並べ替え中は [Cancel] と接続の変更が効かず、U1 の BR5.8 の 10,000 件ごとの中断と NFR2（評価中も画面が反応する）に反する。(4) BR3.4 の「全行で値がない列を外す」は結果の全行を JSON として読み直す別の走査になるが、手順のどこで行うか、費用に入っているかが書かれていない。NFR1 の数値は後で測る約束だが、その前に作りの形が決まっていないと測る対象が定まらない | BR1.2 を「命令の 1 回の当てはめの中で 1 回だけ読む」に直し、命令をまたぐ再利用の有無を決める。sort は「鍵を区切りごとに取り出して持ち（区切りごとに止める信号を確かめる）、その後の並べ替えは鍵だけで行う」形を決め、並べ替え中の中断の遅れの許容（または並べ替えの途中でも止められる作り）を書く。BR3.4 の列の判定を UC1 の手順のどこで行うか（結果の行だけを対象にする）を書き、測る対象（JSON の項目を使う filter・sort・列の判定）を §7 の速さの確かめに入れる | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > 「U1 の仮の決まりの扱い」表の BR3.3 の行、BR4.1 | 大文字・小文字を「そのまま（確かめた）」としているが、根拠の 3a・3c・3d が流したのは FILTER・LIKE・IN・Fields・Sort・Desc・Limit だけ。AND・OR・NOT は 3b がエラーで、Q1a の読み取り自身が「AND は単独で確かめていない」と書いている。`NOT LIKE`・`NOT IN` も未確認である | 表の「確かめた」を、確かめた言葉の一覧に直す。AND・OR・NOT・NOT LIKE・NOT IN は仮の決まり（区別しない）と書き、U4 で確かめる対象に入れる | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR5.1、BR5.2、entities.md > Command.limit | `limit 0` の結果は 7c で 992 件（全件）と分かったが、992 件は既定の上限 10,000 より小さいため、「0 は絞らない（ただし 10,000 では切る）」と「0 は 100,000 まで」を区別できない。BR5.2 の「limit 0 だけのときも 10,000 で切る」は推測である。entities.md の「0 は件数を絞らない」とも読み方がずれる | BR5.2 の `limit 0` の扱いを仮の決まりと明記し、entities.md の文言を「0 は limit による絞りなし。既定の上限は別に効く」にそろえる。10,000 件を超えるロググループでの確かめを U4 に入れる | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR4.2、BR4.3、entities.md > FieldValue | sort の比べ方が使う「数の値」と「数でない値」の分け方が、FieldValue の種類（Missing・Text・Number・Boolean）との対応で書かれていない。JSON の数、数字だけの JSON の文字列 "10"、`@timestamp`（数値のミリ秒）が、昇順でどの側に入るか決まっていない。`sort @timestamp` は最もよく書かれる形なので、実装者が推測することになる | BR4.2 に、FieldValue の種類ごとに「数の側か数でない側か」を表で書く（Number と @timestamp は数、Text は数字だけでも数でない側かどうかを決める、Boolean は文字列、Missing は最小）。`sort @timestamp` の例を足す | New |
| R-06 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR1.1、BR1.4 | 配列の要素 `a.0` を Unsupported にするとしているが、BR1.1 の名前の文字（英数字・_・-）と . の区切りでは `a.0` は普通の入れ子の書き方として読めてしまう。数字だけの区切りを Unsupported にする規則が書かれていないと、`a.0` は Missing として通り、`!=` の条件が全件を満たすという黙った誤りになる。確かめ用の CloudTrail のロググループは `resources.0.…` のように配列が多いのに、1 行流すだけで確かめられる配列の扱いが残された問いに回っている | BR1.1 に「数字だけの区切りは Unsupported（配列の要素）」を足し、実際のオブジェクトのキーが数字の場合の扱い（バッククォートで囲めば JSON のキーとして読む、など）を決める。可能なら `console-check.sh` に `resources.0` の 1 本を足して確かめる | New |
| R-07 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR2.4 | `not` の読み方が 1 つの IF 文に混ざっている。「not の次が ( なら否定」は条件の先頭の `not`、「not の次が like・in」は項目の後ろに来る `not like`・`not in` の `not` で、置かれる場所が違う。このまま書くと、条件の先頭の `not like /x/` も受け付ける実装になりうる | `not` を 2 つの場面（条件の先頭の `not (…)`、項目の直後の `not like`・`not in`）に分けて書く。条件の先頭の `not` の次が括弧でないものは Syntax、とする例を 2〜3 個足す（Q1a の 3b・4d・4e・5e をテストの例にする） | New |
| R-08 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/functional-spec.md > 2. 流れ UC3、4. 状態の移り変わり、entities.md > QueryResult.matchedCount・truncatedAt | (1) EvaluationFailed のときに結果の囲みに前の結果を残すか空にするかが書かれていない（QueryError は「前の結果を残す」と書かれている）。「異常終了」が何を指すか（評価の作業の異常な終わり、など）も定義がない。(2) matchedCount は「打ち切る前の条件に合った件数」だが、`filter a \| limit 5 \| filter b` のように途中に limit があると、どの時点の件数かが決まらない | UC3 に結果の囲みの扱いと、異常終了の意味を書く。matchedCount を「最後の limit の直前の件数」などに定義し、途中の limit がある場合の例を足す | New |
| R-09 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/functional-design/rules.md > BR1.7、BR7.3 | 入れ子の名前を `a.b` の形で集めて出すと、バッククォートで囲む必要のあるキー（名前に . を含む `user.id`）と入れ子の `user` の中の `id` が同じ見た目になる。補完で選んだ名前をクエリに入れると別の項目を指す | 補完の候補と行の一覧では、名前に . などを含むキーはバッククォートつきで出す（BR1.3 と同じ書き方）と書く | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS（gaps・orphans・invalid の各 0 件） | 上流の要件と作業単位への対応に抜けはない |
| aidlc engine sensor-upstream-coverage | PASS（consumes が空のため走査対象なし） | 検査の対象がなかったので、上流の抜けの検査としての意味はない。上流の確認は手作業で行い、FR3.3〜FR9.3 に対応する決まり（BR1〜BR10）があることを確かめた |

### Summary

コンソールの実測（Q1a・Q1b）は、並び順、既定の列、優先順位、`not` の括弧、ない項目、limit の上限について決まりに正しく反映されており、状態の移り変わりと U1 から回した 3 つの直しも整っている。実装前に詰めるべきは、実測の範囲を超えて断定した型の比べ方（BR1.6）と、JSON の項目を使う sort・列の判定の費用と中断の作り（BR1.2・UC1）の 2 点で、どちらも重大な欠陥ではないため、READY とした。ただし人が承認するときに、この 2 点を直す（または推測の部分を仮の決まりと明記する）かを決めてほしい。
