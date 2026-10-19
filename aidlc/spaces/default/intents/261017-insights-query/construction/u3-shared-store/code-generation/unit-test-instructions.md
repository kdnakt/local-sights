# Unit Test Instructions — U3 クエリの画面の取り足しと複数のロググループ（u3-shared-store）

## テストの仕組み

- Rust：今の `cargo test`（テストは各モジュールの中の `#[cfg(test)] mod tests`）。非同期のテストは今と同じ `tokio` の `test-util`。キャッシュのテストは今と同じく `tempfile` の一時ディレクトリを使う。メモリと速さの測りは `#[ignore]` を付けたテストにし、`--release -- --ignored` で流す。
- 画面：今の Vitest（jsdom、Testing Library）。
- 進め方は計画の Testing Contract のとおり。純粋なロジック（`cache/plan.rs` の範囲の計算、`query/parser.rs`・`query/evaluator.rs` の並びの形と入れ子の上限、`query/store.rs` の順番の振り直し）はテストを先に書いて失敗を確かめてから実装する。キャッシュの照会（ファイルに触れる）、QueryStore の保持、QueryRunner、InsightsSession・AppSession、Tauri のコマンド、画面は、実装してからテストを書いて実行する。

## U3 のテストの実行（この作業単位だけ）

```bash
# 中核ライブラリ：範囲の計算とキャッシュの重なる部分の照会
cargo test -p local-sights-core cache::

# 中核ライブラリ：読み取り・評価・QueryStore・実行の流れ
cargo test -p local-sights-core query::

# 中核ライブラリ：クエリの画面の状態と、AppSession のうち U3 で足すテスト（名前に insights か query を含める）
cargo test -p local-sights-core insights_session::
cargo test -p local-sights-core session::tests::query

# Tauri アプリ：入力と行の読み出しの形、コマンドの登録と許可の一致
cargo test -p local-sights query

# 画面：U3 で変える部品
npx vitest run src/components/LogGroupMultiselect.test.tsx src/components/LogAnalyticsPage.test.tsx src/components/QueryResultTable.test.tsx src/i18n/messages.test.ts

# メモリと速さの測り（Step 13。手元で、--release で）
cargo test --release -p local-sights-core query:: -- --ignored
```

Step 12 では、既存のテストが壊れていないことを確かめるため、あわせて `cargo test --workspace` と `npx vitest run` も流す。キャッシュの今のテスト（`tests/u6_cache_flow.rs` など）と、ログ管理の画面の速さのテストも通ることを確かめる。

## 期待するカバレッジ

- 今と同じく、行カバレッジ 80% を CI（Linux のランナー）で確かめる。下限の設定は変えない。
- 純粋なロジックの公開関数には必ずテストを書く。

## 偽物の使い方

- AWS は呼ばない。実行の流れのテストは、U1・U2 の偽のゲートウェイ（ロググループとストリームごとにページを返し、一部のストリームを失敗させ、認証情報の失敗を返せるもの）を使う。AWS を呼んだ回数を数え、「AWS を呼ばない」ことを確かめる。
- 1 MB のスタックのスレッドは、テストの中で `std::thread::Builder::new().stack_size(1024 * 1024)` で作る（Windows のメインのスレッドの大きさに合わせる）。

## テストのデータ

- 要件の FR2 の受け入れ基準 6 つを、そのまま結合のテストの例にする（テストの名前に `fr2_ac1` などを入れる）。
- イベントとキャッシュのファイルは各テストの中で作る。実際のログはリポジトリに置かない（NFR4）。
- メモリの測りのイベントは、今の速さのテストと同じく、テストの中で作る。

## 手元の確認（自動テストではない）

- macOS と Windows で、実際の AWS に対して、取得済みの範囲の一部を含む期間でクエリを実行し、足りない分だけが取得されること（進み具合と取得した件数で分かる）と、複数のロググループの結果に `@log` が出ることを確かめる。
- 本物の Ace で、構文の色分け・補完・エラーの下線と行番号の横の印が出ることを目で確かめる（U2 の見直しの R-04）。
