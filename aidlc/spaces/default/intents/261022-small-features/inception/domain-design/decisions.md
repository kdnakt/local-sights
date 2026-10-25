# Architecture Decision Records — 261022-small-features

上流：`inception/requirements-analysis/requirements.md`、`codekb/local-sights/architecture.md`、`codekb/local-sights/component-inventory.md`、`domain-design-questions.md`（Q1〜Q4）。部品の一覧は `components.md`。

## ADR-001: 書き出しの組み立ては中核ライブラリに置き、Tauri の境界が書く

- **Status**: Accepted
- **Context**: 書き出す行はすべて中核が持つ（LogView の表示中の行、InsightsSession の結果）。行数に上限を置かない（FR2.4、FR3.4）ため 100 万行になりうる。チームの決まりは「判断は中核に、OS に触れる処理は境界に、画面は表示だけ」（team.md Code Style、codekb architecture）。純粋なロジックはテスト先行（team.md Testing Posture）。
- **Decision**: CSV と Markdown の組み立て（ExportFormatter）と書き出しの流れ（LogExporter：写し・ファイル名・状態）を中核に置く。ファイル書き込みとクリップボードはアプリ側の ExportBoundary が担う。画面は操作（ExportMenu）と状態の表示だけを持つ（Q1 A）。
- **Consequences**:
  - 良い点：行を IPC で運ばない。組み立ては `cargo test` で守れる。両画面で同じ流れを使える。
  - 悪い点：Rust 側のコード（モジュール、コマンド、状態）が増える。画面側だけで済む案より手数が多い。
  - 中立：状態の運び方は今の `session-changed` に乗せるので、新しいイベントは要らない。
- **Alternatives Rejected**:
  - 画面側（TypeScript）で全部（Q1 B）：行を全部 IPC で運ぶ必要があり、100 万行で時間とメモリがかさむ。チームの層の分け方にも合わない。
  - 中核で組み立て、コピーだけ `navigator.clipboard`（Q1 C）：組み立てた文字を IPC で画面へ戻す往復が要り、ファイルとコピーで流れが分かれる。

## ADR-002: 保存ダイアログとクリップボードは Tauri の公式プラグインで実装する

- **Status**: Accepted
- **Context**: 保存ダイアログとクリップボードは OS に触れる処理で、macOS と Windows で同じに動く必要がある（NFR3）。今のアプリは公式プラグインを使っていない。依存は `cargo-deny` と `npm audit` で確かめる決まり（team.md Code Style）。
- **Decision**: `tauri-plugin-dialog`（保存ダイアログ）と `tauri-plugin-clipboard-manager`（クリップボード）を入れ、権限を `capabilities/default.json` に足す（Q2 A）。ExportBoundary にはこの 2 つの呼び出しだけを置き、`cfg` の OS 分岐は置かない。
- **Consequences**:
  - 良い点：両 OS の差をプラグインが吸収する。権限が設定ファイルで見える。Tauri 2 の流儀に沿う。
  - 悪い点：依存が 2 つ増え、`cargo-deny` の対象が広がる。プラグインの版の更新が Dependabot に載る。
  - 中立：クリップボードへの大きな文字の受け渡しの振る舞いはプラグインに依存する（components.md の前提）。
- **Alternatives Rejected**:
  - `rfd` と `arboard` を直接使う（Q2 B）：プラグインの権限の仕組みを通らず、WebView と OS の窓の親子関係を自分で扱う必要がある。
  - ダイアログはプラグイン、コピーはブラウザの API（Q2 C）：組み立てた文字を画面へ戻す往復が要り（ADR-001 の却下案と同じ）、WebView のクリップボード権限に依存する。

## ADR-003: JSON の検出と整形は画面側の純粋な TypeScript に置き、字句を保って整形する

- **Status**: Accepted
- **Context**: 整形するのは行を開いたときの全文の表示だけ（FR5）。メッセージの文字はすでに画面側にある。表記を変えてはいけない（FR5.4：`1.0`、桁の大きい整数、キーの順、逃がし。要件のレビュー R-05）。切替は行ごとの画面の状態（FR5.5）。
- **Decision**: JsonPrettyPrinter を `src/` の純粋な補助として置き、Vitest でテスト先行で書く（Q3 A）。整形は `JSON.parse` で読み直さず、字句（文字列・数値・記号）をそのまま保って空白・改行・字下げだけを足す走査器にする。結果は「前・JSON・後」の並びで返し、ExpandedMessage が切替つきで描く。
- **Consequences**:
  - 良い点：IPC を増やさない。表記が変わらないことをテストで確かめられる。表示だけの変更なので中核に影響しない。
  - 悪い点：JSON の字句解析を自分で書く（小さいが正しさの確認が要る）。同じ整形を Rust 側で使いたくなっても共有できない。
  - 中立：JSON とみなす条件の細部（OQ1）は機能設計で決める。
- **Alternatives Rejected**:
  - 中核（Rust）で整形してコマンドで返す（Q3 B）：行を開くたびに IPC が 1 回増える。表示だけの機能に中核を触る理由がない。
  - 中核で整形して `DisplayRow` に載せる（Q3 C）：開かない行の分も作り、`get_rows` の応答が重くなる。
  - `JSON.parse` → `JSON.stringify(…, 2)`：実装は短いが、数値の表記・桁の大きい整数・逃がしが変わり FR5.4 を満たせない。

## ADR-004: 書き出しは操作した時点の写しから出力し、処理中もほかの操作を止めない

- **Status**: Accepted
- **Context**: 書き出しの処理中も画面はスクロールと画面の切り替えに応じる（FR4.6.3）。一方で、処理中に絞り込み・再取得・接続や言語の変更が起きたとき何が書かれるかが決まっていなかった（要件のレビュー R-01）。行数に上限がなく（FR2.4）、処理が数秒以上かかりうる。
- **Decision**: LogExporter は要求を受けた時点で表示中の行の写し（ExportSnapshot）を確定し、以後は写しだけから出力を作る。処理は blocking スレッドで走らせ、書き出しの操作だけを無効にし（FR4.6.1、FR4.6.2）、取得・絞り込み・接続や言語の変更は止めない。写しは 1 つの ExportJob につき 1 つで、終わったら捨てる。
- **Consequences**:
  - 良い点：処理中に一覧が変わっても出力は押した時点の内容で、QA が確かめられる。利用者を待たせない。
  - 悪い点：写しの分だけメモリを一時的に 2 重に持つ（100 万行のメッセージの文字）。
  - 中立：写しの持ち方（文字のコピーか、行への参照か）は機能設計と実装で決める。メモリの上限は置かない（Q3 A の答えに従う）。
- **Alternatives Rejected**:
  - 処理中は一覧を変える操作をすべて無効にする：写しが要らないが、利用者を待たせ、無効にする操作の範囲（接続の変更、画面の切り替え、言語）が広く漏れやすい。
  - 写しを取らず一覧を直接読む：一覧が変わると出力が途中で混ざり、何が書かれたか説明できない。

## ADR-005: 言語の設定は中核の設定ファイルに持ち、SessionView で画面に配る

- **Status**: Accepted
- **Context**: 言語は次の起動でも覚える（FR1.3）。設定ファイルは今の `settings.json` を広げて使う（要件の制約）。「状態は中核が持ち、画面は映す」（codekb architecture）。設定が読めないときや読み込み前の最初の描画の言語を決める必要がある（要件のレビュー R-06）。
- **Decision**: AppSettings（今の CacheSettings を広げる）が `language` を読み書きし、AppSession が設定の状態として `SessionView` に載せる。画面の MessageCatalog が「設定 × OS の言語」から表示の言語を決める純粋な関数を持ち、AppShell が翻訳・Cloudscape の文言・`lang` 属性をまとめて切り替える。設定がない・読めない・形式が違う・項目がない・知らない値のときは「OS に合わせる」。最初の描画は `SessionView` が届くまで OS の言語で行う。
- **Consequences**:
  - 良い点：設定の置き場所が 1 つで、キャッシュの設定と同じ流れ（読み・保存・失敗の知らせ）を使える。画面はどこでも同じ言語を見る。
  - 悪い点：最初の描画のあと設定の言語が届くと一度描き直す（OS と設定が違うときだけ）。
  - 中立：`formatVersion` を上げるかどうかは機能設計で決める（前の版のファイルは読めることが条件）。
- **Alternatives Rejected**:
  - 画面側の `localStorage` に持つ：設定ファイルを広げる制約に合わず、設定が 2 か所に分かれる。
  - 専用のコマンドで言語だけ別に読み書きする：`save_settings` の流れと知らせを 2 重に持つことになる。

## ADR-006: 書き出しの操作は 1 つのドロップダウンに 4 項目を並べる

- **Status**: Accepted
- **Context**: 選ぶのは形式（CSV／Markdown）× 出し方（保存／コピー）の 4 通り（FR4.1.1、FR4.1.2）。画面は標準部品のみ（project.md）。コンソールの「結果をエクスポート」に合わせる（intent-statement）。要件の OQ4。
- **Decision**: ExportMenu として `ButtonDropdown` 1 つに 4 項目を置き、ログの画面とクエリの画面の 2 か所で同じ部品を使う（Q4 A）。
- **Consequences**:
  - 良い点：コンソールと同じ形で、標準部品 1 つで済む。2 画面で重複しない。
  - 悪い点：1 回の操作が 2 クリックになる。
  - 中立：項目の文言と並び順は機能設計で決める。
- **Alternatives Rejected**:
  - ［保存］と［コピー］の 2 つのドロップダウン（Q4 B）：部品が 2 つになり、横幅を取る。
  - 4 つのボタン（Q4 C）：一覧の上の領域を占め、絞り込みの欄と競合する。

## Assumptions & Open Questions

None.
