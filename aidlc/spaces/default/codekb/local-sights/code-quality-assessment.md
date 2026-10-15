# コード品質の評価（Local Sights）

走査したコミット：`9d08cbf5c6d97820f0ec7f276c3ce6020c903eb5`（2026-10-12）。この走査ではテスト・ビルド・lint を実行していない（読み取りのみ）。数はコードの数え上げによる。

## 総評

全体として良好。GUI に依存しない中核と薄いアプリの分離、trait による AWS の切り離し、純粋な判断関数への寄せ方、業務ルール番号つきの文書コメントが一貫している。team.md の決まり（`unwrap`／`expect` を使わない、AWS 呼び出しは trait の裏、テストと CI は AWS に接続しない、秘密を出さない）はコード上で守られている。弱い点は `AppSession`（`session.rs`）とアプリの `lib.rs` の大きさ、そして今回の intent に関わる「直列を前提にした取得の形」である。

## 3 段階の評価（code-analysis-guide の観点）

| 観点 | 評価 | 根拠 |
|---|---|---|
| 名前の分かりやすさ | good | 業務の言葉（`StreamPlan`、`ExhaustionStreak`、`DeferredFinishSink` など）と Rust／TS の慣用どおりの命名 |
| 関数の大きさ | fair | 判断関数は小さい。流れの関数（`run_fetch` 約 80 行、`run_fetch_with_cache` 約 55 行、`begin_and_spawn_fetch` 約 60 行）と `AppSession` のメソッド群はやや大きい |
| エラー処理 | good | すべて `Result`、失敗は種類と安全な詳細に変換、毒されたロックは取り戻す、異常終了した取得は `abort_fetch_with_failure` で回復。黙って捨てる箇所は見当たらない（診断は標準エラーへ） |
| テストの有無 | good | 中核 409、アプリ 12、画面とスクリプト 281 のテスト（属性・呼び出しの数）。取得経路は単体（`retry.rs`、`paging.rs`、`streams/mod.rs`、`coordinator.rs`）と結合（`tests/u1`〜`u6`）の両方 |
| 重複 | fair | SDK エラーの変換が操作ごとに 3 つ（`failure_from_sdk_error` ほか）。共通部分は `classify_sdk_error` にまとめてある。`copy_chunk_in_range` は共有されている |
| 使われないコード | fair | `virtualScroll.ts`（U3 のテストのための互換層）、`LogTableCloudscape.tsx`（試作、環境変数で切り替え）、`dev/perfRows.ts`（計測用）が残る。いずれも意図が文書化されている |

## テスト

- 中核：各モジュール内の単体テストと `tests/` の結合テスト 5 本。`tokio::test(start_paused = true)` で再試行の待ち（1・2 秒など）を実時間なしで確かめている。偽物は `tests/support/fake_gateway.rs`（台本どおりに即座に答え、要求と順を記録）と `recording_sink.rs`。
- アプリ：`sso_cli.rs` の 12 テスト（偽の CLI スクリプト、実際の AWS CLI と AWS は使わない）。
- 画面：Vitest＋jsdom＋Testing Library。`api` は `vi.mock` で差し替え、Tauri は起動しない。`commandRegistration.test.ts` がコマンドの登録のそろいを確かめる。
- カバレッジ：計測の設定は無い（team.md で数値の下限は設けないと決めている。代わりにライブラリ側の公開関数には必ずテストを書く）。
- 実 AWS での確認：`examples/fetch_check.rs` と `README.md` の手動確認の手順。CI では動かさない。

## lint・整形・CI

- Rust：`cargo fmt --all --check` 必須、`clippy` は警告をエラーにしない（`RUSTFLAGS: ""`）。ライブラリ単体の clippy は `rust-core` ジョブ、ワークスペース全体は `tauri-app` ジョブ（Linux と macOS）。
- 画面：`tsc --noEmit`、`eslint .`、`prettier --check .`、`vitest run`、`vite build`、`npm audit --audit-level=high`、版のそろいの検査。
- 依存：`cargo-deny`。
- 抑止：`#[allow]` 3 件（うち 2 件はテスト支援、1 件は macOS 以外の `dead_code`）、`@ts-expect-error` 1 件（テスト）。TODO／FIXME／HACK は 0 件。

## 技術的負債と気になる点

| # | 内容 | 場所 | 影響 |
|---|---|---|---|
| D1 | `AppSession` が接続・取得・絞り込み・キャッシュ設定・終了確認・SSO を 1 つの型で持つ（本体 約 2000 行、テストを含め 4663 行） | `crates/local-sights-core/src/session.rs` | 状態を足すたびに相互の断り条件（取得中、確認待ち、設定ダイアログ、SSO 中）が増える |
| D2 | アプリの `lib.rs` が 1449 行（コマンド、状態、受け口、メニュー、終了確認、SSO の起動） | `src-tauri/src/lib.rs` | 取り消しのコマンド追加でさらに増える |
| D3 | AWS SDK の再試行・時間切れの設定を明示していない | `crates/local-sights-core/src/gateway/aws.rs:173-189` | SDK 既定の再試行がアプリの `Retrier` の下で重なっていると考えられ、実際の呼び出し回数と待ちが見積もりとずれる（要確認） |
| D4 | CI のコメントが `deny.toml` の設定と食い違う（コメントは `all-features = true`、設定は `false`） | `.github/workflows/ci.yml:76`、`deny.toml:7` | 読み手の誤解 |
| D5 | リリースは下書きにせず公開するが、team.md の Deployment は「下書きとして置く」 | `.github/workflows/release.yml:4`、`docs/releasing.md:7` | 実践の記録と実装のずれ |
| D6 | Dock メニューの「終了」は確認なしに終わる（文書化済みの制限） | `src-tauri/src/lib.rs` 冒頭コメント | 取得中のログが確認なしに失われうる |
| D7 | `useEscapeKey` の説明（エラーバナーを Escape で閉じる）が `FetchErrorBanner` の振る舞い（Escape は使わない）と合わない | `src/hooks/useEscapeKey.ts`、`src/components/FetchErrorBanner.tsx` | 説明の古さ |
| D8 | 並行の初回呼び出しで同じ `(プロファイル, リージョン)` のクライアントを二重に作りうる（照会と登録の間でロックを放す） | `crates/local-sights-core/src/gateway/aws.rs` の `client_for` | 今は直列なので起きない。並行化すると最初だけ設定の解決が重なる（結果は同じで害は小さい） |

## 今回の intent（261012-parallel-fetch）への影響

取得経路の現状と、変更時に守ること・壊れやすいところを根拠つきでまとめる。

### 並行取得

- 直列の流れ：`run_fetch` は `plan_streams` を最後まで読んでから、選んだストリームを `for` で 1 つずつ `fetch_stream` に渡す（`coordinator.rs:314-373`）。
- 受け口が直列前提：`FetchSink` は全メソッド `&mut self`（`coordinator.rs:200-220`）。`fetch_stream` のページ受け取りは `FnMut`（`fetcher.rs:100-108`）。`run_fetch` は閉包の中で `timeline`、`latest_version`、`total`、`sink` を可変で借りる（`coordinator.rs:335-358`）。`TauriSink` は呼ばれるたびにセッションのロックを取りイベントを送る（`src-tauri/src/lib.rs:1274-1356`）。
- 並行に使える部分：`CloudWatchLogsGateway` は `&self` で `Send + Sync`。`Retrier` は `Copy`。`AbortSignal` は複製できる。`TimelineStore` は `&self` で `Send + Sync`。
- 件数と順番：ストリーム内の順番（`sequence`）はストリームごとに 0 から振るので、並行でも変わらない。並びのキーは `(timestamp, logStreamName, sequence)` で、届く順に依らず同じ並びになる。
- 混ぜ込みの費用：`EventTimeline::append`（`timeline.rs:57-92`）はページの最初の要素が入る位置から後ろを作り直す。古いページが後から届くほど作り直しが増え、`LogView` のロック（絞り込みの判定を含む、`log_view.rs:120-133`）が長くなり、画面の `get_rows` を待たせる。
- `job.outcomes` と `failed_streams` は「終わった順」に並ぶ（今は取得順と同じ）。並行では終わる順が揃わない。

### 呼び出し回数の上限とスロットリング時の引き下げ

- 今はアプリ全体の毎秒の回数を抑える仕組みが無い。再試行は要求ごとの固定の待ち（`retry.rs:25-33`、`297-329`）で、他の要求の失敗を知らない。
- 上限の前提：東京リージョンの `GetLogEvents` はアカウント全体で毎秒 25 回、その半分程度に抑える（ideation の制約 C-T2、C-O1）。D3 の SDK 側の再試行も回数に入る。
- 一覧（`DescribeLogStreams`）は 1 ページずつ順に読むので、今回の並行化の対象外になる見込み（制約 C-T3）。

### 連続使い切りの決まり

- `ExhaustionStreak`（`retry.rs:124-153`）は終わった順に「使い切り」が続いた数を数え、成功か使い切り以外の失敗で 0 に戻る。`run_fetch` は上限に達した後の残りを呼ばずに失敗とし、最後の失敗の内容を写す（`coordinator.rs:325-367`、`fetcher.rs:45-56` の `skipped`）。並行では「続けて」の意味が決まらないため作り直しが要る。
- 関連テスト：`tests/u3_fetch_flow.rs:345`（3 連続で残りを呼ばない）、`:374`（使い切り以外の失敗で数え直し）、`retry.rs` の単体テスト。

### 取り消し（ログを残す）

- 今の中断はログを捨てる：`abort()`（`coordinator.rs:380-391`）が `LogView` を空にし、件数 0、`Aborted` で終える。`finish_fetch` は `Aborted` を Idle に戻し、件数・失敗・要約を残さない（`session.rs:1436-1455`）。
- 中断の契機は閉じる確認の［閉じる］とウィンドウの `Destroyed` だけ（`src-tauri/src/lib.rs:215-222`、`388-399`、`run` 内）。利用者が取得だけを止めるコマンドは無い（`build.rs`、`capabilities/default.json`、`generate_handler!`、`src/api.ts`、`commandRegistration.test.ts` の 5 か所にそろえて足す必要がある）。
- 中断は「次の呼び出しの前」と「再試行の待ちの間」に効き、送信中の要求そのものは止めない（`retry.rs:297-329`）。取り消しの応答の速さは、1 回の `GetLogEvents` の応答時間に左右される。
- 終了確認の文言は「ここまで取得したログは失われます」（`src/i18n/messages.ts:375-376`、`CloseConfirmDialog.tsx`）。
- 関連テスト（捨てる振る舞いを確かめている）：`tests/u3_fetch_flow.rs:392`、`:432`、`coordinator.rs:1129`、`:1262`。

### 経過時間の表示

- `FetchProgress`（`session.rs:478-492`）と `FetchProgressUpdate` に開始時刻・経過時間は無い。取得の開始時刻はアプリが `now_epoch_ms()` で取りキャッシュの判断にだけ渡している（`src-tauri/src/lib.rs` の `begin_and_spawn_fetch`）。
- 状態行（`StatusLine.tsx:93-127`、文言 `status.fetching`、`messages.ts:190-193`）はイベントが届いたときだけ描き直される。再試行の待ち（最長 16 秒 +20%）の間はイベントが来ないので、経過時間を進めるには画面側で時計を回すか、中核から定期的に送る必要がある。

### ディスクキャッシュを変えないこと

- 書き込みは `Completed` かつ失敗ストリーム 0 のときだけ（`cache/plan.rs:373-384`）。照会は 1 つの保存範囲が丸ごと含むときだけ当たる。取り消しで終えた取得は書いてはいけない。
- `run_fetch_with_cache` は `run_fetch` の終わりを保留し、書き込んでから終える（`coordinator.rs:478-534`）。並行化しても、全ストリームの終わりを待ってから判定・書き込みをする順序を保つ必要がある。
- 関連テスト：`coordinator.rs` のキャッシュのテスト群（1019〜1417 行）、`tests/u6_cache_flow.rs`。

### 守るべき決まり（project.md）

- 読み取りの 3 操作以外を呼ばない（境界に他の操作は無い）。
- 自動テストと CI は AWS に接続しない（並行・回数・取り消しのテストも偽のゲートウェイで行う）。
- 秘密の認証情報とアクセスキー ID を表示・ログ・キャッシュに書かない（新しい警告や診断も `failure.rs` の決まりで作る）。
- テレメトリを送らない。
