# Unit of Work — `stats` による集計と途中結果の表示（261025-insights-stats）

上流：`inception/domain-design/components.md`（新規 2 部品・変更 16 部品）・`decisions.md`（ADR-001〜ADR-006）、`inception/requirements-analysis/requirements.md`（FR1〜FR7、NFR1〜NFR12、C1〜C5）、質問票 `units-generation-questions.md`（Q1〜Q3、分け方の計画の承認）。

分け方は「利用者に見える機能のまとまりごと」（Q1 A）。依頼の 2 つの柱（`stats` と途中結果）を、結果の形を先に決める薄い一本（U1）、`stats` を仕上げる U2、途中結果の U3 に分け、互換の確かめは前の作業と同じく独立の U4 にした。どの作業単位も今の 1 つのデスクトップアプリ（中核ライブラリ＋Tauri アプリ＋画面）に足し、作業単位ごとに別々に配ることはない（team.md Deployment、C4：この作業だけでは配らず次の版で配る）。どれから作るかは作る段階の計画で決める。

最初の作業単位 U1 は、team.md の Walking Skeleton にならい、画面から AWS まで通る薄い一本にする（この計画は refactor を元にしており骨組みのチェックポイントの儀式はない。確かめ方は team.md の 3 つに合わせる）。

## 作業単位の一覧

| Unit ID | Directory | 名前 | 種類 | 配置 | 大きさ | 主な要件 |
|---------|-----------|------|------|------|--------|----------|
| U1 | u1-stats-skeleton | 薄い一本（`stats` の最小版を画面から AWS まで） | service | アプリに組み込み | L | FR1.1・FR1.3・FR1.5・FR1.8、FR2、FR3.1〜FR3.6、FR6.3、FR1.2 の 6 つの書き方（`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`。依頼文の 5 つの関数） |
| U2 | u2-stats-complete | `stats` をそろえる（関数すべて・`sort`/`limit`・一覧の一元化） | library | ライブラリと画面に組み込み | L | FR1.2・FR1.4・FR1.6・FR1.7、FR6 |
| U3 | u3-partial-results | 途中結果と中断後の結果 | service | アプリに組み込み | L | FR4、FR5 |
| U4 | u4-compat-stats | 互換の確かめ（`stats` の比較） | library | 手元で動かす確認用プログラム（配らない） | S | FR7 |

## U1 薄い一本（u1-stats-skeleton）

- **説明**：Log Analytics の画面で `stats count(*) by bin(5m)` のような集計のクエリを実行すると、今までどおり取得してから手元で集計し、集計の行（鍵の列と集計値の列）を結果の表に出し、行を開けば鍵と集計値が見え、CSV / Markdown に書き出せる。結果の形の変更（ADR-001）を中核・Tauri・画面・書き出しでそろえて通すことがねらい（C3）。
- **責務**：
  - QueryPlan：`CommandKind::Stats`（集計の指定と `by` の鍵）、結果の列の名前と形の導出（FR3.1）
  - QueryParser：`stats <集計>, ... [by <鍵>, ...]`・`as`・`bin(<間隔>)` の読み取り（FR1.1、FR1.3）。U1 で読める関数は Q2 A の 6 つの書き方（`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`。依頼文の 5 つの関数）。他の集計の関数は `stats` の中では「対応していない」の問題として返す（場所つき）。振り分けは QueryCatalog なしの暫定の表（今の `OTHER_COMMANDS`・`function()` に足す）で行い、U2 で QueryCatalog に置き換える（units レビュー R-02）。`stats` の書き間違い（`by` のあとに何もない、閉じていない括弧、引き数の数の違い）のエラー（FR6.3）
  - StatsAggregator（新規）：`by` の鍵によるグループ化、5 つの関数の累積、`bin()` の区切り（UTC のエポック基準、A8）と空のビンを出さない決まり（FR2.1〜FR2.3）、型つきの集計値と数値の桁・丸め（FR1.5、A7）。鍵の項目を持たないイベントと数値でない値の扱い（FR1.6）は、この作業単位の機能設計で AWS の資料から確かめ、確かめられないものは仮の決まりにして U4 でコンソールに合わせる
  - QueryEvaluator：形つきの QueryResult（イベントの行／集計の行の列挙型、列の名前の一覧。行はすべて写し）。計画に `stats` があれば `filter` で残ったイベントを StatsAggregator に塊ごとに渡す（FR1.8、ADR-001・002）。`stats` のあとの `sort`・`limit` は U2（U1 では `stats` を最後の命令とし、あとに命令があれば「対応していない」とする）
  - InsightsSession：結果の形に従う行と詳細の表示（集計の行の詳細は鍵と集計値だけ、FR3.3）、時刻のセルの画面のタイムゾーンでの整形（FR2.3）、書き出しの元（`export_source`）を形つきに（FR3.4）、0 行の扱い（FR3.2）
  - ExportJob：`ExportSnapshotSource::LogAnalytics` を形つきの行に（FR3.4）。ExportFormatter は変えない
  - TauriApp・ApiBridge：結果の見出し（形・列の名前）と形つきの行・詳細のコマンド（今の `get_query_rows` / `get_query_row_detail` を広げる）
  - QueryResultTable・LogAnalyticsPage：形に従った表（集計の行はセルをそのまま）、行の詳細、横スクロール（FR3.5）、同じ場所に 2 つの形（FR3.6）、集計の 0 行の表示。MessageCatalog に集計の 0 行と `stats` のエラーの文言（NFR9）
  - 結果の型の変更に追従するだけの最小の変更（units レビュー R-01）：QueryStore（`evaluate_step`・`copy_rows` が形つきの行を返す）、QueryRunner（`on_evaluated` に形つきの QueryResult を渡す。流れは変えない）、TauriSink（結果の写しの型）、CompatCheck（イベントの形の結果が今までどおり比べられるように型だけ合わせる。`stats` の比較は U4）。途中の view・版・中断後の評価は U3
- **含む部品**：QueryPlan、QueryParser、StatsAggregator、QueryEvaluator、InsightsSession、ExportJob、TauriApp、ApiBridge、QueryResultTable、LogAnalyticsPage、MessageCatalog、（型の追従のみ）QueryStore、QueryRunner、TauriSink、CompatCheck
- **確かめ方**（team.md Walking Skeleton の 3 つ）：`cargo test` と Vitest がすべて通る（各 OS のランナー）。手元の確認用プログラム（今の FetchCheck を最小限広げる。`compat_check` は使わない、units レビュー R-06）で実際の AWS から取得して `stats count(*) by bin(5m)` の結果を出す。取得済み 100 万件に対する `stats` の最終結果までの時間を測って記録し、NFR1 の数値の材料にする（units レビュー R-07）。GUI でクエリを実行して集計の行が表に出て、開けて、書き出せることを目で確かめる。共通のコードを触るため、macOS と Windows の両方で確かめる。あわせて、`filter`・`fields`・`sort`・`limit` だけのクエリが前と同じ結果になること（C5）を確かめる。
- **実装の注意**：
  - 結果の形の変更は中核・Tauri・画面・書き出し・確認用プログラムをそろえて変える（C3）。`compat_check` はイベントの形の結果が今までどおり比べられるように型だけ合わせ、`stats` の比較は U4
  - 評価は 10,000 行ずつ進める今の流れに StatsAggregator を乗せる（NFR2）
  - 集計の関数の確定した一覧（OQ1）と、`stats` の引き数と `by` の鍵に書ける式の一覧（RA レビュー R-05）、イベントが 0 件のときの `by` なしの `stats`（RA レビュー R-04）は、U1 の機能設計で AWS の資料から確かめて決める。U2 はその一覧に残りの関数を足す
  - ドメイン設計のレビュー（DD レビュー）の持ち越しのうち、行は写しで保持を指さない（DD R-01）、時刻のセルは表示時にタイムゾーンで整形（DD R-05）、QueryEvaluator と QueryStore の依存（DD R-09）は U1 の機能設計で決める。持ち越し全体の対応先は末尾の表

## U2 `stats` をそろえる（u2-stats-complete）

- **説明**：コンソールの `stats` で使える集計の関数をすべて入れ、`stats` のあとの `sort`・`limit` を書けるようにし、命令・関数の一覧を 1 か所（QueryCatalog）にまとめて、補完・色分け・エラー文言・README をそろえる。
- **責務**：
  - StatsAggregator：残りの関数（`count_distinct`・`pct`・`stddev`・`earliest`・`latest`・`sortsFirst`・`sortsLast` など。確定した一覧は U1 の機能設計の OQ1 に従う）の累積と型（FR1.2）、数値でない値や鍵のないイベントの扱いの仕上げ（FR1.6）
  - QueryParser・QueryEvaluator：`stats` のあとの `sort`・`limit`（集計の結果の列、`as` の名前で並べ替えと件数の制限、FR1.4）、`stats` を含むクエリの結果の件数の上限と打ち切りの表示（FR1.7）、`stats` のあとの `fields`・`filter` と `stats` を 2 回書くの扱い（OQ2）
  - QueryCatalog（新規）：命令・集計の関数・その他の関数・キーワードの一覧（ADR-005）。U1 の暫定の振り分けの表（`OTHER_COMMANDS`・`function()` と補完の固定の候補）をこれに置き換え、QueryParser と InsightsSession の補完はこれを参照する（units レビュー R-02）
  - TauriApp・ApiBridge・AnalyticsHelpers・QueryEditor：一覧を画面に渡し、色分けの規則と「使える命令」のエラー文言を一覧から作る（FR6.2）。補完の候補に `stats`・`by`・集計の関数（FR6.2）
  - MessageCatalog：`supportedList` を一覧を差し込む形に（FR6.2）
  - README：対応する命令・関数の一覧に `stats`・集計の関数・`bin()` を足し、「Not supported」から外す。一覧との一致をテストで確かめる（FR6.1、NFR7）
- **含む部品**：StatsAggregator、QueryParser、QueryPlan、QueryEvaluator、QueryCatalog、InsightsSession、TauriApp、ApiBridge、AnalyticsHelpers、QueryEditor、QueryResultTable（打ち切りの表示）、LogAnalyticsPage、MessageCatalog
- **確かめ方**：`cargo test` と Vitest がすべて通る。関数ごとの累積はテストを先に書く。README と QueryCatalog の一致のテスト。手元で各関数のクエリと `stats ... | sort n desc | limit 10` を実行して目で確かめる（macOS と Windows）。
- **確かめ方（速さ）**：関数すべてを含む `stats` の最終結果までの時間を U1 と同じ条件で測り、U1 より遅くならないことを確かめる（units レビュー R-07）
- **実装の注意**：`pct`・`count_distinct` など値を保持する関数のメモリ（DD レビュー R-10）は、この作業単位の機能設計で上限の扱いを決める。U3 と重なる部品の分担は次の表のとおりで、どちらも U1 で決めた結果の形とコマンドの内側の変更にとどめる（Q3 A、units レビュー R-03）

| 重なる部品 | U2 が触るところ | U3 が触るところ |
|---|---|---|
| QueryEvaluator | 集計の行への `sort`・`limit`、件数の上限と打ち切り | 触らない（途中の view を入力に同じ評価を呼ぶだけ） |
| InsightsSession | 補完の候補（QueryCatalog）、打ち切りの状態 | 結果の版、途中・最終・中断・前回の印、書き出しの可否 |
| TauriApp・ApiBridge・SessionView | 一覧を返すコマンド、打ち切りの項目 | 版が進んだイベント、結果の見出しと行のコマンドへの版の追加、印の項目 |
| QueryResultTable | 打ち切りの注記 | 版に従った差し替え、操作の保持、途中・最終・中断・前回の印 |
| LogAnalyticsPage | 「使える命令」の文言 | 途中結果でも表を描く、中断の表示、書き出しの可否 |
| MessageCatalog | 一覧を差し込む文、打ち切り | 途中・最終・中断の印の文言 |

同じファイルを両方が変えるため、作る段階で並べて進めるときは U1 の形を共有の土台にし、マージで項目を足し合わせる。

## U3 途中結果と中断後の結果（u3-partial-results）

- **説明**：クエリの実行中、取得が進むにつれて途中結果が結果の表に出て差し替わり、取得が終わると最終結果になる。［Cancel］を押すと、止めた時点までの分で評価した結果が「一部のログだけの結果（中断）」の印つきで残る。
- **責務**：
  - QueryStore：確定済みの分と取得中の保持の写しで途中の view を作る入口（epoch とは別の版つき、ADR-003、DD レビュー R-03・R-08）
  - QueryRunner：一定の間隔ごとの途中の評価（取得と並行、別のタスク。前の評価が終わらないうちは次を始めない）、版の番号、QuerySink への途中結果の通知、取得の終了・中断のときに動いている途中の評価を止めてから最終の view を作る順番（DD レビュー R-02・R-03）、中断後の 1 回の評価と印（FR5.1〜FR5.4、ADR-006、DD レビュー R-04：中断後の評価は中断の合図で止めない）、取得済みのときは途中結果を経ない（FR4.9）
  - InsightsSession：最新の版の結果（途中か最終か）、前回の結果の印と最初の途中結果での差し替え（FR4.7、ADR-006）、途中・最終・中断・失敗の状態、古い版と別の runId の結果を捨てる（A4）、実行中は書き出し不可・中断後は可（FR4.10、FR5.2）
  - TauriSink・TauriApp・ApiBridge：途中結果の通知を受けて「結果の版が進んだ」イベント（間隔を絞る、最終と中断後はすぐ）、版つきの結果の見出しと行のコマンド（ADR-004）
  - QueryResultTable・LogAnalyticsPage・MessageCatalog：版に従った差し替えとスクロール位置・開いた行・選択の保持（FR4.4）、途中であることとどこまで終わったかの印、最終・中断・前回の印（FR4.5・FR4.6・FR5.1、NFR10：色だけに頼らず、読み上げは状態の変化だけ）、途中結果でも表を描く（FR4.1）、中断の表示の変更
- **含む部品**：QueryStore、QueryRunner、InsightsSession、TauriSink、TauriApp、ApiBridge、QueryResultTable、LogAnalyticsPage、MessageCatalog
- **確かめ方**：`cargo test` と Vitest がすべて通る。途中の view の版と、2 回数えない・落とさない決まり（FR4.8）はテストを先に書く。手元で実際の AWS に対して取得に時間のかかる期間のクエリを実行し、行が増えて差し替わること、最終結果になること、［Cancel］で印つきの結果が残ることを目で確かめる（macOS と Windows）。確かめるクエリは U1 で動く範囲（`filter`・`fields`・`sort`・`limit` だけのクエリと、`stats` を最後の命令にしたクエリ）に限る。`stats ... | sort ...` の途中結果（FR4.3 の一部）は U2 がそろったあとに確かめる（units レビュー R-03）。ログ管理の画面が今と同じに動くこと（C5）も確かめる。
- **実装の注意**：
  - 途中結果の更新の間隔の数値と NFR1 の合否（OQ3）は、この作業単位で測ってから決める
  - FR4.2 の文言（「末尾に足す・動かない」→「最終結果と同じ並びに保ち、行が途中に入ることがある」）は、この作業単位の機能設計で要件の記述を改める（ADR-003、RA レビュー R-01）
  - 中断の印をファイルに含めない理由（RA レビュー R-08）は、この作業単位の機能設計で書く
  - DD レビューの持ち越しのうち、保持の登録先と並行モデル（DD R-02）、途中の view の版と最終の view を作る順番（DD R-03）、中断後の評価の相と 2 回目の［Cancel］（DD R-04）、「確定済み」の用語（DD R-08）は、この作業単位の機能設計で決める
  - 前の作業の中断のテスト（評価しない）は FR5.1 に合わせて直す。ログ管理の画面の中断は変えない

## U4 互換の確かめ（u4-compat-stats）

- **説明**：手元の確認用プログラム（`compat_check`）に、`stats` を含むクエリの比べ方を足し、互換を確かめるクエリの組に集計のクエリを足す（FR7）。
- **責務**：
  - CompatCheck：集計の行の集まり（鍵と集計値）の一致の比較、`sort` を書いたクエリだけ並び順も比べる（FR7.1）。比べる文字は StatsAggregator と InsightsSession の整形と同じ
  - 比べるクエリの組：`count(*) by bin(5m)`、`by` の鍵が JSON の項目、`avg`/`sum`/`max`/`min`、`count_distinct`、`pct`、`stats` のあとの `sort`/`limit`（FR7.2。具体例は機能設計の OQ4）。空のビンを出さないなどコンソールと同じにした決まりを組で確かめ、食い違う拡張は入れない（FR7.3）
  - U1・U2 で仮の決まりにした点（鍵のないイベント、数値でない値、丸めなど）をコンソールと比べ、食い違いを記録する
- **含む部品**：CompatCheck（`examples/compat_check.rs`、`examples/compat/*`、`examples/compat_queries.txt`）
- **確かめ方**：比較のロジックは見本のデータで自動テストする（AWS には接続しない、NFR4）。手元で実際に比べ、結果を作る段階の記録に残す。コンソールから書き出した結果はリポジトリに置かない。
- **実装の注意**：U3 には依存しない（最終結果だけを比べる）。U2 に依存する（関数すべてと `sort`/`limit` の後置がそろってから）

## レビューの持ち越しと解く作業単位

RA レビューは `inception/requirements-analysis/reviews/review-01.md`、DD レビューは `inception/domain-design/reviews/review-01.md` の番号。

| 持ち越し | 中身 | 解く作業単位（機能設計） |
|---|---|---|
| RA R-01 | FR4.2 の並びの文言 | U3（ADR-003 のとおり改める） |
| RA R-02 | 形の切り替えの時点 | U1（結果の形）・U3（前回の結果の印と差し替え） |
| RA R-04 | イベントが 0 件のときの `by` なしの `stats` | U1 |
| RA R-05 | `stats` の引き数と `by` の鍵に書ける式 | U1 |
| RA R-06 | 最初の途中結果の時間の目安、読み上げ | U3 |
| RA R-07 | C5 の例外、古い版の結果 | U3 |
| RA R-08 | 中断の印をファイルに含めない理由 | U3 |
| DD R-01 | 行は写しで保持を指さない | U1 |
| DD R-02 | 保持の登録先、並行モデル、写しの順序 | U3 |
| DD R-03 | 途中の view の版、最終の view を作る順番 | U3 |
| DD R-04 | 中断後の評価の相、2 回目の［Cancel］ | U3 |
| DD R-05 | 時刻のセルの表示時の整形 | U1 |
| DD R-06 | QueryPlan の持ち主（文書の訂正、済み） | — |
| DD R-07 | 書き出しの元は InsightsSession（文書の訂正、済み） | U1 で実装 |
| DD R-08 | 「確定済み」の用語 | U3 |
| DD R-09 | QueryEvaluator と QueryStore の依存 | U1 |
| DD R-10 | 値を保持する関数のメモリ、評価の中断の確認 | U2（メモリ）・U3（中断の確認） |
