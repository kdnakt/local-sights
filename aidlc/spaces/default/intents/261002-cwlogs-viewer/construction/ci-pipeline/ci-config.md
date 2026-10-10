# CI Config — CI の構成（GitHub Actions）

上流：`construction/build-and-test/build-and-test-summary.md`・`test-results.md`（Build and Test で実行したコマンドと結果）、U1〜U7 の `code-generation/code-summary.md`（各単位のテストの実行方法）、`memory/team.md`（Way of Working・Code Style・Deployment）。質問票：`ci-pipeline-questions.md`（Q1 GitHub Actions、Q2 Linux と macOS の両方で Tauri アプリを組み立てる、Q3 cargo-deny は 4 つとも検査、Q4 Dependabot だけ、F1 cargo-deny の 2 件への対処）。

## 置いたファイル（リポジトリ）

| ファイル | 役割 |
|----------|------|
| `.github/workflows/ci.yml` | CI のワークフロー本体（下の「ジョブ」） |
| `deny.toml` | `cargo-deny` の設定（脆弱性・ライセンス・重複・取得元） |
| `Cargo.toml`・`Cargo.lock` | `aws-sdk-cloudwatchlogs` の既定機能から `rustls` を外した（F1。古い rustls 0.21 / rustls-webpki 0.101 が依存から消え、`Cargo.lock` が 147 行減った） |
| `.github/dependabot.yml` | cargo・npm・GitHub Actions の更新 PR を週 1 回出す |

## きっかけ（トリガー）

- すべての `pull_request`。
- `main` への `push`（スクワッシュマージの後に `main` がまだ緑であることの確認）。
- 同じブランチで新しい push があれば古い実行は取り消す（`concurrency`、`cancel-in-progress`）。
- タグや定期実行はない（Q4：定期実行は入れない。配布の自動化は MVP の後。team.md Deployment）。
- 権限は `contents: read` だけ。AWS の認証情報は secrets に置かず、どのジョブも AWS に接続しない（project.md Forbidden）。

## ジョブ

| ジョブ | ランナー | 内容 | Build and Test で対応する確認 |
|--------|----------|------|-------------------------------|
| `rust-core` Rust library | ubuntu-latest | `cargo fmt --all --check` → `cargo clippy -p local-sights-core --all-targets` → `cargo test -p local-sights-core`（単体 305 件 + 結合 41 件） | B-4、B-5、TC-1〜TC-3、単体・統合テスト |
| `frontend` Front end | ubuntu-latest（Node 22、npm キャッシュ） | `npm ci` → `npx tsc --noEmit` → `npx eslint .` → `npx prettier --check .` → `npx vitest run`（171 件） → `npx vite build` → `npm audit --audit-level=high` | B-2、B-4、B-7、B-8 |
| `cargo-deny` | ubuntu-latest | `EmbarkStudios/cargo-deny-action@v2` で `cargo deny check`（機能の選び方は `deny.toml` の `[graph]`） | B-9 |
| `tauri-app` Tauri app | ubuntu-latest と macos-latest（matrix、`fail-fast: false`） | Linux では WebKitGTK・GTK の開発パッケージを apt で導入 → `npm ci` → `npm run build`（`frontendDist` を用意） → `cargo clippy --workspace --all-targets` → `cargo build -p local-sights`。`rust-core` と `frontend` が通ってから走る（`needs`） | B-3、B-6（Build and Test でこの環境では未確認だったもの） |

ジョブは `rust-core`・`frontend`・`cargo-deny` が並行し、`tauri-app` の 2 OS がそのあとに並行する。見込みの所要時間は、キャッシュが効いた状態で `rust-core` 3〜5 分、`frontend` 2 分、`cargo-deny` 1 分、`tauri-app` は Linux 5〜8 分・macOS 8〜12 分（初回はそれぞれ倍程度）。

## 使う Action と固定の方針

| Action | 用途 | 版 |
|--------|------|----|
| `actions/checkout` | チェックアウト | v4 |
| `dtolnay/rust-toolchain` | Rust 安定版（`rustfmt`・`clippy`） | `stable` |
| `Swatinem/rust-cache` | `~/.cargo` と `target/` のキャッシュ（OS ごとに鍵を分ける） | v2 |
| `actions/setup-node` | Node 22 と npm キャッシュ | v4 |
| `EmbarkStudios/cargo-deny-action` | `cargo deny check` | v2 |

Action の版はメジャー版で指定し、Dependabot（`github-actions`）が更新 PR を出す。Rust の版は `stable`（`Cargo.toml` の `rust-version = "1.85"`、edition 2024 が必要）。

## 環境変数

| 変数 | 値 | 理由 |
|------|----|------|
| `CARGO_INCREMENTAL` | `0` | CI ではインクリメンタルの中間物を残しても再利用されない。キャッシュを小さくする |
| `RUSTFLAGS` | 空 | clippy の警告をエラーにしない（team.md Code Style）。deny レベルの lint は clippy 自身がエラーにする |
| `CARGO_TERM_COLOR` | `always` | ログの読みやすさ |

AWS 関連の環境変数・secrets は置かない。

## `deny.toml` の要点（Q3）

- `[graph]`：ワークスペースが実際にビルドする機能で検査する（`all-features = false`）。
- `advisories`：脆弱性・保守停止・yanked をすべて失敗にする。`ignore` は RUSTSEC-2024-0370（`proc-macro-error` の保守停止。Tauri → gtk → glib-macros 経由で、こちらからは外せない。脆弱性ではない）の 1 件だけで、理由を添えた（F1）。
- `licenses`：許可リストは MIT、Apache-2.0、Apache-2.0 WITH LLVM-exception、BSD-3-Clause、ISC、Unicode-3.0、Zlib、MPL-2.0。現在の 507 クレートのライセンス式はすべてこの許可リストで通る（`OR` の式はどれか 1 つが許可されていればよい。質問票 Q3 の一覧に LLVM-exception を足したのは、`Apache-2.0 WITH LLVM-exception` 単独のクレートが 1 つあるため。BSD-2-Clause は依存に現れず、`cargo-deny` が「使われていない許可」と警告するため外した）。
- `bans`：同じクレートの複数の版は警告のみ。ワイルドカードの版指定は失敗（ワークスペース内のパス依存は除く：`allow-wildcard-paths = true`）。
- `sources`：crates.io だけ。git 依存は失敗。

## Dependabot（Q4）

cargo・npm・GitHub Actions を週 1 回。同時に開く PR は各 5 件まで。AWS SDK のクレート、Tauri のクレートと npm パッケージ、テスト関連の npm パッケージはそれぞれまとめて 1 つの PR にする。定期実行の検査は入れない（Q4 の B）。新しい勧告は、Dependabot の更新 PR の CI（`cargo-deny`・`npm audit`）で検出する。

## 手で行う設定（リポジトリの設定画面。ワークフローからは設定できない）

1. `main` のブランチ保護（ルールセット）：直接 push を禁止、PR 必須、必須のステータスチェックに `Rust library (fmt, clippy, test)`・`Front end (tsc, eslint, prettier, vitest, build)`・`cargo-deny`・`Tauri app (ubuntu-latest)`・`Tauri app (macos-latest)` を指定、スクワッシュマージだけを許可（team.md Way of Working）。
2. Secret scanning と push protection を有効にする（team.md Code Style、security-test-instructions.md）。
3. Dependabot の security updates を有効にする（`dependabot.yml` は version updates の設定。security updates は設定画面で有効にする）。

## 成果物（アーティファクト）

ビルド成果物はどこにも公開しない。配布はソースからのビルドだけ（team.md Deployment）。GitHub Releases・署名・公証は MVP の後に別のステージで決める。CI の成果物は保存しない（`vite build` の `dist/` も捨てる）。

## 手元での同じ確認

CI と同じコマンドは `construction/build-and-test/build-instructions.md` の「ビルドの検証」にある。`cargo deny check` は `cargo install cargo-deny --locked` のあとに実行できる。

## この環境での確認（2026-10-10）

| 確認 | 結果 |
|------|------|
| `ci.yml`・`dependabot.yml` の YAML、`deny.toml` の TOML の構文 | 正しく読める |
| `npx prettier --check .github` | 問題なし |
| `cargo deny check`（cargo-deny 0.20.2、変更前） | advisories FAILED（RUSTSEC-2026-0104、RUSTSEC-2024-0370）、bans FAILED（パス依存がワイルドカード扱い）、licenses・sources ok |
| `cargo deny check`（F1 の対処と `deny.toml` の調整のあと） | **advisories ok, bans ok, licenses ok, sources ok** |
| 依存の変更後の `cargo test -p local-sights-core`・`cargo fmt --check`・`cargo clippy -p local-sights-core --all-targets` | 346 件成功、差分なし、警告なし |
| ワークフローの実際の実行 | 未確認（GitHub に push して PR を作ったときに初めて走る。Tauri アプリの組み立ては Linux・macOS ともここが初回） |
