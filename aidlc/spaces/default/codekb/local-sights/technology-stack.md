# 技術スタック（Local Sights）

走査したコミット：`9d08cbf5c6d97820f0ec7f276c3ce6020c903eb5`（2026-10-12）。版は `Cargo.lock`／`package-lock.json` に固定された実際の版（宣言の範囲は [dependencies.md](dependencies.md)）。

## 言語と実行環境

| 項目 | 版・設定 | 根拠 |
|---|---|---|
| Rust | edition 2024、`rust-version = "1.85"`（ワークスペース共通）。CI は stable | `Cargo.toml`、`.github/workflows/ci.yml` |
| TypeScript | 6.0.3（`strict`、`noUncheckedIndexedAccess`、`noUnusedLocals` など、ES2022、`moduleResolution: bundler`） | `tsconfig.json` |
| Node.js | 22（CI とリリース） | `.github/workflows/*.yml` |
| 対象 OS | macOS（Apple Silicon）向けに配布。CI は Linux（WebKitGTK）でもビルドとテストを行う | team.md Deployment、`release.yml` |

## アプリの枠組み

| 名前 | 版 | 使い方 |
|---|---|---|
| Tauri | 2.12.3（`tauri-build` 2.7.2、`@tauri-apps/cli` 2.12.1、`@tauri-apps/api` 2.12.2） | デスクトップアプリ。コマンドを権限として宣言し、メインウィンドウにだけ許す。CSP は `self` と IPC だけ。`withGlobalTauri: false`。`custom-protocol` 機能で `dist/` を埋め込む |
| React | 19.3.0（`react-dom` 19.3.0） | 画面 |
| Cloudscape Design System | `@cloudscape-design/components` 3.0.1396、`design-tokens` 3.0.114、`global-styles` 1.0.71 | AWS マネジメントコンソール風の部品（Button、Alert、Modal、Table、DatePicker、TimeInput、StatusIndicator など）、OS の明暗に追従 |
| Vite | 8.3.4（`@vitejs/plugin-react` 6.1.2） | 開発サーバー（ポート 1420 固定）と本番のまとめ（`dist/`、ES2022） |

## 中核の主なライブラリ

| 名前 | 版 | 使い方 |
|---|---|---|
| aws-sdk-cloudwatchlogs | 1.156.0 | `GetLogEvents`、`DescribeLogStreams`、`DescribeLogGroups`。既定機能から `rustls`（古い rustls 0.21 系、RUSTSEC-2026-0104）を外し、`default-https-client` と `rt-tokio` を使う |
| aws-config | 1.12.0 | `BehaviorVersion::latest()` の既定チェーンで認証と接続先を解決（再試行・時間切れの設定は明示していない） |
| aws-credential-types | 1.3.0 | 認証情報・トークン取得の失敗の判定 |
| tokio | 1.53.2 | 非同期実行（Tauri の実行環境上）、`time`（再試行の待ち）、`sync::watch`（中断信号）、`spawn_blocking`。テストは `test-util` で時計を止める |
| chrono 0.4.45 / chrono-tz 0.10.4 / iana-time-zone 0.1.65 | — | 日時の読み書き、IANA のゾーン（夏時間）、OS のゾーン名 |
| serde 1.0.229 / serde_json 1.0.152 | — | 画面への受け渡し（camelCase）、キャッシュファイル（JSON Lines）、設定ファイル |
| sha2 | 0.11.0 | キャッシュのファイル名（キーの SHA-256） |
| regex | 1.13.1 | 秘密情報の伏せ字 |
| thiserror | 2.0.21 | エラー型 |
| uuid | 1.28.0 | セッション ID、一覧 ID など |
| fastrand | 2.5.0 | 再試行の待ちの揺らぎ |
| home | 0.5.12 | `~/.aws/` の場所 |
| libc | 0.2.190（アプリ側） | AWS CLI のプロセスグループへのシグナル |

## 開発・テストの道具

| 名前 | 版 | 使い方 |
|---|---|---|
| Rust 標準のテスト、`#[tokio::test]` | — | 中核とアプリのテスト。`start_paused = true` で待ちを実時間なしに検証 |
| tempfile | 3.27.0 | キャッシュ・設定・偽の CLI のテスト用フォルダ |
| aws-smithy-runtime-api 1.19.0 / aws-smithy-types 1.8.1 | — | SDK エラーをテストで作る（開発時のみ） |
| Vitest | 5.0.3 | 画面とスクリプトのテスト（jsdom 30.1.2、globals、`src/test/setup.ts`） |
| Testing Library | `@testing-library/react` 16.3.3、`dom` 10.4.2、`jest-dom` 7.0.1、`user-event` 14.6.7 | 部品のテスト |
| ESLint | 10.12.0（`@eslint/js` 10.0.1、typescript-eslint 8.71.1、react-hooks 7.1.1、react-refresh 0.5.7） | 画面の lint |
| Prettier | 3.9.10 | 画面の整形（幅 100、セミコロンあり、ダブルクォート、末尾カンマ all） |
| rustfmt / clippy | stable | Rust の整形と lint（警告はエラーにしない） |
| cargo-deny | `EmbarkStudios/cargo-deny-action` v2.1.1 | 脆弱性、ライセンス、重複、出所の検査（`deny.toml`） |
| `@resvg/resvg-js` | 2.6.2 | アイコンの PNG 生成 |

## CI/CD と配布

- GitHub Actions：`ci.yml`（PR と `main`）、`release.yml`（`v*` タグ）、`release-dry-run.yml`（手動）。外部の Action はコミットのハッシュで固定（`actions/checkout` v7.0.1、`actions/setup-node` v7.1.0、`dtolnay/rust-toolchain`、`Swatinem/rust-cache` v2.9.2、`actions/upload-artifact` v7.0.2）。権限は既定で `contents: read`、公開ジョブだけ `contents: write`。
- 配布物：未署名・未公証の `.dmg`（aarch64）。GitHub Releases に置く。
- Dependabot：cargo、npm、github-actions を毎週（AWS SDK、Tauri、テスト系をまとめる）。

## 実行時の外部とのつながり

- AWS CloudWatch Logs（HTTPS、SDK 既定の HTTPS クライアント）。
- AWS CLI（SSO ログインのときだけ、決まった 2 か所から起動）。
- OS のフォルダ：設定 `~/Library/Application Support/dev.local-sights.app/`、キャッシュ `~/Library/Caches/dev.local-sights.app/log-cache`（macOS）。
- 送信は他に無い（テレメトリやクラッシュレポートなし）。
