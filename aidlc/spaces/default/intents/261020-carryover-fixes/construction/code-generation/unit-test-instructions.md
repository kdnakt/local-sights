# 単体テストの手引き（261020-carryover-fixes）

## 使う道具

- Rust：`cargo test`（テストは各モジュールの `#[cfg(test)] mod tests` と `crates/local-sights-core/tests/` の結合テスト）。非同期のテストは `#[tokio::test(start_paused = true)]`。設定の追加はない。
- 画面：Vitest（`vitest.config.ts`、環境は jsdom、`src/test/setup.ts`）と Testing Library。Rust のソースを読むテストは `?raw` の読み込みを使う（`src/commandRegistration.test.ts` と同じ）。設定の追加はない。

## この作業のテストの流し方

どれもリポジトリの根で流す。全体のコマンド（`cargo test --workspace`、`npx vitest run`）は Build and Test で流すため、ここではこの作業で触るテストだけに絞る。

```bash
# FR1：不安定だったテスト（1 回）
cargo test -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup

# FR1：200 回続けて流す（FR1.3 の受け入れ基準）
for i in $(seq 1 200); do cargo test -q -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup || break; done

# FR2.1：読み止めの判断とファイルの読み
cargo test -p local-sights-core --lib cache::

# FR2.2：保持の捨て方と取り足し
cargo test -p local-sights-core --lib query::store
cargo test -p local-sights-core --lib query::runner

# FR2.3：holdings_epoch を取り除いたあとの結果・行・詳細
cargo test -p local-sights-core --lib insights_session
cargo test -p local-sights-core --lib query::evaluator

# FR4.3：権限のテスト
cargo test -p local-sights --lib tests::the_manifest_the_capability_and_the_handler_list_the_same_commands
cargo test -p local-sights --lib tests::same_commands

# FR4.1・FR5（文と名前の表、絞り込み、一致）、FR4.2・FR5（部品）、FR2.3（画面の受け渡し）
npx vitest run src/i18n src/components/TopBar.test.tsx src/components/CancelNotice.test.tsx src/components/QueryResultTable.test.tsx src/App.test.tsx

# FR4.4：クエリの画面の進み具合の置き場所
npx vitest run src/components/LogAnalyticsPage.test.tsx
```

アプリのパッケージ名（`-p local-sights`）は `src-tauri/Cargo.toml` の `[package] name` に合わせる。違っていたら、その名前で流し、code-summary.md に書く。

## 足すテストと直すテスト（Standard：部品ごとに 5〜8 件）

| 部品 | ファイル | テスト |
|---|---|---|
| 不安定なテスト（FR1） | `crates/local-sights-core/tests/u5_cancel.rs` | 1 件を直す（取得の前に中断を送る補助関数に替える）。狙った回帰：200 回続けて通る |
| 読み止めの判断（FR2.1） | `crates/local-sights-core/src/cache/plan.rs` | `last_overlapping_range` 4 件、`EventCounter::verify_through` 3 件（テスト先行） |
| キャッシュの一部の読み（FR2.1） | `crates/local-sights-core/src/cache/mod.rs` | 4 件（1 つ目だけ／後ろの壊れに気づかない／重なる中の壊れは Corrupt で消す／またがる範囲は全部読んだときと同じ） |
| 保持の捨て方（FR2.2） | `crates/local-sights-core/src/query/store.rs` | 1 件を直し 1 件を足す（テスト先行） |
| 取り足し（FR2.2） | `crates/local-sights-core/src/query/runner.rs` | 1 件を足す（捨てたものは取り直し、残ったものは足りない範囲だけ） |
| holdings_epoch の削除（FR2.3） | `insights_session.rs`・`evaluator.rs`・`QueryResultTable.test.tsx`・`App.test.tsx` | 既存のテストの期待値から項目を消す。中身・並び・件数の期待値は変えない。古い `runId` の応答を捨てるテストを確かめる（なければ 1 件足す） |
| 単数・複数（FR4.1） | `src/i18n/messages.test.ts` | 6 か所 × 単数・複数、対象外の文 2 件、日本語、文字列の `"1"`、単数形の表の形（テスト先行） |
| 中断の知らせ（FR4.2） | `src/components/CancelNotice.test.tsx`・`src/App.test.tsx` | 今の 4 件＋2 件（空の囲み、同じ囲みに中身が入る）＋ App の 1 件 |
| 権限のテスト（FR4.3） | `src-tauri/src/lib.rs` | 1 件を直し、`same_commands` の落ちる場面 3 件を足す |
| 進み具合の置き場所（FR4.4） | `src/components/LogAnalyticsPage.test.tsx` | 4 件を足す（取得中はクエリの枠の中・前回の結果は結果の枠だけ・終わったら出ない・前回の結果がなければ結果の枠は見出しだけ）。置き場所を前提にした既存のテストを直す（文の期待値は変えない） |
| 名前と絞り込み（FR5） | `src/i18n/regions.test.ts` | 名前の形 3 件、絞り込み 5 件、`STANDARD_REGIONS` との一致 1 件（テスト先行） |
| リージョンの選択欄（FR5） | `src/components/TopBar.test.tsx`・`src/App.test.tsx` | 8 件を足し、ButtonDropdown 前提のテストを直す |

## カバレッジの目安

- 画面：`npm run test:coverage` で行カバレッジ 80% 以上（`vitest.config.ts` の閾値。下げない）。
- ライブラリ：CI の `cargo-llvm-cov` で 80% 以上（下げない）。足す関数（`last_overlapping_range`、`verify_through`、`regionLabel`、`filterRegionOptions`、単数形の選び分け）はすべてテストで通る。

## モックとスタブ

- AWS には接続しない。結合テストは `tests/support/fake_gateway.rs` の `FakeGateway` を使う。FR1 の外れの場面では台本のない `FakeGateway::default()` を使い、呼び出しの記録が空であることで「一覧の前に中断した」ことを確かめる。
- 中断は `retry::stop_pair()` の `StopHandle::stop` を取得の前に呼んで作る。時計（`tokio::time::sleep`）で中断の時刻を決めない。
- 画面のテストは `src/test/fixtures.ts` の `sessionView`・`resultSummary`・`queryRowWindow` などを使い、`api` は `vi.mocked` で差し替える（今のテストと同じ）。

## テストのデータ

- キャッシュのファイルはテストの中で `tempfile::tempdir()` の下に `LogCache::write` で作り、壊すときは書いたファイルの行をテストの中で書き換える（今の `cache/mod.rs` のテストと同じやり方）。
- リージョンの名前は `src/i18n/regions.ts` の表がそのまま正本で、テストは別の写しを持たない。`STANDARD_REGIONS` との一致は Rust のソースを読んで確かめる。
