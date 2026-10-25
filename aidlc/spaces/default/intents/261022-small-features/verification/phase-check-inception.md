# Inception → Construction の境界の確かめ（261022-small-features）

元にしたもの：`inception/requirements-analysis/requirements.md`、`inception/domain-design/traceability.json`、`inception/units-generation/traceability.json`、`inception/units-generation/unit-of-work-dependency.md`。ユーザーストーリーの段はこの計画にないため、追跡の鍵は要件の `FR` である。

## 追跡の連なり

| 連なり | 件数 | 結果 |
|---|---|---|
| 要件（FR1〜FR5 とその小項目）→ 部品（ドメイン設計） | 36 / 36 | すべて OK。対象はすべて `components.md` に宣言された部品か実体 |
| 要件（FR1〜FR5 とその小項目）→ 作業単位（作業単位の分割） | 36 / 36 | すべて OK。U1 に 7 件、U2 に 29 件 |
| NFR1〜NFR6 → 作業単位 | 6 / 6 | story-map に割り当て（U1・U2 の確かめ方で満たす） |
| 作業単位の依存 | 2 単位・1 辺 | 循環なし。U2 → U1 |

## 一貫性

- 部品の一覧（新規 5・変更 9）はすべてどちらかの作業単位に含まれている（U1：7、U2：12。AppSession・TauriApp・ApiBridge・MessageCatalog・AppShell は両方）。
- 孤立した部品（要件に結びつかないもの）はない。孤立した要件（部品も作業単位もないもの）はない。
- ドメイン設計の ADR-001〜006 と作業単位の分け方に矛盾はない（書き出しは中核＋Tauri、JSON の整形は画面側、言語は設定ファイル）。

## 注意（進める前に知っておくこと）

- 要件の 3 段の ID（FR4.1.1 など）は、設計と作業単位の追跡では 2 段（FR4.1〜FR4.6）に丸めている。機能設計で業務ルールに対応づけるときは 3 段の ID まで追う。
- 作業単位の分割のレビューで残った点（R-01：言語の 2 点は U1 の機能設計で決める、R-04：種類は service のまま）は、U1・U2 の機能設計で引き取る。
- この計画に配布の段はないため、「配る」の確かめは作業単位の分割に書いた手元の目視で行う。

## 結果

通過。欠けた結び付き・孤立・矛盾はなく、Construction に進める。
