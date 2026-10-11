# Design System Mapping — Cloudscape Design System への対応

上流：`inception/requirements-analysis/requirements.md`（FR4.1、FR4.5、FR4.6、NFR1、NFR5、CN2）、`mockups.md`、質問票 Q1〜Q6。

## 採用するパッケージ

| パッケージ | 用途 | ライセンス |
|---|---|---|
| `@cloudscape-design/components` | 見える部品すべて（FR4.1） | Apache-2.0 |
| `@cloudscape-design/global-styles` | 全体のスタイルと明暗モードの切り替え（`applyMode`） | Apache-2.0 |
| `@cloudscape-design/design-tokens` | 自前実装が残る場合（ログ一覧の仮想スクロール、Q3）の配色・余白・文字 | Apache-2.0 |

依存の追加は CI の依存関係チェックと秘密情報スキャンを通す（NFR5）。Rust 側に依存は足さない。

## 既存部品 → Cloudscape 部品の対応

| 今の部品（src/components） | 今の実装 | Cloudscape の部品 | 備考 |
|---|---|---|---|
| App のレイアウト | flex の 2 ペイン | `Grid`（左 3 / 右 9 相当）＋ `ContentLayout` | AppLayout は使わない（ナビゲーションドロワーが不要） |
| ConnectionBar | `<select>` ×2、ラジオ、`[*]` ボタン | `TopNavigation`（utilities：`menu-dropdown` ×3、`button` with `iconName="settings"`） | Q1。タイムゾーンのラジオはドロップダウンに変わる |
| TimeZoneToggle | ラジオ | `TopNavigation` の `menu-dropdown` | 上に統合 |
| LogGroupPane | 検索欄＋ `<ul role="listbox">` | `Container` + `Header` + `TextFilter` + `Table`（`selectionType="single"`、`variant="embedded"`）+ `Button`（Reload） | Q6 |
| FetchForm | 見出し＋ `<input type="text">` ×2 ＋ `<button>` ＋理由の箇条書き | `Container` + `Header`（actions に `Button variant="primary"`）+ `FormField` + `DatePicker` + `TimeInput` | Q2。理由の箇条書きは `FormField.errorText` に |
| LogFilterInput | `<input type="search">` | `TextFilter` または `Input type="search"` | 条件行の左 |
| FetchErrorBanner / ErrorMessage | `<div role="alert">`＋ `<p>` ×3 | `Alert type="error" dismissible header={what}` | FR3.2 |
| FailureList | `<section role="dialog">` | `Modal` + `Table` | FR3.3 |
| StatusLine | `<div role="status">` | `Box` + `StatusIndicator` + `Button variant="link"` | FR3.3 |
| LogTable / ExpandedMessage | 自前の仮想スクロール | `Table`（まず）→ 満たせなければ自前＋トークン | Q3、NFR2 |
| ConfirmDialog / CloseConfirmDialog | 自前のダイアログ | `Modal`（footer に `SpaceBetween` でボタン） | |
| SettingsDialog | 自前のダイアログ | `Modal` + `Checkbox` + `Box` + `Button` | |

## 日付・時刻の部品の設定

| 部品 | 設定 | 理由 |
|---|---|---|
| `DatePicker` | 表示形式は `yyyy/mm/dd`（`locale` を `ja` または `en` に合わせたうえで、入力マスクは `YYYY/MM/DD`）。`value` は内部表現（ISO の `YYYY-MM-DD`）を画面側で `yyyy/mm/dd` に変換してコアへ渡す | FR2.1 |
| `DatePicker` の `i18nStrings` | 「次の月」「前の月」「今日」などを英・日で用意。月名・曜日は `locale` に従う | FR4.6 |
| `TimeInput` | `format="hh:mm:ss"`、`use24Hour` | FR1.3 |
| `FormField` | `label` にタイムゾーンを添える。`errorText` に理由 | FR3.1 |

## 明暗モード（FR4.5、Q6 の要件回答）

- 起動時と `prefers-color-scheme` の変化で `applyMode(Mode.Light | Mode.Dark)` を呼ぶ。再起動は不要。
- 独自の色は使わない。自前実装が残る部品はデザイントークン（`colorBackgroundContainerContent`、`colorBorderDividerDefault`、`colorTextBodyDefault`、`fontFamilyMonospace` など）を使う。

## 言語（FR4.6）

- `I18nProvider` に `@cloudscape-design/components/i18n/messages/all.ja` と `all.en` を渡し、OS の言語（既存の `Locale`）で切り替える。
- アプリ独自の文言は既存の `i18n/messages.ts` を使い続ける（interaction-spec.md §9 の追加分を足す）。

## 配置と寸法

| 項目 | 値 |
|---|---|
| 最小ウィンドウ | 1024×640（NFR7） |
| 左ペインの幅 | 18rem（今のまま） |
| 条件行 | 絞り込み欄は残り幅、日付欄は 10em、時刻欄は 7em。1 行に収まらなければ日付・時刻を下に折り返す |
| 表の行の高さ | Cloudscape の Table の既定。自前実装の場合は 22px のまま |
| 余白 | Cloudscape の `SpaceBetween size="s"`／`"m"` に統一 |
