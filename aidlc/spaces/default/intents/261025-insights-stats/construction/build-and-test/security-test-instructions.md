# Security Test Instructions — 261025-insights-stats

## 目標（requirements.md NFR4・NFR5・NFR12、project.md Forbidden）

| 目標 | 中身 | 確かめ方 |
|---|---|---|
| NFR5 | アプリが呼ぶ AWS API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ。Logs Insights の API は呼ばない | 中核とアプリのソースで、AWS SDK の呼び出しを探し、この 3 つ以外がないことを確かめる（下のコマンド）。取得の流れは変えていない（U1〜U3 の code-summary） |
| NFR4 | 自動テストと CI は AWS に接続しない。コンソールから書き出した結果をリポジトリに置かない | テストは偽の取得元と偽の CLI だけを使う。組のファイルにはクエリの文だけがある。差分に 12 桁の数・ARN・アクセスキーの形がない |
| NFR12 | 外部に何も送らない | 足したコードに、外部への送信（HTTP の呼び出しなど）がない |
| 秘密の情報 | 秘密の認証情報・アクセスキー ID をリポジトリ・ログ・画面のエラー・キャッシュに書かない | 差分を探す（下のコマンド）。新しいログの出力やエラーの文に認証情報を入れていない |
| 依存関係 | 脆弱性のある依存がない | `cargo deny check`、`npm audit --audit-level=high` |
| 新しいコマンドの権限 | `get_query_catalog`・`find_query_row` は読み取りだけで、権限は既存と同じ形で最小 | `src-tauri/capabilities/default.json` と権限の一致のテスト |

## 実行のしかた

```bash
grep -rnE "start_query|get_query_results|StartQuery|GetQueryResults|put_|delete_|create_" crates/local-sights-core/src src-tauri/src
git diff <この作業の最初のコミット> HEAD | grep -nE "[0-9]{12}|arn:aws|AKIA[0-9A-Z]{16}|ASIA[0-9A-Z]{16}"
cargo deny check
npm audit --audit-level=high
```

1 行目と 2 行目は、何も出ないこと（`scripts/compat/insights-export.sh` の `start-query`・`get-query-results` は開発者が手元で動かす道具で、アプリは呼ばない。前の作業と同じ扱い）。
