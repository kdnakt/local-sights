# Tech Stack Decisions — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

新しい言語・フレームワーク・依存は足さない（NFR8.11）。

| 対象 | 選んだもの | 理由 | 採らなかったもの |
|------|-----------|------|------------------|
| ライブラリの時計 | Rust 標準の戻らない時計（`std::time::Instant`）を、差し替えられる小さな型で包み、AppSession を組み立てるときに渡す。AppSession と同じく複製でき、別のタスクと共有できる形にする | PC の時刻に左右されない（NFR6.8）。テストで決まった時刻を与えられる（NFR6.2、R-04） | PC の時刻（`SystemTime`）：時刻を変えると負や飛びが起きる。tokio の時計：AppSession は非同期の外でも使うため持ち込まない |
| 画面の時計 | ブラウザの戻らない時計（`performance.now`）と 1 秒ごとのタイマーを、経過だけを描く小さな部品の中のフックで使う | 再試行の待ちの間も進む（NFR6.5）。描き直しを経過に閉じ込める（NFR5.3） | 上位の部品（App）での時計：毎秒ログの表まで描き直す |
| 表記と合わせ直し | 画面の書式の関数と、合わせ直し・保持の判断の関数（どちらも純粋な関数） | テスト先行で境目を確かめやすい（NFR6、BR4.7） | `Intl.DurationFormat`：環境による差があり、mm:ss の形を決めきれない |
| 画面へ渡す形 | 今の `session-changed` の SessionView に `elapsed` を足す（serde の camelCase） | 新しいイベントを作らない（C5） | 経過専用のイベント、fetch-progress への追加：送信が増える（ADR-005） |
| テスト | Rust の標準のテスト、Vitest（jsdom、時計の差し替えは Vitest の偽のタイマー）、カバレッジは `cargo-llvm-cov` と `@vitest/coverage-v8`（今のもの） | 今の道具で足りる | 新しいテストの道具 |

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR7.3 | 行カバレッジはライブラリ・画面ともに 80% 以上を保つ（CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` と `npm run test:coverage`） | CI | NFR7、team.md Testing Posture |
| NFR6.12 | 純粋な判断はテストを先に書く：表記の関数、画面の合わせ直しと保持の判断（受け取った elapsed と今の表示から次の表示の起点を決める関数、NFR6.6・NFR6.9）、AppSession の経過の決まり。タイマーに触れるフックと状態行の描画は実装してからテストを書く | コード生成の計画とテストの順 | NFR6、team.md Testing Posture |

## Assumptions & Open Questions

None.
