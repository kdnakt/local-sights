# Unit Test Instructions — U2 接続とロググループの選択（u2-connection-selection）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`、結合テストは `crates/local-sights-core/tests/`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続しない。

ライブラリ側（U2 のモジュールと結合テストだけ）：

```bash
cargo test -p local-sights-core --lib -- catalog:: log_groups:: connection:: session:: gateway::
cargo test -p local-sights-core --test u2_log_group_listing
```

画面側（U2 のテストファイルだけ）：

```bash
npx vitest run src/components/ConnectionBar.test.tsx src/components/LogGroupPane.test.tsx src/components/ConfirmDialog.test.tsx src/components/FetchForm.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、`catalog::`・`log_groups::`・`connection::` のテストを書いたら最初のコマンドを実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| ConnectionCatalog（純粋なロジック） | `catalog/mod.rs`・`catalog/regions.rs` | 7〜9 | BR1.1、BR1.2、BR1.4、BR1.5 |
| ConnectionCatalog（設定ファイルの読み込み） | `catalog/files.rs` | 4〜5 | BR1.3、R-12 |
| LogGroupBrowser（純粋なロジック） | `log_groups/mod.rs` | 7〜8 | BR3.1〜BR3.4、BR3.6、BR3.7、BR3.10 |
| LogGroupBrowser（取得の流れ） | `tests/u2_log_group_listing.rs` | 6〜7 | BR3.1、BR3.4、BR3.6、BR4.1 |
| CloudWatchLogsGateway（追加分） | `gateway/aws.rs` | 3〜4 | BR2.8、BR3.9、エラーの分類 |
| AppSession（接続の選択） | `connection.rs` | 7〜8 | BR2.1〜BR2.5 |
| AppSession（つなぎ） | `session.rs` | 6〜7 | BR2.4、BR2.6〜BR2.8、BR3.5、BR3.8 |
| DesktopUi（追加分） | `src/components/*.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 15 前後 | BR5.1、BR5.2、BR3.8 の表示、BR3.10、確認ダイアログ |

## モックとスタブ

- AWS は trait `CloudWatchLogsGateway` の裏に置き、テストでは偽物（`crates/local-sights-core/tests/support/fake_gateway.rs`）に DescribeLogGroups の決めた応答（ロググループ・次のトークン・エラー）を足して使う。偽物は受け取った次のトークンを記録する。
- 設定ファイルのテストは、`AWS_CONFIG_FILE`・`AWS_SHARED_CREDENTIALS_FILE` を一時ファイルに向け、`AWS_EC2_METADATA_DISABLED=true` にして行う。純粋なロジックのテストは、ファイルを読まずに文字列と環境変数の値を引数で渡す。環境変数を変えるテストは 1 つのテスト関数の中で順に行う。
- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。

## テストデータ

- 設定ファイルの中身はテストの中で組み立てる。認証情報の項目には、テストの中で組み立てたダミーの値だけを使い、結果に含まれないことを確かめる（実在の値は使わない）。
- ロググループ名には大文字を含むもの（例：`/aws/lambda/MyFunction`）と、100 件を超える複数ページの一覧を用意する。
