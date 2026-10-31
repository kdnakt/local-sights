# Unit of Work Story Map — `stats` による集計と途中結果の表示（261025-insights-stats）

上流：`inception/requirements-analysis/requirements.md`（FR1〜FR7、FR1.1〜FR7.3）、`unit-of-work.md`、`inception/domain-design/components.md`。ユーザーストーリーの段はこの計画にないため、行は要件の `FR` で持つ。

## 要件と作業単位

| 要件 | Unit ID | Directory | 備考 |
|---|---|---|---|
| FR1 | U1 | u1-stats-skeleton | `stats` 命令のまとまり。U1 で読み取り・評価・結果の形を作り、関数の残りと `sort`/`limit` の後置は U2 |
| FR2 | U1 | u1-stats-skeleton | `bin()` のまとまり |
| FR3 | U1 | u1-stats-skeleton | 集計の結果の表示のまとまり |
| FR4 | U3 | u3-partial-results | 途中結果のまとまり |
| FR5 | U3 | u3-partial-results | 中断と途中結果のまとまり |
| FR6 | U2 | u2-stats-complete | 一覧・補完・色分け・文言のまとまり。FR6.3 は U1 |
| FR7 | U4 | u4-compat-stats | 互換の確かめのまとまり |
| FR1.1 | U1 | u1-stats-skeleton | `stats ... by ...`・`as`・`bin()` の書き方 |
| FR1.2 | U2 | u2-stats-complete | U1 は依頼文の 5 つの関数（6 つの書き方：`count(*)`・`count(項目)`・`sum`・`avg`・`min`・`max`、Q2 A）。残りの関数すべてを U2 で足す |
| FR1.3 | U1 | u1-stats-skeleton | `by` の鍵（決まった項目・JSON の項目・`bin()`・`as` の式）と `by` なしの 1 行 |
| FR1.4 | U2 | u2-stats-complete | `stats` の前の `fields`・`filter` は U1 から効く。`stats` のあとの `sort`・`limit` と、あとの `fields`/`filter`・2 回の `stats` の扱いは U2 |
| FR1.5 | U1 | u1-stats-skeleton | 集計の行の列と型、時刻の表示（タイムゾーン） |
| FR1.6 | U2 | u2-stats-complete | U1 は仮の決まり（機能設計で資料から確かめた分）。仕上げは U2、コンソールとの比較は U4 |
| FR1.7 | U2 | u2-stats-complete | `stats` を含むクエリの件数の上限と打ち切りの表示 |
| FR1.8 | U1 | u1-stats-skeleton | 手元で評価。Logs Insights の API は呼ばない |
| FR2.1 | U1 | u1-stats-skeleton | `bin()` の間隔と揃え方 |
| FR2.2 | U1 | u1-stats-skeleton | 空のビンを出さない |
| FR2.3 | U1 | u1-stats-skeleton | 区切りの時刻のタイムゾーン表示 |
| FR3.1 | U1 | u1-stats-skeleton | 鍵の列と集計値の列をクエリに書いた順で |
| FR3.2 | U1 | u1-stats-skeleton | 0 行の表示 |
| FR3.3 | U1 | u1-stats-skeleton | 集計の行の詳細 |
| FR3.4 | U1 | u1-stats-skeleton | 集計の行の書き出し（CSV / Markdown） |
| FR3.5 | U1 | u1-stats-skeleton | 列が多いときの横スクロール |
| FR3.6 | U1 | u1-stats-skeleton | 2 つの形を同じ場所に |
| FR4.1 | U3 | u3-partial-results | 途中結果をどのクエリにも |
| FR4.2 | U3 | u3-partial-results | `filter`・`fields` だけのクエリの途中結果。並びの文言は U3 の機能設計で改める（ADR-003） |
| FR4.3 | U3 | u3-partial-results | `sort`・`limit`・`stats` を含むクエリの差し替え |
| FR4.4 | U3 | u3-partial-results | 更新の間隔と操作の保持 |
| FR4.5 | U3 | u3-partial-results | 途中の印とどこまで終わったか |
| FR4.6 | U3 | u3-partial-results | 最終結果への切り替えと印 |
| FR4.7 | U3 | u3-partial-results | 前回の結果の印と最初の途中結果での差し替え |
| FR4.8 | U3 | u3-partial-results | 2 回数えない・落とさない |
| FR4.9 | U3 | u3-partial-results | 取得済みのときは途中結果を経ない |
| FR4.10 | U3 | u3-partial-results | 途中結果の表示中の操作と書き出しの無効 |
| FR5.1 | U3 | u3-partial-results | 中断後の印つきの最終結果 |
| FR5.2 | U3 | u3-partial-results | 中断後の書き出し。印はファイルに含めない |
| FR5.3 | U3 | u3-partial-results | 途中まで取得した範囲は確定しない |
| FR5.4 | U3 | u3-partial-results | 失敗と中断の両方の表示 |
| FR6.1 | U2 | u2-stats-complete | README の一覧 |
| FR6.2 | U2 | u2-stats-complete | QueryCatalog による一元化、補完・色分け・エラー文言 |
| FR6.3 | U1 | u1-stats-skeleton | `stats` の書き間違いのエラーと場所 |
| FR7.1 | U4 | u4-compat-stats | `compat_check` の `stats` の比較 |
| FR7.2 | U4 | u4-compat-stats | 比べるクエリの組 |
| FR7.3 | U4 | u4-compat-stats | コンソールと同じにした決まりの確かめ |

## 複数の作業単位にまたがる要件

| 要件 | 作業単位 | 分け方 |
|---|---|---|
| FR1.2 集計の関数 | U1 → U2 | U1 は依頼文の 5 つの関数（6 つの書き方）、U2 で残りすべて。確定した一覧は U1 の機能設計（OQ1）で起こし、U2 はそれに従う |
| FR1.4 命令の順番 | U1 → U2 | U1 は `stats` を最後の命令とし、あとに命令があれば「対応していない」。U2 で `sort`・`limit` の後置 |
| FR1.6 鍵のないイベント・数値でない値 | U1 → U2 → U4 | U1 は仮の決まり、U2 で仕上げ、U4 でコンソールと比べる |
| FR3.4・FR5.2 書き出し | U1 → U3 | U1 で形つきの書き出し、U3 で中断後の結果も書き出せるようにする |
| FR6.2・FR6.3 エラー | U1 → U2 | U1 は `stats` の書き間違いの場所つきのエラー、U2 で文言を一覧から作る |
| NFR1・NFR2 速さと応答性 | U1・U2・U3 | 集計の速さは U1・U2 の確かめ方で測り、途中結果の間隔と取得を待たせないことは U3 で測る（OQ3） |
| NFR7・NFR8 テストとカバレッジ | U1〜U4 | 純粋なロジック（StatsAggregator、途中の view、比較）はテストを先に書く。80% を下回らない |
| NFR9・NFR10 文言とアクセシビリティ | U1・U3 | 集計の文言は U1、途中・中断の印と読み上げは U3 |

## 単位の中の主な流れ（目安。単位どうしの順ではなく、作る段階の計画で決める）

- **U1**：QueryPlan の `Stats` と列の導出 → QueryParser の `stats`/`by`/`bin()` → StatsAggregator（テスト先行） → QueryEvaluator の形つきの結果 → InsightsSession の表示と書き出しの元 → ExportJob の型 → TauriApp・ApiBridge のコマンド → QueryResultTable・LogAnalyticsPage・MessageCatalog → 手元の確認
- **U2**：U1 の機能設計の関数の一覧に沿って StatsAggregator に関数を足す（テスト先行） → `sort`/`limit` の後置と上限 → QueryCatalog と QueryParser・補完の参照 → TauriApp・ApiBridge・AnalyticsHelpers・QueryEditor・MessageCatalog → README と一致のテスト
- **U3**：QueryStore の途中の view と版（テスト先行） → QuerySink の通知と QueryRunner の途中の評価・中断後の評価 → InsightsSession の版と印 → TauriSink・TauriApp・ApiBridge のイベントとコマンド → QueryResultTable・LogAnalyticsPage・MessageCatalog → 間隔と NFR1 の計測
- **U4**：比較のロジック（見本のデータでテスト） → クエリの組 → 手元での比較と記録

## 網羅の確認

- 要件の ID 44 個（FR1〜FR7 と FR1.1〜FR7.3）はすべて 1 つの作業単位に割り当てた。
- 4 つの作業単位すべてに行がある（U1：FR1・FR2・FR3 と FR6.3、U2：FR1.2・FR1.4・FR1.6・FR1.7・FR6、U3：FR4・FR5、U4：FR7）。
- `traceability.json` の対応先は、この表の Unit ID と一致する。
