# Refined Mockups Questions — Logs Insights 互換のクエリ（IB-11）

## Sources

- `inception/requirements-analysis/requirements.md`（FR1〜FR9、NFR9〜NFR11）。ラフな画面設計（rough-mockups）はこの計画では行っていないため、要件と今の画面（261010-console-ui-polish の mockups.md と今のコード）から起こす。
- 今の画面：上部ナビゲーション（プロファイル・リージョン・タイムゾーン・設定）、左ペイン（ロググループの表、1 行選択）、右ペイン（見出しにロググループ名と [Fetch]、絞り込み欄、開始・終了、ログの一覧）、ステータス行。部品は Cloudscape。

## Q1. ログの画面（Log events）とクエリの画面（Logs Insights）は、どう切り替えますか？

背景：要件で 2 つの画面を分けると決めました（FR1.1）。コンソールでは、左のナビゲーションの「Log groups」と「Logs Insights」で画面が切り替わります。今のアプリには、画面を切り替える仕組みがまだありません。

A. 左の端に細いナビゲーション（Cloudscape の SideNavigation）を置き、「Log events」「Logs Insights」で切り替える（コンソールに近い）
B. 上部ナビゲーションに「Log events」「Logs Insights」の切り替えを置く
C. 右ペインの上にタブ（Cloudscape の Tabs）を置いて切り替える。左ペインのロググループの表はログの画面だけで使う
D. Not yet defined
X. Other (please specify)

[Answer]: A. 左の端に細いナビゲーション（Cloudscape の SideNavigation）を置き、「Log events」「Logs Insights」で切り替える（コンソールに近い）

## Q2. クエリの画面では、ロググループをどう選びますか？

背景：クエリの画面ではロググループを 1 つ以上選びます（FR1.3）。コンソールの Logs Insights では、クエリ欄の上の「ロググループを選ぶ」ドロップダウンで、名前で探して複数選びます。今のログの画面は、左ペインの表から 1 つだけ選びます。

A. コンソールと同じく、クエリ欄の上に複数選択のドロップダウン（Cloudscape の Multiselect、名前で探せる）を置く。左ペインはクエリの画面では出さない
B. 左ペインの表をクエリの画面では複数選択（チェックボックス）にして使う
C. Not yet defined
X. Other (please specify)

[Answer]: A. コンソールと同じく、クエリ欄の上に複数選択のドロップダウン（Cloudscape の Multiselect、名前で探せる）を置く。左ペインはクエリの画面では出さない

## Q3. クエリの文を書く欄は、何で作りますか？

背景：書く手助け（入力の補完・構文の色分け、FR9、Should）を入れるかどうかで、欄の作り方が変わります。Cloudscape には、ふつうの複数行の欄（Textarea）と、色分けや補完を付けられるコードの欄（CodeEditor。中で Ace というエディタを使い、アプリが少し重くなる）があります。

A. 最初は等幅の複数行の欄（Textarea）で作る。補完と色分け（Should）を入れるときに、コードの欄へ置き換えるか決める
B. 最初からコードの欄（CodeEditor）で作る。色分けと補完をそのまま載せられる
C. Not yet defined
X. Other (please specify)

[Answer]: B. 最初からコードの欄（CodeEditor）で作る。色分けと補完をそのまま載せられる

## Q4. 結果の一覧で行を開いたとき、何を見せますか？

背景：要件では、行を開くとメッセージ全体を見られる（FR7.4）とだけ決めています。コンソールの Logs Insights では、行を開くと、その行の項目と値の一覧（`@timestamp`、`@message`、JSON から取り出した項目など）が出ます。今のログの画面は、行を開くとメッセージ全体を出します。

A. コンソールと同じく、項目と値の一覧を出す（メッセージ全体も一覧の 1 行として出す）
B. 今のログの画面と同じく、メッセージ全体だけを出す
C. Not yet defined
X. Other (please specify)

[Answer]: A. コンソールと同じく、項目と値の一覧を出す（メッセージ全体も一覧の 1 行として出す）

## Q5. クエリの書き間違い・対応外のエラーは、どこに出しますか？

背景：エラーには、対応していないことか書き間違いかと、クエリの中の場所（行と列）を示します（FR6）。

A. クエリ欄の直下に、文字でエラーを出す（例：「3 行 12 列：`stats` には対応していません」）
B. A に加えて、クエリ欄の中の該当する場所にも印を付ける（Q3 でコードの欄にした場合だけできる）
C. Not yet defined
X. Other (please specify)

[Answer]: B. A に加えて、クエリ欄の中の該当する場所にも印を付ける（Q3 でコードの欄にした場合だけできる）
