# Code Summary — U4 互換の確かめ（u4-compat-stats）

計画の Step 1〜16 をすべて行い、計画ファイルのチェックもすべて付いている。最後の確認として、次のものがすべて通った。

- `cargo fmt --check`、`cargo clippy --workspace --all-targets`（警告 0）
- `prettier --check`、`eslint`、`tsc --noEmit`
- `cargo test --workspace`、Vitest 全体

コンソールとの比較そのもの（UC1・UC2）は、開発者の手元で行う作業として残っている（下の「手元の比較の結果」）。

## 作った・変えたファイル

- 新しく作ったもの：`crates/local-sights-core/examples/compat_queries_numeric.txt`（数の組 n01〜n06）
- 変えたもの：
  - 確認用プログラム：`examples/compat/queries.rs`・`format.rs`・`compare.rs`・`report.rs`、`examples/compat_check.rs`
  - 比べるクエリの組：`examples/compat_queries.txt`（s01〜s16 を足した）
  - 書き出しのスクリプト：`scripts/compat/insights-export.sh`（設計からのずれ。下の 1）
  - `README.md`
  - 画面（持ち越し）：`src/components/QueryResultTable.tsx`・`ExportNoticeAlert.tsx`、`src/App.tsx`、`src/analytics.ts` と、それぞれのテスト（`App.test.tsx` を含む）

消したファイルはない。新しい依存は足していない。

## 実装の主な決め方

- **集計の結果の形**：`AppSide::Aggregates` を集計の行を持つ形（`Aggregates(AppRows)`）にした。U1 で入れた「U4 で比べる」として比べない分岐は消した。
- **比べる列**：どれかの行に値がある列だけを比べる（`columns_with_values`）。コンソールの書き出しは値のある項目だけを列に出すため。
- **緩めの判定**：小数の範囲には、浮動小数の読み取りの誤差 1e-12 を足した。0.5001 − 0.5 が f64 ではわずかに 0.0001 を超えるため。範囲そのものは広げていない（BR2.2）。
- **緩めのある列がある行**：緩めのない列の (名前, 値) を鍵にして対にする。同じ鍵の行は、緩めのある列の昇順に対にする（BR2.6）。
- **打ち切り**：
  - 切られたかどうかは `matched_count` が表に出した行の数より多いかで決める。
  - 打ち切りの同点の段は、最後の行と同じ並べ替えの値の行を両側から除いて、件数だけを比べる。
  - 並べ替えの鍵と `row_cap` は、stats のあとの sort・limit だけから取る（BR2.7）。
- **期間の端**：bin の鍵ごとに、期間の最初と最後の区切りの値を求め、その行の違いは「期間の端の違い」として分けて出す。違いとして数える（BR2.7 (5)）。
- **報告**：範囲内の違いは今の英語の決まった言い方に合わせ、`Same (within decimal tolerance: n cells)`・`Same (within approximate tolerance: n cells)` と出す。例は最初の 5 つ。仮の決まりごとのまとめ（matches・differs・notChecked）は要約のあとに出す。終了コードは今のまま（BR2.5・BR1.4）。
- **組のファイル**：注記は `# provisional: <rule> = <ids>` か `= notChecked` と書く。組にない id を書くと、その行のエラーになる（BR1.4）。
- **持ち越し（U3 レビュー）**：
  - R-02：開いた行の状態に「どの版の位置か」を持たせ、`findQueryRow` で位置が付け替わるまで詳細を取りに行かない。
  - R-05：添え書きを付けるかどうかは、書き出しを始めたときの結果の stage（`isPartialResultExport`）で決め、次の書き出しまで変えない。

## テスト

| コマンド | 前 | 後 |
|---|---|---|
| `cargo test -p local-sights-core --example compat_check` | 54 | 85 |
| `npx vitest run src/components/QueryResultTable.test.tsx src/components/ExportNoticeAlert.test.tsx` | 26 | 29 |

- Vitest 全体は 597（`App.test.tsx` に 2 本を足した）。`cargo test --workspace` はすべて通った。
- 画面の行カバレッジは 91.03%。examples は中核のカバレッジの計測の外。
- 純粋なロジックの段（Step 2・4・9）では、テストを先に書き、型や関数がまだないためのコンパイルエラー（28・79・23）を確かめてから実装した。
- events の比べ方のテストは、期待値を変えずに通った。

## 計画からずれたところ

1. **`scripts/compat/insights-export.sh` を 2 か所直した**（設計では「変えない」）。
   - 何をしたか：mode に `keyorder` を受け付け、クエリを次のタブで終えるようにした。
   - 理由：直さないと、新しい組で書き出しが使えない。スクリプトは s15（keyorder）で止まり、s05・s14 では 5 つ目の欄までクエリとしてコンソールに送ってしまう。
   - 確かめ方：偽の CLI を使うテストを 1 つ足した。
2. **期待値を変えた既存のテスト**：
   - 組のクエリの数の確認：23 → 39（s01〜s16 を足したため）。
   - 「order は sort があるときだけ」の判定：stats のあとの sort も、sort があるとみなすように広げた。
   - U1 の「集計は U4 で比べる」の 2 本：実際の比較のテストに置き換えた（計画どおり）。
3. **組み立ての追加**：新しい欄（`CompareSpec` の tolerances・key_columns・period_edges、`Prepared.provisional`）を、テストの中で組み立てている所に書き足した。期待値は変えていない。
4. **`parse_query_set` を `#[cfg(test)]` にした**：本体は `read_query_set` を使うようになり、テストでしか使わなくなったため。

## 手元の比較の結果（BR3.1。開発者が手元で比べたあとに埋める）

| 仮の決まり | 確かめるクエリ | 結果（matches / differs / notChecked） | 対応（differs のとき：直した / 諮った） |
|---|---|---|---|
| U1:BR1.1 | s12-upper-command | | |
| U1:BR2.1 | s03-missing-key | | |
| U1:BR2.3 | n05-avg-text | | |
| U1:BR2.5・FR2.2 | s01-bin5m, s10-two-keys, n03-avg-bin | | |
| U1:BR2.6 | s04-nobykey-zero | | |
| U1:BR2.7 | s15-keyorder | | |
| U1:BR2.8 | n01-num-aggregates, n02-pct, n03-avg-bin, n04-sum-order, n06-to-millis | | |
| U2:BR1.1 | s13-upper-function, s14-mixed-function | | |
| U2:BR1.4 | s08-after-filter, s16-ne-missing | | |
| U2:BR2.1 | s05-count-distinct | | |
| U2:BR2.2 | n02-pct | | |
| U2:BR2.3 | n01-num-aggregates | | |
| U2:BR2.4 | s07-sorts-first | | |
| U2:BR2.5 | s06-earliest-latest | | |
| U2:BR3.2 | （なし） | notChecked（CloudTrail の組では 10,000 グループを作れない） | — |

## 残したこと

- コンソールとの比較そのもの（UC1・UC2）は、開発者の手元で行う。その結果を上の表に埋め、違いがあれば BR3.1 のとおり直すか諮る。
- アプリが s12〜s14（命令と関数名の大文字・小文字）を拒むかどうかは、手元の比較で分かる。今のパーサは命令を大文字・小文字を区別せずに読み、関数名は書いたとおりにだけ読む（U2:BR1.1 の仮の決まり）。
- R-02 の直しでは、`findQueryRow` が失敗すると、その版の間、開いた行は「読み込み中」のままになる（エラーは onError で知らせる）。
- 秘密情報の確認：差分と組のファイルに、12 桁の数・ARN・アクセスキーの形・書き出しの中身はなかった（NFR4）。
