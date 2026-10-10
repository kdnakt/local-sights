# Code Summary — U6 ディスクキャッシュ（u6-disk-cache）

計画：`code-generation-plan.md`（Plan Approval 済み、埋め込みの Testing Contract を含む）。単位テストの手順：`unit-test-instructions.md`。書いたファイルの一覧は `source-manifest.json`、ルールと要件の対応は `traceability.json`。キャッシュのファイルの書式は質問票 Q1 の回答どおり JSON Lines。

## 今回の作業（2026-10-10、既存のコードと計画の照合）

U6 のコードは以前の作業で作られており、その上に U7 の手が入っている。ツールを 2.11.0 に上げた後、U6 の計画承認がもう一度求められた。利用者は計画を承認し、U1〜U5 と同じく既存のコードを計画と照合する形で進めた。下の「最初のビルドの記録」は U7 より前の形で書かれているため、いまのコードとの違いはこの節に書く。

- 今回作った・変えた・消したアプリのソース：なし（変更は計画ファイルのチェックボックスだけ）。
- `source-manifest.json` のパス 35 件と、`traceability.json` の `OK` の対象は、すべて今も存在する。どちらも前回のものを引き続き使う。
- 計画の Step 12 が求める「U5 の記録の補足」（U5 R-04）は、最初のビルドで下に書いてあり、今回もそのまま有効。
- テストを先に書く順序（Step 3・4）は最初のビルドで行われた（下の「テスト先行の失敗の記録」）。今回は失敗の実行を作り直さず、既存のテストの中身と実行で確かめた。
- 計画の 21 項目のうち次の 3 項目はチェックを付けていない（この環境で使えないコマンドやツールを含むため）：
  - Step 1 の 1 つ目：`sha2` と `serde_json` は入っている。ただし `deny.toml` がなく `cargo-deny` も入っていないため、許可ライセンスの確かめはできない。代わりに `cargo tree -p sha2@0.11.0 -e normal -f '{p} {l}'` で手で確かめ、依存（sha2、digest 0.11.3、block-buffer、crypto-common、hybrid-array、cpufeatures 0.3.1、const-oid、ctutils、cmov、cfg-if、typenum）はすべて MIT OR Apache-2.0 だった。下の最初のビルドの記録にある「`generic-array` は MIT」は誤りで、いまの依存の木にあるのは `hybrid-array`（MIT OR Apache-2.0）。
  - Step 8 の 2 つ目（`cargo build -p local-sights`）と Step 11 の 2 つ目（`cargo clippy --workspace --all-targets`・`cargo build -p local-sights` を含む検査）：Tauri の Linux 用の前提ライブラリがないため。

### 手順ごとの結果

| 手順 | 結果 | 計画の文言との違い |
|------|------|--------------------|
| Step 1 骨組みと設定 | 2 つ目は満たしていた。1 つ目は未チェック（上のとおり） | なし |
| Step 2 テストの実行環境 | 満たしていた | なし |
| Step 3・4 純粋なロジック | 満たしていた | なし。`cache/plan.rs` のテストは 14 件（U7 が件数の確かめのテスト 2 件を足した）、`should_report_progress` は 4 件。テスト以外に `unwrap()` / `expect()` はない |
| Step 5 LogCache のファイル | 満たしている（U7 による違いあり） | ヘッダに `eventCount` と `rangeEventCounts` が加わり、`FORMAT_VERSION = 2` になった（U7。最初のビルドのレビュー R-03）。認証情報の型はない。U6 で書いた版 1 のファイルは「知らない版」として消して取り直す。壊れたものの種類に `CountMismatch` が加わった。Hit の読み出しは指定範囲の終わりではなく、それを含むキャッシュ済み範囲の終わりまで読んで件数と並びを確かめ、返すのは指定範囲のイベントだけ（計画の R-13 より厳しい）。書き込み中の印 `WriteTracker` が加わった（最初のビルドのレビュー R-02） |
| Step 6 FetchCoordinator | 満たしている（U7 による違いあり） | 保持ログからの写し取りもブロッキング用のスレッドで行う（`copy_on_blocking_thread`、最初のビルドのレビュー R-04）。そのため引数が `timeline: &Arc<T>` になり、`T: TimelineStore + HeldEvents + Send + Sync + 'static` が要る。書き込みの前に印を立てる `prepare_write` が加わった。Hit の途中で中断されたときは Aborted・NotSaved で終わる（最初のビルドの「計画との違い 9」）。テストは 16 件 |
| Step 7 AppSession | 満たしている（U7 による違いあり） | 設定のファイル操作は `SettingsWork`・`begin_save_settings`／`begin_clear_cache`・`finish_settings_work` に分かれ、セッションのロックの外で行える（最初のビルドのレビュー R-01。`save_settings`・`clear_cache` はこの 3 つを一度に回す形で残る）。書き込み中（取得が異常終了した後も続くものを含む）と設定の作業中はダイアログを開けず `Busy` になる。作業中に [Cancel] で閉じたダイアログは閉じたまま。`begin_fetch` は変えず、キーは `CacheKey::from_request` で作る（最初のビルドの違い 2）。計画にない誤りのキー `SessionError::SettingsClosed` がある |
| Step 8 Tauri のつなぎ | 1 つ目は満たしている（違いあり、コードを読んで確かめた）。2 つ目は未チェック | キャッシュの場所は `app_cache_dir()` の直下ではなく `log-cache`（最初のビルドの違い 4）。`save_settings`・`clear_cache` は async のコマンドで、ファイル操作は `spawn_blocking` でロックの外で行う（最初のビルドのレビュー R-01）。U7 で閉じる確認と、書き込み中の終了の止め方が加わった |
| Step 9 結合テスト | 満たしていた | なし（`u6_cache_flow` 3 件） |
| Step 10 画面 | 満たしていた | なし。`cache.*` の文言は英日とも BR5.3 と一致する |
| Step 11 ビルドの設定 | 1 つ目は満たしていた。2 つ目は未チェック | なし |
| Step 12 doc コメントと記録 | 満たしている | `cargo rustdoc -- -W missing_docs` で missing_docs の警告は 0 件。記録はこのファイル |

### ルールの文言といまのコードの違い

`traceability.json` では次の項目も `OK` のままにしている。U6 として作った実装とテストは残っており、下の「そのまま成り立つ部分」はいまも満たしているため：

| ID | いまのコードとの違い | そのまま成り立つ部分 |
|----|----------------------|----------------------|
| BR3.6 | ヘッダに `eventCount` と `rangeEventCounts` も書く（U7） | 認証情報を書かない、0700・0600、緩いフォルダを直す |
| BR4.1 | 書式は版 2 になり、版 1 のファイルは「知らない版」として消して取り直す。件数が合わないことも壊れたとみなす。行の切れ目ちょうどで切れたファイルも「途中で切れている」と見分ける（文言より厳しい） | ほかの文言の内容はすべて守られている |
| BR4.2 | Hit のとき、指定範囲ではなくキャッシュ済み範囲の終わりまで読む（下の最初のビルドの記録の「指定範囲の終わりを過ぎたところで読むのをやめる」とも合わなくなった） | 範囲に入らないときはイベントを読まない |
| BR5.1 | 取得が異常終了した後も続く書き込みと、設定の作業中も、設定ダイアログを開けない（文言より厳しい） | 取得中（キャッシュへの書き込み中を含む）は開けない |
| BR3.5 | 写し取りもブロッキング用のスレッドで行う（文言より厳しい） | ロックを握り続けない、書き込みに失敗しても取得結果を残す |
| BR1.3 | ファイル操作はロックの外で行い、作業中に [Cancel] で閉じたダイアログは閉じたまま | 設定を書く → 全削除 → 両方成功したときだけ閉じる（R-10） |

上の表以外の U6 の BR は、ルールの文といまのコードが合っている。FR7.x・NFR については、コードに反するものは見つかっていない（requirements.md の文言と 1 件ずつの照合まではしていない）。

### テストと検査の結果（今回）

| コマンド | 結果 |
|----------|------|
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- cache:: coordinator:: session:: filter::should_report_progress` | 135 件成功 |
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u6_cache_flow` | 3 件成功 |
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core`（全体） | lib 305 件成功（5 件 ignore）、結合テスト u1 11・u2 9・u3 14・u5 4・u6 3 件成功 |
| `npx vitest run`（U6 の 6 ファイル） | 69 件成功 |
| `npx vitest run`（全体） | 21 ファイル 171 件成功 |
| `cargo fmt --check` | 成功 |
| `CARGO_INCREMENTAL=0 cargo clippy -p local-sights-core --all-targets` | 成功・警告 0 件（`--workspace` は src-tauri を組み立てられないため実行できない） |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `npm audit` | 脆弱性 0 件 |
| `cargo rustdoc -p local-sights-core --lib -- -W missing_docs` | missing_docs の警告 0 件。ほかに `[Save]`・`[Cancel]`・`[Close]` を doc のリンクと読み違えた `broken_intra_doc_links` の警告が `session.rs` に 10 件（見た目だけの問題。直していない） |
| `cargo build -p local-sights` | このコンテナでは実行できない（Tauri の Linux 用の前提ライブラリがない） |
| `cargo deny check licenses` | 未実行（`cargo-deny` も `deny.toml` もない） |

## 最初のビルドの記録

### 作ったもの・変えたもの

| 場所 | 中身 |
|------|------|
| `crates/local-sights-core/src/cache/plan.rs`（新規） | LogCache の純粋なロジック。CacheKey（プロファイルの kind と名前・リージョン・ロググループ。`from_request`、`digest` は各部分の長さを前に置いて SHA-256 にした 16 進 64 文字、`file_name` は `<digest>.cache`、`temp_file_name` は `<digest>.cache.tmp-<任意>`、BR2.1・BR2.2・BR1.4）、CoveredRange、CachedEvent、CacheHeader（formatVersion・key・coveredRanges）、`record_range`（min(終了, 開始時刻 − 300,000)、開始より前なら記録なし、BR3.1）、`find_covering_range`（1 つの範囲にすっぽり入るか、境界のミリ秒を含む、BR2.3・BR2.4）、`merge_ranges`（重なりと隣り合いをつなぐ、BR3.3）、`replace_events_in_range`（範囲外は残し範囲内は新しいものだけ、U3:BR4.1 の順に合わせる、BR3.3）、`decide_write`（有効・Hit でない・Completed・失敗 0・記録できる範囲あり、BR3.2）、`resequence`（ストリームごとに 0 から振り直す、BR4.3）、`ranges_are_well_formed`・`validate_header`・`EventCheck`・`validate_events`（壊れたものの見分け、BR4.1・R-13）、`classify_file_name`、CacheOutcome（NotUsed・Hit・Saved・NotSaved・SaveFailed）、CacheNotice と `cache_notices`（BR5.3） |
| `crates/local-sights-core/src/cache/mod.rs`（新規） | LogCache（キャッシュ用フォルダ）。`read_header`（1 行目だけ、BR4.2）、`lookup`（ヘッダで判定し、すっぽり入るときだけ指定範囲の終わりまでイベントを読み、読んだイベントを 1 件ずつ確かめる。戻り値で Missing・Unreadable・Corrupt・NotCovered・Hit を分け、Corrupt のときだけファイルを消す、R-13・BR4.1）、`write`（既存を読んで確かめ、通れば入れ替えてつなぎ、通らなければ今回の範囲だけで作り直す。キーの残った一時ファイルを消してから、一時ファイルに全部書いて `sync_all` し `rename`、BR3.3・BR3.4）、`remove_entry`、`clear_all`（直下の通常のファイルで名前の型が合うものだけ。`DirEntry::file_type` でシンボリックリンクをたどらず消さない、BR1.4）。フォルダは 0700 で作り、緩ければ 0700 に直し、直せなければ書かない。ファイルは 0600（BR3.6）。CacheError（I/O の種類だけを持ち、パスや中身を持たない） |
| `crates/local-sights-core/src/cache/settings.rs`（新規） | 設定ファイル `{"formatVersion":1,"cacheEnabled":bool}`。`load_settings`（ない・読めない・解釈できない・知らない版は無効。読めない・解釈できないときは標準エラーにだけ残す、BR1.2）、`save_settings`（フォルダの権限と一時ファイルからの入れ替えはキャッシュと同じ、BR1.1・BR3.6） |
| `crates/local-sights-core/src/coordinator.rs` | FetchJob に `served_from_cache`・`cache_outcome`・`read_failed`。FetchSink に `on_saving(job_id)`（既定は何もしない）。HeldEvents（保持ログから範囲のイベントを区切りごとにロックを取って放しながら写す）、DeferredFinishSink（`on_finished` だけを保留）、`run_fetch_with_cache`（無効なら `run_fetch` のまま。有効なら `spawn_blocking` で引き、Hit なら破棄 → on_started → 1 回の追加 → on_batch（1 件以上）→ on_finished。入らない・ない・読めない・壊れたなら `run_fetch` を DeferredFinishSink で回し、書く条件なら on_saving → 4,096 件ごとに写す → `spawn_blocking` で書く → Saved / SaveFailed、書かないなら NotSaved として on_finished、BR1.5・BR2.3・BR2.4・BR3.2・BR3.5・BR3.7・BR4.1・R-12）。`run_fetch` は変えていない |
| `crates/local-sights-core/src/log_view.rs` | `Mutex<LogView>` に HeldEvents を実装（書き込みのための写し取り、BR3.5） |
| `crates/local-sights-core/src/filter.rs` | `FILTER_PROGRESS_INTERVAL`（100 ミリ秒）と `should_report_progress`（最初と Ready は必ず送る、間は 100 ミリ秒に 1 回まで、U5 R-03） |
| `crates/local-sights-core/src/session.rs` | CacheLocation・SettingsDialog（Closed・Open）・SettingsNotice（Cleared・ClearFailed・SaveFailed）。`with_cache_location`（起動時に設定を読む。渡さない既定の作り方は無効でディスクに触れない、BR1.6）、`fetch_cache`（無効なら None、BR1.5）、`open_settings`（取得中は Busy、確認待ちは ConfirmationPending）、`cancel_settings`、`save_settings(enabled)`（R-10 の順：設定を書く → 書けなければ前の値のまま開いたまま → 有効から無効なら全削除 → 失敗なら無効のまま開いたまま → 成功で閉じる）、`clear_cache`、`on_saving`（現在のジョブのときだけ）。ダイアログが開いているあいだ、取得の開始・入力・接続・ロググループ・一覧の絞り込み・ログの絞り込みを ConfirmationPending で拒む（BR5.1）。`finish_fetch` で cacheSaving を false にして cacheNotices を作り、`abort_fetch_with_failure` でも cacheSaving を false にして cacheNotices を空にする（R-11）。取得の開始と接続の変更で cacheNotices を空にする。SessionView に cacheEnabled・cacheDirectory・settingsDialog・settingsNotice・canOpenSettings・cacheSaving・cacheNotices、FetchProgressUpdate に cacheSaving。SessionError に SettingsClosed |
| `crates/local-sights-core/src/lib.rs`・`Cargo.toml`・`crates/local-sights-core/Cargo.toml`・`Cargo.lock` | `cache` モジュールの宣言。`sha2 = "0.11"` を足し、`serde_json` を dev から本番の依存に移した |
| `crates/local-sights-core/tests/u6_cache_flow.rs`（新規） | U3 の偽物の gateway・`Mutex<LogView>`・一時フォルダでの結合テスト 3 件 |
| `src-tauri/` | `setup` の中で `app_config_dir()/settings.json` と `app_cache_dir()/log-cache` を決めて AppSession に渡す（決められなければ無効のまま、診断ログ）。取得は `run_fetch_with_cache`（キャッシュは `fetch_cache()`、取得を始めた時刻は `SystemTime`）。`TauriSink::on_saving` は `fetch-progress` を送る。コマンド `open_settings`・`cancel_settings`・`save_settings(enabled)`・`clear_cache`（どれも `session-changed` を送る）。走査の間引きを `should_report_progress` に置き換えた。`build.rs`・`capabilities/default.json` に 4 つの権限だけを足し、生成された `permissions/autogenerated/*.toml` 4 つ |
| `src/` | `api.ts`（4 つのコマンド、SettingsDialog・SettingsNotice・CacheNotice の型、SessionView と FetchProgressUpdate の新しい項目、`withProgress` が cacheSaving を写す）、`components/SettingsDialog.tsx`（新規。チェックボックス・保存場所・注意・[Clear cache]・[Cancel]・[Save]、開いたらチェックボックスに入力位置、Tab と Shift+Tab はダイアログの中で回る、Escape は [Cancel]、閉じたら [*] に戻す、知らせの文字）、`ConnectionBar.tsx`（[*]、canOpenSettings が false なら押せない）、`StatusLine.tsx`（取得中の cache.saving、終わったあとの cache.hit・cache.readFailed・cache.saveFailed、Hit のときは件数と cache.hit だけ）、`LogFilterInput.tsx`（Enter ですぐ渡す、失敗の回数が増えたときとダイアログが閉じたときに `logFilter` と比べて違えば送り直す）、`FetchForm.tsx`（設定ダイアログ中も入力と絞り込みの欄を無効に）、`App.tsx`（ダイアログの表示とコマンド、[*] の ref、絞り込みの失敗の回数）、`i18n/messages.ts`（`settings.*`・`cache.*`・`session.settingsClosed` を英日。`cache.*` は BR5.3 の文言のまま） |
| `README.md` | 状態の説明、絞り込みの Enter、U6 の手元の確認項目（ダイアログの操作、設定が残ること・権限、取得 → 同じ条件で Hit、5 分前より新しい部分、広げた範囲、壊れたファイル、[Clear cache] と無効にして保存、100 万件近い書き込みの時間） |

### 主な判断

- R-12：`run_fetch` は変えず、`run_fetch_with_cache` が Hit と AWS の両方の流れを持つ。AWS から取得するときは DeferredFinishSink で `on_finished` を保留し、書き込みを済ませてから cacheOutcome・readFailed を入れたジョブで元の受け口に `on_finished` を出す。受け口の知らせの順は BR3.7 のとおり。
- 書き込みのための写し取りは、`TimelineStore` に手を入れず、新しい trait `HeldEvents` で行う（1 回の呼び出しで 1 回ロックを取って放す、4,096 件ずつ）。取得中のロック（フェーズが Fetching のまま）で追加と破棄は起きないため、添字を区切りの位置として使える。
- R-13：ファイルが「ない」は単にキャッシュなし。開けない・読めない（I/O の誤り）は readFailed として AWS から取り直し、ファイルは消さない。消すのは、解釈できない・途中で切れている（最後の行に改行がない、空）・知らない版・キーが違う・範囲の形が違う・範囲外のイベント・並びが違う、のときだけ。UTF-8 でない行は中身が壊れたものとして扱う。
- Hit の読み出しは、ヘッダで判定したあと、指定範囲の終わりを過ぎたところで読むのをやめる。確かめは読んだ行すべて（指定範囲より前の行を含む）に行う。
- キャッシュのキーに要る値（kind と名前・リージョン・ロググループ）は、U2 から `begin_fetch` が返す `ValidatedFetch` の FetchRequest（`with_connection` 済み）にすでに入っているため、`begin_fetch` の形は変えず `CacheKey::from_request` で作る。
- キャッシュ用フォルダは `app_cache_dir()` の直下ではなく、その中の `log-cache` にした。`app_cache_dir()` は macOS でウェブビューのキャッシュなどと同じ場所になりうるため、このアプリのファイルだけのフォルダにした（全削除は名前の型が合うファイルだけを消すため、どちらでも他のファイルは消さない）。
- `sha2` は AWS SDK がすでに使っている 0.11 を使い、`Cargo.lock` に新しいクレートは増えていない（依存の行が 1 つ増えただけ）。
- 設定ダイアログが開いているあいだの拒否は、計画のとおり U2 の確認待ちと同じ誤りのキー（`session.confirmationPending`）を使う。ダイアログが閉じているのに [Save]・[Clear cache] が届いたときのために、新しい誤り `session.settingsClosed` を足した。
- 場所を渡されていない AppSession で [Save] を押すと、書く場所がないため「設定を保存できませんでした」を出し、無効のまま開いておく。[Clear cache] は消すものがないため「キャッシュを消しました」を出す。アプリは常に場所を渡す（OS のフォルダが決められないときだけ渡さない）。
- 絞り込みの欄の送り直し（U5 R-02）は、入力欄が無効のあいだはしない。失敗はダイアログ中の競合でしか起きないため、ダイアログが閉じたときに送り直せば足りる。失敗は今までどおり誤りの帯にも出す。
- 秘密情報：書く型は CacheHeader（formatVersion・CacheKey・coveredRanges）・CachedEvent・設定の 1 項目だけで、認証情報の型を受け取る関数はない。新しい診断ログはキー・ロググループ・メッセージを含まず、事実と I/O の種類だけを出す。AWS の新しい API 呼び出しはない（Hit のときは 1 回も呼ばない）。

### テストの結果（最初のビルド）

#### テスト先行の失敗の記録（Testing Contract の Step 3）

`cache/plan.rs` のテスト 12 件と `filter::should_report_progress` のテスト 4 件を、中身を `todo!()` にした関数に対して書き、実装の前に単位のコマンドを実行した。

```text
$ CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- cache:: coordinator:: session:: filter::should_report_progress
test cache::plan::tests::the_recorded_range_ends_five_minutes_before_the_fetch_started ... FAILED
test cache::plan::tests::ranges_join_when_they_overlap_or_touch_and_stay_apart_otherwise ... FAILED
test filter::should_report_progress_tests::the_first_report_always_goes ... FAILED
...（plan.rs 12 件・should_report_progress 4 件がすべて FAILED）
thread 'cache::plan::tests::the_recorded_range_ends_five_minutes_before_the_fetch_started' panicked at crates/local-sights-core/src/cache/plan.rs:245:5:
not yet implemented
test result: FAILED. 61 passed; 16 failed; 0 ignored; 0 measured; 169 filtered out
```

実装の後、同じコマンドで 77 件成功（失敗 0）。ファイルの読み書き・コーディネーター・AppSession・Tauri・画面は、計画どおり実装してからテストを書いた。

#### 最終の結果

| コマンド | 結果 |
|----------|------|
| `CARGO_INCREMENTAL=0 cargo test --workspace` | 成功（lib 274 件成功・5 件 ignored、u1_fetch_flow 11、u2_log_group_listing 9、u3_fetch_flow 14、u5_filter_flow 4、u6_cache_flow 3）。始める前は lib 225 件で合計 263 件、終わりで合計 315 件 |
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- cache:: coordinator:: session:: filter::should_report_progress`（U6 の単位のコマンド） | 110 件成功 |
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u6_cache_flow` | 3 件成功 |
| `npx vitest run` | 14 ファイル 122 件成功（始める前は 13 ファイル 104 件） |
| `npx vitest run src/components/SettingsDialog.test.tsx src/components/ConnectionBar.test.tsx src/components/StatusLine.test.tsx src/components/LogFilterInput.test.tsx src/App.test.tsx src/i18n/messages.test.ts`（U6 の単位のコマンド） | 6 ファイル 60 件成功 |
| `cargo fmt --check`・`cargo clippy --workspace --all-targets` | 成功・警告 0 |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `npm audit` | 脆弱性 0 |
| `CARGO_INCREMENTAL=0 cargo build -p local-sights` | 成功 |

#### 部品ごとの新しいテストの件数

| 部品 | 場所 | 件数 |
|------|------|------|
| LogCache（純粋なロジック） | `cache/plan.rs` | 12 |
| LogCache（ファイル） | `cache/mod.rs` 8・`cache/settings.rs` 5 | 13 |
| FetchCoordinator | `coordinator.rs` | 9 |
| AppSession | `session.rs` | 11 |
| FilterEngine | `filter.rs`（`should_report_progress`） | 4 |
| 取得とキャッシュ | `tests/u6_cache_flow.rs` | 3 |
| DesktopUi | `SettingsDialog.test.tsx` 6・`ConnectionBar.test.tsx` 1・`StatusLine.test.tsx` 4・`LogFilterInput.test.tsx` 3・`App.test.tsx` 3・`messages.test.ts` 1 | 18 |

### 計画との違い（最初のビルド）

1. `deny.toml` がリポジトリにないため、計画 Step 1 の「`deny.toml` の許可ライセンスに収まることを確かめる」はできなかった。代わりに `cargo metadata` で `sha2` 0.11 とその依存のライセンスが MIT OR Apache-2.0（`generic-array` は MIT）であることを確かめた。新しいクレートは `Cargo.lock` に増えていない。
2. 計画は「`begin_fetch` の結果にキャッシュのキーに要る値を入れる」としていたが、その値はすでに `begin_fetch` が返す FetchRequest に入っていたため、`begin_fetch` は変えずに `CacheKey::from_request` で作った。
3. 計画の部品の表にない `log_view.rs` を変えた（`Mutex<LogView>` に HeldEvents を実装）。保持ログから区切りごとに写すための trait `HeldEvents` は計画にない新しい型。
4. キャッシュ用フォルダを `app_cache_dir()` そのものではなく、その中の `log-cache` にした（理由は「主な判断」）。
5. `session.settingsClosed`（SessionError::SettingsClosed）を足した。計画にない誤りのキー。
6. FetchCoordinator の単体テストは、ライブラリの中から `tests/support/fake_gateway.rs` を使えないため、`coordinator.rs` のテストの中に小さな偽物の gateway（`FakeLogs`、呼び出しを数える）を置いた。U3 の偽物の gateway は結合テスト `u6_cache_flow.rs` で使った。
7. 書き込みの途中の失敗で前のファイルが残ることは、実際のディスクの失敗ではなく、入れ替えの関数 `write_atomically` に途中で失敗する書き手を渡して確かめた。権限で読めない状態は root では作れないため、キャッシュのファイルの位置にフォルダを置いて「開けるが読めない」状態を作った。
8. 「壊れたキャッシュは取り直してファイルが消える」の FetchCoordinator のテストは、記録できる範囲がない取得で行った（記録できる取得では、消えたあとに新しいファイルで置き換わるため）。置き換わる場合は LogCache のテスト（壊れた既存を作り直す）で確かめた。
9. 取得の中断の合図がキャッシュの読み出しのあとに来ていたときは、Hit の追加をせず U3:BR5.5 のとおり Aborted で終える処理を足した（計画に書いていない場合の扱い）。

## U5 の記録の補足（U5 R-04）

承認済みの U5 の記録（`construction/u5-filter/code-generation/` の `traceability.json`・`code-summary.md`）は書き換えず、ここに補う。

### U5 の traceability にないテストのファイルと `src-tauri/src/lib.rs` の対応

| U5 の ID | U5 の traceability の対応先 | 補う対応先 |
|----------|------------------------------|------------|
| U5:BR1.1・BR1.2・BR1.4・BR2.3・FR6・FR6.5 | `filter.rs` | `filter.rs` の中のテスト（`#[cfg(test)] mod tests`） |
| U5:BR1.3 | `src/components/LogFilterInput.tsx` | `src/components/LogFilterInput.test.tsx` |
| U5:BR1.5 | `filter.rs` | `src-tauri/src/lib.rs`（`spawn_filter_scan`・`run_filter_scan`：ブロッキング用のスレッドで 4,096 行ずつロックを取って放す）、`filter.rs` のテスト |
| U5:BR2.1 | `filter.rs` | `tests/u5_filter_flow.rs`、`filter.rs` のテスト |
| U5:BR2.2 | `log_view.rs` | `log_view.rs` の中のテスト、`tests/u5_filter_flow.rs` |
| U5:BR2.4 | `filter.rs` | `src-tauri/src/lib.rs`（古いチケットの走査が Stale で自分で終わる）、`tests/u5_filter_flow.rs` |
| U5:BR2.5 | `log_view.rs` | `src-tauri/src/lib.rs`（ロックの順をセッション → LogView に保つ `set_log_filter`・`get_rows`・`sync_filter`）、`log_view.rs` のテスト |
| U5:BR3.1・FR6.1・FR6.4 | `log_view.rs` | `log_view.rs` のテスト、`tests/u5_filter_flow.rs` |
| U5:BR3.2 | `src/components/LogTable.tsx` | `src/components/LogTable.test.tsx` |
| U5:BR3.3・FR6.3 | `src/components/StatusLine.tsx` | `src/components/StatusLine.test.tsx` |
| U5:BR3.4・NFR13 | `src/components/LogFilterInput.tsx` | `src/components/LogFilterInput.test.tsx`、`src/components/FetchForm.test.tsx` |
| U5:BR3.6 | `session.rs` | `session.rs` のテスト、`src/App.test.tsx` |
| U5:NFR1・NFR2 | `log_view.rs` | `filter.rs`・`log_view.rs` の速さのテスト（`--release --ignored`） |
| U5:NFR12 | `src/i18n/messages.ts` | `src/i18n/messages.test.ts` |

U5 R-03 により、U5:BR3.6 の走査の知らせの間引き（100 ミリ秒、Ready は必ず送る）の判断は U6 で `filter::should_report_progress` に移し、`filter.rs` のテストで確かめている（`src-tauri/src/lib.rs` はそれを呼ぶだけ）。

### U5 の「計画との違い」のうち違いではなかった 1 件

U5 の `code-summary.md` の「計画との違い」の 9 番「テストの件数は目安どおりか多い」は、計画の目安を満たしたという報告で、計画との違いではない。U5 の計画との違いは 1〜8 番の 8 件。

## まだ確かめていないこと・未解決

- `cargo build -p local-sights`・`cargo clippy --workspace --all-targets`・`cargo-deny` によるライセンスの確かめは、Tauri の前提ライブラリと `cargo-deny`（と `deny.toml`）がある手元か CI で確かめる（2026-10-10 の照合ではこのコンテナで実行できなかった）。確かめたら計画の Step 1 の 1 つ目、Step 8・Step 11 の 2 つ目にチェックを付ける。

- 実際の AWS での取得、画面での見た目と操作（ダイアログのキーボード操作、[*] への入力位置の戻り、Hit の表示、「キャッシュに保存中」の表示）、macOS の `~/Library/Caches/dev.local-sights.app/log-cache` と `~/Library/Application Support/dev.local-sights.app/settings.json` の場所と権限は、開発者本人の手元で確かめる（`README.md` の「Disk cache (U6)」）。
- 100 万件近い書き込みにかかる時間は測っていない（設計で上限を決めていない、NFR2）。書き込みのあいだは取得中のままのため、その分だけ取得の終わりが遅れる。写し取りはイベントを 1 度メモリに写すため、100 万件ではその分のメモリを一時的に使う。手元の確認で測る。
- ファイルがちょうど行の切れ目で切れた場合（最後の行に改行が残る形）は、ヘッダに件数を持たないため「途中で切れた」と見分けられない。書き込みは一時ファイルからの入れ替えのため、この形はアプリの止まり方では起きない見込み。
- ディスクの読み書きは設定ダイアログの操作（[Save]・[Clear cache]）ではセッションのロックを持ったまま行う。ダイアログは取得中に開けないため、取得とはぶつからない。キャッシュがとても大きいと全削除のあいだ画面の他の要求が待つ。
- リポジトリにはまだ `deny.toml` と CI の設定がないため、`cargo-deny` は実行していない。
