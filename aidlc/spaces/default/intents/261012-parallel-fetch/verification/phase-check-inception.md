# Phase Check — Inception → Construction

**判定：PASS**（GAP・ORPHAN・不正な対象・抜けた上流 ID はない）

対象：`aidlc/spaces/default/intents/261012-parallel-fetch/inception/` の各ステージの `traceability.json`。Contract Design は要件の網羅を持たないため対象外。

## 1. 要件 → ストーリー（user-stories/traceability.json）

| 項目 | 結果 |
|------|------|
| 上流 ID の数 | 52（FR1〜FR6 とその細目、NFR1〜NFR10） |
| OK | 45 |
| N/A | 7（FR6、FR6.1〜FR6.4、NFR6、NFR7）。技術的な下準備とテストの決まりで、ストーリーにしないと決めたもの（user-stories-questions Q3）。FR6 は作業単位の外の下準備として `delivery-planning/bolt-plan.md` の「0. 下準備」で扱い、NFR6・NFR7 は全作業単位の決まり（`unit-of-work-story-map.md` の「複数の作業単位にまたがるもの」）で扱う |
| GAP・抜け | なし |

## 2. ストーリー → 部品（domain-design/traceability.json）

| 項目 | 結果 |
|------|------|
| 上流 ID の数 | 9（US1.1〜US4.2） |
| OK | 9（FetchCoordinator、StreamSchedule、AppSession、CallPacer、RetryPolicy、CancelNotice、CachePlan） |
| GAP・抜け | なし |

## 3. ストーリー → 作業単位（units-generation/traceability.json）

| 項目 | 結果 |
|------|------|
| 上流 ID の数 | 9（US1.1〜US4.2） |
| OK | 9（U1：US1.1〜US1.3、U2：US2.2、U3：US2.3、U4：US2.1、U5：US3.1・US4.1・US4.2） |
| 対象の作業単位 | U1〜U5 はすべて `unit-of-work-dependency.md` の作業単位として宣言済み |
| GAP・抜け | なし |

## 4. 持ち越し（Construction の各作業単位の機能設計で片付けるもの）

判定には影響しないが、承認時に受け入れたレビューの指摘で、作業単位の機能設計で片付けるもの。

| 作業単位 | 持ち越し |
|----------|----------|
| U1 | ドメイン設計 R-13・R-14 の一部・R-15、作業単位の切り分け R-05（RetryPolicy を担当部品に）、約束 C1 の `Retrier::call` の形、同時に読む本数と受け渡しの通り道、約束の設計 R-08（止める理由を読む時点）・R-11（StopSignal の持ち主の書き方） |
| U2 | 下げ方と戻し方の数値 |
| U3 | 作業単位の切り分け R-06（FR3.4 の書き方） |
| U4 | — |
| U5 | ドメイン設計 R-14 の一部（「止めた」で終えたストリームの状態）、約束の設計 R-07（キャッシュから表示する経路の中断）・R-09（cancelling が戻る条件）・R-10（段階を足さず cancelling にした記録） |
