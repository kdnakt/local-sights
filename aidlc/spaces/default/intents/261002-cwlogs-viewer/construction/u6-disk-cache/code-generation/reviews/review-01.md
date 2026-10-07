## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-07T08:23:38Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src-tauri/src/lib.rs:497-525（`save_settings`・`clear_cache`・`change_settings`）、crates/local-sights-core/src/session.rs:592-642 | `save_settings` と `clear_cache` は `async` でない Tauri コマンドで、Tauri 2 ではメインスレッドで動く。そこでセッションの Mutex を握ったまま、`write_atomically` の `sync_all`・`rename` と `clear_all` の削除ループを同期で実行する。キャッシュが大きいと、全削除のあいだ UI と `get_rows` などのほかの要求が止まる。code-summary.md「まだ確かめていないこと」にも書かれているが、追跡する項目になっていない。取得とはぶつからないので誤動作はしない | 次の作業単位に回す（org の「レビュー上限後の指摘は次の作業単位へ」ルールに従う）。ファイル操作はロックの外のブロッキング用スレッドで行い、結果だけロックの下で反映する形にする。または `#[tauri::command(async)]` と `spawn_blocking` にする | New |
| R-02 | Minor | src-tauri/src/lib.rs:533-560（`recover_if_still_fetching`）、crates/local-sights-core/src/coordinator.rs:604-623（`save`） | 取得タスクが異常終了すると、復旧処理がフェーズを Failed に戻して [*] を押せるようにする。しかし `spawn_blocking` で動き出した書き込みは、元のタスクが落ちても止まらない。その間に [Clear cache] や無効化の [Save] が走ると、全削除のあとで `rename` が通り、消したはずのファイルが復活しうる。ふだんは起きず、タスクが panic したときだけ起きる | 書き込み中を示す印を `LogCache` か `AppSession` に持たせ、復旧後も書き込みが終わるまで `open_settings` を拒む。または、この状況は受け入れると code-summary.md に明記する | New |
| R-03 | Minor | crates/local-sights-core/src/cache/mod.rs:418-462（`read_events`・`read_line`）、cache/plan.rs:86-97（`CacheHeader`） | ファイルが行の切れ目ちょうどで切れた場合は「途中で切れている」と見分けられず、足りない行のまま Hit（「AWS は呼んでいない」）で表示される。ヘッダにイベント数がない。Hit の読み出しは `end_ms` を過ぎると読むのをやめるため、その後ろの壊れも見ない。code-summary.md:126 で認めており、一時ファイルからの入れ替えのため起きにくい。ただし BR4.1 は「途中で切れている」を壊れたものとしている | ヘッダに `eventCount` を足す（書き込み時は `merged` がメモリにあり数えられる）。範囲全体を読む場合は終端で数を突き合わせる。足さない場合は、この限界を BR4.1 の既知の制約として rules.md か記録に残す | New |
| R-04 | Minor | crates/local-sights-core/src/coordinator.rs:589-600、521（`copy_held_events`） | 保持ログからの写し取りは、`async fn` の中で同期のまま回る。ロックは 4,096 件ごとに放すが、`CachedEvent::from` と `String` の移動・Vec への追加は、100 万件ならまとまった CPU 時間と割り当てになり、その間 tokio のワーカー 1 本を占める。BR3.5 は既存の読み出し・合わせ込み・書き込みをブロッキング用スレッドで行うことを求めており、写し取りは明文化されていない。マルチスレッドのランタイムなのでほかのタスクは進む | 写し取りを `spawn_blocking` の中、または `block_in_place` の中に移す。手元の 100 万件の計測（README の確認項目）で問題が出たときでもよい | New |
| R-05 | Minor | crates/local-sights-core/src/cache/mod.rs:206-214、310-326（`write`・`remove_leftover_temp_files`） | アプリを 2 つ同時に起動すると、片方の `write` が、もう一方の書き込み途中の一時ファイルを同じキーの接頭辞で消す。消された側は `rename` が失敗して SaveFailed になり、キャッシュは壊れず安全側に倒れる。読み出し・合わせ込み・書き込みはプロセス間で排他しないため、後から書いた側が先の追記を失う更新の取りこぼしも起きうる | 1 人用ツールなので許容できる。許容するなら記録に残す。直すなら、一時ファイルの消去を一定時間より古いものだけに絞る。または、フォルダに排他ロックを置く | New |
| R-06 | Minor | crates/local-sights-core/src/coordinator.rs:826-858（`Recorder`）、976-1016（Hit のテスト）、1068-1096（中断のテスト） | BR3.7 の「破棄 → on_started」の順をテストが固定していない。`Recorder` は知らせの種類しか記録せず、`on_started` の時点で保持ログが空かを見ない。破棄を `on_started` の後ろに動かしても通る。計画の違い 9 番で足した Hit のときの中断（`serve_from_cache` の `is_aborted` 分岐）にもテストがない。中断のテストは、キャッシュが別の範囲にある NoCache の流れしか通らない。AWS の流れで readFailed のまま書き込みが成功する Saved の組み合わせ（read_failed と cache_outcome）も、コーディネーター側では確かめていない | `Recorder::on_started` で保持ログの件数を記録して 0 を検証する。キャッシュにある範囲を指定し、中断を立てた状態で取得するテストを足し、Aborted・NotSaved・`on_started` と `on_finished` の順を確かめる。記録できる範囲がある壊れたキャッシュの取得で、`read_failed == true` と Saved を確かめる | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `CARGO_INCREMENTAL=0 cargo test -p local-sights-core --offline` | PASS: lib 274 件成功・5 件 ignored、結合テストは 11・9・14・4・3 件ともに成功 | code-summary.md の数値と一致。実際の AWS には接続しない |
| `npx vitest run`（SettingsDialog・StatusLine・LogFilterInput・App の各テスト） | PASS: 4 ファイル 43 件 | BR5.2・BR5.3・BR5.4 の挙動に反するものなし |
| `git show 118ca83 --name-only` と source-manifest.json | 一致: 書き込み一覧にコードの全ファイルが含まれる。差分にある `aidlc/` 配下の記録は意図したもの | 台帳は正確 |
| `Cargo.lock` の差分 | `sha2 0.11.0` の依存の 1 行だけ増えた | code-summary.md の「新しいクレートは増えていない」と一致 |
| 文言の照合: `src/i18n/messages.ts` と rules.md BR5.3 | 一致: cache.saving・hit・readFailed・saveFailed とも、英日が正本どおり | 文言のずれなし |

### 観点ごとの確認結果

- BR3.7 と DeferredFinishSink（coordinator.rs:393-441、473-527）。Hit は「破棄 → on_started → 1 回の追加 → on_batch → on_finished」で、listing・planned・stream_finished は出さない。AWS の流れは、`run_fetch` を変えずに `on_finished` だけを保留する。書く場合は `on_saving` → 写し取り → 書き込み → `on_finished` の順になる。書かない場合はすぐ `on_finished`。保留しているあいだフェーズは Fetching のままで、`finish_fetch` が `cacheSaving` を false に戻し、通知を作る。仕様どおり。
- BR4.1 と R-13（mod.rs:157-192、363-462）。「ない」は Missing（readFailed にしない）。「開けない・読めない」は Unreadable で、ファイルを残し readFailed として取り直す。「壊れた」は Corrupt で、ファイルを消して取り直す。UTF-8 でない行は壊れとして扱う。消せなくても取得は続ける。Unix ではディレクトリも `File::open` で開けるが、読みで EISDIR になり Unreadable に分類される（テストあり）。
- BR3.5 のロック。取得中のフェーズ固定で、追加と破棄は起きない。`HeldEvents` は区切りごとにロックを取って放し、書き込みはロックの外のブロッキング用スレッドで行う。lib.rs のロック順（セッション → LogView）は崩れていない。
- BR3.6。フォルダは 0700 で作り、緩ければ 0700 に直し、直せなければ書かない。ファイルは `create_new` と mode 0600 で作る。書くのは CacheHeader・CachedEvent と設定の 1 項目だけで、認証情報を受け取る関数はない。診断出力と `CacheError` はパスも本文も持たない。ただし、無効のときにディスクへ書く経路はない（`fetch_cache()` が None）。
- BR1.4。`DirEntry::file_type` は symlink をたどらず、symlink・ディレクトリ・名前の型が合わないファイルは残る（テストあり）。`remove_leftover_temp_files` も同じ判定。`remove_entry` は壊れたキャッシュの削除で、symlink の場合はリンク自体を消すが、全削除ではないため BR1.4 には反しない。
- R-10 の順（session.rs:592-613）。設定を書く → 有効から無効なら全削除 → 両方成功したときだけ閉じる。削除に失敗したときは無効のまま開いておく。R-11 は `abort_fetch_with_failure` が `cacheSaving` と `cacheNotices` を戻す。
- Tauri の権限。`build.rs` と `capabilities/default.json` に新しい 4 コマンドだけが足されている。

### Summary

Critical・Major の指摘はなく、BR1.1〜BR5.5 と計画の R-10〜R-13 は実装とテストで確認できた。Minor 6 件は、UI スレッドでの I/O、異常終了時の書き込み競合、行の切れ目での切断を見分けられないこと、写し取りの実行場所、複数プロセス、BR3.7 の順序と中断のテスト不足で、いずれも次の作業単位へ回してよい。
