# Interaction Spec — 画面を AWS マネジメントコンソールに寄せる

上流：`inception/requirements-analysis/requirements.md`（FR1〜FR4、NFR2、NFR3）、`mockups.md`（画面 1〜7）、質問票 Q1〜Q6。部品の仕様は `.claude/knowledge/aidlc-design-agent/component-spec-template.md` の形式に沿う。コア（Rust）とのやり取りの変更点は §8 にまとめる。

## 1. TopBar（上部ナビゲーション）

| Field | Value |
|---|---|
| Component | TopBar |
| Description | アプリ名と、プロファイル・リージョン・タイムゾーンの選択、設定ボタンを持つ黒い上部ナビゲーション（Q1） |
| Category | navigation |

### States

| State | Description | Trigger |
|---|---|---|
| default | すべてのドロップダウンが使える | 起動時、取得していないとき |
| disabled | 接続のドロップダウン（プロファイル・リージョン）と設定ボタンが無効。タイムゾーンは使える | 取得中、接続変更の確認待ち、設定ダイアログ表示中（MVP の振る舞い） |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| profiles | ProfileSelector[] | yes | — | SDK 既定＋設定ファイルのプロファイル |
| regions | string[] | yes | — | 公開リージョン一覧 |
| timeZone | "Local" \| "Utc" | yes | — | 表示・入力のタイムゾーン |
| canChangeConnection | boolean | yes | — | 接続のドロップダウンと設定ボタンの有効・無効 |
| onSelectProfile / onSelectRegion / onSelectTimeZone / onOpenSettings | callbacks | yes | — | 既存の App のハンドラをそのまま使う |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | TopNavigation が `banner` と `navigation` を持つ。アプリ名は h1 相当 |
| Keyboard interaction | Tab でドロップダウンを順に移動、Enter／Space で開く、矢印で項目移動、Enter で選択、Escape で閉じる |
| Label / aria-label | 各ドロップダウンに「Profile」「Region」「Time zone」、設定ボタンに「Settings」の aria-label |
| Focus management | 設定ダイアログを閉じたら焦点は歯車ボタンに戻る（MVP U6 の振る舞い） |

## 2. LogGroupPane（左ペイン）

| Field | Value |
|---|---|
| Component | LogGroupPane |
| Description | 見出し「Log groups」の囲みに、検索欄、1 行選択の表、[Reload] を置く（Q6） |
| Category | navigation / display |

### States

| State | Description | Trigger |
|---|---|---|
| prompt | 表の空の状態に「プロファイルとリージョンを選んでください」 | 接続が未選択 |
| loading | 表の読み込み中表示 | 一覧の取得中 |
| populated | 行が並び、選択中の行にラジオの印 | 一覧あり |
| empty | 「ロググループがありません」 | 一覧が 0 件、または検索で 0 件 |
| partial | 表の上に warning の Alert「一覧は一部です」＋表は取れた分 | 一覧の取得が途中で失敗 |
| error | 表の空の状態に、何が起きたか・次の行動・詳細 | 一覧の取得が失敗 |
| disabled | 行を選べない。検索欄は使える | 取得中・確認待ち |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Table（`grid`）と、選択列のラジオ。見出しは h2 |
| Keyboard interaction | 表に Tab で入り、↑↓ で行を移動、Space で選択。検索欄は Tab の順で表の前 |
| Screen reader | 選択の変更を Table の `ariaLabels.selectionGroupLabel` で読み上げ |

## 3. ConditionArea（右ペインの見出しと条件行）

| Field | Value |
|---|---|
| Component | ConditionArea |
| Description | 見出し（ロググループ名）と [Fetch]、絞り込み欄、開始・終了の日付・時刻欄（Q2） |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | 入力できる。[Fetch] は条件が揃えば有効 | 取得していないとき |
| invalid | 不備のある欄の直下に理由。[Fetch] 無効 | コアの validationErrors が空でない、または画面側の形式チェックに失敗 |
| fetching | 日付・時刻・[Fetch] 無効。絞り込み欄は使える | 取得中 |
| locked | すべて無効 | 確認待ち・設定ダイアログ表示中 |

### 日付欄（DateField）と時刻欄（TimeField）の振る舞い

| 操作 | 結果 |
|---|---|
| カレンダーで日付を選ぶ | 日付欄に `yyyy/mm/dd` が入り、コアに送る（§8） |
| 日付欄に `2024/03/01` と打つ | 同上 |
| 日付欄に `2024-03-01` や `2024/02/30` を打つ | 欄の直下に「yyyy/mm/dd の形で入力してください」。[Fetch] 無効（FR2.4） |
| 日付欄を空にする | 欄の直下に「日付を入力してください」。[Fetch] 無効（FR1.4） |
| 時刻欄に `10:00` と打って焦点を外す | 開始なら `10:00:00`、終了なら `10:00:59` に書き換えてコアに送る（FR1.3、Q5） |
| 時刻欄を空のまま焦点を外す | 開始なら `00:00:00`、終了なら `23:59:59` に書き換えてコアに送る（Q4、Q5） |
| 時刻欄に `25:00:00` を打つ | 欄の直下に「hh:mm:ss の形で入力してください」。[Fetch] 無効 |
| 存在しないローカル時刻（夏時間の飛び） | コアの判定を時刻欄の直下に出す（FR1.7） |
| 開始が終了より後 | 終了の日付欄の直下に「終了は開始より後にしてください」（FR3.1） |
| タイムゾーンを切り替える | コアが書き換えた文字列を日付と時刻に分けて両欄に反映する（FR1.5） |
| Enter（どの欄でも） | [Fetch] が有効なら取得を始める（MVP の振る舞い） |

### Accessibility

| Requirement | Implementation |
|---|---|
| Label / aria-label | FormField の label に「Start (Local)」「End (Local)」、日付欄と時刻欄にそれぞれ aria-label「Start date」「Start time」 |
| Keyboard interaction | DatePicker：Tab で欄、カレンダーボタンで開く、矢印で日を移動、Enter で選択、Escape で閉じる。TimeInput：数字入力、Tab で次へ |
| Screen reader | errorText は FormField が `aria-describedby` で欄に結びつけ、変更時に読み上げる |

## 4. LogTable（ログ一覧）

| Field | Value |
|---|---|
| Component | LogTable |
| Description | 時刻・ストリーム名・メッセージの表。行の展開で全文を表示（Q3、FR4.3、FR4.4） |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| empty | 表の中に「イベントがありません」と次の操作の案内 | 取得前、0 件 |
| populated | 行が並ぶ。時刻は `yyyy/mm/dd hh:mm:ss.SSS` | 取得済み |
| expanded | 行の直下に全文（等幅、折り返し、20 行まで、超えると中でスクロール、選択してコピー可） | 行を押す、Enter／Space |
| filtered | 絞り込みの該当行だけ | 絞り込み欄に文字 |
| growing | 取得中に行が増える | 取得中 |

### 実装方針（Q3）

1. Cloudscape の Table で作り、`expandableRows` または行直下の描画で展開を実現する。
2. 100 万行のダミーで NFR2 を測る：絞り込み 100 秒以内、スクロールと展開に 1 秒以内に反応、落ちない。
3. 満たせなければ、今の自前の仮想スクロール（`LogTable.tsx`、`rowLayout.ts`）を保ち、見出し・罫線・配色・余白・文字に Cloudscape のデザイントークン（`@cloudscape-design/design-tokens`）を当てる。見た目は mockups.md の画面 4 と同じにする。
4. どちらにしたかと測った数値は Code Generation の計画に書き、SM1 の例外として要件の承認時に確認する（要件レビュー R-01）。

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `grid`。展開部分は `region` で見出しを持つ |
| Keyboard interaction | Tab で表に入り、↑↓ で行、Enter／Space で展開・閉じる（MVP U7 の振る舞い） |
| Focus management | 展開しても焦点は行に残る。展開部分の文字は Tab で入って選択できる |

## 5. ErrorAlert（取得全体の失敗）

| Field | Value |
|---|---|
| Component | ErrorAlert |
| Description | 表の直上の Alert（type=error、閉じるボタン付き）。何が起きたか（header）、次の行動、安全な詳細（FR3.2） |
| Category | feedback |

| State | Description | Trigger |
|---|---|---|
| shown | 3 行構成で表示、`role="alert"` | 取得全体が失敗 |
| dismissed | 消える。次の取得で再び出る | [x]、Escape |

## 6. FailureListDialog・SettingsDialog・ConfirmDialog

- すべて Cloudscape の Modal。ヘッダーに見出し、右上に閉じるボタン、フッターに右寄せのボタン（主ボタンは右端）。
- 焦点は開いたときにダイアログ内の最初の操作部品へ、閉じたときに開いたボタンへ戻る。Escape で閉じる（MVP U6・U7 の振る舞い）。Modal が焦点を内側に閉じ込める。
- FailureListDialog の中身は Table（列：Stream、What happened / what to do）。SettingsDialog の中身は Checkbox（キャッシュ）、保存場所の文字、注意文、[Clear cache]・[Cancel]・[Save]。

## 7. StatusLine

- 表の下の Box。取得中は StatusIndicator（loading）＋文字、失敗後は StatusIndicator（error）＋「何が起きたか」の短い文、完了は件数の文字。失敗一覧を開くボタンは Cloudscape の Button（variant=link）。内容は MVP のまま（FR3.3）。

## 8. コアとのやり取りの変更点

| 項目 | 今 | 変更後 |
|---|---|---|
| 日時の文字列の形 | `yyyy-mm-dd hh:mm:ss`（`INPUT_FORMAT`） | `yyyy/mm/dd hh:mm:ss`。コアの解析（`parse_in_zone`）と書き戻し（`format_input_in_zone`）を変える。ハイフン区切りは不正（FR2.4） |
| 一覧の時刻の形 | `yyyy-mm-dd hh:mm:ss.SSS`（`DISPLAY_FORMAT`） | `yyyy/mm/dd hh:mm:ss.SSS`（FR2.2） |
| 画面からコアへ渡す入力 | 開始・終了それぞれ 1 つの文字列 | 変えない。画面側が日付欄と時刻欄を `日付 + " " + 時刻` に結合して渡し、コアが返す文字列を最初の空白で分けて両欄に戻す |
| 補完（秒・空の時刻） | なし | 画面側（焦点が外れたとき）で行い、補完後の文字列をコアに渡す（Q4、Q5） |
| 形式の判定 | コアの `validation.startFormat` 等 | 画面側で日付欄・時刻欄ごとに形式を先に判定して欄の直下に出す。コアの判定（存在しない時刻、順序、ロググループ）は該当欄に紐づける（mockups.md 画面 5a の表） |
| 「取得できない理由」の一覧 | `form.reasons.title`＋箇条書き | 廃止。validationErrors の各 key を欄に割り当てる表を画面側に持つ |

## 9. 文言（英・日）

| key | en | ja |
|---|---|---|
| 上部：Profile / Region / Time zone / Settings | Profile / Region / Time zone / Settings | プロファイル / リージョン / タイムゾーン / 設定 |
| 左ペインの見出し・検索 | Log groups / Find log groups | ロググループ / ロググループを検索 |
| 右ペインの見出し（未選択） | Select a log group | ロググループを選んでください |
| [Fetch] / [Reload] | Fetch / Reload | 取得 / 再読み込み |
| 絞り込み欄 | Filter events | イベントを絞り込み |
| 日付・時刻のラベル | Start ({zone}) / End ({zone}) | 開始（{zone}）/ 終了（{zone}） |
| 日付の形式の誤り | Enter the date as yyyy/mm/dd. | 日付は yyyy/mm/dd の形で入力してください。 |
| 時刻の形式の誤り | Enter the time as hh:mm:ss. | 時刻は hh:mm:ss の形で入力してください。 |
| 日付が空 | Enter a date. | 日付を入力してください。 |
| 順序の誤り | End must be after start. | 終了は開始より後にしてください。 |
| 表の空の状態 | No events. Select a log group, set a time range, then choose Fetch. | イベントがありません。ロググループを選び、期間を指定して [取得] を押してください。 |
| 既存のエラー文言（error.what.* / error.next.*） | 変えない | 変えない |
