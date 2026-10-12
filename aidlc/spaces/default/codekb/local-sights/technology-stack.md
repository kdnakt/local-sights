# Technology Stack

> リポジトリ `local-sights` の言語・フレームワーク・ライブラリと版。2026-10-12 の全体スキャン（コミット `9d08cbf`）に基づく。版は `Cargo.lock` / `package-lock.json` の解決済みの値（宣言は `Cargo.toml` / `package.json`）。外部依存の役割と内部依存は `dependencies.md` を参照。

## Languages

| 言語 | 版 | 使用箇所 |
|---|---|---|
| Rust | stable（最低 `rust-version = "1.85"`、edition 2024） | `crates/local-sights-core/`、`src-tauri/` |
| TypeScript | 6.0.3（`strict`、`noUncheckedIndexedAccess`、`moduleResolution: bundler`、`jsx: react-jsx`、`noEmit`） | `src/`、`scripts/release/versions.ts`（消去可能構文のみ）、設定ファイル |
| JavaScript（ESM） | Node.js 22（`check:version` は 22.18+ の型除去 import が必要） | `scripts/*.mjs`、`eslint.config.js` |
| CSS | — | `src/styles.css`（Cloudscape トークンの CSS 変数を利用） |
| YAML / TOML / JSON | — | GitHub Actions、Cargo、cargo-deny、Tauri 設定 |
| Bash | — | ワークフローのステップ内 |

## Desktop framework

| 技術 | 版 | 役割 |
|---|---|---|
| Tauri | 2.12.3（crate `tauri`） | OS の WebView 上で画面を描き、Rust のコマンド／イベントで IPC。`generate_context!`、メニュー、`RunEvent` |
| tauri-build | 2.7.2 | `build.rs` でコンテキストと権限マニフェストを生成 |
| @tauri-apps/cli | 2.12.1（npm） | `tauri dev` / `tauri build` / `tauri icon`。macOS 署名は環境変数 `APPLE_SIGNING_IDENTITY` を読む（未設定なら署名せず、`"-"` でアドホック） |
| @tauri-apps/api | 2.12.2 | 画面側の `invoke` / `listen` |
| 対象 OS | macOS（Apple Silicon、`aarch64-apple-darwin`） | 配布対象。CI は ubuntu でもビルド（WebKitGTK）。`sso_cli.rs` は Unix 専用 |

## Rust libraries（`crates/local-sights-core/`、`src-tauri/`）

| クレート | 版 | 役割 |
|---|---|---|
| aws-sdk-cloudwatchlogs | 1.156.0 | CloudWatch Logs クライアント。`default-features = false` + `default-https-client`、`rt-tokio`（rustls 0.21 系の RUSTSEC-2026-0104 を避ける） |
| aws-config | 1.12.0 | プロファイル・リージョンの解決、既定の認証チェーン |
| aws-credential-types | 1.3.0 | 認証情報型 |
| aws-smithy-runtime-api / aws-smithy-types | 1.19.0 / 1.8.1 | dev 依存（テストで SDK エラーを組み立てる） |
| tokio | 1.53.2 | 非同期ランタイム（`rt-multi-thread`、`macros`、`sync`、core は `time`、テストは `test-util`） |
| chrono / chrono-tz / iana-time-zone | 0.4.45 / 0.10.4 / 0.1.65 | 時刻、IANA タイムゾーン（DST）、OS のゾーン名 |
| serde / serde_json | 1.0.229 / 1.0.152 | `SessionView` の直列化、キャッシュの JSON Lines、設定ファイル |
| thiserror | 2.0.21 | エラー型（1.0.69 も推移依存で共存） |
| regex | 1.13.1 | 秘密らしき文字列のマスク、入力解析 |
| sha2 | 0.11.0 | キャッシュキーのハッシュ（0.10.9 も推移依存で共存） |
| uuid | 1.28.0（v4） | セッション ID |
| home | 0.5.12 | `~/.aws` の場所 |
| fastrand | 2.5.0 | 再試行待ちのランダムな広がり |
| libc | 0.2.190 | `killpg` / `kill`（`src-tauri` の SSO CLI 停止） |
| tempfile | 3.27.0 | dev 依存（テストの一時フォルダ） |

## Front-end libraries（`src/`）

| パッケージ | 版 | 役割 |
|---|---|---|
| react / react-dom | 19.3.0 | 画面 |
| @cloudscape-design/components | 3.0.1396 | AWS マネジメントコンソールの部品（TopNavigation、Table、DatePicker、Modal、Alert など）。i18n メッセージ en/ja |
| @cloudscape-design/design-tokens | 3.0.114 | 自前描画部分の色・間隔 |
| @cloudscape-design/global-styles | 1.0.71 | ベーススタイル、ライト／ダーク |
| vite | 8.3.4 | 開発サーバー（port 1420 固定）とバンドル（`dist/`、target es2022） |
| @vitejs/plugin-react | 6.1.2 | React の変換 |

## Test, lint, format

| ツール | 版 | 役割 |
|---|---|---|
| `cargo test` | — | Rust のユニット・統合テスト（421 件、`#[ignore]` の性能計測 5 件） |
| vitest | 5.0.3 | 画面と `scripts/release` のテスト（282 件、24 ファイル、`environment: jsdom`、`globals: true`） |
| jsdom | 30.1.2 | DOM 環境 |
| @testing-library/react / dom / jest-dom / user-event | 16.3.3 / 10.4.2 / 7.0.1 / 14.6.7 | 部品テスト |
| rustfmt / clippy | stable | CI で `cargo fmt --all --check`、`cargo clippy ...`（`-D warnings` は付けない） |
| eslint | 10.12.0 | `eslint.config.js`（flat config） |
| typescript-eslint | 8.71.1 | TS ルール |
| @eslint/js | 10.0.1 | 推奨ルール |
| eslint-plugin-react-hooks / eslint-plugin-react-refresh | 7.1.1 / 0.5.7 | React のルール |
| globals | 17.13.0 | ブラウザのグローバル |
| prettier | 3.9.10 | `printWidth: 100`、`semi`、ダブルクォート、`trailingComma: all` |
| cargo-deny | v2.1.1（action） | 脆弱性・ライセンス・重複・ソースの検査（`deny.toml`） |
| npm audit | — | `--audit-level=high` |
| @resvg/resvg-js | 2.6.2 | アイコン描画（dev） |

## CI/CD and distribution

| 技術 | 版 | 役割 |
|---|---|---|
| GitHub Actions | — | `ci.yml`、`release.yml`、`release-dry-run.yml` |
| actions/checkout | v7.0.1（SHA 固定） | |
| actions/setup-node | v7.1.0 | Node 22、npm キャッシュ |
| dtolnay/rust-toolchain | stable（SHA 固定） | rustfmt、clippy |
| Swatinem/rust-cache | v2.9.2 | Cargo キャッシュ |
| EmbarkStudios/cargo-deny-action | v2.1.1 | |
| actions/upload-artifact | v7.0.2 | ドライランの `.dmg` |
| GitHub CLI `gh` | ランナー同梱 | Release の作成・差し替え |
| macOS ランナー | `macos-latest`（arm64 を実行時に検証） | `.dmg` のビルド |
| Dependabot | v2 設定 | cargo / npm / github-actions 週次 |
| GitHub Releases | — | `Local-Sights_<ver>_aarch64.dmg` の配布 |
| Apple 署名・公証 | **未使用** | Developer Program 不参加。`tauri.conf.json` に `bundle.macOS` 節なし、ワークフローに署名の環境変数なし。本インテントで CI にアドホック署名（`APPLE_SIGNING_IDENTITY="-"`）を加える |

## Runtime dependencies on the user's Mac

| もの | 必要性 |
|---|---|
| macOS（Apple Silicon） | 配布対象 |
| `~/.aws/config`（プロファイルと既定リージョン） | 接続先の選択 |
| AWS CLI v2（`/opt/homebrew/bin/aws` または `/usr/local/bin/aws`） | 任意。アプリからの SSO サインインに使う |
| IAM 権限 | `logs:DescribeLogGroups`、`logs:DescribeLogStreams`、`logs:GetLogEvents` |
