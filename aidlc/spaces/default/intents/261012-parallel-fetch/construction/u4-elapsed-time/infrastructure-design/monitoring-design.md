# Monitoring Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u4-elapsed-time/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

ローカルのアプリで、監視の仕組み・アラート・ダッシュボードは持たず、外へは何も送らない（project.md Forbidden）。U4 の経過は、利用者が状態行で見る表示である。

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|--------|--------|-----------|----------------|
| 状態行の経過（取得中・最終の値） | 画面の状態行 | なし（利用者が見て判断する） | 取得にかかった時間を利用者が知る（NFR2.3）。手元の速さの確認（NFR2）にも使う |
| elapsed_ms（要約の 1 行） | 標準エラー出力の U1 の要約 | なし。U4 で変えない | 状態行の経過と数十ミリ秒ほど違いうる（NFR2.4） |

## Alerts

| Alert | Condition | Severity | Routes to |
|-------|-----------|----------|-----------|
| なし | — | — | ローカルのアプリで、知らせる先がない |

## SLIs / SLOs

| SLI | SLO | Measurement |
|-----|-----|-------------|
| 取得中の表示とライブラリの値のずれ | 1 秒以内（PC のスリープをまたいだ取得を除く） | 画面のテスト（NFR6.7） |
| 毎秒の描き直しの範囲 | 経過の部品だけ | 画面のテスト（NFR5.3） |

## Logs & Tracing

- 経過のために新しいログは出さない（NFR8.11）。U1 の要約の 1 行は変えない（NFR2.4）
- 手元の GUI の確認：取得中に経過が 1 秒ごとに進むこと（再試行の待ちの間も）、終わると残ること、英日の文言、VoiceOver で終わったときに最終の経過が読まれること（NFR9.2）を一度確かめる

## Assumptions & Open Questions

None.
