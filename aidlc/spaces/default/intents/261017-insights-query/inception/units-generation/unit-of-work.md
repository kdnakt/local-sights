# Unit of Work — Logs Insights 互換のクエリ（IB-11）

上流：`inception/domain-design/components.md`（23 部品）・`decisions.md`（ADR-001〜ADR-006）、`inception/requirements-analysis/requirements.md`（FR1〜FR9、NFR1〜NFR12）、質問票 `units-generation-questions.md`（Q1〜Q3、Q3a、分け方の計画の承認）。

分け方は「利用者に見える機能のまとまりごと」（Q1 の B ＋ 確認用プログラムの独立、Q3a）。どの作業単位も今の 1 つのデスクトップアプリ（中核ライブラリ＋Tauri アプリ＋画面）に足し、作業単位ごとに別々に配ることはない（team.md Deployment）。この作業全体を `v0.2.0` で配る。どれから作るかは作る段階の計画で決める。

最初の作業単位 U1 は、team.md の Walking Skeleton にならい、画面から AWS まで通る薄い一本にする（この計画は refactor を元にしているため骨組みのチェックポイントの段取りはないが、確かめ方は team.md の 3 つに合わせる）。

## 作業単位の一覧

| Unit ID | Directory | 名前 | 種類 | 配置 | 大きさ | 主な要件 |
|---------|-----------|------|------|------|--------|----------|
| U1 | u1-filter-skeleton | 薄い一本（filter を実行して表示） | service | アプリに組み込み | L | FR1（FR1.3 は 1 つまで）、FR3.1・FR3.2、FR5、FR6.3、FR7.2、FR7.6 |
| U2 | u2-query-language | クエリの言語をそろえる | library | ライブラリと画面に組み込み | L | FR3.3〜FR3.8、FR4、FR6.1・FR6.2・FR6.4、FR7.1・FR7.3〜FR7.5、FR8、FR9 |
| U3 | u3-shared-store | 共有の保持と取り足し | service | アプリに組み込み | XL | FR1.3（複数）、FR2 |
| U4 | u4-compat-check | 互換の確認用プログラム | library | 手元で動かす確認用プログラム（配らない） | M | SM1、NFR3、NFR4 |

## U1 薄い一本（u1-filter-skeleton）

- **説明**：Log Analytics の画面でロググループを 1 つと期間を選び、条件を組み合わせた `filter` のクエリを実行すると、取得してから手元で評価し、結果を表に出す。範囲の定義の「最初の一本」（B1）にあたる。
- **責務**：
  - AppNavigation（Log Management と Log Analytics の切り替え、閉じられる）と AppShell の変更。Log Management の画面の中身は変えない（FR1.8）
  - LogAnalyticsPage、LogGroupMultiselect（U1 では 1 つだけ選べる）、QueryEditor（CodeEditor。色分け・補完はまだ）、QueryResultTable（決まった列。行を開くとメッセージ全体）
  - QueryParser：`filter` と `|` のつなぎ、比較・and/or/not・括弧・like・=~・正規表現・in、決まった項目（@timestamp・@message・@logStream・@log）。対応外の命令（stats など）は「対応していない」とだけ返す（場所の表示と欄の中の印は U2）
  - QueryEvaluator：`filter` だけ。途中で止められる評価
  - 評価の入口の形を U1 で決める：イベントとそのロググループ名の組の並びを受け取り、結果の行を返す。U1 ではロググループは 1 つのため、選んだロググループ名をすべてのイベントの `@log` の値として渡す（暫定）。LogEvent の型は U1 では変えない（型にロググループ名を足すのは U3 だけが行う）
  - QueryRunner：読み取り → 取得 → 評価。取得は今の流れ（FetchCoordinator が保持中を捨てて取り直す、ロググループ 1 つ）を使う（Q2）
  - InsightsSession と AppSession の変更：クエリの画面の状態、押せない理由（FR1.6）、2 つの画面を合わせて取得は同時に 1 つ（ADR-005）
  - TauriApp・ApiBridge：クエリの実行・中止・結果の行のコマンドとイベント
- **含む部品**：AppNavigation、AppShell、LogAnalyticsPage、LogGroupMultiselect、QueryEditor、QueryResultTable、QueryParser、QueryEvaluator、QueryRunner、InsightsSession、AppSession、TauriApp、ApiBridge
- **確かめ方**（team.md Walking Skeleton の 3 つ）：`cargo test` と Vitest がすべて通る（各 OS のランナー）。手元の確認用プログラム（今の FetchCheck を広げる）で実際の AWS から取得して filter の結果を出す。GUI でクエリを実行して結果が出ることを目で確かめる。共通のコードを触るため、macOS と Windows の両方で確かめる。あわせて、クエリの画面で取得したあと Log Management の画面の表示中のログが入れ替わること（U3 までの暫定の動き）と、その示し方が機能設計のとおりであることを確かめる。
- **実装の注意**：
  - U1 の取得は今の流れのため、クエリを実行するたびに保持中のログを捨てて取り直す。Log Management の画面との取得済みのログの共有（FR2.1）は U3 で入る。それまでは、クエリの画面で取得すると Log Management の画面の表示中のログも入れ替わる。この動きを画面のどこにどう示すかは U1 の機能設計で決める。
  - 正規表現の重さへの備え（ADR-002）は U1 で入れる。
  - U1 の QueryRunner は、今の保持（EventTimeline）から「イベントとロググループ名の組」を作って評価の入口に渡す。U3 では、この供給元を LogStore に置き換えるだけで、評価の入口の形は変えない。

## U2 クエリの言語をそろえる（u2-query-language）

- **説明**：対応する範囲の残りをそろえる。JSON の項目・`fields`・`sort`・`limit`、エラーの場所と印、README の一覧、書く手助け。
- **責務**：
  - EventFieldReader：決まった項目と JSON の項目（入れ子は `a.b`）、壊れた JSON は項目なし（FR4）
  - QueryParser の拡張：`fields`・`sort`・`limit`、言葉の大文字・小文字と優先順位をコンソールに合わせる（OQ2）、書き間違いと対応外の区別と行・列（FR6.1、FR6.2）
  - QueryEvaluator の拡張：列の選択、並べ替え（同じ値の順番の決まり）、打ち切りと既定の上限（OQ1）
  - QueryResultTable：`fields` の列、件数と打ち切りの表示、行を開いたときの項目と値の一覧（FR7）
  - QueryEditor：エラーの下線と行番号の横の印。書く手助け（補完・色分け・例のクエリ、FR9）は Should で、時間が足りなければ外す（範囲 Q9）
  - README の対応範囲の一覧と、今の動きと食い違う記述の修正（FR8）
- **ロググループ名**：`@log` と、同じ値の並びの決まりのロググループ（FR3.4）は、U1 で決めた評価の入口が渡すロググループ名を読む。LogEvent の型には依存しないため、U3 とは独立に進められる。
- **含む部品**：EventFieldReader、QueryParser、QueryEvaluator、QueryResultTable、QueryEditor、LogAnalyticsPage、InsightsSession
- **確かめ方**：`cargo test` と Vitest がすべて通る。純粋なロジック（読み取り・評価・項目の取り出し）はテストを先に書く。手元でクエリを実行して列・並べ替え・エラーを目で確かめる（macOS と Windows）。NFR1 の数値は、この作業単位で測ってから決める（OQ3）。
- **実装の注意**：コンソールの細部（OQ1・OQ2）は機能設計で AWS の資料から確かめる。

## U3 共有の保持と取り足し（u3-shared-store）

- **説明**：2 つの画面が取得済みのログを共有し、足りない分だけを取得する。複数のロググループ（50 個まで）に対してクエリを実行できる。
- **責務**：
  - LogStore（ADR-001）、LogEvent にロググループ名を足す（型の変更はこの作業単位だけが行う）、EventTimeline の持ち方の変更
  - 評価の入口に渡す「イベントとロググループ名の組」の供給元を、U1 の暫定（選んだ 1 つの名前を全イベントに付ける）から LogStore に置き換える。評価の入口の形は変えない
  - CachePlan の足りない範囲の計算、範囲を重ねない取得と範囲をまたいで一意な順番（ADR-004）
  - FetchCoordinator：捨てずに（ロググループ, 足りない範囲）の並びを取得、キャッシュの重なる範囲の利用と書き込み
  - LogCache の照会の拡張、EventFetcher・StreamPlanner の変更
  - LogView を LogStore の上の見え方にする。Log Management の画面の見た目の振る舞いと速さのテストは今と同じに保つ（FR1.8）
  - LogGroupMultiselect を 50 個まで（ADR-006）、AppSession の接続変更での破棄、QueryRunner の取り足し・一部失敗・中止（FR2.4〜FR2.6）
- **含む部品**：LogStore、LogEvent、EventTimeline、CachePlan、LogCache、EventFetcher、StreamPlanner、FetchCoordinator、LogView、AppSession、QueryRunner、LogGroupMultiselect
- **確かめ方**：`cargo test` と Vitest がすべて通る。範囲の計算はテストを先に書く。手元で実際の AWS に対して、取得済みの範囲の一部を含む期間でクエリを実行し、足りない分だけが取得されることを確かめる。Log Management の画面が今と同じに動くことを macOS と Windows で確かめる。既存の速さのテスト（10 万件 10 秒・100 万件 100 秒）を通す。
- **実装の注意**：
  - ドメイン設計のレビューで残った点を、この作業単位の機能設計で決める：取得済みの範囲の新しさ（終了が今に近い期間の取り直し、R-01）、取り足し後のキャッシュへの書き込みの持ち主と単位（R-02）、評価中に保持が増えたときに読む版（R-03）。
  - メモリの上限は設けず、NFR2 で確かめる（components.md の Open Questions）。

## U4 互換の確認用プログラム（u4-compat-check）

- **説明**：AWS の資料にあるクエリの例のうち対応する範囲のものについて、コンソールで実行して書き出した結果と、同じロググループ・期間に対するアプリの結果を、手元で自動で比べる（SM1、NFR3）。
- **責務**：
  - 比べるクエリの組（AWS の資料の例のうち、対応する範囲に入り、開発者の手元のロググループの形に合うもの、OQ5）
  - コンソールの書き出しの読み込み、アプリの中核での評価、行の集まりの比較（並び順は `sort` を書いたクエリだけ）、結果の報告
  - コンソールの書き出しはリポジトリに置かない（NFR4）
- **含む部品**：確認用プログラム（新規。今の FetchCheck と同じく `examples/` に置く見込み）、QueryParser・QueryEvaluator・EventFieldReader を使う
- **確かめ方**：確認用プログラム自身の比較のロジックは、見本のデータで自動テストする（AWS には接続しない）。手元で実際に比べて、結果を作る段階の記録に残す。
- **実装の注意**：U3 には依存しない（1 つのロググループずつ比べる）。複数のロググループの比べ方は、U3 のあとに比べるクエリを足すかどうかで決める。
