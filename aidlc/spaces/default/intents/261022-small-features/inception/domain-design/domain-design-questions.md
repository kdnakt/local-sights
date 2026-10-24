# Domain Design — 質問

元にしたもの：要件（`inception/requirements-analysis/requirements.md` の FR1〜FR5、NFR1〜NFR6）、今のコードの知識ベース（`codekb/local-sights/architecture.md`・`component-inventory.md`）。要件で決まったこと（設定ダイアログに言語、保存とコピー、CSV と Markdown、行数の上限なし、JSON の判定と切替）は聞き直しません。ここでは「どの層にどの部品を置くか」と、新しい外部依存の有無を決めます。

## Q1. 書き出し（CSV・Markdown の組み立て）はどの層に置きますか？

行は中核ライブラリが持っています（ログの画面は `LogView`、クエリの画面は `InsightsSession` の結果）。100 万行を書くとき、行を画面側（WebView）へ運ぶとメモリと時間がかさみます。チームの決まりは「判断は中核に、OS に触れる処理は境界に、画面は表示だけ」です。

- A. 中核ライブラリに純粋な組み立て（CSV・Markdown の文字を作る）を置き、アプリ（Tauri）がファイルの書き込みとクリップボードを担う。画面は操作と状態表示だけ（おすすめ：行を運ばず、純粋なロジックをテスト先行で書ける）
- B. 画面側（TypeScript）で組み立て、ファイルの保存とクリップボードもブラウザの API で行う（新しい Rust のコードは少ないが、行を全部 IPC で運ぶ）
- C. 中核で組み立てるが、クリップボードへのコピーだけは画面側（`navigator.clipboard`）で行う
- D. Not yet defined（機能設計で決める）
- X. Other (please specify)

[Answer]: A

## Q2. 保存ダイアログとクリップボードに、新しい依存を入れますか？

今のアプリは Tauri の公式プラグインを使っていません（コマンドと 3 つのイベントだけ）。保存ダイアログとクリップボードは OS に触れる処理で、何かで実装する必要があります。

- A. Tauri の公式プラグイン（`tauri-plugin-dialog`、`tauri-plugin-clipboard-manager`）を入れる（おすすめ：両 OS の差をプラグインが吸収し、権限は `capabilities/default.json` に足す）
- B. Rust のクレートを直接使う（`rfd` でダイアログ、`arboard` でクリップボード）。プラグインの権限の仕組みを通さない
- C. ダイアログはプラグイン、クリップボードは画面側のブラウザ API（新しい Rust の依存を 1 つに抑える）
- D. Not yet defined（機能設計で決める）
- X. Other (please specify)

[Answer]: A

## Q3. JSON の検出と整形はどの層に置きますか？

整形するのは行を開いたときの表示だけで、メッセージの文字はすでに画面側（`ExpandedMessage`）にあります。「整形／元のまま」の切替は行ごとの画面の状態です。レビューの指摘（表記を変えない：`1.0`、桁の大きい整数）があるため、`JSON.parse` で読んで書き直すのではなく、文字を走査して空白と改行だけを足す整形器が要ります。

- A. 画面側（`src/` の純粋な TypeScript の補助）に置き、Vitest でテスト先行（おすすめ：表示だけの機能で、IPC を増やさない）
- B. 中核ライブラリ（Rust）に置き、Tauri のコマンドで整形後の文字を返す（Rust のテストで守れるが、行を開くたびに IPC が 1 回増える）
- C. 中核に純粋な関数を置き、`get_rows` の `DisplayRow` に整形後の文字も載せて返す（IPC は増えないが、開かない行の分も作る）
- D. Not yet defined（機能設計で決める）
- X. Other (please specify)

[Answer]: A

## Q4. 書き出しの操作は、画面にどう置きますか？（要件の OQ4）

ログの画面の一覧の上（絞り込みの欄のそば）と、クエリの画面の結果の表の上に置きます。選ぶのは「形式（CSV／Markdown）× 出し方（保存／コピー）」の 4 通りです。

- A. ［エクスポート］の 1 つのドロップダウン（`ButtonDropdown`）に 4 項目を並べる（おすすめ：コンソールの「結果をエクスポート」と同じ形。標準部品 1 つで済む）
- B. ［保存］と［コピー］の 2 つのドロップダウンに、それぞれ CSV／Markdown の 2 項目
- C. 4 つのボタンを並べる
- D. Not yet defined（機能設計で決める）
- X. Other (please specify)

[Answer]: A
