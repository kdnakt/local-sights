# Infrastructure Design 質問票 — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/nfr-design/`（security-design.md・logical-components.md）、`construction/u3-give-up-rule/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`、U1・U2 の `infrastructure-design/cicd-pipeline.md`。

U3 は作業単位の種類が library で、ライブラリの中の StreamSchedule・RetryPolicy・EventFetcher を整えるだけである。クラウドの資源・配布の形・CI の手順は U1 のまま変えないため、新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **CI**：U1・U2 と同じ（fmt・clippy・テストと doc テストと行カバレッジ 80%・画面の検査・cargo-deny・Tauri アプリのビルドとテスト）。U3 で手順を足さない。
- **ブランチ**：U1・U2 と同じ `claude/parallel-fetch` に続けて積む。作業単位をまとめた PR は、CI がすべて通ってから `main` にスクワッシュマージする。必須チェックの名前は変えない。
- **配布**：U3 だけではタグを付けない。

## Consolidated Summary Confirmation

- クラウドの資源・配布の形・CI の手順は U1・U2 のまま。U3 で足すものはない
- 確かめは CI の Rust library のジョブ（テスト・doc テスト・行カバレッジ 80%）と Tauri アプリのジョブ（アプリ側が変わらずビルド・テストできること）。実際の AWS には接続しない
- ブランチは U1・U2 と同じ。必須チェックの名前は変えない。U3 だけではタグを付けない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
