# Code Generation 質問票 — U3 取得の作り込み（u3-fetch-robustness）

上流の成果物：`construction/u3-fetch-robustness/functional-design/`（functional-spec.md・rules.md・entities.md）、`inception/units-generation/unit-of-work.md`、`inception/requirements-analysis/requirements.md`。

U3 では、機能設計で後回しにした技術の選択のうち、計画に要るのは画面の仮想スクロールの作り方だけだった（project.md の「先送りした技術選定も、次の作業に必要になった時点で質問して決める」）。

## Question 1

ログ一覧は 100 万件でも表示範囲の行だけを描く（BR6.4）。行の高さと列の幅は固定。画面側の仮想スクロールはどう作るか。

A. 自前で作る（行の高さが固定なので計算は小さく、依存が増えず、位置を保つ処理（BR6.5）も制御しやすい。100 万行ぶんの高さを縮める処理が要る）
B. @tanstack/react-virtual を使う
C. react-window を使う
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

回答のまとめ：

- 画面の仮想スクロール：依存を足さずに自前で作る。行の高さは固定で、表示する行と描く位置は純粋な関数で求め、100 万行ぶんの高さはブラウザの上限を超えないように縮める（Q1）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Plan Approval

対象：`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md`。

[Approval Fingerprint]: sha256:v3:d1bd35251826c451e19d3049cc38ecbc5c5a41e57d6f179cb140727a25486162
[Planned Source]: ea4338035492db3d7ab4e35197d5bc97bddda729f1e8a124387daeb82e987d0c

- Approve Plan
- Request Changes

[Answer]: Approve Plan
