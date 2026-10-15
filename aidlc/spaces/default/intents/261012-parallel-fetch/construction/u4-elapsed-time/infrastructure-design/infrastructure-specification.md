# Infrastructure Specification — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u4-elapsed-time/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

このアプリはローカルのデスクトップアプリで、サーバー・クラウドの資源・ネットワークの設定を持たない（team.md Deployment）。U4 は AppSession と画面の状態行を変えるだけで、インフラは何も変えない。

## Deployment

| Facet | Choice | Rationale |
|-------|--------|-----------|
| 実行の形 | 利用者の Mac（Apple Silicon）の上のデスクトップアプリ（Tauri）のまま。経過の記録は AppSession のメモリの中、毎秒の描き直しは画面（WebView）の中 | U4 はプロセス・スレッド・タスクを足さない（performance-design） |
| ネットワーク | 変えない。経過のために外へ出る通信はない | NFR8.11 |
| 保存 | 変えない。経過はファイル・キャッシュに書かない | NFR8.11 |
| 環境 | 開発者の手元と、利用者の会社の PC だけ | team.md Deployment |
| IaC | なし | 作る資源がない |
| 配布 | 今までどおりタグで GitHub Releases に `.dmg` を公開。U4 だけではタグを付けない（次の版は `v0.1.2`） | team.md Deployment、まとめの確認 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---------|------|---------------|-------|
| Amazon CloudWatch Logs | 読み取り先（外のサービス） | 変えない | U4 は呼び出しを足さない（NFR8.11） |
| Tauri のイベント（`session-changed`） | ライブラリから画面へ SessionView を渡す | 変えない。SessionView に `elapsed` が足されるだけ | 送る契機は今のまま（NFR5.4） |

## Assumptions & Open Questions

None.
