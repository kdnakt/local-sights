# Interaction Spec — ロググループ全体の取得の高速化

形式は `.claude/knowledge/aidlc-design-agent/component-spec-template.md` に従う。画面幅による切り替えは、デスクトップアプリのため扱わない（各部品の Responsive Behaviour は「なし」）。

---

## FetchCancelButton（［取得］／［中断］ボタン）

| Field | Value |
|---|---|
| Component | FetchCancelButton（今の［取得］ボタンを拡張） |
| Description | 取得前は取得を始め、取得中は取得を中断する、同じ場所の 1 つのボタン |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default（取得） | 「取得」／「Fetch」。primary | 取得していないとき |
| disabled（取得） | 条件が足りず押せない（今と同じ） | 条件が足りない、ダイアログが開いている |
| default（中断） | 「中断」／「Cancel」。primary | 取得を始めた（一覧を読んでいる・ストリームを読んでいる） |
| loading（中断中） | 回る印と「中断中…」／「Cancelling…」。押しても何も起きない | ［中断］を押した後、止まるまで |
| disabled（中断） | 「中断」のまま押せない | キャッシュに保存中、キャッシュから表示中 |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| phase | `Idle` \| `Listing` \| `Fetching` \| `Cancelling` \| `SavingCache` \| `ReadingCache` | yes | — | 取得の段階。ボタンの文言と押せるかを決める |
| canSubmit | boolean | yes | — | 取得前に押せるか（今と同じ判定） |
| onFetch | () => void | yes | — | 取得を始める（今と同じ） |
| onCancel | () => void | yes | — | 取得を中断する。押してすぐ `Cancelling` に移る |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | button（Cloudscape Button） |
| Keyboard interaction | Tab で同じ順に移る。Enter / Space で押す |
| Label / aria-label | 見える文言がそのまま名前（取得／中断／中断中…） |
| Contrast ratio | Cloudscape の primary ボタンに従う（WCAG AA） |
| Screen reader | 文言が変わったとき、ボタンにフォーカスがあれば新しい名前が読まれる |
| Focus management | 取得を始めても、中断中になっても、止まって［取得］に戻っても、フォーカスは同じボタンに残る |

### Behaviour

- 押した直後に `Cancelling` に移り、以後の押下は無視する（二度押しの防止、FR5.2）。
- 中断を受け取ったら、新しい呼び出しは 1 回も始めない。出ている呼び出しの応答を待って止まる（NFR4）。再試行の待ちの間ならすぐ止まる。
- 最後のストリームを読み終えた後に中断が届いたら、終わった扱いにする（FR5.9）。
- 取得中にウィンドウを閉じたときの確認と動きは今と同じ（FR5.10）。

---

## ElapsedTime（状態表示の経過）

| Field | Value |
|---|---|
| Component | ElapsedTime（今の StatusLine に足す） |
| Description | 取得を始めてからの経過時間 |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| running | 1 秒ごとに進む | 取得中（一覧・ストリーム・中断中・キャッシュ保存中・キャッシュ読み込み中） |
| final | 最終の時間で止まる | 取得が終わった（成功・一部失敗・失敗・中断） |
| empty | 表示しない | 起動後、まだ一度も取得していない |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| startedAt | 取得を始めた時刻 | yes（取得後） | — | ［取得］を押した時点 |
| finishedAt | 取得が終わった時刻 | no | — | あれば final |

### Format

- 1 時間未満は `mm:ss`（例：`00:07`）、1 時間以上は `h:mm:ss`（例：`1:15:00`）（FR4.5）。
- 文言は「経過 {time}」／「Elapsed {time}」。
- 時計は戻らない時計で数える（PC の時刻を変えても負にならない）。

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | 状態表示（今の `role="status"`）の中の 1 項目 |
| Screen reader | 秒の変化では読み上げない（経過の部分は読み上げの対象から外す、FR4.6）。取得が終わったときの状態表示の変化は今と同じく読み上げる |
| Contrast ratio | 状態表示の文字色に従う |

---

## CancelNotice（中断の注意）

| Field | Value |
|---|---|
| Component | CancelNotice |
| Description | 中断した後に、読み終えていないストリームの本数を知らせる |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| hidden | 出さない | 中断していない |
| afterListing | 「取得を中断しました。{n} 本のストリームは読み終えていないため、一部のログが含まれていません。」 | 一覧を読み終えた後に中断 |
| beforeListing | 「ストリームの一覧を読み終える前に中断したため、一部のログが含まれていません。」 | 一覧の途中で中断 |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| unfinishedStreams | number \| null | yes | — | 読み途中と未着手の合計。一覧の前なら null |

### Behaviour

- ログの表の上、今のエラーの表示と同じ場所に出す。取得そのもののエラーと同時には起きない（中断した取得は「失敗」にならない）。
- 次の取得を始めたとき、接続・ロググループを変えたときに消える（FR5.7）。閉じるボタンは付けない（取れていないことを見落とさないため）。

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `role="status"`（Cloudscape Alert を status の囲みに入れる、今の成功の通知と同じ作り） |
| Screen reader | 出たときに一度だけ読み上げる（FR5.11） |
| Contrast ratio | Cloudscape の warning に従う。色だけでなく注意のアイコンと文で伝える |
| Focus management | フォーカスは動かさない（ボタンに残す） |

---

## 文言の一覧（英語 / 日本語）

| 用途 | English | 日本語 |
|------|---------|--------|
| 中断ボタン | Cancel | 中断 |
| 中断中 | Cancelling… | 中断中… |
| 経過 | Elapsed {time} | 経過 {time} |
| 中断の状態 | Cancelled | 中断 |
| 中断の注意（一覧の後） | Fetch cancelled. {n} streams were not finished, so some logs are missing. | 取得を中断しました。{n} 本のストリームは読み終えていないため、一部のログが含まれていません。 |
| 中断の注意（一覧の前） | Fetch cancelled before the stream listing finished, so some logs are missing. | ストリームの一覧を読み終える前に中断したため、一部のログが含まれていません。 |

## Assumptions & Open Questions

None.
