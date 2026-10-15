# Code Summary — U2 呼び出しの速さ（u2-call-pacing）

上流：`code-generation-plan.md`（承認済み）、`unit-test-instructions.md`、`functional-design/`・`nfr-requirements/`・`nfr-design/`・`infrastructure-design/`。

## 作ったもの・変えたもの

| ファイル | 種類 | 内容 |
|----------|------|------|
| `crates/local-sights-core/src/pacing.rs` | 変更 | PacingState（今の速さ・next_permit_at・hold_decrease_until・quiet_since・最も低い速さ）を 1 つのロックの中の Copy の構造体にした。`report_throttled`・`current_rate_per_second`・`lowest_rate_per_second`、定数 `RECOVERY_STEP`（5 秒）・`DECREASE_HOLD`（1 秒）。冒頭の文書を直した（U1 のコード生成のレビュー R-01、毒化の説明）。テスト 10 件追加 |
| `crates/local-sights-core/src/fetch_stats.rs` | 変更 | `summary(streams, elapsed_ms, min_rate, status)` と `FetchSummary::min_rate`。欄は max_in_flight の後・status の前。書式のテストを U2 の形に書き直し、1 件追加 |
| `crates/local-sights-core/src/fetcher.rs` | 変更 | `counted_call` で Throttled を数えた直後に `report_throttled()`。テスト 2 件追加 |
| `crates/local-sights-core/src/coordinator.rs` | 変更 | 要約に最も低い速さを渡す。一覧の途中で閉じた取得はリージョンの上限。中だけで使う構造体 `FetchEnd` |
| `crates/local-sights-core/tests/u2_call_pacing.rs` | 新規 | 結合テスト 6 件 |
| `crates/local-sights-core/tests/u1_concurrent_fetch.rs` | 変更 | 要約の 1 行全体を比べる期待値に `min_rate=12` を足した（1 行） |
| `crates/local-sights-core/examples/fetch_check.rs` | 変更 | 冒頭のコメントの要約の書式に `min_rate` を足しただけ |
| `README.md` | 変更 | 要約の 1 行の説明に `min_rate` |

## 主な実装の判断

- 下げ・戻し・枠の予約は、古い状態から新しい状態を計算する関数にし、ロックの中で 1 回の代入で書く。壊れたロックの中身は、どれかの操作の前か後の値になる
- 下げてよい時刻は `max(今 + 1 秒, next_permit_at)`、比べ方は `今 < hold`（BR2.3）
- 戻しは端数を持ち越し、上限に達しているときは `quiet_since` を動かさない（BR2.4）
- `current_rate_per_second()` は最後の知らせか予約の時点の値を読むだけで、戻しを当てはめない
- 一覧（DescribeLogStreams）の呼び出しは CallPacer を作る前に `call_unpaced` で行うので、構造上 CallPacer に知らせが届かない
- 画面（FetchSink・JobStatus・Tauri）は変えていない（FR2.6）

## テスト

| 部品 | 前 | 後 |
|------|----|----|
| `pacing::tests` | 8 | 18（新しい 10 件はテスト先行） |
| `fetch_stats::tests` | 5 | 6（テスト先行） |
| `fetcher::tests` | 5 | 7 |
| `tests/u2_call_pacing.rs` | 0 | 6 |

ライブラリの単体テストは 403 件から 416 件（ignored 5 件は同じ）。

| 確かめ | 結果 |
|--------|------|
| `cargo fmt --all --check` | 合格 |
| `cargo clippy -p local-sights-core --all-targets`・`cargo clippy --workspace --all-targets` | 警告 0 |
| `cargo test -p local-sights-core`（結合テストを含む）・`--doc`・`--example fetch_check` | すべて合格 |
| `cargo test -p local-sights` | 12 件合格 |
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 合格。97.55%（pacing 100%、fetch_stats 100%、coordinator 98.10%、fetcher 91.13%） |

## 計画からのずれ

1. `tests/u1_concurrent_fetch.rs` の `no_selected_stream_completes_without_reading` は要約の 1 行全体を比べているため、期待値に `min_rate=12` を足した。1 行全体の一致は変えていない（NFR7.1 の「要約の書式のテストを直す」に当たる）
2. `examples/fetch_check.rs` の冒頭のコメントを直した（コードは変えていない）
3. Step 4 で `summary` の引数を増やすと呼び出し元がビルドできないため、Step 5 の coordinator の変更を同時に行った
4. `fetcher::tests` の 2 件目は「Throttled 以外の失敗では速さが下がらない」にした。DescribeLogStreams の制限で速さが変わらないことは、結合テスト（一覧を 1 回制限して `min_rate=12`・`throttled=0`）で確かめる
5. R-07 のテストは「速さ 3 で枠が 2.7 秒先まで予約されているとき、1 に下げる」に加え、「6 から 3 に下げて 2.7 秒先まで予約済み → 1.5 秒・2.5 秒では 3 のまま → 3.0 秒で 1」の形も確かめる

## Assumptions & Open Questions

None.
