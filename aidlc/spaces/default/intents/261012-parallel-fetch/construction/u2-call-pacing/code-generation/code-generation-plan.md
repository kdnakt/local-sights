# Code Generation Plan — U2 呼び出しの速さ（u2-call-pacing）

## Summary

- Builds: 制限（スロットリング）を受けたら GetLogEvents の速さを半分に下げ、制限なしが 5 秒続くごとに 1 秒あたり 1 回ずつ上限まで戻す CallPacer と、要約の 1 行の `min_rate=`
- Touches: `crates/local-sights-core/src/pacing.rs`（下げ・戻し）、`fetcher.rs`（制限の知らせ）、`fetch_stats.rs`・`coordinator.rs`（`min_rate`）、`tests/u2_call_pacing.rs`（新規）
- Tests: CallPacer の単体テスト約 10 件（テスト先行）、FetchStats の書式のテストの更新、偽物のゲートウェイと止めた時計による結合テスト約 6 件。既存のテストは通ったまま、行カバレッジ 80% 以上

## 上流と前提

- 上流：`construction/u2-call-pacing/functional-design/`（rules.md BR2.1〜BR2.8、functional-spec.md の擬似コードと 4 章、レビュー 2 回目の R-06・R-07）、`nfr-requirements/`（NFR3.5〜NFR8.8）、`nfr-design/`（security-design.md の「確かめること」、logical-components.md、レビューの R-01）、`infrastructure-design/cicd-pipeline.md`（レビューの R-01〜R-03）、`inception/user-stories/stories.md`（US2.2：AC2.2.1〜AC2.2.6）、U1 のコード（`pacing.rs`・`fetcher.rs`・`coordinator.rs`・`fetch_stats.rs`）
- ブランチ：U1 と同じ `claude/parallel-fetch`。実際の AWS には自動テストから接続しない
- 機能設計のレビュー 2 回目の R-06（BR2.6 の古い欄の名前）は holdDecreaseUntil を正として実装する。R-07 のテストは「速さ 3 で枠が 2.7 秒先まで予約されているとき、1 に下げる」の形で書く

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

純粋な部品（CallPacer の下げ方・戻し方、要約の書式）はテストを先に書き、失敗を確かめてから実装する。取得の流れへのつなぎ込み（EventFetcher・FetchCoordinator）は実装してから、偽物のゲートウェイを通す結合テストを書く。

### Step 1: 構成（Project structure and production configuration skeleton）

- [x] 新しい依存・モジュールは足さない。`claude/parallel-fetch` が `origin/claude/parallel-fetch` と同じであることを確かめる

### Step 2: テストの走らせ方を確かめる（Verify the existing test runner）

- [x] `cargo test -p local-sights-core --lib pacing::` と `cargo test -p local-sights-core --lib fetch_stats::` が今のまま通ることを確かめる

### Step 3: CallPacer の下げ方・戻し方 — テスト先行

- [x] 先にテストを書く（`pacing::tests`、止めた時計）：半分に下げる（12 → 6 → 3 → 1 → 1、知らせを下げてよい時刻の後にあける）、まとめて届いた知らせは 1 回分、予約済みの枠の後から届いた知らせ（速さ 3 で枠が 2.7 秒先まで予約されているとき、1.5 秒・2.5 秒の知らせでは下げず、予約済みの枠が出終わった後の知らせで 1 に）、戻す（知らせなしで 5・10・45・60 秒後に 4・5・12・12）、端数の持ち越し（7 秒後に 4、10 秒後に 5）、戻しの数え直し（知らせのたびに数え始めが変わる）、知らせの前にたまった戻し（12 秒後の知らせで 5 にしてから 2）、下げた後に予約する枠の間隔は予約したときの速さの間隔以上で予約済みの枠は動かない、取得ごとに上限から始まる、最も低い速さ。10 件。失敗を確かめる（BR2.2〜BR2.6、NFR3.5、NFR3.6）
- [x] `pacing.rs` の PacingState をロックの中の 1 つの構造体にし（今の速さ・nextPermitAt・holdDecreaseUntil・quietSince・最も低い速さ）、`report_throttled()`・`current_rate_per_second()`・`lowest_rate_per_second()` を足す。`reserve_slot` と `report_throttled` の先頭で戻しを当てはめる（functional-spec.md 1 章の擬似コード）。`min_interval()` は今の速さの間隔を返す
- [x] 冒頭の文書を直す：U1 のコード生成のレビュー R-01（tokio の時計の切り上げで隣どうしの起きる時刻の間は縮みうるが、許可の時刻の間隔と 1 秒の上限は保たれる）と、ロックが守るものが 1 つの時刻ではなくなったことに合わせた毒化の説明（値は先に計算してまとめて書くので、どれかの操作の前か後の値になる）

### Step 4: 要約の 1 行の `min_rate` — テスト先行

- [x] 先にテストを書き直す（`fetch_stats::tests`）：欄の順が streams・calls・throttled・elapsed_ms・max_in_flight・min_rate・status、値は整数と JobStatus の語だけ、何も読まなかった取得の行（U1 の書式のテストを U2 の書式に直す、NFR7.1）。失敗を確かめる
- [x] `FetchStats::summary` に最も低い速さの引数を足し、`FetchSummary` に `min_rate` の欄を足す（BR2.7、NFR8.7）

### Step 5: 取得の流れへのつなぎ込み — 実装してからテスト

- [x] `fetcher.rs` の `counted_call` で、GetLogEvents の試行が Throttled のときに `context.pacer.report_throttled()` を呼ぶ（BR2.1）。DescribeLogStreams（`call_unpaced`）は変えない
- [x] `coordinator.rs` の `report_summary` に最も低い速さを渡す。CallPacer を作った後の終わりは `pacer.lowest_rate_per_second()`、一覧の途中で止めた終わり（CallPacer を作る前）は `RegionCallLimit::for_region(fetch.request.region()).ceiling_per_second()`（BR2.7）
- [x] `fetcher::tests` に、Throttled の試行で速さが下がり、DescribeLogStreams の制限では下がらないテストを足す（2 件）

### Step 6: 結合テスト — 実装の後のテスト

- [x] `tests/u2_call_pacing.rs` を新しく作り、偽物のゲートウェイと止めた時計で次を確かめる：制限が続くと GetLogEvents の呼んだ時刻の間隔が広がり、どの 1 秒も上限以下（AC2.2.1、AC2.2.3）、制限が止むと間隔が上限の間隔まで戻る（AC2.2.3）、下限まで下げた状態で閉じると許可待ちがすぐ止まり新しい呼び出しを始めない（U1 の NFR4.1）、再試行の待ちは今と同じ 1・2 秒（AC2.2.4）、制限なしなら速さは上限のまま（U1 の NFR1.1）、要約の 1 行の `min_rate`（下げた取得と一覧の途中で止めた取得）（BR2.7）。6 件
- [x] 既存の結合テスト（`tests/u1_concurrent_fetch.rs` ほか）が変えずに通ることを確かめる

### Step 7: ビルドと確かめ（Environment/build configuration）

- [x] `cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo test -p local-sights-core`、`cargo test -p local-sights-core --doc`（CI と同じ、インフラ設計のレビュー R-01）
- [x] `cargo llvm-cov -p local-sights-core --fail-under-lines 80` が通る。下限は下げない
- [x] `cargo clippy --workspace --all-targets` と `cargo test -p local-sights` が通る（アプリ側は変えないことの確かめ）

### Step 8: 文書と対応表（Documentation and traceability）

- [x] README の要約の 1 行の説明に `min_rate` を足す
- [ ] `code-summary.md`・`source-manifest.json`・`traceability.json` を書く

## 対応表（手順とストーリー）

| 手順 | ストーリー・要件 |
|------|------------------|
| Step 3、5、6 | US2.2（AC2.2.1、AC2.2.3）、FR2.4、BR2.1〜BR2.6、NFR3.5、NFR3.6 |
| Step 4、5、6 | US2.2（AC2.2.6）、FR2.6、BR2.7、NFR8.7、NFR8.8 |
| Step 6 | AC2.2.4（FR2.5、BR2.8）、U1 の NFR4.1・NFR1.1 の引き継ぎ |
| Step 7 | NFR6.1、NFR7.1、NFR7.2、NFR8.1 |

AC2.2.2・AC2.2.5 は U1 で満たし、U2 では変えない。

## Assumptions & Open Questions

None.
