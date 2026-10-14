# Security Requirements — U5 中断（u5-cancel）

上流：`construction/u5-cancel/functional-design/`（functional-spec.md・rules.md BR5.1〜BR5.14、レビュー 2 回目の新しい指摘 2 件）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C3〜C7）、`codekb/local-sights/technology-stack.md`。質問票は `nfr-requirements-questions.md`（まとめの確認）。

| ID | 要件 | 確かめ方 | 出典 |
|----|------|--------------|------|
| NFR8.13 | 中断の注意・状態行・要約の 1 行に足すのは、本数（N）と決まった語（Cancelled、中断など）だけ。ストリーム名・ロググループ名・接続の情報・失敗の詳細を足さない | 注意の文言と要約の 1 行の組み立てのテストで、入るのが数値と決まった語だけであること | project.md Forbidden、NFR8 |
| NFR8.14 | 新しい AWS の API・依存・外への送信を足さない。呼ぶのは読み取り 3 つだけのまま | コードの差分の確認。CI の cargo-deny と npm audit が今のまま通る | project.md Forbidden |
| NFR8.15 | Tauri の権限に足すのは `cancel_fetch` だけ。このコマンドは走っている取得を止めるだけで、入力を取らない | `src-tauri/build.rs` のコマンドの一覧、`src-tauri/capabilities/default.json` の allow、`generate_handler` への登録の 3 か所に `cancel_fetch` がそろい、ほかの権限が増えていないことを差分で確かめる。コマンドのテスト（押せないときは何もしない） | ADR-004、C6、レビュー R-04 |
| NFR6.13 | 自動テストと CI は実際の AWS に接続しない。中断のテストは偽物のゲートウェイと止めた時計で行う | 新しいテストはすべて FakeGateway を使う | project.md Forbidden、NFR6 |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 画面から中断のコマンドを何度も送られて状態が崩れる | 何度送っても結果が同じで、押せないときは何もしない（BR5.1） |
| 中断の注意に接続の情報や失敗の詳細が混ざる | 注意は本数と決まった文だけ（NFR8.13） |

## Assumptions & Open Questions

None.
