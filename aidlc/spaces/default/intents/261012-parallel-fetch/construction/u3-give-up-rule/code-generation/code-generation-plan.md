# Code Generation Plan — U3 あきらめの決まり（u3-give-up-rule）

## Summary

- Builds: 使い切りの数え・あきらめる本数・終わり方の分け方を StreamSchedule に移し（ADR-007）、あきらめの記録 `GiveUp` を結果に載せ、あきらめた後は数えを動かさない形にする
- Touches: `crates/local-sights-core/src/schedule.rs`（数え・GiveUp）、`retry.rs`（数えの型と項目をなくす）、`fetcher.rs`（`StreamFetchEnd::Failed` に失敗の内容）、`coordinator.rs`（変換をなくす）、`tests/u3_give_up_rule.rs`（新規）
- Tests: StreamSchedule の単体テスト約 8 件の追加（テスト先行）と retry.rs から移す 2 件、偽物のゲートウェイによる結合テスト約 5 件。既存の結合テストは変えずに通し、行カバレッジ 80% 以上

## 上流と前提

- 上流：`construction/u3-give-up-rule/functional-design/`（rules.md BR3.1〜BR3.7、functional-spec.md）、`nfr-requirements/`（NFR3.7〜NFR8.9）、`nfr-design/`（security-design.md の設計と「決まりの正本」・「確かめること」、logical-components.md、レビュー 2 回目の R-05）、`infrastructure-design/cicd-pipeline.md`、`inception/user-stories/stories.md`（US2.3：AC2.3.1〜AC2.3.4）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）
- 機能設計とのずれは nfr-design/security-design.md の「決まりの正本」を正とする（例：あきらめの時点の未着手は s14 以降、`StreamEnding` という型は作らない）
- ブランチ：U1・U2 と同じ `claude/parallel-fetch`。実際の AWS には自動テストから接続しない

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

純粋な部品（StreamSchedule の数えとあきらめ）はテストを先に書き、失敗を確かめてから実装する。取得の流れ（FetchCoordinator・EventFetcher）へのつなぎ込みは実装してから、偽物のゲートウェイを通す結合テストを書く。

### Step 1: 構成（Project structure and production configuration skeleton）

- [x] 新しい依存・モジュールは足さない。作業ツリーがきれいで `claude/parallel-fetch` が `origin/claude/parallel-fetch` と同じであることを確かめる

### Step 2: テストの走らせ方を確かめる（Verify the existing test runner）

- [x] `cargo test -p local-sights-core --lib schedule::` が今のまま通ることを確かめる

### Step 3: StreamSchedule の数えとあきらめ — テスト先行

- [x] 先にテストを書く（`schedule::tests`）：成功で数え直す（使い切り 2 → 成功 → 使い切りで 1、AC2.3.1）、3 本で未着手を Skipped にし 3 本目の失敗を写す・選ばれた順・`give_up` に名前と失敗と本数（AC2.3.2）、あきらめた後に読み途中が終わっても数えが動かず `give_up` は 1 回（AC2.3.3、BR3.3）、使い切り以外の失敗で 0（AC2.3.4）、閉じて止めたもの（Aborted）は数えない（BR3.5）、未着手 0 本でも `give_up` が残り skipped_count 0、`with_limit(…, 0)` は 1 として扱う、`GIVE_UP_LIMIT` は 3。retry.rs の数えのテスト 2 件も schedule.rs に移す。失敗を確かめる
- [x] schedule.rs に非公開の数え（今の `ExhaustionStreak` を移す）、公開の定数 `GIVE_UP_LIMIT`、`StreamSchedule::new(names)` と `with_limit(names, limit)`、`finish(index, end: StreamFetchEnd)`、公開の構造体 `GiveUp { triggered_by, failure, skipped_count }` と `ScheduleOutcome.give_up: Option<GiveUp>`（`ScheduleOutcome` は Clone）を作る。`SlotEnd` が数えのための分け方を持つ（`StreamEnding` という型は作らない）。あきらめた後は数えを動かさない
- [x] 既存の schedule.rs のテストは新しい `finish`・`new` に合わせて呼び出しを直す（確かめる中身は弱めない）

### Step 4: EventFetcher と RetryPolicy の整理 — 実装してからテスト

- [x] fetcher.rs の `StreamFetchEnd::Failed` を `Failed { exhausted: bool, failure: ApiFailure }` にし（`Copy` をやめて `Clone`）、`CallResult::Failed { failure, exhausted }` の値をそのまま入れる。`streak_ending` をなくす
- [x] retry.rs から `StreamEnding`・`ExhaustionStreak`・`RetryPolicy::max_consecutive_exhausted` をなくす
- [x] fetcher.rs のテストで `StreamFetchEnd::Failed { exhausted: true }`・`{ exhausted: false }` と比べているものは、`matches!` で exhausted と失敗の種類を確かめる形にする（NFR 設計のレビュー R-05）

### Step 5: FetchCoordinator のつなぎ込み — 実装してからテスト

- [x] coordinator.rs で `StreamSchedule::new(names)` を使い、`StreamFetchEnd` から `SlotEnd` への変換をなくして `finish` にそのまま渡す。FetchJob への記録の仕方（終わった順、Skipped も 1 本ずつ）は変えない（BR3.7）

### Step 6: 結合テスト — 実装の後のテスト

- [x] `tests/u3_give_up_rule.rs` を新しく作り、偽物のゲートウェイと止めた時計で確かめる：終わる順を入れ替えた台本（読み始めと違う順に終わる）で、終わった順に数えてあきらめる（AC2.3.1、AC2.3.2）、あきらめた後は未着手の GetLogEvents が 0 回で、Skipped の失敗は 3 本目のもの（NFR3.7、NFR8.9）、読み途中は最後まで読み結果が残る（AC2.3.3）、使い切りの間の権限エラーで数え直す（AC2.3.4）、閉じたときは Aborted のまま（BR3.5）。5 件
- [x] 既存の結合テスト（u1〜u6、u1_concurrent_fetch、u2_call_pacing）が変えずに通ることを確かめる

### Step 7: ビルドと確かめ（Environment/build configuration）

- [x] `cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo test -p local-sights-core`、`cargo test -p local-sights-core --doc`、`cargo test -p local-sights-core --example fetch_check`
- [x] `cargo llvm-cov -p local-sights-core --fail-under-lines 80` が通る。下限は下げない
- [x] `cargo clippy --workspace --all-targets` と `cargo test -p local-sights` が通る（アプリと examples が変わらずビルドできること）

### Step 8: 文書と対応表（Documentation and traceability）

- [x] 変えたモジュールの冒頭の文書（schedule.rs・retry.rs・fetcher.rs）を新しい持ち主に合わせる
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json` を書く

## 対応表（手順とストーリー）

| 手順 | ストーリー・要件 |
|------|------------------|
| Step 3、6 | US2.3（AC2.3.1〜AC2.3.4）、FR3.1〜FR3.4、BR3.1〜BR3.5 |
| Step 3、4、5 | ADR-007、C2、BR3.6、BR3.7、NFR3.7、NFR8.9 |
| Step 7 | NFR6.1、NFR7.1、NFR7.2、NFR8.1 |

## Assumptions & Open Questions

None.
