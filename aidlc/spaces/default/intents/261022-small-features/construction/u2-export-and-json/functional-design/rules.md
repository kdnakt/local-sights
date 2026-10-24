# Business Rules — U2 書き出しと JSON の整形（u2-export-and-json）

上流：`inception/units-generation/unit-of-work.md`（U2 の責務・実装の注意）・`unit-of-work-story-map.md`（FR2〜FR5）、`inception/requirements-analysis/requirements.md`（FR2、FR3、FR4、FR5、NFR2、NFR3、NFR5、NFR6、AS3、AS6、OQ1〜OQ3）、`inception/domain-design/components.md`・`decisions.md`（ADR-001〜ADR-004、ADR-006）、ドメイン設計のレビュー R-01・R-03・R-04、質問票 `functional-design-questions.md`（Q1〜Q4）、今のコード（`log_view.rs` の `rows`、`session.rs`、`insights_session.rs`、`lib.rs` の `run_settings_work`、`ExpandedMessage.tsx`）。

## 決めたこと（質問を設けずに決めた細部）

- **写しの持ち方**：文字の写し（各セルを文字として複製する）。行への参照にすると、処理中に一覧が捨てられたときの扱いが複雑になるため。メモリを一時的に 2 重に持つことは ADR-004 で受け入れ済み。
- **写しを取る場所と順**：session の錠を取り、可否を確かめて ExportJob を Running にし、LogView の錠（ログの画面）か結果（クエリの画面）から行を写し、既定のファイル名の元（ロググループ名・時刻）を決めて、錠を離す。そのあとダイアログ → 書き込み。写し → 既定名 → ダイアログ → 書き込みの順（レビュー R-03）。
- **保存ダイアログの位置**：写しを取ったあと、文字を組み立てる前に出す。取り消したら文字を組み立てずに写しを捨てる（無駄な処理をしない）。
- **一時ファイルの名前**：保存先と同じフォルダに `<名前>.tmp-<乱数 16 桁>` を作って書き、終わったら保存先の名前に rename する（今の設定ファイルと同じ流儀）。書けなかったら一時ファイルを消す。
- **ファイルの拡張子**：保存ダイアログの既定名に `.csv` / `.md` を付け、拡張子のフィルタも同じにする。利用者が別の拡張子に変えてもそのまま書く。
- **日時の形**：既定のファイル名の日時は `yyyyMMdd-HHmmss`、選んでいるタイムゾーン（Local / UTC）で書く。
- **ファイル名に使えない文字**：`/ \ : * ? " < > |`、制御文字（U+0000〜U+001F、U+007F）を `_` に置き換え、先頭と末尾の空白と `.` を取り、空になったら `export` にする。ロググループ名の部分は 100 文字で切る（両 OS のパス長の制限に余裕を残す）。
- **知らせの出し方**：各画面の警告の領域（今の SSO ログイン成功の知らせと同じ場所）に閉じられる Alert で出す。成功は success、失敗は error。次の書き出しが受け付けられると前の知らせは消える。閉じる操作はコマンド（`dismiss_export_notice`）で中核の知らせを消す。
- **知らせの文の中身**：保存先のパスはそのまま出す（利用者自身のファイル）。失敗の原因は OS のエラーの種類（権限がない・空きがない・見つからない・その他）とパスで、秘密の認証情報は含まれない。
- **クリップボードの CSV**：BOM を付けない（FR4.2.3）。Markdown はどちらにも BOM を付けない。
- **Markdown の表の形**：`| a | b |` の形で、見出し行、`| --- | --- |` の区切り行、値の行。セルの中の `|` は `\|`、改行（CRLF・LF・CR のどれも）は `<br>`（Q2 A）。セルの前後に半角の空白 1 つ。空のセルは空白だけ。ファイルの最後は LF 1 つ。
- **CSV の形**：RFC 4180。コンマ・二重引用符・CR・LF を含むセルだけ二重引用符で括り、中の二重引用符は 2 つ重ねる。行の終わりは CRLF。最後の行のあとにも CRLF。
- **JSON の字下げ**：半角の空白 2 つ。`key: value` の `:` のあとに空白 1 つ。オブジェクトと配列は要素ごとに改行し、空の `{}` と `[]` はそのまま。字句（文字列の中身と逃がし、数値の表記、`true`/`false`/`null`）は 1 文字も変えない（FR5.4）。
- **JSON の探し方**：メッセージの先頭から `{` か `[` を探し、そこから JSON の値として読み切れるかを字句の走査で確かめる。読み切れなければ次の `{` か `[` から試す。最初に読み切れたものだけを整形し（Q4 A）、その後ろは走査しない。文字列の中の `{` は候補に含めない（走査は候補の位置から始めるため、前の文字の中身は見ない。前置きの中に `{` があって読み切れなければ次の候補に進む）。
- **切替の部品**：Cloudscape の `SegmentedControl`（「整形」「元のまま」の 2 択）を開いた表示の上に置く。初期値は「整形」。読み上げの名前を持つ（NFR6）。
- **アプリの終了中の書き出し**：書き出し中の終了を止めない（終了の確認は取得中だけ、今のまま）。終了で一時ファイルが残ることはあり得るが、まれで害が小さいため受け入れる（Assumptions に書く）。

## ルール（機械可読）

```yaml
rules:
  - id: BR2.1
    statement: ログの画面の写しは、絞り込みをかけているときは絞り込みの結果として一覧に出ている行だけ、かけていないときは取得済みの全行を、一覧と同じ順で取る
    category: calculation
    applies_to: AppSession、LogExporter
    trigger: ログの画面の書き出しの依頼を受け付けたとき
    logic: LogView の `rows(0, total_count)` と同じ読み方で、絞り込みが有効なら一致した行、無効なら全行を時刻順に写す。隠れている行は含めない
    violation: なし（写しは一覧の読み方そのまま）
    source: FR2.1、FR2.3
  - id: BR2.2
    statement: ログの画面の写しの列は `@timestamp`・`@logStream`・`@message` の 3 列で、時刻は一覧に表示している文字、メッセージは全文
    category: calculation
    applies_to: AppSession
    trigger: 写しを取るとき
    logic: 時刻は今の `display_rows` と同じ `format_display_in_zone`（選んでいるゾーン）の文字。ストリームは `logStreamName`。メッセージは `message` の全文（省略しない）。見出しの文字は言語に依存しない固定の 3 つ（Q1 A）
    violation: なし
    source: FR2.2、FR2.3、Q1 A
  - id: BR2.3
    statement: ログの画面の書き出しは、表示中の行が 0・取得中・絞り込みの走査中・書き出し中のどれかなら無効
    category: validation
    applies_to: AppSession、ExportMenu
    trigger: SessionView を作るとき、依頼を受け付けるとき
    logic: IF 表示中の行数（絞り込み中は一致した数）が 0 THEN 無効（NoRows）。IF phase が Fetching THEN 無効（Fetching）。IF 絞り込みの状態が Filtering THEN 無効（Filtering）。IF ExportJob が Running THEN 無効（Exporting）。そうでなければ有効。可否は `canExportLogs` として SessionView に載せ、メニューはそれを映す。依頼が来たときも同じ判断をもう一度行い、無効なら受け付けない
    violation: 無効なのに依頼が届いたとき（二重の操作・古い画面）は、エラーとして返し、状態は変えない（FR4.6.2）
    source: FR2.5、FR2.6、FR4.6.1、FR4.6.2
  - id: BR3.1
    statement: クエリの画面の写しは、表に出ている結果の列と行を、表と同じ順で取り、`@message` は全文、欠けた値は空の文字にする
    category: calculation
    applies_to: InsightsSession
    trigger: クエリの画面の書き出しの依頼を受け付けたとき
    logic: 列は `ResultSummary.columns`（`fields` の名前。指定がなければ `@timestamp`・`@message`）。行は結果の行を `sort`・`limit` の結果の順に全部。セルは今の `display_rows` と同じ `row_cells` の値で、`@timestamp` は選んでいるゾーンの文字、`@message` は 1 行目ではなく全文、値がない（None）セルは空の文字。表に出ていない項目は入れない
    violation: なし
    source: FR3.1、FR3.2、FR3.3、FR3.4
  - id: BR3.2
    statement: クエリの画面の書き出しは、結果がない・結果が 0 行・実行中・書き出し中のどれかなら無効
    category: validation
    applies_to: InsightsSession、ExportMenu
    trigger: InsightsView を作るとき、依頼を受け付けるとき
    logic: IF result がない THEN 無効（NoResult）。IF result.rowCount が 0 THEN 無効（NoRows）。IF phase が Fetching か Evaluating THEN 無効（Running。前回の結果を表示していても無効）。IF ExportJob が Running THEN 無効（Exporting）。可否は `canExport` として InsightsView に載せる。書き出し中かは AppSession の ExportJob から渡す
    violation: 無効なのに依頼が届いたときは、エラーとして返し、状態は変えない
    source: FR3.5、FR4.6.1、FR4.6.2
  - id: BR4.1
    statement: 書き出しの操作は、両画面とも 1 つの［エクスポート］メニューに「CSV で保存」「Markdown で保存」「CSV をコピー」「Markdown をコピー」の 4 項目を、この順で置く
    category: policy
    applies_to: ExportMenu
    trigger: 一覧を描くとき
    logic: ログの画面は一覧の上（絞り込みの欄のある行）、クエリの画面は結果の見出し（Header）の actions に置く。可否（BR2.3、BR3.2）が無効ならメニュー全体を無効にする。書き出し中は無効にしたうえで処理中を示す（ボタンの loading）。キーボードで開けて選べ、読み上げの名前「エクスポート」を持つ
    violation: なし
    source: FR4.1.1、FR4.1.2、FR4.1.3、FR4.6.1、NFR6、ADR-006
  - id: BR4.2
    statement: CSV は RFC 4180 の形で、1 行目が見出し、区切りはコンマ、行の終わりは CRLF、コンマ・二重引用符・改行を含むセルは二重引用符で括り中の二重引用符は 2 つ重ねる
    category: calculation
    applies_to: ExportFormatter
    trigger: 形式が csv のとき
    logic: 各セル：IF セルに `,`・`"`・CR・LF のどれかを含む THEN `"` で括り、中の `"` を `""` にする。ELSE そのまま。セルの中の改行はそのまま残す。行は CRLF で終え、最後の行のあとにも CRLF を付ける。セルの文字はそれ以外に加工しない（`=` で始まる値もそのまま、AS3）
    violation: なし（どの文字も表せる）
    source: FR4.2.1、FR4.2.2、FR4.1.4
  - id: BR4.3
    statement: ファイルに保存する CSV は UTF-8 で先頭に BOM を付け、クリップボードへコピーする CSV と Markdown には BOM を付けない
    category: policy
    applies_to: LogExporter
    trigger: 出力を作るとき
    logic: IF format が csv AND target が file THEN 先頭に U+FEFF を書く。ELSE 付けない。文字コードは常に UTF-8
    violation: なし
    source: FR4.2.3、FR4.3.3
  - id: BR4.4
    statement: Markdown は見出し行・区切り行・値の行からなる表で、セルの中の `|` は `\|` に、改行は `<br>` に置き換える
    category: calculation
    applies_to: ExportFormatter
    trigger: 形式が markdown のとき
    logic: 見出し行は `| 列名 | … |`、区切り行は `| --- | … |`（列の数だけ）、値の行は `| セル | … |`。各セル：`|` → `\|`、CRLF・LF・CR のどれも → `<br>`。セルの前後に半角の空白 1 つ。空のセルは空白だけ。行は LF で終え、最後の行のあとにも LF を付ける。それ以外は加工しない
    violation: なし
    source: FR4.3.1、FR4.3.2、FR4.1.4、Q2 A
  - id: BR4.5
    statement: 保存するファイルの既定の名前は `<ロググループ名を安全にした文字>_<yyyyMMdd-HHmmss>.<csv|md>` で、ロググループ名のファイル名に使えない文字は `_` に置き換える
    category: calculation
    applies_to: LogExporter
    trigger: 保存ダイアログを開く前
    logic: ロググループ名はログの画面では選んでいるもの、クエリの画面では最初に選んだもの。`/ \ : * ? " < > |` と制御文字を `_` に、先頭と末尾の空白と `.` を取り、空なら `export`、100 文字を超えたら 100 文字で切る。日時は写しを取った時刻を選んでいるタイムゾーン（Local / UTC）で `yyyyMMdd-HHmmss` に書く。拡張子は csv なら `.csv`、markdown なら `.md`。純粋な関数で作る
    violation: なし（どんな名前も安全な名前になる）
    source: FR4.4.1、FR4.2.4、FR4.3.3、Q3 A、レビュー R-03
  - id: BR4.6
    statement: ファイルへの保存は、OS の保存ダイアログで保存先を決め、同じフォルダの一時ファイルに書いてから保存先の名前に rename し、書けなかったときは一時ファイルを消して原因が分かる知らせを出す
    category: policy
    applies_to: ExportBoundary、LogExporter
    trigger: target が file のとき
    logic: 写しを取ったあと、既定の名前（BR4.5）と拡張子のフィルタで保存ダイアログを出す。IF 取り消された THEN 写しを捨て、ExportJob を Idle に戻し、何も書かず知らせも出さない。ELSE 保存先と同じフォルダに `<名前>.tmp-<乱数>` を作り、出力（BR4.2〜BR4.4）を 1 行ずつ書き、閉じてから保存先の名前に rename する。IF 作れない・書けない・rename できない THEN 一時ファイルを消し（消せなくても続ける）、ExportJob を Failed にし、原因（権限がない・空きがない・見つからない・その他）と保存先のパスを知らせに載せる。IF 書けた THEN ExportJob を Done にし、保存先のパスと行数を知らせに載せる
    violation: 失敗は Failed と知らせ。書きかけのファイルは残さない
    source: FR4.4.1、FR4.4.2、FR4.4.3、FR4.4.4
  - id: BR4.7
    statement: クリップボードへのコピーは、出力の文字を 1 つにまとめてクリップボードに渡し、終わったら行数を含む知らせを、失敗したらその旨の知らせを出す
    category: policy
    applies_to: ExportBoundary、LogExporter
    trigger: target が clipboard のとき
    logic: 出力（BR4.2、BR4.4。BOM なし）を 1 つの文字にし、クリップボードのプラグインに渡す。IF 渡せた THEN ExportJob を Done にし、行数を知らせに載せる。ELSE Failed にし、失敗の種類を知らせに載せる
    violation: 失敗は Failed と知らせ
    source: FR4.5.1、FR4.5.2
  - id: BR4.8
    statement: 書き出しはアプリ全体で同時に 1 つで、依頼を受け付けた時点で写しを取り、以後は写しだけから出力を作る。処理は錠を持たない blocking スレッドで走り、ほかの操作を止めない
    category: policy
    applies_to: AppSession、LogExporter、ExportBoundary
    trigger: 依頼を受け付けてから終わるまで
    logic: 依頼を受け付けると ExportJob を Running にして exportNumber を 1 増やし、写しを取り、前の知らせを消し、錠を離す。処理中に来た新しい依頼は受け付けない（BR2.3、BR3.2 の Exporting）。処理中も取得・絞り込み・画面の切り替え・言語の変更・接続の変更は止めず、写しがあるので結果は変わらない。終わったら錠を取り直し、exportNumber が一致する結果だけを ExportJob に反映し、SessionView を送る。スレッドが途中で落ちたら Failed（Interrupted）にする
    violation: 二重の依頼はエラーで返し、状態は変えない
    source: FR4.6.1、FR4.6.2、FR4.6.3、NFR2、ADR-004、レビュー R-01
  - id: BR4.9
    statement: 書き出しの結果の知らせは画面の警告の領域に閉じられる形で出し、利用者が閉じるか次の書き出しが受け付けられるまで残る
    category: policy
    applies_to: AppShell、AppSession
    trigger: ExportJob が Done か Failed になったとき、利用者が知らせを閉じたとき、次の依頼を受け付けたとき
    logic: Done なら成功の知らせ（Saved：保存先のパスと行数／Copied：行数）、Failed なら失敗の知らせ（SaveFailed／CopyFailed：原因の文）。知らせは両画面の警告の領域に同じものを出す。閉じるとコマンドで中核の知らせを消す。言語を切り替えると知らせの文も新しい言語で出す（鍵と引数で持つ）
    violation: なし
    source: FR4.4.4、FR4.5.1、FR4.5.2、FR4.4.2
  - id: BR5.1
    statement: 開いた行のメッセージから、最初に読み切れる JSON のオブジェクトか配列を 1 つ見つけ、前と後ろの文字はそのまま残す。読み切れるものがなければ見つからなかったとする
    category: calculation
    applies_to: JsonPrettyPrinter
    trigger: 行を開いて全文を描くとき
    logic: 先頭から `{` か `[` を探し、その位置から JSON の値（RFC 8259）として字句を走査して読み切れるか確かめる。IF 読み切れた THEN そこまでを JSON とし、前を prefix、後ろを suffix として返す（後ろは走査しない、Q4 A）。ELSE 次の `{` か `[` から試す。候補がなくなれば null。空白だけの prefix / suffix もそのまま残す。`JSON.parse` は使わない
    violation: なし（壊れた JSON は null になり、元の文字がそのまま出る、FR5.3）
    source: FR5.1、FR5.2、FR5.3、Q4 A、ADR-003
  - id: BR5.2
    statement: 整形は字句を保ち、空白・改行・字下げだけを足す。キーの順、文字列の中身と逃がし、数値の表記、`true`/`false`/`null` は 1 文字も変えない
    category: calculation
    applies_to: JsonPrettyPrinter
    trigger: BR5.1 で JSON が見つかったとき
    logic: 字下げは半角の空白 2 つ。オブジェクトは `{` のあとで改行し、`"key": value` を 1 行ずつ `,` で区切り、閉じる前に改行して 1 段戻す。配列も同じ。空の `{}` と `[]` はそのまま。文字列の字句（引用符から引用符まで、逃がしを含む）、数値の字句（`1.0`、`1e5`、桁の大きい整数）、`true`/`false`/`null` は入力の文字をそのまま写す。元の JSON の中の空白と改行は捨てる
    violation: なし
    source: FR5.4、ADR-003、要件レビュー R-05
  - id: BR5.3
    statement: JSON が見つかった行には「整形／元のまま」の切替を置き、初期値は「整形」、切替は行ごとに持ち、行を閉じて開き直すと初期値に戻る
    category: policy
    applies_to: ExpandedMessage
    trigger: 行を開いたとき、切替を押したとき、行を閉じたとき
    logic: IF JsonSegments が null THEN 切替を出さず、今までどおり元の文字を描く（FR5.6）。ELSE 切替（SegmentedControl、2 択）を全文の上に出し、「整形」なら prefix + pretty + suffix、「元のまま」なら元のメッセージを描く。状態は開いた行の部品の中に持ち、閉じると捨てる
    violation: なし
    source: FR5.5、FR5.6
  - id: BR5.4
    statement: 整形した表示でも、今の全文の表示の振る舞い（文字ノードだけで描く、20 行までの高さと内側のスクロール、Tab と Escape、読み上げの名前、選択とコピー）は変えず、コピーされる文字は表示している文字
    category: constraint
    applies_to: ExpandedMessage
    trigger: 開いた行を描くとき
    logic: 整形した文字も元の文字も、1 つの文字ノードとして描く（HTML として解釈しない）。切替の部品の高さは開いた行の高さの計測に含める。Tab で全文に入り Escape で戻る操作、`aria-label` の名前、20 行の上限と内側のスクロールは今のまま。文字を選んでコピーすると、表示中の文字（整形なら整形後）がコピーされる
    violation: なし
    source: FR5.7、FR5.8
  - id: BR5.5
    statement: 書き出しのメニュー・知らせ・JSON の切替はキーボードで操作でき、読み上げの名前を持つ
    category: constraint
    applies_to: ExportMenu、AppShell、ExpandedMessage
    trigger: 描くとき
    logic: メニューは Tab で届き、Enter と矢印で開いて選べる。知らせは `role="status"`（成功）か `role="alert"`（失敗）の領域に出し、閉じるボタンに名前を付ける。切替は Tab で届き矢印で選べ、名前「表示」（整形／元のまま）を持つ
    violation: なし
    source: NFR6
```

## ルールの一覧

| ID | 区分 | 対象 | 要点 | 出典 |
|---|---|---|---|---|
| BR2.1 | calculation | AppSession、LogExporter | ログの画面の写しは絞り込み後の表示中の行を一覧の順で | FR2.1、FR2.3 |
| BR2.2 | calculation | AppSession | 3 列 `@timestamp`・`@logStream`・`@message`、時刻は表示の文字、メッセージ全文 | FR2.2、FR2.3 |
| BR2.3 | validation | AppSession、ExportMenu | 0 行・取得中・走査中・書き出し中は無効 | FR2.5、FR2.6、FR4.6 |
| BR3.1 | calculation | InsightsSession | 結果の列と行を表の順で、`@message` 全文、欠けは空 | FR3.1〜FR3.4 |
| BR3.2 | validation | InsightsSession、ExportMenu | 結果なし・0 行・実行中・書き出し中は無効 | FR3.5、FR4.6 |
| BR4.1 | policy | ExportMenu | 1 つのメニューに 4 項目、可否と処理中を映す | FR4.1、ADR-006 |
| BR4.2 | calculation | ExportFormatter | CSV は RFC 4180、CRLF、括りと重ね | FR4.2 |
| BR4.3 | policy | LogExporter | ファイルの CSV だけ BOM | FR4.2.3 |
| BR4.4 | calculation | ExportFormatter | Markdown の表、`\|` と `<br>` | FR4.3、Q2 |
| BR4.5 | calculation | LogExporter | 既定名 `<安全なロググループ名>_<yyyyMMdd-HHmmss>.<ext>` | FR4.4.1、Q3 |
| BR4.6 | policy | ExportBoundary | ダイアログ → 一時ファイル → rename、取り消しは無言、失敗は原因と後始末 | FR4.4 |
| BR4.7 | policy | ExportBoundary | コピーは行数の知らせ、失敗はその旨 | FR4.5 |
| BR4.8 | policy | AppSession、LogExporter、ExportBoundary | 同時に 1 つ、写しから出力、blocking でほかを止めない | FR4.6、ADR-004 |
| BR4.9 | policy | AppShell、AppSession | 知らせは閉じられる Alert、次の書き出しで消える | FR4.4.4、FR4.5 |
| BR5.1 | calculation | JsonPrettyPrinter | 最初に読み切れる JSON 1 つ、前後はそのまま | FR5.1〜FR5.3、Q4 |
| BR5.2 | calculation | JsonPrettyPrinter | 字句を保って空白だけ足す | FR5.4 |
| BR5.3 | policy | ExpandedMessage | 切替、初期値「整形」、行ごと、開き直すと戻る | FR5.5、FR5.6 |
| BR5.4 | constraint | ExpandedMessage | 今の振る舞いは変えない、コピーは表示の文字 | FR5.7、FR5.8 |
| BR5.5 | constraint | ExportMenu、AppShell、ExpandedMessage | キーボードと読み上げ | NFR6 |
