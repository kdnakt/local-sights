# Test Results — 261012-parallel-fetch

実行日：2026-10-14。場所：クラウドのコンテナ（Linux、Rust 1.97.0、Node.js v22.22.0）。枝 `claude/parallel-fetch`。

## ビルド

| コマンド | 結果 |
|----------|------|
| `cargo build -p local-sights` | 成功（終了コード 0） |
| `npx tsc --noEmit` | 成功 |
| `npx vite build` | 成功 |
| `npm run check:version` | 成功（版がそろっている） |

## 書式・リンタ・依存関係

| コマンド | 結果 |
|----------|------|
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功 |
| `npx eslint .` | 成功 |
| `npx prettier --check .` | 成功 |
| `cargo deny check` | 成功（advisories ok、bans ok、licenses ok、sources ok） |
| `npm audit --audit-level=high` | 成功（脆弱性 0 件） |

## テスト

各作業単位の `unit-test-instructions.md` のコマンドは、作業単位ごとの絞り込み（`--lib session::` など）が下の全体のコマンドに含まれるため、全体のコマンドを 1 回ずつ走らせた（同じテストを二重に数えない）。

| コマンド | 合計 | 成功 | 失敗 | 飛ばし |
|----------|------|------|------|--------|
| `cargo test --workspace`（ライブラリの単体・結合、アプリ、doc テスト） | 553 | 548 | 0 | 5（速度測定、`#[ignore]`） |
| `cargo test -p local-sights-core --release --lib -- --ignored`（速度測定） | 5 | 5 | 0 | 0 |
| `cargo test -p local-sights-core --example fetch_check` | 成功 | — | 0 | — |
| `npm run test:coverage`（Vitest） | 385 | 385 | 0 | 0 |

失敗はない。

## カバレッジ

| 対象 | コマンド | 行カバレッジ | 下限 |
|------|----------|--------------|------|
| ライブラリ（local-sights-core） | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 97.67% | 80% |
| 画面（src/） | `npm run test:coverage` | 90.85% | 80% |

## main との差分の確認

| 確かめたこと | 結果 |
|--------------|------|
| `gateway/mod.rs`（trait）の差分 | なし |
| SDK の呼び出し | `get_log_events`・`describe_log_groups`・`describe_log_streams` だけ |
| GetLogEvents の SDK の再試行 | `RetryConfig::standard().with_max_attempts(1)` |
| 新しい依存 | `futures-util`（`default-features = false, features = ["std"]`）だけ |
| 新しい出力 | 標準エラー出力の要約の 1 行と、確認用プログラムの中断のメッセージだけ。ファイル・ネットワークへの書き込みなし |
| Tauri の権限 | `cancel_fetch` を足しただけ |

## 手元の確認

2026-10-15、開発者本人の macOS で、本人の AWS 認証情報を使って行った。数値・プロファイル名・ロググループ名は記録しない（project.md の Forbidden、目標に数値なし）。

| 目標 | 確認 | 結果 |
|------|------|------|
| U1-NFR2.1、AC1.1.4 | 実際の AWS で main とこの枝を比べる（件数が同じで、この枝が速い） | 合格（開発者の報告：速さは良く、問題なし） |
| U4-NFR2.3 | GUI で取得中に経過が進み、終わると残る | 合格 |
| U4-NFR6.7 | スリープをまたいだ取得の経過の見え方 | 合格 |
| （目標外）U5 の中断 | ［中断］で止まり、取れたログが残る | 合格 |
| U4-NFR9.2、U5-NFR9.4 | VoiceOver での読み上げ | 行わない（利用者の判断、`build-and-test-questions.md` Q2）。画面のテストの結果で判定 |

確認の途中で、開発者が `examples/fetch_check.rs` の比較手順の説明を zsh でも動く書き方に直した（f95829a、コメントだけの変更）。取り込んだ後に `cargo fmt --all --check` と `cargo test -p local-sights-core --example fetch_check` が成功した。

## Target Verification Matrix の集計

89 件のうち Met 89 件、Unverified 0 件、Not Met 0 件。表は `build-and-test-summary.md`。

## 要件の対応表

直接 OK 49 件、間接 45 件（AC1.1.4 は手元の確認で合格したが、code-generation の対応表には行がない）。判定は FAIL（直接の対応では）。中身は `cross-unit-traceability.md`。
