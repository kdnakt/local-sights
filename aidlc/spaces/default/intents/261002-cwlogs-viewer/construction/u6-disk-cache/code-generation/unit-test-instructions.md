# Unit Test Instructions — U6 ディスクキャッシュ（u6-disk-cache）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1〜U5 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`、結合テストは `crates/local-sights-core/tests/`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。ディスクの空きが少ないため、ビルドは `CARGO_INCREMENTAL=0` で行う。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続せず、利用者の `~/Library/Caches/` と設定用フォルダにも触れない。

ライブラリ側（U6 のモジュールと結合テストだけ）：

```bash
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- cache:: coordinator:: session:: filter::should_report_progress
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u6_cache_flow
```

画面側（U6 のテストファイルだけ）：

```bash
npx vitest run src/components/SettingsDialog.test.tsx src/components/ConnectionBar.test.tsx src/components/StatusLine.test.tsx src/components/LogFilterInput.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、`cache::plan` と `filter::should_report_progress` のテストを書いたら最初のコマンドを実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| LogCache（純粋なロジック） | `cache/plan.rs` | 8 前後 | BR2.1〜BR2.4、BR3.1〜BR3.3、BR4.1、BR4.3、BR5.3 |
| LogCache（ファイル） | `cache/mod.rs`、`cache/settings.rs` | 7〜8 | BR1.1〜BR1.4、BR3.4、BR3.6、BR4.1（R-13）、BR4.2 |
| FetchCoordinator | `coordinator.rs` | 6〜8 | BR1.5、BR2.3、BR2.4、BR3.2、BR3.5、BR3.7、BR4.1 |
| AppSession | `session.rs` | 6〜8 | BR1.3、BR1.6、BR5.1、BR5.3、R-10、R-11、R-12 |
| FilterEngine | `filter.rs` | 3〜4 | U5 R-03 |
| 取得とキャッシュ | `tests/u6_cache_flow.rs` | 3 | FR7.4、FR7.9、BR3.1、BR3.3 |
| DesktopUi | `src/components/*.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 12 前後 | BR5.1〜BR5.4、U5 R-01、R-02、英日 |

## モックとスタブ

- 取得の流れは U3 の偽物の gateway（`crates/local-sights-core/tests/support/fake_gateway.rs`）を使い、API の呼び出し回数で「Hit のとき呼ばない」を確かめる。
- ファイルを使うテストは `tempfile` の一時フォルダを `LogCache`・`AppSession` に渡す。場所を渡さない既定の作り方がディスクに触れないことも確かめる。
- 取得を始めた時刻は引数で渡し、テストでは固定の値にする（5 分前の境界を作るため）。
- 書き込みの失敗は、読み取り専用にしたフォルダや、フォルダの位置に置いた通常のファイルで作る。
- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。0.3 秒の待ちは Vitest の偽の時計（`vi.useFakeTimers`）で進める。

## テストデータ

- キャッシュのファイルはテストの中で書いて作る。壊れたファイルは、正しいファイルの途中で切ったもの、1 行目の版を変えたもの、キーを変えたもの、範囲が重なるもの、並びを入れ替えたものを用意する。
- プロファイルは SdkDefault と、同じ名前の Named（`default` など）を用意し、別のキーになることを確かめる。
- 実際の AWS の認証情報やログは使わない。
