# CI Pipeline — 質問

CI の道具（GitHub Actions）、ブランチの進め方（トランクベース、PR → CI 緑 → スクワッシュマージ）、マージ前の品質の門（rustfmt・clippy・Prettier・ESLint・tsc・Vitest・cargo test・npm audit・cargo-deny・版の検査）、成果物の置き場所（GitHub Releases）は、これまでの工程で決まっている。ここでは、Code Generation と Build and Test のレビューで残った 2 点だけを聞く。

## Q1. `.dmg` のビルドを、タグを付ける前にも確かめるか

今のままだと、macOS で `.dmg` を作る処理（`tauri build --bundles dmg`）が初めて動くのは、タグを push したときになる。そこで失敗すると、タグを付け直す手間がかかる。

A. 手で動かせる「試しビルド」を足す（GitHub の画面から好きなときに実行でき、`.dmg` を作るだけで公開はしない。作った `.dmg` は実行結果から取得できる）
B. プルリクエストの CI でも毎回 `.dmg` を作る（CI が macOS で 10〜15 分ほど長くなる）
C. 足さない。タグの push のときに確かめる
X. Other (please specify)

[Answer]: A

## Q2. 書き込み権限を持つジョブの外部 action を固定するか

リリース用ワークフローの公開ジョブ（書き込み権限あり）は、外部の action（`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`）を、作者が付け替えられるタグで参照している。付け替えられると、意図しないコードが書き込み権限付きで動く可能性がある。既存の CI も同じやり方である。

A. リリース用ワークフローの外部 action だけ、コミットの SHA で固定する（Dependabot が更新の PR を出す）
B. すべてのワークフロー（既存の CI を含む）の外部 action を SHA で固定する
C. 今のままにする
X. Other (please specify)

[Answer]: B
