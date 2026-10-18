# Security Test Instructions — Logs Insights 互換のクエリ（IB-11）

NFR4・NFR5・NFR12 と project.md の Forbidden を確かめる。

## 1. 確かめること

| 決まり | 確かめ方 |
|---|---|
| NFR5：呼ぶ AWS API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ | アプリと確認用プログラムの AWS 呼び出しは、今のゲートウェイの trait の裏だけにある。クエリの層は新しい API を足していない（U1〜U3 の code-summary）。偽のゲートウェイのテストは台本にない呼び出しで失敗する。U4 のスクリプトのテストは、`aws` の呼び出しが `logs start-query` と `logs get-query-results` だけであることを確かめる（スクリプトは開発者が手で流すもので、アプリからは動かない） |
| NFR4：自動テストと CI は AWS に接続しない。書き出しはリポジトリに置かない | テストはすべて偽のゲートウェイと偽の `aws`。確認用プログラムとスクリプトは、書き出しのフォルダがリポジトリの中なら止まる（U4 のテスト）。`.gitignore` に `insights-export/` を入れた |
| 秘密の認証情報とアクセスキー ID を書かない | クエリのエラーはクエリの言葉と場所だけを出す（FR6.4）。確認用プログラムの報告は、クエリの id・件数・行の要約（先頭 40 文字）だけを出す（U4 のテスト） |
| NFR12：外部に何も送らない | この作業で足したコードに、外部への送信（HTTP の呼び出し、テレメトリ）がないことを差分で確かめる |
| 依存関係の脆弱性 | `npm audit --audit-level=high` と `cargo deny check`（実際にビルドする機能を対象に） |
| 秘密情報のスキャン | GitHub の secret scanning（リポジトリの設定。CI の外） |

## 2. 流し方

```bash
npm audit --audit-level=high
cargo deny check

# 足した行に外部への送信がないかの確かめ（関数の名前などの見かけの一致は目で除く）
git diff origin/main...HEAD -U0 -- src src-tauri/src crates \
  | grep -E '^\+' | grep -nE 'reqwest|https?://|fetch\(|XMLHttpRequest|WebSocket|sentry|telemetry'

# AWS の呼び出しとスクリプトのテスト
cargo test -p local-sights-core
cargo test -p local-sights-core --examples
```

## 3. 範囲外

- DAST・侵入テスト：アプリはサーバーを持たない手元のデスクトップアプリで、外から受ける口がないため行わない。
