# Business Rules — U6 ディスクキャッシュ（u6-disk-cache）

上流の成果物：`inception/requirements-analysis/requirements.md`（U6 の FR：FR7.1〜FR7.9、NFR5、NFR8、NFR15）、`inception/units-generation/unit-of-work.md`、`inception/domain-design/components.md`、`ideation/rough-mockups/wireframes.md`（画面 6）、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q5。

U1〜U5 のルールは、ここで置き換えると書いたもの以外はそのまま有効。前の単位のルールは `U3:BR4.3` のように作業単位名を前に付けて書く。前に何も付けない `BRx.y` は、この文書（U6）のルールを指す。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 設定ファイルには有効・無効の 1 項目だけを保存し、次の起動でも覚えておく
    category: policy
    applies_to: CacheSettings
    trigger: 設定ダイアログで [Save] を押したとき、アプリの起動時
    logic: "[Save] で cacheEnabled と書式の版だけを settingsPath に書く。起動時に読み、SessionState.cacheEnabled に写す。プロファイル名・リージョン・ロググループ・認証情報は書かない（U2 の「前回の選択は覚えない」はそのまま）"
    violation: なし
    source: Q1、FR7.1、project.md Forbidden、U2 の機能設計の Q2
  - id: BR1.2
    statement: 設定ファイルがない・読めない・解釈できないときは、無効として扱う
    category: validation
    applies_to: CacheSettings
    trigger: 起動時に設定ファイルを読むとき
    logic: IF ファイルがない OR 読めない OR 解釈できない OR 知らない書式の版 THEN cacheEnabled = false として起動する。知らせは出さず、診断ログ（標準エラー出力）にだけ残す。ファイルは書き換えない（次に [Save] を押したときに書き直す）
    violation: なし
    source: FR7.1（既定は無効）、NFR8
  - id: BR1.3
    statement: "[Save] で無効にしたら、設定を書いたあとに保存済みのキャッシュをすべて消す"
    category: policy
    applies_to: LogCache
    trigger: 設定ダイアログで [Save] を押したとき
    logic: "設定を settingsPath に書く。書けたら SessionState.cacheEnabled を更新してダイアログを閉じる。IF 有効から無効に変わった THEN 続けて BR1.4 と同じ全削除を行う。設定が書けなかったときは、ダイアログを開いたまま誤りを出し、cacheEnabled は前のままにする。削除に失敗したときは、設定は無効のまま、ダイアログに削除できなかったことを出す"
    violation: 設定を書けない（画面に誤りを出す）、キャッシュを消せない（画面に出す）
    source: Q4、FR7.6、FR7.7
  - id: BR1.4
    statement: "[Clear cache] は、その場で保存済みのキャッシュをすべて消す"
    category: policy
    applies_to: LogCache
    trigger: 設定ダイアログで [Clear cache] を押したとき
    logic: "cacheDirectory の中のこのアプリのキャッシュファイルをすべて消す（書きかけの一時ファイルも含む）。[Save]・[Cancel] を待たず、[Cancel] で取り消せない。終わったら、ダイアログに消したこと、または消せなかったことを出す。有効・無効の状態は変えない。アプリ専用フォルダの外は消さない"
    violation: 消せない（画面に出す）
    source: FR7.6、wireframes 画面 6
  - id: BR1.5
    statement: 無効のときは、キャッシュを読みも書きもしない
    category: constraint
    applies_to: FetchCoordinator
    trigger: 取得を始めるとき、取得が終わったとき
    logic: IF cacheEnabled = false THEN キャッシュを引かずに AWS から取得し（U3 のまま）、取得結果をディスクに書かない。cacheOutcome = NotUsed
    violation: なし
    source: FR7.7、NFR8、Q4

  - id: BR2.1
    statement: キャッシュは（プロファイル・リージョン・ロググループ）の組み合わせで引く
    category: calculation
    applies_to: CacheKey
    trigger: 取得を始めるとき
    logic: 選択中の ConnectionProfile（SdkDefault ならその印、Named ならプロファイル名）、リージョンのコード、ロググループの名前の 3 つで CacheKey を作る。3 つがすべて一致するものだけを同じキャッシュとみなす。AWS のアカウントは調べない（読み取り 3 API 以外を呼ばないため）
    violation: なし
    source: FR7.4、project.md Forbidden
  - id: BR2.2
    statement: キャッシュのファイル名はキーのハッシュにし、キー自体はファイルの中で確かめる
    category: constraint
    applies_to: LogCache
    trigger: キャッシュのファイルを決めるとき
    logic: ファイル名は CacheKey から求めたハッシュ（ロググループ名などの文字をファイル名に使わない）。読むときは、ファイルの中のキーが引いたキーと一致することを確かめ、一致しなければ BR4.1 の壊れたものとして扱う
    violation: なし
    source: FR7.3、BR4.1
  - id: BR2.3
    statement: 指定範囲が 1 つのキャッシュ済み範囲にすっぽり入るときは、キャッシュだけで表示し AWS を呼ばない
    category: policy
    applies_to: FetchCoordinator
    trigger: "[Fetch] で取得を始めるとき"
    logic: "U1:BR1.8 の算出どおりの範囲（終了はその秒の 999 ミリ秒まで、FR3.5） [startMs, endMs] について、IF cacheEnabled AND キーの CacheEntry がある AND startMs >= r.startMs AND endMs <= r.endMs となる CoveredRange r がある THEN CacheEntry から timestamp が [startMs, endMs] のイベントだけを取り出し、U3 の保持ログに追加する（U3 の取得と同じ受け口を通し、U5 の絞り込みもかかる）。StreamPlanner と EventFetcher は使わず、AWS の API は 1 回も呼ばない。FetchJob は status = Completed、failedStreamCount = 0、servedFromCache = true、cacheOutcome = Hit。0 件でも Hit"
    violation: なし
    source: FR7.4
  - id: BR2.4
    statement: すっぽり入らないときは、指定範囲をすべて AWS から取得する
    category: policy
    applies_to: FetchCoordinator
    trigger: "[Fetch] で取得を始めるとき"
    logic: IF BR2.3 に当たらない（範囲の一部だけがキャッシュ済み、2 つの範囲にまたがる、キャッシュがない）THEN キャッシュの部分は使わず、U3 のとおり [startMs, endMs] をすべて AWS から取得する
    violation: なし
    source: FR7.4、Q3

  - id: BR3.1
    statement: キャッシュ済みとして記録する範囲の終わりは、取得を始めた時刻の 5 分前までにする
    category: calculation
    applies_to: CoveredRange
    trigger: 取得結果を書き込むとき
    logic: fetchStartedAt = 取得を始めた時刻（エポックミリ秒）。recordEnd = min(endMs, fetchStartedAt - 300000)。IF recordEnd < startMs THEN 記録できる範囲はなく、書き込まない（cacheOutcome = NotSaved）。ELSE 記録する範囲は [startMs, recordEnd]
    violation: なし
    source: Q2
  - id: BR3.2
    statement: 書き込むのは、すべてのストリームの取得が成功して最後まで終わった取得だけ
    category: constraint
    applies_to: FetchCoordinator
    trigger: AWS からの取得が終わったとき
    logic: IF cacheEnabled AND U3 の FetchJob.status = Completed AND failedStreamCount = 0 AND 途中で終わっていない AND BR3.1 で記録できる範囲がある THEN 保持ログのうち timestamp が [startMs, recordEnd] のイベントを書き込む。それ以外（失敗したストリームがある、途中で終わった、ジョブ全体の失敗）は書き込まず、既存のキャッシュも変えない（cacheOutcome = NotSaved）。キャッシュから出した取得（Hit）は書き込まない
    violation: なし
    source: FR7.9、FR4.9、FR4.10
  - id: BR3.3
    statement: 同じキーのキャッシュには、新しい範囲のイベントを入れ替えて足し、範囲をつなげる
    category: calculation
    applies_to: CacheEntry
    trigger: 取得結果を書き込むとき
    logic: "記録する範囲を n = [startMs, recordEnd] とする。既存の CacheEntry から timestamp が n に入るイベントを捨て、新しい取得の n のイベントを足す（同じ範囲のイベントを二重に持たない。sequence は取得ごとに振られるため、イベント同士の突き合わせはしない）。coveredRanges に n を足し、重なる区間と隣り合う区間（a.endMs + 1 = b.startMs）をつないで 1 つにする。並びは U3:BR4.1 の順。CacheEntry がなければ新しく作る"
    violation: なし
    source: Q3
  - id: BR3.4
    statement: 書き込みは、全部書けたときだけ前のキャッシュと入れ替わる
    category: constraint
    applies_to: LogCache
    trigger: 取得結果を書き込むとき
    logic: 同じフォルダの一時ファイルに全部を書いてから、キャッシュのファイルと入れ替える。途中で失敗した・アプリが止まったときは前のキャッシュがそのまま残る。残った一時ファイルは次の書き込みと全削除で消す
    violation: なし
    source: Q5、FR7.9
  - id: BR3.5
    statement: 書き込みは取得の終わりの一部として行い、失敗しても取得結果は残す
    category: policy
    applies_to: FetchCoordinator
    trigger: BR3.2 で書き込むと決めたとき
    logic: "取得ジョブを終える前に書き込む。書き込みのあいだもステータス行は取得中のままにし、「キャッシュに保存中」を添える（U1:BR1.4 の取得中のロックはそのまま）。書き込みに失敗したら（ディスクが一杯など）、取得結果は表示したまま、cacheOutcome = SaveFailed とし、ステータス行で知らせる。成功したら cacheOutcome = Saved（知らせは出さない）"
    violation: 書き込めない（ステータス行で知らせる。取得は成功のまま）
    source: FR7.9、NFR8
  - id: BR3.6
    statement: キャッシュには取得したログとキーと範囲だけを、本人だけが読める形で書く
    category: constraint
    applies_to: LogCache
    trigger: 書き込むとき、フォルダを作るとき
    logic: "書くのは CacheKey・formatVersion・coveredRanges・CachedEvent だけ。秘密の認証情報・アクセスキー ID は書かない。アプリ専用フォルダがなければ作る。フォルダは本人だけが開ける権限（0700）、ファイルは本人だけが読み書きできる権限（0600）で作る"
    violation: なし
    source: project.md Forbidden、NFR5、FR7.2

  - id: BR4.1
    statement: 読めないキャッシュは捨てて、AWS から取り直す
    category: policy
    applies_to: FetchCoordinator
    trigger: BR2.3 でキャッシュを読むとき
    logic: "IF ファイルが読めない OR 解釈できない OR 知らない書式の版 OR 中のキーが一致しない OR coveredRanges が BR3.3 の形でない THEN そのキーのキャッシュファイルを消し、BR2.4 と同じく AWS から取得する。cacheOutcome = ReadFailed とし、ステータス行に「キャッシュが読めなかったので取り直した」と出す。取り直した結果は BR3.2 の条件で書き込む（そのときも ReadFailed の知らせを優先する）"
    violation: なし
    source: Q5
  - id: BR4.2
    statement: 範囲の判定はキャッシュの中身を全部読まずに行えるようにする
    category: constraint
    applies_to: LogCache
    trigger: BR2.3 の判定のとき
    logic: CacheKey と coveredRanges は、イベントを読まずに確かめられるように持つ（ファイルの先頭や別の小さなファイルなど。形は Code Generation で決める）。BR2.4 に当たるときは、イベントを読まない
    violation: なし
    source: NFR2（大量件数でも取得の開始を待たせない）
  - id: BR4.3
    statement: キャッシュから出すイベントの sequence は、ストリームごとに 0 から振り直す
    category: calculation
    applies_to: CachedEvent
    trigger: キャッシュから保持ログに追加するとき
    logic: 取り出したイベントを U3:BR4.1 の順に並べたまま、ストリームごとに 0 から隙間なく sequence を振り直す（U3:BR3.2 と同じ意味にする）
    violation: なし
    source: U3:BR3.2、U3:BR4.4

  - id: BR5.1
    statement: 設定ダイアログは [*] から開き、取得中と接続変更の確認中は開けない
    category: policy
    applies_to: DesktopUi
    trigger: 利用者が [*] を押したとき
    logic: "上部バーの [*] で画面 6 を開く。IF 取得中（キャッシュへの書き込み中を含む）OR 接続変更の確認待ち THEN [*] は押せない。ダイアログには、キャッシュの有効・無効のチェックボックス、保存場所（cacheDirectory）、「ログに機密情報が含まれうる」の注意、[Clear cache]、[Cancel]、[Save] を出す。保存場所と注意は、チェックの有無にかかわらず常に出す。ダイアログを開いているあいだは、ほかの操作はできない"
    violation: なし
    source: FR7.1、FR7.2、FR7.3、wireframes 画面 6、U2 の確認ダイアログ
  - id: BR5.2
    statement: 設定ダイアログはキーボードだけで操作でき、Escape は [Cancel] と同じ
    category: policy
    applies_to: DesktopUi
    trigger: ダイアログを開いた・閉じたとき、キーを押したとき
    logic: "開いたら最初の項目（チェックボックス）に入力位置を移し、閉じたら [*] に戻す。Tab で項目を順に移り、Space でチェックを切り替え、Enter で入力位置のボタンを押す。Escape で [Cancel] と同じく、保存せずに閉じる（[Clear cache] で消したものは戻らない）。文言は英日"
    violation: なし
    source: wireframes 画面 6 のアクセシビリティ、U1 のキーボード操作の土台
  - id: BR5.3
    statement: キャッシュから出したこと・読めなかったこと・保存できなかったことをステータス行に出す
    category: policy
    applies_to: DesktopUi
    trigger: 取得が終わったとき
    logic: "cacheOutcome が Hit なら「キャッシュから表示（AWS は呼んでいない）」、ReadFailed なら「キャッシュが読めなかったので AWS から取り直した」、SaveFailed なら「キャッシュに保存できなかった」を、U3 の件数の表示と並べて文字で出す（色やアイコンだけにしない）。Saved・NotSaved・NotUsed は何も出さない。次の [Fetch] で消える。文言は英日"
    violation: なし
    source: FR7.4、Q5、wireframes の「エラーをアイコンや色だけで示さない」
  - id: BR5.4
    statement: 絞り込みの欄で Enter を押すと、0.3 秒待たずにすぐ絞り込む
    category: policy
    applies_to: DesktopUi
    trigger: 絞り込みの欄で Enter を押したとき
    logic: 待っている 0.3 秒の待ちをやめ、その時点の文字列をすぐライブラリに渡す（文字列が前と同じなら何もしない）。取得は始めない。U1:BR6.2（どの入力欄でも Enter で取得）の例外で、U5:BR1.3 に Enter の場合を足す
    violation: なし
    source: U5 のコード生成のレビュー R-01（人間の判断で例外を認めた）
  - id: BR5.5
    statement: 確認用プログラムはキャッシュを使わない
    category: constraint
    applies_to: 確認用プログラム
    trigger: 確認用プログラムを実行したとき
    logic: 確認用プログラムは U1 のまま、常に AWS から取得し、ディスクに書かない（実際の AWS との疎通を確かめる目的のため）
    violation: なし
    source: U1:BR7.1、team.md の Walking Skeleton
```

## まとめ

| 分類 | ルール |
|------|--------|
| 設定 | BR1.1 1 項目だけ覚える／BR1.2 読めなければ無効／BR1.3 無効にして保存したら全削除／BR1.4 [Clear cache] はその場で全削除／BR1.5 無効なら読みも書きもしない |
| 引き方 | BR2.1 3 つの組み合わせ／BR2.2 ファイル名はハッシュ、キーは中で確かめる／BR2.3 すっぽり入れば AWS を呼ばない／BR2.4 入らなければ全部 AWS から |
| 書き込み | BR3.1 開始の 5 分前まで／BR3.2 全部成功したときだけ／BR3.3 範囲で入れ替えてつなげる／BR3.4 全部書けたら入れ替え／BR3.5 取得の終わりの一部、失敗しても結果は残す／BR3.6 本人だけが読める |
| 読み出し | BR4.1 読めなければ捨てて取り直す／BR4.2 範囲の判定は中身を読まずに／BR4.3 sequence を振り直す |
| 画面 | BR5.1 [*] と開けないとき／BR5.2 キーボードと Escape／BR5.3 ステータス行の知らせ／BR5.4 絞り込みの Enter（U5 から持ち越し）／BR5.5 確認用プログラムは使わない |
