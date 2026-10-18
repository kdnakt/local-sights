# Integration Test Instructions — Logs Insights 互換のクエリ（IB-11）

Test Strategy は `Standard`。単体テストは各作業単位の `unit-test-instructions.md` のとおり。ここでは作業単位をまたぐ境目のテストを並べる。自動テストは実際の AWS に接続せず、取得はすべて偽のゲートウェイで行う（project.md Forbidden、NFR4）。

## 1. 作業単位をまたぐ境目と、それを確かめるテスト

| 境目 | 作業単位 | テスト | 場所 |
|---|---|---|---|
| クエリの読み取り → 評価 → 結果の行 | U1・U2 | `query::` の単体テスト（読み取ったクエリを、偽のイベントの並びで評価する） | `crates/local-sights-core/src/query/*.rs` |
| QueryRunner → 今の取得の流れ（`run_fetch_with_cache`）→ QueryStore | U1・U3 | `query::runner` のテスト（偽のゲートウェイ、取り消し、一部の失敗、認証の失敗） | `crates/local-sights-core/src/query/runner.rs` |
| 取得済みの範囲 → 足りない部分だけの取得 → ディスクキャッシュ | U3 | `cache::plan`・`cache::` と `query::store` のテスト（FR2 の 6 つの受け入れ基準） | `crates/local-sights-core/src/cache/`、`query/store.rs` |
| InsightsSession ↔ AppSession（2 つの画面の取得の排他、接続の切り替え） | U1・U3 | `insights_session::`・`session::` のテスト | `crates/local-sights-core/src/insights_session.rs`、`session.rs` |
| Tauri のコマンド ↔ 画面 | U1〜U3 | アプリのテスト（`cargo test -p local-sights`）と Vitest の画面のテスト | `src-tauri/src/lib.rs`、`src/**/*.test.tsx` |
| ログの画面の取得と絞り込み（変えないもの、C5） | 既存 | 結合テスト `tests/u1_*`〜`tests/u6_*` | `crates/local-sights-core/tests/` |
| 確認用プログラム ↔ アプリの中核 ↔ 書き出し | U4 | `compat_check` の配線のテスト（偽のゲートウェイと一時フォルダ）とスクリプトのテスト（偽の `aws`、`#[cfg(unix)]`） | `crates/local-sights-core/examples/compat_check.rs` |

## 2. 流し方

```bash
# 中核ライブラリ（単体と結合）。行カバレッジつき（80% を下回ったら失敗）
cargo llvm-cov -p local-sights-core --fail-under-lines 80

# 例（fetch_check と compat_check）のテスト
cargo test -p local-sights-core --examples

# Tauri アプリのテスト
cargo test -p local-sights

# 画面のテスト（行カバレッジつき、80% を下回ったら失敗）
npm run test:coverage
```

Rust のテストは、CI の Linux・macOS・Windows（`windows-latest`）の各ランナーで流れる（team.md Testing Posture）。画面のテストは Linux のランナーだけ。

## 3. 期待するカバレッジ

- 中核ライブラリの行カバレッジ 80% 以上（`cargo-llvm-cov`、Linux）。
- 画面の行カバレッジ 80% 以上（Vitest）。
- 確認用プログラム（例）はカバレッジの対象の外。比べ方の公開関数にはすべてテストがある（U4）。

## 4. テストのデータと環境

- イベントはテストの中で作る。実際のログはリポジトリに置かない（NFR4）。
- 偽のゲートウェイはストリームごとにページを返す（`crates/local-sights-core/tests/support/` と各モジュールのテストの道具）。
- スクリプトのテストは PATH の先頭に偽の `aws` を置く。Windows のランナーでは飛ばす。

## 5. 手元で行う結合の確かめ（自動テストではない）

実際の AWS と本物の GUI を使う確かめは、開発者の手元で本人の認証情報で行う（team.md Walking Skeleton の 2 と 3）。手順と記録の仕方は `build-and-test-summary.md` の「手元で確かめること」と `test-results.md` を見る。
