# Tech Stack Decisions — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/functional-design/`、`codekb/local-sights/technology-stack.md`、U1・U2 の `nfr-requirements/tech-stack-decisions.md`。まとめの確認は `nfr-requirements-questions.md`。

U3 は新しい技術を足さない。

| 分野 | 使うもの | 理由 |
|------|----------|------|
| 数えの決まり | Rust の純粋な関数と構造体（StreamSchedule の中） | 非同期も入出力も持たない純粋な部品として、テストを先に書ける（約束 C2） |
| あきらめる本数 | StreamSchedule の定数（3） | 画面や設定からは変えない。RetryPolicy から項目をなくす（ADR-007） |
| テスト | Rust の標準のテスト、`tests/support/fake_gateway.rs` と止めた時計 | 終わる順を入れ替える台本を偽物のゲートウェイの呼び出し時間で作る |
| カバレッジ | `cargo-llvm-cov`（CI） | 行カバレッジ 80% |
| 依存 | 追加なし | cargo-deny の対象は変わらない |

## 却下した案

| 案 | 却下の理由 |
|----|------------|
| あきらめる本数を RetryPolicy に残し、StreamSchedule から読む | 数えとストリームの状態が 2 つの部品に分かれる（ADR-007 の却下案） |
| あきらめる本数を `run_fetch` の引数にする | 使う側（確認用プログラム・Tauri アプリ）がすべて既定の 3 を渡すだけで、変える場面がない |

## Assumptions & Open Questions

None.
