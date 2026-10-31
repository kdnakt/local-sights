# Unit Test Instructions — U3 途中結果と中断後の結果（u3-partial-results）

## テストの道具と設定

- Rust：標準の `cargo test`。テストは各モジュールの中の `#[cfg(test)] mod tests` に書く（今の書き方）。非同期のテストは今のテストと同じく tokio の実行時を使う。新しい依存は足さない。
- 画面：Vitest（`vitest.config.ts` は今のまま）。Testing Library を今のまま使う。
- 自動テストは実際の AWS に接続しない。取得は今の偽のゲートウェイ（`CloudWatchLogsGateway` の trait の裏）に差し替え、ページを届ける時を偽の取得元で操る。

## この作業単位のテストの実行のしかた

この作業単位のテストだけを、次のコマンドで実行する（Step 1 で、作業の前に通ることを確かめる）。

```bash
cargo test -p local-sights-core --lib query::store
cargo test -p local-sights-core --lib query::runner
cargo test -p local-sights-core --lib insights_session
cargo test -p local-sights --lib
cargo test -p local-sights-core --example fetch_check
npx vitest run src/components/QueryResultTable.test.tsx src/components/LogAnalyticsPage.test.tsx src/components/StatusLine.test.tsx src/components/CancelNotice.test.tsx src/components/ExportNoticeAlert.test.tsx src/components/QueryEditor.test.tsx src/hooks/expansionState.test.ts src/i18n/messages.test.ts src/commandRegistration.test.ts
```

- 純粋なロジックの段（Step 2・4・6・22）では、テストを書いたあとにその部品のコマンド（`query::store`・`query::runner`・`insights_session`）を実行し、新しいテストが失敗することを確かめてから実装する。
- 写しの大きさの測定（Step 22）は、ふだんは流れない。手で次のように流す：`cargo test -p local-sights-core --release --lib query::store -- --ignored`
- 持ち越しの確認（Step 25）は `npx vitest run src/components/QueryEditor.test.tsx` と `cargo test -p local-sights-core --lib query::catalog`。

## テストの量（Standard）

| 部品 | テストの数の目安 | 中身 |
|---|---|---|
| QueryStore | 12 | 登録・確定・外し、2 回数えない・落とさない、途中の view の並び、期間の外、キャッシュ、viewVersion の照合 |
| QueryRunner の判断の関数 | 8 | 起動の条件、取得の終わり方からの判断 |
| QueryRunner の流れ | 10 | 途中結果の版の順、最終結果、取得済み、stats、中断の 3 つの場合、失敗との両方、ログ管理の画面 |
| InsightsSession | 14 | 版の受け取りと捨て方、stage、前回の結果、Cancelling と cancel_available、中断の 3 つ、rowKey、版つきの読み出し |
| Tauri | 4 | 版つきの読み出し、find_query_row、2 回目の cancel、権限 |
| fetch_check | 2 | 引き数、出力 |
| Vitest | 20 前後 | 印、読み上げ、rowKey、先頭の行、古い版、Cancelling、中断の表示、通知、権限 |

各部品で、普通の場合と、少なくとも 2 つの端の場合（0 件、取得済み、境界の時刻、同じ時刻のイベント）を確かめる。

## カバレッジ

- 中核ライブラリ（`cargo-llvm-cov`）と画面（Vitest のカバレッジ）の行カバレッジは 80% を下回らない（CI の Linux のランナーで測る）。下限は下げない。
- `store.rs` の新しい部分と、`runner.rs` の判断の関数は 90% 以上を目安にする。

## モックとスタブ

- 取得：今の偽のゲートウェイを使い、ページを届ける順と間を操る（例：チャンネルで 1 ページずつ届ける）。時計に頼るテストは書かず、起動の判断は時刻を引き数で渡す関数で確かめる。
- 画面：`api.ts` のコマンドは今のテストと同じく差し替える。`src/test/fixtures.ts` に、版つきの見出し（partial・final・cancelledPartial）と rowKey つきの行の見本を足す。
- 本物の AWS と本物の AWS CLI は使わない。

## テストのデータ

- イベントは、テストの中で `LogEvent` を組み立てる。同じ時刻のイベントを複数のロググループとストリームに置き、並びの決まりを確かめる。ログの中身の実物はリポジトリに置かない。
- 速さ（NFR1）は自動テストで測らない。手元で `fetch_check` を実際の AWS に対して流して測り、合否の数値をその実測から決める（決めたら下げない）。
