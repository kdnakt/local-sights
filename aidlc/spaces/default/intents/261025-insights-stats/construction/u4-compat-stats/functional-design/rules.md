# Business Rules — U4 互換の確かめ（u4-compat-stats）

上流：`inception/units-generation/unit-of-work.md`（U4）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR7.1〜FR7.3、NFR3・NFR4、OQ4、project.md Forbidden）、U1・U2 の機能設計、質問票 `functional-design-questions.md`（Q1〜Q4）、今のコード（`examples/compat/compare.rs` の 7 段の比べ方、`format.rs` の Row と insights_timestamp、`queries.rs` の組の読み方、`report.rs`、`scripts/compat/insights-export.sh`）。

## 守ること（NFR4・project.md Forbidden）

- 確認用プログラムと書き出しのスクリプトは、開発者の手元で本人の認証情報を使って動かす。自動テストと CI からは動かさず、AWS に接続しない。比べる処理そのものは見本のデータで自動テストする。
- コンソールから書き出した結果（ログの中身を含む）はリポジトリに置かない。組のファイルにはクエリの文だけを書く。
- 書き出しのスクリプトが使う `aws logs start-query` / `get-query-results` は、開発者が手元で実行する道具で、アプリは呼ばない（前の作業 261017 の U4 と同じ。アプリが呼ぶ API は読み取りの 3 つだけのまま）。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 今の CloudTrail の組に、集計のクエリを足す
    category: policy
    applies_to: compat_queries.txt
    trigger: U4 を作るとき
    logic: 足すクエリ（s01〜）は少なくとも次を含む：`stats count(*) by bin(5m)`（空のビンを出さない、FR2.2）、`stats count(*) as n by eventSource | sort n desc | limit 10`（order）、`stats count(*) by errorCode`（鍵のないイベント、U1:BR2.1）、`filter eventName = "NoSuchEvent" | stats count(*)`（by なしの 0 件、U1:BR2.6）、`stats count_distinct(eventName) by eventSource`（近似、U2:BR2.1）、`stats earliest(eventName), latest(eventName) by eventSource`（U2:BR2.5）、`stats sortsFirst(eventName), min(eventName), sortsLast(eventName), max(eventName)`（U2:BR2.4）、`stats count(*) as n by eventSource | filter n > 10`（U2:BR1.4）、`stats count(*) by userIdentity.type`（JSON の入れ子の鍵、FR7.2）、`stats count(*) by bin(1h), eventSource`（鍵が 2 つ）、`stats count(*) by eventName`（既定の 10,000 行、U2:BR3.2。行が少なければ打ち切られないことだけを確かめる）、`STATS count(*) by eventSource`（命令の大文字）。sort を書いたものは order、ほかは set
    violation: なし
    source: FR7.2、Q3 A
  - id: BR1.2
    statement: 数の集計用に、数の JSON の項目を持つロググループのための組を新しく作る
    category: policy
    applies_to: compat_queries_numeric.txt
    trigger: U4 を作るとき
    logic: 新しい組（n01〜）を作り、項目の名前は仮に `latency`（数）と `status`（文字）とする。冒頭の注記に「開発者が自分のロググループの項目の名前に書き換えて使う」と書く。含めるクエリ：`stats avg(latency), sum(latency), min(latency), max(latency), stddev(latency) by status`、`stats pct(latency, 50) as p50, pct(latency, 95) as p95, pct(latency, 99) as p99`、`stats avg(latency) by bin(5m)`、`stats sum(latency) as s by status | sort s desc`（order）、`stats avg(status)`（数でない値だけ、U1:BR2.3）、`stats avg(toMillis(@timestamp))`（どのロググループでも数になる）。小数の列には tolerance の decimal を、count_distinct の列には approximate を書く
    violation: なし
    source: FR7.2、Q3 A
  - id: BR1.3
    statement: 組のファイルに、列ごとの比べ方の緩めを書ける
    category: validation
    applies_to: queries.rs
    trigger: 組を読むとき
    logic: 今の 4 つの欄（id、order|set、目的、クエリ）に、省略できる 5 つ目の欄を足す。中身は `列の名前=decimal` か `列の名前=approximate` を `;` で区切ったもの。読めない欄はその行のエラー（今の組の行のエラーと同じ扱い）
    violation: その行を比べず、報告に組の行のエラーとして出す
    source: FR7.1、Q1 A、Q2 A
  - id: BR1.4
    statement: U1・U2 の仮の決まりと、それを確かめるクエリの対応を、組のファイルの注記と報告に持つ
    category: policy
    applies_to: compat_queries.txt、compat_queries_numeric.txt、report.rs
    trigger: 組を書くとき、報告を出すとき
    logic: 組の注記に「# provisional: U1:BR2.1 = s03」のような行を書き、報告の最後に、仮の決まりごとに対応するクエリの結果（一致・違い・比べていない）をまとめる
    violation: なし
    source: FR7.3、Q4 A
  - id: BR2.1
    statement: アプリの結果が集計の形なら、集計の行の集まりを比べる。比べ方の段は今の 7 段のまま
    category: policy
    applies_to: compare.rs、format.rs
    trigger: 1 つのクエリを比べるとき
    logic: アプリの結果（AppRows）を shape で分ける。events は今のまま。aggregates は、列の名前（as の名前、または切り出しの文字）と、セルの文字（表と同じ。時刻の列は今の insights_timestamp の UTC の書き方に変える）で Row を作る。コンソールの書き出しの行は今のまま読む（@ptr を外す）。そのあと今の 7 段（書き出しがない／拒否／打ち切り／列の名前／打ち切りの同点／行の集まり／順番）で比べる。行の集まりと順番の段で、セルどうしを BR2.2・BR2.3 で比べる
    violation: なし
    source: FR7.1
  - id: BR2.2
    statement: 列の比べ方が decimal なら、数に読める値どうしは相対誤差 1e-6 か絶対誤差 0.0001 に収まれば同じとみなす
    category: calculation
    applies_to: compare.rs
    trigger: tolerance が decimal の列のセルを比べるとき
    logic: 両方が数に読めれば、|a − b| ≤ 0.0001 または |a − b| ≤ 1e-6 × max(|a|, |b|) なら同じ（withinDecimal）。完全に同じ文字なら exact。片方でも数に読めなければ文字で比べる。行の集まりの段では、行の一致を見るときにこの比べ方を使う（並べてから 1 つずつ対にする。対になった行の中で範囲内の違いを報告に分けて出す）
    violation: 範囲を超えたら different
    source: FR7.1、Q1 A
  - id: BR2.3
    statement: 列の比べ方が approximate（count_distinct）なら、差が 2% 以内なら同じとみなす
    category: calculation
    applies_to: compare.rs
    trigger: tolerance が approximate の列のセルを比べるとき
    logic: 両方が整数に読めれば、|a − b| ≤ 0.02 × max(a, b) なら同じ（withinApproximate）。完全に同じなら exact。読めなければ文字で比べる
    violation: 範囲を超えたら different
    source: FR7.1、Q2 A
  - id: BR2.4
    statement: 集計の結果の打ち切りの段は、今の events と同じ決まりで扱う
    category: policy
    applies_to: compare.rs
    trigger: 打ち切りの段
    logic: sort のない集計のクエリが limit か既定の 10,000 行で切られたときは比べない（Skipped。どのグループが残るかは並びによる）。sort のある集計のクエリは、今の「打ち切りの同点」の段で扱う
    violation: なし
    source: FR7.1、U2:BR3.2
  - id: BR2.5
    statement: 報告に、範囲内の違い（小数・近似）を、違いとは別に分けて出す
    category: policy
    applies_to: report.rs
    trigger: 報告を出すとき
    logic: クエリごとに「同じ」「同じ（小数の誤差の範囲：n セル）」「同じ（近似の範囲：n セル）」「違い」を出す。範囲内の違いは最初の 5 つ（今の LISTED_ROWS）を例として並べる。終了コードは今のまま（違いがあれば 1）で、範囲内の違いだけなら 0
    violation: なし
    source: FR7.1、Q1 A、Q2 A
  - id: BR3.1
    statement: 比べて U1・U2 の仮の決まりがコンソールと違うと分かったら、U4 の中で直す。設計の見直しが要る大きなものは利用者に諮る
    category: policy
    applies_to: U4 の作る段階（StatsAggregator・QueryParser・QueryEvaluator など中核の部品）
    trigger: 手元で比べた結果に違いがあったとき
    logic: 違いの原因が仮の決まり（BR1.4 の対応にあるもの）なら、コンソールに合わせて中核を直し、テストの期待値を直し、直したことを U4 の作る段階の記録（code-generation の記録）と、U1・U2 の rules.md の該当の BR の注記に残す。直すのが 1 つの関数の中で済まないもの（結果の型、画面、取得の流れに及ぶもの）は、直す前に利用者に諮る。コンソール側の近似や丸めによる範囲内の違いは直さない
    violation: なし
    source: FR7.3、Q4 A
  - id: BR3.2
    statement: コンソールと食い違う拡張はクエリの組に入れない
    category: constraint
    applies_to: compat_queries.txt、compat_queries_numeric.txt
    trigger: 組を書くとき
    logic: 今の決まりのまま（@log の値、日時の文字との比較は入れない）。集計では、@log を鍵にするクエリを入れない（@log の値がアカウント ID を持たないため）
    violation: なし
    source: FR7.3
```

## ルールの要約

| ID | 分類 | 要点 | 出典 |
|---|---|---|---|
| BR1.1 | policy | CloudTrail の組に集計のクエリを足す | FR7.2、Q3 |
| BR1.2 | policy | 数の集計用の新しい組 | FR7.2、Q3 |
| BR1.3 | validation | 組の 5 つ目の欄で列ごとの比べ方 | FR7.1 |
| BR1.4 | policy | 仮の決まりとクエリの対応 | FR7.3、Q4 |
| BR2.1 | policy | 集計の行の集まりを今の 7 段で比べる | FR7.1 |
| BR2.2 | calculation | 小数の誤差の範囲 | FR7.1、Q1 |
| BR2.3 | calculation | count_distinct の近似の範囲 | FR7.1、Q2 |
| BR2.4 | policy | 打ち切りの段 | FR7.1 |
| BR2.5 | policy | 範囲内の違いを分けて報告 | FR7.1 |
| BR3.1 | policy | 違いは U4 で直す、大きいものは諮る | FR7.3、Q4 |
| BR3.2 | constraint | 食い違う拡張は入れない | FR7.3 |
