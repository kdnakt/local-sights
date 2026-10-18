# Functional Design Questions — U2 クエリの言語をそろえる（u2-query-language）

## Sources

- `inception/units-generation/unit-of-work.md`（U2 の責務・確かめ方・実装の注意）、`unit-of-work-story-map.md`
- `inception/requirements-analysis/requirements.md`（FR3.3〜FR3.8、FR4.2〜FR4.5、FR6.1・FR6.2・FR6.4、FR7.1・FR7.3〜FR7.5、FR8、FR9、OQ1・OQ2）
- `inception/domain-design/components.md`（EventFieldReader・QueryParser・QueryEvaluator・QueryResultTable・QueryEditor）
- U1 の機能設計 `construction/u1-filter-skeleton/functional-design/`（rules.md の BR3.3・BR3.4・BR5.1・BR6.3 の仮の決まり）と、U1 のコードの見直し（R-02〜R-04 を U2 に回す）
- AWS の資料「limit」（CWL_QuerySyntax-Limit）：`limit` を書かないときは 10,000 件まで。`limit` は 100,000 まで書ける。`limit any N` もある
- AWS の資料「sort」（CWL_QuerySyntax-Sort）：自然順の改良版。昇順では、数でない値がすべて数の値より前。数でない値は、数字の続きと文字の続きに分けて、文字の部分は Unicode の順、数字の部分は桁数→値で比べる。降順はその逆
- AWS の資料「fields」（CWL_QuerySyntax-Fields）：`fields` を何度書いても、書いた項目をすべて出す
- AWS の資料に書かれていないもの：`sort` の既定の向き、`sort` がないときの並び順、`fields` がないときの列、ない項目の条件と並び、型の違う値の比べ方、言葉の大文字・小文字、優先順位

## Q1. 資料に書かれていないコンソールの細部を、どう決めますか？

背景：要件では、これらを「コンソールと同じ」にすると決めています（FR3.3〜FR3.5、FR3.8、FR4.4、FR4.5、FR5.2、OQ2）。U1 では仮の決まりで作りました。U2 で、言語として固めます。

A. U1 の Q2 と同じく、確かめ用のスクリプトを用意し、あなたの手元で CLI から Logs Insights に流して確かめる（JSON のログがあるロググループを使う。読み取りのクエリだけで、短い期間にとどめる）。結果に合わせて決める（推奨：互換が目標のため、推測で決めずに済む）
B. 一般的な決まり（SQL に近い形）で決め、U4 の確認用プログラムでコンソールと比べて、違いがあれば直す
C. Not yet defined
X. Other (please specify)

[Answer]: A. U1 の Q2 と同じく、確かめ用のスクリプトをあなたの手元で CLI から流して確かめ、結果に合わせて決める

## Q1a. `console-check.sh` の出力を貼ってください（Q1 の確かめ）

背景：Q1 の A に合わせて、同じディレクトリに `console-check.sh` を置いた。あなたの手元で、JSON のメッセージを含むロググループと、一部のイベントにだけある JSON の項目の名前（FIELD）を指定して流す。出力（件数・時刻・列の名前・エラーの文。FIELD の値は先頭 12 文字まで）に合わせて、U2 の決まりを書く。

[Answer]: 利用者がスクリプトを直して（優先順位の言葉を PA・PB・PC で変えられるようにして）push し、2026-10-18 に CloudTrail のロググループ（us-east-1、FIELD=errorCode、PA=true・PB=kms・PC=AssumeRole、1767435900〜1767439500 秒、992 件）で流した。出力（利用者が貼ったまま。リクエスト ID は省いた）：

```text
--- 並び順（@timestamp の値。U2 の sort の既定）
[1a sort の既定の向き] 2026-01-03 1 | 2026-01-03 1 | 2026-01-03 1
[1b sort asc] 2026-01-03 1 | 2026-01-03 1 | 2026-01-03 1
[1c sort desc] 2026-01-03 1 | 2026-01-03 1 | 2026-01-03 1
[1d sort なし] 2026-01-03 1 | 2026-01-03 1 | 2026-01-03 1
--- fields を書かないときの列
[2a 命令なし] 列=@timestamp,@message,@ptr 行数=1
[2b filter だけ] 列=@timestamp,@message,@ptr 行数=1
[2c fields を 2 回] 列=@timestamp,@logStream,@ptr 行数=1
--- 言葉の大文字・小文字
[3a FILTER と LIKE] Complete 件数=992
[3b AND・OR・NOT] ERROR: MalformedQueryException: Query cannot be compiled
[3c IN] Complete 件数=0
[3d Fields・Sort・Limit] Complete 件数=992
--- 優先順位（a=/true/、b=/kms/、c=/AssumeRole/ を @message に当てる）
[4a a or b and c] Complete 件数=885
[4b a or (b and c)] Complete 件数=885
[4c (a or b) and c] Complete 件数=40
[4d not a and b] ERROR: MalformedQueryException: Query cannot be compiled
[4e (not a) and b] ERROR: MalformedQueryException: Query cannot be compiled
[4f not (a and b)] Complete 件数=908
--- ない項目（FIELD は一部のイベントにだけある）
[5a 全体] Complete 件数=992
[5b FIELD がある] Complete 件数=73
[5c FIELD = "zzz"] Complete 件数=0
[5d FIELD != "zzz"] Complete 件数=992
[5e not FIELD = "zzz"] ERROR: MalformedQueryException: Query cannot be compiled
[5f FIELD not like /zzz/] Complete 件数=992
[5g FIELD not in ["zzz"]] Complete 件数=992
[5h sort FIELD asc（FIELD の値）] (なし) | (なし) | (なし)
[5i sort FIELD desc（FIELD の値）] ValidationEx | ValidationEx | ValidationEx
[5j fields にない項目] 列=@timestamp,@ptr 行数=1
--- 型の違う値（ログに依らない値で比べる）
[6a "10" > 9] ERROR: MalformedQueryException: Type Mismatch ([87,98])
[6b "10" = 10] Complete 件数=992
[6c "abc" > 5] Complete 件数=0
[6d "abc" < 5] Complete 件数=0
[6e "abc" != 5] Complete 件数=992
[6f "10" > "9"] Complete 件数=992
[6g @logStream = 1] Complete 件数=0
[6h @logStream != 1] Complete 件数=992
--- limit
[7a limit 100000] Complete 件数=992
[7b limit 100001] ERROR: MalformedQueryException: Row limit exceeded. Specified: 100001. Maximum: 100000 ([80,91])
[7c limit 0] Complete 件数=992
```

### 読み取り（この答えから分かったこと）

- **並び順（1a〜1d）**：@timestamp を先頭 12 文字で切ったため、向きが読み取れなかった。Q1b で確かめ直す。
- **fields を書かないときの列（2a・2b）**：`@timestamp` と `@message`（`@ptr` はコンソールの内部の項目）。`fields` を 2 回書くと両方の項目が出る（2c、資料のとおり）。`fields` に書いた項目がどのイベントにもないときは、その列は出ない（5j）。
- **言葉の大文字・小文字（3a・3c・3d）**：`FILTER`・`LIKE`・`IN`・`Fields`・`Sort`・`Desc`・`Limit` を受け付ける。`AND` は単独で確かめていない（3b は `not` の書き方のためにエラーになった。下の `not` を参照）。
- **優先順位（4a〜4c）**：`and` は `or` より強い（`a or b and c` は `a or (b and c)` と同じ件数）。
- **not（3b・4d・4e・4f・5e）**：`not` の後ろに括弧のない条件を書くとエラーになる（`not @message like /x/`、`(not a) and b`、`not FIELD = "zzz"` はどれも読めない）。`not (a and b)` のように括弧で囲めば使える。否定は `not like`・`not in`・`!=` で書く。
- **ない項目（5c〜5g）**：`= "zzz"` は満たさない。`!= "zzz"`・`not like`・`not in` は、項目がないイベントでも満たす。
- **ない項目の並び（5h・5i）**：昇順ではない項目のイベントが先に、降順では値のあるイベントが先に来る（ない項目はいちばん小さい値として並ぶ）。
- **型の違う値（6b〜6h）**：数字だけの文字列と数は数として比べる（`"10" = 10` を満たし、`"10" > "9"` も満たす）。数字でない文字列と数の `=`・`<`・`>` は満たさず、`!=` は満たす（6c〜6e、6g・6h）。6a（文字の値どうしを `fields ... as` で作った比べ方）はコンソールが型の誤りとして読めなかったが、アプリは `fields ... as` を作らないため、この形は起きない。
- **limit（7a〜7c）**：100,000 まで書ける。100,001 は「上限を超えた」エラーになる。`limit 0` は件数を絞らない（992 件すべて）。

## Q1b. 並び順だけを、もう一度確かめてください（Q1a の続き）

背景：Q1a では @timestamp を先頭 12 文字で切って出したため、`sort` の既定の向き（1a）と、`sort` がないときの並び順（1d）が読み取れなかった。スクリプトを直して、@timestamp をミリ秒まで出し、`ONLY=sort` で並び順の 4 本だけを流せるようにした。

使い方：`git pull` してから、Q1a と同じ値に `ONLY=sort` を足して流す（4 本だけなので、読む量は Q1a より少ない）。

[Answer]:
