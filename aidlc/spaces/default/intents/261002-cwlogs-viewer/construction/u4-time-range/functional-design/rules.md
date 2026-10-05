# Business Rules — U4 時間範囲とタイムゾーン（u4-time-range）

上流の成果物：`inception/requirements-analysis/requirements.md`（U4 の FR：FR3.2〜FR3.4、FR3.6。FR3.1・FR3.5 は U1 で作ったもの）、`inception/units-generation/unit-of-work.md`、`inception/domain-design/components.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q4。

U1〜U3 のルールは、ここで置き換えると書いたもの以外はそのまま有効。前の単位のルールは `U1:BR1.2` のように作業単位名を前に付けて書く。前に何も付けない `BRx.y` は、この文書（U4）のルールを指す。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: タイムゾーンはローカルと UTC の 2 つで、起動のたびにローカルから始める
    category: policy
    applies_to: TimeZoneChoice
    trigger: アプリの起動時
    logic: 起動時は Local にする。選んだタイムゾーンは保存せず、次の起動に引き継がない（設定ファイルを作らない。U2 の Q2 と同じ方針）。Local は OS のタイムゾーンの設定で、夏時間の有無と切り替えの日時もその設定に従う
    violation: なし
    source: FR3.2、Q2
  - id: BR1.2
    statement: 日時の入力は、選んだタイムゾーンの壁時計の時刻として解釈する
    category: calculation
    applies_to: DateTimeInput
    trigger: 入力欄の文字列が変わったとき
    logic: 前後の空白を除き、空なら instant も error も持たない。U1:BR1.2 の形と日付・時刻の正しさを確かめ、通らなければ error = Format。通ったら、IF timeZone = Utc THEN その壁時計の時刻を UTC として instant にする。IF timeZone = Local THEN その壁時計の時刻に当たる瞬間を求め、1 つならそれ、2 つ（夏時間の終わりで 2 回現れる）なら早い方を instant にし、0 個（夏時間の始まりで存在しない）なら error = NonexistentLocalTime とする（U1:BR1.2 の「UTC として解釈」を置き換える）
    violation: error を持ち、[Fetch] を押せない理由に入る（BR2.1）
    source: FR3.1、FR3.6
  - id: BR1.3
    statement: 入力欄は、文字列と、解釈できたときの瞬間を一緒に持つ
    category: constraint
    applies_to: DateTimeInput
    trigger: 常に
    logic: instant はその秒の 0 ミリ秒（U1:BR2.1）。instant と error は同時に持たない。取得の範囲・順序の確認・タイムゾーンの切り替えは、文字列ではなく instant を使う
    violation: なし
    source: FR3.3、FR3.6
  - id: BR1.4
    statement: タイムゾーンを切り替えても、入力済みの日時は同じ瞬間を指したまま表示だけを変える
    category: policy
    applies_to: DateTimeInput
    trigger: タイムゾーンを切り替えたとき
    logic: 開始・終了のそれぞれについて、IF instant を持つ THEN text を、instant を新しいタイムゾーンで表した yyyy-mm-dd hh:mm:ss に作り直し、instant は変えない（作り直した文字列を解釈し直さない。そのため、夏時間の終わりで 2 回現れる時刻の遅い方の瞬間でも、瞬間は保たれる）。ELSE（形式の誤り・存在しない日時・空）THEN text と error をそのまま残す（Q1）。そのあと [Fetch] を押せる条件を確かめ直す（BR2.1）。タイムゾーンの切り替えは取得条件を変えないため、取得中（phase Fetching）と接続の変更の確認待ちでも受け付ける
    violation: なし
    source: FR3.3、Q1
  - id: BR1.5
    statement: 入力欄を書き換えたら、いまのタイムゾーンで解釈し直す
    category: policy
    applies_to: DateTimeInput
    trigger: 入力欄の文字列が利用者の入力で変わったとき
    logic: BR1.2 で解釈し、instant と error を置き換える。取得中は U1:BR1.4 のとおり書き換えを受け付けない
    violation: なし
    source: FR3.1、FR3.6
  - id: BR1.6
    statement: 取得の範囲と開始・終了の順序は、瞬間から決める
    category: calculation
    applies_to: TimeRange
    trigger: [Fetch] を押せるかを確かめるとき、取得を始めるとき
    logic: 開始・終了の両方が instant を持つときだけ、U1:BR1.3（開始の秒が終了の秒より前）を instant で確かめ、U1:BR2.1・U1:BR2.2（開始は 0 ミリ秒、終了はその秒の 999 ミリ秒まで）で TimeRange を作る。タイムゾーンは範囲に影響しない
    violation: 順序が誤りなら RangeOrder（BR2.1）
    source: FR3.4、FR3.5

  - id: BR2.1
    statement: "[Fetch] は FR3.4 の条件をすべて満たすときだけ押せ、満たさない理由をすべて文字で示す"
    category: validation
    applies_to: AppSession
    trigger: 選択・入力・タイムゾーン・phase が変わったとき
    logic: 条件は、プロファイルとリージョンが選ばれている（U2:BR2.7）、ロググループが 1 つ選ばれている（U2:BR2.7）、開始・終了が空でなく error を持たない（BR1.2）、開始が終了より前（BR1.6）、取得中でない（U1:BR1.4）。満たさない理由は、画面だけの選択の理由と共通の検証の理由（U1:BR1.8）を合わせ、条件の並びの順にすべて出す。開始または終了が error を持つときは、順序の確認（RangeOrder）は出さない
    violation: "[Fetch] を無効にし、理由の文言キーを並べて示す"
    source: FR3.4
  - id: BR2.2
    statement: 夏時間で存在しない日時は、形式の誤りとは別の理由として示す
    category: validation
    applies_to: ValidationError
    trigger: 開始または終了が error = NonexistentLocalTime を持つとき
    logic: 開始なら StartNonexistentLocalTime、終了なら EndNonexistentLocalTime を理由にし、文言は「開始日時は夏時間の切り替えで存在しない日時です」「終了日時は夏時間の切り替えで存在しない日時です」（英日）。形式の誤り（StartFormat・EndFormat）とは別の文言キーにする
    violation: "[Fetch] を押せない"
    source: FR3.6、Q3

  - id: BR3.1
    statement: ログ一覧の時刻は、選んだタイムゾーンの yyyy-mm-dd hh:mm:ss.mmm で表示する
    category: calculation
    applies_to: DesktopUi
    trigger: ログ一覧を描くとき、タイムゾーンを切り替えたとき
    logic: 各行の timestamp を、選んだタイムゾーンで yyyy-mm-dd hh:mm:ss.mmm に表す。Local では、その行の瞬間でのオフセット（夏時間を含む）を使う。タイムゾーンを切り替えたら、取得し直さずに表示だけを描き直す。並び順（U3:BR4.1）は瞬間で決まるため変わらない（U1:BR5.3 の「UTC で表示」を置き換える）
    violation: なし
    source: FR3.3
  - id: BR3.2
    statement: ログ一覧の時刻の列の見出しに、いまのタイムゾーンを出す
    category: policy
    applies_to: DesktopUi
    trigger: ログ一覧を描くとき
    logic: 見出しを「時刻（ローカル）」「時刻（UTC）」（英語は「Time (Local)」「Time (UTC)」）にする。各行の時刻にはオフセットを付けない
    violation: なし
    source: FR3.3、Q4
  - id: BR3.3
    statement: 上部バーにタイムゾーンの切替を置き、キーボードでも操作できる
    category: policy
    applies_to: DesktopUi
    trigger: 常に
    logic: 上部バーにローカルと UTC の 2 つから選ぶ標準部品を置く（見た目は標準部品だけ、project.md Corrections）。Tab で選んで矢印キーかスペースで切り替えられる。文言は英日（U1:BR6.1）。取得中と確認待ちでも使える（BR1.4）
    violation: なし
    source: FR3.2、NFR12、NFR13
  - id: BR3.4
    statement: 画面とライブラリは同じローカルのタイムゾーンを使う
    category: constraint
    applies_to: TimeRangeModel
    trigger: 常に
    logic: 入力の解釈（BR1.2）・入力の作り直し（BR1.4）・ログ一覧の時刻表示（BR3.1）は、すべて OS のタイムゾーンの設定を使い、同じ瞬間には同じ壁時計の時刻を出す。自動テストでは、タイムゾーンを決めた値（夏時間のある地域と UTC）に差し替えて確かめる
    violation: なし
    source: FR3.3、FR3.6
  - id: BR3.5
    statement: 確認用プログラムの日時の引数は、UTC のまま変えない
    category: constraint
    applies_to: 確認用プログラム
    trigger: 手元で実行したとき
    logic: 確認用プログラムの開始・終了の引数は U1:BR7.1 のとおり UTC として解釈し、出力の時刻も UTC のまま（U3:BR6.9）。タイムゾーンの切替は画面だけ
    violation: なし
    source: U1:BR7.1
```

## まとめ

| ID | ルール | 種類 | 出典 |
|----|--------|------|------|
| BR1.1 | ローカルと UTC、起動のたびにローカル | policy | FR3.2、Q2 |
| BR1.2 | 入力は選んだタイムゾーンで解釈。存在しない日時は誤り、2 回は早い方 | calculation | FR3.1、FR3.6 |
| BR1.3 | 入力欄は文字列と瞬間を持つ | constraint | FR3.3 |
| BR1.4 | 切り替えても同じ瞬間、変換できない入力はそのまま | policy | FR3.3、Q1 |
| BR1.5 | 書き換えたら解釈し直す | policy | FR3.1 |
| BR1.6 | 範囲と順序は瞬間から | calculation | FR3.4、FR3.5 |
| BR2.1 | [Fetch] の条件と理由 | validation | FR3.4 |
| BR2.2 | 存在しない日時は別の理由 | validation | FR3.6、Q3 |
| BR3.1 | 一覧の時刻は選んだタイムゾーンで | calculation | FR3.3 |
| BR3.2 | 時刻の列の見出しにタイムゾーン | policy | Q4 |
| BR3.3 | 上部バーの切替、キーボード | policy | FR3.2、NFR12、NFR13 |
| BR3.4 | 画面とライブラリは同じローカル | constraint | FR3.3 |
| BR3.5 | 確認用プログラムは UTC のまま | constraint | U1:BR7.1 |
