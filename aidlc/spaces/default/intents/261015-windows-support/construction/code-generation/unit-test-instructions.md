# Unit Test Instructions — Windows 対応（IB-16）

## テストの道具

- Rust：`cargo test`（ワークスペースの標準）。モック用の追加の道具はない。AWS CLI の偽物は、テストが一時フォルダに書くスクリプト（Unix は `/bin/sh`、Windows は `.cmd` と PowerShell）。`tempfile` は `src-tauri` の dev-dependency にある。
- TypeScript：Vitest（`vitest.config.ts`）。`scripts/release/signing.test.ts` は実際の `release.yml` と `docs/*.md` も読む。
- 実際の AWS にはつながない。CI にも認証情報はない。

## この作業のテストを動かす命令（この作業に限った範囲）

| 対象 | 命令 | 動く OS |
|---|---|---|
| AWS CLI の探索と起動・停止（`src-tauri/src/sso_cli/`） | `cargo test -p local-sights sso_cli` | Linux・macOS（Unix のテスト）、Windows（Windows のテスト） |
| キャッシュの Windows の権限（`crates/local-sights-core/src/cache/mod.rs`） | `cargo test -p local-sights-core cache::` | すべて（Windows の 1 件は Windows だけ） |
| リリースの署名の検査（`scripts/release/signing.ts`） | `npx vitest run scripts/release/signing.test.ts` | Linux |

最初のテストを書く前に、この環境（Linux）で上の 3 つが今のまま通ることを確かめる。Windows のテストは、この環境ではコンパイルも実行もできない（`x86_64-pc-windows-msvc` のリンカがない）ため、CI の `Tauri app (windows-latest)` で確かめる。

## 期待するカバレッジ

- 行カバレッジ 80% 以上（CI の Linux のランナーで `cargo llvm-cov -p local-sights-core` と `vitest --coverage` が測る。team.md）。`cfg(windows)` のコードは Linux の計測の対象外（了解済みの制約。要件分析 R-03）。
- `sso_cli` の Windows の境界：6 件（成功、失敗の終了コード、起動できない、時間切れで残らない、中断で残らない、アプリ終了で残らない）。
- `cli_candidates`（純粋なロジック、テスト先行）：Windows 3 件、Unix 1 件。
- `locate_cli` の Windows：2 件（`.exe` だけが見つかる、なければ `None`）。
- キャッシュ：Windows 1 件。
- `signing.ts`：3〜4 件。

## 偽物と差し替え

- 偽の AWS CLI（Unix）：`#!/bin/sh` のスクリプト。既存のまま。
- 偽の AWS CLI（Windows）：`.cmd` のバッチ ファイル。子プロセスが要るときは `powershell -NoProfile -Command "Set-Content <file> $PID; Start-Sleep 600"` を起こす。
- 「プロセスが残らない」の判定：Unix は `/proc/<pid>/stat` か `kill(pid, 0)`（既存）。Windows は `tasklist /FI "PID eq <pid>"` の出力に `<pid>` が含まれないこと。2 秒まで待つ。
- 一度に動かす偽の CLI は 1 つ（`SERIAL` のロック）。

## テストデータ

- すべて一時フォルダ（`tempfile::tempdir`）に作り、テストの終わりに消える。
- 出力の行は URL とコードの例（`https://device.sso.us-east-1.amazonaws.com/`、`ABCD-EFGH`）だけで、実在の認証情報は使わない。
