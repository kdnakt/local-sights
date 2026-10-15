# Infrastructure Specification — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-design/`（performance・security・scalability・reliability・observability の各設計、logical-components.md）、`construction/u1-concurrent-skeleton/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。質問票の回答は `infrastructure-design-questions.md` の Q1 と、まとめの確認。

このアプリはローカルのデスクトップアプリで、サーバー・クラウドの資源・ネットワークの設定を持たない（team.md Deployment）。U1 はそれを変えない。U1 で変わるのは、利用者の端末から CloudWatch Logs を呼ぶ「呼び方」だけである。

## Deployment

| Facet | Choice | Rationale |
|-------|--------|-----------|
| 実行の形 | 利用者の Mac（Apple Silicon）の上のデスクトップアプリ（Tauri）。取得は取得 1 回のタスクの中で動く | U1 で変えない。同時に読むのはタスクの中の並び（FuturesUnordered）で、スレッドや別プロセスを足さない（performance-design） |
| ネットワーク | 端末から CloudWatch Logs の公開のエンドポイントへ HTTPS で出るだけ。入ってくる通信はない | U1 で変えない。呼ぶ API は読み取り 3 つだけ（NFR8.1） |
| 保存 | 今のディスクのキャッシュ（`.app` の外）のまま。U1 で新しい保存先を足さない | 要約の 1 行はファイルに書かない（NFR8.4） |
| 環境 | 開発者の手元と、利用者の会社の PC だけ。ステージング・本番の環境はない | team.md Deployment |
| IaC | なし。クラウドの資源を作らない | 作る資源がない |
| 大きさ | 同時 8 本は定数、渡し待ちは最大 8 ページ、速さの上限はリージョンの表（東京で 1 秒 12 回） | scalability-design・performance-design。端末の資源で決めるものはない |
| 配布 | 今までどおりタグで GitHub Releases に `.dmg` を公開。U1 の PR だけではタグを付けない（次の版は `v0.1.2`） | team.md Deployment、まとめの確認 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---------|------|---------------|-------|
| Amazon CloudWatch Logs | 読み取り先（外のサービス） | DescribeLogGroups・DescribeLogStreams はクライアントの既定のまま。GetLogEvents だけ、操作ごとの設定の上書きで SDK の最大試行回数を 1（再試行なし）にする | 新しい API・新しい権限は要らない（NFR8.1）。最大試行回数の値の意味は NFR 設計のレビュー R-01 のとおりコード生成で明記する |
| AWS の認証（プロファイル・SSO） | 認証情報の取り出し | 今のまま（利用者のプロファイル。SSO は AWS CLI の `aws sso login`） | U1 で変えない。秘密の値はログ・画面・キャッシュに書かない（NFR8.2） |

## Assumptions & Open Questions

None.
