# 部品の一覧（Local Sights）

走査したコミット：`2760b13b8bc5b6daba36a6c283b3958c974a5a2d`（2026-10-17）。project.md の決まりにならい、部品は「書いたコード」に限る。AWS SDK、Tauri、React、Cloudscape、ファイルシステム、AWS CLI は外部依存（[dependencies.md](dependencies.md)）とし、それらに触れる境界のコードを部品とする。見出しの名前は [reverse-engineering-timestamp.md](reverse-engineering-timestamp.md) の範囲の記録と同じ綴りで使う。「IB-11」は intent 261017-insights-query（Logs Insights 互換のクエリ）で変更の対象になる見込みの強さ（高・中・低）。

## 中核ライブラリ（`crates/local-sights-core/`）

### FetchCoordinator
- 場所：`src/coordinator.rs`
- 責務：ロググループ全体の取得 1 回の流れ（保持中を捨てる → ストリームの一覧と選択 → 最大 8 本を同時に読む → 各ページを保持先へ）、`FetchJob` の数え上げ、終わりの状態の決定（`decide_job_status`）、停止（Close は破棄して Aborted、Cancel は残して Cancelled）、キャッシュを前段に置く `run_fetch_with_cache`、1 行の要約の出力
- 依存：StreamPlanner、EventFetcher、StreamSchedule、CallPacer、RetryPolicy、FetchStats、LogView（`TimelineStore`／`HeldEvents` として）、LogCache、CachePlan
- 外への口：`FetchSink`、`TimelineStore`、`HeldEvents`、`DeferredFinishSink`
- IB-11：高（最初の `discard` をやめ、足りない範囲・ロググループだけを取り足す流れが要る）

### EventFetcher
- 場所：`src/fetcher.rs`
- 責務：1 ストリームを `GetLogEvents`（`startFromHead`）で 1 ページずつ読み、ストリーム内の順番（0 から）を振る。各試行は CallPacer の許可を待ち、スロットリングを報告する。終わり方を `StreamFetchEnd` で返す
- 依存：CloudWatchLogsGateway、PageCursor、RetryPolicy、CallPacer、FetchStats
- IB-11：中（取り足しで同じストリームを別範囲から読むと `sequence` が 0 から振り直され、既存とぶつかる）

### StreamPlanner
- 場所：`src/streams/mod.rs`、`src/streams/planner.rs`
- 責務：`DescribeLogStreams` を最終イベントの新しい順に読み、範囲に関わるストリームを選ぶ。開始の 1 時間前より古いストリームで打ち切り（`StoppedEarly`）、時刻の無いストリームは選ぶ、途中の失敗は選べた分を残して `Partial`
- 依存：CloudWatchLogsGateway、RetryPolicy、TimeRangeModel
- IB-11：中（複数ロググループ・取り足しの範囲ごとに選び直す）

### StreamSchedule
- 場所：`src/schedule.rs`
- 責務：次に読むストリームの決定（最大 8 本、`MAX_IN_FLIGHT`）、あきらめの決まり（再試行の使い切りが終わった順に 3 本続いたら、まだのストリームを呼ばずに失敗、`GIVE_UP_LIMIT`）
- 依存：EventFetcher の結果型
- IB-11：低

### CallPacer
- 場所：`src/pacing.rs`
- 責務：リージョンごとの `GetLogEvents` の呼び出しの上限（AWS の上限の半分、既定 5 回／秒）、スロットリングで半分に下げ、5 秒ごとに 1 ずつ戻す。停止信号で待ちを即座に終える
- 依存：RetryPolicy（停止信号）
- IB-11：低（複数ロググループを同時に取るなら、同じリージョンで 1 つの pacer を共有するかが論点）

### RetryPolicy
- 場所：`src/retry.rs`
- 責務：再試行の決まり（スロットリングと通信失敗だけ、1・2・4・8・16 秒 ±20%、要求ごとに最大 5 回）、`Retrier`、揺らぎの源、停止信号（`StopReason`：Close が Cancel に勝つ）
- 依存：ApiFailure
- IB-11：低

### FetchStats
- 場所：`src/fetch_stats.rs`
- 責務：1 回の取得の数（呼び出し・スロットリング・同時数の最大・最低の速さ）と、標準エラーに出す 1 行の要約（数と状態の語だけ）
- 依存：FetchStatus
- IB-11：低

### FetchStatus
- 場所：`src/fetch_status.rs`
- 責務：`JobStatus`（Running・Completed・CompletedWithFailures・Failed・Aborted・Cancelled）
- 依存：なし
- IB-11：低

### PageCursor
- 場所：`src/paging.rs`
- 責務：`GetLogEvents` のページの終わりの判定（同じトークンが返ったら終わり、トークンが無ければ終わり）
- 依存：なし
- IB-11：低

### LogEvent
- 場所：`src/event.rs`
- 責務：取得した 1 件のイベント（`timestamp`・`ingestion_time`・`message`・`log_stream_name`・`sequence`）と並びキー `(timestamp, logStreamName, sequence)`
- 依存：なし
- IB-11：高（ロググループ名を持たない。複数ロググループや `@log` を扱うにはキーと型を広げる。`@timestamp`・`@message`・`@logStream`・`@ingestionTime` に当たる値はここにある）

### EventTimeline
- 場所：`src/timeline.rs`
- 責務：今の取得の全イベントを時刻順に保つ（ページを並べ替えて入る位置から混ぜ込む）、行の窓 `rows`、ストリームごとの `sequence → timestamp` の索引と二分探索による位置、追加・破棄ごとの版
- 依存：LogEvent
- IB-11：高（取り足しで既存の保持分に足す。取得済みの範囲は記録していない）

### LogView
- 場所：`src/log_view.rs`
- 責務：EventTimeline と FilterEngine を 1 つのロックに入れ、取得の `TimelineStore`・キャッシュ書き込みの `HeldEvents` になる。絞り込みを考えた行と位置の読み出し、破棄の世代、複数行の位置の一括応答（10,002 キーまで）。速さのテスト（10 万件 10 秒・100 万件 100 秒、`#[ignore]`）もここ
- 依存：EventTimeline、FilterEngine
- IB-11：高（クエリの結果の行の読み出し、`sort`・`limit` の並び、列の選択はここで組み立てる見込み）

### FilterEngine
- 場所：`src/filter.rs`
- 責務：今の絞り込み（FR6）。条件 `FilterCondition`（前後の空白を除いた文字列、小文字にした部分一致、`message` だけを見る）と結果（一致したイベントの並びキーの整列済み配列、ストリーム名は番号に）。4,096 件ずつの走査 `scan_chunk`、追加時の判定 `on_added`（走査位置まで）、券 `ScanTicket` で古い作業を捨てる。純粋で、ロックも待ちも API 呼び出しもしない
- 依存：LogEvent
- IB-11：高（クエリの評価器に差し替える中心。継ぎ目は `FilterCondition::matches`、`scan_chunk`／`on_added`、結果のキー配列）

### ApiFailure
- 場所：`src/failure.rs`
- 責務：失敗の種類（`FailureKind` 8 種）、許可リストの項目だけで作る安全な詳細、アクセスキー ID・秘密の伏せ字
- 依存：なし（`regex`）
- IB-11：低（クエリの構文エラーは AWS の失敗ではないので、別の型にするのが自然）

### CloudWatchLogsGateway
- 場所：`src/gateway/mod.rs`
- 責務：AWS への唯一の境界 trait（読み取り 3 操作だけ）と要求・応答の型、API 名の定数、`ConnectionTarget`
- 依存：ApiFailure、LogGroupBrowser・StreamPlanner の型
- IB-11：低（新しい API は足さない）

### AwsCloudWatchLogsGateway
- 場所：`src/gateway/aws.rs`
- 責務：AWS SDK による実装。接続先の解決（プロファイル、リージョン、無ければ `RegionMissing`）、（プロファイル, リージョン）ごとのクライアントの使い回しと破棄、`GetLogEvents` は SDK の再試行を切って送る、SDK エラーの安全な変換
- 依存：ErrorClassifier、ApiFailure、外部の AWS SDK
- IB-11：低

### ErrorClassifier
- 場所：`src/gateway/classify.rs`
- 責務：エラー信号（サービスのコード、認証情報なし、時間切れ、接続失敗、不明）から `FailureKind` への対応表
- 依存：ApiFailure
- IB-11：低

### LogCache
- 場所：`src/cache/mod.rs`
- 責務：キャッシュファイル（JSON Lines、先頭が見出し、キーの SHA-256 名）の照会・書き込み（一時ファイル → 改名）・削除・全消去。Unix ではフォルダ 0700・ファイル 0600（`cfg(unix)`）、Windows は利用者の `AppData` の下に置き権限は変えない。書き込み中の追跡（`WriteTracker`）
- 依存：CachePlan、外部のファイルシステム
- IB-11：中（取り足しでは、1 範囲が全体を含むときだけ当たる照会を、記録範囲の重なりを使う照会に広げるかが論点。キーは 1 ロググループ）

### CachePlan
- 場所：`src/cache/plan.rs`
- 責務：キャッシュの純粋な判断。キー（プロファイル, リージョン, ロググループ）、記録できる範囲（取得開始の 5 分前まで）、書き込み判定（Completed かつ失敗 0）、範囲の結合（`merge_ranges`）、範囲内の置き換え、読み戻しの番号の振り直し（`resequence`）、壊れの検査、状態行の通知
- 依存：FetchRequest、LogEvent、FetchStatus
- IB-11：中（「足りない範囲」を求める計算は、ここの `CoveredRange` の考え方を流用できる）

### CacheSettings
- 場所：`src/cache/settings.rs`
- 責務：設定ファイル（`{"formatVersion":1,"cacheEnabled":true}`）の読み書き
- 依存：LogCache の書き込みの補助
- IB-11：低

### AppSession
- 場所：`src/session.rs`
- 責務：画面の状態のすべて（段階、タイムゾーンと入力、検証、接続と確認待ち、ロググループの一覧と選択（1 つ）、取得の進み具合と結果、失敗、絞り込みの文字列と要約、キャッシュ設定とダイアログ、閉じる確認、SSO ログイン、経過時間、取り消し）。`SessionView` を作り、行に表示用の時刻を付ける（`display_rows`）
- 依存：ほぼすべての中核の部品
- IB-11：高（クエリの文字列・解釈の結果・エラーの表示、クエリの中の期間とロググループから取得条件を作る流れ。5,428 行あるので別の型に分けて持たせるのがよい）

### ConnectionState
- 場所：`src/connection.rs`
- 責務：プロファイルとリージョンの選択、表示中のログがあるときの確認待ち、変更時の一覧のやり直しの指示
- 依存：ConnectionCatalog
- IB-11：低

### ConnectionCatalog
- 場所：`src/catalog/mod.rs`、`src/catalog/files.rs`、`src/catalog/regions.rs`
- 責務：AWS の設定ファイルからプロファイル一覧（見出しと `region` と SSO 設定の有無だけ、認証情報は捨てる）、内蔵のリージョン一覧
- 依存：外部のファイルシステム
- IB-11：低

### LogGroupBrowser
- 場所：`src/log_groups/mod.rs`、`src/log_groups/listing.rs`
- 責務：`DescribeLogGroups` の全ページ、並べ替え、部分的な結果、古い応答の破棄、名前の絞り込み、空の状態
- 依存：CloudWatchLogsGateway
- IB-11：中（クエリの中のロググループ名を、一覧にある名前と照らすときに使える）

### FetchRequest
- 場所：`src/request.rs`
- 責務：取得条件（プロファイル、リージョン、ロググループ 1 つ）と検証（ロググループの必須・512 文字まで、開始・終了の形と順序）、`ValidatedFetch`
- 依存：TimeRangeModel、DateTimeInput
- IB-11：高（ロググループを複数にする、クエリの中の期間から範囲を作る）

### TimeRangeModel
- 場所：`src/time_range.rs`
- 責務：`yyyy/mm/dd hh:mm:ss` の読み書き（選んだゾーン）、表示用の `yyyy/mm/dd hh:mm:ss.SSS`、`TimeRange`（開始は 0 ms、終了は 999 ms まで含む、API の `endTime` は +1）
- 依存：外部の chrono・chrono-tz
- IB-11：中（クエリの中の期間の書き方を、今の入力の形とどう合わせるか）

### TimeZoneContext
- 場所：`src/time_zone.rs`
- 責務：Local／UTC の選択、OS のタイムゾーンの検出（使えなければ UTC）
- 依存：外部の iana-time-zone
- IB-11：低

### DateTimeInput
- 場所：`src/date_input.rs`
- 責務：開始・終了の入力欄 1 つ（文字列と、読めたときの瞬間）、ゾーン切り替え時の書き直し
- 依存：TimeRangeModel
- IB-11：低

### SsoLoginPlanner
- 場所：`src/sso_login.rs`
- 責務：SSO ログインの純粋な判断（対象のプロファイル、CLI の引数、出力から出してよい URL とコード）
- 依存：ConnectionCatalog
- IB-11：低

### FetchCheck
- 場所：`examples/fetch_check.rs`
- 責務：開発者が手元で実際の AWS に対して取得を試す確認用プログラム（`cargo test` はコンパイルだけで実行しない）。骨組みの確認の 2 番目に使う
- 依存：FetchCoordinator、AwsCloudWatchLogsGateway、FetchRequest
- IB-11：中（薄い一本の確認で、クエリの評価を通すかどうか）

## アプリ（`src-tauri/`）

### TauriApp
- 場所：`src/lib.rs`、`src/main.rs`、`tauri.conf.json`、`capabilities/default.json`、`permissions/autogenerated/`
- 責務：`AppState`（セッション・LogView・ゲートウェイ・停止の取っ手・SSO の実行）、26 個のコマンドと 3 種のイベント、取得タスクと絞り込みの走査スレッド（4,096 件ずつ）の起動、閉じる処理、macOS のアプリメニュー（`cfg(target_os = "macos")`）、設定とキャッシュの置き場所（Tauri のパス解決）
- 依存：AppSession、LogView、FetchCoordinator、AwsCloudWatchLogsGateway、SsoCliRunner、BrowserOpener
- IB-11：高（クエリのコマンド、走査の起動、取り足しの取得の起動。新しいコマンドは `capabilities` と `permissions` にも足す）

### TauriSink
- 場所：`src/lib.rs`（`TauriSink`、`TauriListingSink`、`change_and_view`）
- 責務：取得の進み具合を AppSession に入れ、`session-changed` または軽い `fetch-progress` を送る。絞り込みの要約を LogView から写す
- 依存：AppSession、LogView
- IB-11：中

### SsoCliRunner
- 場所：`src/sso_cli/mod.rs`、`src/sso_cli/unix.rs`、`src/sso_cli/windows.rs`
- 責務：AWS CLI の場所を探し、シェルを通さず起動し、出力から URL とコードだけを通し、取り消し・時間切れ・終了で止める。Unix はプロセスグループ（SIGTERM → SIGKILL）、Windows はジョブオブジェクト
- 依存：SsoLoginPlanner、外部の libc／windows-sys
- IB-11：低

### BrowserOpener
- 場所：`src/browser.rs`
- 責務：SSO の URL を既定のブラウザで開く。Unix は `/usr/bin/open`、Windows は `ShellExecuteW`
- 依存：外部の windows-sys（Windows）
- IB-11：低

## 画面（`src/`）

### ApiBridge
- 場所：`src/api.ts`
- 責務：Tauri のコマンドとイベントの型と関数、受け取った更新の合成（版の古いものを捨てる）
- 依存：外部の `@tauri-apps/api`、PerfRowsHarness
- IB-11：高（クエリのコマンド、結果の型（列の可変化））

### AppShell
- 場所：`src/App.tsx`、`src/main.tsx`
- 責務：2 枚の区画の配置、イベントの購読、操作の送信、一覧への件数と版の受け渡し（絞り込み中は `filterSummary.matchedCount`）
- 依存：ほぼすべての画面の部品、ApiBridge
- IB-11：中

### ConditionArea
- 場所：`src/components/ConditionArea.tsx`、`src/dateTime.ts`
- 責務：開始・終了（日付欄と時刻欄、カレンダー）、各欄の理由の表示、[Fetch]／[Cancel]、絞り込みの欄の配置
- 依存：LogFilterInput、MessageCatalog
- IB-11：高（クエリの入力欄をどこに置くか、クエリの期間と条件欄の期間の関係）

### LogFilterInput
- 場所：`src/components/LogFilterInput.tsx`、`src/hooks/useDebouncedValue.ts`
- 責務：今の絞り込みの欄（Cloudscape の TextFilter、"Filter messages"）。入力が 300 ms 止まるか Enter で文字列を送る。送れなかったときやダイアログが閉じたときに送り直す
- 依存：ApiBridge（`setLogFilter` は AppShell 経由）
- IB-11：高（クエリの入力欄に置き換えるか並べるか。複数行の入力、実行の合図、エラーの表示）

### StatusLine
- 場所：`src/components/StatusLine.tsx`
- 責務：件数（絞り込み中は「一致件数／保持件数」と Filtering 表示）、取得の進み具合、キャッシュの通知、経過時間
- 依存：ElapsedTime、MessageCatalog
- IB-11：中

### ElapsedTime
- 場所：`src/components/ElapsedTime.tsx`、`src/elapsed.ts`、`src/hooks/useElapsed.ts`
- 責務：経過時間の表示（取得中は画面の時計で 1 秒ごと）
- 依存：なし
- IB-11：低

### CancelNotice
- 場所：`src/components/CancelNotice.tsx`
- 責務：取り消した取得の、最後まで読めなかったストリームの数の知らせ
- 依存：MessageCatalog
- IB-11：低

### FetchErrorBanner
- 場所：`src/components/FetchErrorBanner.tsx`、`src/components/ErrorMessage.tsx`、`src/errorText.ts`
- 責務：取得全体が失敗したときのエラー表示（何が起きたか・次にすること・安全な詳細）
- 依存：MessageCatalog、SsoLoginModal（[Sign in with SSO] のボタン）
- IB-11：中（対応していない命令や関数のエラーの表示に流用できる）

### FailureList
- 場所：`src/components/FailureList.tsx`
- 責務：失敗したストリームと一覧の失敗の表
- 依存：MessageCatalog
- IB-11：低

### LogTable
- 場所：`src/components/LogTable.tsx`、`src/rowLayout.ts`、`src/virtualScroll.ts`、`src/format.ts`、`src/hooks/useRowWindow.ts`、`src/hooks/useRowPositions.ts`、`src/hooks/useRowLayout.ts`、`src/hooks/expansionState.ts`
- 責務：一覧（時刻／ストリーム／メッセージの固定 3 列、メッセージは 1 行に）。見えている範囲だけ `get_rows`、開いた行の高さを考えた仮想スクロール、選択とキー操作、開いた行の位置の追跡（`row_positions`）
- 依存：ApiBridge、ExpandedMessage
- IB-11：高（`fields` で列が変わる。行の識別 `RowKey(logStreamName, sequence)` は複数ロググループでぶつかりうる）

### LogTableCloudscape
- 場所：`src/components/LogTableCloudscape.tsx`
- 責務：Cloudscape の Table による一覧の試作（`VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape` のときだけ）
- 依存：ApiBridge
- IB-11：低

### ExpandedMessage
- 場所：`src/components/ExpandedMessage.tsx`
- 責務：開いた行の下にメッセージ全体を文字として出す（HTML として読まない、JSON は整形しない、20 行まで）
- 依存：なし
- IB-11：低

### TopBar
- 場所：`src/components/TopBar.tsx`
- 責務：上の黒い帯。プロファイル、リージョン、タイムゾーン、設定のボタン
- 依存：MessageCatalog
- IB-11：低

### LogGroupPane
- 場所：`src/components/LogGroupPane.tsx`
- 責務：左の区画。ロググループの表（名前の検索欄、[Reload]、1 つを選ぶ）、一覧のエラー
- 依存：MessageCatalog、SsoLoginModal（ボタン）
- IB-11：中（複数のロググループを選べるようにするか、クエリの中の指定との関係）

### PaneResizer
- 場所：`src/components/PaneResizer.tsx`、`src/hooks/usePaneWidth.ts`
- 責務：左の区画の幅の変更と記憶（`localStorage`）
- 依存：なし
- IB-11：低

### SettingsDialog
- 場所：`src/components/SettingsDialog.tsx`
- 責務：キャッシュの有効・無効、置き場所、全消去
- 依存：MessageCatalog
- IB-11：低

### CloseConfirmDialog
- 場所：`src/components/CloseConfirmDialog.tsx`
- 責務：取得中に閉じるときの確認
- 依存：MessageCatalog
- IB-11：低

### ConfirmDialog
- 場所：`src/components/ConfirmDialog.tsx`
- 責務：接続の変更で表示中のログを捨てる前の確認
- 依存：MessageCatalog
- IB-11：低

### SsoLoginModal
- 場所：`src/components/SsoLoginModal.tsx`、`src/components/SsoLoginButton.tsx`
- 責務：SSO ログインの進み具合と結果のダイアログ、エラーの中の [Sign in with SSO]
- 依存：ApiBridge、MessageCatalog
- IB-11：低

### ScreenHooks
- 場所：`src/hooks/useColorScheme.ts`、`src/hooks/useEscapeKey.ts`、`src/hooks/useInitialFocus.ts`、`src/designTokens.ts`、`src/styles.css`
- 責務：明暗の切り替え、Escape、最初の焦点、Cloudscape のデザイントークン
- 依存：外部の Cloudscape
- IB-11：低

### MessageCatalog
- 場所：`src/i18n/messages.ts`
- 責務：英語と日本語の文言。中核が返すキーもここで引く
- 依存：なし
- IB-11：中（クエリのエラーの文言を足す）

### PerfRowsHarness
- 場所：`src/dev/perfRows.ts`
- 責務：開発ビルドで環境変数を付けたときだけ、生成した行（平文と JSON のメッセージ）で一覧と絞り込みの速さを測る
- 依存：ApiBridge の型
- IB-11：低（JSON のメッセージを含む生成行は、クエリの画面側の確認にも使える）

## テストと道具

### FakeGateway
- 場所：`crates/local-sights-core/tests/support/fake_gateway.rs`
- 責務：テスト用の偽ゲートウェイ（ページ・失敗・待ちを台本で返す）。AWS にはつながない
- 依存：CloudWatchLogsGateway
- IB-11：中（取り足しの取得の試験で使う）

### RecordingSink
- 場所：`crates/local-sights-core/tests/support/recording_sink.rs`
- 責務：`FetchSink` の呼ばれた順番と中身を記録する
- 依存：FetchCoordinator の型
- IB-11：低

### ReleaseScripts
- 場所：`scripts/release/`、`scripts/render-icon.mjs`
- 責務：5 つのファイルの版のそろいとタグの確認、`.app` のアドホック署名の確認、アイコンの書き出し
- 依存：なし
- IB-11：低（`v0.2.0` のタグは IB-11 が終わるまで付けない、team.md Deployment）

### CiWorkflows
- 場所：`.github/workflows/ci.yml`、`.github/workflows/release.yml`、`.github/workflows/release-dry-run.yml`、`.github/dependabot.yml`
- 責務：PR と `main` の検査（中核の fmt・clippy・行カバレッジ 80%、画面の tsc・eslint・prettier・vitest の行カバレッジ 80%・build・npm audit、cargo-deny、Tauri のビルドとテストを Linux・macOS・Windows で）、`v*` タグで両 OS の配布物を作って公開、手動の試しビルド
- 依存：なし
- IB-11：低（ジョブ名を変えるなら、ルールセットの必須チェックも同じ PR で直す）
