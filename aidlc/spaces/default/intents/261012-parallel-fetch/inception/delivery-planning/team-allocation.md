# Team Allocation — ロググループ全体の取得の高速化

上流：`bolt-plan.md`、`inception/units-generation/unit-of-work.md`、`inception/practices-discovery/team-practices.md`。

このスコープ（mvp）ではチーム編成のステージを行っていないため、すべての Bolt（作業を 1 回通して、動くものができるところまで進める区切り）は、AI の開発者役（aidlc-developer-agent）が作る。人間の開発者は 1 人で、各作業単位のチェックポイントで確認して承認し、手元の実際の AWS での確認（認証情報は本人のもの）と GUI の目視の確認を行う（team-practices Walking Skeleton）。

| Bolt | 作業単位 | 作る役 | 人間が行うこと |
|------|----------|--------|----------------|
| 下準備 | — | aidlc-developer-agent | PR の確認とマージ |
| u1-concurrent-skeleton | U1 | aidlc-developer-agent | 手元の実際の AWS での確認、GUI の目視、薄い一本のチェックポイントの承認 |
| u2-call-pacing | U2 | aidlc-developer-agent | チェックポイントの承認 |
| u3-give-up-rule | U3 | aidlc-developer-agent | チェックポイントの承認 |
| u4-elapsed-time | U4 | aidlc-developer-agent | GUI の目視、チェックポイントの承認 |
| u5-cancel | U5 | aidlc-developer-agent | GUI の目視、チェックポイントの承認 |

チームは 1 つだけなので、複数のチームの間の作業の受け渡し（Program Board：チームどうしの依存と日程を並べた表）は作らない。

## Assumptions & Open Questions

None.
