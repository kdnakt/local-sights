# Unit Test Instructions — U1 薄い一本（u1-filter-skeleton）

## テストの仕組み

- Rust：今の `cargo test`（テストは各モジュールの中の `#[cfg(test)] mod tests`）。非同期のテストは今と同じ `tokio` の `test-util`（止めた時計）を使う。新しいテストの仕組みは足さない。
- 画面：今の Vitest（`vitest.config` は今のまま、jsdom、Testing Library）。
- 進め方は計画の Testing Contract のとおり。純粋なロジック（`query/parser.rs`・`query/fields.rs`・`query/instant.rs`・`query/evaluator.rs`）はテストを先に書いて失敗を確かめてから実装する。QueryStore・QueryRunner・InsightsSession・AppSession・Tauri のコマンド・画面は、実装してからテストを書いて実行する。

## U1 のテストの実行（この作業単位だけ）

最初のテスト先行の手順の前に、次のコマンドが動くことを確かめる（まだテストがないときは 0 件で通る）。

```bash
# 中核ライブラリ：クエリの層（読み取り・項目・日時の文字列・評価・QueryStore・実行の流れ）
cargo test -p local-sights-core query::

# 中核ライブラリ：クエリの画面の状態
cargo test -p local-sights-core insights_session::

# 中核ライブラリ：AppSession のうち U1 で足すテスト（名前に insights か query を含めて書く）
cargo test -p local-sights-core session::tests::insights
cargo test -p local-sights-core session::tests::query

# Tauri アプリ：コマンドの登録と許可の一致、行の読み出し
cargo test -p local-sights generate_handler
cargo test -p local-sights query

# 画面：U1 で足す部品と変える部品
npx vitest run src/components/AppNavigation.test.tsx src/components/LogAnalyticsPage.test.tsx src/components/LogGroupMultiselect.test.tsx src/components/QueryEditor.test.tsx src/components/QueryResultTable.test.tsx src/components/ConfirmDialog.test.tsx src/App.test.tsx
```

Step 17 では、既存のテストが壊れていないことを確かめるため、あわせて `cargo test`（ワークスペース全体）と `npx vitest run` も流す（これは Build and Test でも流す）。

## 期待するカバレッジ

- 今と同じく、行カバレッジ 80% を CI（Linux のランナー、ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）で確かめる。U1 で足すコードでこれを下回らない。下限の設定は変えない。
- 純粋なロジックの公開関数には必ずテストを書く（team.md Testing Posture）。

## 偽物の使い方

- AWS は呼ばない。取得のテストは `coordinator.rs` の今の `FakeLogs`（ストリームごとにページを返す偽のゲートウェイ）にならった偽物を使う。一部のストリームの失敗・一覧の失敗・止める信号の届く時機は、偽物の答え方で作る。
- 知らせ先（進み具合・終わり）は、受け取ったものを並べて持つだけの偽の知らせ先を使う。
- 画面のテストは、今の `src/test` の Tauri の `invoke` の差し替えにならい、`run_query` などの答えと `session-changed` の中身を差し替える。
- CodeEditor（Ace）は jsdom では動かないため、画面のテストでは QueryEditor の中の CodeEditor を、同じ値と変更の知らせを持つ簡単な入力欄に差し替える（Cmd/Ctrl+Enter の扱いは QueryEditor の側で確かめる）。

## テストのデータ

- イベントは各テストの中で作る（今の `event(timestamp, stream, sequence)` のような小さな関数）。実際のログはリポジトリに置かない（NFR4）。
- 日時の文字列の例は要件の受け入れ基準のもの（Asia/Tokyo の 2025-08-25 01:43:26 → 1756053806000）と、夏時間のあるタイムゾーン（America/New_York など）の曖昧な時刻・存在しない時刻を使う。

## 手元の確認（自動テストではない）

team.md Walking Skeleton の 2 と 3。開発者本人の手元で、本人の AWS 認証情報で行う。CI では行わない。

```bash
# 確認用プログラム：1 つのロググループの期間を取得して filter を評価し、件数と先頭の行を出す
cargo run -p local-sights-core --example fetch_check -- --profile <プロファイル> --region us-east-1 \
  --log-group <ロググループ> --start "2025/08/24 16:13:26" --end "2025/08/24 17:13:26" \
  --query 'filter @timestamp > 1756053806000'
```

- timestamp-check.sh と同じロググループ・期間なら 2190 件になる（Q2 の確かめの結果）。
- `--start`・`--end` と、クエリの中の日時の文字列（`@timestamp > '2025-08-24 16:43:26'` など）は UTC として読む。
- 出力：標準出力に `matched N of M events`（M は取得して評価した件数）と、合うイベントの先頭 10 件（`UTC の時刻<TAB>ストリーム名<TAB>1 行にしたメッセージ`）。標準エラーに取得のまとめ（`local-sights: fetch summary ...`）と件数、失敗したストリームを出す。
- 終了コード：0 は成功、1 は取得の失敗（一部のストリームの失敗を含む）か評価されなかったとき、2 は引数の誤り。クエリが読めないときは行・列とメッセージのキーを出して 2 で終わり、何も取得しない（U1:BR3.9）。
- `cargo test` は確認用プログラムのテスト（引数とクエリの読み取り）だけを実行し、AWS には接続しない。
- GUI（macOS と Windows）：Log Analytics の画面でクエリを実行して結果が出ること、ログ管理の画面の表示中のログと絞り込みがそのまま残ること、読み取りのエラー・[Cancel]・接続を変えるときの確認を目で確かめる。
