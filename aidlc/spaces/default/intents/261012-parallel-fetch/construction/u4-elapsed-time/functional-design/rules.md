# Business Rules — U4 かかった時間（u4-elapsed-time）

上流：`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR6、NFR9）、`inception/user-stories/stories.md`（US2.1：AC2.1.1〜AC2.1.7）、`inception/units-generation/unit-of-work.md`（U4）、`inception/domain-design/decisions.md`（ADR-005）、`inception/contract-design/contract-summary.md`（C4、C5）、`inception/refined-mockups/`（mockups.md、interaction-spec.md の ElapsedTime）、`entities.md`。質問票の回答は `functional-design-questions.md` の Q1・Q2 とまとめの確認。

## ルール（機械可読）

```yaml
rules:
  - id: BR4.1
    statement: ［取得］の押下を受けて取得中に移ったら、その時点を経過の始まりとして記録する
    category: policy
    applies_to: AppSession、ElapsedRecord.startedAt
    trigger: 取得を始める操作が受け入れられ、段階が取得中に移ったとき（今の begin_fetch）。キャッシュから表示する取得も同じ
    logic: IF 取得を始める操作が受け入れられた THEN 前の ElapsedRecord を捨て、startedAt = 今（戻らない時計）、finalElapsedMs は無しにする。IF 操作が断られた（取得中・確認待ち・条件の誤りなど） THEN ElapsedRecord は変えない
    violation: なし
    source: FR4.2、FR4.4、ADR-005
  - id: BR4.2
    statement: 取得が終わったら、その時点で最終の経過を決め、次の取得を始めるまで残す
    category: calculation
    applies_to: AppSession、ElapsedRecord.finalElapsedMs
    trigger: 取得の終わりを受けたとき（成功・一部失敗・失敗、U5 で足す中断を含む、閉じる＝Aborted を除く）。取得の作業が異常に止まって失敗として終えたとき（今の abort_fetch_with_failure）も同じ
    logic: IF 取得中で、終わりがいま表示中の取得のもの THEN finalElapsedMs = max(0, 今 − startedAt) をミリ秒で決める。キャッシュへの保存は終わりより前に行われるので、保存の時間も含む。IF すでに finalElapsedMs がある、または古い取得の終わり THEN 変えない（今の取得の終わりの受け取りと同じ判定に従う）
    violation: なし
    source: FR4.3、FR4.4、ADR-005、refined-mockups 画面 E
  - id: BR4.3
    statement: 閉じるために止めた取得（Aborted）では、経過を出さない
    category: policy
    applies_to: AppSession、ElapsedRecord
    trigger: 取得の終わりの状態が Aborted のとき
    logic: IF 終わりが Aborted THEN ElapsedRecord を無しにする（今と同じく段階は取得前に戻り、件数も 0 になる）
    violation: なし
    source: FR4.3（対象は成功・一部失敗・失敗・中断）、ADR-009
  - id: BR4.4
    statement: 前の取得の結果が消えるときは、経過も一緒に消す
    category: policy
    applies_to: AppSession、ElapsedRecord
    trigger: 前の結果（件数・最後の結果）を消して取得前の表示に戻すとき。今は接続（プロファイル・リージョン）の変更がこれに当たる
    logic: IF 前の結果を消す THEN ElapsedRecord を無しにする。IF 同じ接続でロググループを選び直すだけ（ログも件数も残る） THEN 経過も残す。取得中に接続は変えられないので、取得中の経過が消えることはない
    violation: なし
    source: Q2、FR4.3
  - id: BR4.5
    statement: 画面へは、SessionView を作るたびに経過をその時点の値で渡し、秒ごとには送らない
    category: constraint
    applies_to: ElapsedView、SessionView.elapsed、FetchProgressUpdate
    trigger: SessionView を作るとき
    logic: IF ElapsedRecord が無い THEN elapsed は空。ELSE IF finalElapsedMs が無い THEN running = true、elapsedMs = max(0, 今 − startedAt)。ELSE running = false、elapsedMs = finalElapsedMs。fetch-progress（FetchProgressUpdate）には経過を足さず、経過のためだけに SessionView を送ることもしない
    violation: なし
    source: FR4.2、ADR-005、C5
  - id: BR4.6
    statement: 取得中の経過は、画面が自分の戻らない時計で 1 秒ごとに進め、新しい値を受け取るたびに合わせ直す
    category: policy
    applies_to: ElapsedTicker、ScreenHooks
    trigger: ElapsedView を受け取ったとき、および running の間 1 秒ごと
    logic: ElapsedView を受け取ったら baseMs = elapsedMs、receivedAt = 今（画面の戻らない時計）、running を写す。running の間は 1 秒ごとに描き直し、表示する経過 = baseMs + (今 − receivedAt)。running でなくなったら時計を止め、baseMs（最終の値）を出す。elapsed が空になったら時計を止めて何も出さない。再試行の待ちなど、ライブラリから何も届かない間も進み続ける
    violation: なし（ライブラリの値との小さなずれは ADR-005 で認めている）
    source: FR4.2、AC2.1.2、ADR-005
  - id: BR4.7
    statement: 経過は秒未満を切り捨て、1 時間未満は mm:ss、1 時間以上は h:mm:ss で表す
    category: calculation
    applies_to: ElapsedText.time（表記の関数）
    trigger: 経過を表示するとき
    logic: s = floor(経過ミリ秒 ÷ 1000)（負なら 0）。IF s < 3600 THEN 分 = floor(s ÷ 60)、秒 = s mod 60 を 2 桁ずつ「mm:ss」。ELSE 時 = floor(s ÷ 3600)（桁を埋めない）、分・秒を 2 桁ずつ「h:mm:ss」。例：0 → 00:00、7,999 ms → 00:07、59:59 → 59:59、3600 秒 → 1:00:00、75 分 → 1:15:00、10 時間 → 10:00:00
    violation: なし
    source: FR4.5、AC2.1.5
  - id: BR4.8
    statement: 経過は、状態行の今の内容の後ろに「経過 {time}」／「Elapsed {time}」として、取得中と取得後のどの段階でも出す
    category: policy
    applies_to: StatusLine、MessageCatalog
    trigger: 状態行を描くとき
    logic: IF elapsed が空、または段階が取得前（Idle） THEN 出さない。ELSE 今の内容（一覧を読んでいる・取得中・キャッシュに保存中・件数・0 件・失敗の件数とリンク・一覧が途中までの注意・失敗の文・キャッシュの通知）は変えずに、その後ろ（ログの絞り込みの表示より前）に経過を出す。文言は OS の言語で英語か日本語を選ぶ
    violation: なし
    source: FR4.1、FR4.3、FR4.4、NFR9、AC2.1.1、AC2.1.6、refined-mockups の状態と見た目の対応
  - id: BR4.9
    statement: 取得中の経過の進みは読み上げず、取得が終わったときは最終の経過も状態表示の変化と一緒に読み上げる
    category: constraint
    applies_to: StatusLine
    trigger: 経過を描き直すとき、取得が終わったとき
    logic: IF running THEN 経過の部分は読み上げの対象から外す（秒が進んでも読み上げが起きない）。IF running でない THEN 経過の部分を読み上げの対象に戻し、終わったときの状態表示の変化（今の role="status" の控えめな読み上げ）に最終の経過も含める
    violation: なし
    source: FR4.6、AC2.1.7、Q1
  - id: BR4.10
    statement: 経過の時計は戻らない時計で数え、テストでは差し替えられるようにする
    category: constraint
    applies_to: AppSession、ScreenHooks
    trigger: 経過の時刻を読むとき
    logic: ライブラリは PC の時刻ではなく戻らない時計を使い、AppSession の時計はテストで決まった時刻を返すものに差し替えられる。画面も戻らない時計（ブラウザの performance.now に当たるもの）を使い、テストで差し替えられる。差が負になる場合は 0 として扱う
    violation: なし
    source: interaction-spec の ElapsedTime（Format）、NFR6
```

## まとめ

| ID | 内容 | 種類 |
|----|------|------|
| BR4.1 | 取得中に移った時点を始まりにする | policy |
| BR4.2 | 終わった時点で最終の値を決め、次の取得まで残す | calculation |
| BR4.3 | 閉じて止めた（Aborted）ときは出さない | policy |
| BR4.4 | 前の結果が消えるとき（接続の変更）は一緒に消す | policy |
| BR4.5 | SessionView を作るたびに渡し、秒ごとには送らない | constraint |
| BR4.6 | 取得中は画面の時計で 1 秒ごとに進め、受け取るたびに合わせ直す | policy |
| BR4.7 | mm:ss / h:mm:ss、秒未満は切り捨て | calculation |
| BR4.8 | 状態行の今の内容の後ろに「経過 {time}」を出す | policy |
| BR4.9 | 取得中は読み上げず、終わったら最終の値も読み上げる | constraint |
| BR4.10 | 戻らない時計で数え、テストで差し替える | constraint |

## Assumptions & Open Questions

None.
