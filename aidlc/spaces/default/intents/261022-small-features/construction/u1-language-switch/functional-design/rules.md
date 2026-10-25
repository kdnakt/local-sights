# Business Rules — U1 言語切替（u1-language-switch）

上流：`inception/units-generation/unit-of-work.md`（U1 の責務・実装の注意）・`unit-of-work-story-map.md`（FR1.1〜FR1.6）、`inception/requirements-analysis/requirements.md`（FR1、NFR1・NFR3・NFR4・NFR5・NFR6、RA Q1・Q2、レビュー R-06）、`inception/domain-design/components.md`（AppSettings・AppSession・MessageCatalog・AppShell・SettingsDialog・TauriApp・ApiBridge）・`decisions.md`（ADR-005）、作業単位の分割のレビュー R-01、今のコード（`cache/settings.rs`、`session.rs` の SettingsWork、`src/main.tsx`、`SettingsDialog.tsx`）。

## 決めたこと（質問を設けずに決めた細部）

- 設定ファイルの値の綴りは `system` / `ja` / `en`。`formatVersion` は 1 のまま（entities.md）。
- 選択肢の表示名は、選択肢そのものの言語で固定する：「日本語」は常に日本語、「English」は常に英語。「OS の設定に合わせる」だけは表示の言語で訳す（英語では "Follow the OS setting"）。利用者が読めない言語に切り替えてしまっても、戻る選択肢を読めるようにするため。
- 設定が届く前の最初の描画は OS の言語で行う（今の `main.tsx` と同じ）。設定の言語が OS と違うときだけ、SessionView が届いた時点で一度描き直す（ADR-005）。
- OS の言語がアプリの起動中に変わっても追わない。次の起動で反映する。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 設定ダイアログに「言語」の選択を置き、「OS の設定に合わせる」「日本語」「English」の 3 つから 1 つを選ぶ。選択は［保存］で確定し、［キャンセル］と閉じるでは捨てる
    category: policy
    applies_to: SettingsDialog
    trigger: 設定ダイアログを開いたとき、選択を変えたとき、［保存］［キャンセル］を押したとき
    logic: 開いたときの初期値は SessionView.settings.language。選択を変えてもその場では何も起きない。［保存］を押すと、キャッシュの有無と言語を 1 回の保存としてアプリに渡す。［キャンセル］・Escape・閉じるボタンでは選択を捨て、言語は変わらない。「日本語」「English」の表示名はその言語で固定し、「OS の設定に合わせる」だけ表示の言語で訳す
    violation: なし
    source: FR1.1、RA Q1 A
  - id: BR1.2
    statement: 表示の言語は「設定の言語 × OS の言語」から 1 つに決め、画面のすべての文言がその 1 つを見る
    category: calculation
    applies_to: MessageCatalog、AppShell
    trigger: SessionView.settings.language が変わったとき、起動したとき
    logic: IF 設定が ja THEN ja。IF 設定が en THEN en。IF 設定が system THEN 今の detectLocale（OS の言語の並びの先頭が ja か ja- で始まれば ja、それ以外は en）。決めた言語を、翻訳（createTranslator）、Cloudscape の I18nProvider の locale と messages、リージョン名（regionLabel）、document.documentElement.lang に同時に渡す
    violation: なし（純粋な関数で、入力は常に有効）
    source: FR1.2、ADR-005
  - id: BR1.3
    statement: 保存に成功したときだけ、SessionView の言語を新しい値にし、画面は起動し直さずに切り替わる。選んだ値は次の起動でも使う
    category: policy
    applies_to: AppSession、AppSettings、AppShell
    trigger: 設定の保存の仕事（SettingsWork::Save）が終わったとき、起動したとき
    logic: 保存の仕事はキャッシュの有無と言語を 1 つのファイルに書く（今の一時ファイル → rename）。IF 書けた THEN AppSession は cache_enabled と language の両方を新しい値にし、SessionView を送る。画面は BR1.2 で言語を決め直し、取得済みのログ・開いた行・入力欄・選んだ接続・絞り込みの文字を保ったまま文言だけを入れ替える。起動時は AppSettings の読みの結果を language にする
    violation: 書けなかったときは BR1.6
    source: FR1.2、FR1.3、FR1.5、RA Q2 A
  - id: BR1.4
    statement: 設定ファイルの言語の読みはキャッシュの項目と独立で、言語の項目がない・知らない値のときは system とし、cacheEnabled は今までどおり読む
    category: validation
    applies_to: AppSettings
    trigger: 起動時に設定ファイルを読むとき
    logic: IF ファイルがない・読めない・JSON として読めない・formatVersion が 1 でない THEN キャッシュは無効（今のまま）、言語は system。IF 読めた AND language がない THEN cacheEnabled は書かれた値、言語は system。IF 読めた AND language が system/ja/en のどれでもない THEN cacheEnabled は書かれた値、言語は system（警告を標準エラーに 1 行出す。値そのものは出してよい：秘密ではない）
    violation: なし（どの場合も既定に落ちる）
    source: FR1.4、RA レビュー R-06、units レビュー R-01
  - id: BR1.5
    statement: 前の版の設定ファイル（language のないもの）はそのまま読め、今の版が書いたファイルは前の版のアプリでも読める
    category: constraint
    applies_to: AppSettings
    trigger: 起動時、保存時
    logic: formatVersion を 1 のままにし、language を省略できる項目として足す。読むときは知らない項目を無視する（今の読み方と同じ）。書くときは formatVersion・cacheEnabled・language の 3 つを書く
    violation: なし
    source: FR1.4、要件の制約（settings.json を広げて使う）
  - id: BR1.6
    statement: 保存に失敗したときは今の知らせ（SaveFailed）を出し、言語もキャッシュの設定も変えない
    category: policy
    applies_to: AppSession、SettingsDialog
    trigger: 設定の保存の仕事が失敗したとき（書けない、仕事のスレッドが落ちた）
    logic: SessionView の language と cache_enabled は前の値のまま。settingsNotice に SaveFailed を載せ、ダイアログは開いたままで知らせを出す（今と同じ）。画面の言語は変わらない
    violation: なし
    source: FR1.6
  - id: BR1.7
    statement: 言語の選択はキーボードで操作でき、読み上げの名前を持つ
    category: constraint
    applies_to: SettingsDialog
    trigger: 設定ダイアログを描くとき
    logic: 言語の Select は Tab で届き、矢印で選べ、ラベル「言語」を持つ。ダイアログの初回フォーカスは今のままチェックボックス
    violation: なし
    source: NFR6
```

## ルールの一覧

| ID | 区分 | 対象 | 要点 | 出典 |
|---|---|---|---|---|
| BR1.1 | policy | SettingsDialog | 「言語」の 3 択。［保存］で確定、［キャンセル］で捨てる。表示名は選択肢の言語で固定 | FR1.1 |
| BR1.2 | calculation | MessageCatalog、AppShell | 設定 × OS → 1 つの言語。翻訳・Cloudscape・リージョン名・lang 属性が同じ値を見る | FR1.2 |
| BR1.3 | policy | AppSession、AppSettings、AppShell | 保存成功で切り替え、起動し直さない、状態は保つ、次の起動でも使う | FR1.2、FR1.3、FR1.5 |
| BR1.4 | validation | AppSettings | 言語の読みはキャッシュと独立。ない・知らない値は system | FR1.4 |
| BR1.5 | constraint | AppSettings | formatVersion 1 のまま、前後の版と互換 | FR1.4 |
| BR1.6 | policy | AppSession、SettingsDialog | 保存失敗は SaveFailed、言語は変えない | FR1.6 |
| BR1.7 | constraint | SettingsDialog | キーボード操作と読み上げの名前 | NFR6 |
