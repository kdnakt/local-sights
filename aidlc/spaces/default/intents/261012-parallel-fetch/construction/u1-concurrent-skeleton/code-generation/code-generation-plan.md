# Code Generation Plan — U1 薄い一本・同時取得（u1-concurrent-skeleton）

## Summary

- Builds: ロググループのストリームを最大 8 本同時に読み、リージョンごとの速さの上限（東京 1 秒 12 回）で GetLogEvents を呼ぶ取得と、取得 1 回の要約の 1 行
- Touches: `crates/local-sights-core/src/`（新規 `pacing.rs`・`schedule.rs`・`fetch_status.rs`・`fetch_stats.rs`、変更 `retry.rs`・`fetcher.rs`・`coordinator.rs`・`gateway/aws.rs` ほか取り込み先）、`tests/`、`examples/fetch_check.rs`、`src-tauri/src/lib.rs`、`Cargo.toml`
- Tests: 単体テスト約 45 件（純粋な部品はテスト先行）と、偽物のゲートウェイと止めた時計による結合テスト約 14 件。既存のテストはすべて通ったまま、行カバレッジ 80% 以上

## 上流と前提

- 上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md BR1.1〜BR1.20・entities.md）、`nfr-requirements/`（NFR1.1〜NFR8.6）、`nfr-design/`（各設計と logical-components.md、レビュー R-01〜R-05）、`infrastructure-design/`（cicd-pipeline.md、Q1）、`inception/units-generation/unit-of-work.md`（U1）・`unit-of-work-story-map.md`（US1.1〜US1.3）、`inception/requirements-analysis/requirements.md`、`inception/contract-design/contract-summary.md`（C1〜C4、C7）
- ブランチ：記録と同じ `claude/parallel-fetch` で作業し、PR #20 がマージされた `main`（a79f2ad）を先に取り込む（infrastructure-design Q1）。PR を出してから CI がすべて通ったら `main` にスクワッシュマージする
- 実際の AWS には自動テストから接続しない。手元の確認（FetchCheck と GUI）は開発者本人が行う（team.md Walking Skeleton）

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
  "scope": "mvp",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Methodology evidence**: 261002-cwlogs-viewer の面談（Q4 の回答 B）で、純粋なロジックはテスト先行、AWS 接続と GUI の層は実装後にテスト、という混在した進め方に決め、261011-local-release の面談でも変えていない。今回（261012-parallel-fetch）の面談でも進め方そのものを変える回答はなく（Q2 はカバレッジの下限を CI で確かめる決まりを加えるだけ）、今のコード（判断は副作用のない関数に寄せ、テストは各モジュールの中にある）と git の履歴にもこれと食い違う跡はない。\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- カバレッジ：行カバレッジを CI で測り、80% を下回る PR はマージしない（例：ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）。これは org.md のスコープ別の下限（`mvp` などの 80% 行カバレッジとマージ前の CI での実行）を CI の合否で確かめるもので、下限を下げることはしない。そのほかのスコープ別の決まりも org.md に従う。\n- GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。\n- Rust 側のテストは `cargo test`（ライブラリとアプリの両方）で、画面側（フロントエンド）のテストは Vitest で実行し、どちらも CI でマージ前に実行する。既存のテストはすべて通った状態を保つ。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Meet an 80% line-coverage floor.",
      "Run the selected tests in CI before merge."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "custom",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:a911d6bb21309d64e6219c4b865c2c2358f027be00fe4d25d16815b9e07a6509",
  "contract_sha256": "sha256:3febdc81499dd0c916a65778f126e962ddb2bd1ee28dd3b52b83b28677c7afbe"
}
```

## 手順

進め方は Testing Contract の custom の順に従う。純粋な部品（速さの計算と許可の予約、読む順とあきらめ、止める理由の強さ、要約の文字列、`Retrier` の試行の前の待ち）はテストを先に書き、失敗を確かめてから実装する。AWS 接続（ゲートウェイ）・取得の流れの組み立て（偽物のゲートウェイを通す結合）・GUI（Tauri アプリ）・確認用プログラムは、実装してからその層のテストを書いて実行する。

### Step 1: 構成と依存（Project structure and production configuration skeleton）

- [x] `origin/main`（PR #20 のマージ後）を `claude/parallel-fetch` にマージする
- [x] ワークスペースの `Cargo.toml` に `futures-util`（`default-features = false`、`features = ["std"]`）を足し、`crates/local-sights-core/Cargo.toml` から使う。理由のコメントを付ける（NFR8.5）
- [x] 空のモジュール `pacing`・`schedule`・`fetch_status`・`fetch_stats` を `lib.rs` に足す
- [x] `cargo build -p local-sights-core` と `cargo deny check` が通ることを確かめる

### Step 2: テストの走らせ方を確かめる（Verify the existing test runner）

- [x] `cargo test -p local-sights-core --lib retry::` が今のまま通ることを確かめ、`unit-test-instructions.md` の単位ごとのコマンドが動くことを確かめる

### Step 3: FetchStatus（JobStatus の移動）— 型の移動

- [x] `JobStatus` を `coordinator.rs` から `fetch_status.rs` に移す。値と serde の形は変えない（BR1.17、C3）
- [x] 取り込み先をすべて直す：`coordinator.rs`・`cache/plan.rs`・`session.rs`・`src-tauri/src/lib.rs`・`examples/fetch_check.rs`・`tests/support/recording_sink.rs`・`tests/u1〜u6` の流れのテスト（取り込みの行だけ、NFR7.1）
- [x] 単体テスト（`fetch_status::tests`）：値の一覧と serde の名前が今と同じ（2 件）

### Step 4: 止める信号（StopReason・StopHandle・StopSignal）— テスト先行

- [x] 先にテストを書く（`retry::tests`）：まだ止めていない → 送った理由になる、Cancel → Close は Close、Close → Cancel は Close のまま、同じ理由の繰り返しで変わらない、`stopped()` は止めると終わる、止める側を捨てても `never()` 相当で終わらない（6 件、BR1.20）。失敗を確かめる
- [x] `retry.rs` の `abort_pair`・`AbortHandle`・`AbortSignal` を、理由つきの `stop_pair`・`StopHandle`・`StopSignal`（`StopReason {Cancel, Close}`、`reason()`・`is_stopped()`・`stopped()`・`never()`）に置き換える。`watch` の `send_if_modified` で強さの決まりを守る
- [x] 使う側を直す：`Retrier`、`wait_or_abort`（`wait_or_stop`）、`coordinator.rs`、`src-tauri/src/lib.rs`（ウィンドウを閉じる・[Close] は `StopReason::Close`）、`examples/fetch_check.rs`、`tests/` の流れのテスト

### Step 5: CallPacer（速さの上限と許可の予約）— テスト先行

- [x] 先にテストを書く（`pacing::tests`、止めた時計 `start_paused`）：リージョンの表（25 → 12、10 → 5 の 7 リージョン、eu-west-3 → 15、表にない → 5、指定なし → 5）（BR1.1、BR1.2）、間隔の切り上げ（12 → 83,333,334 ナノ秒、5 → 200,000,000）（BR1.3）、最初の許可はすぐ出る、8 本が同時に求めても許可の時刻がすべて間隔以上あく、どの 1 秒の区間でも上限以下、許可待ちの間に止めると `Stopped` を返し許可を出さない、許可と止める信号が同時にそろったときは止める側を優先（biased）（BR1.5）。8 件。失敗を確かめる
- [x] `pacing.rs` に `RegionCallLimit`（表）、`CallPacer::for_region(Option<&str>)`、`CallPacer::permit(&StopSignal) -> Permit {Proceed, Stopped}` を作る。`std::sync::Mutex` の中で `slot = max(now, next_permit_at)`、`next_permit_at = slot + interval` を決めてからロックを放し、`tokio::time::sleep_until(slot)` を止める信号と `biased` の `select!` で待つ（NFR3.2、NFR3.3）
- [x] ロックの毒化は中身をそのまま使う（`PoisonError::into_inner`。中身は時刻 1 つで、途中の状態にならないため。NFR 設計のレビュー R-05）
- [x] 文書のコメントに、守るのは許可の時刻の間隔で、実際の呼び出しの時刻はタスクの遅れでずれうること、その余裕は AWS の上限の半分で吸収することを書く（NFR 設計のレビュー R-04）

### Step 6: Retrier の試行の前の待ち — テスト先行

- [x] 先にテストを書く（`retry::tests`）：`call` は最初の試行と再試行のどちらの前にも待ち処理を呼ぶ、待ち処理が `Stopped` なら試行せずに `Aborted`、再試行の待ちと回数は今と同じ（1・2・4・8・16 秒、最大 5 回）、`call_unpaced` は今の `call` と同じ動き（今の 4 件の `call` のテストは `call_unpaced` で同じ結果）、試行と Throttled の数が数えられる（5 件追加）。失敗を確かめる
- [x] `Retrier::call(counter, before_attempt, attempt)` と `call_unpaced(counter, attempt)` を作る。`RetryPolicy` は変えない（NFR1.7）
- [x] `streams/planner.rs`（DescribeLogStreams）は `call_unpaced` にする（速さの上限の対象外）

### Step 7: StreamSchedule（読む順・同時 8 本・あきらめ）— テスト先行

- [x] 先にテストを書く（`schedule::tests`）：`next()` は選ばれた順に返す（BR1.8）、読み途中が 8 本なら返さない（BR1.7）、終わると空きができる、0 本ならすぐ空、使い切りが終わった順に 3 本続くと未着手がすべて Skipped になり最後に使い切った失敗を写す（BR1.13）、成功や使い切り以外の失敗で数が 0 に戻る、Skipped の本数と終わった本数（BR1.19）、読み途中は Skipped にしない。8 件。失敗を確かめる
- [x] `schedule.rs` に `StreamSchedule`（`StreamSlot`・`SlotState`・`SlotEnd`・`ScheduleOutcome`）を作る。今の `ExhaustionStreak` を使う（ADR-007、移動は U3）

### Step 8: FetchStats と要約の 1 行 — テスト先行

- [x] 先にテストを書く（`fetch_stats::tests`）：書式が `local-sights: fetch summary streams=… calls=… throttled=… elapsed_ms=… max_in_flight=… status=…` の順で固定、値は整数と JobStatus の語だけ、`calls`・`throttled` を数える、`max_in_flight` は最大を残す、何も読まなかった取得は 0 が並ぶ。5 件。失敗を確かめる（NFR2.2、NFR2.4、NFR8.3、NFR8.6）
- [x] `fetch_stats.rs` に `FetchStats` を作る。`calls`・`throttled` は試行ごとに増やすため `AtomicU64` で持ち、同じタスクの中の読み取りから共有の参照で数える（NFR 設計のレビュー R-02 の受け渡しの経路）。要約の組み立ての関数は数値と `JobStatus` だけを受け取る
- [x] 要約を出す場面を決めて文書のコメントに書く（R-02）：AWS から取得したすべての取得の終わり（Completed・CompletedWithFailures・Failed・Aborted、0 本を含む）で 1 回。キャッシュから表示した取得では出さない。`streams` は終わった本数（Skipped を含む）、`elapsed_ms` は読む段の始まりから終わりまで（読む段に入らなかったときは 0）

### Step 9: EventFetcher を「次の 1 ページ」に分ける — 実装してからテスト

- [x] `fetcher.rs` の `fetch_stream` を、ストリームごとの読み取りの状態（カーソル・ストリーム内の順番・再試行の数・件数）を持つ `StreamReader` と、`next_page(reader, …) -> (StreamReader, PageStep)` に分ける。`PageStep {Page(events), Finished(StreamFetch)}`。ページの終わりの判定と順番の振り方は今のまま
- [x] 試行の前に `CallPacer::permit` を待ち、試行ごとに `FetchStats` の `calls`、Throttled なら `throttled` を数える（BR1.4、NFR2.4）
- [x] テスト（`fetcher::tests`、偽物のゲートウェイ）：ページが順に返り最後に Finished、空のページは渡さない、失敗で Finished（Failed）、止めると Aborted、試行の前に許可を求める（5 件）

### Step 10: FetchCoordinator の同時に読む段 — 実装してからテスト

- [x] `coordinator.rs` の `run_fetch` の読む段を、取得 1 回のタスクの中で `FuturesUnordered` に最大 8 つの「次の 1 ページ」を並べる形にする（`tokio::spawn` は使わない、functional-spec 2 節）。取得の条件のリージョンから `CallPacer` を 1 つ作る
- [x] 止める信号が来たら未着手も次の 1 ページも始めず、出ている読み取りの終わりを待ち、返ったページは渡さない。すべて止まったら保持先を捨てて Aborted（BR1.16）
- [x] 失敗は失敗の一覧に足して続ける（BR1.14）。終わりの状態は `decide_job_status` で決める（BR1.15）。`failed_streams` と `outcomes` は終わった順（BR1.19）
- [x] 取得の終わりに要約の 1 行を標準エラー出力へ出す（Step 8 の決まり）
- [x] 結合テストは Step 11 で行う

### Step 11: 偽物のゲートウェイを足して結合テスト — 実装の後のテスト

- [x] `tests/support/fake_gateway.rs` に、1 回の呼び出しにかかる時間（`tokio::time::sleep`、止めた時計）、呼んだ時刻の記録、同時に出ている呼び出しの数の最大の記録を足す（今のテストの書き方は変えない）
- [x] `tests/u1_concurrent_fetch.rs` に、functional-spec 5 節の表の結合テストを書く：同時に呼んでいる（10 本・200 ミリ秒、最大 8、AC1.1.1）、速くなる（東京・100 本・200 ミリ秒で 10 秒以下、AC1.1.2）、呼び出しの量（どの 1 秒も 12 回以下、再試行を含む）、選ぶストリームが同じ（AC1.1.3）、最初の 1 件が早い（AC1.2.1、AC1.2.4）、並びが時刻順（AC1.2.2）、一部の失敗（AC1.3.1、AC1.3.2）、許可待ちで止まる（BR1.5）、閉じた後のページ（BR1.16）、ページの順の保証と渡し待ち 8 以下（BR1.10、BR1.11）、0 本（BR1.9）、要約の 1 行の数（NFR2.2）、700 ミリ秒の呼び出しで同時の最大が 8（NFR1.3）。14 件
- [ ] 既存の流れのテスト（`tests/u1〜u6`）が、取り込みの行の変更だけで通ることを確かめる（NFR7.1）

### Step 12: AwsCloudWatchLogsGateway の SDK の再試行 — 実装してからテスト

- [x] GetLogEvents の呼び出しにだけ、操作ごとの設定の上書き（`customize().config_override(...)`）で SDK の最大試行回数を 1（`RetryConfig::standard().with_max_attempts(1)`、再試行なし）にする。DescribeLogGroups・DescribeLogStreams は変えない（BR1.6、NFR3.4、NFR 設計のレビュー R-01）
- [x] 上書きの設定を作る関数を分け、テスト（`gateway::aws::tests`）で最大試行回数が 1 であることを確かめる。AWS には接続しない（2 件）

### Step 13: Tauri アプリ — 実装してからテスト

- [x] `src-tauri/src/lib.rs`：`AbortHandle` を `StopHandle` に、`abort()` を `stop(StopReason::Close)` に直す。`JobStatus` の取り込み先を直す。画面と命令の形は変えない
- [x] `cargo test -p local-sights` と `cargo clippy --workspace --all-targets` が通ることを確かめる

### Step 14: 確認用プログラム（FetchCheck）— 実装してからテスト

- [x] `examples/fetch_check.rs` に、リージョンを必ず指定する `--region` を足し、取得の条件のリージョンにする（CallPacer の上限がそのリージョンで決まる）。要約の 1 行と件数を表示する（NFR2.3）
- [x] 文書のコメントに、手元の比較の手順を書く（NFR2.1、NFR 設計のレビュー R-03）：main の版は `AWS_REGION=ap-northeast-1`、U1 の版は `AWS_REGION=ap-northeast-1` と `--region ap-northeast-1` の両方を付け、同じプロファイル・ロググループ・時間範囲で 3 回ずつ動かし、`time` の実時間の中央値と件数を比べる
- [x] 引数の読み取りのテスト（`--region` がないと使い方の誤り、あると取り込む）（3 件）

### Step 15: ビルドと確かめ（Environment/build configuration）

- [x] `cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo test -p local-sights-core`、`cargo test -p local-sights-core --doc`
- [x] `cargo llvm-cov -p local-sights-core --fail-under-lines 80` が通る。下限は下げない
- [x] 100 万件の速度測定テスト（`cargo test -p local-sights-core --release -- --ignored`）が今と同じ基準で通る（NFR5.1、AC1.2.3）
- [x] `cargo deny check` が通る（NFR8.5）

### Step 16: 文書と対応表（Documentation and traceability）

- [x] README の「Tests and checks」に、FetchCheck の `--region` と要約の 1 行を足す
- [ ] `code-summary.md`・`source-manifest.json`・`traceability.json` を書く

## 対応表（手順とストーリー）

| 手順 | ストーリー・要件 |
|------|------------------|
| Step 5、6、9、10、11、12 | US1.1（AC1.1.1〜AC1.1.3）、FR1.1、FR2.1〜FR2.3、NFR1.1〜NFR1.4、NFR3.1〜NFR3.4 |
| Step 7、10、11 | US1.1、US1.3（AC1.3.1、AC1.3.2）、FR1.4、FR1.5、NFR1.5、NFR1.6、NFR1.8 |
| Step 9、10、11、15 | US1.2（AC1.2.1〜AC1.2.4）、FR1.2、FR1.3、NFR5.1〜NFR5.3 |
| Step 3、4、13 | C3、C7、NFR4.1、NFR7.1 |
| Step 8、10、14 | NFR2.1〜NFR2.4、NFR8.3、NFR8.6 |
| Step 1、12、15 | NFR8.1、NFR8.2、NFR8.4、NFR8.5、NFR6.1 |

## Assumptions & Open Questions

- 仮の決め：既存の流れのテストのうち、`failed_streams` と `outcomes` の並びを「読んだ順」として確かめているものは、偽物のゲートウェイがすぐ答えるため終わった順と同じになる見込み。食い違ったときは、BR1.19（終わった順）に合わせてそのテストの期待だけを直し、`code-summary.md` に書く
