# External Dependency Map — ロググループ全体の取得の高速化

上流：`bolt-plan.md`、`ideation/feasibility/constraint-register.md`・`raid-log.md`、`inception/practices-discovery/team-practices.md`。

作業は AI の開発者役と開発者本人だけで進め、他のチームの受け渡しや承認待ちはない。外に頼るものは、手元の確認に使う次のものだけ。

| 依存 | 持ち主 | 準備にかかる時間 | 使う Bolt（作業を 1 回通して、動くものができるところまで進める区切り） | 遅れたときの対応 |
|------|--------|------------------|------|------------------|
| 本人の AWS の認証情報（SSO など） | 開発者本人 | すでにある（今のアプリで使っている） | u1-concurrent-skeleton（手元の実際の AWS での確認） | 自動テストは偽物のゲートウェイで進め、確認だけを後に回す。CI には認証情報を置かない（project.md Forbidden） |
| ストリームの多いテスト用のロググループ（東京） | 開発者本人 | 用意済み（feasibility Q5） | u1-concurrent-skeleton、u4-elapsed-time（速さを目で見る） | 手元で確かめられる別のロググループを使う |
| GitHub Actions（CI） | GitHub | すでにある | すべて（下準備でカバレッジの検査を足す） | 手元で同じ検査を走らせ、CI が戻ってからマージする（CI が赤のままマージしない） |
| CloudWatch Logs の読み取りの上限（アカウントとリージョンで共有、引き上げ不可） | AWS | 変えられない | u1-concurrent-skeleton、u2-call-pacing | 上限の半分程度に抑える設計で対応済み（FR2）。手元の確認は利用の少ない時間に行う |

## Assumptions & Open Questions

None.
