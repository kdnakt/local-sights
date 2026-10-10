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
    logic: "[Save] で cacheEnabled と書式の版だけを settingsPath に書く（権限は BR3.6）。起動時に読み、SessionState.cacheEnabled に写す。プロファイル名・リージョン・ロググループ・認証情報は書かない（U2 の「前回の選択は覚えない」はそのまま）"
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
    statement: "[Clear cache] は、その場でこのアプリのキャッシュファイルだけをすべて消す"
    category: policy
    applies_to: LogCache
    trigger: 設定ダイアログで [Clear cache] を押したとき
    logic: "cacheDirectory の直下にある、名前が BR2.2 のキャッシュファイルの型（`<SHA-256 の 16 進 64 文字>.cache`）か一時ファイルの型（`<同じ 64 文字>.cache.tmp-<任意>`）に合う通常のファイルだけを消す。シンボリックリンクはたどらず、リンク自体も消さない。下のフォルダには入らない。名前が合わないファイルは残す。[Save]・[Cancel] を待たず、[Cancel] で取り消せない。終わったら、ダイアログに消したこと、または消せなかったことを出す。有効・無効の状態は変えない"
    violation: 消せない（画面に出す）
    source: FR7.6、wireframes 画面 6、レビュー R-08
  - id: BR1.5
    statement: 無効のときは、キャッシュを読みも書きもしない
    category: constraint
    applies_to: FetchCoordinator
    trigger: 取得を始めるとき、取得が終わったとき
    logic: IF cacheEnabled = false THEN キャッシュを引かずに AWS から取得し（U3 のまま）、取得結果をディスクに書かない。cacheOutcome = NotUsed
    violation: なし
    source: FR7.7、NFR8、Q4
  - id: BR1.6
    statement: 設定ファイルとキャッシュの場所は外から渡し、既定の作り方ではディスクに触れない
    category: constraint
    applies_to: AppSession
    trigger: AppSession と LogCache を作るとき
    logic: settingsPath と cacheDirectory は、アプリの起動時に OS のフォルダから決めて AppSession（とその中の LogCache）に渡す。場所を渡さない既定の作り方（既存のテストが使う形）はキャッシュなしとして無効で始まり、設定ファイルもキャッシュも読み書きしない。テストはテストごとの一時フォルダを渡す
    violation: なし
    source: レビュー R-09、team.md の Testing Posture

  - id: BR2.1
    statement: キャッシュは（プロファイル・リージョン・ロググループ）の組み合わせで引く
    category: calculation
    applies_to: CacheKey
    trigger: 取得を始めるとき
    logic: 選択中の ConnectionProfile を kind（SdkDefault / Named）と名前の組で表し（SdkDefault は名前なし）、リージョンのコード、ロググループの名前と合わせて CacheKey を作る。kind が違えば、名前が同じでも別のキー。3 つがすべて一致するものだけを同じキャッシュとみなす。AWS のアカウントは調べない（読み取り 3 API 以外を呼ばないため）
    violation: なし
    source: FR7.4、project.md Forbidden、レビュー R-08
  - id: BR2.2
    statement: キャッシュのファイル名はキーの SHA-256 にし、キー自体はファイルの中で確かめる
    category: constraint
    applies_to: LogCache
    trigger: キャッシュのファイルを決めるとき
    logic: CacheKey の 3 つの値を区切りがあいまいにならない形（kind と各値の長さを含める）で並べ、その SHA-256 を 16 進 64 文字にしたものに `.cache` を付けてファイル名にする（ロググループ名などの文字をファイル名に使わない）。読むときは、ファイルの中のキーが引いたキーと一致することを確かめ、一致しなければ BR4.1 の壊れたものとして扱う
    violation: なし
    source: FR7.3、BR4.1、レビュー R-08
  - id: BR2.3
    statement: 指定範囲が 1 つのキャッシュ済み範囲にすっぽり入るときは、キャッシュだけで表示し AWS を呼ばない
    category: policy
    applies_to: FetchCoordinator
    trigger: "[Fetch] で取得を始めるとき"
    logic: "U1:BR1.8 の算出どおりの範囲 [startMs, endMs]（終了はその秒の 999 ミリ秒まで、FR3.5）について、IF cacheEnabled AND キーの CacheEntry がある AND startMs >= r.startMs AND endMs <= r.endMs となる CoveredRange r がある THEN キャッシュから timestamp が [startMs, endMs] のイベントをすべて読み、BR4.1 の確かめを全部終えてから、保持ログを破棄して 1 回でまとめて追加する（読みながら追加しない。確かめに失敗したら保持ログには何も足さず、破棄もしない）。知らせの順は BR3.7。U5 の絞り込みもそのままかかる。StreamPlanner と EventFetcher は使わず、AWS の API は 1 回も呼ばない。FetchJob は status = Completed、failedStreamCount = 0、servedFromCache = true、cacheOutcome = Hit。0 件でも Hit"
    violation: なし
    source: FR7.4、レビュー R-02
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
    statement: 同じキーのキャッシュには、新しい範囲のイベントを入れ替えて足し、範囲をつなげる。既存が読めなければ作り直す
    category: calculation
    applies_to: CacheEntry
    trigger: 取得結果を書き込むとき
    logic: "記録する範囲を n = [startMs, recordEnd] とする。既存の CacheEntry を読み、BR4.1 の確かめをする。読めて確かめに通ったら、timestamp が n に入るイベントを捨て、新しい取得の n のイベントを足す（同じ範囲のイベントを二重に持たない。sequence は取得ごとに振られるため、イベント同士の突き合わせはしない）。coveredRanges に n を足し、重なる区間と隣り合う区間（a.endMs + 1 = b.startMs）をつないで 1 つにする。並びは U3:BR4.1 の順。CacheEntry がない、または既存が読めない・確かめに通らないときは、既存を捨てて n と新しいイベントだけで作り直す（前の範囲は失う。知らせは出さず、診断ログにだけ残す）"
    violation: なし
    source: Q3、レビュー R-03
  - id: BR3.4
    statement: 書き込みは、全部書けたときだけ前のキャッシュと入れ替わる
    category: constraint
    applies_to: LogCache
    trigger: 取得結果を書き込むとき
    logic: 同じフォルダの一時ファイル（BR1.4 の名前の型）に全部を書いてから、キャッシュのファイルと入れ替える。途中で失敗した・アプリが止まったときは前のキャッシュがそのまま残る。残った一時ファイルは次の書き込みと全削除で消す
    violation: なし
    source: Q5、FR7.9
  - id: BR3.5
    statement: 書き込みは取得の終わりの一部として、保持ログのロックを握らずに別のスレッドで行い、失敗しても取得結果は残す
    category: policy
    applies_to: FetchCoordinator
    trigger: BR3.2 で書き込むと決めたとき
    logic: "取得ジョブを終える前（BR3.7 の on_finished の前）に書き込む。書き込みのあいだもフェーズは取得中のままにし（U1:BR1.4 の取得中のロックはそのまま）、ステータス行に BR5.3 の「保存中」を添える。保持ログからは、記録する範囲のイベントを短い区切りごとにロックを取って放しながら写し取り、ロックを握ったまま書き込まない（取得中のロックで新しい追加・破棄は起きないため、写し取るあいだに中身は変わらない）。既存の読み出し・合わせ込み・ファイルへの書き込みは、取得の非同期の処理を塞がないようにブロッキング用のスレッドで行う。書き込みに失敗したら（ディスクが一杯など）、取得結果は表示したまま cacheOutcome = SaveFailed とする。成功したら cacheOutcome = Saved（知らせは出さない）"
    violation: 書き込めない（ステータス行で知らせる。取得は成功のまま）
    source: FR7.9、NFR2、NFR8、レビュー R-07
  - id: BR3.6
    statement: キャッシュと設定ファイルには決めたものだけを、本人だけが読める形で書く
    category: constraint
    applies_to: LogCache
    trigger: 書き込むとき、フォルダを作るとき
    logic: "キャッシュに書くのは CacheKey・formatVersion・coveredRanges・CachedEvent だけ、設定ファイルに書くのは cacheEnabled と書式の版だけ。アプリ自身が扱う認証情報（SDK の資格情報・シークレットアクセスキー・セッショントークン・SSO のトークン・アクセスキー ID）は、どちらにも書かず、書く経路も持たない。ログの本文（CachedEvent.message）は利用者のデータとして API が返したまま保存する。本文に機密情報が含まれうることは、設定ダイアログの注意（FR7.2、BR5.1）で示す。フォルダは本人だけが開ける権限（0700）、ファイルは本人だけが読み書きできる権限（0600）で作る。フォルダが既にあり権限がそれより緩いときは 0700 に直し、直せなければ書き込まない（キャッシュは SaveFailed、設定は BR1.3 の書けなかったとき）"
    violation: なし
    source: project.md Forbidden、NFR5、FR7.2、レビュー R-06、R-08
  - id: BR3.7
    statement: キャッシュが有効なときの取得の知らせは、決めた順で出す（U3:BR5.4 の例外）
    category: policy
    applies_to: FetchCoordinator
    trigger: キャッシュが有効なときの取得
    logic: "Hit のとき：BR4.2 と BR4.1 の確かめをすべて終えてから、保持ログを破棄 → on_started → 1 回の追加 → 件数が 1 以上なら on_batch を 1 回（added = total = 件数）→ on_finished。on_listing_progress・on_planned・on_stream_finished は出さない。AWS から取得するとき：U3 の順（started → listing → planned → batches・streams）のあと、BR3.2 で書き込むなら on_saving（新しい知らせ）→ 書き込み → on_finished、書き込まないならすぐ on_finished。途中で終わったとき（Aborted）は U3:BR5.5 のまま on_finished。on_finished で渡す FetchJob には servedFromCache・cacheOutcome・readFailed が入っており、AppSession はそれを SessionState の cacheNotices に写す（BR5.3）。on_saving を受けた AppSession は SessionState の cacheSaving を true にし、on_finished で false に戻す"
    violation: なし
    source: レビュー R-01、U3:BR5.4、U3:BR5.5、U3:BR5.6

  - id: BR4.1
    statement: 読めないキャッシュは捨てて、空の保持ログから AWS で取り直す
    category: policy
    applies_to: FetchCoordinator
    trigger: BR2.3 でキャッシュを読むとき
    logic: "次のどれかに当たれば壊れたものとする：ファイルが読めない、解釈できない、途中で切れている、知らない書式の版、中のキーが一致しない、coveredRanges が BR3.3 の形（重なり・隣り合いなし、開始の早い順、各区間で startMs <= endMs）でない、イベントの timestamp がどの coveredRange にも入らない、イベントの並びが U3:BR4.1 の順でない。壊れていたら、保持ログには何も足さず、そのキーのキャッシュファイルを消し、readFailed = true として BR2.4 と同じく AWS から取得する（U3 の取得の始めで保持ログは破棄されるため、取り直しは空から始まる）。ファイルを消せなかったときも AWS からの取得は続け、条件を満たせば BR3.4 の入れ替えで置き換える。取り直した結果は BR3.2 の条件で書き込み、cacheOutcome はその結果（Saved / SaveFailed / NotSaved）になる。知らせは BR5.3"
    violation: なし
    source: Q5、レビュー R-02、R-03、R-04
  - id: BR4.2
    statement: 範囲の判定はキャッシュの中身を全部読まずに行えるようにする
    category: constraint
    applies_to: LogCache
    trigger: BR2.3 の判定のとき
    logic: CacheKey と coveredRanges は、イベントを読まずに確かめられるように持つ（ファイルの先頭や別の小さなファイルなど。形は Code Generation で決める）。BR2.4 に当たるときは、イベントを読まない。読み出しはブロッキング用のスレッドで行う（BR3.5 と同じ）
    violation: なし
    source: NFR2（大量件数でも取得の開始を待たせない）
  - id: BR4.3
    statement: キャッシュから出すイベントの sequence は、ストリームごとに 0 から振り直す
    category: calculation
    applies_to: CachedEvent
    trigger: キャッシュから保持ログに追加するとき
    logic: 取り出したイベントを U3:BR4.1 の順に並べたまま、ストリームごとに 0 から隙間なく sequence を振り直す（U3 で取得したイベントと同じ意味にする）
    violation: なし
    source: U3:BR4.1、U3:BR4.4

  - id: BR5.1
    statement: 設定ダイアログは [*] から開き、取得中と接続変更の確認中は開けない。開いているあいだはライブラリ側でもほかの操作を拒む
    category: policy
    applies_to: AppSession
    trigger: 利用者が [*] を押したとき、ダイアログを開いているあいだの操作
    logic: "上部バーの [*] で画面 6 を開く。IF 取得中（キャッシュへの書き込み中を含む）OR 接続変更の確認待ち THEN [*] は押せず、AppSession もダイアログを開く求めを拒む。ダイアログには、キャッシュの有効・無効のチェックボックス、保存場所（cacheDirectory）、「ログに機密情報が含まれうる」の注意、[Clear cache]、[Cancel]、[Save] を出す。保存場所と注意は、チェックの有無にかかわらず常に出す。settingsDialog = Open のあいだ、AppSession は U2 の確認待ちと同じく、取得の開始・接続とロググループの変更・入力の変更を拒む（絞り込みの文字列の変更も拒む）。閉じるのは [Save]（書けたとき）・[Cancel]・Escape だけ"
    violation: 開けない・拒む（画面では押せない形にする）
    source: FR7.1、FR7.2、FR7.3、wireframes 画面 6、U2 の確認ダイアログ、レビュー R-09
  - id: BR5.2
    statement: 設定ダイアログはキーボードだけで操作でき、Escape は [Cancel] と同じ
    category: policy
    applies_to: DesktopUi
    trigger: ダイアログを開いた・閉じたとき、キーを押したとき
    logic: "開いたら最初の項目（チェックボックス）に入力位置を移し、閉じたら [*] に戻す。Tab で項目を順に移り、Space でチェックを切り替え、Enter で入力位置のボタンを押す。Escape で [Cancel] と同じく、保存せずに閉じる（[Clear cache] で消したものは戻らない）。文言は英日"
    violation: なし
    source: wireframes 画面 6 のアクセシビリティ、U1 のキーボード操作の土台、NFR13
  - id: BR5.3
    statement: キャッシュについての知らせは、決めた文言でステータス行に出す（文言の正本）
    category: policy
    applies_to: DesktopUi
    trigger: 取得中・取得が終わったとき
    logic: "文言の正本はこの表だけとし、ほかの文書はここを参照する。cache.saving＝ja「キャッシュに保存中」／en「Saving to cache」（cacheSaving が true のあいだ、取得中の表示に添える）。cache.hit＝ja「キャッシュから表示（AWS は呼んでいない）」／en「Shown from cache (AWS was not called)」（cacheOutcome = Hit）。cache.readFailed＝ja「キャッシュが読めなかったので取り直した」／en「The cache could not be read, so the logs were fetched again」（readFailed = true、Q5 の回答の文言）。cache.saveFailed＝ja「キャッシュに保存できなかった」／en「Could not save to the cache」（cacheOutcome = SaveFailed）。readFailed と SaveFailed が両方あれば両方を出す。U3 の件数の表示と並べて文字で出す（色やアイコンだけにしない）。Saved・NotSaved・NotUsed は何も出さない。次の [Fetch] で消える"
    violation: なし
    source: FR7.4、Q5、wireframes の「エラーをアイコンや色だけで示さない」、NFR12、レビュー R-04、R-05
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
| 設定 | BR1.1 1 項目だけ覚える／BR1.2 読めなければ無効／BR1.3 無効にして保存したら全削除／BR1.4 [Clear cache] は名前の型が合うファイルだけ消す／BR1.5 無効なら読みも書きもしない／BR1.6 場所は外から渡す |
| 引き方 | BR2.1 kind 付きの 3 つの組み合わせ／BR2.2 ファイル名は SHA-256、キーは中で確かめる／BR2.3 すっぽり入れば全部確かめてから 1 回で追加、AWS を呼ばない／BR2.4 入らなければ全部 AWS から |
| 書き込み | BR3.1 開始の 5 分前まで／BR3.2 全部成功したときだけ／BR3.3 範囲で入れ替えてつなげる、読めなければ作り直す／BR3.4 全部書けたら入れ替え／BR3.5 ロックを握らず別のスレッドで、失敗しても結果は残す／BR3.6 決めたものだけ、本人だけが読める／BR3.7 知らせの順 |
| 読み出し | BR4.1 読めなければ捨てて空から取り直す／BR4.2 範囲の判定は中身を読まずに／BR4.3 sequence を振り直す |
| 画面 | BR5.1 [*] と開けないとき、開いているあいだは拒む／BR5.2 キーボードと Escape／BR5.3 文言の正本／BR5.4 絞り込みの Enter（U5 から持ち越し）／BR5.5 確認用プログラムは使わない |
