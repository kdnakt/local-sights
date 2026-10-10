# Build and Test 質問票

上流の成果物：U1〜U7 の `code-generation/code-generation-plan.md`（Testing Contract を含む）、`unit-test-instructions.md`、`code-summary.md`、`traceability.json`。要件は `inception/requirements-analysis/requirements.md`（FR1〜FR8、NFR1〜NFR16）。Test Strategy は Standard。

このステージでは、全単位をまたいだビルドとテストの手順書を作り、この環境で動かせる確認をすべて実行して結果を記録し、要件 ID ごとの対応（トレーサビリティ）を最終確認する。

前提として分かっていること：

- この環境（Linux のコンテナ）には WebKitGTK がなく、Tauri アプリの組み立て（`cargo build -p local-sights`）と `cargo clippy --workspace --all-targets` は動かせない。ライブラリ側（`local-sights-core`）の `cargo test`・`cargo clippy -p local-sights-core`・`cargo fmt --check` と、画面側の `vitest`・`tsc`・`eslint`・`prettier`・`npm audit` は動かせる。
- NFR1〜NFR3（絞り込みの速さ・100 万件の操作・取得中の応答）と NFR9〜NFR11（macOS で動く・標準部品の見た目・1024×640 とダークモード）は、要件の計測方法が「開発者の Mac で目視またはストップウォッチ」であり、この環境では確かめられない。
- 性能検証のステージ（4.6）はこのスコープでは実行しないため、この環境で確かめられない項目を後のステージに任せることはできない。確かめられなかった項目は「未確認」として記録し、このステージの結果は失敗の扱いになる（判断は人間が行う）。

## Q1. この環境で確かめられない項目（Tauri アプリの組み立て、ワークスペース全体の clippy、macOS での性能・応答・見た目の確認）はどう扱いますか？

背景：要件の NFR1〜NFR3・NFR9〜NFR11 と、各単位の「`cargo build -p local-sights` で組み立てを確かめる」は、開発者本人の macOS でしか確かめられない。手順書には手元で実行するコマンドと確認項目を書く。

A. このステージの中で手元（macOS）のコマンドと確認を実行して結果を教えてもらい、その結果を証拠として記録する（私は結果を待つ。確認できた項目は「達成」になる）
B. 手順書だけを書き、結果は「未確認」として記録して、人間の判断（失敗を受け入れて承認に進む、など）を仰ぐ。手元の確認は後で行う
X. Other (please specify)

[Answer]: B. 手順書だけを書き、結果は「未確認」として記録して、人間の判断（失敗を受け入れて承認に進む、など）を仰ぐ。手元の確認は後で行う

## Q2. 速さを測る `#[ignore]` のテスト（10 万件・100 万件の絞り込み、100 万件の時刻順マージ。リリースビルドで実行）を、この環境で実行して参考値として記録しますか？

背景：要件（NFR1・NFR2）の計測機は開発者の Mac で、この環境の値は参考値になる。リリースビルドには数分と数 GB のディスクを使い、実行後に `target/release` を消す。

A. 実行して参考値として記録する（NFR1・NFR2 の達成の判定は手元の Mac の値で行う）
B. 実行しない（手元の Mac だけで計測する）
X. Other (please specify)

[Answer]: A. 実行して参考値として記録する（NFR1・NFR2 の達成の判定は手元の Mac の値で行う）

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
