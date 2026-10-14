# Infrastructure Design 質問票 — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-design/`（security-design.md の設計の正本、logical-components.md ほか）、`construction/u4-elapsed-time/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`、今の CI（`.github/workflows/ci.yml`）と `vitest.config.ts`。

U4 は AppSession と画面の状態行を変えるだけで、クラウドの資源・ネットワーク・保存先・配布の形・CI の手順を変えない。新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **CI**：今のジョブのまま。Rust library（fmt・clippy・テストと行カバレッジ 80%）、Front end（tsc・eslint・prettier・Vitest と行カバレッジ 80%・build・npm audit）、cargo-deny、Tauri app（ubuntu-latest / macos-latest）。必須チェックの名前は変えない（project.md Corrections）。
- **ブランチ**：U1〜U3 と同じ `claude/parallel-fetch` に積む。作業単位をまとめた PR は、CI がすべて通ってから `main` にスクワッシュマージする。
- **配布**：U4 だけではタグを付けない。
- **監視**：外へ送るものはない。U1 の要約の 1 行は変えない。経過は利用者が状態行で見る。
- **手元の確認**：GUI で、取得中に経過が 1 秒ごとに進むこと（再試行の待ちの間も）、終わると残ること、英日の文言、VoiceOver で終わったときに最終の経過が読まれることを一度確かめる。

## Consolidated Summary Confirmation

- クラウドの資源・ネットワーク・保存先・配布の形・CI の手順は U1〜U3 のまま。U4 で足すものはない
- 確かめは CI の Rust library のジョブ（AppSession の経過のテスト、行カバレッジ 80%）と Front end のジョブ（表記・合わせ直し・経過の部品・状態行のテスト、行カバレッジ 80%）。実際の AWS には接続しない
- 手元の GUI で、経過が進み・残ること、英日の文言、終わったときの読み上げを一度確かめる
- ブランチは U1〜U3 と同じ。必須チェックの名前は変えない。U4 だけではタグを付けない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
