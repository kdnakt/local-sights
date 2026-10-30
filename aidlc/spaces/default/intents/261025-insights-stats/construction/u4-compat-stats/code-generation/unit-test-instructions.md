# Unit Test Instructions — U4 互換の確かめ（u4-compat-stats）

## テストの道具と設定

- Rust：標準の `cargo test`。確認用プログラムのテストは `crates/local-sights-core/examples/compat/*.rs` と `examples/compat_check.rs` の `#[cfg(test)] mod tests` に書く（今の書き方）。新しい依存は足さない。
- 画面：Vitest（`vitest.config.ts` は今のまま）、Testing Library。
- 自動テストと CI は実際の AWS に接続しない。比べる処理は見本の結果（テストの中で組み立てたアプリ側の結果とコンソール側の書き出しの行）で確かめる。

## この作業単位のテストの実行のしかた

この作業単位のテストだけを、次のコマンドで実行する（Step 1 で、作業の前に通ることを確かめる）。

```bash
cargo test -p local-sights-core --example compat_check
npx vitest run src/components/QueryResultTable.test.tsx src/components/ExportNoticeAlert.test.tsx
```

- `--example compat_check` は `examples/compat/` の queries・format・compare・report のテストと、compat_check.rs のテストを含む。
- 純粋なロジックの段（Step 2・4・9）では、テストを書いたあとに 1 行目を実行し、新しいテストが失敗することを確かめてから実装する。
- コンソールとの比較そのもの（手元の作業）は自動テストに含めない。手順は `README.md` の確認用プログラムの節（Step 12）。

## テストの量（Standard）

| 部品 | テストの数の目安 | 中身 |
|---|---|---|
| queries（組の読み方） | 7 | 4 欄の行、5 つ目の欄、読めない欄、keyorder、provisional の注記 |
| format（集計の Row） | 5 | 列の名前、セルの文字、時刻の書き方、events は今のまま |
| compare（範囲） | 8 | 小数の絶対・相対の範囲、近似の 2%、数でない値、境界の値 |
| compare（対にする） | 6 | 緩めなし、鍵で対、同じ鍵が複数、by なしの 1 行、片側にしかない鍵 |
| compare（keyorder・打ち切り） | 5 | 鍵の並び、食い違う位置、sort なしの打ち切りは Skipped、同点の段 |
| report | 5 | 4 つの結果の出し方、例は 5 つ、仮の決まりのまとめ、期間の端、終了コード |
| compat_check（つなぎ） | 4 | 集計の AppRows、row_cap、打ち切りの判定、期間の端 |
| 組のファイル | 2 | 2 つの組が読める、provisional の id がすべてある |
| Vitest（持ち越し） | 4 | 詳細の取得を待つ、正しい行の詳細、通知の添え書き |

各部品で、普通の場合と、少なくとも 2 つの端の場合（境界の値、空の結果、片側にしかない行）を確かめる。

## カバレッジ

- 中核ライブラリ（`cargo-llvm-cov`）と画面（Vitest のカバレッジ）の行カバレッジは 80% を下回らない（CI の Linux のランナーで測る）。下限は下げない。
- 確認用プログラム（examples）は中核のカバレッジの計測の外だが、比べる処理の分岐は見本のデータでテストする。

## モックとスタブ

- アプリ側の結果は、中核の `QueryResult`（aggregates）をテストの中で組み立てるか、偽の取得元で評価して作る。本物の AWS は使わない。
- コンソール側の書き出しは、テストの中で JSON の行を組み立てる（実物の書き出しはリポジトリに置かない）。

## テストのデータ

- 見本の値は作ったもの（`eventSource` などの名前と、作った件数）にし、実際のアカウント ID・ロール名・ログの中身は使わない。
- 小数の範囲のテストには、境界のすぐ内側と外側の値（例：差がちょうど 0.0001 と 0.00011）を使う。
