# Unit Test Instructions — U3 取得の作り込み（u3-fetch-robustness）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1・U2 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`、結合テストは `crates/local-sights-core/tests/`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。U3 では dev-dependencies の `tokio` に `test-util` の機能を足し、再試行の待ちを含むテストは `#[tokio::test(start_paused = true)]` で実時間を待たずに行う。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続しない。

ライブラリ側（U3 のモジュールと結合テストだけ）：

```bash
cargo test -p local-sights-core --lib -- streams:: retry:: timeline:: coordinator:: fetcher:: session:: request:: gateway::
cargo test -p local-sights-core --test u3_fetch_flow
```

100 万件の速さのテスト（リリースビルド、`#[ignore]`、手元で必要なときに実行）：

```bash
cargo test -p local-sights-core --release --lib -- timeline:: --ignored
```

画面側（U3 のテストファイルだけ）：

```bash
npx vitest run src/virtualScroll.test.ts src/components/LogTable.test.tsx src/components/StatusLine.test.tsx src/components/FailureList.test.tsx src/components/FetchForm.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、`streams::`・`retry::`・`timeline::`・`coordinator::`・`session::` のテストを書いたら最初のコマンドを、`src/virtualScroll.test.ts` を書いたら `npx vitest run src/virtualScroll.test.ts` を実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| StreamPlanner（純粋なロジック） | `streams/mod.rs` | 7〜8 | BR1.1〜BR1.3、BR1.5、余裕 1 時間の境界、時刻なし（R-01） |
| RetryPolicy | `retry.rs` | 6〜8 | BR2.1、BR2.2、続けての使い切りの上限（R-03） |
| EventTimeline | `timeline.rs` | 7〜8 | BR4.1〜BR4.5、位置の索引（R-10）、100 万件（`#[ignore]`） |
| FetchCoordinator（結果の状態） | `coordinator.rs` | 5〜6 | BR5.2 |
| 列挙と取得の流れ | `tests/u3_fetch_flow.rs` | 8〜10 | BR1.2、BR1.5、BR2.1〜BR2.3、BR3.1〜BR3.3、BR5.1〜BR5.5、FR4.10 の受け入れ条件 3 つ |
| CloudWatchLogsGateway（追加分） | `gateway/aws.rs` | 2〜3 | NFR6、エラーの分類、時刻なし |
| AppSession | `session.rs`・`request.rs` | 7〜8 | BR5.6、BR6.1〜BR6.3、BR6.6、BR6.7 |
| DesktopUi | `src/virtualScroll.test.ts`、`src/components/*.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 20 前後 | BR6.2〜BR6.5、BR6.8、世代番号の受け渡し |

## モックとスタブ

- AWS は trait `CloudWatchLogsGateway` の裏に置き、テストでは偽物（`crates/local-sights-core/tests/support/fake_gateway.rs`）に DescribeLogStreams の決めた応答（ストリーム・次のトークン・エラー）と、ストリーム名ごとの GetLogEvents の応答の列（ページとエラー）を足して使う。偽物は呼ばれた API・ストリーム名・次のトークンを記録する。
- 再試行の待ちのばらつきは `JitterSource` を固定の値を返す実装に差し替える。待ちは `start_paused` の tokio の時刻で進め、テストは実時間を待たない。
- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。`get_rows` の偽物は、決めた件数のログから offset と limit で切り出して返す。jsdom には描画の大きさがないため、表示範囲の高さはテストで渡す。

## テストデータ

- ストリーム名は同じ時刻で文字列の順が効くもの（例：`a`・`b`・`B`）と、時刻を持たないストリームを用意する。
- エラーには Throttled・Network・AccessDenied を使い、安全な詳細だけが出ることを確かめる。認証情報らしい値はテストの中で組み立てたダミーだけを使う。
- 100 万件のデータはテストの中で生成する（ファイルに置かない）。
