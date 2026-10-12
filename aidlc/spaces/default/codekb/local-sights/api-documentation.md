# API の説明（Local Sights）

走査したコミット：`9d08cbf5c6d97820f0ec7f276c3ce6020c903eb5`（2026-10-12）。HTTP のエンドポイントは無い。口は (1) AWS への外向きの呼び出し、(2) 中核の境界 trait、(3) 画面とアプリの間の Tauri コマンドとイベント、(4) 中核ライブラリの公開関数、の 4 種類である。

## 1. 外向き：AWS CloudWatch Logs（読み取りのみ）

実装は `crates/local-sights-core/src/gateway/aws.rs` の `AwsCloudWatchLogsGateway`。認証は AWS SDK の既定チェーンに任せ（SSO、アクセスキー、AssumeRole、環境変数）、アプリは認証情報を読まない・持たない・書かない。

| 操作 | 渡すもの | 受け取るもの | 呼ぶ場所 |
|---|---|---|---|
| `GetLogEvents` | ロググループ名、ストリーム名、`startTime`（開始の瞬間）、`endTime`（終了の瞬間 + 1、排他）、`startFromHead = true`、`nextToken` | イベント（時刻、取り込み時刻、メッセージ）、`nextForwardToken` | `fetcher::fetch_stream` |
| `DescribeLogStreams` | ロググループ名、`orderBy = LastEventTime`、`descending = true`、`nextToken`（名前の前方一致は渡さない） | ストリーム（名前、最初と最後のイベント時刻）、`nextToken` | `streams::planner::plan_streams` |
| `DescribeLogGroups` | `nextToken` のみ（絞り込みは手元で） | ロググループ（名前、ARN、作成時刻）、`nextToken` | `log_groups::listing::run_listing` |

- 接続先：`(プロファイル, リージョン)` ごとに SDK のクライアントを作って使い回す（`Mutex<HashMap>`）。明示のリージョンが無く、プロファイルにも既定のリージョンが無いときは、呼ばずに `RegionMissing` を返す。`clear_clients()` は SSO ログインの成功後にクライアントを捨てる。
- ページの終わり：`GetLogEvents` は送ったトークンと同じトークンが返ったら終わり（空のページや短いページは終わりではない）。トークンが無い応答も終わりとする（`paging.rs`）。一覧の 2 操作はトークンが無いか同じなら終わり。
- 失敗の変換：SDK のエラーを `ErrorSignal`（サービスのコード、認証情報が無い、時間切れ、接続失敗、不明）に落とし、`FailureKind` に分類する（`gateway/classify.rs`）。

| サービスのコードなど | `FailureKind` | 再試行 |
|---|---|---|
| `ExpiredToken(Exception)`、`UnrecognizedClientException`、`InvalidClientTokenId`、`InvalidSignatureException`、`MissingAuthenticationToken`、認証情報の取得失敗 | `AuthRequired` | しない |
| `AccessDeniedException`、`AccessDenied` | `AccessDenied` | しない |
| `ThrottlingException`、`Throttling` | `Throttled` | する |
| 時間切れ、接続の失敗 | `Network` | する |
| `ResourceNotFoundException` | `NotFound` | しない |
| `InvalidParameterException` | `InvalidInput` | しない |
| リージョンが決まらない | `RegionMissing` | しない |
| それ以外（`ServiceUnavailableException` を含む） | `Other` | しない |

- 再試行の設定：アプリ側の `RetryPolicy`（1・2・4・8・16 秒 ±20%、要求ごとに最大 5 回）。AWS SDK 側の再試行設定は明示していない（`aws_config::defaults(BehaviorVersion::latest())` のまま）。
- 呼び出し回数の制限：アプリ全体での毎秒の回数を抑える仕組みは無い（今は直列なので 1 本ずつ）。
- 必要な IAM 権限：`logs:DescribeLogGroups`、`logs:DescribeLogStreams`、`logs:GetLogEvents`（`README.md`）。

## 2. 中核の境界 trait

### `CloudWatchLogsGateway`（`gateway/mod.rs`）

```rust
pub trait CloudWatchLogsGateway: Send + Sync {
    fn get_log_events(&self, request: GetLogEventsRequest)
        -> impl Future<Output = Result<GetLogEventsPage, ApiFailure>> + Send;
    fn describe_log_groups(&self, request: DescribeLogGroupsRequest)
        -> impl Future<Output = Result<DescribeLogGroupsPage, ApiFailure>> + Send;
    fn describe_log_streams(&self, request: DescribeLogStreamsRequest)
        -> impl Future<Output = Result<DescribeLogStreamsPage, ApiFailure>> + Send;
}
```

`&self` で呼べ、`Send + Sync` なので、複数のタスクから同時に呼べる形になっている。テストでは `tests/support/fake_gateway.rs` の `FakeGateway`（台本どおりに即座に答え、受けた要求を記録する）や `coordinator.rs` のテスト内の偽物に差し替える。

### `FetchSink`（`coordinator.rs`）

取得の進み具合の受け口。届く順は「started → listing progress（一覧のページごと）→ planned → batch と stream finished → finished」。

| メソッド | 呼ばれるとき |
|---|---|
| `on_started(&mut self, job, timeline_version)` | 取得の開始（`LogView` を空にした直後） |
| `on_listing_progress(&mut self, job_id, ListingProgress)` | 一覧のページを 1 つ読んだ |
| `on_planned(&mut self, job)` | 一覧が終わり対象数が決まった |
| `on_batch(&mut self, job_id, BatchProgress)` | ページを 1 つ `LogView` に入れた（`added`、`total`、`timeline_version`） |
| `on_stream_finished(&mut self, job, outcome)` | ストリームが 1 つ終わった（成功・失敗） |
| `on_saving(&mut self, job_id)`（既定は何もしない） | キャッシュに書く直前 |
| `on_finished(&mut self, job, timeline_version)` | 最終状態が決まった |

すべて `&mut self` で、直列に呼ばれる前提である。

### `TimelineStore` / `HeldEvents`（`coordinator.rs`）

- `TimelineStore: Send + Sync`：`discard(&self) -> u64`、`add(&self, Vec<LogEvent>) -> u64`。呼び出しは短く、ページの間でロックを持たない。実装は `Mutex<EventTimeline>` と `Mutex<LogView>`。
- `HeldEvents`：`copy_events_in_range(&self, range, cursor, max)`。キャッシュへ書く前に少しずつ写すため。

### `ListingSink`（`log_groups/listing.rs`）

`is_current(&self, listing_id)`、`on_page(&mut self, ...)`、`on_failure(&mut self, listing_id, failure) -> bool`。古い一覧は次のページを呼ばずに止まる。

### その他の差し替え口

- `JitterSource: Send + Sync`（`retry.rs`）：`RandomJitter`（本番、`fastrand`）と `FixedJitter`（テスト）。
- `TimeZoneContext`（`time_zone.rs`）：ローカル時刻のゾーンを固定してテストする。

## 3. 画面とアプリの間：Tauri コマンドとイベント

コマンドは `src-tauri/src/lib.rs`、権限の宣言は `src-tauri/build.rs` と `src-tauri/capabilities/default.json`、画面側の包みは `src/api.ts`（関数名は camelCase）。状態を変えるコマンドは何も返さず、新しい状態は `session-changed` で届く。断られたときは `CommandError { key }`（文言キー）で失敗する。接続に依存する操作は `generation` を添え、古ければ答えずに捨てる。

| コマンド | 引数 | 戻り値 | 内容 |
|---|---|---|---|
| `get_session` | — | `SessionView` | 今の状態 |
| `update_input` | `field: "startText" \| "endText"`、`value` | `SessionView` | 日時の入力を変える（取得中は断る） |
| `start_fetch` | `generation` | なし | 取得を始める（結果はイベントで） |
| `get_rows` | `offset`、`limit`（最大 1,000） | `DisplayRowWindow` | 見えている範囲の行（時刻は選んだゾーンで書式済み、絞り込み中はその結果） |
| `row_positions` | `keys: RowKey[]` | `RowPositions` | 複数の行の今の位置と各版（多すぎる要求は断る） |
| `set_failure_list_open` | `open` | なし | 失敗一覧を開く・閉じる |
| `select_time_zone` | `timeZone: "Local" \| "Utc"` | なし | 表示と入力のゾーン（取得中も可） |
| `set_log_filter` | `text` | なし | 絞り込みの文字列（取得中も可）、走査を別スレッドで開始 |
| `select_profile` | `profile: ProfileSelector` | なし | プロファイルを選ぶ（表示中のログがあれば確認待ち） |
| `select_region` | `region` | なし | リージョンを選ぶ |
| `confirm_connection_change` / `cancel_connection_change` | — | なし | 接続変更の確認に答える |
| `reload_log_groups` | `generation` | なし | ロググループを読み直す |
| `update_log_group_filter` | `text` | なし | ロググループ名の絞り込み |
| `select_log_group` | `name`、`generation` | なし | ロググループを選ぶ |
| `open_settings` / `cancel_settings` | — | なし | 設定ダイアログ |
| `save_settings` | `enabled` | なし | キャッシュのオン・オフを保存（オフでファイルを消す） |
| `clear_cache` | — | なし | キャッシュを消す |
| `cancel_close` / `confirm_close` | — | なし | 閉じる確認への答え（［閉じる］は取得を中断して終了） |
| `start_sso_login` / `cancel_sso_login` / `dismiss_sso_login` | — | なし | AWS CLI による SSO ログイン |
| `open_sso_url` | `url` | なし | その回の URL だけを `/usr/bin/open` で開く |

利用者が取得だけを止めるコマンド（取り消し）は無い。取得の中断は `confirm_close` とウィンドウの `Destroyed` からだけ起きる。

| イベント | 中身 | 送るとき |
|---|---|---|
| `session-changed` | `SessionView` 全体 | 状態が変わるたび（取得では開始・一覧の終わり・ストリームの終わり・終わり） |
| `fetch-progress` | `FetchProgressUpdate`：`jobId`、`progress`（`seenStreamCount`、`selectedStreamCount`、`plannedStreamCount`、`finishedStreamCount`、`eventCount`）、`eventCount`、`timelineVersion`、`filterSummary`、`filterResultVersion`、`cacheSaving` | 一覧のページごと、追加したページごと、キャッシュへの保存開始 |
| `filter-progress` | `FilterProgressUpdate`：`filterSummary`、`filterResultVersion` | 絞り込みの走査中（最短 100 ms ごと）と完了時 |

画面は `withProgress`（走っている `jobId` のものだけ反映）、`withFilter`・`mergeSessionView`（新しい絞り込み結果を古いもので上書きしない）で受け取ったものを合わせる。

### `SessionView` の取得に関わる項目

`phase`（`Idle`／`Fetching`／`Done`／`Failed`）、`canFetch`、`currentJobId`、`eventCount`、`lastJob`（`JobSummary`：`status`、件数、対象・完了・失敗ストリーム数、全体の失敗）、`progress`（取得中のみ）、`failedStreams`、`listingStatus`（`Complete`／`StoppedEarly`／`Partial`）、`listingFailure`、`failureListOpen`、`timelineVersion`、`discardGeneration`、`cacheEnabled`、`cacheSaving`、`cacheNotices`（`Hit`／`ReadFailed`／`SaveFailed`）、`closeConfirmation`。`JobStatus` は `Running`／`Completed`／`CompletedWithFailures`／`Failed`／`Aborted`。進み具合に開始時刻や経過時間の項目は無い。

## 4. 中核ライブラリの主な公開関数

（公開型の全体は各モジュールの文書コメントを参照。ここでは流れの入口と純粋な判断だけを挙げる。）

| 関数 | モジュール | 内容 |
|---|---|---|
| `run_fetch(gateway, fetch, timeline, retrier, sink) -> FetchJob` | `coordinator` | ロググループ全体の取得 1 回 |
| `run_fetch_with_cache(gateway, fetch, timeline: &Arc<T>, retrier, cache, fetch_started_at_ms, sink) -> FetchJob` | `coordinator` | キャッシュを前に置いた取得 |
| `decide_job_status(aborted, plan, failed_stream_count) -> JobStatus` | `coordinator` | 終わりの状態 |
| `fetch_stream(gateway, request, range, name, retrier, on_page) -> StreamFetch` | `fetcher` | 1 ストリームをページの終わりまで |
| `plan_streams(gateway, request, range, retrier, on_page) -> PlanOutcome` | `streams::planner` | ストリームの一覧と選別 |
| `is_selected`、`stops_listing`、`listing_cutoff` | `streams` | 選別の判断（1 時間の余裕） |
| `RetryPolicy::decide`、`delay_before_retry`、`Retrier::call`、`abort_pair`、`wait_or_abort` | `retry` | 再試行と中断 |
| `paging::decide`、`PageCursor` | `paging` | ページの終わり |
| `EventTimeline::append`、`rows`、`position_of`、`clear` | `timeline` | 時刻順の保持 |
| `run_listing(gateway, request, sink) -> ListingEnd` | `log_groups::listing` | ロググループの一覧 |
| `validate_fetch_input`、`validate_session_input` | `request` | 取得条件の検証 |
| `decide_write`、`record_range`、`find_covering_range`、`merge_ranges`、`cache_notices`、`resequence` | `cache::plan` | キャッシュの判断 |
| `LogCache::lookup`、`write`、`clear_all` | `cache` | キャッシュファイル |
| `build_safe_detail`、`redact_secrets`、`redact_access_key_ids` | `failure` | 安全な詳細と伏せ字 |
| `resolve_login_target`、`cli_args`、`extract_login_hints`、`is_allowed_login_url` | `sso_login` | SSO ログインの判断 |
| `resolve_connection_target` | `gateway::aws` | 接続先（プロファイル、リージョン）の解決 |

## 5. 開発用のコマンドライン

`cargo run -p local-sights-core --example fetch_check -- [--profile NAME] --log-group NAME --start "yyyy/mm/dd hh:mm:ss" --end "yyyy/mm/dd hh:mm:ss"`（UTC）。標準出力に 1 行 1 イベント、標準エラーに件数と失敗。終了コードは 0 成功、1 失敗あり、2 引数の誤り。開発者の手元で本人の認証情報を使うときだけ動かし、`cargo test` と CI はビルドするだけで動かさない。
