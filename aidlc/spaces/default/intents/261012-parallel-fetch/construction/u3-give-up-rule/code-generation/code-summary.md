# Code Summary — U3 あきらめの決まり（u3-give-up-rule）

上流：`code-generation-plan.md`（承認済み）、`unit-test-instructions.md`、`functional-design/`・`nfr-requirements/`・`nfr-design/`（security-design.md の「決まりの正本」を正とする）・`infrastructure-design/`。

## 作ったもの・変えたもの

| ファイル | 種類 | 内容 |
|----------|------|------|
| `crates/local-sights-core/src/schedule.rs` | 変更 | 使い切りの数えとあきらめの持ち主をここに移した（ADR-007、BR3.6）。公開：定数 `GIVE_UP_LIMIT`（3）、`StreamSchedule::new(names)`・`with_limit(names, limit)`（1 未満は 1）、`finish(order, end: StreamFetchEnd)`、構造体 `GiveUp { triggered_by, failure, skipped_count }`、`ScheduleOutcome.give_up: Option<GiveUp>`（`ScheduleOutcome` は Clone）。非公開：数えの構造体 `ExhaustionStreak`（retry.rs から移した）、`SlotEnd::of`・`SlotEnd::count_after` が終わり方の分け方を持つ（`StreamEnding` という型は作らない）。あきらめた後は数えを動かさない（BR3.3）。冒頭の文書を新しい持ち主に合わせた。テスト 8 件から 18 件 |
| `crates/local-sights-core/src/fetcher.rs` | 変更 | `StreamFetchEnd::Failed { exhausted, failure: ApiFailure }`（Copy をやめて Clone）。`CallResult::Failed` の値をそのまま入れる。`streak_ending` をなくした。`Failed { exhausted: … }` と比べていたテスト 3 か所を `matches!` で exhausted と失敗の種類を確かめる形にした（NFR 設計レビュー R-05） |
| `crates/local-sights-core/src/retry.rs` | 変更 | `StreamEnding`・`ExhaustionStreak`・`RetryPolicy::max_consecutive_exhausted` と数えのテスト 2 件をなくした（テストは schedule.rs に移した） |
| `crates/local-sights-core/src/coordinator.rs` | 変更 | `StreamSchedule::new(names)` を使い、`StreamFetchEnd` から `SlotEnd` への変換をなくして `finish` にそのまま渡す。FetchJob への記録の仕方（終わった順、Skipped も 1 本ずつ）は変えていない（BR3.7） |
| `crates/local-sights-core/tests/u3_give_up_rule.rs` | 新規 | 結合テスト 5 件（偽物のゲートウェイと止めた時計） |

## 主な実装の判断

- あきらめの時点の未着手は、正本どおり s14 以降（8 本の枠は 1 本終わるごとに埋まるので、s09〜s13 は読み途中）
- Skipped にしたストリームには 3 本目の失敗を写し、`give_up` に名前・失敗・本数を一度だけ記録する。未着手が 0 本でも `give_up` は残り、`skipped_count` は 0
- 閉じて止めたもの（Aborted）は数えに入れない（BR3.5）
- 画面（FetchSink・JobStatus・Tauri）とアプリ側は変えていない

## テスト

| 部品 | 前 | 後 |
|------|----|----|
| `schedule::tests` | 8 | 18（既存 8 件は呼び出しだけ直す、retry.rs から 2 件移す、新しい 8 件はテスト先行） |
| `retry::tests` | 22 | 20（2 件を schedule.rs へ移した） |
| `fetcher::tests` | 7 | 7（3 か所を `matches!` に書き直し） |
| `tests/u3_give_up_rule.rs` | 0 | 5 |

ライブラリの単体テストは 416 件から 424 件（ignored 5 件は同じ）。既存の結合テスト（u1_concurrent_fetch・u1_fetch_flow・u2_call_pacing・u2_log_group_listing・u3_fetch_flow・u5_filter_flow・u6_cache_flow）は変えずに通る。

テスト先行の確かめ：新しい schedule のテストを書いた時点ではコンパイルできずに失敗し、実装の後に 18 件すべて通った。

| 確かめ | 結果 |
|--------|------|
| `cargo fmt --all --check` | 合格 |
| `cargo clippy -p local-sights-core --all-targets`・`cargo clippy --workspace --all-targets` | 警告 0 |
| `cargo test -p local-sights-core`（結合テストを含む）・`--doc`・`--example fetch_check` | すべて合格 |
| `cargo test -p local-sights` | 12 件合格 |
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 合格。97.63%（schedule 99.41%、retry 97.66%、fetcher 92.79%、coordinator 98.18%） |

## 計画からのずれ

1. `finish(order, end: StreamFetchEnd)` は新しい `StreamFetchEnd::Failed { failure }` がないとビルドできず、lib のテストは lib 全体がビルドできないと動かないため、Step 4（fetcher.rs・retry.rs）と Step 5（coordinator.rs）を Step 3 の実装と同時に行った。手順の中身は計画どおり
2. 写された失敗が「3 本目のもの」であることを種類で見分けるため、テストデータの一部に `FailureKind::Network`（再試行の対象で、使い切りの扱いは Throttled と同じ）を使った
3. 変えた Rust ファイルの既存のコメントが英語のため、足したコメントと文書も英語で書いた
4. 最初の `cargo fmt --all --check` で新しく書いた 2 ファイルに差が出たため、`cargo fmt --all` で整形してから確かめ直した

## Assumptions & Open Questions

None.
