## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-07T01:02:12Z
**Iteration:** 2

READY

場所の基点は `aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u6-disk-cache/functional-design/`（以下 `FD/`）。Q1〜Q5 と BR5.4 は再検討していない。既存コード（`crates/local-sights-core/src/coordinator.rs`、`session.rs`、`streams/`）と照らして確認した。

### 前回の指摘 R-01〜R-09 の解消状況

| ID | 前回の重大度 | 結果 | 確認した内容 |
|---|---|---|---|
| R-01 | Major | Resolved | BR3.7 と spec §4 が、Hit と AWS 取得（書き込みあり・なし）と途中終了の知らせの順を定めた。新しい `on_saving`、`SessionState.cacheSaving`、`cacheOutcome` を載せた `FetchJob` を `on_finished` で渡す経路（`cacheNotices`）が揃った。Hit は discard → on_started → 1 回の追加 → on_batch → on_finished で、`finish_fetch` が `current_job_id` を要する点（`session.rs` 808 行付近）とも矛盾しない。`run_fetch` が `on_finished` を内側で出す件の改修方法は設計に書かれていないが、Code Generation で決められる範囲（R-12 に記録）。 |
| R-02 | Major | Resolved | BR2.3・BR4.1・UC2 の 4.1 が、全部読んで検証し終えてから保持ログを破棄して 1 回で追加すること、検査項目に範囲の形・範囲外のイベント・並びを加えること、取り直しは空から始まることを定めた。 |
| R-03 | Major | Resolved | BR3.3 が、書き込み時に既存が読めなければ作り直すこと（前の範囲は失う）を定めた。BR4.1 が、削除に失敗しても取得を続け、BR3.4 の入れ替えで置き換えることを定めた。テストの方針（§9）にも入った。 |
| R-04 | Minor | Resolved | `FetchJob.readFailed` を別に持ち、BR5.3 が ReadFailed と SaveFailed の併記を定めた。状態遷移図の text fallback もこれに合う。 |
| R-05 | Minor | Resolved | 文言の正本は BR5.3 だけになり、spec と BR4.1 は参照に変わった。Q5 の文言とも一致する。 |
| R-06 | Minor | Resolved | BR3.6 が「アプリ自身が扱う認証情報」に範囲を限り、ログ本文は利用者のデータとして保存し、注意表示で委ねると明記した。テストは書く経路がないことで確かめる形になった。 |
| R-07 | Minor | Resolved | BR3.5 が、ロックを握らず短い区切りで写し取ること、I/O をブロッキング用のスレッドに出すこと、取得中のロックは UI の入力ロックであることを定めた。§8 の「数秒」は「測っていない」に直った。 |
| R-08 | Minor | Resolved | BR1.4（名前の型で見分ける・リンクをたどらない）、BR2.1・BR2.2（kind 付きのキー・SHA-256）、BR3.6（0600・0700、緩いフォルダを直す）が入り、`CacheKey.profileKind` / `profileName` も定まった。 |
| R-09 | Minor | Resolved | entities.md に components.md との対応が書かれ、BR5.1 がダイアログ中にライブラリ側でも拒むことを定めた。BR1.6 が場所を外から渡し、既定の作り方ではディスクに触れないことを定めた。 |

### Findings

R-01〜R-09 はすべて Resolved のため、下の表では前回の行を ID のまま Resolved として引き継ぐ。新しい指摘は R-10 から始め、いずれも Minor（Critical・Major なし）。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | FD/rules.md BR3.7、FD/functional-spec.md §4 | 通知の順と `on_saving`・`cacheSaving`・`cacheOutcome` の経路が定まった | なし | Resolved |
| R-02 | Major | FD/rules.md BR2.3・BR4.1 | 全部検証してから 1 回で追加する手順になった | なし | Resolved |
| R-03 | Major | FD/rules.md BR3.3・BR4.1 | 書き込み時に既存が読めないときと削除失敗時の扱いが定まった | なし | Resolved |
| R-04 | Minor | FD/entities.md FetchJob.readFailed、FD/rules.md BR5.3 | `readFailed` を別に持ち併記できる | なし | Resolved |
| R-05 | Minor | FD/rules.md BR5.3 | 文言の正本が BR5.3 に一本化された | なし | Resolved |
| R-06 | Minor | FD/rules.md BR3.6 | 認証情報の範囲とログ本文の扱いが明記された | なし | Resolved |
| R-07 | Minor | FD/rules.md BR3.5 | ロックを握らない・ブロッキング用のスレッドで行う | なし | Resolved |
| R-08 | Minor | FD/rules.md BR1.4・BR2.2・BR3.6 | 名前の型・kind 付きのキー・SHA-256・権限が定まった | なし | Resolved |
| R-09 | Minor | FD/rules.md BR1.6・BR5.1、FD/entities.md CacheSettings | 注入と拒否が定まった | なし | Resolved |
| R-10 | Minor | FD/rules.md BR1.3、FD/functional-spec.md UC1 の 4・UC4 の 2 | 無効にして [Save] したときの手順が食い違う。BR1.3 は「設定を書けたらダイアログを閉じる、続けて全削除、削除に失敗したらダイアログに出す」と読め、閉じたあとの画面に出すことになってしまう。UC1 の 4 も「書けたら閉じる」。UC4 の 2 は「消せなかったときダイアログにその旨を出す」。順序が曖昧で、開発者が推測で決める。 | 順序を「設定を書く → 有効から無効なら全削除 → すべて成功したときだけ閉じる」に統一する。削除に失敗したときはダイアログを開いたまま（cacheEnabled は無効に更新済み）、消せなかった旨を出し、利用者が [Clear cache] で再試行するか閉じられるようにする。UC1 の 4 と BR1.3 と状態遷移図の説明をそろえる。 | New |
| R-11 | Minor | FD/entities.md SessionState.cacheSaving・cacheNotices、FD/rules.md BR3.7 | `cacheSaving` を false に戻すのは `on_finished` だけと定めている。既存の `abort_fetch_with_failure`（`session.rs` 863 行付近。取得のタスクが結果を返さず異常終了したとき Failed にする）は `progress` だけを捨てるため、保存中や Hit の読み出し中にブロッキング用のスレッドが panic すると、`cacheSaving = true` のまま Failed に移り、ステータス行に「保存中」が残りうる。`cacheNotices` も前回の値が残りうる。 | BR3.7 に、異常終了で取得を終えるときも `cacheSaving` を false にし `cacheNotices` を空にすることを足す（`on_finished` と同じ後始末）。テストの方針（§9）の AppSession に加える。 | New |
| R-12 | Minor | FD/functional-spec.md UC2 の 6、FD/rules.md BR3.5・BR3.7 | 既存の `run_fetch` は最後に自分で `sink.on_finished(&job, ..)` を呼ぶ。設計は「書き込みのあとに `on_finished`、`FetchJob` に `cacheOutcome` を入れる」としているが、`on_finished` を保留して書き込む位置（`run_fetch` を分けるか、FetchCoordinator の外側の関数でシンクを包むか）と、`on_saving` に `job_id` を渡し AppSession が `progress_of` と同じく現在のジョブだけを受け付けることは書かれていない。また Hit のジョブでは `planned_stream_count` と `finished_stream_count` が 0 になり、U3 の件数の表示が「0 ストリーム」と出るか否かが未定。 | `on_saving(job_id)` の形と古いジョブの無視を BR3.7 に一行で足す。`on_finished` を遅らせる方法は Code Generation の計画で決めると明記する。Hit のときの件数の表示（ストリーム数を出さない、または 0 件のまま出す）を BR5.3 か spec §5 に決める。 | New |
| R-13 | Minor | FD/rules.md BR4.1・BR4.2、FD/functional-spec.md UC2 の 4.1 | BR4.1 の検査は「ファイル全体」（すべてのイベントがどれかの coveredRange に入る、並びが順）とも「範囲内のイベントだけ」（UC2 の 4.1）とも読める。Hit は範囲内だけを読む設計（BR4.2）のため、全体を検査するなら全部読むことになり NFR2 に響く。また、一時的な読み取りの失敗（権限・I/O）でも有効なキャッシュを消してしまう。 | Hit の確かめは、読んだイベント（指定範囲のもの）と、ヘッダの coveredRanges に対して行う、と範囲を明記する。ファイルが「ない」ことと「読めない」ことを区別し、削除は壊れたと判断できたもの（解釈できない・切れている・版・キー不一致・検査失敗）だけに限る旨を足す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この単位に機能設計用の検証ツールの指定はなし） | 実行せず | 既存コードと照合した。`StreamListingStatus::StoppedEarly` は正常な終わりで `JobStatus::Completed` になるため BR3.2 の条件と矛盾しない。`finish_fetch` は `current_job_id` が空か一致のときだけ受け付けるため、Hit が `on_started` を出す設計（BR3.7）は整合する。FR7.1〜FR7.9 は traceability.json で BR に対応している。 |

### Summary

前回の Major 3 件（R-01〜R-03）を含む 9 件はすべて直っており、通知の順・検証してからの追加・破損時の再構築・ロックを握らない書き込みが、開発者が推測せずに実装できる粒度まで定まった。残りは、無効にして保存したときの手順の食い違い（R-10）、異常終了時の後始末（R-11）、実装の細部（R-12、R-13）の Minor だけで、Critical・Major はない。

READY
