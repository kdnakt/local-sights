# Infrastructure Specification — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u5-cancel/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

このアプリはローカルのデスクトップアプリで、サーバー・クラウドの資源・ネットワークの設定を持たない（team.md Deployment）。U5 はそれを変えず、アプリの権限に中断の操作を 1 つ足す。

## Deployment

| Facet | Choice | Rationale |
|-------|--------|-----------|
| 実行の形 | 利用者の Mac（Apple Silicon）の上のデスクトップアプリ（Tauri）のまま。中断は取得 1 回のタスクの中の打ち切りで、スレッド・プロセスを足さない | performance-design |
| ネットワーク | 変えない。中断で新しい通信は起きない（出ている HTTP の要求を打ち切るだけ） | NFR4.2、NFR8.14 |
| 保存 | 変えない。中断した取得はキャッシュに書かない | BR5.7 |
| 権限 | Tauri のコマンドの権限に `cancel_fetch` を足す（`src-tauri/build.rs` のコマンドの一覧、`capabilities/default.json` の allow、`generate_handler`）。ほかは増やさない | NFR8.15 |
| 環境 | 開発者の手元と、利用者の会社の PC だけ | team.md Deployment |
| IaC | なし | 作る資源がない |
| 配布 | 今までどおりタグで GitHub Releases に `.dmg` を公開。U5 だけではタグを付けない（次の版は `v0.1.2`） | team.md Deployment |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---------|------|---------------|-------|
| Amazon CloudWatch Logs | 読み取り先（外のサービス） | 変えない | 中断の後は新しい呼び出しを始めない（NFR4.2） |
| Tauri のコマンドとイベント | 画面 → アプリの `cancel_fetch`、アプリ → 画面の `session-changed` | `cancel_fetch` を足す。session-changed に項目を足すだけ | 新しいイベントは作らない（C5、C6） |

## Assumptions & Open Questions

None.
