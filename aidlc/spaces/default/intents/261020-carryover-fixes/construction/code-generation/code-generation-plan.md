# Code Generation の計画（261020-carryover-fixes）

## Summary

- Builds: 持ち越し Minor 5 項目の直し（不安定なテスト、クエリの画面の保持、古い説明、画面の小物、リージョンの名前と絞り込み）
- Touches: `crates/local-sights-core`（cache・query・insights_session・session・tests/u5_cancel.rs・examples/compat_check.rs）、`src-tauri/src/lib.rs`、`src/`（i18n・TopBar・CancelNotice・App・api・QueryResultTable）、`docs/releasing.md`、261017・261012 の記録 2 つ
- Tests: Rust の単体テスト 約 12 件の追加・直し、結合テスト 1 件の直し、Vitest 約 20 件の追加・直し（既存のテストはすべて緑のまま）

## 前提と決めたこと

作業単位はない（bugfix のスコープ、Units Generation なし）。この段の記録は `construction/code-generation/` の下に置く。入力は `inception/requirements-analysis/requirements.md`（承認済み）と、コードの知識ベース `aidlc/spaces/default/codekb/local-sights/`。

コードを読んで、要件の Open Questions と前提を次のように決めた。

- **OQ1（FR1 の製品のコードの入口）**：要らない。`run_fetch_with_cache` は照合（`look_up`）の前に中断を見ておらず、照合のあとに初めて中断の有無で分かれる（当たり：`serve_from_cache` が `Cancel` なら全部を出して Completed、外れ：`run_fetch` → `plan_streams` が `Retrier` の中断で一覧を呼ばずに終わる）。そのため「取得を始める前に中断を送っておく」ことは、製品のコードから見て「照合が答える前に中断が届いている」状態と同じになる。テストの道具（`tests/u5_cancel.rs` の中の新しい補助関数）だけで順番を固定でき、時計や実スレッドの速さに依らない。
- **OQ2（内部の世代）**：残す。`QueryStore` の `epoch` は `view_epoch`・`copy_rows`・`view_len`・`build_view` で、前の実行の表示用の保持を使わないための内部の判定に使っている。外に出る `holdings_epoch`（結果・行・詳細の項目と、コマンドとライブラリの関数の引数）だけを取り除く。画面は `runId` だけで古い応答を見分ける（接続を変えると結果は消えるため、同じ `runId` で別の保持になることはない）。
- **FR2.1 の読み止め**：重なる記録済みの範囲のうち、一番後ろのもの（ファイルの時刻順で最後に重なる範囲）の終わりを過ぎた行で読むのをやめる。件数の確かめは、その範囲までのすべての記録済みの範囲について行う（読んだ範囲は全部読み切っているため）。
- **FR4.1 の仕組み**：英語の単数形の文を別の表（`englishSingular`：キー → 単数形の文と決め手の数の名前）に置き、`translate` の 1 か所で「英語で、決め手の数がちょうど 1 なら単数形」を選ぶ。`messages` の形（`en`・`ja` の 2 つ）は変えない。
- **FR5 の部品**：Cloudscape の `Select` を `filteringType="manual"` で使い、絞り込みは自分で書く純粋な関数（`filterRegionOptions`）で行う。A7 の括弧の全角・半角の違いを吸収するため、Cloudscape の自動の絞り込みは使わない。候補なしの文は `empty`／`noMatch` に新しい文 `connection.region.noMatch` を渡す。今の接続のリージョンは `selectedOption` で印が付く（Select の標準のチェックの印）。
- **FR5.10 の一致のテスト（レビュー R-08）**：Vitest で `crates/local-sights-core/src/catalog/regions.rs` を `?raw` で読み（`src/commandRegistration.test.ts` と同じやり方）、`pub const STANDARD_REGIONS: &[&str] = &[` から次の `];` までを取り出し、その中の `"…"` の文字列をすべて抜く。取り出せないとき、または 0 件のときはテストを落とす（黙って通さない）。
- **FR3.2 の U4 の説明**：`examples/compat_check.rs` の `limit 0` の説明（「`limit 0` keeps every row up to 10,000」など）に、今のアプリは `limit 0` で 0 行を返すこと、両側が 0 行になるため比べには影響しないことを書き足す。振る舞いは変えない。
- **A5 のリージョンの名前**：34 個の英語と日本語の名前は、AWS の公式の資料（「AWS リージョン」の一覧の英語版と日本語版）で確かめてから書く。確かめられなかった名前は code-summary.md に書く。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。",
  "scope": "bugfix",
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
      "Include a targeted regression for the bug or vulnerability.",
      "Keep the existing test suite green."
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
  "input_sha256": "sha256:a3605e648751490efc1d9773f0bde53d4b22646d185474153b9f790be2526830",
  "contract_sha256": "sha256:abe164b161552109cbf34b9fb99cb24f5e54413218a85ceccd4dabf7cfdbdddf"
}
```

## 手順

進め方は Testing Contract の custom の順番に従う：純粋なロジック（FR2.1 の読み止めの判断、FR2.2 の捨て方、FR4.1 の単数・複数の選び分け、FR5.4 の絞り込み）はテストを先に書いて落ちることを確かめてから実装する。境界の層（キャッシュのファイルの読み、Tauri のコマンド、画面の部品）は実装してからテストを書いて流す。

### 準備

- [ ] **Step 1：構成の確かめ。** 新しく作るファイルは `src/i18n/regions.ts` と `src/i18n/regions.test.ts` だけで、ほかは今のファイルを直す（重複のファイルは作らない）。設定（`Cargo.toml`、`package.json`、`vitest.config.ts`）は変えない。（要件：全体）
- [ ] **Step 2：テストの道具の確かめ。** 次のコマンドが今の `main` の状態で流れて通ることを確かめ、`unit-test-instructions.md` のコマンドと一致させる：`cargo test -p local-sights-core --lib cache::`、`cargo test -p local-sights-core --lib query::store`、`cargo test -p local-sights-core --test u5_cancel`、`cargo test -p local-sights --lib tests::the_manifest_the_capability_and_the_handler_list_the_same_commands`、`npx vitest run src/i18n src/components/TopBar.test.tsx src/components/CancelNotice.test.tsx src/components/QueryResultTable.test.tsx src/App.test.tsx`。（NFR1）

### FR1 不安定な結合テスト（境界：テストの道具）

- [ ] **Step 3：取得の前に中断を送る補助関数を足す。** `crates/local-sights-core/tests/u5_cancel.rs` に `fetch_already_stopped(gateway, timeline, cache, reason)` を足す：`stop_pair` で中断を先に `stop` してから `run_fetch_with_cache` を呼ぶ（時計を待たない）。製品のコードは変えない。（FR1.1、FR1.2、A3、OQ1）
- [ ] **Step 4：テストを直す。** `a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss` を、`fetch_and_stop(…, Duration::ZERO, …)` から `fetch_already_stopped(…, StopReason::Cancel)` に替える。当たりは Completed・キャッシュから・2 件、外れは Cancelled・`cancelled_before_listing`・AWS の呼び出しなし・0 件、という確かめは今のまま。説明のコメントに、照合の前に中断が届いている状態をこの補助関数で作る理由（照合は実スレッドで走るため、時計で中断を送ると照合の答えが先に返ることがある）を書く。（FR1.1〜FR1.3、NFR6.19 は変えない）
- [ ] **Step 5：200 回流す。** `for i in $(seq 1 200); do cargo test -q -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup || break; done` を流し、200 回とも通ったことを code-summary.md に書く。（FR1.3 の受け入れ基準）

### FR2.1 キャッシュの読み止め（純粋なロジックはテスト先行、ファイルの読みは実装後）

- [ ] **Step 6：判断の関数のテストを先に書く（Red）。** `crates/local-sights-core/src/cache/plan.rs` のテストに、まだない 2 つの関数のテストを足し、コンパイルが通らない（落ちる）ことを確かめる：
  - `last_overlapping_range(wanted, available) -> Option<CoveredRange>`：重なる記録済みの範囲のうち一番後ろのもの。重なりなしは `None`、1 つ目とだけ重なる、2 つ目と 3 つ目にまたがる、境の 1 ミリ秒だけ重なる、の 4 件。
  - `EventCounter::verify_through(last)`：`last` までのすべての範囲の件数が見出しと合えば `Ok`。合う、`last` の手前の範囲が足りない、`last` が見出しにない、の 3 件。
- [ ] **Step 7：判断の関数を実装する（Green）。** 上の 2 つを実装し、Step 6 のテストを通す。（FR2.1.1、FR2.1.2）
- [ ] **Step 8：`read_overlaps` を直す。** `cache/mod.rs` の `EventSelection` に `Through { last: CoveredRange }` を足し、`read_events` で `last.end_ms` を過ぎた行で読むのをやめて `verify_through(last)` で件数を確かめる。`read_overlaps` は重なりがあるとき `EventSelection::All` の代わりにこれを使う。重なりなし（見出しだけ）・壊れたら消す・読めないファイルは残す、の振る舞いは変えない。`lookup` と `write` の読み方は変えない。doc コメントを新しい読み方に直す。（FR2.1.1〜FR2.1.5、A4）
- [ ] **Step 9：ファイルの読みのテストを足す。** `cache/mod.rs` のテストに次を足す：3 つの範囲のファイルで 1 つ目とだけ重なると 1 つ目のイベントだけが返る／1 つ目の後ろの行（2 つ目の範囲の行）を壊したファイルでも 1 つ目とだけ重なるなら壊れに気づかず結果が返る（＝その先を読んでいない）／重なる範囲の中の行を壊すと Corrupt でファイルが消える／2 つ目と 3 つ目にまたがると両方のイベントが返り、全部読んだときと同じ結果になる。（FR2.1 の受け入れ基準、FR2.1.5）

### FR2.2 選ばれなくなったロググループの保持を捨てる（純粋なロジック：テスト先行）

- [ ] **Step 10：テストを先に直す（Red）。** `crates/local-sights-core/src/query/store.rs` の `a_log_group_no_longer_chosen_keeps_its_holdings_until_the_connection_changes` を、新しい決まりのテスト `a_log_group_no_longer_chosen_is_discarded_at_the_next_run` に替える（A と B で実行 → A だけで実行すると B の件数と範囲が 0、A は残る）。もう 1 件、「A だけのあと A と B で実行すると、B の範囲は空（取り直しになる）で A の範囲は残る」を足す。`discard_all` のテストはそのまま。流して落ちることを確かめる。（FR2.2.1〜FR2.2.5）
- [ ] **Step 11：`begin_run` を直す（Green）。** `begin_run` で、`keys` に入っていないロググループの保持を `groups` から取り除く。doc コメントを直す（U3:BR1.3 に「選ばれていないものは捨てる」を足し、BR1.6 の「接続を変えるまで残す」の記述を消す）。
- [ ] **Step 12：取り足しの結合を確かめる。** `crates/local-sights-core/src/query/runner.rs` のテストに、A と B で実行 → A だけ → A と B で実行したとき、2 回目に B が偽のゲートウェイ（またはキャッシュ）から取り直され、A は足りない範囲だけを取ることを確かめるテストを 1 件足す（実装後にテスト）。今の runner のテストで「選ばれなくなったものが残る」前提のものがあれば、新しい決まりに合わせて直す。（FR2.2.2、FR2.2.3 の受け入れ基準）

### FR2.3 `holdings_epoch` を取り除く（境界：実装してからテスト）

- [ ] **Step 13：ライブラリから取り除く。** `query/evaluator.rs` の `QueryResult` の `holdings_epoch` 項目と、作る関数の引数、`insights_session.rs` の `ResultSummary`・`QueryRowWindow`・`QueryRowDetail` の項目と `display_rows`・`display_row_detail` の引数、`session.rs` の `display_query_rows`・`display_query_row_detail` の引数から取り除く。`QueryStore` の内部の `epoch` は残す（OQ2）。関係するテストの期待値から `holdings_epoch` を消す（中身・並び・件数の期待値は変えない）。（FR2.3.1、FR2.3.2、FR2.3.4、FR2.3.5）
- [ ] **Step 14：アプリから取り除く。** `src-tauri/src/lib.rs` の `get_query_rows`・`get_query_row_detail` のコマンドと補助関数の引数から取り除き、doc コメントを直す。（FR2.3.2）
- [ ] **Step 15：画面から取り除く。** `src/api.ts` の型（`ResultSummary`・`QueryRowWindow`・`QueryRowDetail`）と `getQueryRows`・`getQueryRowDetail` の引数、`src/components/QueryResultTable.tsx` の古い応答の見分け（`runId` だけで見分ける）と詳細の鍵、`src/test/fixtures.ts`・`src/App.test.tsx`・`src/components/QueryResultTable.test.tsx` から取り除く。（FR2.3.3）
- [ ] **Step 16：取り除いたことを確かめる。** `cargo test --workspace` と、Step 2 の Vitest のコマンドを流し、`holdings_epoch`／`holdingsEpoch` を `crates`・`src-tauri/src`・`src` で検索して、外に出る項目と引数に残っていないことを確かめる（内部の `epoch` は残る）。`QueryResultTable.test.tsx` に「古い `runId` の応答は捨てる」テストがあればそれで、なければ 1 件足す。（FR2.3 の受け入れ基準）

### FR3 記述の古さ（説明だけ）

- [ ] **Step 17：`DEFAULT_RESULT_LIMIT` の説明を直す。** `crates/local-sights-core/src/query/plan.rs` の説明を「`limit` のコマンドがないときの結果の行の上限。`limit` があれば `limit 0` も含めてその値で切る（`has_row_limit`）」の形にする。（FR3.1）
- [ ] **Step 18：261017 の U2 の記録を直す。** `aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/code-generation/code-summary.md` の部品の表（QueryPlan の行）の「`limit 0` は絞りなし、R-04」を「`limit 0` は 0 行、Loop-back 3 の p08」に直す。（FR3.2）
- [ ] **Step 19：U4 の確認用プログラムの説明を足す。** `crates/local-sights-core/examples/compat_check.rs` の `limit 0` の説明に、今のアプリは `limit 0` で 0 行を返し、両側が 0 行になるため比べには影響しないことを書き足す。振る舞いは変えない。（FR3.2）
- [ ] **Step 20：`nextAnchor` の説明を足す。** `src/elapsed.ts` の `nextAnchor` の doc コメントに、取得の番号を等しいかどうかだけで比べるため、起動直後に古い `getSession` の応答が遅れて届くと新しい取得として受け取られ、経過時間が一度だけ戻ることがある、と書く。`aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/code-generation/code-summary.md` にも同じ点を 1 項目で足す。振る舞いは変えない。（FR3.3）
- [ ] **Step 21：`docs/releasing.md` を直す。** 「The next release is `v0.2.0` …」の段落を、公開済みの版（`v0.1.0`・`v0.1.1`・`v0.2.0`）の一覧と、次の版の番号はタグを打つ直前に決めること、の形にする。手順の例の番号（`0.2.0`、`v0.2.0`、`v0.2.1`）を `X.Y.Z`／`vX.Y.Z` の置き場所にする。`npx prettier --check docs/releasing.md` を通す。（FR3.4）

### FR4.1 英語の単数・複数（純粋なロジック：テスト先行）

- [ ] **Step 22：テストを先に書く（Red）。** `src/i18n/messages.test.ts` に次を足し、落ちることを確かめる：表の 6 か所それぞれについて、英語で決め手の数が 1 のとき単数の例、0 と 2 のとき複数の例になる（6 件、表の例の文どおり）／`status.listing` の先頭の「Listing streams…」と `insights.status.listing` は数に関わらず今と同じ／日本語は 6 か所とも今と同じ／決め手の数が文字列の `"1"` でも単数になる（`t` は数も文字列も受けるため）。（FR4.1.1〜FR4.1.4、FR4.1 の受け入れ基準）
- [ ] **Step 23：選び分けの仕組みを実装する（Green）。** `src/i18n/messages.ts` に `englishSingular`（6 つのキー → `{ count: 決め手の数の名前, text: 単数形の文 }`）を足し、`translate` で「英語で、そのキーに単数形があり、決め手の数が 1 なら単数形の文を使う」を 1 か所で行う。複数形の文（今の `en`）の中の動詞もそろっているか確かめる（`were`、`could not` はそのまま）。冒頭の説明に 1 段落足す。（FR4.1.5）
- [ ] **Step 24：単数形の表の確かめ。** `messages.test.ts` に、`englishSingular` のキーがすべて `messages` にあり、決め手の数の名前がその英語の文の `{…}` にあり、単数形の文と複数形の文が同じ置き場所を持つことを確かめるテストを足す。（FR4.1.5、NFR6）

### FR4.2 中断の知らせの読み上げ（GUI：実装してからテスト）

- [ ] **Step 25：囲みを先に置く。** `src/components/CancelNotice.tsx` を `notice: CancelNoticeView | null` を受ける形にし、`role="status"` の囲み（`data-testid="cancel-notice"`）は常に出し、中身（警告と文）は `notice` があるときだけ入れる。`src/App.tsx` は条件なしで `<CancelNotice notice={session.cancelNotice} t={t} />` を置く。囲みが空のときは見た目の余白が出ないようにする（空の `div`）。見た目と文は変えない。（FR4.2.1、FR4.2.2）
- [ ] **Step 26：テストを足す。** `CancelNotice.test.tsx` に、`notice` が `null` のとき囲みはあって中身が空／`null` から中断の知らせに替えると同じ囲みの要素の中に文が入る（`rerender` の前後で同じ要素であることを確かめる）を足す。今の 4 件はそのまま通す。`App.test.tsx` に、取得中から中断の知らせのある状態に替わっても `cancel-notice` の要素が同じであることを確かめる 1 件を足す。（FR4.2 の受け入れ基準）

### FR4.3 権限のテスト（境界：実装してからテスト）

- [ ] **Step 27：数の固定をやめる。** `src-tauri/src/lib.rs` の `the_manifest_the_capability_and_the_handler_list_the_same_commands` から `assert_eq!(manifest.len(), 32)` と数の経緯のコメントを消し、3 つの一覧がどれも空でないことと、3 つが同じことだけを確かめる。比べを `fn same_commands(manifest, handler, allowed) -> Result<(), String>`（テストのモジュールの中）に切り出す。（FR4.3.1、FR4.3.2）
- [ ] **Step 28：落ちることのテストを足す。** `same_commands` に、1 か所にだけ足した一覧・1 か所からだけ消した一覧・空の一覧を渡すと `Err` になるテストを 3 件足す。（FR4.3 の受け入れ基準）

### FR5 リージョンの名前と絞り込み（絞り込みはテスト先行、部品は実装後）

- [ ] **Step 29：名前を確かめる。** AWS の公式の資料（リージョンの一覧の英語版と日本語版）で、`STANDARD_REGIONS` の 34 個の英語と日本語の名前を確かめる。括弧の全角・半角も資料に合わせる（OQ3）。確かめた資料の場所を code-summary.md に書く。（FR5.1、A5）
- [ ] **Step 30：絞り込みと一致のテストを先に書く（Red）。** `src/i18n/regions.test.ts` を作り、まだない `regionLabel(code, locale)` と `filterRegionOptions(codes, text, locale)` のテストを書いて落ちることを確かめる：日本語で `ap-northeast-1 アジアパシフィック (東京)`、英語で `ap-northeast-1 Asia Pacific (Tokyo)`／表にないコードはコードだけ／「東京」「tokyo」「TOKYO」「northeast」で `ap-northeast-1` が残る（どちらの言語の画面でも、もう一方の言語の名前でも一致）／全角の括弧「（東京）」でも一致する／「zzz」で空／空の文字で全部が今の順で残る／表にないコードもコードで絞り込める。さらに FR5.10 の一致のテスト：`regions.rs` を `?raw` で読み、`STANDARD_REGIONS` の文字列を取り出して（取り出せない・0 件なら落とす）、名前の表のコードの集まりと同じであることを確かめる。（FR5.3〜FR5.5、FR5.10、A7、R-08）
- [ ] **Step 31：名前の表と絞り込みを実装する（Green）。** `src/i18n/regions.ts` に `REGION_NAMES`（コード → `{ en, ja }`、Step 29 で確かめた 34 個）、`regionLabel`、`filterRegionOptions`（大文字・小文字と括弧の全角・半角の違いを吸収した部分一致）を書く。（FR5.1、FR5.1.1、FR5.4、FR5.5）
- [ ] **Step 32：選択欄を替える。** `src/components/TopBar.tsx` のリージョンの `ButtonDropdown` を Cloudscape の `Select`（`filteringType="manual"`）に替える：候補は `session.regions` の順で `regionLabel` の文字、打った文字で `filterRegionOptions`、選んでいるものは `selectedOption`（閉じているときも同じ文字）、未選択は `placeholder={t("connection.region.placeholder")}`、無効は今と同じ `connectionDisabled`、`ariaLabel`／`filteringAriaLabel` は `connection.region.label`、候補なしは `noMatch`／`empty` に `connection.region.noMatch`、選んだら `onSelectRegion(code)`（確認ダイアログの流れは呼び出し側のまま）。`data-testid="top-bar-region"` は残す。上の帯の中の幅は `src/styles.css` で今のボタンに近づける。冒頭の説明を直す。プロファイルとタイムゾーンは替えない。（FR5.2、FR5.3、FR5.6〜FR5.9、A6）
- [ ] **Step 33：新しい文を足す。** `src/i18n/messages.ts` に `connection.region.noMatch`（英語「No matching regions」、日本語「一致するリージョンはありません」）と、必要なら `connection.region.filterPlaceholder` を英語と日本語で足す。既存の「両方の言語がそろっている」テストが通ることを確かめる。（FR5.6、NFR6）
- [ ] **Step 34：選択欄のテストを直して足す。** `src/components/TopBar.test.tsx` の、リージョンの `ButtonDropdown` を前提にしたテストを `Select` に合わせて直し、次を足す：日本語の画面で開くと `ap-northeast-1 アジアパシフィック (東京)` の形でコードの順に並ぶ／英語の画面の形／「tokyo」を打つと `ap-northeast-1` だけが残る／「zzz」で候補なしの文／選んでいるものが閉じた状態で名前つきで出る／未選択で「Choose a region」／`canChangeConnection` が偽のときと SSO のサインイン中は無効／候補を選ぶと `onSelectRegion` がコードで呼ばれる。`App.test.tsx` のリージョンを選ぶ流れ（確認ダイアログを含む）のテストを新しい部品の操作に合わせて直し、振る舞いの期待値は変えない。（FR5 の受け入れ基準、FR5.9）

### 仕上げ

- [ ] **Step 35：書式と検査。** `cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit` を流して通す。テスト以外のコードに `unwrap()`／`expect()` を足していないことを確かめる。（NFR7）
- [ ] **Step 36：全体のテストとカバレッジ。** `cargo test --workspace` と `npx vitest run` を流して通し、`npm run test:coverage` で画面の行カバレッジが 80% 以上であることを確かめる。（NFR1、NFR3）
- [ ] **Step 37：呼ぶ API の確かめ。** 変えたコードに AWS の SDK の新しい呼び出しがないこと（`CloudWatchLogsGateway` の 3 つ以外を呼ばない）と、リージョンの名前をネットワークから取っていないことを確かめる。（NFR5、project.md Forbidden）
- [ ] **Step 38：記録。** code-summary.md と traceability.json を書く（FR1〜FR5・NFR1〜NFR7 の対応、200 回の結果、リージョンの名前の資料、計画との違い）。（Documentation and traceability）

## 要件との対応

| 要件 | 手順 |
|---|---|
| FR1.1〜FR1.3 | Step 3〜5 |
| FR2.1.1〜FR2.1.5 | Step 6〜9 |
| FR2.2.1〜FR2.2.5 | Step 10〜12 |
| FR2.3.1〜FR2.3.5 | Step 13〜16 |
| FR3.1 | Step 17 |
| FR3.2 | Step 18、19 |
| FR3.3 | Step 20 |
| FR3.4 | Step 21 |
| FR4.1.1〜FR4.1.5 | Step 22〜24 |
| FR4.2.1、FR4.2.2 | Step 25、26 |
| FR4.3.1、FR4.3.2 | Step 27、28 |
| FR5.1〜FR5.10 | Step 29〜34 |
| NFR1、NFR3 | Step 2、16、36 |
| NFR2 | Step 4、6、9、10、12、22、26、28、30、34 |
| NFR4 | Step 36（`cargo test --release -p local-sights-core --lib -- --ignored` も流す） |
| NFR5 | Step 37 |
| NFR6 | Step 24、33 |
| NFR7 | Step 35 |

## 範囲外（この計画ではしないこと）

版を上げること・タグ・配ること、Windows の手元の確認（A1）、キャッシュのファイルの形とログの画面の照合の変更（A4）、確認用プログラムの `limit 0` の振る舞いの変更、経過時間の番号の比べ方の変更、ストリーム以外の数の英語の単数・複数（A8）。
