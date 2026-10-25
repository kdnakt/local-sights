# Unit of Work — アプリ内の言語切替・書き出し・JSON の整形（261022-small-features）

上流：`inception/domain-design/components.md`（新規 5・変更 9 の部品）・`decisions.md`（ADR-001〜ADR-006）、`inception/requirements-analysis/requirements.md`（FR1〜FR5、NFR1〜NFR6）、質問票 `units-generation-questions.md`（Q1〜Q3、分け方の計画の承認）。

分け方は 2 つ（Q3 B）：最初の一本として言語切替を独立させ（Q2 A）、書き出しと JSON の整形を 1 つにまとめる。どの作業単位も今の 1 つのデスクトップアプリ（中核ライブラリ＋Tauri アプリ＋画面）に足し、作業単位ごとに別々に配ることはない（team.md Deployment：この作業は次の版に合わせて配る）。どれから作るかの経済的な順番は作る段階の計画で決めるが、依存の形（U2 → U1）から U1 が先になる。

最初の作業単位 U1 は、team.md の Walking Skeleton にならい、新しい振る舞いが画面から端（設定ファイル）まで通る薄い一本にする。この作業では AWS を新しく呼ばないため、確かめ方の「実際の AWS に対して取得する確認用プログラム」はあてはまらず、自動テストと GUI の目視で確かめる。

## 作業単位の一覧

| Unit ID | Directory | 名前 | 種類 | 配置 | 大きさ | 主な要件 |
|---------|-----------|------|------|------|--------|----------|
| U1 | u1-language-switch | 言語切替（薄い一本） | service | アプリに組み込み | S | FR1 |
| U2 | u2-export-and-json | 書き出しと JSON の整形 | service | アプリに組み込み | L | FR2、FR3、FR4、FR5 |

## U1 言語切替（u1-language-switch）

- **説明**：設定ダイアログで言語（OS に合わせる／日本語／English）を選んで保存すると、起動し直さずに画面の言葉が切り替わり、次の起動でも覚えている。設定ファイル → 中核 → 画面の経路を一通り通る薄い一本。
- **責務**：
  - AppSettings：今の `cache/settings.rs` を広げ、`language` を読み書きする。言語の値はキャッシュの項目と独立に読み、知らない値・欠けた値・ファイルがない・読めない・形式が違うときは「OS に合わせる」とし、`cacheEnabled` は今までどおり読む（FR1.3、FR1.4、ADR-005）
  - AppSession：設定の状態に言語を足して `SessionView` に載せる。保存の失敗は今の知らせを使い言語は変えない（FR1.2、FR1.6）。取得済みのログ・開いた行・入力・接続は変えない（FR1.5）
  - MessageCatalog：「設定 × OS の言語」から表示の言語を決める純粋な関数と、言語の選択肢の文言（FR1.1）。テストを先に書く
  - AppShell：`SessionView` の言語で翻訳・Cloudscape の文言・`lang` 属性をまとめて切り替える。最初の描画は OS の言語（FR1.2、ADR-005）
  - SettingsDialog：「言語」の Select を足し、［保存］でキャッシュの設定と一緒に渡す（FR1.1）
  - TauriApp・ApiBridge：`save_settings` に言語を足す。コマンドの登録は 3 か所でそろえる
- **含む部品**：AppSettings、AppSession、MessageCatalog、AppShell、SettingsDialog、TauriApp、ApiBridge
- **確かめ方**（team.md Walking Skeleton の 3 つのうち、この作業にあてはまるもの）：`cargo test` と Vitest がすべて通る（各 OS のランナー）。GUI で言語を切り替え、起動し直さずに画面が切り替わること、起動し直しても覚えていること、前の版の `settings.json` でも起動することを目で確かめる。共通のコードを触るため、macOS と Windows の両方で確かめる。AWS に対する確認用プログラムはこの作業単位にはあてはまらない（AWS を呼ばない）。
- **実装の注意**：
  - `formatVersion` を上げるかどうかは機能設計で決める（前の版のファイルが読めることが条件）。
  - U2 はこの作業単位が広げる `save_settings` と `SessionView` の設定の形を前提にする。
  - 先にこの作業単位が入ると、U2 の英語の画面の確認を起動し直さずに行える（Q2 A）。

## U2 書き出しと JSON の整形（u2-export-and-json）

- **説明**：ログの画面の表示中のログと、クエリの画面の実行結果を、CSV と Markdown で、ファイルへの保存とクリップボードへのコピーで持ち出せるようにする。あわせて、行を開いたときの全文の表示で JSON を字句を保って整形し、「整形／元のまま」を切り替えられるようにする。
- **責務**：
  - ExportFormatter（中核、純粋）：CSV（RFC 4180、BOM の有無は呼び出し側）と Markdown（表、`|` の逃がし、改行の置き換え）の組み立て。欠けた値は空のセル（FR4.2、FR4.3）。テストを先に書く
  - LogExporter（中核）：要求を受けた時点で表示中の行の写し（ExportSnapshot）を確定し、写しだけから出力を作る（ADR-004）。既定のファイル名（ロググループ名と日時、両 OS で安全な文字に置き換え）を純粋な関数で作る。ExportJob の状態（Idle / Running / Done / Failed）。行数の上限なし（FR2.4、FR3.4、FR4.4.1、FR4.6）。写し → 既定名 → ダイアログ → 書き込みの順。書き出しはアプリ全体で同時に 1 つ
  - ExportBoundary（アプリ）：`tauri-plugin-dialog` と `tauri-plugin-clipboard-manager` で保存とコピー。一時ファイル → rename、取り消しは何もしない、失敗は理由を返す（FR4.4、FR4.5、ADR-002）。blocking スレッドで走らせ、状態を `session-changed` で押す
  - AppSession：ログの画面の写し（表示中の行の 3 列、時刻は表示の文字、メッセージは全文）と書き出しの可否（0 行・取得中・絞り込みの走査中・書き出し中は無効）。ExportJob の実体を持ち、`SessionView` に載せる（FR2.1〜FR2.3、FR2.5、FR2.6、FR4.6）
  - InsightsSession：クエリの画面の写し（表の列、`@message` は全文、欠けた値は空）と可否（0 行・実行中・結果なし・書き出し中は無効）。書き出し中かは AppSession の ExportJob から読む（FR3、FR4.6）
  - TauriApp・ApiBridge：書き出しのコマンド（画面・形式・出し方）、プラグインの登録と `capabilities` の権限、型の追加。コマンドの登録は 3 か所でそろえる
  - ExportMenu（画面）：［エクスポート］の `ButtonDropdown` 1 つに 4 項目。両画面で同じ部品。可否を映し、処理中を示す（FR4.1、FR4.6、ADR-006）
  - AppShell：書き出しの知らせ（保存先・行数・失敗）の表示（FR4.4.4、FR4.5）
  - JsonPrettyPrinter（画面側、純粋）：JSON の範囲の検出と、字句を保った整形（`1.0`・桁の大きい整数・キーの順・逃がしを変えない）。壊れた JSON は見つからなかったとして返す（FR5.1〜FR5.4、ADR-003）。テストを先に書く
  - ExpandedMessage：整形した表示と「整形／元のまま」の切替（行ごと、開き直すと初期値）。JSON がなければ今までどおり。今の振る舞い（選択とコピー、高さ、キー操作、読み上げ）は変えない（FR5.5〜FR5.8）
  - 新しい文言（MessageCatalog）：書き出しのメニューと知らせ、JSON の切替
- **含む部品**：ExportFormatter、LogExporter、ExportBoundary、AppSession、InsightsSession、TauriApp、ApiBridge、ExportMenu、AppShell、JsonPrettyPrinter、ExpandedMessage、MessageCatalog（文言の追加のみ）
- **確かめ方**：`cargo test` と Vitest がすべて通る（各 OS のランナー）。純粋なロジック（CSV・Markdown の組み立て、ファイル名、JSON の検出と整形、写しの確定）はテストを先に書く。GUI で、ログの画面とクエリの画面の両方から 4 通りの書き出しを行い、保存したファイルを Excel で開いて読めること、コピーした Markdown を貼れること、`/` を含むロググループ名で保存できること、JSON のメッセージが整形されて切替で元に戻ることを目で確かめる。新しい依存（公式プラグイン）と OS の境界を触るため、macOS と Windows の両方で確かめる。`cargo-deny` と `npm audit` が通る。
- **実装の注意**：
  - 要件の OQ1（JSON とみなす条件の細部）、OQ2（Markdown の改行の置き換え）、OQ3（既定のファイル名の形）と、ドメイン設計のレビューで残った点（R-01 写しと状態の持ち主：各セッションが列と行を渡し LogExporter が写しを確定、ExportJob は AppSession が 1 つ持つ、書き出しはアプリ全体で同時に 1 つ／R-02 言語の値の読みはキャッシュの項目と独立／R-03 写し → 既定名 → ダイアログ → 書き込みの順と取り消し時の写しの破棄／R-04 LanguageSetting は AppSettingsFile の実行時の写し）は、この作業単位の機能設計で決めて書く。
  - 書き出し中に取得や絞り込みを始めることは止めない（ADR-004）。写しの持ち方（文字のコピーか参照か）は機能設計で決める。
  - スプレッドシートの式として解釈される値の加工はしない（要件 AS3）。

## Assumptions & Open Questions

- [assumption] U1 で広げる `save_settings` の引数と `SessionView` の設定の形は、U2 の機能設計までに固まっている（U2 → U1 の依存の根拠）。
- U2 の機能設計で決める項目は上の「実装の注意」のとおり。
