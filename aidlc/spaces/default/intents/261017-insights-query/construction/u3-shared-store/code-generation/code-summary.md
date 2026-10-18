# Code Summary — U3 クエリの画面の取り足しと複数のロググループ（u3-shared-store）

上流：`code-generation-plan.md`（Step 1〜Step 14）、`functional-design/`（`rules.md` の BR1.1〜BR5.3、`functional-spec.md`、`entities.md`）、`unit-test-instructions.md`。Testing Contract は `custom`（純粋なロジックはテスト先行、AWS 接続・GUI・境界の層は実装してからテスト）。コミットと push はしていない。

## 作ったもの・変えたもの

### 中核ライブラリ（`crates/local-sights-core`）

| ファイル | 中身 | 決まり |
|---|---|---|
| `src/cache/plan.rs` | 足りない範囲の計算 `missing_ranges(start, end, covered)` と、重なる部分の計算 `overlap_ranges(wanted, available)` を足した。テストを先に書いた（8 件） | BR1.1、BR2.1 |
| `src/cache/mod.rs` | `CacheOverlap`（Missing / Unreadable / Corrupt / Found）と `LogCache::read_overlaps(key, wanted)` を足した。重なりがあるときだけファイル全体を読み、なければ見出しだけを読む。壊れたファイルは消し、読めないファイルは残す | BR2.1 |
| `src/query/store.rs` | 作り直した。（接続, ロググループ）ごとに committed のイベントと取得済みの範囲、staging のイベント、ストリームごとの順番の振り直し（`StreamSequences`）を持つ。`begin_run`・`missing_ranges`・`add_cached`・`settle_range`（取得を始めた時刻の 5 分前で区切る）・`build_view`（時刻・ロググループ・ストリーム・順番で並べる）・`copy_rows`・`json_field_names` | BR1.1〜BR1.5、BR3.2、BR3.3、BR5.3 |
| `src/query/runner.rs` | 作り直した。ロググループ 1 つずつ、足りない範囲 1 つずつ、キャッシュから読み、残りを今の取得の流れで取得する。ロググループごとの結果（`GroupFetchStatus`）から実行の終わりを決める。認証の失敗のときは残りのロググループを始めず、同じ失敗で Failed にする。評価が終わったら結果の行を写す | BR2.1〜BR2.5、BR3.2 |
| `src/query/evaluator.rs` | 結果の行を `ResultRow { event, log_group_name }` の写しで持つ。`window`・`row_at` は runId だけを確かめる。And / Or を並びで評価する | BR3.2 |
| `src/query/parser.rs`・`plan.rs`・`instant.rs` | 条件の読み取りを再帰から、段を自分で積む繰り返しに書き直した。And / Or は `Vec<Condition>`。括弧の入れ子は `MAX_NESTING = 100` 段までで、超えると `query.syntax.nestingTooDeep` | BR5.1 |
| `src/insights_session.rs` | ロググループを 50 個まで（`MAX_LOG_GROUPS`）。入力の値は `AnalyticsValue`（文字列か名前の並び）。実行を始めるときは前の結果を残して補完の JSON の項目の名前を消す。進み具合にキャッシュを読む段・ロググループの番号と数・失敗したロググループの数を足した。取得の失敗と評価の失敗のときは結果を消す。表示の行は結果の写しから読む | BR1.5、BR2.5〜BR2.7、BR3.1、BR3.4、BR4.1、BR4.2、BR5.2 |
| `src/session.rs` | 新しい入口（キャッシュを読む段、ロググループの始まり、範囲の取得の終わり、取得の終わり）を足し、行の位置を返す関数をやめた。接続を変えるとクエリの画面の保持と選んだロググループを捨てる | BR1.6、BR2.4 |
| `src/timeline.rs` | `take_events`（取得したイベントを取り出す） | BR1.2 |
| `src/time_range.rs` | `TimeRange::from_instants`（開始と終了が同じでもよい） | BR1.1 |
| `examples/fetch_check.rs` | 新しい `QueryRun` と `QuerySink` に合わせた | — |

### アプリ（`src-tauri/src/lib.rs`）

`update_analytics_input` が `AnalyticsValue` を受け取る。`spawn_query` が新しい `QueryRun`（時計は `now_epoch_ms`）を作る。`query_rows`・`query_row_detail` はセッションの写しだけを読む。`TauriQuerySink` に新しい通知を足した。

### 画面（`src/`）

| ファイル | 中身 | 決まり |
|---|---|---|
| `components/LogGroupMultiselect.tsx` | 複数選択に作り直した。50 個選ぶと残りの選択肢を押せなくし、上限の文言（`log-group-limit`）を出す | BR4.1 |
| `components/LogAnalyticsPage.tsx` | 進み具合（キャッシュを読む・準備・一覧・取得、ロググループの i/n、失敗したロググループの数）。次の実行の取得と評価の間も前の結果を表示する。取得の失敗にロググループを並べる | BR2.6、BR2.7、BR3.4 |
| `components/QueryResultTable.tsx`・`styles.css` | `previous` のときに「前の結果」の見出しを出す | BR3.4 |
| `components/FailureList.tsx` | 失敗したストリームを「ロググループ / ストリーム」で示す | BR2.6 |
| `api.ts`・`analytics.ts`・`App.tsx` | 型と `limitSelection`、`updateAnalyticsInput` が名前の並びを受け取る | BR4.1 |
| `i18n/messages.ts` | 英語と日本語の文言を足した・変えた（ロググループの欄と上限、選ばれていないとき、進み具合、前の結果、失敗したロググループ、`query.syntax.nestingTooDeep`） | NFR9 |
| `test/fixtures.ts` | テストの道具を新しい型に合わせた | — |

## テストの数

作業の前は、中核ライブラリ 579 件（無視 8 件）、アプリ 30 件、画面 458 件で、すべて通っていた。

| 部品 | テストの数（作業の後） | 主に足したもの |
|---|---|---|
| `cache`（`mod.rs`） | 18 | `read_overlaps` 6 件 |
| `cache::plan` | 23 | 足りない範囲・重なり 8 件（テスト先行） |
| `query::store` | 15（＋無視 1 件の `memory`） | 順番の振り直し 5 件（テスト先行）、保持と範囲の記録 約 10 件 |
| `query::runner` | 17 | FR2 の受け入れ条件 ac1〜ac6、キャッシュの重なり、失敗したロググループ、認証、中止、新しい 5 分、写しが残ること、評価の中止・パニック、項目の名前、JSON の並べ替え |
| `query::parser` | 27 | 1 MB のスタックでの長い or / and の並び、100 段は通り 101 段目で失敗（場所は括弧）、同時に開いた括弧だけを数える、and が or より強い |
| `query::evaluator` | 33（＋速さ 3 件、無視） | 写しの行に合わせて直した |
| `insights_session` | 24 | 50 / 51 個、1〜50 個で実行できる、終わりの名前、進み具合、接続を変える、前の結果、失敗で結果を消す |
| `session` | 134 | 複数のロググループと接続を変えたときに捨てること |
| `timeline`・`time_range` | 11・21 | 各 1 件 |
| 中核ライブラリの合計 | 622（無視 9 件） | |
| アプリ（`src-tauri`） | 31 | ロググループは並び、ほかは文字列で届く |
| 画面（Vitest） | 41 ファイル・469 件 | LogGroupMultiselect 9 件、LogAnalyticsPage の U3 の 7 件、QueryResultTable 1 件、messages 1 件（キーは 42 個） |

## 確かめたこと（Step 12・Step 13）

| コマンド | 結果 |
|---|---|
| `cargo fmt --check` | 通った |
| `cargo clippy --workspace --all-targets` | 警告 0 |
| `cargo test --workspace` | 中核ライブラリ 622、アプリ 31、結合テストは今のまま通った |
| `npx eslint .`・`npx tsc --noEmit`・`npx prettier --check` | 通った |
| `npx vitest run` | 41 ファイル・469 件が通った |
| `npm run build` | 通った |
| `cargo llvm-cov`（Linux） | 中核ライブラリの行カバレッジ 96.60%（`query/store.rs` は 85.47%） |
| `npx vitest run --coverage` | 画面の行カバレッジ 89.88% |

どちらも 80% の下限を上回る。下限も速さのテストも下げていない。

### ログ管理の画面の速さ（今と同じであることの確認、release ビルド）

10 万件の絞り込み 3.4 ms、100 万件の絞り込み 30.8 ms、ページの追加 110 ms。今の基準のテストがそのまま通った。

### クエリの速さ（100 万件、2 回目の実行）

| クエリ | U2 | U3 |
|---|---|---|
| 絞り込みだけ | 252.6 ms | 253.5 ms |
| JSON の項目で絞り込み | 731 ms | 660 ms |
| JSON の項目で並べ替え | 1.61 s | 1.56 s |
| fields＋sort＋limit | 1.12 s | 1.13 s |

U2 より遅くなっていない（差は測りのばらつきの中）。

### メモリ（BR5.3、NFR2）

`query::store::memory::both_screens_full_and_a_previous_result_evaluate_a_json_sort`（無視つき）を、それだけで release ビルドで動かした。Linux の `VmHWM`（ピークの常駐メモリ）を読んでいる。

| 時点 | ピークの常駐メモリ |
|---|---|
| 始め | 8.0 MiB |
| ログ管理の画面に 100 万件 | 483.3 MiB |
| クエリの画面にも 100 万件 | 522.4 MiB |
| 10 万行の前の結果を持ったあと | 664.9 MiB |
| JSON の並べ替えのクエリのあと | 666.2 MiB |

50 個のロググループの範囲の記録 460 ms、評価の並びを作る 62 ms。10 万行の前の結果 2.40 s、JSON の並べ替えのクエリ 2.47 s（いちばん長い 1 歩 1.20 s）。2 回動かして同じ値だった。ピークにはテストが作るイベントの一時的な分も入っている。落ちずに結果が出た。

## U2 の記録の直し（U2 の R-05）

U2 の `code-summary.md` の「権限ファイル……作業ツリーにあり、コミットはしていない」は誤り。`src-tauri/permissions/autogenerated/` の 2 つの権限ファイルはコミット `cd3a346` に入っている。

## 手元で確かめること（自動テストでは確かめられないもの）

- **本物の Ace（U2 の R-04）**：テストは Ace を偽物に差し替えている。開発者の手元で、GUI を起動して、本物の Ace でクエリの色分け・補完の候補・エラーの下線と行の横の印が出ることを目で確かめる。
- **薄い一本の確認（team.md の Walking Skeleton の 2 と 3）**：共通のコードを触ったため、macOS（開発に使う Mac）と Windows 11（x64、CI で作ったインストーラで入れたアプリ）の両方で行う。
  - 本人の AWS 認証情報で、複数のロググループを選んで実行し、結果が出ること。
  - 同じ期間で 2 回目を実行すると AWS から取得しないこと、期間を広げると足りない部分だけを取得すること。
  - ディスクキャッシュがある期間は、キャッシュから読むこと。
  - 取得と評価の間は前の結果が表示されていること、接続を変えると選択と保持が捨てられること。
  - ログ管理の画面の振る舞いが前と同じであること。
- **macOS のメモリ**：メモリの測りは Linux だけで行った。macOS で測るときは `/usr/bin/time -l` でピークを見る。

## 計画や設計との違い

- **条件の読み取りの書き直し**：100 段の括弧の入れ子は、debug ビルドの 1 MB のスタックで再帰の読み取りがあふれた。そのため、条件の読み取りを、段を自分で積む繰り返しに書き直した（計画は入れ子の深さを数えて止めるだけだった）。
- **計画の一覧にないファイル**：`time_range.rs`（`TimeRange::from_instants`）、`examples/fetch_check.rs`、`FailureList.tsx`、`App.tsx`、`styles.css`、`test/fixtures.ts` を変えた。型の変更に合わせるためと、失敗したストリームにロググループ名を添えるため。
- **`QueryRunSummary.closed`**：`finish_fetch` の引数を 3 つのままにするため、アプリを閉じたかどうかを要約に持たせた。
- **`holdings_epoch`**：API に残したが、今は返すだけで、行を読むときの確かめには使っていない（確かめは runId、BR3.2）。
- **51 個目のロググループ**：画面では選べないが、届いたときは `SessionError::Invalid` で断る。
- **認証の失敗で止めたロググループ**：始めなかったロググループも、同じ失敗で Failed として示す。
- **何も足りないものがないとき**：それでも取得の段（Fetching）に入り、進み具合は「足りない部分を探している」の文言になる。すぐに評価に進む。
- **メモリの測り**：Linux だけで測った（上の「手元で確かめること」）。

## 残っていること

- 上の「手元で確かめること」（本物の Ace の目視、macOS と Windows での薄い一本の確認）。
- 自動では確かめていないコンソールとの互換は U4 で行う。
