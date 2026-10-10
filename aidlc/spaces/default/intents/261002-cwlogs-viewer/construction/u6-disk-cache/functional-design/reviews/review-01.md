## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-06T09:20:35Z
**Iteration:** 1

NOT-READY

### Findings

場所の基点は `aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u6-disk-cache/functional-design/`（以下 `FD/`）。Q1〜Q5 の人間の回答と BR5.4 は再検討していない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | FD/functional-spec.md UC2 の 4・6、FD/rules.md BR3.5・BR2.3、FD/entities.md FetchJob・SessionState | 既存の `coordinator.rs` では、`run_fetch` が最後に自分で `sink.on_finished(&job, ..)` を呼んで終わる。ところが BR3.5 は「取得ジョブを終える前に書き込む」「書き込みのあいだも取得中のまま」「結果を FetchJob.cacheOutcome に持つ」と定める。`cacheOutcome` が決まるのは書き込みのあと。しかし `on_finished` は `run_fetch` の内側で先に出てしまい、UI は Done に移ってしまう。「キャッシュに保存中」を UI に届ける手段（`FetchSink` のイベントも `FetchProgress` の項目も `SessionState` の属性も）が entities.md にも rules.md にもない。Hit の経路も、U3:BR5.4 の通知の順（started → … → finished）のどれを出すか、`timeline.discard()` をいつ呼ぶか（BR5.6・BR4.5）、`on_batch` を出すか、が書かれていない。AppSession は `current_job_id` が空の間は `finish_fetch` を無視するため、Hit が `on_started` を出さないと Fetching のまま固まりうる。開発者が推測で決めることになる。 | キャッシュが有効なときの通知の順を仕様として決める。(a) Hit：`discard` → `on_started` → 追加（`on_batch` の有無を決める）→ `on_finished`。(b) AWS から取得：`run_fetch` の `on_finished` は書き込みの結果が決まってから出す。そのため `on_finished` を遅らせる、または書き込みまでを `run_fetch` の中に入れるかを選ぶ。(c)「保存中」を伝える新しいイベント（例：`on_saving`）と、それを受ける `FetchProgress` か `SessionState` の項目。(d) `cacheOutcome` を `finish_fetch` に渡す経路。この 4 点を functional-spec の UC2 と entities.md に書き、U3 のイベント順の例外であることを明記する。 | New |
| R-02 | Major | FD/rules.md BR2.3・BR4.1・BR4.2、FD/functional-spec.md UC2 の 4・UC3 | BR4.2 でヘッダだけ先に読み、Hit のときにイベントを読む。BR4.1 の「読めない」はそのときに初めて分かりうる（イベント部の破損・途中で切れたファイル）。ところが、イベントを全部読み切って検証してから保持ログに追加するのか、読みながら追加するのかが書かれていない。読みながら追加すると、途中で壊れていると分かった時点で保持ログに一部のイベントが入っており、AWS から取り直すと重複や半端な表示になる（`EventTimeline` は重複を除かない）。「events は coveredRanges のどれかに入る」「並び順は U3:BR4.1」「(stream, timestamp, sequence) が重複しない」も BR4.1 の検査項目にない。 | BR2.3・BR4.1 に、Hit のイベントは全部読んで検証し終えてから 1 回で保持ログに追加する（検証に失敗したら何も追加しない）こと、検査項目に「イベントの時刻が coveredRanges に入る」「並びが U3:BR4.1 の順」を加えること、ReadFailed のあとの取り直しは保持ログが空の状態から始まることを書く。 | New |
| R-03 | Major | FD/rules.md BR3.3・BR4.1・BR4.2・BR3.4 | BR3.3 は「既存の CacheEntry から範囲内のイベントを捨てて足す」ため、書き込みの前に既存ファイルのイベント部を全部読む。しかし BR4.2 により、キャッシュに入らない取得では読み出し時にイベントを読まない。そのため既存ファイルの破損は書き込みの時点で初めて見つかりうる。BR4.1 は「BR2.3 で読むとき」しか扱っておらず、(1) 書き込みのための読み出しに失敗したら新しい結果だけで作り直すのか、書かずに SaveFailed にするのか、(2) ReadFailed 時の「ファイルを消す」が失敗したとき（権限・使用中）にどうするか、が未定義。書き込みが失敗する典型は破損したキャッシュなので、方針がないと一度壊れたキャッシュが回復しない可能性がある。 | BR3.3 に「既存ファイルが読めない・検証に失敗したら、既存を捨てて今回の範囲だけで作り直す（旧範囲は失う。通知は出さない、または ReadFailed を使う）」と決める。BR4.1 に、削除に失敗しても AWS から取得は続行し、書き込みの入れ替え（BR3.4）で置き換える旨を足す。テストの方針（§8）にも加える。 | New |
| R-04 | Minor | FD/rules.md BR4.1・BR3.5、FD/entities.md FetchJob.cacheOutcome、FD/functional-spec.md §3 の FetchJob.cacheOutcome 状態遷移 | `cacheOutcome` は 1 つの値だが、ReadFailed のあとに書き込んだ結果（Saved / SaveFailed / NotSaved）も出す。BR4.1 は「ReadFailed の知らせを優先する」とするが、状態遷移図は ReadFailed → FetchingFromAws → Saved / SaveFailed / NotSaved と進み、ReadFailed が消える。entities.md の「ReadFailed は読めずに取り直したとき」とも食い違う。その結果、取り直したあとの SaveFailed（ディスク満杯など）は利用者に伝わらない。 | 値を 1 つに潰すルールを決める。例：`readFailed: boolean` を別に持つ、または ReadFailed のとき SaveFailed も併記する。状態遷移図と entities.md の説明をそれに合わせる。 | New |
| R-05 | Minor | FD/rules.md BR4.1 と BR5.3、FD/functional-spec.md UC2 の 4・UC3・§4、functional-design-questions.md Q5 | ステータス行の文言がファイルごとに違う。ReadFailed は BR4.1 と Q5 が「キャッシュが読めなかったので取り直した」、BR5.3 と functional-spec が「…AWS から取り直した」。Hit は BR5.3 が「キャッシュから表示（AWS は呼んでいない）」、functional-spec は「キャッシュから表示」。テストと英日の文言が文字列で突き合わされるため、正が決まらない。 | 3 つの文言の正本（日本語と英語、文言キー）を BR5.3 に 1 か所だけ書き、ほかは参照に変える。Q5 の回答は人間の決定のため、Q5 の文言を正本にするのが安全。 | New |
| R-06 | Minor | FD/rules.md BR3.6・FD/entities.md CacheEntry.constraints・CachedEvent.message | BR3.6 は「秘密の認証情報・アクセスキー ID は書かない」と言うが、`CachedEvent.message` は API が返したままを書く。ログ本文に秘密の文字列が含まれていればキャッシュに残るため、この文は文字どおりには守れない（project.md Forbidden、NFR5 の対象は「アプリが扱う認証情報」と読むしかない）。FR7.2 の注意表示が実質の緩和策である点が書かれていない。 | 「アプリ自身の認証情報（SDK の資格情報・トークン・アクセスキー ID）はキャッシュに書かない。ログ本文は利用者のデータとして保存し、機密情報が含まれうる旨を FR7.2 の注意で示す」と範囲を明記する。テストは「資格情報の型・値をキャッシュに渡す経路がない」ことを確かめる形にする。 | New |
| R-07 | Minor | FD/rules.md BR3.2・BR3.5・BR3.3、FD/functional-spec.md §7 | 書き込みのためにタイムライン（`Mutex<EventTimeline>`）の全イベントを読む。BR3.5 の「取得中のロックを保つ」は UI の入力ロックを指すが、実装者が `Mutex` を保ったまま 100 万件を直列化すると、画面の行取得が数秒止まり、NFR2（スクロールに 1 秒以内に反応）に反する。また `LogCache` は sync の想定（components.md）で、100 万件のファイル I/O を async の取得タスクの上で直接行うとランタイムを塞ぐ。§7 の「数秒」は根拠のない見積もり。 | BR3.5 に、書き込みはタイムラインの `Mutex` を保たずに行うこと（範囲を絞った複製、または短い区間ごとの読み出し）、および I/O をブロッキング用のスレッドに出すことを書く。「数秒」は「測っていない」と書き換える。 | New |
| R-08 | Minor | FD/rules.md BR1.4・BR2.2・BR3.4・BR3.6、FD/entities.md CacheKey.profileKey | (1) [Clear cache] の「このアプリのキャッシュファイル」をどう見分けるか（ファイル名の型・拡張子・一時ファイルの名前）と、シンボリックリンクをたどらないことが未定義。(2) `profileKey` が enum-or-text のため、SdkDefault の「印」と、たまたま同じ名前の Named プロファイルがハッシュ上で衝突しうる。(3) ハッシュの種類（衝突しにくい暗号学的ハッシュ）の指定がない。(4) 設定ファイルの権限が BR3.6 の対象外。(5) フォルダが既にあって権限が緩いときの扱い（0700 に直すか）が未定義。 | BR1.4・BR2.2・BR3.6 に、(1) 名前の型で見分け、リンクをたどらない、(2) キーを kind と値のタグ付きで作る、(3) SHA-256 など、(4) 設定ファイルも 0600、(5) 既存フォルダが緩ければ 0700 に直す（直せなければ書かない）を足す。 | New |
| R-09 | Minor | FD/entities.md CacheSettings.components_md、FD/rules.md BR5.1、FD/functional-spec.md §8 | (1) entities.md は「components.md には名前がない」と書くが、components.md には CacheSettings（`settingsKey, enabled, location`）がある。属性名も食い違う（`cacheEnabled` / `settingsPath` / `cacheDirectory`）。(2) BR5.1「ダイアログを開いているあいだは、ほかの操作はできない」は UI だけのルールで、コア側（`begin_fetch` など）が `settingsDialog = Open` を拒むかが書かれていない（U2 の ConfirmationPending は拒む）。(3) AppSession が設定ファイルを読む（UC5）ため、既存の `AppSession::new()` を使う多数のテストが利用者の実フォルダに触れないよう、パスを外から渡す形（注入）が必要だが、設計に書かれていない。§8 の「実フォルダに触れない」と整合しない。 | (1) components.md の名前と対応表を書き直す（またはリネームの理由を明記）。(2) `settingsDialog = Open` のあいだ `begin_fetch` などを拒むか、UI だけで足りるとする理由を決めて BR5.1 に書く。(3) LogCache のパス（設定・キャッシュ）を外から渡す前提と、既定のコンストラクタは無効・ディスクに触れないことを spec に書く。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この単位に機能設計用の検証ツールの指定はなし） | 実行せず | 代わりに既存コード（`coordinator.rs`、`session.rs`、`timeline.rs`、`event.rs`、`request.rs`、`streams/mod.rs`、`LogFilterInput.tsx`）と照らして確認した。R-01 は `run_fetch` が `on_finished` を内部で出すこと、`finish_fetch` が `current_job_id` に依存することから、R-02 は `EventTimeline::append` が重複を除かないことから確認した。BR5.4 は既存の `LogFilterInput.tsx`（Enter は `preventDefault` のみ・0.3 秒の待ち）に対して実装可能。 |

### Summary

FR7.1〜FR7.9 は rules.md の BR と traceability.json で漏れなく対応しており、Q1〜Q5 と BR5.4 の反映、プロジェクトの禁止事項（読み取り 3 API のみ・認証情報・CI）との整合も取れている。ただし、既存の `run_fetch`／`FetchSink` の通知の順とキャッシュの書き込みが両立しない点（R-01）、読み出し・書き込みで破損を見つけたときの手順が未定義な点（R-02、R-03）は、開発者が推測で決めることになるため Major とした。この 3 件を直せば READY に近い。他は Minor。

なお、他の作業単位の機能設計（U1〜U5 の rules.md）はレビューの範囲外のため開けず、`U1:BR1.4`・`U1:BR1.8`・`U3:BR4.4` などの参照先は、既存コード側の記述（`event.rs`・`LogFilterInput.tsx` のコメント）での裏取りにとどまる。

NOT-READY
