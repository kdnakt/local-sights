# 結合テストの手引き（261020-carryover-fixes）

Test Strategy は Standard。部品の境をまたぐところだけを結合テストで確かめる。AWS にはつながず、偽のゲートウェイ（`crates/local-sights-core/tests/support/fake_gateway.rs`）と一時フォルダのキャッシュを使う。

## 境と確かめること

| 境 | 確かめること | テスト | 要件 |
|---|---|---|---|
| 取得の流れ × キャッシュ × 中断 | 照合が答える前に中断が届いている：当たりは Completed で 2 件、外れは一覧の前に Cancelled で AWS を呼ばない。時計に依らず毎回同じ | `crates/local-sights-core/tests/u5_cancel.rs` の `a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss` | FR1 |
| クエリの実行 × QueryStore × キャッシュ × 取得 | 選ばれなくなったロググループは次の実行で捨てられ、もう一度選ぶと取り直す。残ったものは足りない範囲だけを取る | `crates/local-sights-core/src/query/runner.rs` の FR2.2 のテスト | FR2.2 |
| キャッシュのファイル × クエリの取り足し | 重なる範囲のうち一番後ろまでで読むのをやめ、結果は全部読んだときと同じ | `crates/local-sights-core/src/cache/mod.rs` の FR2.1 のテスト | FR2.1 |
| ライブラリ × Tauri のコマンド × 画面 | 行と詳細を取るコマンドが `holdings_epoch` なしで同じ行と詳細を返し、画面は `runId` だけで古い応答を捨てる | `src-tauri/src/lib.rs` のコマンドのテスト、`src/components/QueryResultTable.test.tsx`、`src/App.test.tsx` | FR2.3 |
| build.rs × ハンドラ × capabilities | 3 か所のコマンドの一覧が同じで空でない（数は固定しない） | `src-tauri/src/lib.rs` の `the_manifest_the_capability_and_the_handler_list_the_same_commands` と `same_commands` のテスト | FR4.3 |
| 画面 × Rust のリージョンの一覧 | 名前の表のコードの集まりが `STANDARD_REGIONS` と同じ（Rust のソースを読む） | `src/i18n/regions.test.ts` | FR5.10 |

## 流し方

```bash
cargo test -p local-sights-core --test u5_cancel
cargo test -p local-sights-core --lib query::runner
cargo test -p local-sights-core --lib cache::
cargo test -p local-sights
npx vitest run src/i18n/regions.test.ts src/components/QueryResultTable.test.tsx src/App.test.tsx
```

CI では、Rust のテストは Linux・macOS・Windows（`windows-latest`）の各ランナーで（`Tauri app (<os>)` のジョブ）、画面のテストは Linux のランナーで流れる。

## 手元の目視（A2、macOS だけ）

GUI を起動して次を目で確かめる（実際の AWS への取得の流れは変えていないため、確認用プログラムは流さない）。Windows の手元の確認は省く（A1）。

1. リージョンの選択欄：日本語と英語の名前、「東京」「tokyo」での絞り込み、候補なしの文、閉じたときの表示、サインイン中は押せないこと。
2. 英語の単数・複数：1 本のストリームだけのロググループで「1 stream」と出ること。
3. 中断の知らせ：取得を中断すると知らせが出て、知らせがないときに上の余白が前と同じであること。
4. クエリの画面：ロググループを変えて実行し直しても結果が出ること。

## カバレッジ

画面は `npm run test:coverage`、ライブラリは `cargo llvm-cov -p local-sights-core --fail-under-lines 80`。どちらも 80% 以上（下げない）。
