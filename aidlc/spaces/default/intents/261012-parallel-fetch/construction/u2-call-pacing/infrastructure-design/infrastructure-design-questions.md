# Infrastructure Design 質問票 — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/nfr-design/`（security-design.md・logical-components.md）、`construction/u2-call-pacing/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`、U1 の `infrastructure-design/`（cicd-pipeline.md、Q1）。

U2 は作業単位の種類が library で、ライブラリの中の CallPacer を変えるだけである。クラウドの資源・配布の形・CI の手順は U1 で決めたまま変えないため、新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **CI**：U1 の cicd-pipeline.md のとおり（fmt・clippy・テストと行カバレッジ 80%・画面の検査・cargo-deny・Tauri アプリのビルド）。U2 で手順を足さない。
- **ブランチと PR**：U1 と同じ `claude/parallel-fetch` で続けて作り、作業単位をまとめた PR を、CI がすべて通ってから `main` にスクワッシュマージする。必須チェックの名前は変えない（project.md Corrections）。
- **配布**：U2 だけではタグを付けない。

## Consolidated Summary Confirmation

- クラウドの資源・配布の形・CI の手順は U1 のまま。U2 で足すものはない
- 確かめは CI の Rust library のジョブ（テストと行カバレッジ 80%）で行う。テストは止めた時計と偽物のゲートウェイで、実際の AWS に接続しない
- ブランチは U1 と同じ。必須チェックの名前は変えない。U2 だけではタグを付けない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
