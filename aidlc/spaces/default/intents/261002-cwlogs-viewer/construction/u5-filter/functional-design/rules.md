# Business Rules — U5 絞り込み（u5-filter）

上流の成果物：`inception/requirements-analysis/requirements.md`（U5 の FR：FR6.1〜FR6.5、NFR1、NFR2）、`inception/units-generation/unit-of-work.md`、`inception/domain-design/components.md`、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1〜Q4。

U1〜U4 のルールは、ここで置き換えると書いたもの以外はそのまま有効。前の単位のルールは `U3:BR4.3` のように作業単位名を前に付けて書く。前に何も付けない `BRx.y` は、この文書（U5）のルールを指す。

## ルール（機械可読）

```yaml
rules:
  - id: BR1.1
    statement: 絞り込みの文字列は前後の空白を除き、空なら絞り込まない
    category: validation
    applies_to: FilterCondition
    trigger: 絞り込みの文字列を受けたとき
    logic: 前後の空白を除いて filterText にする。IF 空 THEN 条件なしと同じに扱い、絞り込まず、一覧は全件を出し、「絞り込み後 / 全件」は出さない。文字列の長さの上限は設けない
    violation: なし
    source: FR6.1、U2 のロググループの絞り込み（U2:BR3.7）と同じ扱い、U5 の機能設計のレビュー R-05
  - id: BR1.2
    statement: メッセージに文字列を含む行だけを、大文字・小文字を区別せずに選ぶ
    category: calculation
    applies_to: FilterEngine
    trigger: 行に条件をかけるとき
    logic: 行のメッセージ全体（改行を含む。一覧で 1 行に省略して見せている部分だけではない）と filterText を、どちらも Unicode の小文字に揃えてから、部分一致で比べる。比べるのはメッセージだけで、時刻とストリーム名は比べない
    violation: なし
    source: FR6.1、FR6.5、Q1
  - id: BR1.3
    statement: 絞り込みは、入力が止まってから約 0.3 秒後にかける
    category: policy
    applies_to: DesktopUi
    trigger: 絞り込みの欄に入力したとき
    logic: 最後の入力から 300 ミリ秒、新しい入力がなければ、その時点の文字列をライブラリに渡す。300 ミリ秒以内に次の入力があれば待ち直す。渡すのは最後の文字列だけ
    violation: なし
    source: Q2
  - id: BR1.4
    statement: 新しい文字列が来たら、前の絞り込みをやめて新しい文字列でかけ直す
    category: policy
    applies_to: FilterEngine
    trigger: filterText が変わったとき
    logic: filterId を 1 増やし、進行中の絞り込みをやめる。やめた絞り込みの結果（filterId が古いもの）は捨て、画面に返さない。filterText が前と同じなら何もしない
    violation: なし
    source: Q4
  - id: BR1.5
    statement: 保持ログ全体への絞り込みは、画面を止めずに手元で行い、見つかった行から出していく
    category: constraint
    applies_to: FilterEngine
    trigger: filterText が変わったとき
    logic: AWS の API は呼ばない。新しい filterId の空の結果（status = Filtering、scanCursor なし）を作って前の結果と入れ替え、保持ログをキーの小さい順に走査する。走査は小分け（例：数千行ずつ）にし、1 回分ごとに保持ログのロックを取って放す（走査の間ずっとは持たない、BR2.5）。1 回分ごとに、合う行のキーを matchedKeys に入れ、scanCursor をその回の最後の行のキーに進め、resultVersion を増やす。画面にはその時点の「ここまでに見つかった行」を出し、ステータス行に絞り込み中を出す（Q4、レビュー R-02）。保持ログの終わりまで走査したら status = Ready とし、scanCursor を消す。10 万件で 10 秒以内、100 万件で 100 秒以内に Ready にする。絞り込み中も、画面の操作（スクロール・入力・取得の開始）は止めない
    violation: なし
    source: FR6.2、NFR1、NFR2、Q4、U5 の機能設計のレビュー R-01、R-02

  - id: BR2.1
    statement: 取得中に届いたログには、逐次同じ絞り込みをかける
    category: policy
    applies_to: FilterEngine
    trigger: 保持ログにページが追加されたとき（U3:BR4.2）
    logic: AppSession は、取得の流れから届いたページの行（保持ログに足したものと同じ行）をそのまま FilterEngine に渡す（レビュー R-06）。IF filterText が空でない THEN 渡された行のうち、IF status = Ready THEN すべての行に、IF status = Filtering THEN キーが scanCursor 以下（走査済みの範囲）の行だけに条件をかける。scanCursor より大きいキーの行は、その後の走査で見るため、ここでは判定しない（取りこぼしも重複もしない、レビュー R-01）。合う行のキーを matchedKeys のキーの順の位置に入れ、allCount と matchedCount を更新し、resultVersion を増やす。結果はキーで持つため、差し込みで既存の値はずれない。1 ページあたりの更新は、そのページの行数と合う行の数に比例する程度に収める
    violation: なし
    source: FR6.4、Q3、U5 の機能設計のレビュー R-01、R-06
  - id: BR2.2
    statement: 保持ログを破棄したら結果を空にし、絞り込みの文字列は残す
    category: policy
    applies_to: FilterResult
    trigger: 保持ログの破棄（取得の開始 U3:BR5.6、中断 U3:BR5.5、接続先の変更 U3:BR4.5）
    logic: timelineEpoch を 1 増やし、結果を新しい timelineEpoch の空（matchedCount = 0、allCount = 0、status = Ready）に置き換え、resultVersion を増やす。進行中の走査があればやめる。filterText と入力欄の文字列は残し、新しく届くログには BR2.1 のとおり同じ絞り込みをかける
    violation: なし
    source: FR6.4、U5 の機能設計のレビュー R-01
  - id: BR2.3
    statement: 絞り込み結果の行は、保持ログと同じ順に並べる
    category: calculation
    applies_to: FilterResult
    trigger: 常に
    logic: matchedKeys は、保持ログの並べ替えのキー（時刻 → ストリーム名 → ストリームの中の順、U3:BR4.1）の昇順
    violation: なし
    source: FR4.7、FR6.1
  - id: BR2.4
    statement: 古い条件や古い保持ログに対する結果は捨てる
    category: constraint
    applies_to: FilterEngine
    trigger: 走査の 1 回分を結果に入れるとき、逐次の判定を結果に入れるとき
    logic: 入れる前に、その作業を始めたときの filterId と timelineEpoch が、いまの値と同じかを確かめる。IF どちらかが違う THEN その作業の結果は捨て、走査ならそこでやめる
    violation: 古い結果は FilterResult に入れない
    source: Q4、U5 の機能設計のレビュー R-01
  - id: BR2.5
    statement: 保持ログと絞り込み結果は、決まった順でロックを取り、同じロックの中で読み書きする
    category: constraint
    applies_to: AppSession
    trigger: 常に
    logic: ロックを取る順は、セッション → 保持ログ → 絞り込み結果とする（U3 で決めたセッション → 保持ログの順に足す）。ページの追加と逐次の判定（BR2.1）は、保持ログと絞り込み結果の両方のロックを持ったまま続けて行い、その間に表示範囲の取り寄せが割り込まないようにする。表示範囲の取り寄せ（BR3.1）と位置の問い合わせ（BR3.2）も、保持ログと絞り込み結果の両方のロックを持ったまま読む。走査の 1 回分も同じ順でロックを取り、1 回分が終わったら放す
    violation: なし
    source: U5 の機能設計のレビュー R-02、U3 のコード生成のロックの順

  - id: BR3.1
    statement: 絞り込み中は、表示範囲の取り寄せで絞り込み結果の行を返す
    category: calculation
    applies_to: RowWindow
    trigger: 画面が行を求めたとき（U3:BR4.3）
    logic: IF filterText が空でない THEN offset・limit を matchedKeys の中の位置として扱い、その範囲のキーの行を保持ログから引いて返す（status が Filtering でも、ここまでに見つかった行を返す）。totalCount は matchedCount、allCount は保持ログの全件、filtered = true、resultVersion を付ける。displayTime は U4:BR3.1 のまま付ける。ELSE U3・U4 のまま（filtered = false、allCount = totalCount）。1 回の取り出しの速さは U3:BR4.3 と同じ
    violation: なし
    source: FR6.1、NFR2、unit-of-work の U5（レビュー R-07）、U5 の機能設計のレビュー R-02
  - id: BR3.2
    statement: 絞り込み中の位置の保ち方と、絞り込みを変えたときの位置
    category: policy
    applies_to: DesktopUi
    trigger: 結果の版や timelineVersion が変わったとき、filterId が変わったとき
    logic: filterId が同じまま結果の行が増えたとき（逐次の追加・走査の進み）は、U3:BR6.5 のとおり一番上に見えている行を動かさない。位置の問い合わせ（U3:BR4.4）は、絞り込み中は matchedKeys の中の位置を返す（キーから二分探索で求める）。filterId が変わったとき（文字列を変えた・絞り込みを解いた）は、一覧の一番上に戻る。絞り込みを解いたときも、それまで読んでいた位置は保たない（意図した割り切り、レビュー R-04）
    violation: なし
    source: FR6.1、U3:BR6.5、U5 の機能設計のレビュー R-04
  - id: BR3.3
    statement: ステータス行に「絞り込み後 / 全件」と絞り込み中を出す
    category: policy
    applies_to: DesktopUi
    trigger: 絞り込みの状態が変わったとき
    logic: IF filterText が空でない THEN「絞り込み後 matchedCount 件 / 全 allCount 件」を出す。matchedCount が 0 なら 0 件であることを文字で出す。status = Filtering の間は「絞り込み中」も出す。取得中の進み具合（U3:BR6.2）と並べて出す。filterText が空なら何も出さない。文言は英日（U1:BR6.1）
    violation: なし
    source: FR6.3、Q4
  - id: BR3.4
    statement: 絞り込みの欄は条件エリアに置き、キーボードで使え、取得中も使える
    category: policy
    applies_to: DesktopUi
    trigger: 常に
    logic: 条件エリアに絞り込みの入力欄を置く（標準部品、project.md Corrections）。Tab で選べる。取得中も入力できる（取得条件ではないため U1:BR1.4 の対象外）。接続の変更の確認ダイアログを開いている間は、ダイアログが操作を受けるため入力できない。ラベルとプレースホルダーは英日
    violation: なし
    source: FR6.1、NFR12、NFR13
  - id: BR3.5
    statement: 確認用プログラムには絞り込みを足さない
    category: constraint
    applies_to: 確認用プログラム
    trigger: 常に
    logic: 確認用プログラム（U3:BR6.9）は変えない
    violation: なし
    source: U1:BR7.1
  - id: BR3.6
    statement: 絞り込みの状態と結果の版を画面に知らせ、版が変わったら画面は行を取り寄せ直す
    category: policy
    applies_to: AppSession
    trigger: 絞り込みの結果が変わったとき
    logic: SessionView（session-changed）と取得中の進み具合の知らせ（fetch-progress、U3）の両方に、filterSummary（filterId・matchedCount・allCount・status・resultVersion）を入れる。走査の進みや Filtering から Ready への変化のように、取得と関係なく結果が変わったときも、fetch-progress と同じ軽い知らせを送る。画面は、filterId か resultVersion が変わったら表示範囲の行を取り寄せ直す（件数が同じでも取り寄せ直す）。取り寄せた行の resultVersion が、いま知っている版より古ければ捨てる
    violation: なし
    source: FR6.3、U5 の機能設計のレビュー R-03
```

## まとめ

| ID | ルール | 種類 | 出典 |
|----|--------|------|------|
| BR1.1 | 前後の空白を除き、空なら条件なし | validation | FR6.1、R-05 |
| BR1.2 | メッセージ全体の部分一致、大文字・小文字を区別しない | calculation | FR6.1、FR6.5、Q1 |
| BR1.3 | 入力が止まって約 0.3 秒後にかける | policy | Q2 |
| BR1.4 | 新しい文字列で前の絞り込みをやめてかけ直す | policy | Q4 |
| BR1.5 | 小分けに走査し、見つかった行から出す。10 万件 10 秒・100 万件 100 秒 | constraint | FR6.2、NFR1、NFR2、R-01、R-02 |
| BR2.1 | 取得中は届いたページの行に逐次かける（走査済みの範囲だけ） | policy | FR6.4、Q3、R-01、R-06 |
| BR2.2 | 保持ログの破棄で世代を進めて結果を空に、文字列は残す | policy | FR6.4、R-01 |
| BR2.3 | 結果はキーの順（保持ログと同じ順） | calculation | FR4.7 |
| BR2.4 | 古い条件・古い保持ログの結果は捨てる | constraint | Q4、R-01 |
| BR2.5 | ロックの順と同じロックの中での読み書き | constraint | R-02 |
| BR3.1 | 絞り込み中は結果の行を取り寄せる | calculation | FR6.1、NFR2、R-02 |
| BR3.2 | 位置の保ち方、条件が変わったら一番上 | policy | U3:BR6.5、R-04 |
| BR3.3 | 「絞り込み後 / 全件」と絞り込み中 | policy | FR6.3、Q4 |
| BR3.4 | 条件エリアの入力欄、取得中も使える | policy | FR6.1、NFR12、NFR13 |
| BR3.5 | 確認用プログラムは変えない | constraint | U1:BR7.1 |
| BR3.6 | 絞り込みの状態と結果の版を知らせ、画面は取り寄せ直す | policy | FR6.3、R-03 |
