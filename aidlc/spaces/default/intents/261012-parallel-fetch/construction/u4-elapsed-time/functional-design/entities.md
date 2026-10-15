# Entities — U4 かかった時間（u4-elapsed-time）

上流：`inception/domain-design/components.md`（AppSession の FetchSessionView、ScreenHooks、StatusLine、MessageCatalog）・`decisions.md`（ADR-005）、`inception/contract-design/contract-summary.md`（C4、C5 の `elapsed`）、`inception/units-generation/unit-of-work.md`（U4）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR9）、`inception/refined-mockups/interaction-spec.md`（ElapsedTime）。質問票の回答は `functional-design-questions.md` の Q1・Q2 とまとめの確認。

U4 で足すのは、ライブラリ側の経過の記録（ElapsedRecord）、画面へ渡す形（ElapsedView）、画面の中だけの時計の状態（ElapsedTicker）の 3 つである。FetchJob には足さない（C4）。

## エンティティ（機械可読）

```yaml
entities:
  - name: ElapsedRecord
    owner: AppSession
    description: 取得 1 回にかかった時間の記録。値の持ち主はライブラリ（ADR-005）。components.md の FetchSessionView の startedAt・finishedAt・elapsedMs に当たる
    attributes:
      - name: startedAt
        type: instant
        required: true
        constraints: 戻らない時計の時刻。［取得］の押下を AppSession が受けて取得中に移った時点（BR4.1）。PC の時刻を変えても動かない
      - name: finalElapsedMs
        type: integer
        required: false
        min: 0
        constraints: 取得が終わった時点の経過（ミリ秒、0 以上）。取得中は無い。終わり（BR4.2）で一度だけ決まり、次の取得を始めるまで変わらない
    constraints:
      - AppSession が持つのは 0 個か 1 個。一度も取得していない、閉じて止めた（BR4.3）、前の結果が消えた（BR4.4）ときは無い
      - 時計は戻らない時計を使い、テストでは差し替えられる（NFR6）
      - 秘密の情報を含まない（時刻と長さだけ）
    relationships:
      - target: FetchJob
        cardinality: 1 対 0..1
        direction: ElapsedRecord → FetchJob（同じ取得 1 回の経過。FetchJob の型には項目を足さない、C4）

  - name: ElapsedView
    owner: AppSession
    description: 画面へ渡す経過。SessionView の項目 `elapsed`（C5）。秒ごとには送らず、SessionView を作るたびにその時点の値を入れる
    attributes:
      - name: running
        type: boolean
        required: true
        constraints: ElapsedRecord に finalElapsedMs が無ければ true、あれば false
      - name: elapsedMs
        type: integer
        required: true
        min: 0
        constraints: running のときは SessionView を作った時点までの経過（今 − startedAt）。running でなければ finalElapsedMs
    constraints:
      - ElapsedRecord が無ければ、SessionView の `elapsed` は空（null）
      - 項目名は camelCase（`running`、`elapsedMs`）
    relationships:
      - target: ElapsedRecord
        cardinality: 1 対 1
        direction: ElapsedView → ElapsedRecord（写し）

  - name: ElapsedTicker
    owner: ScreenHooks
    description: 画面の中だけの時計の状態。running の間、受け取った ElapsedView から 1 秒ごとに表示を進める（ADR-005）
    attributes:
      - name: baseMs
        type: integer
        required: true
        min: 0
        constraints: 最後に受け取った ElapsedView の elapsedMs
      - name: receivedAt
        type: instant
        required: true
        constraints: その ElapsedView を受け取った時点の画面の戻らない時計の時刻
      - name: running
        type: boolean
        required: true
        constraints: 最後に受け取った ElapsedView の running
    constraints:
      - 表示する経過 = running なら baseMs + (今 − receivedAt)、running でなければ baseMs
      - 新しい ElapsedView を受け取るたびに baseMs・receivedAt・running を置き換える（合わせ直し）
      - ElapsedView が空なら ElapsedTicker も無く、時計を止める

  - name: ElapsedText
    owner: ScreenHooks（表記の関数）、MessageCatalog（文言）
    description: 状態行に出す経過の文字列
    attributes:
      - name: time
        type: string
        required: true
        constraints: 経過を秒未満切り捨てで表す。1 時間未満は mm:ss（2 桁ずつ）、1 時間以上は h:mm:ss（時は必要な桁だけ、分・秒は 2 桁）（BR4.7）
      - name: label
        type: string
        required: true
        allowed_values: ["経過 {time}", "Elapsed {time}"]
        constraints: OS の言語で出し分ける（NFR9）
```

## まとめ

| エンティティ | 持ち主 | 役目 |
|--------------|--------|------|
| ElapsedRecord | AppSession | 始まりと最終の値。戻らない時計で記録する |
| ElapsedView | AppSession → 画面 | SessionView の `elapsed`。状態が変わるときだけ送る |
| ElapsedTicker | ScreenHooks | 取得中に画面の時計で 1 秒ごとに進める |
| ElapsedText | ScreenHooks・MessageCatalog | mm:ss / h:mm:ss と英日の文言 |

## Assumptions & Open Questions

None.
