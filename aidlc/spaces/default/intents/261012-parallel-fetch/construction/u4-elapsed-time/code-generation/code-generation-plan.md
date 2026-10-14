# Code Generation Plan — U4 かかった時間（u4-elapsed-time）

## Summary

- Builds: 取得にかかった時間を状態行の末尾に「経過 mm:ss」として出す。AppSession が戻らない時計で始まりと最終の値を記録し、SessionView の `elapsed` で渡し、画面の小さな部品が取得中に 1 秒ごとに進める（ADR-005）
- Touches: `crates/local-sights-core/src/session.rs`（時計・記録・SessionView）、`src/api.ts`（型）、`src/elapsed.ts`（新規、純粋な関数）、`src/hooks/useElapsed.ts`（新規）、`src/components/ElapsedTime.tsx`（新規）、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`、`src/test/fixtures.ts`
- Tests: AppSession の経過のテスト約 8 件（テスト先行）、`src/elapsed.ts` のテスト約 12 件（テスト先行）、ElapsedTime・StatusLine・App のテスト約 8 件（実装の後）。既存のテストは変えずに通し、行カバレッジはライブラリ・画面とも 80% 以上

## 上流と前提

- 上流：`construction/u4-elapsed-time/functional-design/`（rules.md BR4.1〜BR4.10、functional-spec.md）、`nfr-requirements/`（NFR2.2〜NFR9.2）、`nfr-design/`（security-design.md の設計の正本、レビュー 1 回目の R-01〜R-05）、`infrastructure-design/`（cicd-pipeline.md、monitoring-design.md、レビューの R-01・R-02）、`inception/user-stories/stories.md`（US2.1：AC2.1.1〜AC2.1.7）、`inception/domain-design/decisions.md`（ADR-005）、`inception/contract-design/contract-summary.md`（C4、C5）
- ブランチ：U1〜U3 と同じ `claude/parallel-fetch`。実際の AWS には自動テストから接続しない
- 後の段のレビューの軽微な指摘は、この計画で次のように片付ける
  - **非機能設計 R-01（新しい取得の見分け方）**：`ElapsedView` に取得の番号 `fetchNumber`（AppSession の `fetch_number`、数値）を足す。保持（減らさない）は「表示中が running、受け取ったものも running、番号が同じ」のときだけにする。番号が変わったら新しい取得として受け取った値に置き換える。さらに、すでに running でない値を見た番号の running は無視する（終わりの後に古い running が遅れて届いても時計が動き出さない）
  - **非機能設計 R-02（ずれの理屈）**：保持で先に出た分は送受信の遅れのばらつき分に収まり、終わりの running false で最終の値に置き換わる。テストは「保持の直後と取得中のずれが 1 秒を超えない」を確かめ、追いつくことは期待しない
  - **非機能設計 R-03（useElapsed の中身）**：起点（Anchor）は `useState` に置き、`elapsed` のオブジェクトが前の描画と変わったときに、描画の中で `nextAnchor` を呼んで起点を置き換える（React の「描画中に state を合わせる」形。前の `elapsed` も state に持つ）。今の時刻は差し替えられる関数（既定は `performance.now`）から読む。描き直しの予約は `useEffect` の中の `setTimeout`
  - **非機能設計 R-04（C5 の null の意味）**：`elapsed` が null になるのは「一度も取得していない」「閉じて止めた」「接続の変更で前の結果が消えた」の 3 つ。型の文書（Rust の doc コメントと `src/api.ts` のコメント）にこの意味を書き、契約 C5 の文言の拡張として code-summary.md に記録する
  - **非機能設計 R-05（見本の更新）**：`src/test/fixtures.ts` の SessionView の見本に `elapsed: null` を足す。`nextAnchor` と `useElapsed` は `undefined` も null と同じに扱う
  - **インフラ設計 R-01**：CI の Front end のジョブの `npm run check:version` と `npm audit` は U4 で変わらないが、Step 7 で手元でも流す
  - **インフラ設計 R-02（ずれの測り方）**：ElapsedTime のテストで、running の elapsed を固定値で渡し、偽のタイマーで画面の時計を進め、受け取った値に経った時間を足した値と表示の差が 1 秒未満であることを比べる

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

純粋な判断（AppSession の経過の決まり、`src/elapsed.ts` の表記と合わせ直し）はテストを先に書き、失敗を確かめてから実装する。時計のフック・経過の部品・状態行・App とのつなぎ込みは実装してから、その層のテストを書く。

### Step 1: 構成（Project structure and production configuration skeleton）

- [x] 新しい依存は足さない。作業ツリーがきれいで `claude/parallel-fetch` が `origin/claude/parallel-fetch` と同じであることを確かめる。新しいファイル（`src/elapsed.ts`、`src/hooks/useElapsed.ts`、`src/components/ElapsedTime.tsx`）の置き場所を確かめる

### Step 2: テストの走らせ方を確かめる（Verify the existing test runner）

- [x] `cargo test -p local-sights-core --lib session::` と `npx vitest run src/components/StatusLine.test.tsx` が今のまま通ることを確かめる

### Step 3: AppSession の経過 — テスト先行

- [x] 先にテストを書く（`session::tests`、時計を差し替える）：始めて 7 秒後の view が `{ running: true, elapsedMs: 7000, fetchNumber: n }`（BR4.5）、始めて 31.4 秒後に終わると `{ running: false, elapsedMs: 31400 }` で、その後に時計を進めても変わらない（BR4.2、NFR2.2）、キャッシュへの保存の知らせの後に 3 秒たって終わると 3 秒が入る（BR4.2）、取得中の begin_fetch（Busy）で始まりが変わらない（BR4.1）、2 回目の取得で 1 回目の値が消え番号が変わる（NFR6.4）、Aborted で null（BR4.3）、接続の変更で null・ロググループの選び直しで残る（BR4.4、NFR6.10）、abort_fetch_with_failure で running false（NFR6.11）、起動直後は null、時計が戻っても 0（NFR6.8）、SessionView を JSON にすると `elapsed` が `{"running":…,"elapsedMs":…,"fetchNumber":…}` か `null`（NFR8.10）。失敗を確かめる
- [x] session.rs に `SessionClock`（差し替えられる戻らない時計。Clone・Debug・Send・Sync、本番は `Instant::now`）、`with_clock`、`ElapsedRecord`、`ElapsedView { running, elapsed_ms, fetch_number }`、`SessionView.elapsed` を作り、`begin_fetch`・`finish_fetch`・`abort_fetch_with_failure`・`apply_connection_change`・`view` で書き換える（security-design 2.1）。`unwrap`・`expect` は使わない
- [x] 既存の session.rs のテストが変えずに通ることを確かめる（SessionView を組み立てるテストがあれば `elapsed` を足すだけにする）

### Step 4: 画面の純粋な関数 — テスト先行

- [x] 先にテストを書く（`src/elapsed.test.ts`）：`formatElapsed` の境目（0、999、7,999、59,999、3,599,999、3,600,000、75 分、10 時間、100 時間、負）（BR4.7、NFR6.3）、`nextAnchor`：null・undefined で null、running→running の同じ番号で小さい値は保持（NFR6.6）、番号が変わったら受け取った値（00:31 の最終の値の後に新しい取得の 0 で 00:00）、running false で置き換え（1 秒戻りうる）、running false を見た番号の running は無視（非機能設計 R-01）、`shownMs` の running と停止、`msUntilNextSecond`。失敗を確かめる
- [x] `src/elapsed.ts` を実装し、`src/api.ts` に `ElapsedView` 型と `SessionView.elapsed`（null の意味の文書つき）を足す。`src/test/fixtures.ts` の見本に `elapsed: null` を足す

### Step 5: 時計のフックと経過の部品 — 実装してからテスト

- [x] `src/hooks/useElapsed.ts`（非機能設計 R-03 の形、今の時刻の関数は差し替えられる）と `src/components/ElapsedTime.tsx`（`data-testid="status-line-elapsed"`、running の間 `aria-hidden="true"`）を作る。`src/i18n/messages.ts` に `status.elapsed`（英「Elapsed {time}」、日「経過 {time}」）を足す
- [x] テストを書く（`ElapsedTime.test.tsx`、偽のタイマー）：1 回受け取って 16 秒進めると 16 秒分進む（NFR6.5）、running false の後は進まない、null で何も出ない、running の間だけ `aria-hidden`（NFR9.2）、ずれが 1 秒未満（インフラ設計 R-02、NFR6.7）、部品が外れると予約が消える

### Step 6: 状態行と App のつなぎ込み — 実装してからテスト

- [x] `StatusLine.tsx` に `ElapsedTime` を「取得の表示 → キャッシュの表示 → 経過 → 絞り込みの表示」の順で置く。Idle・null では出さない（BR4.8）
- [x] テストを書く（`StatusLine.test.tsx`・`App.test.tsx`）：取得中・終わった・0 件・一部失敗・失敗・キャッシュから表示で末尾に経過、英日の文言（NFR9.1、AC2.1.6）、Idle で出ない、fetch-progress を合成しても表示が戻らない・時計が合わせ直されない、getSession の running で時計が始まる（NFR6.9）、終わりの表示と同じ描き直しで `aria-hidden` が外れる（NFR9.2）、毎秒の描き直しが経過の部品だけ（NFR5.3、描画の回数を数える）
- [x] 既存の画面のテストと文言の一覧のテストが通ることを確かめる

### Step 7: ビルドと確かめ（Environment/build configuration）

- [x] ライブラリ：`cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`cargo test -p local-sights-core --doc`
- [x] 画面：`npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .`、`npm run test:coverage`（80% 以上）、`npx vite build`、`npm run check:version`、`npm audit --audit-level=high`
- [x] アプリ：`cargo clippy --workspace --all-targets`、`cargo test -p local-sights`

### Step 8: 文書と対応表（Documentation and traceability）

- [x] 変えたモジュールの冒頭の文書（session.rs の経過、StatusLine.tsx の説明）を直し、README の状態行の説明に経過を足す
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json` を書く

## 対応表（手順とストーリー）

| 手順 | ストーリー・要件 |
|------|------------------|
| Step 3 | US2.1（AC2.1.3、AC2.1.4）、BR4.1〜BR4.5、BR4.10、NFR2.2、NFR6.4、NFR6.8、NFR6.10、NFR6.11、NFR8.10 |
| Step 4 | AC2.1.5、BR4.6、BR4.7、NFR6.3、NFR6.6、NFR6.12 |
| Step 5 | AC2.1.2、AC2.1.7、BR4.6、BR4.9、NFR5.3、NFR6.5、NFR6.7、NFR9.1、NFR9.2 |
| Step 6 | AC2.1.1、AC2.1.6、BR4.8、NFR5.3、NFR5.4、NFR6.9 |
| Step 7 | NFR6.2、NFR7.3、NFR8.11 |

## Assumptions & Open Questions

- [assumption] `ElapsedView` に `fetchNumber`（数値）を足す。契約 C5 は TypeScript の型の正確な形を U4 の機能設計以降で決めるとしており（C5 の仮の決め）、数値なので NFR8.10（真偽値と数値だけ）にも収まる
