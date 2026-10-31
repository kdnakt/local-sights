# Functional Design Questions — U4 互換の確かめ（u4-compat-stats）

## Sources

- `inception/units-generation/unit-of-work.md`（U4 の責務・確かめ方）、`unit-of-work-story-map.md`（FR7.1〜FR7.3）
- `inception/requirements-analysis/requirements.md`（FR7、NFR3・NFR4、OQ4）
- U1・U2 の機能設計の「コンソールと比べる」としたもの：鍵のないイベント、数でない値、by なしの 0 件の 1 行、小数 4 桁、鍵の昇順、関数名の大文字・小文字、pct の順位、stddev の分母、count_distinct の正確さ、sortsFirst と min、earliest の同じ時刻、stats のあとの filter、既定の 10,000 行
- 今の確認用プログラム：`examples/compat_check.rs`・`examples/compat/*.rs`（compare：書き出しがない／拒否／打ち切り／列の名前／打ち切りの同点／行の集まり／順番の 7 段）、`examples/compat_queries.txt`（CloudTrail のロググループに合わせた q01〜q11・p01〜p09、`order` / `set` の比べ方）、`scripts/compat/insights-export.sh`（コンソールの Logs Insights の結果を書き出す。本人の認証情報で手元だけで動かす）

決まっていて聞き直さないこと：集計の行の集まり（鍵と集計値）が一致することを比べ、sort を書いたクエリだけ並び順も比べる（FR7.1）。比べるクエリは AWS の資料の例と開発者のロググループに合わせる（FR7.2）。コンソールと食い違う拡張は組に入れない（FR7.3）。手元だけで動かし、書き出しはリポジトリに置かない（NFR4）。

## Q1. `avg`・`stddev`・`pct` のような小数の集計値は、どう比べますか？（FR7.1、U1 の機能設計 Q5）

背景：アプリは小数を 4 桁に丸めて出します（U1 の Q5 A）。コンソールの桁数は資料に書かれておらず、丸め方の違いで末尾の桁だけが違うことがありえます。

- A. 数に読める値どうしは、相対誤差 1e-6 か絶対誤差 0.0001 のどちらかに収まれば同じとみなし、差があったときは桁の違いとして報告に分けて出す（推奨。丸めの違いと計算の違いを見分けられる）
- B. 文字のまま完全に一致することを求める（違えばすべて差として報告）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q2. コンソールの `count_distinct` は値が非常に多いと近似です。アプリは正確に数えます（U2 の Q1 A）。比べ方はどうしますか？

背景：資料は「非常に多い値では近似」とだけ書き、境目は書いていません。

- A. `count_distinct` の列は、差が 2% 以内なら「近似の範囲」として同じとみなし、報告に近似の範囲だったことを書く。2% を超えたら差（推奨）
- B. 完全に一致することを求める。近似による差は報告を見て人が判断する
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. 比べる集計のクエリの組は、どのロググループに合わせますか？（FR7.2、OQ4）

背景：今の組は CloudTrail のロググループに合わせています（`eventSource`・`eventName`・`errorCode`・`userIdentity.type`・`readOnly` など）。CloudTrail には数の JSON の項目がほとんどなく、`avg`・`sum`・`pct` を意味のある形で試しにくいです。

- A. 今の CloudTrail の組に集計のクエリを足す（`count(*) by bin(5m)`、`by eventSource`、`count_distinct(eventName)`、`earliest`・`latest`、`stats ... | sort ... | limit`、`stats ... | filter ...`、鍵のないイベントの `by errorCode` など）。数の集計（`avg`・`sum`・`pct`・`stddev`・`min`・`max`）は、数の JSON の項目を持つ別のロググループ用の組（`compat_queries_numeric.txt`）を作り、項目の名前は開発者が自分のロググループに合わせて書き換える前提にする（推奨）
- B. CloudTrail の組だけにし、数の集計は `toMillis(@timestamp)` を使って試す（`avg(toMillis(@timestamp))` など）
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. 比べた結果、U1・U2 の仮の決まりがコンソールと違うと分かったときは、どうしますか？

背景：仮の決まり（鍵のないイベント、0 件の 1 行、鍵の昇順、pct の順位など）はコンソールと比べて直す約束です（U1・U2 の機能設計）。U4 はその比べる道具を作る作業単位です。

- A. U4 の作る段階で比べ、違いが分かった決まりは U4 の中で直す（中核の StatsAggregator などを直し、直したことを記録する）。直すのが大きいもの（設計の見直しが要るもの）は利用者に諮ってから決める（推奨）
- B. U4 では比べて報告するだけにし、直すのは次の作業にする
- C. Not yet defined（設計で決める）
- X. Other (please specify)

[Answer]: A
