# Unit of Work Story Map — アプリ内の言語切替・書き出し・JSON の整形（261022-small-features）

上流：`inception/requirements-analysis/requirements.md`（FR1〜FR5 とその小項目、NFR1〜NFR6）、`unit-of-work.md`、`inception/domain-design/components.md`。ユーザーストーリーの段はこの計画にないため、行は要件の `FR` で持つ。

## 要件と作業単位

| 要件 | Unit ID | Directory | 備考 |
|---|---|---|---|
| FR1 | U1 | u1-language-switch | 言語切替のまとまり |
| FR1.1 | U1 | u1-language-switch | SettingsDialog の言語の選択 |
| FR1.2 | U1 | u1-language-switch | AppShell の切り替え |
| FR1.3 | U1 | u1-language-switch | AppSettings の保存と起動時の読み |
| FR1.4 | U1 | u1-language-switch | 欠け・不正・読めないときの既定（言語だけ） |
| FR1.5 | U1 | u1-language-switch | AppSession：切り替えで状態を失わない |
| FR1.6 | U1 | u1-language-switch | 保存の失敗の知らせ |
| FR2 | U2 | u2-export-and-json | ログの画面の書き出しのまとまり |
| FR2.1 | U2 | u2-export-and-json | AppSession の写し（絞り込み後の表示中の行） |
| FR2.2 | U2 | u2-export-and-json | 3 列、時刻は表示の文字 |
| FR2.3 | U2 | u2-export-and-json | 行の順、メッセージ全文 |
| FR2.4 | U2 | u2-export-and-json | 行数の上限なし |
| FR2.5 | U2 | u2-export-and-json | 0 行のとき無効 |
| FR2.6 | U2 | u2-export-and-json | 取得中・走査中は無効 |
| FR3 | U2 | u2-export-and-json | クエリの画面の書き出しのまとまり |
| FR3.1 | U2 | u2-export-and-json | InsightsSession の写し |
| FR3.2 | U2 | u2-export-and-json | 表の列、`@timestamp` は表の文字 |
| FR3.3 | U2 | u2-export-and-json | `@message` は全文 |
| FR3.4 | U2 | u2-export-and-json | 行の順と数は表のまま |
| FR3.5 | U2 | u2-export-and-json | 0 行・実行中・結果なしは無効 |
| FR4 | U2 | u2-export-and-json | 書き出しの共通の決まりのまとまり |
| FR4.1 | U2 | u2-export-and-json | 形式と出し方、ExportMenu、値はそのまま |
| FR4.2 | U2 | u2-export-and-json | CSV の形（ExportFormatter） |
| FR4.3 | U2 | u2-export-and-json | Markdown の形（ExportFormatter） |
| FR4.4 | U2 | u2-export-and-json | ファイルへの保存（ExportBoundary、既定名は LogExporter） |
| FR4.5 | U2 | u2-export-and-json | クリップボードへのコピー（ExportBoundary） |
| FR4.6 | U2 | u2-export-and-json | 進み具合と二重の操作（ExportJob） |
| FR5 | U2 | u2-export-and-json | JSON の整形のまとまり |
| FR5.1 | U2 | u2-export-and-json | JsonPrettyPrinter：途中からの JSON も |
| FR5.2 | U2 | u2-export-and-json | JSON とみなす条件（機能設計 OQ1） |
| FR5.3 | U2 | u2-export-and-json | 壊れた JSON はそのまま |
| FR5.4 | U2 | u2-export-and-json | 字句を保つ |
| FR5.5 | U2 | u2-export-and-json | ExpandedMessage の切替 |
| FR5.6 | U2 | u2-export-and-json | JSON がなければ今までどおり |
| FR5.7 | U2 | u2-export-and-json | 今の振る舞いの維持 |
| FR5.8 | U2 | u2-export-and-json | 整形後の文字のコピー |
| NFR1 | U1、U2 | u1-language-switch、u2-export-and-json | 既存のテストと機能の維持（両方） |
| NFR2 | U2 | u2-export-and-json | 書き出しと整形で画面が固まらない |
| NFR3 | U1、U2 | u1-language-switch、u2-export-and-json | 両 OS で同じ振る舞い（U2 は OS の境界を触る） |
| NFR4 | U1、U2 | u1-language-switch、u2-export-and-json | テストの進め方（純粋なロジックは先行） |
| NFR5 | U1、U2 | u1-language-switch、u2-export-and-json | 秘密の扱い（設定ファイル、書き出しの中身は加工しない） |
| NFR6 | U1、U2 | u1-language-switch、u2-export-and-json | キーボード操作と読み上げ |

## 作業単位をまたぐ事項

| 事項 | かかる作業単位 | 扱い |
|---|---|---|
| AppSession・TauriApp・ApiBridge・MessageCatalog・AppShell の変更 | U1、U2 | U1 が設定の形を決め、U2 はそれに足す（`unit-of-work-dependency.md` のつなぎ目） |
| NFR1・NFR3〜NFR6 | U1、U2 | 各作業単位の確かめ方で満たす |

## 作業単位の中の実装の順序

- **U1**：MessageCatalog の言語の決定（テスト先行）→ AppSettings の読み書き（テスト先行）→ AppSession の設定の状態 → TauriApp・ApiBridge の `save_settings` → SettingsDialog と AppShell → 両 OS の目視。
- **U2**：ExportFormatter と既定のファイル名（テスト先行）→ LogExporter の写しと状態 → AppSession・InsightsSession の写しと可否 → ExportBoundary（プラグインの導入と権限）と TauriApp・ApiBridge のコマンド → ExportMenu と知らせ → JsonPrettyPrinter（テスト先行）→ ExpandedMessage の切替 → 両 OS の目視。

## 網羅の確認

- FR1〜FR5 とその小項目 35 件すべてに作業単位が割り当てられている（U1：7 件、U2：28 件）。
- NFR1〜NFR6 の 6 件すべてに作業単位が割り当てられている。
- U1 と U2 のどちらにも行がある。
