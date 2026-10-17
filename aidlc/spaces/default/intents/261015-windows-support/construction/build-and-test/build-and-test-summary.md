# Build and Test Summary — 261015-windows-support

IB-16「Windows 対応」をまとめてビルドし、テストした結果。Part A（PR #23、`main` にマージ済み）と Part B（PR #24、枝 `claude/windows-support`、head `572f3af`）を合わせた状態が対象。

## ビルドの状態と前提

- ビルド：**成功**（`cargo build -p local-sights`、`npx tsc --noEmit`、`npx vite build`、`npm run check:version` = 0.2.0）。手順は `build-instructions.md`。
- 前提：Rust stable（1.97.0）、Node.js 22、cargo-llvm-cov 0.9.1、cargo-deny。自動テストは実際の AWS に接続しない。
- Windows 向けのコンパイルと NSIS のインストーラは CI（`Tauri app (windows-latest)`、PR #23 と #24 で成功）と Release dry run（run 38858473889、両 OS の成果物あり）で確かめた。

## テストの種類

Test Strategy は `Standard`。単体テストはステージ単位の `code-generation/unit-test-instructions.md`、このステージで足した指示書は次のとおり。

| 種類 | 指示書 | 中身 |
|------|--------|------|
| ビルド | `build-instructions.md` | 依存・ビルド・確かめ方・つまずき（Windows のリンカ、NSIS、ディスク） |
| 結合 | `integration-test-instructions.md` | OS の境界（CLI の探索と起動・停止、キャッシュ、ブラウザー）、リリースのワークフロー、手順書の検査、手元の確認（GUI と実際の AWS） |
| 速度 | `performance-test-instructions.md` | 新しい速さの目標はない。既存の速度測定の回帰のみ |
| 安全 | `security-test-instructions.md` | cargo-deny・npm audit・GitGuardian、main との差分の確認（API・依存・プロセス起動・secrets）、Windows の権限の記載、署名の言い回し |

## カバレッジの見込み

作業単位はないため、ライブラリ全体と画面全体で測る（CI と同じ、Linux）。`cfg(windows)` のコードは Linux の計測の対象外（了解済みの制約。要件分析 R-03）。

| 対象 | 下限 | 今回 |
|------|------|------|
| ライブラリ `crates/local-sights-core` | 行 80% | 97.67% |
| 画面 `src/` | 行 80% | 90.86% |
| アプリ `src-tauri` | 測らない（今と同じ） | `cargo test -p local-sights` 20 件成功（Linux）。Windows の 9 件は CI の Windows で成功 |

## Target Verification Matrix

目標は `inception/requirements-analysis/requirements.md` の NFR1〜NFR6（細目に分けた）と、`code-generation-plan.md` の `## Testing Contract`（進め方・スコープの下限・量）から集めた。この作業には nfr-requirements / nfr-design のステージがない。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|-----------|--------|----------|--------|----------|--------------|---------|
| NFR1.a | requirements.md > NFR1 | 既存のテストが Linux・macOS・Windows のすべてで通る | 3 OS の `Tauri app` と `Rust library` が成功（PR #23 head f86c808、PR #24 head 572f3af） | CI run 38857429707、38939149369 | build-and-test | Met |
| NFR1.b | requirements.md > NFR1 | 薄い一本の手元の確認を Windows で行う（実際の AWS からの取得、画面を目で見る） | 開発者が Windows 11 の PC で起動・選択・取得・表示と SSO ログインを確認（2026-10-16） | `build-and-test-questions.md` Q1（A、B） | build-and-test | Met |
| NFR1.c | requirements.md > NFR1 | macOS で前と同じに動くことを確かめる | 開発者が Mac で枝 `claude/windows-support` を `npm run tauri dev` で起動し、取得・表示・SSO ログイン・終了が前と同じに動くことを確認（2026-10-17） | `build-and-test-questions.md` Q3 | build-and-test | Met |
| NFR2.a | requirements.md > NFR2 | OS ごとに分かれるコードのテストは、その OS のランナーで実行される。偽の AWS CLI は Windows 向けにもある | `windows_tests`（偽の `aws.cmd` + PowerShell の子）6 件、`cli_candidates`/`locate_cli` の Windows 5 件、キャッシュの Windows 1 件が `windows-latest` の `cargo test --workspace` で成功 | CI `Tauri app (windows-latest)`（PR #23、#24） | build-and-test | Met |
| NFR2.b | requirements.md > NFR2、team.md Testing Posture | 行カバレッジは Linux で測り 80% を下回らない | ライブラリ 97.67%、画面 90.86% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`npm run test:coverage` | build-and-test | Met |
| NFR2.c | requirements.md > NFR2 | 自動テストと CI は実際の AWS につながない | 偽の CLI と偽のゲートウェイだけ。ワークフローに AWS の secrets なし | `security-test-instructions.md` 2（`grep "secrets\.\|AWS_"` は一致なし） | build-and-test | Met |
| NFR3.a | requirements.md > NFR3、project.md Forbidden | 呼ぶ API は読み取り 3 つだけ、秘密を書かない、テレメトリを送らない | `gateway/` に差分なし。差分にキー・トークンの形の文字列なし。新しい出力先なし | `security-test-instructions.md` 2（`git diff f09f448 HEAD`） | build-and-test | Met |
| NFR3.b | requirements.md > NFR3 | Windows のキャッシュと設定のアクセス権は OS の既定に任せ、その旨を手順書に書く | `ensure_private_dir` は Windows ではフォルダを作るだけ。`docs/install.md` の Settings and cache on Windows に記載 | `crates/local-sights-core/src/cache/mod.rs`、`docs/install.md` | build-and-test | Met |
| NFR3.c | requirements.md > NFR3 | 依存関係に脆弱性がない（`windows-sys` を含む） | advisories・bans・licenses・sources ok、npm の high 以上 0 件、GitGuardian 成功 | `cargo deny check`、`npm audit --audit-level=high`、PR のチェック | build-and-test | Met |
| NFR4 | requirements.md > NFR4 | Microsoft がサポートしている版の Windows 11（x64）の開発者の PC で動く | dry run のインストーラで入れて起動・取得・表示を確認 | `build-and-test-questions.md` Q1（A） | build-and-test | Met |
| NFR5.a | requirements.md > NFR5 | AWS CLI の場所に空白があっても扱える（`Program Files`） | `cli_candidates` のテスト（Windows 3 件、CI）と、手元の SSO ログインで `C:\Program Files\Amazon\AWSCLIV2\aws.exe` から起動できた | CI `Tauri app (windows-latest)`、`build-and-test-questions.md` Q1（B） | build-and-test | Met |
| NFR5.b | requirements.md > NFR5 | 利用者のフォルダの名前に空白や日本語が入っていても設定・キャッシュ・AWS の設定ファイルを扱える | 確認していない。開発者の Windows のユーザー名には日本語も空白もなく、テスト用アカウントでの確認は行わない（利用者の判断、Q4 の B） | `build-and-test-questions.md` Q4 | build-and-test | Unverified |
| NFR6 | requirements.md > NFR6 | VoiceOver で確かめている読み上げを、Windows ではナレーターで同じように確かめる | ナレーターでの確認は行わない（利用者の判断、2026-10-17）。読み上げのための属性（`aria-hidden`、`role=status` など）は画面のテストで成功。画面のコードは OS で分かれず、この作業で変えていない | `npm run test:coverage`（392 件成功）、前の作業 261012-parallel-fetch の VoiceOver の扱い（Q2 の A）と同じ判定 | build-and-test | Met |
| TC.ordering | code-generation-plan.md > Testing Contract > ordering | 純粋なロジックはテスト先行、境界の層は実装後にテスト | `cli_candidates` はテストを先に書いて失敗を確かめてから実装。境界（unix.rs / windows.rs、browser.rs、キャッシュ）は実装後にテスト | `code-summary.md` テストの結果 > テスト先行の記録 | build-and-test | Met |
| TC.scope_floor | code-generation-plan.md > Testing Contract > obligations.scope_floor（bugfix） | 対象の不具合への回帰テストがあり、既存のテストが緑のまま | Windows で落ちた既存のキャッシュのテストが `is_missing` の修正で通る（回帰）。既存のテストは 3 OS で成功 | CI（PR #23 f86c808 以降） | build-and-test | Met |
| TC.volume | code-generation-plan.md > Testing Contract > obligations.strategy_volume（standard） | 部品ごとに 5〜8 件、主な境界に結合テスト | `sso_cli` 25 件（Windows 11、Unix 12、共通 2）、キャッシュ 1 件、`signing.test.ts` 5 件。境界は偽の CLI の子プロセスまで含めて結合で確認 | `code-summary.md` 追加したテスト | build-and-test | Met |

## 準備の状態

- **build-ready**：はい（Linux・macOS・Windows でビルドできる。CI で確認）。
- **test-ready**：はい（自動テストはすべて成功。`cargo test --workspace` の 1 回目で、PR #22 由来の既存の結合テスト 1 件が時間依存で落ちたが、単独で 5 回連続成功。`test-results.md` に記録）。
- **deployment-ready**：条件つき。16 件のうち Met 15 件、Unverified 1 件（NFR5.b、利用者が未確認のまま受け入れた）。このため本ステージは失敗の扱い（Accept failure）で承認ゲートに進む。`main` のルールセットの必須チェックへの `Tauri app (windows-latest)` の追加（FR1.3）は利用者の操作で、まだ行われていない（2026-10-17 時点で必須は 5 つ）。

## 残っている項目

- NFR5.b（日本語や空白の入った利用者フォルダ名）は未確認。該当する利用者から不具合の報告があれば次のパッチ版で直す。
- `main` のルールセットの必須チェックに `Tauri app (windows-latest)` を加える（FR1.3。PR #24 のマージ前）。
- PR #24 のマージ後、`docs/releasing.md` の順序で `v0.2.0` を出す（dry run → 両 OS で確認 → タグ）。
- 時間依存の既存テスト `u5_cancel::a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss`（PR #22 由来）の安定化は、この作業の範囲外。次の作業で扱う候補。
