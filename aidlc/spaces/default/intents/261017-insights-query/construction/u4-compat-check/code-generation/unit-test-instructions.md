# Unit Test Instructions — U4 互換の確認用プログラム（u4-compat-check）

## テストの仕組み

- Rust：確認用プログラムは `crates/local-sights-core/examples/compat_check.rs` とその下の `examples/compat/` に置き、テストは各モジュールの中の `#[cfg(test)] mod tests` に書く。例のテストは `cargo test --example compat_check` で流れる（`cargo test --workspace` でも流れる）。一時フォルダは今の `tempfile` を使う。
- スクリプト：`bash -n` と、PATH の先頭に置いた偽の `aws`（決まった JSON を返すシェルのスクリプト）で流すテスト。Rust のテストの中から `bash` を起動して確かめ、Windows のランナーでは飛ばす（`#[cfg(unix)]`）。
- 進め方は計画の Testing Contract のとおり。比べ方（クエリの組の読み込み、フォルダと期間の確かめ、値の書き方の合わせ、多重集合と並びの比べ、判定と終了コード）はテストを先に書いて失敗を確かめてから実装する。確認用プログラムの配線とスクリプトは、実装してからテストを書いて実行する。

## U4 のテストの実行（この作業単位だけ）

```bash
# 確認用プログラムの比べ方・配線・スクリプトのテスト
cargo test -p local-sights-core --example compat_check

# スクリプトの書き方の確かめ
bash -n scripts/compat/insights-export.sh
```

Step 9 では、既存のテストが壊れていないことを確かめるため、あわせて `cargo test --workspace` と `npx vitest run` も流す。

## 期待するカバレッジ

- 確認用プログラムは例のため、今の CI の行カバレッジ（ライブラリの `cargo-llvm-cov`）の対象の外になる。ライブラリの行カバレッジ 80% の下限は今のまま下げない。
- 比べ方の公開関数には必ずテストを書く。

## 偽物の使い方

- AWS は呼ばない。配線のテストは、U1〜U3 の偽のゲートウェイ（ストリームごとにページを返すもの）を使う。
- Logs Insights の書き出しは、テストの中で一時フォルダに作る（`settings.json` と `<id>.json`）。
- スクリプトのテストは、偽の `aws` が start-query に決まった queryId を、get-query-results に決まった結果を返す。

## テストのデータ

- 書き出しの行は、U2 の Q1a・Q1b の出力と同じ形（`{field, value}` の並び、@ptr を含む、@timestamp は `YYYY-MM-DD HH:MM:SS.sss`）で、テストの中で作る。実際のログ（CloudTrail など）はリポジトリに置かない（NFR4）。

## 手元の確認（自動テストではない）

```bash
# 1. Logs Insights の結果を書き出す（リポジトリの外のフォルダに。END は 30 分以上前）
PROFILE=<プロファイル> REGION=us-east-1 GROUP=CloudTrail START=<秒> END=<秒> OUT=~/insights-export \
  ./scripts/compat/insights-export.sh

# 2. アプリの結果と比べる
cargo run -p local-sights-core --example compat_check -- --export-dir ~/insights-export
```

- 結果の要約（ロググループの種類、期間の長さ、件数、結果の数、違いのあったクエリと原因の見立て）を記録に残す。ログの中身は残さない。
