# Unit of Work Story Map — Logs Insights 互換のクエリ（IB-11）

上流：`inception/requirements-analysis/requirements.md`（FR1.1〜FR9.3）、`unit-of-work.md`、`inception/domain-design/components.md`。ユーザーストーリーの段はこの計画にないため、行は要件の `FR` で持つ。

## 要件と作業単位

| 要件 | Unit ID | Directory | 備考 |
|---|---|---|---|
| FR1.1 | U1 | u1-filter-skeleton | — |
| FR1.2 | U1 | u1-filter-skeleton | — |
| FR1.3 | U3 | u3-shared-store | U1 では 1 つだけ選べる形で作り、U3 で複数（50 個まで）にする |
| FR1.4 | U1 | u1-filter-skeleton | — |
| FR1.5 | U1 | u1-filter-skeleton | — |
| FR1.6 | U1 | u1-filter-skeleton | — |
| FR1.7 | U1 | u1-filter-skeleton | — |
| FR1.8 | U1 | u1-filter-skeleton | 部分一致の絞り込みは U1 から今のまま。ただし U3 までは、クエリの画面で取得すると Log Management の表示中のログも入れ替わる（U1 の暫定の動き。承認と引き継ぎの段で許した）。U3 で共有の保持に載せ替えたあとは入れ替わらない（U3 にもかかる） |
| FR2.1 | U3 | u3-shared-store | — |
| FR2.2 | U3 | u3-shared-store | — |
| FR2.3 | U3 | u3-shared-store | — |
| FR2.4 | U3 | u3-shared-store | — |
| FR2.5 | U3 | u3-shared-store | — |
| FR2.6 | U3 | u3-shared-store | — |
| FR2.7 | U3 | u3-shared-store | — |
| FR2.8 | U3 | u3-shared-store | — |
| FR3.1 | U1 | u1-filter-skeleton | — |
| FR3.2 | U1 | u1-filter-skeleton | — |
| FR3.3 | U2 | u2-query-language | — |
| FR3.4 | U2 | u2-query-language | — |
| FR3.5 | U2 | u2-query-language | — |
| FR3.6 | U2 | u2-query-language | — |
| FR3.7 | U2 | u2-query-language | — |
| FR3.8 | U2 | u2-query-language | — |
| FR4.1 | U2 | u2-query-language | `@log` は評価の入口が渡すロググループ名を読む（U1 で入口を決める。LogEvent の型の変更は U3） |
| FR4.2 | U2 | u2-query-language | — |
| FR4.3 | U2 | u2-query-language | — |
| FR4.4 | U2 | u2-query-language | — |
| FR4.5 | U2 | u2-query-language | — |
| FR5.1 | U1 | u1-filter-skeleton | — |
| FR5.2 | U1 | u1-filter-skeleton | — |
| FR5.3 | U1 | u1-filter-skeleton | — |
| FR5.4 | U1 | u1-filter-skeleton | — |
| FR5.5 | U1 | u1-filter-skeleton | — |
| FR6.1 | U2 | u2-query-language | U1 は対応外の命令を「対応していない」と返すだけ。場所と印は U2 |
| FR6.2 | U2 | u2-query-language | 同上 |
| FR6.3 | U1 | u1-filter-skeleton | — |
| FR6.4 | U2 | u2-query-language | — |
| FR7.1 | U2 | u2-query-language | — |
| FR7.2 | U1 | u1-filter-skeleton | — |
| FR7.3 | U2 | u2-query-language | — |
| FR7.4 | U2 | u2-query-language | — |
| FR7.5 | U2 | u2-query-language | — |
| FR7.6 | U1 | u1-filter-skeleton | U3 で取り足しが入っても保つ |
| FR8.1 | U2 | u2-query-language | — |
| FR8.2 | U2 | u2-query-language | — |
| FR9.1 | U2 | u2-query-language | — |
| FR9.2 | U2 | u2-query-language | — |
| FR9.3 | U2 | u2-query-language | — |

## 複数の作業単位にかかるもの

| 要件・決まり | かかる作業単位 | 中身 |
|---|---|---|
| FR1.3 ロググループの選択 | U1、U3 | U1 は 1 つ、U3 で 50 個まで |
| FR1.8 Log Management の画面は変えない | U1、U3 | 画面を分けるとき（U1）と、取得を共有の保持に載せ替えるとき（U3）の両方で、今の振る舞いと速さのテストを保つ。U1〜U3 の間は表示中のログが入れ替わる暫定の動きがあり、U1 の確かめ方で確かめる |
| ロググループ名（FR2.8・FR3.4・FR4.1） | U1、U2、U3 | U1 が評価の入口でロググループ名を渡す形を決め（1 つの名前を全イベントに付ける暫定）、U2 はそれを読み、U3 だけが LogEvent の型に名前を足して供給元を LogStore に変える |
| FR6.1・FR6.2 エラー | U1、U2 | U1 は対応外の命令の検出だけ、U2 で場所・印・書き間違い |
| NFR1・NFR2 速さと容量 | U2、U3 | 評価の速さは U2 で測って数値を決め、共有の保持の容量は U3 で確かめる |
| NFR6 macOS と Windows | U1〜U3 | 共通のコードを触るため、どの作業単位も両方の OS で確かめる |
| SM1・NFR3・NFR4 互換の確かめ | U4 | 確認用プログラム。U2 のあと |

## 作業単位の中の作る順

- **U1**：QueryParser（filter）と QueryEvaluator（filter）をテスト先行で作る → QueryRunner と InsightsSession → Tauri のコマンド → 画面（ナビゲーション、Log Analytics の画面）→ 手元の確認。
- **U2**：EventFieldReader → QueryParser の拡張（fields・sort・limit・エラーの場所）→ QueryEvaluator の拡張 → 結果の表と行の項目と値 → エラーの印 → README → 書く手助け（Should）。
- **U3**：CachePlan の足りない範囲の計算（テスト先行）→ LogEvent と EventTimeline の変更 → LogStore → EventFetcher・StreamPlanner・FetchCoordinator → LogCache → LogView の載せ替え（速さのテスト）→ QueryRunner の取り足し → 画面の複数選択。
- **U4**：比べるクエリの組を決める → 書き出しの読み込みと比較（テスト）→ 手元で実行。

## 対応の確かめ

- 要件 49 件（FR1.1〜FR9.3）は、すべていずれかの作業単位に割り当てた。
- どの作業単位にも行がある（U4 は FR ではなく SM1・NFR3・NFR4 にかかる。FR の行は持たないが、上の「複数の作業単位にかかるもの」に行がある）。
- `traceability.json` は機能要件（FR）だけを追う決まりとする。NFR・SM と U4 の割り当ては、上の「複数の作業単位にかかるもの」の表で持つ。
