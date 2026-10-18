# Unit Test Instructions — U2 クエリの言語をそろえる（u2-query-language）

## テストの仕組み

- Rust：今の `cargo test`（テストは各モジュールの中の `#[cfg(test)] mod tests`）。速さの測りは今と同じく `#[ignore]` を付けたテストにし、`--release -- --ignored` で流す。新しいテストの仕組みは足さない。
- 画面：今の Vitest（jsdom、Testing Library）。CodeEditor（Ace）は U1 の `src/test/codeEditorMock.tsx` の差し替えを使う。
- 進め方は計画の Testing Contract のとおり。純粋なロジック（`query/parser.rs`・`query/fields.rs`・`query/natural.rs`・`query/evaluator.rs`）はテストを先に書いて失敗を確かめてから実装する。QueryRunner・InsightsSession・Tauri のコマンド・画面は、実装してからテストを書いて実行する。

## U2 のテストの実行（この作業単位だけ）

最初のテスト先行の手順の前に、次のコマンドが動くことを確かめる。

```bash
# 中核ライブラリ：読み取り・項目・自然順・評価・実行の流れ
cargo test -p local-sights-core query::

# 中核ライブラリ：クエリの画面の状態
cargo test -p local-sights-core insights_session::

# Tauri アプリ：コマンドの登録と許可の一致、行の値と一覧の形
cargo test -p local-sights query

# 画面：U2 で変える部品
npx vitest run src/components/QueryEditor.test.tsx src/components/QueryResultTable.test.tsx src/components/LogAnalyticsPage.test.tsx src/i18n/messages.test.ts

# 速さの測り（Step 17。手元で、--release で）
cargo test --release -p local-sights-core query:: -- --ignored
```

Step 16 では、既存のテストが壊れていないことを確かめるため、あわせて `cargo test --workspace` と `npx vitest run` も流す。

## 期待するカバレッジ

- 今と同じく、行カバレッジ 80% を CI（Linux のランナー）で確かめる。下限の設定は変えない。
- 純粋なロジックの公開関数には必ずテストを書く。

## 偽物の使い方

- AWS は呼ばない。実行の流れのテストは U1 の偽のゲートウェイ（`query/runner.rs` のテストの道具）を使う。
- 評価の異常終了は、テストの中だけで panic する偽の評価を差し込んで作る。

## テストのデータ

- コンソールで確かめた結果（`functional-design/functional-design-questions.md` の Q1a・Q1b）を、そのままテストの例にする。たとえば、ない項目の `!= "zzz"` は満たす、`"10" = 10` は満たす、`limit 100001` は Syntax、sort なしは降順。テストの名前に Q1a の番号（`q1a_5d` など）を入れて、根拠をたどれるようにする。
- 確かめていない組（数字だけの文字列と数の `<`・`>`、真偽値、JSON の数と文字列の実値、`AND`・`OR`・`NOT` の大文字、`limit 0` と 10,000 件を超えるログ）のテストは、名前に `provisional` を入れる。U4 で確かめて、違えば直す。
- JSON のメッセージは各テストの中で作る。実際のログ（CloudTrail など）はリポジトリに置かない（NFR4）。
- 速さの測りのイベントは、今の速さのテストと同じく、テストの中で作る（10 万件・100 万件。半分ほどに JSON の項目を持たせる）。

## 手元の確認（自動テストではない）

```bash
cargo run -p local-sights-core --example fetch_check -- --profile <プロファイル> --region us-east-1 \
  --log-group <ロググループ> --start "yyyy/mm/dd hh:mm:ss" --end "yyyy/mm/dd hh:mm:ss" \
  --query 'fields @timestamp, errorCode | filter errorCode like /Exception/ | sort @timestamp desc | limit 20'
```

- macOS と Windows の GUI で、JSON のログに fields・sort・limit を使ったクエリを実行し、列・並べ替え・打ち切りの表示・エラーの下線と印・行を開いた一覧を目で確かめる。
