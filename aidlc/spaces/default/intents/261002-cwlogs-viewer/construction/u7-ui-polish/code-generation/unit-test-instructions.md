# Unit Test Instructions — U7 画面の仕上げ（u7-ui-polish）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1〜U6 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。ディスクの空きが少ないため、ビルドは `CARGO_INCREMENTAL=0` で行う。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続しない。

ライブラリ側（U7 で変えるモジュールだけ）：

```bash
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- log_view:: session:: coordinator:: cache::
```

画面側（U7 のテストファイルだけ）：

```bash
npx vitest run src/rowLayout.test.ts src/virtualScroll.test.ts src/errorText.test.ts src/hooks/expansionState.test.ts src/hooks/useRowPositions.test.tsx src/components/LogTable.test.tsx src/components/ErrorMessage.test.tsx src/components/FetchErrorBanner.test.tsx src/components/StatusLine.test.tsx src/components/LogGroupPane.test.tsx src/components/FailureList.test.tsx src/components/CloseConfirmDialog.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、純粋なロジックのテストを書いたら上のコマンドを実行して失敗の出力を記録してから実装に進む。`virtualScroll.test.ts` は U3 のテストで、`rowLayout.ts` に置き換えたあとも同じ答えになることの確認に使う（中身の参照先を `rowLayout.ts` に替えてもよいが、期待する値は変えない）。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| 行の位置の計算 | `src/rowLayout.test.ts` | 8 前後 | BR1.4、BR1.8、BR1.9、R-12、R-14 |
| エラーの文 | `src/errorText.test.ts` | 6〜8 | BR2.2、BR2.3 |
| 展開と選択の更新 | `src/hooks/expansionState.test.ts` | 5〜6 | BR1.1、BR1.5、BR1.6 |
| LogView | `log_view.rs` | 4〜5 | BR1.6、BR1.7、R-11 |
| AppSession | `session.rs` | 6〜8 | BR3.1〜BR3.4、U6 R-01・R-02 |
| LogCache・FetchCoordinator | `cache/plan.rs`・`cache/mod.rs`・`coordinator.rs` | 6 前後 | U6 R-03、R-04、R-06 |
| DesktopUi | `src/components/*.test.tsx`、`src/hooks/useRowPositions.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 25 前後 | BR1.1〜BR1.10、BR2.4、BR2.5、BR3.3、BR4.3、BR4.4 |

## モックとスタブ

- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。`row_positions` の答えは版を変えて返し、古い版が捨てられることを確かめる。
- `ResizeObserver` は jsdom にないため、テストでは小さな偽物を置いて高さを渡す。文字の幅は固定の値を渡す。
- 取得の流れは U3 の偽物の gateway とライブラリの中の偽物を使う。
- ファイルを使うテストは `tempfile` の一時フォルダで行う。

## テストデータ

- メッセージには、改行を含むもの、20 行を超えるもの、空白のない長い 1 語（base64 のような文字列）、日本語（全角）、HTML のように見える文字（`<b>` など。そのまま文字として出ることを確かめる）を用意する。
- 100 万件の行数はテストの中で数として渡す（実際の行は作らない）。
- 実際の AWS の認証情報やログは使わない。
