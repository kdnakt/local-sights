# 技術の構成（Local Sights）

> 2026-10-20、コミット `3208713`。版は `Cargo.lock` と `package-lock.json`（lockfileVersion 3）に固定されているものを書く。宣言の範囲は `Cargo.toml` の `[workspace.dependencies]` と `package.json`。依存の間の関係と管理の仕方は `dependencies.md`。

## 言語と実行環境

| 層 | 言語・環境 | 根拠 |
|---|---|---|
| 中核・アプリ | Rust、edition 2024、`rust-version = "1.85"`、CI は `dtolnay/rust-toolchain` の stable | `Cargo.toml:6-9`、`.github/workflows/ci.yml:31` |
| 画面 | TypeScript 6.0.3（`tsc --noEmit` で型検査）、React 19.3.0、ES modules（`"type": "module"`） | `package.json`、`tsconfig.json` |
| スクリプト | Node.js 22.18 以上（`versions.ts` を型除去で直接 import）、bash（`insights-export.sh`） | `scripts/release/check-version.mjs:12-13` |
| 配布 | macOS（Apple Silicon、`.dmg`）、Windows 11 x64（NSIS `-setup.exe`、`installMode: currentUser`、WebView2 は `downloadBootstrapper`） | `src-tauri/tauri.conf.json`、`release.yml` |
| 版 | 0.2.0（`Cargo.toml`、`Cargo.lock` の 2 クレート、`package.json`、`tauri.conf.json` でそろえ、`check-version.mjs` が検査） | `docs/releasing.md:28-48` |

## 枠組みとライブラリ（Rust）

| crate | 版 | 使う場所と目的 |
|---|---|---|
| tauri / tauri-build | 2.12.3 / 2.7.2 | アプリの枠組み。`custom-protocol` feature で画面を同梱 |
| aws-config | 1.12.0 | プロファイルとリージョンの解決（認証情報はアプリが読まない） |
| aws-sdk-cloudwatchlogs | 1.156.0 | 読み取り 3 操作。default-features off、`rustls` だけ |
| aws-credential-types、aws-smithy-runtime-api、aws-smithy-types | （SDK と同系） | 型と、テストでの SDK エラーの組み立て（dev） |
| rustls | 0.23.46 | TLS（SDK 経由） |
| tokio | 1.53.3 | 非同期の実行（rt-multi-thread, macros, sync, time；dev で test-util の止まった時計） |
| futures-util | 0.3.34 | `FuturesUnordered` で最大 8 本の同時読み（default-features off、std） |
| chrono / chrono-tz / iana-time-zone | 0.4.45 / 0.10.4 / 0.1.65 | 時刻の読み書き、IANA のゾーン、OS のゾーンの名前 |
| regex | 1.13.1 | クエリの `like /…/` と `=~`、SSO の出力の抜き出し |
| serde / serde_json | 1.0.229 / 1.0.154 | IPC の JSON、キャッシュの JSON Lines、設定、メッセージの JSON 読み |
| sha2 | 0.11.1 | キャッシュの鍵のファイル名 |
| thiserror | 2.0.21 | エラー型 |
| uuid | 1.28.0 | 一時ファイルの名前など（v4） |
| fastrand | 2.5.0 | 再試行のジッター |
| home | 0.5.12 | `~/.aws` の場所 |
| tempfile | 3.27.0（dev） | キャッシュと偽の CLI のテスト |
| libc | 0.2.190（`cfg(unix)`、アプリ） | `killpg` でプロセスグループを止める |
| windows-sys | 0.61.2（`cfg(windows)`、アプリ） | ジョブオブジェクトと `ShellExecuteW` |

## 枠組みとライブラリ（画面）

| パッケージ | 版 | 目的 |
|---|---|---|
| react / react-dom | 19.3.0 | 画面 |
| @cloudscape-design/components | 3.0.1400 | 部品（TopNavigation、SideNavigation、Table、Multiselect、CodeEditor、Modal、Alert、DatePicker、TimeInput、ButtonDropdown など） |
| @cloudscape-design/design-tokens / global-styles | 3.0.114 / 1.0.71 | 自前で描く部分の色、明暗モード |
| ace-builds | 1.44.0 | CodeEditor のエディタ本体（アプリ内から読み込み、ネットワークなし） |
| @tauri-apps/api | 2.12.4 | `invoke` と `listen` |
| vite / @vitejs/plugin-react | 8.3.4 / 6.1.2 | ビルドと開発サーバー（1420 番） |
| typescript / typescript-eslint / eslint / eslint-plugin-react-hooks / eslint-plugin-react-refresh / globals | 6.0.3 / 8.71.2 / 10.12.0 / 7.1.1 / 0.5.7 / 17.13.0 | 型検査と lint |
| prettier | 3.9.10 | 整形（printWidth 100、semi、double quote、trailingComma all） |
| vitest / @vitest/coverage-v8 / jsdom | 5.0.4 / 5.0.4 / 30.1.2 | テストとカバレッジ（行 80%） |
| @testing-library/react / dom / jest-dom / user-event | 16.3.3 / 10.4.2 / 7.0.1 / 14.6.8 | 画面のテスト |
| @tauri-apps/cli | 2.12.3 | `npm run tauri dev` / `build` |
| @resvg/resvg-js | 2.6.2 | アイコンの SVG → PNG |

## ビルド・テスト・配布の道具

- **ビルド**：Cargo（ワークスペース）、Vite（`npm run build` = `tsc --noEmit && vite build`）、Tauri CLI（`beforeBuildCommand: npm run build`、`frontendDist: ../dist`）。
- **テスト**：`cargo test --workspace`、`cargo llvm-cov -p local-sights-core --fail-under-lines 80`（cargo-llvm-cov 0.9.1）、`cargo test … --doc`、`cargo test … --examples`、`npm run test:coverage`（Vitest、行 80%）。
- **検査**：`cargo fmt --all --check`、`cargo clippy`（警告は警告のまま）、cargo-deny（`EmbarkStudios/cargo-deny-action` v2.1.1）、`npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .`、`npm audit --audit-level=high`、`npm run check:version`。
- **CI**：GitHub Actions（`actions/checkout` v7.0.1、`actions/setup-node` v7.1.0、`Swatinem/rust-cache` v2.9.2、すべて SHA 固定）。Linux ランナーでカバレッジ、3 OS でアプリのビルドとテスト。Dependabot が cargo・npm・github-actions を週次でまとめて上げる。
- **配布**：`release.yml` が `v*` タグで `.dmg`（アドホック署名 `APPLE_SIGNING_IDENTITY=-`、公証なし）と NSIS x64 を作り、`gh release create --verify-tag` で 1 つの Release に。`release-dry-run.yml` は手動で作成だけを試す。更新は手で入れ替え、自動更新はない。
- **改行**：`.gitattributes` で全テキストを LF に固定（Windows の checkout で整形の検査が一致するため）。

## 設定・データの置き場所

- 利用者の AWS 設定：`~/.aws/config`、`~/.aws/credentials`（読みだけ。`AWS_PROFILE` などの環境変数も見る）。
- アプリの設定とキャッシュ：Tauri の `app_config_dir` の下の `settings.json` と `log-cache/`（macOS は `~/Library` の下、Windows は `AppData` の下）。キャッシュは JSON Lines、unix では 0700/0600。
- 画面の記憶：`localStorage` にペインの幅と開閉（`local-sights.logGroupPaneWidthPercent`、`local-sights.logGroupPaneCollapsed`）。

## この作業（carryover-fixes）から見た技術の手持ち

5 項目はいずれも今の依存の範囲で直せる。リージョンの名前（5）は表をコードに持てばよく、新しい crate は要らない。検索できる選択欄は Cloudscape の `Select`（`filteringType="auto"`）が `ButtonDropdown` の代わりになる。英語の単複（4）は `translate` の置き換えに数の分岐を足すか、文言を数に依存しない形にする。
