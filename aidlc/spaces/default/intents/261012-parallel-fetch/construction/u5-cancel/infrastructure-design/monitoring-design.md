# Monitoring Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u5-cancel/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

ローカルのアプリで、監視の仕組み・アラート・ダッシュボードは持たず、外へは何も送らない（project.md Forbidden）。

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|--------|--------|-----------|----------------|
| status=Cancelled（要約の 1 行） | 標準エラー出力の U1 の要約 | なし | 中断で終えた取得を読み取る（NFR2.5） |
| 読み終えていない本数 N | 画面の中断の注意 | なし（利用者が見て判断する） | 足りないログを利用者が知る（FR5.4） |

## Alerts

| Alert | Condition | Severity | Routes to |
|-------|-----------|----------|-----------|
| なし | — | — | ローカルのアプリで、知らせる先がない |

## SLIs / SLOs

| SLI | SLO | Measurement |
|-----|-----|-------------|
| 中断の後に始まる呼び出し | 0 回 | 結合テスト（NFR4.2） |
| 中断から止まるまで | 待ちの最中・応答待ちの最中とも、偽の時計を進めずに止まる | 結合テスト（NFR4.3） |

## Logs & Tracing

- 中断のために新しいログは出さない。要約の 1 行の status に Cancelled が入るだけ（NFR2.5）
- 手元の GUI の確認：［中断］・［中断中…］・注意・状態行の「中断」、キーボードだけで押せること、VoiceOver で注意が一度読まれること、中断した範囲がキャッシュから表示されないこと（AC4.2.1）を一度確かめる

## Assumptions & Open Questions

None.
