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
