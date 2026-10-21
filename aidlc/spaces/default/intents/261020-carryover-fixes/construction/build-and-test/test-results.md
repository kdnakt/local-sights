# テストの結果（261020-carryover-fixes）

実行日：2026-10-21。場所：手元の Linux（このセッションの作業用の機械）と GitHub Actions の CI（PR [kdnakt/local-sights#36](https://github.com/kdnakt/local-sights/pull/36)）。AWS にはつないでいない。

## ビルド

| コマンド | 結果 |
|---|---|
| `npm ci` | 成功 |
| `npx vite build`（`npm run build` の中身） | 成功 |
| `cargo build -p local-sights`（Tauri のアプリ） | 成功（dev プロファイル） |
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功（警告 0） |
| `npx tsc --noEmit` | 成功 |
| `npx eslint .` | 成功 |
| `npx prettier --check .` | 成功 |
| `npm run check:version` | 成功（0.2.0 で 4 つのファイルがそろう） |
| `npm audit --audit-level=high` | 成功（脆弱性 0） |
| `cargo deny check` | 最初は CI で失敗（RUSTSEC-2026-0382、下の「CI」）。`rustls-webpki` を 0.103.16 に上げたあとは、手元で `advisories ok, bans ok, licenses ok, sources ok` |

## テスト

| コマンド | 合計 | 通過 | 失敗 | 無視 |
|---|---|---|---|---|
| `cargo test --workspace` | 771 | 762 | 0 | 9（速さのテスト） |
| `cargo test -p local-sights-core --doc` | 0 | 0 | 0 | 0 |
| `cargo test -p local-sights-core --examples` | 58 | 58 | 0 | 0 |
| `cargo test --release -p local-sights-core --lib -- --ignored`（速さ、NFR4） | 9 | 9 | 0 | 0 |
| `cargo test -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup` を 200 回 | 200 | 200 | 0 | 0 |
| `npm run test:coverage`（Vitest、42 ファイル） | 496 | 496 | 0 | 0 |

`rustls-webpki` を上げたあとにも、`cargo test --workspace` を流し直した（762 件が通過）。

### 時間切れ（1 回目の Vitest）

1 回目の `npm run test:coverage` では、`src/components/LogGroupMultiselect.test.tsx` の 1 件（「says the limit and offers no other log group once fifty are chosen」）が 5 秒の上限で時間切れになった。

- このとき機械では、同時に `cargo install cargo-llvm-cov` のビルドが走っていた。
- このファイルはこの作業で触っていない（`origin/main` との差分なし）。
- 単体で流し直すと 9 件とも通った。重い処理が終わってから全体を流し直すと、42 ファイル・496 件がすべて通った。
- 上限は変えていない。

## カバレッジ

| 対象 | コマンド | 行カバレッジ | 下限 |
|---|---|---|---|
| ライブラリ | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 96.63%（21196 行中 714 行が通らない） | 80% |
| 画面 | `npm run test:coverage` | 90.01%（1783 行中 1605 行） | 80% |

## 安全の確かめ

- `git diff origin/main...HEAD` に、AWS の SDK の新しい呼び出しも `fetch(` もない（`no new AWS or network calls`）。
- 認証情報らしい文字列もない（`no credentials`）。
- テスト以外のコードの `unwrap()`／`expect()` の追加がないことは、Code Generation のレビュー（review-01.md）で確かめた。
- GitGuardian の確認は CI で成功した。

## CI（PR kdnakt/local-sights#36）

| ジョブ | 1 回目（b10ddfc） | 最後（13bc6a9、run 39542854716） |
|---|---|---|
| cargo-deny | 失敗：RUSTSEC-2026-0382（`rustls-webpki` 0.103.15、AWS SDK の `rustls` の下）。この作業の変更とは関係がなく、`main` の `Cargo.lock` でも同じ。`cargo update -p rustls-webpki` で 0.103.16 に上げた（e6c53b4） | 成功 |
| Rust library (fmt, clippy, test) | 次の push で取り消され、結果なし | 成功（`cargo llvm-cov --fail-under-lines 80` を含む） |
| Front end (tsc, eslint, prettier, vitest, build) | 同上 | 成功（`npm run test:coverage` を含む） |
| Tauri app (ubuntu-latest) | 同上 | 成功 |
| Tauri app (macos-latest) | 同上 | 成功 |
| Tauri app (windows-latest) | 同上 | 成功 |
| GitGuardian Security Checks | 成功 | 成功 |

13bc6a9 は e6c53b4 に記録の文書だけを足したコミットで、コードと `Cargo.lock` は e6c53b4 と同じ。

## 手元の目視（A2、macOS）

2026-10-21、開発者本人の Mac で行った（会話での報告）。

| 項目 | 結果 |
|---|---|
| 1. リージョンの選択欄（日本語の画面） | 問題なし |
| 2. 英語の単数形 | 見ていない（英語の画面に切り替える手間のため）。自動のテストは通過 |
| 3. 中断の知らせ | 見ていない。自動のテストは通過 |
| 4. クエリの画面 | 見ていない。自動のテストは通過 |

見ていない 3 つは、次の版を配る前の両 OS の手元の確認で見る。開発者から、アプリの中で言語を切り替えられるとよい、という要望があった（範囲外。次の作業の候補）。Windows の手元の確認は省く（A1）。
