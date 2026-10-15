# Stakeholder Map — ロググループ全体の取得の高速化

## Key Stakeholders

| Stakeholder | Interest | Source |
|-------------|----------|--------|
| 開発者本人（主な利用者で、作る人） | ストリームが数十〜数百本あるロググループでも、短い時間範囲のログを待たずに見られること | [Q1] [Q2] |
| 同じ会社でこのアプリを使う同僚 | 速くなった取得の恩恵を受けること | [Q2] |
| OSS としてこのアプリを使う不特定の利用者 | 速くなった取得の恩恵を受けること | [Q2] |
| 同じ AWS アカウントの他の利用者 | アプリの取得によって自分たちの操作が待たされないこと。多少の影響は許される | [Q5] |

## Decision-Makers vs. Influencers

| Role | Who | Source |
|------|-----|--------|
| 範囲・優先順位の決定者 | 開発者本人（これまでと同じ） | [Q6] |
| 考慮すべき関係者 | 同じ AWS アカウントの他の利用者（影響を少し気にする） | [Q5] |

## Communication Requirements

| Requirement | Detail | Source |
|-------------|--------|--------|
| 進捗の共有 | これまでと同じく、GitHub の PR と README 程度で十分 | [Q6] |

## Assumptions & Open Questions

None.
