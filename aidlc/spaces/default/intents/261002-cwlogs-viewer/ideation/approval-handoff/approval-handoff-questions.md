# Approval & Handoff 質問票

上流の成果物（アイデア整理フェーズのすべて）：
- `intent-statement`、`stakeholder-map`（`ideation/intent-capture/`）
- `feasibility-assessment`、`constraint-register`、RAID ログ（`ideation/feasibility/`）
- `scope-document`、`intent-backlog`（`ideation/scope-definition/`）
- `wireframes`、`user-flow`（`ideation/rough-mockups/`）

市場調査とチーム編成のステップは、このワークフローでは実施していない（個人・OSS のツールで、開発者は 1 人のため）。関係者は開発者本人だけで、予算の承認も不要（stakeholder-map）。

## Q1. 画面設計で決まった反映事項を、承認済みの範囲文書とバックログにどう取り込みますか？

背景：画面設計（wireframes の「上流への反映事項」）で、次の事項が決まりました。デスクトップ GUI であること、MVP は macOS のみで Windows は MVP 後、1 週間は仮説、キャッシュ削除、取得中の操作ロックと終了確認、日時入力とタイムゾーン。いずれも、承認済みの `scope-document` と `intent-backlog` にはまだ載っていません。

A. 企画書（initiative-brief）に統合版の範囲とバックログを載せ、以後はそれを正とする。承認済みの文書は書き換えない
B. 承認済みの `scope-document` と `intent-backlog` を直接書き換える
C. A と B の両方
D. Not yet defined
X. Other (please specify)

[Answer]: A. 企画書（initiative-brief）に統合版の範囲とバックログを載せ、以後はそれを正とする。承認済みの文書は書き換えない

## Q2. 次の主なリスクを、対策込みで受け入れて進めますか？（select all that apply）

背景：要件定義・設計に進む前に、アイデア整理で見えたリスクを確認します。

A. 他の古いロググループでは GetLogEvents でも過去ログが見えない可能性がある（原因は調べない方針。feasibility の R-01）
B. 時間範囲が広いと、取得中に何も表示されない時間が数分以上続く（上限なし・進捗表示なし。scope-document）
C. 「1 週間で作れる」は GUI・macOS のみ・見た目最小限を前提とした仮説（wireframes の「期限と削減順」）
D. API 料金をまだ公式価格表で確認していない（feasibility の I-01）
E. None（受け入れられないものがある。Other で具体的に記入）
X. Other (please specify)

[Answer]: A, B, C, D

## Q3. ラフな画面設計は、あなたが思い描く形と合っていますか？

背景：wireframes（1 画面構成、画面 1〜7）と user-flow（主要フロー 6 操作）の確認です。

A. 合っている
B. おおむね合っているが、要件分析で調整したい点がある（Other で記入）
C. 合っていない（Other で記入）
X. Other (please specify)

[Answer]: A. 合っている

## Q4. MVP の完成目標日はいつにしますか？

背景：「1 週間以内」の起点を決めます。今日は 2026-10-03 です。

A. 2026-10-10（今日から 1 週間）
B. 要件分析が終わってから 1 週間
C. 日付は決めない（目安として 1 週間）
D. Not yet defined
X. Other (please specify)

[Answer]: A. 2026-10-10（今日から 1 週間）

## Q5. この企画で次のフェーズ（要件定義・設計）に進めますか？

背景：アイデア整理フェーズのまとめとして、進めるかどうかの判断です。

A. Go：このまま進める
B. 条件付き Go：条件を付けて進める（Other で条件を記入）
C. 保留：何かが決まるまで止める（Other で記入）
D. 中止
X. Other (please specify)

[Answer]: A. Go：このまま進める

## Consolidated Summary Confirmation

回答のまとめ：

- 反映事項の取り込み：企画書（initiative-brief）に統合版の範囲とバックログを載せ、以後はそれを正とする。承認済みの文書は書き換えない（Q1）
- リスクの受け入れ：次の 4 つをすべて対策込みで受け入れる（Q2）
  - 他の古いロググループでは GetLogEvents でも見えない可能性
  - 広い時間範囲では数分以上待つ
  - 1 週間は仮説
  - API 料金が未確認
- ラフな画面設計：思い描く形と合っている（Q3）
- MVP の完成目標日：2026-10-10（Q4）
- 判断：Go（Q5）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
