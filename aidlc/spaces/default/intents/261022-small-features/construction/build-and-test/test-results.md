# Test Results — アプリの中での言語切替・書き出し・JSON の整形（261022-small-features）

実行日時：2026-10-24T18:00Z〜19:05Z（UTC）。実行環境：Linux のコンテナ（x86_64、GUI なし、AWS の認証情報なし）。対象のコミット：`34abd56`（U2 の記録）に、この段の修正（`Cargo.lock`・`deny.toml`・`package-lock.json`・README・`docs/install.md`）を加えた作業ツリー。

## 1. この段で直したこと（in-stage fix）

| # | 失敗 | 原因 | 直し方 | 直した後 |
|---|---|---|---|---|
| 1 | `npm audit --audit-level=high`：high 2 件（`undici` 8.0.0〜8.12.1、`zod` < 4.6.6） | 既存の推移的な依存の勧告（この作業は `package.json` を変えていない） | `npm audit fix`。変わったのは `package-lock.json` だけ（undici 8.11.2 → 8.12.3、zod 4.6.5 → 4.6.7） | `found 0 vulnerabilities`。Vitest 549 件は変わらず全件通過 |
| 2 | `cargo deny check` advisories：RUSTSEC-2026-0385（`ctutils` 0.4.3、`digest` → `sha2` → `aws-sigv4` 経由） | 既存の推移的な依存（`main` の `Cargo.lock` にも 0.4.3）。勧告が新しく出たもの | `cargo update -p ctutils`（0.4.3 → 0.4.5。`Cargo.lock` の 1 行） | advisories ok |
| 3 | `cargo deny check` licenses：`clipboard-win` 5.4.3・`error-code` 3.4.0 の BSL-1.0 が許可にない | U2 で足した `tauri-plugin-clipboard-manager` → `arboard` の Windows 向けの依存。cargo-deny はすべての target を調べる | `deny.toml` の `[licenses].allow` に `BSL-1.0`（Boost Software License、寛容）を足した。クレート単位の例外ではなく一覧への追加 | licenses ok |

いずれも、この段の持ち分（依存関係の lockfile と検査の設定）の中で直した。生成したコードとテストは変えていない。品質の目標を緩めてもいない（npm audit の水準と cargo-deny の各検査はそのまま）。

## 2. ビルド

| コマンド | 結果 |
|---|---|
| `npm run check:version` | 成功（4 つの版がそろっている） |
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功。warning 0、error 0 |
| `npx tsc --noEmit` | 成功 |
| `npx eslint .` | 成功 |
| `npx prettier --check .` | 成功 |
| `npx vite build` | 成功 |
| `npm audit --audit-level=high` | 1 の修正後、成功（0 件） |
| `cargo deny check` | 1 の修正後、成功（advisories ok, bans ok, licenses ok, sources ok） |
| `cargo build -p local-sights` | 成功（clippy とテストのビルドの中で確認） |

## 3. テスト

### 3.1 Rust（`cargo test --workspace`、Linux）

| 対象 | 通過 | 失敗 | 無視 |
|---|---|---|---|
| `local-sights-core` 単体（lib） | 690 | 0 | 9（既存。実 AWS 向けなど） |
| `local-sights-core` 結合テスト（`tests/u1_*`〜`u6_*` ほか 9 ファイル） | 78 | 0 | 0 |
| `local-sights-core` doc テスト | 51 | 0 | 0 |
| `local-sights-core` examples のテスト | 7 | 0 | 0 |
| `local-sights`（Tauri アプリ。`export_boundary` と SSO CLI） | 39 | 0 | 0 |
| **合計** | **865** | **0** | **9** |

### 3.2 画面（`npm run test:coverage`、Vitest）

| 対象 | 通過 | 失敗 |
|---|---|---|
| 46 ファイル | 549 | 0 |

1 回目（cargo-llvm-cov と cargo-deny のインストールと同時に実行）は `App.test.tsx` の画面の切り替え 2 件と `LogGroupMultiselect.test.tsx` 1 件が 5 秒の timeout で失敗した。負荷が原因で、単独で流し直して 549 件すべて通った。timeout の値は変えていない。CI では画面のテストは単独のジョブで走る。

### 3.3 記録済みの確認コマンド

`cargo test --workspace && npx vitest run`：両方とも成功（3.1・3.2 のとおり）。

## 4. カバレッジ

| 対象 | 行カバレッジ | 下限 | 判定 |
|---|---|---|---|
| `local-sights-core`（`cargo llvm-cov -p local-sights-core --fail-under-lines 80`） | 96.74%（22,353 行中 728 行が未実行） | 80% | 合格 |
| 画面（Vitest v8） | 90.48%（Statements 90.55%、Branches 88.17%、Functions 86.95%） | 80% | 合格 |

この作業で足した・変えたモジュールの行カバレッジ：`cache/settings.rs` 99.57%、`export/exporter.rs` 99.62%、`export/formatter.rs` 98.34%、`session.rs` 98.25%、`insights_session.rs` 97.44%。

## 5. 失敗の詳細

自動テスト・ビルドの失敗は残っていない（1 の 3 件はこの段で直した）。

## 6. 手元で行う確認（この環境では実行できない）

team.md Walking Skeleton の 3（GUI の目視）と NFR3（両 OS で同じ振る舞い）は、GUI のない Linux のコンテナでは行えない。また CI は pull request と `main` への push でしか走らないため、このブランチでは macOS・Windows のランナーの結果もまだない。記録の仕方は `performance-test-instructions.md` の 4 と `integration-test-instructions.md` の 5 のとおり。確かめたら、ここに OS・日時・結果を 1 行ずつ足す。

| OS | 確かめること | 結果 |
|---|---|---|
| macOS（Apple Silicon） | README「Language, export and JSON (261022-small-features)」のチェックリスト（言語切替、4 通りの書き出し、取り消し、書けないフォルダ、JSON の整形と切替） | 未実施 |
| Windows 11（x64） | 同上（CI で作ったインストーラで入れたアプリ。SmartScreen の開き方は `docs/install.md`） | 未実施 |
| 両 OS | 100 万行の CSV 保存の間にスクロールと画面の切り替えができる（時間を記録。合否の数値はない） | 未実施 |
| CI（macOS・Windows のランナー） | `Tauri app (macos-latest)`・`Tauri app (windows-latest)` の clippy・ビルド・`cargo test`（`cfg(windows)` の `export_boundary` のテストを含む） | **成功**（2026-10-24T23:50Z、pull request #46、CI run 167、コミット 6ce1582。4 ジョブすべて緑） |

## 7. 停止して問うこと（halt-and-ask）

Target Verification Matrix（`build-and-test-summary.md`）で T-OS-CI・T-OS-MAC・T-OS-WIN の 3 つが `Unverified` のまま残る。これはこの段の持ち分（設定・環境）の中では直せず、生成したコードの欠陥でもないため、code-generation への戻し（loop-back）の対象ではない。候補とその影響の見積もりは `build-and-test-summary.md` の「停止して問うこと」に書き、承認ゲートの前に人に問った。

**人の答え（2026-10-24T19:10Z）**：`Accept failure`。3 つの `Unverified` を記録したまま承認ゲートに進む。CI の macOS・Windows の結果（pull request を作ると走る）と両 OS の手元の目視は、マージ前・配布前の作業として持ち越す。

## Loop-Back Log

（なし。code-generation への戻しは行っていない）
