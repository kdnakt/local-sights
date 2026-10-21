# 安全の確かめの手引き（261020-carryover-fixes）

この作業に新しい安全の要件はない。project.md の Forbidden と NFR5・NFR7 が破られていないことを確かめる。

## 確かめること

| 確かめること | やり方 | 要件 |
|---|---|---|
| CloudWatch Logs の読み取りの 3 つ（DescribeLogGroups・DescribeLogStreams・GetLogEvents）以外の AWS の API を呼んでいない | この作業の差分（`git diff origin/main...HEAD`）に AWS の SDK の新しい呼び出し（`aws_sdk_*` の新しいクライアントや操作）がないことを検索で確かめる | NFR5、Forbidden |
| リージョンの名前をネットワークから取っていない | `src/i18n/regions.ts` が組み込みの表であること、差分に `fetch(`・新しい Tauri のコマンドがないことを確かめる | NFR5 |
| 自動テストと CI が AWS につながない | 差分のテストが `FakeGateway`・一時フォルダ・`vi.mocked` だけを使うことを確かめる。CI の設定は変えていない | Forbidden |
| 認証情報を書かない | 差分に認証情報らしい文字列（`AKIA`、`aws_secret_access_key`、`aws_session_token`）がないことを検索で確かめる | Forbidden |
| テスト以外のコードで `unwrap()`／`expect()` を足していない | 差分の追加行のうちテストのモジュールの外のものを確かめる | NFR7 |
| 依存関係の脆弱性 | `npm audit --audit-level=high`（手元）、`cargo-deny`（CI のジョブ）。この作業で依存関係は変えていない | NFR7 |

## 流し方

```bash
git diff origin/main...HEAD -- crates src-tauri src | grep -nE '^\+.*(aws_sdk_|\.send\(\)|fetch\()' || echo "no new AWS or network calls"
git diff origin/main...HEAD | grep -nE 'AKIA[0-9A-Z]{16}|aws_secret_access_key|aws_session_token' || echo "no credentials"
npm audit --audit-level=high
```
