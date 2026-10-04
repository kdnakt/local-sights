# Unit Test Instructions — U1 薄い一本（u1-walking-skeleton）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

| 側 | 枠組み | 設定 |
|----|--------|------|
| ライブラリ（Rust） | 標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`、結合テストは `crates/local-sights-core/tests/`） | 非同期のテストは `#[tokio::test]`。追加の設定ファイルはない |
| 画面（TypeScript） | Vitest + React Testing Library + `@testing-library/user-event` + `@testing-library/jest-dom`、環境は jsdom | `vitest.config.ts`（`environment: "jsdom"`、`setupFiles: ["src/test/setup.ts"]`） |

`src-tauri`（Tauri のつなぎ）には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続しない。

ライブラリ側（U1 のモジュールと結合テストだけ）：

```bash
cargo test -p local-sights-core --lib -- request:: time_range:: paging:: event:: timeline:: failure:: gateway:: session::
cargo test -p local-sights-core --test u1_fetch_flow
```

画面側（U1 のテストファイルだけ）：

```bash
npx vitest run src/components/FetchForm.test.tsx src/components/LogTable.test.tsx src/components/StatusLine.test.tsx src/hooks/useEscapeKey.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、最初の 1 本目のコマンドを実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| 共通の検証（FetchRequest） | `request.rs` | 6〜8 | BR1.1〜BR1.3、BR1.8、文言キーの理由 |
| TimeRangeModel | `time_range.rs` | 6〜8 | BR1.2、BR2.1、BR2.2、BR5.3、うるう年 |
| EventFetcher のページ終端 | `paging.rs` | 6 | BR3.2 と pageCount |
| EventTimeline（最小版）と LogEvent | `event.rs`・`timeline.rs` | 5 | BR5.1、追加・読み出し・破棄 |
| CloudWatchLogsGateway | `failure.rs`・`gateway/classify.rs`・`gateway/aws.rs` | 8〜10 | BR4.2、BR4.3、BR1.5、BR1.6 |
| EventFetcher と FetchCoordinator | `tests/u1_fetch_flow.rs` | 8 | BR3.1、BR3.2、BR4.1、BR4.5、RegionMissing |
| AppSession | `session.rs` | 7 | BR1.4、BR5.4、phase の遷移 |
| DesktopUi | `src/**/*.test.ts(x)` | 5〜8 ファイル合計で 15 前後 | BR4.4、BR5.2〜BR5.4、BR6.1、BR6.2 |

## モックとスタブ

- AWS は trait `CloudWatchLogsGateway` の裏に置き、テストでは偽物（`crates/local-sights-core/tests/support/fake_gateway.rs`）に差し替える。偽物は決めた応答（イベント・次のトークン・エラー）を順に返し、受け取った引数（startTime・endTime・startFromHead・トークン）を記録する。
- 接続先の解決のテストは、`AWS_CONFIG_FILE`・`AWS_SHARED_CREDENTIALS_FILE` を一時ファイルに向け、`AWS_EC2_METADATA_DISABLED=true` にして行う。AWS の API は呼ばない。環境変数を変えるテストは 1 つのテスト関数の中で順に行う。
- 画面側のテストでは `src/api.ts`（Tauri のコマンドとイベント）を `vi.mock` で差し替え、Tauri を起動しない。

## テストデータ

- ログのイベントはテストの中で組み立てる（時刻はミリ秒の整数、メッセージは改行を含むものも用意する）。
- 秘密の認証情報に見える文字列は、テストの中で組み立てたダミー（例：`AKIA` に続けて英大文字と数字 16 文字）だけを使い、実在の値は使わない。
- 実際の AWS での確認は確認用プログラム（`cargo run -p local-sights-core --example fetch_check -- ...`）で開発者の手元だけで行い、テストには含めない。
