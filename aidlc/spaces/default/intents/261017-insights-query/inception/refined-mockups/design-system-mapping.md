# Design System Mapping — Logs Insights 互換のクエリ（IB-11）

上流：`inception/requirements-analysis/requirements.md`（FR1〜FR9、NFR6、NFR11）、`mockups.md`、`interaction-spec.md`、質問票 Q1〜Q5。今の画面は Cloudscape Design System で組んである（261010-console-ui-polish）。この作業でも Cloudscape の部品だけを使い、独自のデザインは作らない（NFR11）。

## 使う部品

| 画面の部分 | Cloudscape の部品 | 設定 | 出典 |
|---|---|---|---|
| 画面の切り替え | `SideNavigation` | 項目 2 つ（Log Management、Log Analytics）、`activeHref` で今の画面 | Q1、FR1.1 |
| 全体の枠 | 今の 2 ペインの枠＋左端にナビゲーションの列 | ナビゲーションは畳める | Q1 |
| クエリの画面の囲み | `Container` + `Header`（「Log Analytics」） | | FR1.2 |
| ロググループ | `FormField` + `Multiselect` | `filteringType="auto"`、`tokenLimit` で並べる数を絞り「+N」表示、`statusType` で loading / error | Q2、FR1.3 |
| 期間 | `FormField` + `DatePicker` + `TimeInput` | Log Management と同じ設定（`yyyy/mm/dd`、`hh:mm:ss`、24 時間） | FR1.4 |
| クエリ欄 | `FormField` + `CodeEditor` | 独自の言語の定義（色分け）、補完の提供、エラーの印（下線と行番号の横の印）、`preferences` は固定、`loading` を使う | Q3、Q5、FR6、FR9 |
| 実行 | `Button variant="primary"`（Run query）、`Button`（Cancel）、`ButtonDropdown`（Examples） | | FR1.7、FR2.4、FR9.3 |
| 実行中の表示 | `StatusIndicator type="loading"` | 結果の囲みの中 | FR2.4 |
| 結果 | `Container` + `Header`（件数、`counter`）+ 結果の表 | 表は Log Management の表と同じ作り（下の「結果の表」） | FR7 |
| 行の項目と値 | `KeyValuePairs` か、2 列の簡単な表 | 決まった項目 → JSON の項目 | Q4、FR7.4 |
| エラー・警告・知らせ | `Alert`（error / warning / info、dismissible） | | FR2.5、FR2.6 |
| 欄の不備 | `FormField` の `errorText` | | FR1.6 |
| 失敗したストリーム | 今の `Modal` + `Table` にロググループの列を足す | | FR2.5 |
| 例のクエリの置き換えの確認 | 今の確認ダイアログ（`Modal`） | | FR9.3 |

## CodeEditor について

- Cloudscape の `CodeEditor` は、中で Ace（`ace-builds`）を使う。`ace-builds` は画面側の依存として足し、`CodeEditor` の `ace` に渡す（Q3）。
- 依存を足すときは、今の決まりどおり `npm audit --audit-level=high` と GitHub の秘密情報スキャンを通す（team.md Code Style）。ライセンスは BSD-3-Clause で、今の依存の確認（`deny.toml` は Rust 側だけ）とは別に、PR で確かめる。
- Ace の Logs Insights 用の言語の定義はないため、色分けの規則（命令・項目・文字列・数値・正規表現・演算子）を自前で書く。規則は中核の読み取りと同じ言葉の一覧から作り、ずれないようにする（Functional Design で決める）。
- 補完は Ace の補完の仕組みに、中核から受け取った候補（命令、決まった項目、クエリの画面で取得済みのログの JSON の項目の名前）を渡す。
- エラーの印は Ace の注釈（行番号の横の印）と目印（下線）で出す。位置は中核の読み取りが返す行と列を使う。
- アプリの大きさ：Ace を足すと画面側の配布物が大きくなる。必要な部分（本体、言語の定義、補完）だけを読み込み、クエリの画面を初めて開いたときに読み込む（`loading` の状態を出す）。
- テーマは今の明暗（OS の設定に合わせる）に合わせて 2 つだけ使う。CodeEditor の設定画面は出さない。

## 結果の表

- Log Management の表（自前の仮想スクロール＋デザイントークン）を元に、列を可変にした版を作る。見えている行だけを中核から読む作りは同じ（FR7.5）。
- 列の見出しは項目名そのまま（`@timestamp` など）。時刻の列は `@timestamp (Local)` のようにタイムゾーンを添える。
- 行を開く印（▶ / ▼）、行の選び方、開いた中身の表示は Log Management と同じ部品を使い、中身だけを項目と値の一覧にする。

## デザイントークン

- 新しい色や余白は作らず、Cloudscape のトークン（今の表で使っているもの）を使う。
- エラーの下線と行番号の横の印は、Ace のテーマの中で Cloudscape のエラーの色のトークンに合わせる。

## OS ごとの違い

- キーの組み合わせの表示と受け付け：macOS は Cmd、Windows は Ctrl（実行、補完、画面の切り替え）。それ以外の見た目は同じ（NFR6）。
- 判断は画面側の OS を見る小さな関数に寄せ、テストで両方を確かめる（team.md Code Style）。
