# Performance Test Instructions — リリース準備

## 対象

この作業はアプリの機能を変えないため、アプリの性能テストは新しく行わない。既存の性能確認（README の `cargo test -p local-sights-core --release --lib -- timeline:: --ignored` など）はそのまま使える。

性能に関わる要件は、リリース用ワークフローの所要時間だけである。

- NFR7：タグの push から公開まで 30 分以内を目安とし、各ジョブにタイムアウトを設ける。

## 実行の方法

- タイムアウトの設定：`release.yml` の `verify`（10 分）・`test`（20 分）・`build-and-publish`（40 分）に `timeout-minutes` があることを、定義で確かめる（この工程で実行できる）。
- 実際の所要時間：本人がタグ `v0.1.0` を push したときに、GitHub Actions の実行時間で確かめる（目安であり合否の門ではない）。

## カバレッジの目安

なし。

## テストデータと環境

なし。
