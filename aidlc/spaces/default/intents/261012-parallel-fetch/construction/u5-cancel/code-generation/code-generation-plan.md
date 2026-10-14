# Code Generation Plan — U5 中断（u5-cancel）

## Summary

- Builds: 取得中の［取得］を同じ場所で［中断］に変え、押すと出ている呼び出しを待たずに打ち切って止める。取れたログは残し、読み終えていない本数 N（または一覧の前）を注意で知らせ、中断した取得はキャッシュに書かない（FR5、ADR-004、ADR-009）
- Touches: `crates/local-sights-core/src/`（fetch_status.rs、coordinator.rs、session.rs、テストの偽物と RecordingSink）、`examples/fetch_check.rs`、`src-tauri/`（lib.rs、build.rs、capabilities/default.json）、`src/`（api.ts、ConditionArea.tsx、CancelNotice.tsx 新規、App.tsx、StatusLine.tsx、i18n/messages.ts、test/fixtures.ts）
- Tests: 純粋な判断（decide_job_status の表、AppSession の押せるかの判定と状態の変わり方）はテスト先行で約 15 件。取得の流れの打ち切りの結合テスト約 8 件、Tauri のコマンドのテスト、画面のテスト約 12 件は実装の後。閉じるときの今のテストは変えずに通し、行カバレッジはライブラリ・画面とも 80% 以上

## 上流と前提

- 上流：`construction/u5-cancel/functional-design/`（rules.md BR5.1〜BR5.14、functional-spec.md）、`nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1：中断したら出ている呼び出しを待たずに打ち切る）、`nfr-design/`（security-design.md の設計の正本、レビュー 2 回目の新しい指摘 3 件）、`infrastructure-design/`（cicd-pipeline.md、レビューの指摘 2 件）、`inception/user-stories/stories.md`（US3.1、US4.1、US4.2）、`inception/domain-design/decisions.md`（ADR-004、ADR-006、ADR-009）、`inception/contract-design/contract-summary.md`（C3〜C7）、`inception/refined-mockups/`（文言の一覧）
- 機能設計と食い違うところは、非機能要件と nfr-design/security-design.md を正とする（応答を待たずに打ち切る、止めた後に届く応答はない、StreamSchedule は変えない）
- ブランチ：U1〜U4 と同じ `claude/parallel-fetch`。実際の AWS には自動テストから接続しない
- 後の段のレビューの軽微な指摘は、この計画で次のように片付ける
  - **非機能設計 R-07（偽物の拡張）**：`tests/support/fake_gateway.rs` の DescribeLogStreams に、遅らせる・返さない台本を足す（一覧の段の打ち切りのテストに使う）
  - **非機能設計 R-08（途中の古い view）**：途中の更新（TauriSink::update）の view が cancel_fetch の view より後に届き、［中断］が一時的に押せる見た目に戻ることはありうる。終わりの view で必ず直り、その間に押しても request_cancel が None で何も起きないので、許容する。code-summary.md に書く
  - **非機能設計 R-09（ボタンの見た目の表）**：ボタンの見た目は次の表で決め、画面のテストも同じ表で確かめる

    | 段階 | cancelling | fetchCancelAvailable | ボタン |
    |------|------------|----------------------|--------|
    | Fetching 以外 | — | — | ［取得］（今のまま） |
    | Fetching | true | — | loading の［中断中…］（要素は無効にしないのでフォーカスが残る） |
    | Fetching | false | true | 主ボタンの［中断］ |
    | Fetching | false | false | disabled の［中断］（キャッシュに保存中、閉じる確認の途中） |

  - **インフラ設計 R-01（権限の 3 か所）**：`src-tauri` のテストに、build.rs のコマンドの一覧・capabilities/default.json の allow・generate_handler に登録したコマンドの一覧が一致することを確かめる小さなテストを足す（一覧を読み比べる）。手元の GUI でも［中断］が効くことを確かめる
  - **インフラ設計 R-02（GitGuardian）**：GitGuardian Security Checks は外部の GitHub アプリが出すチェックで、ワークフローの中には定義がない。code-summary.md の確かめの表に一言書く
  - **非機能要件の質問票の前半の古い決め**（応答を待って止まる、最後のページかを次を求めて決める）は追加の質問 Q1 で置き換わっている。この計画は Q1 の決めだけを使う

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

純粋な判断（終わりの状態の決め方、AppSession の押せるかの判定と状態の変わり方）はテストを先に書き、失敗を確かめてから実装する。取得の流れの打ち切り・Tauri のコマンド・画面は実装してから、その層のテストを書く。

### Step 1: 構成（Project structure and production configuration skeleton）

- [x] 新しい依存は足さない。作業ツリーのソースに変更がなく `claude/parallel-fetch` が `origin/claude/parallel-fetch` と同じであることを確かめる。新しいファイル（`src/components/CancelNotice.tsx`）の置き場所を確かめる

### Step 2: テストの走らせ方を確かめる（Verify the existing test runner）

- [x] `cargo test -p local-sights-core --lib coordinator::`、`cargo test -p local-sights-core --lib session::`、`cargo test -p local-sights`、`npx vitest run src/components/ConditionArea.test.tsx` が今のまま通ることを確かめる

### Step 3: 終わりの状態と AppSession — テスト先行

- [x] 先にテストを書く：`decide_job_status(stop, plan, failed, unfinished)` の表（止めていない・Close・一覧の前の Cancel・N > 0 の Cancel・N = 0 の Cancel、BR5.5）、`JobStatus::Cancelled` の名前と ALL（6 つ）、AppSession の `fetch_cancel_available`（取得中・cancelling・cache_saving・閉じる確認の途中、BR5.2）、`request_cancel`（押せれば cancelling と番号、押せなければ None、BR5.1）、`finish_fetch` のどの終わり方でも cancelling が戻る（NFR6.16）、Cancelled で段階 Done・件数と失敗が残る・注意（N または一覧の前）・cache_notices が空（BR5.8）、注意が消える 3 つの場合と同じロググループの選び直しで残る（BR5.9）、中断 → 次の取得 → 中断で注意が 1 つ（NFR6.14）、SessionView の JSON の 3 項目の形（NFR8.13）。失敗を確かめる
- [x] `fetch_status.rs` に Cancelled、`coordinator.rs` に decide_job_status の新しい入力と FetchJob の 2 項目（`unfinished_stream_count`、`cancelled_before_listing`）、`session.rs` に cancelling・cancel_notice・request_cancel・SessionView の 3 項目を実装する（security-design 2.1・2.2）。JobStatus の match の漏れはコンパイルに従って直す

### Step 4: 取得の流れの打ち切り — 実装してからテスト

- [x] `coordinator.rs`：ReadStage::run を `biased;` の select で止めを先にし、止めたら読みを落として抜け、N を `job.unfinished_stream_count` に入れる。一覧の段も止める信号と並べて待つ。run_fetch の止めたときの分岐を理由つきにし（Close は今の abort()、Cancel は decide_job_status）、一覧の前の Cancel を Cancelled・一覧の前の印で終える。serve_from_cache は Close のときだけ abort()（security-design 2.1）
- [x] `tests/support/fake_gateway.rs` に DescribeLogStreams を遅らせる・返さない台本を足す（R-07）。RecordingSink に Cancelled・N・一覧の前の印を記録させる
- [x] 結合テスト `tests/u5_cancel.rs` を書く：中断の後に始まる呼び出しが 0 回（NFR4.2）、再試行の待ち・許可待ち・応答待ち・返さない偽物で時計を進めずに終わる（NFR4.3）、応答待ちの 5 本が N に入りページが一覧にない（NFR4.4）、N の例（止めた＋まだ始めていない、失敗は入らない、1,000 本）（BR5.4、NFR4.6）、一覧の途中で Cancelled・一覧の前・0 件・キャッシュに書かない（BR5.5、BR5.7）、全部読み終えた後の中断で Completed・キャッシュに書く（NFR4.5、NFR6.18）、キャッシュを引いている間の中断（当たり Completed、外れ一覧の前）（NFR6.19）、中断の後に閉じるで Aborted（NFR6.15）
- [x] 閉じるときの今の結合テスト（u1〜u6、u1_concurrent_fetch など）が変えずに通ることを確かめる。`examples/fetch_check.rs` の終わりの状態の分岐に Cancelled を足す

### Step 5: Tauri のコマンド — 実装してからテスト

- [x] `src-tauri/src/lib.rs`：begin_and_spawn_fetch で持ち手を session のロックの中で登録する。`cancel_fetch` を足す（session → abort の順、view は session のロックを持ったまま送る、常に成功）。build.rs のコマンドの一覧、capabilities/default.json、generate_handler に `cancel_fetch`（security-design 2.3）
- [x] テストを書く：登録と状態の更新をまとめた関数で、始める直後の中断が新しい持ち手に届く・持ち手が無いときは何もしない（NFR6.17）、cancel_fetch の view の後に終わりの view が来る順（NFR6.16）、権限の 3 か所の一覧が一致する（インフラ設計 R-01）

### Step 6: 画面 — 実装してからテスト

- [x] `api.ts`（Cancelled、cancelling、fetchCancelAvailable、cancelNotice、cancelFetch）、`ConditionArea.tsx`（R-09 の表のボタン、R-06 の effect の条件を「ボタンが要素として無効か」に変える）、新しい `CancelNotice.tsx`（role="status" の囲みの Alert warning、閉じるボタンなし）、`App.tsx`（FetchErrorBanner と同じ場所）、`StatusLine.tsx`（Cancelled の「中断」と件数、0 件でも範囲にログがない文は出さない、キャッシュの通知を出さない、経過は U4 のまま）、`messages.ts`（`fetch.cancel`・`fetch.cancelling`・`status.cancelled`・注意 2 種の英日）、`test/fixtures.ts`
- [x] テストを書く：R-09 の表のボタン 4 つ、［取得］を押した後も［中断］・［中断中…］の間はフォーカスがボタンに残る・保存中だけ絞り込み欄へ移る（NFR10.1）、押すと cancelFetch が呼ばれる、日付欄の Enter で中断しない、注意の 2 つの文と role と閉じるボタンなし（NFR9.4）、エラーの表示と同時に出ない、状態行の「中断」と 0 件（BR5.12）、中断の後に絞り込みが使える（NFR5.5）、文言の一覧の英日（NFR9.3）

### Step 7: ビルドと確かめ（Environment/build configuration）

- [x] ライブラリ：`cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`cargo test -p local-sights-core --doc`、`cargo test -p local-sights-core --example fetch_check`
- [x] 画面：`npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .`、`npm run test:coverage`（80% 以上）、`npx vite build`、`npm run check:version`、`npm audit --audit-level=high`
- [x] アプリ：`cargo clippy --workspace --all-targets`、`cargo test -p local-sights`

### Step 8: 文書と対応表（Documentation and traceability）

- [x] 変えたモジュールの冒頭の文書（coordinator.rs の止め方、session.rs、ConditionArea・StatusLine の説明）を直し、README の画面の説明と手元の確認の一覧に中断を足す
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json` を書く

## 対応表（手順とストーリー）

| 手順 | ストーリー・要件 |
|------|------------------|
| Step 3 | US3.1（AC3.1.3、AC3.1.5、AC3.1.6）、US4.1（AC4.1.1〜AC4.1.5）、BR5.1、BR5.2、BR5.5、BR5.8、BR5.9、NFR6.14、NFR6.16、NFR8.13 |
| Step 4 | AC3.1.3、AC3.1.4、AC3.1.6、AC3.1.7、AC4.1.2、AC4.1.4、AC4.2.1、BR5.3〜BR5.7、BR5.14、NFR4.2〜NFR4.6、NFR6.15、NFR6.18、NFR6.19 |
| Step 5 | AC3.1.2、BR5.1、NFR6.17、NFR8.15 |
| Step 6 | AC3.1.1、AC3.1.5、AC3.1.8、AC4.1.1、AC4.1.3、AC4.1.6、BR5.10〜BR5.13、NFR5.5、NFR9.3、NFR9.4、NFR10.1 |
| Step 7 | NFR6.13、NFR7.4、NFR8.14 |

## Assumptions & Open Questions

None.
