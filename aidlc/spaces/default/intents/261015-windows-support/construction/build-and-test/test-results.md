# Test Results — 261015-windows-support

実行日：2026-10-16〜17。場所：クラウドのコンテナ（Linux、Rust 1.97.0、Node.js v22.22.0）。枝 `claude/windows-support`（head `572f3af`、PR #24）。Windows と macOS のビルド・テストは CI の結果を記録する。

## ビルド

| コマンド | 結果 |
|----------|------|
| `cargo build -p local-sights` | 成功（終了コード 0） |
| `npx tsc --noEmit` | 成功 |
| `npx vite build` | 成功 |
| `npm run check:version` | 成功（`Version 0.2.0 is the same in Cargo.toml, Cargo.lock, package.json, src-tauri/tauri.conf.json.`） |
| CI `Tauri app (ubuntu-latest / macos-latest / windows-latest)`（clippy、アプリのビルド、`cargo test --workspace`） | 3 OS とも成功（PR #23 head f86c808、PR #24 head 572f3af） |
| Release dry run（run 38858473889） | `.dmg` と `-setup.exe` の両方を作成（成果物 `local-sights-dmg-dry-run`、`local-sights-windows-dry-run`） |

## 書式・リンタ・依存関係

| コマンド | 結果 |
|----------|------|
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功（警告 0） |
| `npx eslint .` | 成功 |
| `npx prettier --check .` | 成功 |
| `cargo deny check` | 成功（advisories ok、bans ok、licenses ok、sources ok） |
| `npm audit --audit-level=high` | 成功（脆弱性 0 件） |
| GitGuardian（PR のチェック） | 成功 |

## テスト

`unit-test-instructions.md` のコマンド（`cargo test -p local-sights sso_cli`、`cargo test -p local-sights-core cache::`、`npx vitest run scripts/release/signing.test.ts`）は、下の全体のコマンドに含まれるため、全体のコマンドを 1 回ずつ走らせた（同じテストを二重に数えない）。

| コマンド | 合計 | 成功 | 失敗 | 飛ばし |
|----------|------|------|------|--------|
| `cargo test --workspace`（Linux：ライブラリの単体・結合、アプリ、doc テスト） | 554 | 549 | 0 | 5（速度測定、`#[ignore]`） |
| `cargo test --workspace`（CI の `windows-latest`。Windows 専用の 12 件を含む） | — | すべて成功 | 0 | — |
| `cargo test --workspace`（CI の `macos-latest`） | — | すべて成功 | 0 | — |
| `npm run test:coverage`（Vitest） | 392 | 392 | 0 | 0 |
| `npx vitest run scripts/release/signing.test.ts` | 31 | 31 | 0 | 0 |

### 1 回目の `cargo test --workspace` の失敗（時間依存の既存テスト）

1 回目の全体実行（カバレッジ測定や画面のビルドと同じ時間帯に走らせた）で、次の 1 件が落ちた。

```
test a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss ... FAILED
thread '...' panicked at crates/local-sights-core/tests/support/fake_gateway.rs:297:14:
FakeGateway received more DescribeLogStreams calls than scripted
test result: FAILED. 10 passed; 1 failed; ... (u5_cancel)
```

- このテストと偽のゲートウェイは PR #22（261012-parallel-fetch、コミット f09f448）のもので、今回の作業では触っていない。
- 単独で 5 回連続（`cargo test -p local-sights-core --test u5_cancel`）、全体で 2 回目（上の表）とも成功。CI でも PR #22・#23・#24 で落ちていない。
- 判断：負荷の高いときに現れる時間依存の失敗で、この作業の不具合ではない。このステージの範囲（テストの設定やビルド）でもないため直さず、残っている項目として記録する（Build and Test の失敗の扱いには含めない。原因は Windows 対応の変更の外にある）。

## カバレッジ

| 対象 | コマンド | 行カバレッジ | 下限 |
|------|----------|--------------|------|
| ライブラリ（local-sights-core） | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 97.67% | 80% |
| 画面（src/） | `npm run test:coverage` | 90.86% | 80% |

`cfg(windows)` のコードは Linux の計測の対象外（了解済みの制約）。

## main との差分の確認（`security-test-instructions.md` 2）

基点 `f09f448`（PR #22 の後の `main`）と head。

| 確かめたこと | 結果 |
|--------------|------|
| `crates/local-sights-core/src/gateway/` の差分 | なし（呼ぶ API は読み取り 3 つのまま） |
| 依存の差分 | `windows-sys 0.61`（`cfg(windows)`）の追加、`libc` の `cfg(unix)` への移動、版 0.2.0 だけ |
| プロセスの起動 | `Command::new(cli)`（`locate_cli` の絶対パス）に引数を配列で渡す。Windows は `creation_flags(CREATE_NO_WINDOW)`。ブラウザーは `/usr/bin/open` / `ShellExecuteW`（許可した URL だけ）。テストの `tasklist` は確認用 |
| ワークフローの secrets | `GITHUB_TOKEN` だけ。`AWS_` を含む secrets なし |
| キー・トークンの形の文字列 | 差分になし |
| Windows の権限の記載 | `docs/install.md` の Settings and cache on Windows に、Windows の既定のままと記載 |

## 手元の確認

開発者本人の PC で、本人の AWS 認証情報を使って行う。数値・プロファイル名・ロググループ名は記録しない。

| 目標 | 確認 | 結果 |
|------|------|------|
| NFR1.b、NFR4、FR3.1 | Windows 11 の PC に dry run のインストーラで入れ、起動・プロファイルとリージョンの選択・取得・表示 | 合格（2026-10-16、Q1 の A） |
| NFR5.a、FR4.1〜FR4.5 | Windows で SSO ログイン：コンソール窓が出ずブラウザーが開く（CLI は `Program Files` から起動） | 合格（Q1 の B） |
| FR4.4（手元） | 中断・時間切れ・終了のあと `aws.exe` が残らない | 手元では未確認。自動テスト（`windows_tests` 4〜6、CI の Windows）で確認済み |
| NFR1.c | macOS で前と同じに動く（取得・表示・SSO ログイン） | 合格（2026-10-17、枝を `npm run tauri dev` で起動。Q3） |
| NFR5.b | 利用者フォルダに空白や日本語があるアカウント | 未確認（利用者が受け入れ。Q4 の B） |
| NFR6 | ナレーター（経過時間の秒は読まない、警告は 1 回） | 行わない（利用者の判断）。画面のテストで判定 |
| （目標外）ダークモードと 1024×640 | README の Windows の項目 | 未確認（残っている項目） |

## Target Verification Matrix の集計

16 件のうち Met 15 件、Unverified 1 件（NFR5.b）、Not Met 0 件。表は `build-and-test-summary.md`。

## 失敗の扱い

Unverified が 1 件あるため、このステージの判定は失敗。原因はコードや設定ではなく、該当する利用者アカウントでの手元の確認をしていないこと。利用者が「未確認のまま受け入れる」を選んだ（`build-and-test-questions.md` Q4、Accept failure に当たる）。直す対象がないため、コード生成への差し戻しは行わない。

## 要件の対応表

35 件のうち OK 34 件、Deferred 1 件（FR1.3：リポジトリの設定、利用者の操作）。判定は PASS。中身は `cross-unit-traceability.md`。
