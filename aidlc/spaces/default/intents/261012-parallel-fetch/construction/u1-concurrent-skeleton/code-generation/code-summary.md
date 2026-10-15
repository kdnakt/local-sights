# Code Summary — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`code-generation-plan.md`（承認済み）、`unit-test-instructions.md`、`functional-design/`・`nfr-requirements/`・`nfr-design/`・`infrastructure-design/`。

## 作ったもの・変えたもの

| ファイル | 種類 | 内容 |
|----------|------|------|
| `crates/local-sights-core/src/pacing.rs` | 新規 | CallPacer。リージョンの表（BR1.1、BR1.2）、間隔の切り上げ（BR1.3）、ロックの中で枠を予約してから待つ許可、止める信号を `biased` で優先（BR1.5） |
| `crates/local-sights-core/src/schedule.rs` | 新規 | StreamSchedule。選ばれた順に最大 8 本（BR1.7、BR1.8）、終わった順のあきらめと Skipped（BR1.13、BR1.19） |
| `crates/local-sights-core/src/fetch_status.rs` | 新規 | JobStatus の置き場所（値と serde の形は変えない、C3） |
| `crates/local-sights-core/src/fetch_stats.rs` | 新規 | FetchStats と要約の 1 行（NFR2.2、NFR2.4、NFR8.3、NFR8.6） |
| `crates/local-sights-core/src/retry.rs` | 変更 | 理由つきの `stop_pair`・`StopHandle`・`StopSignal`（`StopReason {Cancel, Close}`、BR1.20）。`Retrier::call(counter, before_attempt, attempt)` と `call_unpaced`。`RetryCounter` に試行数と Throttled 数 |
| `crates/local-sights-core/src/fetcher.rs` | 変更 | `fetch_stream` をなくし、`StreamReader`・`ReadContext`・`next_page` と `PageStep {Page, Finished}` に分けた。試行の前に許可を待つ（BR1.4） |
| `crates/local-sights-core/src/coordinator.rs` | 変更 | 読む段を取得 1 回のタスクの中の `FuturesUnordered`（最大 8）にした。止めた後のページは渡さない（BR1.16）。要約の 1 行を標準エラー出力へ。`FetchSink::on_summary`（既定は何もしない）を足した |
| `crates/local-sights-core/src/gateway/aws.rs` | 変更 | GetLogEvents だけ操作ごとの設定の上書きで SDK の最大試行回数 1（BR1.6、R-01） |
| `crates/local-sights-core/src/streams/planner.rs` | 変更 | DescribeLogStreams は `call_unpaced` |
| `crates/local-sights-core/src/cache/plan.rs`・`session.rs`・`lib.rs` | 変更 | JobStatus の取り込み先、新しいモジュールの宣言 |
| `crates/local-sights-core/examples/fetch_check.rs` | 変更 | 必須の `--region`（SDK のリージョンにも使う）、要約の 1 行、比べる手順のコメント（NFR2.1、NFR2.3、R-03） |
| `crates/local-sights-core/tests/u1_concurrent_fetch.rs` | 新規 | 同時取得の結合テスト 14 件 |
| `crates/local-sights-core/tests/support/fake_gateway.rs`・`recording_sink.rs` | 変更 | 呼び出しの時間・時刻・同時の最大の記録、要約の受け取り |
| `crates/local-sights-core/tests/u1_fetch_flow.rs`・`u3_fetch_flow.rs`・`u5_filter_flow.rs`・`u6_cache_flow.rs` | 変更 | 取り込みの変更。u3 は下の「計画からのずれ」のとおり |
| `src-tauri/src/lib.rs` | 変更 | `StopHandle` と `StopReason::Close`、JobStatus の取り込み先 |
| `Cargo.toml`・`crates/local-sights-core/Cargo.toml`・`Cargo.lock` | 変更 | `futures-util`（`default-features = false`、`std` だけ、NFR8.5） |
| `README.md` | 変更 | FetchCheck の `--region`、要約の 1 行、比べる手順 |

## 主な実装の判断

- リージョンの表：既定 10 回・30 回のリージョンは個別に持つ。組み込みのリージョン一覧（`STANDARD_REGIONS`）にあるそれ以外は既定 25 回として 12、一覧にないリージョンと指定なしは 5（BR1.1、BR1.2、entities.md の「その他の公開リージョン」の扱い）
- 許可：守るのは許可の時刻の間隔で、実際の呼び出しの時刻はタスクの遅れでずれうる。その余裕は AWS の上限の半分で吸収する、と文書のコメントに書いた（R-04）。ロックの毒化は中身をそのまま使う（R-05）。tokio のタイマーはミリ秒単位で早くは起きないため、テストは予約した枠の時刻と、整数ミリ秒の間隔で確かめる。止める側の優先が `biased` に頼っていることは、外すと 13 回中 7 回失敗することで確かめた
- 要約の数（R-02）：GetLogEvents の試行ごとに共有の Atomic を増やす。要約は AWS から取得したすべての取得の終わり（0 本・Failed・Aborted を含む）で 1 回、キャッシュから表示した取得では出さない。`streams` は終わった本数（Skipped を含む）、`elapsed_ms` は読む段の始まりから終わりまで
- 1 ページずつの読み取り：イベントのあるページが最後のページでもあるときは、次の読み取りで呼び出しをせずに Finished を返す
- SDK の再試行（R-01）：上書きの値が `max_attempts == 1` であることに加え、要求を数えて必ず失敗するプロセス内の接続で、GetLogEvents は 1 回、DescribeLogStreams は 3 回（既定の再試行）になることを確かめた。AWS には接続しない

## テスト

| 対象 | 件数 |
|------|------|
| `fetch_status` | 2 |
| `retry`（止める信号 6、試行の前の待ち 5） | 11 |
| `pacing` | 8 |
| `schedule` | 8 |
| `fetch_stats` | 5 |
| `fetcher` | 5 |
| `gateway::aws` | 2 |
| `tests/u1_concurrent_fetch.rs` | 14 |
| `fetch_check` の引数 | 3 |

ライブラリの単体テストは 362 件から 403 件（ほかに ignored 5 件）、結合テストは 42 件から 56 件。

| 確かめ | 結果 |
|--------|------|
| `cargo fmt --all --check` | 合格 |
| `cargo clippy -p local-sights-core --all-targets`・`cargo clippy --workspace --all-targets` | 警告 0 |
| `cargo test -p local-sights-core` | すべて合格 |
| `cargo test -p local-sights`（Tauri アプリ） | 12 件合格 |
| `cargo test -p local-sights-core --example fetch_check` | 3 件合格 |
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 合格。行カバレッジ 97.47%（pacing 100%、fetch_stats 100%、fetch_status 100%、schedule 98.52%、retry 98.28%、coordinator 98.07%、gateway/aws 95.40%、fetcher 89.50%） |
| `cargo test -p local-sights-core --release -- --ignored` | 5 件合格（100 万件の速度測定を含む、NFR5.1、AC1.2.3） |
| `cargo deny check` | advisories・bans・licenses・sources すべて ok |

画面側（TypeScript）は変えていない。README だけ `prettier --check` を通した。

## 計画からのずれ

1. **既存の `tests/u3_fetch_flow.rs` の 5 件を、取り込みの行より広く直した。** どれも U3:BR3.1（1 本ずつ順に読む）を前提にしていて、U1 の BR1.7・BR1.8・BR1.19 と両立しない。このため計画 Step 11 の「取り込みの変更だけで通る」（NFR7.1）は満たせず、その項目は未チェックのまま残した。計画の「仮の決め」が想定した範囲（outcomes が終わった順）を超えるものを含む
   - `outcomes[0]` を読む 2 件：終わった順に合わせた
   - 呼び出しの順を確かめる 1 件：同時に読む順に合わせて期待を直した
   - 中止のテスト 1 件：a と同時に読まれる b に台本を足し、呼び出しの記録を厳密に確かめる形にした
   - 「使い切り 3 本続くと残りを失敗」1 件：8 本同時だと 8 本を超えないとあきらめが見えないため 11 本の構成にした。最後のストリームが一度も呼ばれないこと、あきらめを起こしたストリームの失敗を写すことは今までどおり確かめる
   - u1・u2・u5・u6 は取り込みの変更（`abort_pair` → `stop_pair` の名前の変更を含む）だけで通った
2. **`FetchSink` に `on_summary` を足した。** 既定は何もしないので既存の受け口は変わらない。`DeferredFinishSink` も引き渡す。結合テストで要約の数を確かめるため。標準エラー出力の 1 行は coordinator が書く
3. **`StreamSchedule::next` を `start_next` にした。** clippy が `Iterator::next` と紛らわしいと警告するため
4. **`origin/main` のマージ（`bb18418`）で AI-DLC の記録が衝突した。** `aidlc/spaces/default/codekb/local-sights/` は、この intent の設計の前提にしたこのブランチの走査（同じコミット 9d08cbf を走査したもの）を残した。`intents.json` は両方の行を残した

## 残ったこと

- 既存の `u1_fetch_flow` のテストは止めた時計を使っていないため、指定なしの上限（1 秒 5 回）で実時間が約 0.6 秒かかるようになった（合格はしている）
- 手元の確認（実際の AWS での FetchCheck と比較、GUI での表示）は開発者本人が行う（team.md Walking Skeleton）

## Assumptions & Open Questions

None.
