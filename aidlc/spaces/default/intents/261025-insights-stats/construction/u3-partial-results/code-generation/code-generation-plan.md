# Code Generation Plan — U3 途中結果と中断後の結果（u3-partial-results）

## Summary

- Builds: 取得中の途中結果（取得中の保持の登録と途中の view、viewVersion、1 秒あけた別タスクの途中の評価、版つきの結果と行の見分け）と、［Cancel］のあとの中断の結果（Cancelling、中断後の 1 回の評価、2 回目の［Cancel］）、途中・中断の印と読み上げ、U2 のレビューの持ち越し
- Touches: `crates/local-sights-core/src/query/`（store・runner）、`insights_session.rs`、`examples/fetch_check.rs`、`src-tauri/src/lib.rs`・`build.rs`・`capabilities/default.json`・`permissions/`、`src/api.ts`・`QueryResultTable.tsx`・`LogAnalyticsPage.tsx`・`StatusLine.tsx`・`CancelNotice.tsx`・`ExportNoticeAlert.tsx`・`hooks/expansionState.ts`・`QueryEditor.tsx`・`i18n/messages.ts`、`README.md`
- Tests: Rust の単体テスト 約 45（QueryStore の登録・確定・途中の view・viewVersion 12、QueryRunner の途中の評価・中断の 3 つの場合 10、InsightsSession の版・stage・Cancelling・rowKey 14、Tauri 4）と Vitest 約 20（途中・中断の印、読み上げ、rowKey で開いた行と先頭の行を保つ、Cancelling の判定、中断の表示と通知）

上流：`construction/u3-partial-results/functional-design/`（entities.md・rules.md の BR1.1〜BR5.1・functional-spec.md の UC1〜UC5）、`inception/units-generation/unit-of-work.md`（U3）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.10・FR5.1〜FR5.4、NFR1・NFR2・NFR9・NFR10、C5）、`inception/domain-design/decisions.md`（ADR-003・ADR-004・ADR-006）。ストーリーはないため、各手順に BR と FR を書いて追う。U2 のコード生成のレビューの低い所見のうち R-01・R-02・R-05 は、決まり（レビューのあとの修正は次の作業単位に回す）に従ってこの作業単位で直す（Step 23〜25）。R-03（打ち切りの注記は LogAnalyticsPage の見出しにある）は Step 19 の前提にする。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。",
  "scope": "refactor",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Methodology evidence**: 261002-cwlogs-viewer の面談（Q4 の回答 B）で、純粋なロジックはテスト先行、AWS 接続と GUI の層は実装後にテスト、という混在した進め方に決め、261011-local-release と 261012-parallel-fetch の面談でも変えていない。261015-windows-support の面談でも進め方そのものを変える回答はなく（Q2 は Windows のランナーでテストを流す決まりを加えるだけ）、増えるのは OS ごとに分かれる境界の層（AWS CLI の起動と停止、ファイルの置き場所と権限など）で、純粋なロジックの扱いは変わらない。今のコード（判断は副作用のない関数に寄せ、OS に触れる処理は `cfg(unix)` などで境界に閉じ込め、テストは各モジュールの中にある）と git の履歴にも、これと食い違う跡はない。\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- カバレッジ：行カバレッジを CI で測り、80% を下回る PR はマージしない（ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）。計測は Linux のランナーで行う。これは org.md のスコープ別の下限（`mvp` などの 80% 行カバレッジとマージ前の CI での実行）を CI の合否で確かめるもので、下限を下げることはしない。そのほかのスコープ別の決まりも org.md に従う。\n- GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。AWS CLI もテストでは本物を使わず、偽の CLI に差し替える。\n- OS ごとに分かれるコード（`cfg(windows)`、`cfg(unix)` など）のテストは、その OS のランナーで実行されるように書く。偽の CLI などテストの道具も OS ごとに用意する。\n- Rust 側のテストは `cargo test`（ライブラリとアプリの両方）で、画面側（フロントエンド）のテストは Vitest で実行し、どちらも CI でマージ前に実行する。Rust 側のテストは CI の Linux・macOS・Windows（`windows-latest`）の各ランナーで実行し、画面側のテストは OS に依存しないため Linux のランナーで実行する。既存のテストはすべての OS で通った状態を保つ。"
    },
    {
      "layer": "project",
      "text": "- クエリの速さの合否の数値は、利用者の答え（作る段階で決める）に従って要件では決めなかった（NFR1） (learned 2026-10-17) \n\n- 依頼 (4) の権限のテストの固定値は依頼文の 26 ではなく今は 32 だったため、数の固定そのものをやめて 3 か所の一覧の一致だけを確かめることにした（Q8 A）。 (learned 2026-10-20)"
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
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
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
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:6b791a61f9f432be6d2e64e14b35060648c7ba1feb658930c23c1bc6ff063b71",
  "contract_sha256": "sha256:1e653206b60c1801cccf7ea17a92ff67599510c6fe6d79d8aa8a845da0a0571a"
}
```

## 進め方

Testing Contract の custom の順番に従う。純粋なロジック（QueryStore の登録・確定・途中の view の並び・viewVersion の照合、InsightsSession の版の受け取り・stage・Cancelling の判定・rowKey、起動の判断の関数）はテストを先に書いて失敗を確かめてから実装する。QueryRunner のタスクとチャンネル（並行の境界）、Tauri、画面は先に実装してからテストを書く。既存のテストは期待値を変えずに通す。ただし前の作業のクエリの中断のテスト（中断なら評価しない）だけは、FR5.1 に合わせて直す（BR5.1）。ログ管理の画面の取得・表示・中断は変えない。テスト以外のコードでは `unwrap()` / `expect()` を使わない。AWS の呼び出しは増やさない。

## 手順

### 準備

- [x] **Step 1** — テストの実行の確認：unit-test-instructions.md のコマンドが今の状態で通ることを確かめ、通過の数を記録する

### 取得中の保持と途中の view（純粋なロジック：テスト先行）

- [x] **Step 2** — `store.rs` のテストを先に書く：
  - `register_inflight` と `settle_inflight`：holder から取り出して確定し、登録から外す。確定のあとは holder にも登録にもなく、確定済みに 1 回だけある。取り出しの前後で 2 回数えない・落とさない（BR1.2、FR4.8）。completed = false なら staging に入り、取得済みの範囲の記録に入らない（BR1.2・BR3.6）。
  - `build_partial_view`：確定済みの分と取得中の分の写しを（時刻, ロググループ, ストリーム, ストリーム内の順番）の順に並べる。写しの順番は確定のときと同じ規則で、`StreamSequences` を進めない。期間の外は除く。キャッシュから読んだ分は確定済みとして入る。`ViewEntry::Inflight` を足す（BR1.3）。
  - viewVersion：view を作るとき、`settle_inflight`・`add_cached`・`begin_run`・`discard_all` のたびに進む。`evaluate_step`・`copy_rows` は (epoch, viewVersion) が食い違えば失敗を返す（BR1.4）。
  - 途中の view の評価の結果が、同じイベントを確定したあとの最終の view の評価の結果と同じ並びになる（BR2.2）。
  - 実行して失敗を確かめる（FR4.1・FR4.2（読み替え）・FR4.8・FR5.3）
- [x] **Step 3** — `store.rs` を実装し、Step 2 のテストと既存のテストを通す：`InflightRegistry`・`InflightHolder`、鍵の順番は「QueryStore → holder」に固定、`PartialView`（epoch・viewVersion・entries・inflightCopies）

### 途中の評価の判断と中断（純粋なロジック：テスト先行）

- [x] **Step 4** — `runner.rs` の起動の判断のテストを先に書く（副作用のない関数 `should_start_partial`）：最初のページが届いていなければ始めない、最初のページのあとの最初はすぐ、動いている途中の評価があれば始めない、前の評価の終わりから 1 秒たっていなければ始めない、取得した件数が前の途中結果のときより増えていなければ始めない、足りない範囲がなく取得を始めないときは始めない（BR1.1、FR4.4・FR4.9）。あわせて、取得の終わり方から「評価する / 中断として評価する / しない」を決める関数のテスト（BR3.1・BR3.2）。実行して失敗を確かめる
- [x] **Step 5** — `runner.rs` の判断の関数を実装し、Step 4 のテストを通す（`PARTIAL_EVALUATION_INTERVAL` は 1 秒の定数）

### 結果の版と段階（純粋なロジック：テスト先行）

- [x] **Step 6** — `insights_session.rs` のテストを先に書く：
  - 版：runId が今の実行と同じで版が新しいものだけを受け取り、版を 1 つ進める。古い版・別の実行は捨てる（BR2.4）。
  - stage：partial（coverage つき）、final、cancelledPartial（BR4.1）。coverage はロググループの番号・数・含めた件数（BR2.6）。
  - 前回の結果：実行を始めたら previous のまま残し、最初の版（途中結果か、途中結果を経ない最終結果）で差し替える。形が前回と違っても差し替える（BR2.5）。
  - Cancelling：`is_running` は Fetching・Evaluating・Cancelling で真。`cancel_available` は BR3.2 の条件（2 回目は 1 回だけ、3 回目は押せない）。`finish_fetch`・`on_fetch_ended` の戻り値を Evaluate / EvaluateCancelled / Skip の enum に（BR3.1・BR3.7）。
  - 中断の 3 つの場合：取得中の［Cancel］で中断後の結果が cancelledPartial になる（BR3.1）。Cancelling の間の 2 回目で、今の実行の途中結果があれば cancelledPartial にして残し（版は進めない）、なければ今の Cancelled（BR3.2）。Evaluating の間の［Cancel］も同じ（BR3.3）。
  - rowKey：イベントの行は（ロググループ名, ストリーム名, sequence）を U+001F でつなぐ、集計の行は鍵のセルの文字をつなぐ、by なしの 1 行は空の文字（BR4.2）。
  - 版つきの読み出し：`display_rows`・`display_row_detail` は版が違えば空、`find_row(run_id, version, row_key)` は位置か none（BR2.4・BR2.8）。
  - 実行して失敗を確かめる（FR4.3・FR4.5〜FR4.7・FR5.1・FR5.4）
- [x] **Step 7** — `insights_session.rs` を実装し、Step 6 のテストと既存のテストを通す：`AnalyticsPhase::Cancelling`、`abandonRequested`、`ResultVersion`・`PartialCoverage`、`ResultSummary` に version・stage・coverage、`QueryRow.rowKey`、`find_row`。前の作業のクエリの中断のテスト（評価しない）は FR5.1 に合わせて直す（BR5.1）

### 途中の評価のタスクと中断の流れ（並行の境界：実装してからテスト）

- [x] **Step 8** — `runner.rs` を実装：`QuerySink::on_partial_result(run_id, result, coverage)` を足す。`fetch_range` は holder を `store.register_inflight` で登録し、終わったら `store.settle_inflight` を 1 回呼ぶ（今の take_events → settle_range をまとめる）。取得の進み具合を `Arc` の `AtomicBool`・`AtomicU64` にも書く。本流は取得の future と途中結果の mpsc を `tokio::select!` で待ち、sink は本流だけが持つ。途中の評価は blocking のタスクで行い、最後まで進んだものだけ送る。取得が終わったら途中のタスクに止める信号を出して終わるのを待ち、最終の view を作る（BR1.1・BR1.4・BR2.1）。中断では、取得が止まったあとに staging を含む最終の view で評価し、その評価には abandon だけを見せる。abandon なら評価を始めない／次の塊の間で止める（BR3.1・BR3.2）。StopHandle に abandon を足す
- [x] **Step 9** — `runner.rs` のテストを書いて実行する（偽の取得元。時間は測らない）：取得中に途中結果が版の順に届き最後に最終結果が来る、途中結果の並びが最終結果と同じ決まり、2 回数えない・落とさない（FR4.8）、取得済みで AWS を呼ばないときは途中結果なし（FR4.9）、stats と U2 の stats のあとの命令も途中で同じ、取得中の［Cancel］で中断後の結果、2 回目の［Cancel］で中断後の評価を止める（重い stats でも止まる、DD R-10 の U3 分）、Evaluating の間の［Cancel］、取得の一部のストリームが失敗したときの中断（FR5.4）、ログ管理の画面の取得のテストが今のまま通る（BR5.1）

### Tauri（境界の層：実装してからテスト）

- [x] **Step 10** — `src-tauri/src/lib.rs` を実装：`TauriQuerySink::on_partial_result` で InsightsSession の結果を差し替え、session-changed を送る（行は載せない。最終・中断の結果はすぐ送る、BR2.3）。`get_query_rows`・`get_query_row_detail` に version を足し、新しいコマンド `find_query_row` を足す。`cancel_query` は Fetching なら中断、Cancelling なら abandon（BR3.1・BR3.2・BR4.1）。`build.rs`・`capabilities/default.json`・`permissions/` と、権限の一致のテストの名前の一覧に足す
- [x] **Step 11** — Tauri のテストを書いて `cargo test -p local-sights --lib` を実行する：版が違う読み出しは空、`find_query_row` の位置と none、2 回目の `cancel_query`、新しいコマンドの権限

### 画面（境界の層：実装してからテスト）

- [x] **Step 12** — `src/api.ts`：`ResultSummary` に version・stage・coverage、`QueryRow.rowKey`、`getQueryRows`・`getQueryRowDetail` に version、`findQueryRow`、phase の型に `cancelling`（BR4.1・BR3.7）
- [x] **Step 13** — `src/hooks/expansionState.ts`：開いた行を位置ではなく rowKey で覚え、新しい版で同じ rowKey がなければ閉じる（BR2.8）
- [x] **Step 14** — `src/components/QueryResultTable.tsx`：行の読み出しと開いた行の詳細の取得の鍵・効果の依存に version を含める。版が進んだら、差し替えの前に先頭に見えていた行の rowKey を `findQueryRow` で探し、見つかればその行を先頭に合わせる。答えの (runId, version) が今の見出しと違えば捨てる（BR2.4・BR2.8）
- [x] **Step 15** — `LogAnalyticsPage.tsx`：見出しの横に途中の印（「途中の結果（取得中：ロググループ {n} / {m}、{count} 件まで）」、ロググループが 1 つなら省く）と中断の印（「一部のログだけの結果（中断）」）を Cloudscape の標準の部品と文字で出し、最終結果で消す（BR2.6・BR3.4）。読み上げの領域には、途中結果が初めて出たとき・最終結果になったとき・中断の結果になったときだけ文を出す（BR2.7）。running の判定 3 か所に Cancelling を足す（BR3.7）。途中でも表を描き、［Export］は実行中は押せないまま（BR2.9）
- [x] **Step 16** — `StatusLine.tsx` に Cancelling（「結果を作っています」と評価の件数）、`CancelNotice.tsx` は結果があれば「一部のログだけの結果を出しました」、なければ今の「止めた」（BR3.4・BR3.7）
- [x] **Step 17** — `ExportNoticeAlert.tsx`：結果が cancelledPartial のときだけ、完了の通知に「一部のログだけの結果です」を添える。ファイルの中身には印を含めない（BR3.5、FR5.2）
- [x] **Step 18** — `src/i18n/messages.ts` に新しい文言（途中の印、中断の印、読み上げの 3 つの文、Cancelling の段階、中断の表示、書き出しの通知の添え書き）を英語と日本語で足す（NFR9）
- [x] **Step 19** — Vitest のテストを書いて実行する：途中の印の文字（ロググループが 1 つ・複数）、中断の印と失敗の一覧が両方出る、最終結果で印が消える、読み上げは状態の変化のときだけで版が進むたびには出ない、開いた行が rowKey で保たれ同じ行がなければ閉じる、先頭の行が保たれる、古い版の答えを捨てる、Cancelling の間は［Run query］・欄・［Export］が使えず［Cancel］は 1 回だけ押せる、中断の表示の文、書き出しの通知の添え書き、U2 の aggregates の打ち切りの注記（LogAnalyticsPage の見出し）が途中結果でも出る、権限の一致（`commandRegistration.test.ts`）

### 速さの確かめ（境界の層：実装してからテスト）

- [x] **Step 20** — `examples/fetch_check.rs` に、途中結果あり・なしを切り替える引き数を足し、取得の終わりまでの時間と、最初のページから最初の途中結果までの時間を出す（手元で実際の AWS に対して流す用。AWS の呼び出しは今の取得のまま、NFR1）。自動テストでは時間を測らない
- [x] **Step 21** — `fetch_check` のテスト（偽の取得元）を足して実行する：引き数の読み取り、出力の形

### 写しの大きさ（純粋なロジック：テスト先行）

- [x] **Step 22** — 1 つの範囲の取得中の保持が大きい（数十万件）ときの写しの時間を、`#[ignore]` の測定のテストで測る。長ければ（目安：1 回の鍵で 50 ms を超える）写しを塊に分ける。分けるときは、先に並びと件数が同じになることのテストを書いてから直す（functional-spec の残した問い）

### 持ち越し（U2 のコード生成のレビュー）

- [x] **Step 23** — README に書き足す（U2 レビュー R-01・R-02）：pct の百分位の小数は 15 桁まで、stats のあとの filter で値のない列への `!=` は今の filter と同じく成り立つこと。あわせて U3 の途中結果と中断の結果の振る舞いを短く書く。U2 の README と一覧の一致のテストが通ったままであることを確かめる
- [x] **Step 24** — `QueryEditor.tsx` の `registerQueryMode` の呼び出しを `useMemo` から外し、ESLint の `react-hooks/set-state-in-effect` に当たらない形（一覧が届いたあとに 1 回だけ登録する効果か、描画の外の関数）にする。色分けのテストの期待値は変えない（U2 レビュー R-05）
- [x] **Step 25** — 持ち越しのテストを実行する：`npx vitest run src/components/QueryEditor.test.tsx` と `cargo test -p local-sights-core --lib query::catalog`

### 仕上げ

- [x] **Step 26** — 全体の確認：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、unit-test-instructions.md のすべてのコマンド、`cargo test --workspace` と Vitest 全体（既存の速さのテストを含む）が通ることを確かめる

## 追跡（要件・決まり → 手順）

| 要件 | 決まり | 手順 |
|---|---|---|
| FR4.1 | BR1.1・BR1.3・BR2.1 | Step 2〜5・8・9 |
| FR4.2（読み替え） | BR2.2 | Step 2・9 |
| FR4.3 | BR2.1・BR2.2 | Step 6・9 |
| FR4.4 | BR1.1・BR2.3・BR2.8・BR4.1・BR4.2 | Step 4・5・10〜14・19 |
| FR4.5・FR4.6 | BR2.6 | Step 6・15・18・19 |
| FR4.7 | BR2.5 | Step 6・7 |
| FR4.8 | BR1.2〜BR1.4 | Step 2・3・9 |
| FR4.9 | BR1.1 | Step 4・9 |
| FR4.10 | BR2.4・BR2.9 | Step 15・19 |
| FR5.1 | BR3.1〜BR3.3・BR3.7 | Step 4〜11・15・16・19 |
| FR5.2 | BR3.5 | Step 17・19 |
| FR5.3 | BR1.2・BR3.6 | Step 2・3・9 |
| FR5.4 | BR3.4 | Step 6・9・15・19 |
| NFR1 | BR5.1 | Step 20・21・26 |
| NFR2 | BR1.1・BR1.3 | Step 8・22 |
| NFR9・NFR10 | BR2.6・BR2.7・BR3.4 | Step 15・18・19 |
| C5 | BR5.1 | Step 7・9・26 |
| U2 レビュー R-01・R-02・R-05 | — | Step 23〜25 |
