# Security Test Instructions — 画面を AWS マネジメントコンソールに寄せる

今回は見た目の改修で、認証や AWS 呼び出しの経路は変えていない。確かめるのは、依存の追加と表示の変更で project.md の禁止事項に触れていないことである。

| ID | 確認 | 方法 | 合格 |
|---|---|---|---|
| SEC-1 | npm の依存に高以上の脆弱性がない | `npm audit --audit-level=high` | found 0 vulnerabilities |
| SEC-2 | Rust の依存の方針（脆弱性・ライセンス・出所） | `cargo deny check`（CI の `cargo-deny` ジョブ） | エラーなし |
| SEC-3 | 追加した npm パッケージのライセンス | `node_modules/@cloudscape-design/*/package.json` の `license` | Apache-2.0 |
| SEC-4 | 呼ぶ AWS API が増えていない | `grep -rn "\.describe_\|\.get_log_events\|\.filter_log_events\|\.put_\|\.delete_\|\.create_" crates/` | DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ |
| SEC-5 | テレメトリ・外部通信がない | `src-tauri/tauri.conf.json` の CSP が `default-src 'self'` のままで、追加は `font-src 'self' data:` だけ | 外部の接続先がない |
| SEC-6 | エラー表示に秘密の認証情報が出ない | `src/errorText.test.ts`、`FetchErrorBanner.test.tsx`（エラーの中身は変えていない：FR3.4） | テストが通る |
| SEC-7 | テストと CI が実際の AWS に接続しない | `.github/workflows/ci.yml` に AWS の認証情報がなく、テストは trait のスタブを使う | 接続なし |

## 実行

```
npm audit --audit-level=high
cargo deny check
npx vitest run src/errorText.test.ts src/components/FetchErrorBanner.test.tsx
```

`cargo deny` が手元にない場合は、PR の CI の `cargo-deny` ジョブで確かめる。GitHub の秘密情報スキャンはリポジトリの設定で有効になっている（team.md）。
