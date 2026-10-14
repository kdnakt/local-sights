# Infrastructure Design 質問票 — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-design/`（security-design.md の設計の正本、logical-components.md ほか）、`construction/u5-cancel/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`、今の CI（`.github/workflows/ci.yml`）、`src-tauri/build.rs`・`src-tauri/capabilities/default.json`。

U5 はライブラリ・アプリ・画面を変えるが、クラウドの資源・ネットワーク・保存先・配布の形・CI の手順は変えない。アプリの権限に `cancel_fetch` を 1 つ足すだけである。新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **CI**：今のジョブのまま（Rust library、Front end、cargo-deny、Tauri app）。必須チェックの名前は変えない（project.md Corrections）。
- **権限**：`src-tauri/build.rs` のコマンドの一覧、`src-tauri/capabilities/default.json` の allow、`generate_handler` に `cancel_fetch` を足す。ほかの権限は増やさない。
- **ブランチ**：U1〜U4 と同じ `claude/parallel-fetch` に積む。作業単位をまとめた PR は、CI がすべて通ってから `main` にスクワッシュマージする。
- **配布**：U5 だけではタグを付けない。
- **手元の確認**：GUI で、［中断］・［中断中…］・注意・状態行の「中断」、キーボードだけで押せること、VoiceOver で注意が一度読まれること、中断した範囲がキャッシュから表示されないことを一度確かめる。

## Consolidated Summary Confirmation

- クラウドの資源・ネットワーク・保存先・配布の形・CI の手順は U1〜U4 のまま。アプリの権限に中断の操作を 1 つ足すだけ
- 確かめは CI の各ジョブ（止め方・本数の数え方・終わりの状態・画面のテスト、行カバレッジ 80%、Tauri アプリのビルドとテスト）。実際の AWS には接続しない
- 手元の GUI で、中断のボタン・注意・状態行・キーボード・読み上げ・キャッシュに残らないことを一度確かめる
- ブランチは U1〜U4 と同じ。必須チェックの名前は変えない。U5 だけではタグを付けない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
