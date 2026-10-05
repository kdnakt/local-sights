# Code Generation 質問票 — U4 時間範囲とタイムゾーン（u4-time-range）

上流の成果物：`construction/u4-time-range/functional-design/`（functional-spec.md・rules.md・entities.md）、`inception/units-generation/unit-of-work.md`、`inception/requirements-analysis/requirements.md`。

U4 では、機能設計で「タイムゾーンの変換はすべてライブラリが持ち、テストでは差し替えられる境界を置く」（BR3.4）と決めた。その実現に使うライブラリの選択だけを確認する（project.md の「先送りした技術選定も、次の作業に必要になった時点で質問して決める」）。

## Question 1

Rust のライブラリで、ローカルのタイムゾーン（夏時間を含む）をどう扱うか。テストでは夏時間のある地域（例：America/New_York）に差し替える必要がある（BR3.4）。

A. 起動時に OS のタイムゾーン名を iana-time-zone で読み、chrono-tz（タイムゾーンのデータを同梱）で変換する。本番とテストが同じ仕組みで、テストは名前を差し替えるだけ
B. 本番は chrono の Local（OS の設定）、テストだけ chrono-tz を使う
C. 日時ライブラリを jiff に切り替える
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

回答のまとめ：

- ローカルのタイムゾーン：起動時に OS のタイムゾーン名を iana-time-zone で読み、chrono-tz で変換する。テストは名前を差し替えて、夏時間のある地域と UTC で確かめる（Q1）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Plan Approval

対象：`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md`。

[Approval Fingerprint]: sha256:v3:d87e3f38d6c5aaefa054252d496db1ffdfd16eb8af66002f87709176ca7da99a
[Planned Source]: da25e76a6ee61e0f5aa5ef475607a9da2687382890f22d0f9c5f6278e53e7f5c

- Approve Plan
- Request Changes

[Answer]: Approve Plan
