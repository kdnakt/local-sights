# Code Quality Assessment

> リポジトリ `local-sights` のテスト・リント・CI/CD・文書・技術的負債の評価。2026-10-12 の全体スキャン（コミット `9d08cbf`）に基づく。数値はスキャン時の集計。

## Summary

| 観点 | 評価 | 根拠 |
|---|---|---|
| テストの存在 | 良 | Rust 421 件（ユニット + 統合 5 本、`src-tauri` の CLI テスト 11 件）、Vitest 282 件（24 ファイル）。AWS は偽ゲートウェイ、タイムゾーンは固定、CLI は偽スクリプト |
| エラー処理 | 良 | テスト外の `unwrap()` / `expect()` は 0 件。`Result` と文言キーで伝播。Mutex の毒化は回復。SDK エラーは許可リストの安全な詳細に縮約 |
| 命名の明瞭さ | 良 | 言語の慣用に沿い、モジュール名が責務を表す（`planner`、`listing`、`classify`、`plan`） |
| 関数の大きさ | 可 | 小さな純粋関数が多いが、`session.rs` と `lib.rs` に長い関数が集まる（未精読） |
| 重複 | 可 | `release.yml` と `release-dry-run.yml` のビルドステップが同文。`LogTable` と `LogTableCloudscape` の並行実装 |
| 不要コード | 良 | `TODO` / `FIXME` / `HACK` は 0 件。`eslint-disable` 0 件、`#[allow]` 1 件（テスト、理由付き） |
| 文書 | 良 | `#![warn(missing_docs)]`、各モジュールに BR/FR 番号付きの設計説明、37 KB の README、`docs/install.md`、`docs/releasing.md` |
| CI/CD | 良 | 4 ジョブの CI + リリース + ドライラン。すべての action を SHA 固定。秘密情報ゼロ |
| セキュリティ姿勢 | 良 | 読み取り 3 API のみ、秘密のマスク、CSP、Tauri の能力を自アプリのコマンドと `core:event:default` に限定、`cargo-deny` + `npm audit` |

## Tests

### Rust（`cargo test`）
- **場所**: 各モジュールの `#[cfg(test)] mod tests`、`crates/local-sights-core/tests/`（`u1_fetch_flow`、`u2_log_group_listing`、`u3_fetch_flow`、`u5_filter_flow`、`u6_cache_flow` + `support/fake_gateway.rs`、`support/recording_sink.rs`）、`src-tauri/src/sso_cli.rs` の `mod tests`。
- **件数**: `#[test]` / `#[tokio::test]` 421 件。うち `#[ignore]` 5 件は 100 万件規模の性能計測（`timeline::`、`filter::`、`log_view::`。`--release --ignored` で手元実行）。
- **方式**: AWS は `CloudWatchLogsGateway` の偽物（台本応答 + 要求記録）。待ちは tokio の一時停止クロック（`test-util`）で実時間を使わない。タイムゾーンは `TimeZoneContext` で固定。環境変数を触るテストは `ENV_LOCK` で直列化（edition 2024 の `unsafe set_var` に SAFETY コメント）。CLI は一時フォルダの偽スクリプト（SIGTERM を無視するものも含む）。
- **CI**: `cargo test -p local-sights-core`（`rust-core`、`release.yml` の `test`）、`cargo test -p local-sights`（`tauri-app`、ubuntu + macos）。
- **カバレッジ設定**: 無し（team.md: 数値の下限を設けない。代わりにライブラリの公開関数には必ずテスト）。

### 画面と スクリプト（Vitest）
- **場所**: `src/**/*.test.ts(x)` 23 ファイル、`scripts/release/versions.test.ts`。`vitest.config.ts`: `environment: jsdom`、`globals: true`、`setupFiles: src/test/setup.ts`、CSS は 2 ファイルだけ `?raw` で読む。
- **件数**: `it` / `test` 282 件。`src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。`src/commandRegistration.test.ts` が 4 か所のコマンド登録の整合を検査。`versions.test.ts` は 12 ケース（タグ形式、4 ファイルの読み取り、不一致、秘密らしき文字列を漏らさない）。
- **CI**: `npx vitest run`（`frontend`、`release.yml` の `test`）。

### 手動確認
- `README.md` に作業単位ごとの手動 GUI チェックリスト（U1〜U7、console-ui-polish、sso-login）と性能計測の手順。`fetch_check` の例で実 AWS を手元確認。
- 本インテントに関係する macOS の Gatekeeper の挙動は自動テストでは確かめられず、手元の macOS 15 で `.dmg` を開いて目視する以外に検証手段が無い。現状の `docs/releasing.md` にその手順は無い。

## Linting and formatting

| 対象 | ツール | 設定 | CI での扱い |
|---|---|---|---|
| Rust 整形 | rustfmt | 既定 | `cargo fmt --all --check` 必須 |
| Rust リント | clippy | `RUSTFLAGS: ""`（`-D warnings` なし） | `cargo clippy -p local-sights-core --all-targets`（ubuntu）、`cargo clippy --workspace --all-targets`（ubuntu + macos）。警告は失敗にしない。deny レベルのみブロック |
| 抑制 | — | `#[allow(clippy::await_holding_lock)]` 1 件（`gateway/aws.rs:462`、テスト、理由付き） | |
| TS 型検査 | tsc | `strict`、`noUnusedLocals`、`noUnusedParameters`、`noFallthroughCasesInSwitch`、`noUncheckedIndexedAccess` | `npx tsc --noEmit` 必須 |
| TS リント | eslint 10 + typescript-eslint | `eslint.config.js`（flat）。`src-tauri`、`crates`、`aidlc`、`.claude` を除外。`react-refresh/only-export-components` は warn | `npx eslint .` 必須 |
| 抑制 | — | `eslint-disable` 0 件、`@ts-expect-error` 1 件（`useColorScheme.test.ts`、理由付き） | |
| 整形 | prettier 3 | `printWidth 100`、`.prettierignore`（生成物・ロック・AI-DLC） | `npx prettier --check .` 必須 |
| 依存監査 | cargo-deny、npm audit | `deny.toml`（advisories / licenses / bans / sources）、`--audit-level=high` | 必須 |
| 版の一致 | `npm run check:version` | 4 ファイル | 必須 |

## CI/CD

### `.github/workflows/ci.yml`（PR と `main` push）
- `rust-core`: fmt、clippy、`cargo test -p local-sights-core`。
- `frontend`: `npm ci`、check:version、tsc、eslint、prettier、vitest、`vite build`、`npm audit --audit-level=high`。
- `cargo-deny`: `cargo deny check`。
- `tauri-app`（`needs: rust-core, frontend`、ubuntu + macos 行列、`fail-fast: false`）: Linux は WebKitGTK/GTK を apt で入れ、`npm run build`、workspace clippy、`cargo build -p local-sights`、`cargo test -p local-sights`。
- `permissions: contents: read`、`concurrency` でブランチごとに古い実行を取り消し。AWS 認証情報は無い。

### `.github/workflows/release.yml`（`v*` タグ）
- `verify`（ubuntu）: `npm run check:version -- --tag "$TAG"`、タグのコミットが `origin/main` の祖先か。
- `test`（ubuntu、`needs: verify`）: `cargo test -p local-sights-core`、`npx vitest run`。
- `build-and-publish`（macos-latest、`needs: [verify, test]`、`permissions: contents: write`）: `uname -m` が `arm64` か → `npm ci` → **`npm run tauri build -- --bundles dmg`（環境変数なし、ステップ名 `build the .dmg (unsigned)`）** → `target/release/bundle/dmg/*.dmg` がちょうど 1 つで名前に `aarch64` を含むか → 空白を除いた `Local-Sights_<ver>_aarch64.dmg` にコピー → `gh release view` で存在すれば `gh release upload --clobber`、無ければ `gh release create --verify-tag --title "Local Sights $TAG" --notes "$NOTES" --generate-notes`。`NOTES` に「Unsigned macOS (Apple Silicon) build: see docs/install.md.」。
- `concurrency: cancel-in-progress: false`（Release を途中で壊さない）。タイムアウト 10 / 20 / 40 分。

### `.github/workflows/release-dry-run.yml`（手動）
- 同じ `.dmg` ビルドと確認を行い、`actions/upload-artifact` で 7 日保持。公開しない。ビルドステップは `release.yml` と同文。

### Dependabot
- cargo（`aws-*`、`tauri*` グループ）、npm（`@tauri-apps/*`、testing グループ）、github-actions。週次、各 5 件まで。

## Documentation

- `README.md`（37 KB、英語）: 概要、インストールへの導線、配置表、前提、起動、SSO、手動 GUI チェックリスト、性能計測、`fetch_check`、アイコン、テスト、リリース、秘密情報、ライセンス。
- `docs/install.md`（英語）: 配布版のインストール、会社の規則の確認、ダウンロード、インストール、**初回起動の警告**（macOS 15: Done → システム設定 → Open Anyway; 14 以前: Control クリック → Open; 「壊れている」なら `xattr -dr com.apple.quarantine`）、版の確認、更新、戻し方。
- `docs/releasing.md`（英語）: 版上げの 4 ファイル、手順、失敗時、再実行。
- コード内: Rust は `#![warn(missing_docs)]` を満たし、各モジュール先頭に作業単位・BR/FR・レビュー番号を引いた設計説明。TS も JSDoc で同じ番号を引く。ワークフローと `deny.toml` にも意図のコメント。
- ADR（`ADR-001`、`ADR-007`）はコードから参照されるが、リポジトリ内の文書としては AI-DLC の記録側（`aidlc/`）にあり、プロジェクトの `docs/` には無い。

## Security posture

- 読み取り 3 API のみ（trait の型で保証）。認証は SDK に委ね、アプリは認証情報を読まない。
- エラーの安全な詳細は許可リストのフィールドだけから組み、アクセスキー ID らしき文字列をマスク。CLI の出力は URL とコードだけを取り出して捨てる。
- Tauri の能力は `main` ウィンドウに `core:event:default` と自アプリの 25 コマンドだけ。CSP は `default-src 'self'`、`connect-src ipc: http://ipc.localhost`。
- 設定・キャッシュのファイルは所有者のみ、原子的な書き込み。ログファイルを書かない。テレメトリ無し。
- CI は秘密情報ゼロ（`GITHUB_TOKEN` のみ）。`contents: write` は公開ジョブだけ。
- 配布物は未署名・未公証。macOS 15 Sequoia では未署名の `.app` が「壊れているため開けません」と表示されうる（本インテントの動機）。アドホック署名（`APPLE_SIGNING_IDENTITY="-"`）は証明書も Apple のサービスも要らず、CI の秘密情報ゼロの姿勢を保ったまま追加できる。

## Technical debt

| 信号 | 場所 | 重さ | 備考 |
|---|---|---|---|
| 大きなファイル | `crates/local-sights-core/src/session.rs` 4,663 行（公開項目 114 件）、`src-tauri/src/lib.rs` 1,449 行、`coordinator.rs` 1,441 行、`cache/plan.rs` 1,134 行、`cache/mod.rs` 1,130 行 | 中 | テスト同居のため実コードはこれより少ないが、`session.rs` は責務の集中が目立つ |
| ビルドステップの重複 | `release.yml:103-104` と `release-dry-run.yml:45-46` | 小 | 署名の環境変数を足すなら両方。片方だけだとドライランで署名結果を確かめられない |
| 設定と運用のずれ | `tauri.conf.json` の `bundle.targets: "all"` と `--bundles dmg` | 小 | 動作には影響しない |
| 並行実装 | `src/components/LogTable.tsx` と `LogTableCloudscape.tsx`、`src/dev/perfRows.ts` の `api.ts` 分岐 | 小 | 計測後に片付ける余地 |
| 既知の制約 | Dock メニューの「終了」は取得中でも確認なしに終了（`lib.rs` 文書、review R-01） | 小 | tao の `applicationWillTerminate` の制約 |
| advisory 除外 | `deny.toml` の `RUSTSEC-2024-0370`（`proc-macro-error`、Linux のみ） | 小 | Tauri 側の更新待ち |
| 重複クレート | `thiserror` 1 / 2、`sha2` 0.10 / 0.11 | 小 | 警告扱い |
| `unsafe` | core のテスト 4 箇所（`env::set_var`、SAFETY 付き）、`sso_cli.rs` 3 箇所（`killpg` / `kill`） | 小 | 必要最小限 |
| 文言の前提 | `docs/install.md:21,54-85`、`docs/releasing.md:9`、`README.md:38-40`、`release.yml:14-16,103,132` が「not signed or notarized」を前提 | 中（本インテント） | アドホック署名後は「Apple の署名・公証はしない（アドホック署名のみ）」に揃え、初回起動の手順を実際のダイアログに合わせる |
| 自動検証不能な配布挙動 | Gatekeeper のダイアログ | 中（本インテント） | `docs/releasing.md` の Release dry run に手元確認の手順を足す余地 |
| リリース実績 | タグ 0 件（ローカル） | — | `v0.1.0` 未リリースか未取得。`release.yml` と `docs/install.md` は #14 以降未変更 |
