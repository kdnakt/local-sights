# Unit Test Instructions — U5 絞り込み（u5-filter）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1〜U4 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`、結合テストは `crates/local-sights-core/tests/`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。ディスクの空きが少ないため、ビルドは `CARGO_INCREMENTAL=0` で行う。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続しない。

ライブラリ側（U5 のモジュールと結合テストだけ）：

```bash
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- filter:: log_view:: session::
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u5_filter_flow
```

10 万件・100 万件の速さのテスト（リリースビルド、`#[ignore]`、手元で必要なときに実行。実行後は `target/release` を消す）：

```bash
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --release --lib -- filter:: log_view:: --ignored
```

画面側（U5 のテストファイルだけ）：

```bash
npx vitest run src/components/LogFilterInput.test.tsx src/components/StatusLine.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、`filter::`・`log_view::` のテストを書いたら最初のコマンドを実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| FilterEngine | `filter.rs` | 8 前後 | BR1.1、BR1.2、BR1.4、BR1.5、BR2.1、BR2.3、BR2.4、BR3.6 |
| LogView | `log_view.rs` | 7〜8 | BR2.1、BR2.2、BR2.5、BR3.1、BR3.2、R-07、R-08 |
| AppSession | `session.rs` | 5〜6 | BR1.1、BR3.3、BR3.6、取得中・確認待ち |
| 取得と絞り込み | `tests/u5_filter_flow.rs` | 3〜4 | FR6.4、BR2.1、BR2.2 |
| DesktopUi | `src/components/*.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 10 前後 | BR1.3、BR3.3、BR3.6、英日 |

## モックとスタブ

- 取得の流れは U3 の偽物の gateway（`crates/local-sights-core/tests/support/fake_gateway.rs`）を使う。
- 走査の小分けの大きさは引数で渡し、テストでは小さくして走査の途中の状態を作る。
- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。0.3 秒の待ちは Vitest の偽の時計（`vi.useFakeTimers`）で進める。

## テストデータ

- メッセージには、大文字・小文字の混ざったもの（`Error: timeout`）、改行を含むもの、日本語のもの、ストリーム名にだけ文字列が入っているもの（一致しないことを確かめる）を用意する。
- 10 万件・100 万件のデータはテストの中で生成する（ファイルに置かない）。
