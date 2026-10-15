# Performance Test Instructions — 261012-parallel-fetch

この作業の目的は「ロググループ全体の取得を速くする」こと（IB-07）。速さは 2 段で確かめる。

1. **自動の測定（偽物と止めた時計）**：速さの上限・同時の本数・全件がそろうまでの時間を、実時間に左右されずに確かめる。CI でも走る。
2. **手元の確認（実際の AWS）**：開発者本人の PC で、main の版とこの枝の版を同じ条件で取得し、件数が同じでこの枝のほうが速いことを確かめる（U1 の NFR2.1）。CI では走らせない。

あわせて、既存の 100 万件の一覧の速度測定が今と同じ基準で通ることを確かめる（U1 の NFR5.1、U4 の NFR5.3）。

## 1. 自動の測定

| 目標 | テスト | コマンド |
|------|--------|----------|
| U1-NFR1.1 全件がそろうまで 10 秒以下（200 ms × 100 本、東京 12 回／秒） | `tests/u1_concurrent_fetch.rs` | `cargo test -p local-sights-core --test u1_concurrent_fetch` |
| U1-NFR1.2 最初の 1 件が 1 本ずつより早く出る | 同上 | 同上 |
| U1-NFR1.3 同時に出ている呼び出しの最大がちょうど 8 | 同上 | 同上 |
| U1-NFR3.2〜3.3 どの 1 秒でも上限を超えない、許可の間隔 | `src/pacing.rs` | `cargo test -p local-sights-core --lib pacing::` |
| U2-NFR3.5〜3.6 制限で間隔が広がり、戻っても上限を超えない | `tests/u2_call_pacing.rs` | `cargo test -p local-sights-core --test u2_call_pacing` |
| U5-NFR4.2〜4.5 中断の後に呼び出しを始めない、すぐ止まる | `tests/u5_cancel.rs` | `cargo test -p local-sights-core --test u5_cancel` |
| U4-NFR5.3 1 秒ごとの描き直しは経過の部品だけ | `src/components/ElapsedTime.test.tsx` | `npx vitest run src/components/ElapsedTime.test.tsx` |

時間は `start_paused = true` の止めた時計で進めるため、結果は PC の速さに左右されない。

## 2. 100 万件の一覧の速度測定（既存）

```sh
cargo test -p local-sights-core --release --lib -- --ignored
```

`timeline.rs`（2 件）と `log_view.rs`（3 件）の 5 件。基準はテストの中にあり、この作業では変えていない。通常の `cargo test` では `#[ignore]` で飛ばされる。

## 3. 手元の確認（実際の AWS、U1-NFR2.1）

開発者本人の macOS で、本人の AWS 認証情報を使って行う（team.md の Walking Skeleton）。自動テストと CI からは行わない。

### 準備

- テスト用のロググループ（東京、`ap-northeast-1`）。ストリームが数十〜数百本あり、困りごとの場面に近いもの。
- 同じ時間範囲（たとえば 1 時間）を決めておく。範囲の中にログがあること。
- 読み取りだけの権限（DescribeLogGroups・DescribeLogStreams・GetLogEvents）を持つプロファイル。

### 手順

```sh
# この枝の版
git switch claude/parallel-fetch
time cargo run --release -p local-sights-core --example fetch_check -- \
  --profile <プロファイル名> --region ap-northeast-1 --log-group <ロググループ名> \
  --start "yyyy/mm/dd hh:mm:ss" --end "yyyy/mm/dd hh:mm:ss" > /tmp/branch.tsv

# main の版（--region はこの作業で足した引数なので、main では AWS_REGION で渡す）
git switch main
time env AWS_REGION=ap-northeast-1 cargo run --release -p local-sights-core --example fetch_check -- \
  --profile <プロファイル名> --log-group <ロググループ名> \
  --start "yyyy/mm/dd hh:mm:ss" --end "yyyy/mm/dd hh:mm:ss" > /tmp/main.tsv

wc -l /tmp/main.tsv /tmp/branch.tsv
```

- 先に `cargo build --release -p local-sights-core --example fetch_check` でビルドを済ませ、ビルドの時間を測らない。それぞれ 3 回測り、中央値を比べる（`examples/fetch_check.rs` の説明と同じ）。
- zsh では `time` は予約語で `NAME=value` の後に置けないため、`time env AWS_REGION=…` の順に書く（f95829a）。
- この枝の版は標準エラー出力に要約の 1 行（`elapsed_ms=`・`calls=`・`throttled=`・`max_concurrent=`・`min_rate=`・`status=`）を出す。
- 標準出力のファイルにはログの本文が入るので、確認が済んだら消す。リポジトリに置かない。

### 合格の条件

- 件数（行数）が同じ。
- この枝のほうがかかった時間が短い。数字の目標は置かない（U1 の NFR2.1、選択肢の例の数値は目標とみなさない：project.md の Corrections）。
- 結果は `test-results.md` の「手元の確認」に、件数・それぞれの時間・ストリームの本数だけを書く。プロファイル名・ロググループ名はリポジトリに置かない（project.md の Forbidden）。

## 対応する後のステージ

この作業の実行計画では performance-validation は飛ばす。そのため、手元の確認の結果はこのステージの目標の照合表で判定する。
