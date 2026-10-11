# Unit Test Instructions — 画面を AWS マネジメントコンソールに寄せる

計画は `code-generation-plan.md`。テスト戦略は Standard（部品ごとに 5〜8 テスト）、スコープ bugfix の下限として「対象の回帰テスト」と「既存スイートが緑のまま」を加える。Testing Contract の順序：純粋なロジック（コアの書式、画面側の日付・時刻の処理）はテストを先に書き、画面の部品は実装してからテストを書く。

## テストの枠組み

| 層 | 枠組み | 設定 |
|---|---|---|
| コア（Rust） | `cargo test`（モジュール内の `#[cfg(test)]`） | 追加設定なし。AWS には接続しない |
| 画面の純粋ロジック・部品（TypeScript/React） | vitest + @testing-library/react（jsdom） | `vitest.config.ts`（`src/test/setup.ts`、`include: src/**/*.test.{ts,tsx}`）。Tauri の API は `src/test/` の既存モックで差し替える |

## この作業のテストだけを動かすコマンド

最初のテスト先行ステップ（Step 3）の前に、次が今の状態で通ることを確認する（Step 2）：

```
cargo test -p local-sights-core time_range
npx vitest run src/components/FetchForm.test.tsx
```

ステップごとのコマンド（すべてこの作業の対象だけに絞る）：

| ステップ | コマンド |
|---|---|
| Step 3 コアの書式（Red → Green） | `cargo test -p local-sights-core time_range date_input` |
| Step 3 コア全体（旧書式の洗い出し後） | `cargo test -p local-sights-core` |
| Step 4 画面側の純粋ロジック（Red → Green） | `npx vitest run src/dateTime.test.ts` |
| Step 5 基盤 | `npx vitest run src/App.test.tsx src/hooks/useColorScheme.test.ts` |
| Step 6 上部ナビゲーション | `npx vitest run src/components/TopBar.test.tsx` |
| Step 7 左ペイン | `npx vitest run src/components/LogGroupPane.test.tsx` |
| Step 8 条件エリア | `npx vitest run src/components/ConditionArea.test.tsx src/components/LogFilterInput.test.tsx` |
| Step 9 エラー・ダイアログ・ステータス行 | `npx vitest run src/components/FetchErrorBanner.test.tsx src/components/FailureList.test.tsx src/components/StatusLine.test.tsx src/components/ConfirmDialog.test.tsx src/components/CloseConfirmDialog.test.tsx src/components/SettingsDialog.test.tsx src/components/ErrorMessage.test.tsx` |
| Step 10 ログ一覧 | `npx vitest run src/components/LogTable.test.tsx src/rowLayout.test.ts src/virtualScroll.test.ts` |
| Step 11 文言 | `npx vitest run src/i18n/messages.test.ts src/errorText.test.ts` |
| Step 14c 起動時の選択（Red → Green） | `cargo +stable test -p local-sights-core session` と `cargo +stable test -p local-sights-core --test u2_log_group_listing` |
| Step 14a・14b・14d・14f 画面 | `npx vitest run src/components/ConditionArea.test.tsx src/components/TopBar.test.tsx src/components/PaneResizer.test.tsx src/i18n/messages.test.ts src/dev/perfRows.test.ts` |
| Step 16a 日付の初期値（Red → Green） | `cargo +stable test -p local-sights-core session` と `npx vitest run src/App.test.tsx` |
| Step 17a 今日の日付（Red → Green） | `npx vitest run src/dateTime.test.ts` |
| Step 17b カレンダーの未来日 | `npx vitest run src/components/ConditionArea.test.tsx` |
| Step 18 ステータス行 | `npx vitest run src/App.test.tsx src/components/ConditionArea.test.tsx` |
| Step 15 画面 | `npx vitest run src/hooks/usePaneWidth.test.ts src/components/PaneResizer.test.tsx src/components/LogGroupPane.test.tsx src/components/ConditionArea.test.tsx src/components/LogFilterInput.test.tsx src/components/TopBar.test.tsx src/i18n/messages.test.ts src/App.test.tsx` |

注：`cargo test` の名前フィルタは 1 つしか受け取らないため、`time_range` と `date_input` のように 2 つ確かめるときは別々に実行する。手元の既定ツールチェーンが Cargo.lock の MSRV より古い場合は `cargo +stable` を使う（CI は stable）。

全体の確認は Step 12 で `npx vitest run` と `cargo test -p local-sights-core` を 1 回ずつ。

## 対象の回帰テスト（bugfix の下限）

1. `ConditionArea.test.tsx`：「取得できない理由」の箇条書きが画面に存在せず、理由が該当する欄の直下に出る（FR3.1）。
2. `FetchErrorBanner.test.tsx`：取得全体の失敗が「何が起きたか・次の行動・詳細」の 3 行構成のアラートで出て、閉じると消える（FR3.2）。
3. `time_range.rs`：`2024/03/01 10:00:00` を解析でき、`2024-03-01 10:00:00` は形式の誤りになる（FR2.1、FR2.4）。

## 期待するテストの量

| 対象 | 数 | 種類 |
|---|---|---|
| `time_range.rs`・`date_input.rs` | 既存の書き換え＋新規 4 程度（合計 10 前後） | 単体 |
| `dateTime.ts` | 10 前後 | 単体 |
| `useColorScheme.ts` | 3 | 単体 |
| `TopBar`、`LogGroupPane`、`ConditionArea`、`FetchErrorBanner`、`FailureList`、`StatusLine`、各 `Modal` | 部品ごとに 5〜8 | 部品（jsdom） |
| `LogTable` | 既存を緑に保つ＋2 | 部品 |
| `messages.ts` | 3 | 単体 |

カバレッジの数値の下限は設けない（team.md）。コアの公開関数（`parse_in_zone`、`format_input_in_zone`、`format_display_in_zone`、`parse_utc_seconds`）には必ずテストがあること。

## モック・スタブ

- Tauri の `invoke`／`listen` は `src/test/` の既存モック（`vi.mock("../api")` の形）を使う。実際の AWS には接続しない。
- Cloudscape の部品は実物を jsdom で描画する。`DatePicker` のカレンダーは `getByRole("button", {name: /calendar|カレンダー/})` で開き、日付のセルは `getByRole("gridcell")` で選ぶ。`TopNavigation` の `menu-dropdown` はボタンを押してから項目を押す。
- `matchMedia` は jsdom にないため、`useColorScheme` のテストでは `window.matchMedia` をスタブする。
- 性能測定（Step 10）は自動テストではなく、Tauri 実機での手動測定。結果は code-summary.md に記録する。

## テストデータ

- 日時：`2024/03/01 10:00:00`（ローカル UTC+9）、`2024/03/10 02:30:00`（America/New_York の夏時間の飛び）、`2024/11/03 01:30:00`（重複）、`2024/02/30`（存在しない日）、`2024-03-01`（ハイフン）。
- セッション：`src/test/fixtures.ts` の `sessionView(overrides)` を使い、`validationErrors` に各 key を入れて欄ごとの表示を確かめる。
