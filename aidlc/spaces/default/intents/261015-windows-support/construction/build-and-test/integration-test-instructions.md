# Integration Test Instructions — 261015-windows-support

Test Strategy は `Standard`。この作業の境目は「OS に触れる層」と「CI・リリースのワークフロー」にある。自動テストで確かめられる境目はそれぞれの OS のランナーで走らせ、GUI と実際の AWS が要るものは開発者の手元で確かめる。

## テストの道具と設定

- Rust：`cargo test`。偽の AWS CLI は、テストが一時フォルダに書くスクリプト（Unix は `/bin/sh`、Windows は `.cmd` と、子プロセスとして起こす PowerShell）。一度に動く偽の CLI は 1 つ（`SERIAL` のロック）。
- TypeScript：Vitest。`scripts/release/signing.test.ts` は実際の `.github/workflows/release.yml`・`release-dry-run.yml`・`docs/*.md`・`README.md` を読む。
- OS ごとに分かれるテストは `#[cfg(unix)]` / `#[cfg(windows)]` で、その OS のランナーだけで走る（team.md Testing Posture）。
- 実際の AWS には接続しない（project.md の Forbidden）。

## 境目と、それを確かめるテスト

| 境目 | 要件 | テスト | 動く OS |
|------|------|--------|---------|
| AWS CLI の探索（Program Files → PATH、絶対パスで起動） | FR4.1、R-05、NFR5 | `src-tauri/src/sso_cli/mod.rs` の `candidate_tests`（Windows 3 件、Unix 1 件）と `locate_cli` のテスト（Windows 2 件、Unix 2 件） | それぞれ |
| CLI の起動と停止（窓なし、ジョブ オブジェクト、子まで止める） | FR4.2〜FR4.4、NFR2 | `src-tauri/src/sso_cli/mod.rs` の `windows_tests`（6 件：成功、失敗の終了コード、起動できない、時間切れ、中断、アプリ終了。偽の `aws.cmd` が PowerShell の子を起こし、`tasklist` で残っていないことを 2 秒まで待って確かめる） | Windows |
| CLI の起動と停止（Unix、動きは変えない） | NFR1 | `unix_tests`（12 件） | Linux・macOS |
| キャッシュのフォルダ（Windows は OS の既定の権限） | FR3.3、NFR3 | `crates/local-sights-core/src/cache/mod.rs` の `#[cfg(windows)]` のテスト（1 件）と、既存のキャッシュのテスト（Windows で「フォルダのはずの場所にファイル」が `Unreadable` になること） | Windows／すべて |
| ブラウザーを開く（URL の許可の検査 → OS の呼び出し） | FR4.5、FR3.6 | `src-tauri/src/lib.rs` の `open_sso_url` のテスト（許可の検査。既存）。OS の呼び出し自体は手元の確認 | すべて／手元 |
| Tauri のコマンドと権限の対応 | NFR1 | `cargo test -p local-sights`（既存） | すべて |
| リリースのワークフローの並びと署名の検証の位置 | FR2.3、FR2.5、FR2.6 | `scripts/release/signing.test.ts`（31 件：合成のワークフロー + 実際の `release.yml` の `build-macos` → `build-windows` → `publish`、`needs`、`--bundles nsis`） | Linux |
| 手順書の署名の言い回し（`unsigned` と読まれる文がない） | FR5.1 | `signing.test.ts` の repository テスト（`docs/install.md`、`docs/releasing.md`、`README.md`、`release.yml`） | Linux |
| 既存の取得・絞り込み・一覧・キャッシュの流れ（回帰） | NFR1 | `crates/local-sights-core/tests/*.rs`、`src/App.test.tsx` | すべて |

## 走らせ方

```sh
cargo test -p local-sights sso_cli                 # CLI の探索と起動・停止（この OS 向けのテスト）
cargo test -p local-sights-core cache::            # キャッシュ
cargo test -p local-sights                          # アプリ（Tauri のコマンドと権限、URL の検査）
npx vitest run scripts/release/signing.test.ts     # リリースのワークフローと手順書
cargo test --workspace && npx vitest run           # まとめて
```

Windows のテストは、Linux と macOS ではコンパイルされない。CI の `Tauri app (windows-latest)`（`cargo test --workspace` を `windows-latest` で実行）の結果で確かめる。手元の Windows 11 の PC に Rust と Build Tools があれば、同じ `cargo test --workspace` で走る。

## カバレッジの目標

- ライブラリの行カバレッジ 80% 以上：`cargo llvm-cov -p local-sights-core --fail-under-lines 80`（Linux）。
- 画面の行カバレッジ 80% 以上：`npm run test:coverage`（Linux）。
- `cfg(windows)` のコードは Linux の計測の対象外（了解済みの制約。要件分析 R-03）。下限は下げない（team.md Testing Posture）。

## 手元の確認（GUI と実際の AWS）

自動テストで確かめられない境目。開発者本人の PC で、本人の AWS 認証情報を使って行う。結果は合否だけを記録し、プロファイル名・ロググループ名・数値は記録しない。

| 確認 | 要件 | OS | やり方 |
|------|------|----|--------|
| インストーラで入れて起動し、プロファイルとリージョンを選び、取得して表示する | FR3.1、NFR1、NFR4 | Windows 11（x64）、macOS | Release dry run の成果物を入れる。README の「Manual GUI check」の Windows の項目 |
| macOS で前と同じに動く | NFR1 | macOS | 同じ dry run の `.dmg` で取得・表示・SSO ログイン |
| SSO ログイン：窓が出ない、ブラウザーが開く、中断・時間切れ・終了で `aws.exe` が残らない | FR4.2〜FR4.5 | Windows | README の Windows の項目（タスク マネージャーで確かめる） |
| 利用者のフォルダに空白や日本語がある PC での設定・キャッシュ・AWS の設定ファイル | NFR5 | Windows | そのようなアカウントで起動して取得する（開発者のアカウント名が該当すればそれで足りる） |
| ナレーターでの読み上げ（経過時間の秒は読まない、警告は 1 回） | NFR6 | Windows | README の Windows の項目（Narrator） |
| ダークモードの切り替え、1024×640 で重ならない | FR3.1 | Windows | README の Windows の項目 |

## テストのデータ

- 偽の CLI の出力は URL とコードの例（`https://device.sso.us-east-1.amazonaws.com/`、`ABCD-EFGH`）だけで、実在の認証情報は使わない。
- 一時フォルダ（`tempfile::tempdir`）に作り、テストの終わりに消える。
- `signing.test.ts` の合成のワークフローは短い断片で、実際の `release.yml` は repository テストだけが読む。
