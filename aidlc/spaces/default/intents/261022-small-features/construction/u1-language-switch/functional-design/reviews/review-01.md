## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-24T10:26:30Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/functional-design/functional-spec.md > 流れ W2 手順 7 と Assumptions & Open Questions の 1 つ目の assumption | 要件 FR1.5 は「取得済みのログ・クエリの結果・開いた行・入力欄の文字・選んだ接続」を失わないことを求めているが、W2 手順 7 と BR1.3 はクエリの結果とクエリ欄の文字に触れていない。また assumption の逃げ道「崩れる部品は `key` で作り直す」は、その部品の中の状態（クエリの画面や開いた行など）を失わせ、FR1.5 と食い違う。`I18nProvider` を `App` の中に移すと、provider の位置や `locale` 依存の `useMemo` の作りによっては子が作り直されて状態を失うおそれもある。 | 保つ対象にクエリの結果・クエリ欄の文字・クエリの画面の状態を明記する。作り直しが必要な部品は、状態を親に持つか、作り直しの対象をダイアログやドロップダウンなど状態を持たない部品に限ると決める。「切り替えの前後で状態が残る」ことを確かめる自動テスト（AppShell）に、ログ表示とクエリ画面の両方を含める。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/functional-design/functional-spec.md > 流れ W2 手順 6・8、rules.md > BR1.6 | 今の `finish_settings_work` には、設定は書けたが保存後のキャッシュ消去が失敗した場合（`Saved { cleared: Some(false) }`）がある。このときキャッシュの有無も言語も新しい値になり、ダイアログは閉じず ClearFailed の知らせが出る。W2 手順 6 の「書けたら閉じる」と BR1.6 の「失敗なら言語は前の値のまま」はこの場合を扱っておらず、言語を変えたのにダイアログが開いたまま、新しい言語で ClearFailed を読む状態が仕様に現れない。 | この場合の言語の扱い（反映する、ダイアログは開いたまま ClearFailed）を W2 と BR1.3 / BR1.6 に 1 行足し、`finish_settings_work` のテストの 1 件にする。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/functional-design/rules.md > BR1.4 と entities.md > AppSettingsFile | BR1.4 は「言語の読みはキャッシュの項目と独立」と言うが、今の `load_settings` は型つきの構造体で一度に読むため、`cacheEnabled` が文字列など不正な値だとファイル全体が Invalid になり、有効な `language` も system に落ちる。「独立」の範囲が書かれていない。 | 独立なのは「language の欄がない・知らない値」のときだけで、ファイル全体が Invalid のときは言語も system とすると明記する。または `language` を別に拾う読み方に決め、その場合のテストを足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u1-language-switch/functional-design/functional-spec.md > 流れ W1、rules.md > BR1.6 | 設定ファイルの場所が決まらない（`cache_location` が None）場合、`begin_save_settings` は SaveFailed を載せるだけで仕事を返さない。この環境では言語を保存できず、起動時の読みも行われないため、言語の選択が常に失敗する。W1 と BR1.6 にこの場合の記述がない。 | 場所がない場合は「保存できない」の知らせを出し言語は system のままと、W1 と BR1.6 に明記する（既存の挙動を踏襲するなら、その旨だけでよい）。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| （この段階に検証ツールの指定なし） | 実行せず | 既存コード（`cache/settings.rs`、`session.rs` の SettingsWork / finish_settings_work、`main.tsx`、`SettingsDialog.tsx`、`lib.rs` の `save_settings`）と設計の主張を手で突き合わせた。`SettingsFile` に `deny_unknown_fields` がないこと（BR1.5 の前後互換）、`formatVersion` 1 のままで足せること、保存が一時ファイル → rename であること、`save_settings` を追加してもコマンドの集合が変わらないことは、コードと一致した。 |

### Summary

設計は、設定ファイル、AppSession、SessionView、AppShell をつなぐ経路が今のコードと一致し、実装者が迷う点は少ない。主な懸念は FR1.5 の状態保持の範囲（クエリ側と `key` による作り直しの逃げ道）で、これを確定すれば足りる。残りは保存の周辺の細部の取りこぼし（Critical なし、Major 1 件）。
