## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-24T15:18:29Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/functional-spec.md > 流れ W1 手順 3、W5 手順 3、Assumptions & Open Questions（3 つ目） | 写しを取る作業が、session の錠（と LogView の錠）を持ったまま行われる設計になっている。ログの画面は 100 万行の複製と 3 列への変換、クエリの画面は全行分の `row_cells`（行ごとに JSON を読み直す）を、すべて錠の中で行う。W1 は「ここまで短時間」と書くが根拠がない。クエリの結果は `limit` を上げれば 10,000 行を超える。その間は session の錠を使うコマンドと、LogView の錠を使う取得の追加・絞り込みの走査が止まり、NFR2 と FR4.6.3（処理中も画面は応じる）に反する。Assumptions は「時間がかかる」とだけ書き、その間錠を持つことには触れていない。 | 錠の中ですることを、可否の確認・ExportJob の Running 化・exportNumber の加算・写しの元になる参照の取得だけに絞る。文字への変換（3 列への写し、`row_cells`）は錠を離したあとに行う。難しければ、錠を持つ時間が行数に比例することを明記して受け入れるかを、利用者が判断できるよう Assumptions に書く。W1 の「短時間」は根拠のある文に直す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/entities.md > ExportNotice の safeDetail、ExportJob の failureReason / rules.md > BR4.9 | BR4.9 と W6 手順 4 は「言語を切り替えると知らせの文も新しい言語で出す（鍵と引数で持つ）」と決めている。しかし ExportNotice の失敗側は自由な文字列 `safeDetail`（「OS のエラーの種類とパス」）しか持たない。機能設計の書きぶりでは、権限がない・空きがない・見つからない・その他を表す型付きの値がどこにもない（`failureReason` は DialogFailed などの別の分類）。spec が挙げる `export.failure.*` の鍵と結びつく項目がないため、保存先のエラーの種類の文は言語を変えても訳せない。OS のエラーの文を入れると、OS の言語の文が混じる。Code Generation で ExportNotice の形をやり直すことになる。 | ExportNotice に失敗の種類の enum（権限がない・空きがない・見つからない・その他・クリップボード）を足し、`export.failure.*` の鍵と一対一で対応させる。パスは別の項目として持ち、文は画面が鍵と引数（パス、行数）から組み立てると entities.md に明記する。`safeDetail` は残すなら補助の文字列に格下げし、画面に出さないか、出すなら言語が混じることを書く。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/rules.md > BR5.1（JSON の探し方） | 「最初に読み切れる JSON」の方針では、`[1234] {"event":"x"}` や `[200] {...}` のように、先頭に数字だけの `[...]` が付くログで、先頭の `[1234]` が完全な JSON の配列として選ばれる。実際に読みたい `{…}` は整形されず、切替だけが出る。Q4 A の方針どおりだが、利用者が受ける結果は期待と逆になる。また「文字列の中の `{` は候補に含めない」の文は、続く「前置きの中に `{` があれば次の候補に進む」と読みが食い違う。再帰の深さ、候補ごとの走査が失敗までかかる場合の長さの上限（NFR2）も書かれていない。 | 候補の選び方を 1 つに決めて書く（例：中身が数値などのスカラーだけの配列は候補にしない、オブジェクトを先に探す）。ほかの案にするなら Q4 に戻して利用者に確かめる。文字列の中の `{` の扱いの文を整理する。走査器は再帰を使わない（深さの上限を持つ）ことと、候補の試行回数または全体の長さの上限を決める。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/functional-spec.md > 中核とアプリの変更 > TauriApp（capabilities に `dialog:allow-save` と `clipboard-manager:allow-write-text`） | ダイアログとクリップボードは ExportBoundary が Rust 側から呼ぶ設計なのに、WebView 側（JS）から使うための権限を `capabilities/default.json` に足すことになっている。名前は実在するが、現在の default.json は「アプリ自身のコマンドと event だけ」と説明しており、権限を足すと WebView がプラグインを直接呼べるようになる。Rust 側の呼び出しには要らないため、設計の意図とも食い違う。 | Rust 側だけで呼ぶなら 2 つの権限は足さず、`allow-start-export` と `allow-dismiss-export-notice` の 2 つだけを 3 か所にそろえる。WebView から呼ぶ理由があるなら、その理由を書く。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/rules.md > BR5.5、BR5.4 / src/components/ExpandedMessage.tsx | BR5.5 は「切替は Tab で届き矢印で選べる」と書くが、Cloudscape の `SegmentedControl` が矢印キーで選べるかは確かめていない。また、今の ExpandedMessage は Tab で全文に入れるのは「スクロールする選択行」だけ（`focusable`）で、高さの計測も本文の要素だけを見ている。切替の高さを計測に含める方法（20 行の上限は本文だけにかかるか）と、仮想化された一覧の中で切替に Tab で届く条件が決まっていない。 | `SegmentedControl` のキーボード操作を確かめ、書く内容を実際の振る舞いに合わせる。切替の高さの計測の持ち方、切替に届く条件（選択行だけか）、クリックやキーが行の開閉に伝わらないこと（stopPropagation）を BR5.4 に足す。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/functional-spec.md > W5 手順 3（既定名の元） | 既定のファイル名の元を「最初に選んだロググループ名」としているが、クエリの画面では、結果を出したあとに選択を変えられる。どの時点の選択かが決まっていない（実行時の選択か、今の選択か）。 | 結果を出した実行の選択（結果が持つロググループ）から取ると決める。結果が持たないなら、今の選択から取ると書く。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/functional-design/functional-spec.md > 流れ W1（コマンドの形） | `start_export` が、保存ダイアログと書き込みの終わりまで待つ非同期コマンドか、受け付け後すぐ返すコマンドかが書かれていない（今の `run_settings_work` は待つ形、`run_query` は返す形）。画面側のエラーの受け方と、`spawn_blocking` の結果の戻し方が変わる。 | どちらかに決め、`run_settings_work` と同じ形にするか、`run_query` と同じ形にするかを書く。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 手作業の確認（FR2〜FR5 の 29 項目と traceability.json の突き合わせ） | 全項目に BR が対応 | OQ1〜OQ3 と domain-design の R-01・R-03・R-04 は rules.md「決めたこと」と BR2.1・BR4.5・BR4.8・BR5.1 で扱われている |
| 既存コードの確認 | `LogView::rows(0, total)`、`row_cells`、`run_settings_work`（`spawn_blocking`）、`SegmentedControl` の存在を確認 | 実装できる。`dialog:allow-save` と `clipboard-manager:allow-write-text` は実在する名前だが、Rust 側呼び出しでは不要 |

### Summary

流れ・状態機械・ルールは互いに整合し、FR2〜FR5 は漏れなく割り当てられ、既存のコードにも載せられる。気をつけたい点は 2 つの Major で、写しを取る処理が錠の中で行数に比例して長くなること、そして失敗の知らせが言語切替に対応できる形（鍵と引数）になっていないことである。どちらも Code Generation の前に直すと手戻りが少ない。Major は 2 件のため READY とする。
