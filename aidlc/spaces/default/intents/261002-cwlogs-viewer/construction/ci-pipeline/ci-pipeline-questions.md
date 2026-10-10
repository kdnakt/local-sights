# CI Pipeline 質問票

上流の成果物：`construction/build-and-test/build-and-test-summary.md`・`test-results.md`（実行したコマンドと結果）、U1〜U7 の `code-generation/code-summary.md`。開発の進め方は `memory/team.md`（トランクベース、PR と CI が通ってから `main` にスクワッシュマージ、`cargo fmt --check` 必須、clippy は警告をエラーにしない、`cargo-deny`、secret scanning、テストと CI は AWS に接続しない）。配布はソースからのビルドだけで、成果物の置き場（レジストリ）はない（team.md Deployment）。

すでに決まっていて聞かないこと：ブランチ戦略（トランクベース、`main` へのスクワッシュマージ）、マージ前に必須の品質ゲート（整形・clippy・テスト・型検査・ESLint・Prettier・依存関係の検査）、成果物のレジストリ（なし）、CI から AWS に接続しないこと。

リポジトリの状態：`.github/workflows/` はまだない。リモートは GitHub（`kdnakt/local-sights`）。

## Q1. CI はどの道具で動かしますか？

背景：リポジトリは GitHub にあり、サーバーもデプロイ先もない。CI の仕事は PR と `main` のビルド・テスト・検査だけ。

A. GitHub Actions（リポジトリと同じ場所。ワークフローは `.github/workflows/ci.yml`）
B. その他（CircleCI、Buildkite など。名前を教えてください）
X. Other (please specify)

[Answer]: A. GitHub Actions（リポジトリと同じ場所。ワークフローは `.github/workflows/ci.yml`）

## Q2. CI の実行環境（ランナー）はどうしますか？

背景：ライブラリと画面のテストは Linux で動く。Tauri アプリの組み立て（`cargo build -p local-sights`）は Linux では WebKitGTK の開発パッケージの導入が要り、MVP の対象 OS は macOS。GitHub Actions の macOS ランナーは Linux の 10 倍の分数を消費する（公開リポジトリなら無料）。

A. Linux（ubuntu-latest）だけ。Tauri アプリの組み立ても Linux で行う（WebKitGTK を apt で入れる。所要 1〜2 分）
B. Linux でライブラリ・画面・検査を行い、Tauri アプリの組み立てだけ macOS（macos-latest）で行う（対象 OS と同じ環境で型と組み立てを確かめる）
C. Linux と macOS の両方で Tauri アプリを組み立てる（将来の Windows 対応も見据えて複数 OS の体裁にする）
X. Other (please specify)

[Answer]: C. Linux と macOS の両方で Tauri アプリを組み立てる（将来の Windows 対応も見据えて複数 OS の体裁にする）

## Q3. `cargo-deny` で何を検査しますか？

背景：team.md は「`cargo-deny` による依存関係のチェック（脆弱性を含む）」としている。`deny.toml` はまだない。プロジェクトのライセンスは Apache-2.0。

A. 脆弱性（advisories）・ライセンス（許可リスト：MIT・Apache-2.0・BSD 系・ISC・Unicode・Zlib・MPL-2.0。それ以外は失敗）・重複する版（bans：警告のみ）・取得元（sources：crates.io だけ）の 4 つ
B. 脆弱性（advisories）だけ。ライセンスと重複は検査しない
X. Other (please specify)

[Answer]: A. 脆弱性（advisories）・ライセンス（許可リスト：MIT・Apache-2.0・BSD 系・ISC・Unicode・Zlib・MPL-2.0。それ以外は失敗）・重複する版（bans：警告のみ）・取得元（sources：crates.io だけ）の 4 つ

## Q4. 依存関係の自動更新と、`main` の定期的な検査はどうしますか？

背景：公開後に新しい脆弱性の勧告が出ても、PR がなければ CI は走らない。Dependabot は cargo・npm・GitHub Actions の更新 PR を自動で出せる。

A. Dependabot を cargo・npm・github-actions に対して週 1 回で設定し、`main` に対して週 1 回の定期実行（`cargo deny check advisories` と `npm audit`）も入れる
B. Dependabot だけ設定し、定期実行は入れない
C. どちらも入れない（手で更新する）
X. Other (please specify)

[Answer]: B. Dependabot だけ設定し、定期実行は入れない

## Follow-up Questions

## F1. `cargo deny check` を今の依存関係に対して実行すると 2 件で失敗します。どう対処しますか？

背景：(1) **RUSTSEC-2026-0104**（脆弱性）：`rustls-webpki 0.101.7` が、`aws-sdk-cloudwatchlogs` の既定機能 `rustls` が引き込む古い TLS ライブラリ（rustls 0.21）経由で依存に入っている。証明書失効リスト（CRL）を読むときだけ到達する panic で、このアプリは CRL を使わない。`aws-sdk-cloudwatchlogs` の既定機能を外して `default-https-client`・`rt-tokio` だけにすると rustls 0.21 は依存から消え（rustls 0.23 だけになる）、ライブラリのテスト 346 件・`cargo fmt`・clippy はそのまま通ることを確認済み（`Cargo.lock` から 147 行減る）。(2) **RUSTSEC-2024-0370**（保守停止）：`proc-macro-error 1.0.4` が Tauri → gtk → glib-macros 経由で入っている。Tauri 側の依存なのでこちらから外せない。

A. (1) は SDK の機能指定を変えて rustls 0.21 を依存から外す（`Cargo.toml`・`Cargo.lock` の変更。この PR に含める）。(2) は `deny.toml` の `ignore` に ID と理由（Tauri の依存、保守停止のみで脆弱性ではない）を書く
B. どちらも `deny.toml` の `ignore` に ID と理由を書く（依存関係は変えない。(1) の理由：CRL を使わないため影響なし）
C. (1) だけ機能指定を変え、(2) は `ignore` せず CI を赤のままにして Tauri の更新を待つ
X. Other (please specify)

[Answer]: A. (1) は SDK の機能指定を変えて rustls 0.21 を依存から外す（`Cargo.toml`・`Cargo.lock` の変更。この PR に含める）。(2) は `deny.toml` の `ignore` に ID と理由（Tauri の依存、保守停止のみで脆弱性ではない）を書く
