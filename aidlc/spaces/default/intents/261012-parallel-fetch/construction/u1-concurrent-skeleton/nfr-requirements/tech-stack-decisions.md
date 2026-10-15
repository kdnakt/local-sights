# Tech Stack Decisions — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

今のアプリの技術（codekb の technology-stack.md）を変えない。U1 で足すのは 1 つだけ。

| 対象 | 選んだもの | 理由 | 退けたもの |
|------|------------|------|------------|
| 同時に読む処理を並べる入れ物 | `futures-util`（FuturesUnordered。既定の機能は切り、`std` の機能だけ有効にする） | 取得 1 回のタスクの中で、借用したゲートウェイと Retrier のまま、終わった順に結果を受け取れる。広く使われ、ライセンスは MIT／Apache-2.0 で cargo-deny の許可に入る | `tokio::spawn`／JoinSet（`'static` が要り、今の借用の形が使えない）、チャネル（機能設計の 1 回目のレビューで、終わりの判定と詰まりの扱いが複雑と指摘された） |
| 時計 | `tokio::time`（今の依存。テストでは `tokio::time::pause` と `advance` で時間を止めて進める） | 許可待ちと再試行の待ちを同じ時計で扱え、テストで時間を決められる | 自前の時計の差し替え（今の再試行の待ちが tokio の時計を使っているため、揃える） |
| 許可の間隔の管理 | 標準ライブラリのロック（`std::sync::Mutex`）。ロックの中では待たない | 枠の予約は短い計算だけで、await をまたがない | 外部の速さの制限のクレート（引き下げと戻しを U2 で足すため、自前の小さな部品のほうが変えやすい） |
| SDK の再試行を止める | AWS SDK for Rust の、操作ごとの設定の上書き（GetLogEvents の呼び出しだけ再試行の回数を 1 に） | 他の 2 つの API の動きを変えずに済む | クライアント全体の再試行を止める（ロググループ一覧が再試行を失う。機能設計のレビュー 1 の R-04） |

## Assumptions & Open Questions

None.
