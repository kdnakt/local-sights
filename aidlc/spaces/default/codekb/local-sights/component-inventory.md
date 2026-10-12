# 部品の一覧（Local Sights）

走査したコミット：`9d08cbf5c6d97820f0ec7f276c3ce6020c903eb5`（2026-10-12）。project.md の決まりにならい、部品は「書いたコード」に限る。AWS SDK、Tauri、React、Cloudscape、ファイルシステムは外部依存（[dependencies.md](dependencies.md)）とし、それらに触れる境界のコードを部品とする。見出しの名前は [reverse-engineering-timestamp.md](reverse-engineering-timestamp.md) の範囲の記録と同じ綴りで使う。「今回」は intent 261012-parallel-fetch で変更の対象になる見込みの強さ（高・中・低）。

## 中核ライブラリ（`crates/local-sights-core/`）

### FetchCoordinator
- 場所：`src/coordinator.rs`
- 責務：ロググループ全体の取得 1 回の流れ（一覧→ストリームを 1 つずつ取得→各ページを保持先へ）、`FetchJob` の数え上げ、終わりの状態の決定（`decide_job_status`）、中断時の破棄、キャッシュを前段に置く `run_fetch_with_cache`（照会、保留した終わり、写しと書き込み）
- 依存：StreamPlanner、EventFetcher、RetryPolicy、LogView（`TimelineStore`／`HeldEvents` として）、LogCache、CachePlan
- 外への口：`FetchSink`、`TimelineStore`、`HeldEvents`、`DeferredFinishSink`
- 今回：高（直列の `for`、連続使い切りの扱い、中断＝破棄、受け口の `&mut self`）

### EventFetcher
- 場所：`src/fetcher.rs`
- 責務：1 ストリームを `GetLogEvents` でページの終わりまで読み、ストリーム内の順番（0 から）を振り、ページごとに渡す。失敗・使い切り・中断を `StreamFetchEnd` で返す。`StreamFetchOutcome::skipped` は呼ばずに失敗とした結果
- 依存：CloudWatchLogsGateway、PageCursor、RetryPolicy（`Retrier`）
- 今回：高（ページの受け取りが `FnMut`、中断時の結果の扱い）

### StreamPlanner
- 場所：`src/streams/mod.rs`、`src/streams/planner.rs`
- 責務：`DescribeLogStreams` を最終イベントの新しい順にページで読み、範囲に関わるストリームを選ぶ。開始の 1 時間前より古いストリームが出たら打ち切り（`StoppedEarly`）、時刻の無いストリームは必ず選ぶ、途中の失敗は選べた分を残して `Partial`
- 依存：CloudWatchLogsGateway、RetryPolicy、PageCursor（`paging::decide`）、TimeRangeModel
- 今回：中（一覧を最後まで読んでから取得する順序。並行化で重ねるかどうか）

### RetryPolicy
- 場所：`src/retry.rs`
- 責務：再試行の決まり（スロットリングと通信失敗だけ、1・2・4・8・16 秒 ±20%、要求ごとに最大 5 回）、要求ごと・ストリームごとの回数（`RetryCounter`）、連続使い切りの数え（`ExhaustionStreak`、既定 3）、揺らぎの源（`JitterSource`）、中断信号（`AbortHandle`／`AbortSignal`、`tokio::sync::watch`）、それらを回す `Retrier`
- 依存：ApiFailure
- 今回：高（全体の呼び出し回数の上限とスロットリング時の引き下げ、並行時の使い切りの決まり、取り消し）

### PageCursor
- 場所：`src/paging.rs`
- 責務：ページの終わりの判定（送ったトークンと同じものが返ったら終わり、トークンが無ければ終わり）とページ数
- 依存：なし
- 今回：低

### LogEvent
- 場所：`src/event.rs`
- 責務：取得した 1 件のイベントと並びのキー `(timestamp, logStreamName, sequence)`
- 依存：なし
- 今回：低

### EventTimeline
- 場所：`src/timeline.rs`
- 責務：現在の取得の全イベントを時刻順に保つ。ページを並べ替えて入る位置から混ぜ込む、行の窓（`rows`）、`(ストリーム, 順番)` からの位置、追加・破棄ごとの版
- 依存：LogEvent
- 今回：中（並行で古いページが後から届くと混ぜ込みの費用が増える）

### LogView
- 場所：`src/log_view.rs`
- 責務：EventTimeline と FilterEngine を 1 つのロックに入れ、取得の `TimelineStore`・`HeldEvents` になる。破棄の世代、複数行の位置の一括応答（多すぎる要求は断る）
- 依存：EventTimeline、FilterEngine
- 今回：中（ページごとのロック時間）

### FilterEngine
- 場所：`src/filter.rs`
- 責務：メッセージに文字列を含むか（大文字小文字を区別しない）で絞る。保持済みを少しずつ走査し、走査中に届いたページは走査位置までだけ判定する。券で古い作業を捨てる
- 依存：LogEvent
- 今回：低（取り消しで残したログも絞り込めること）

### ApiFailure
- 場所：`src/failure.rs`
- 責務：失敗の種類（`FailureKind` 8 種）、許可リストの項目だけで作る安全な詳細、アクセスキー ID・シークレット・トークンの伏せ字
- 依存：なし（`regex`）
- 今回：低（新しい警告文でも同じ決まりを守る）

### CloudWatchLogsGateway
- 場所：`src/gateway/mod.rs`
- 責務：AWS への唯一の境界 trait（読み取り 3 操作だけ）と要求・応答の型、API 名の定数、`ConnectionTarget`
- 依存：ApiFailure、LogEvent 系の型
- 今回：低（形は並行呼び出しに対応済み）

### AwsCloudWatchLogsGateway
- 場所：`src/gateway/aws.rs`
- 責務：AWS SDK による実装。接続先の解決（プロファイル、リージョン、無ければ `RegionMissing`）、`(プロファイル, リージョン)` ごとのクライアントの再利用と破棄、SDK エラーの安全な変換
- 依存：ErrorClassifier、ApiFailure、外部の AWS SDK
- 今回：中（SDK 側の再試行設定が明示されていない。並行の初回呼び出しでクライアントを二重に作りうる）

### ErrorClassifier
- 場所：`src/gateway/classify.rs`
- 責務：エラー信号（サービスのコード、認証情報なし、時間切れ、接続失敗、不明）から `FailureKind` への対応表
- 依存：ApiFailure
- 今回：低

### LogCache
- 場所：`src/cache/mod.rs`
- 責務：キャッシュファイル（JSON Lines、先頭が見出し、キーの SHA-256 名）の照会・書き込み（一時ファイル→改名）・削除・全消去、フォルダ 0700・ファイル 0600、書き込み中の追跡（`WriteTracker`）
- 依存：CachePlan、外部のファイルシステム
- 今回：低（見かけの振る舞いを変えない）

### CachePlan
- 場所：`src/cache/plan.rs`
- 責務：キャッシュの純粋な判断。キー、記録できる範囲（開始の 5 分前まで）、書き込み判定（`Completed` かつ失敗 0）、範囲の結合、範囲内の置き換え、番号の振り直し、壊れの検査、状態行の通知
- 依存：FetchRequest、LogEvent、`JobStatus`
- 今回：中（取り消しで終えた取得を書かないこと、通知の扱い）

### CacheSettings
- 場所：`src/cache/settings.rs`
- 責務：設定ファイル（キャッシュのオン・オフ）の読み書き
- 依存：LogCache の権限の決まり
- 今回：低

### AppSession
- 場所：`src/session.rs`
- 責務：画面が表示する状態のすべて。段階（`Idle`／`Fetching`／`Done`／`Failed`）、入力と検証、接続とロググループ一覧、取得の開始・進み具合・終わり、失敗一覧、絞り込みの要約、キャッシュ設定とダイアログ、終了確認、SSO ログインの状態。`SessionView`・`FetchProgressUpdate`・`FilterProgressUpdate`・`DisplayRowWindow` を作る
- 依存：ConnectionState、ConnectionCatalog、LogGroupBrowser、FetchRequest、DateTimeInput、TimeZoneContext、TimeRangeModel、CachePlan、LogCache、CacheSettings、SsoLoginPlanner、FetchCoordinator の型
- 今回：高（取り消しの状態、`Aborted` の扱い、経過時間、警告）

### ConnectionState
- 場所：`src/connection.rs`
- 責務：プロファイルとリージョンの選択、表示中のログがあるときの確認待ち、一覧の開始・消去の指示
- 依存：ConnectionCatalog の型
- 今回：低

### ConnectionCatalog
- 場所：`src/catalog/mod.rs`、`src/catalog/files.rs`、`src/catalog/regions.rs`
- 責務：AWS の共有設定ファイルから節の名前と `region`、SSO 設定の有無、`source_profile` だけを読む（認証情報の値は読んだそばから捨てる）。リージョンの固定一覧
- 依存：外部のファイルシステム、`home`
- 今回：低

### LogGroupBrowser
- 場所：`src/log_groups/mod.rs`、`src/log_groups/listing.rs`
- 責務：ロググループ一覧の状態（ページ、並べ替え、途中の失敗、古い応答の破棄、絞り込み、空の状態）と流れ（`run_listing`、`ListingSink`）
- 依存：CloudWatchLogsGateway
- 今回：低

### FetchRequest
- 場所：`src/request.rs`
- 責務：取得条件の検証（ロググループ名、日時の形式、存在しない現地時刻、順序）と `ValidatedFetch`
- 依存：TimeRangeModel、DateTimeInput、ConnectionCatalog の型
- 今回：低

### TimeRangeModel
- 場所：`src/time_range.rs`
- 責務：日時の読み書き（`yyyy/mm/dd hh:mm:ss`）、取得範囲 `TimeRange`（終了はその秒の 999 ミリ秒まで、API には +1）、表示用の書式
- 依存：`chrono`、`chrono-tz`
- 今回：低

### TimeZoneContext
- 場所：`src/time_zone.rs`
- 責務：Local／UTC の選択、OS のゾーン名の解決と UTC への退避
- 依存：`iana-time-zone`、`chrono-tz`
- 今回：低

### DateTimeInput
- 場所：`src/date_input.rs`
- 責務：開始・終了の入力欄 1 つ分の文字列と瞬間、ゾーン切り替え時の書き換え
- 依存：TimeRangeModel
- 今回：低

### SsoLoginPlanner
- 場所：`src/sso_login.rs`
- 責務：SSO ログインの対象プロファイル（`source_profile` を最大 5 段たどる）、CLI の引数、出力行から出してよい URL と確認コードの取り出し
- 依存：ConnectionCatalog
- 今回：低

### FetchCheck
- 場所：`examples/fetch_check.rs`
- 責務：開発者の手元で実 AWS に対して取得を試す確認用プログラム（`run_fetch` と `FetchSink` を使う）
- 依存：FetchCoordinator、AwsCloudWatchLogsGateway、FetchRequest、RetryPolicy、EventTimeline
- 今回：中（`run_fetch`／`FetchSink` の形を変えると直す必要がある）

## アプリ（`src-tauri/`）

### TauriApp
- 場所：`src/lib.rs`、`src/main.rs`、`build.rs`、`capabilities/default.json`、`tauri.conf.json`
- 責務：25 個のコマンド、`AppState`（セッション、LogView、ゲートウェイ、走っている取得の中断ハンドル、終了許可、SSO）、取得・一覧・絞り込みの走査・設定作業の非同期起動、異常終了した取得の回復、メニュー（Cmd+Q）と閉じる確認、イベントの送出
- 依存：AppSession、FetchCoordinator、LogView、AwsCloudWatchLogsGateway、LogGroupBrowser、TauriSink、SsoCliRunner、外部の Tauri
- 今回：高（取り消しのコマンドと権限、取得タスクの組み立て、並行化に合わせた受け口）

### TauriSink
- 場所：`src/lib.rs`（`TauriSink`、`TauriListingSink`）
- 責務：`FetchSink` の実装。呼ばれるたびにセッションのロックを取って状態を写し、絞り込みの要約を写し、`session-changed` か `fetch-progress` を送る。一覧の受け口も同様
- 依存：AppSession、LogView、外部の Tauri
- 今回：高（`&mut self` 前提、並行時の呼ばれ方と送出の量）

### SsoCliRunner
- 場所：`src/sso_cli.rs`
- 責務：AWS CLI を決まった場所（`/opt/homebrew/bin/aws`、`/usr/local/bin/aws`）から探し、シェルなし・新しいプロセスグループで起動し、出力から URL とコードだけを渡し、取り消し・5 分の時間切れ・終了で SIGTERM→SIGKILL で止める
- 依存：SsoLoginPlanner、外部の `libc`
- 今回：低

## 画面（`src/`）

### ApiBridge
- 場所：`src/api.ts`
- 責務：Tauri コマンドの型付きの包み（25 個）、3 つのイベントの購読、受け取ったメッセージの合成（`withProgress`、`withFilter`、`mergeSessionView`）、`SessionView` などの型
- 依存：外部の `@tauri-apps/api`、`src/dev/perfRows.ts`
- 今回：高（取り消しのコマンド、新しい状態・経過時間・警告の型）

### AppShell
- 場所：`src/App.tsx`、`src/main.tsx`
- 責務：起動、イベントの購読、各部品の配置、操作のコマンド送信、エラー表示、ダイアログの出し分け
- 依存：ApiBridge、各画面部品、MessageCatalog
- 今回：中（［取り消し］の送信）

### ConditionArea
- 場所：`src/components/ConditionArea.tsx`
- 責務：開始・終了の日付と時刻、［取得］ボタン（取得中は無効）、理由の表示、絞り込み欄、Enter での開始
- 依存：ApiBridge の型、`dateTime.ts`、LogFilterInput
- 今回：高（取得中に［取り消し］へ変わる）

### StatusLine
- 場所：`src/components/StatusLine.tsx`
- 責務：状態行。一覧中、取得中（n/m ストリーム、k 件）、件数、失敗ストリーム数のリンク、一覧が途中まで、全体の失敗の一文、キャッシュの通知、絞り込みの件数
- 依存：ApiBridge の型、`errorText.ts`、MessageCatalog
- 今回：高（経過時間、取り消し後の表示）

### FetchErrorBanner
- 場所：`src/components/FetchErrorBanner.tsx`、`src/components/ErrorMessage.tsx`、`src/errorText.ts`
- 責務：取得全体が失敗したときの一覧上のエラー（何が起きたか・次に何をするか・詳細）、認証エラーでの［SSO でサインイン］
- 依存：ApiBridge の型、MessageCatalog、SsoLoginModal のボタン
- 今回：中（取り消し時の警告の出し場所の候補）

### FailureList
- 場所：`src/components/FailureList.tsx`
- 責務：一覧の失敗と失敗ストリームの表（モーダル）
- 依存：ErrorMessage、MessageCatalog
- 今回：中（終わらなかったストリームの扱い次第）

### LogTable
- 場所：`src/components/LogTable.tsx`、`ExpandedMessage.tsx`、`LogTableCloudscape.tsx`、`src/rowLayout.ts`、`src/virtualScroll.ts`
- 責務：仮想スクロールの一覧（見えている行だけ `get_rows`）、行の展開と選択、位置の維持
- 依存：ApiBridge、ScreenHooks
- 今回：低

### TopBar
- 場所：`src/components/TopBar.tsx`
- 責務：上部の黒いナビゲーション（プロファイル、リージョン、設定ボタン）
- 依存：ApiBridge の型、MessageCatalog
- 今回：低

### LogGroupPane
- 場所：`src/components/LogGroupPane.tsx`、`src/components/PaneResizer.tsx`
- 責務：左のロググループの表と検索、再読み込み、幅の変更
- 依存：ApiBridge の型、ScreenHooks
- 今回：低

### SettingsDialog
- 場所：`src/components/SettingsDialog.tsx`
- 責務：キャッシュのオン・オフ、場所、機密の注意、［キャッシュを消去］
- 依存：ApiBridge の型、MessageCatalog
- 今回：低

### CloseConfirmDialog
- 場所：`src/components/CloseConfirmDialog.tsx`
- 責務：取得中・書き込み中に閉じるときの確認（「取得したログは失われる」）
- 依存：MessageCatalog
- 今回：中（取り消しでログを残す振る舞いとの整合）

### ConfirmDialog
- 場所：`src/components/ConfirmDialog.tsx`
- 責務：表示中のログがあるときの接続変更の確認
- 依存：MessageCatalog
- 今回：低

### SsoLoginModal
- 場所：`src/components/SsoLoginModal.tsx`、`src/components/SsoLoginButton.tsx`
- 責務：SSO ログインの待ち・結果のダイアログと［SSO でサインイン］ボタン
- 依存：ApiBridge の型、MessageCatalog
- 今回：低

### ScreenHooks
- 場所：`src/hooks/`、`src/dateTime.ts`、`src/designTokens.ts`、`src/format.ts`、`src/dev/perfRows.ts`
- 責務：行の窓の読み込み、位置の問い合わせ、展開状態、配色、Escape、幅の記憶、日時欄の分割と補完、計測用の生成行
- 依存：ApiBridge
- 今回：低（経過時間を画面で数える場合は時計のフックを足す候補）

### MessageCatalog
- 場所：`src/i18n/messages.ts`
- 責務：英語と日本語の文言。`validation.`、`failure.kind.`、`session.`、`rows.` のキーは中核と共有
- 依存：なし
- 今回：高（経過時間、取り消し、警告の文言）

## テスト支援・道具

### FakeGateway
- 場所：`crates/local-sights-core/tests/support/fake_gateway.rs`
- 責務：AWS に接続しない偽のゲートウェイ。台本どおりに答え、要求と呼んだ順を記録する。ストリームごとの台本を持つ
- 今回：高（並行・呼び出し回数・取り消しを試す仕組みが要る）

### RecordingSink
- 場所：`crates/local-sights-core/tests/support/recording_sink.rs`
- 責務：`FetchSink` に届いたものを順に記録する
- 今回：中

### ReleaseScripts
- 場所：`scripts/release/`、`scripts/render-icon.mjs`
- 責務：4 つのファイルの版とタグの一致の検査、アイコン PNG の生成
- 今回：低

### CiWorkflows
- 場所：`.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`、`.github/dependabot.yml`
- 責務：PR と `main` の検査（整形、lint、テスト、型、依存の監査、両 OS でのビルド）、タグからの `.dmg` 作成と公開、依存の更新提案
- 今回：低
