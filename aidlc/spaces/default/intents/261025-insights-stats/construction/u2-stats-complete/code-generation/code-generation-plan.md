# Code Generation Plan — U2 `stats` をそろえる（u2-stats-complete）

## Summary

- Builds: 残りの集計の関数（count_distinct・pct・stddev・earliest・latest・sortsFirst・sortsLast）、`stats` のあとの `filter`・`sort`・`limit` と既定 10,000 行の打ち切り、命令と関数の一覧の一元化（QueryCatalog）と、それに合わせた補完・色分け・エラー文言・README の一致のテスト
- Touches: `crates/local-sights-core/src/query/`（新規 catalog.rs・plan・parser・stats・evaluator）、`insights_session.rs`、`src-tauri/src/lib.rs`・`capabilities/default.json`・`permissions/`・`build.rs`、`src/api.ts`・`analytics.ts`・`QueryEditor.tsx`・`QueryResultTable.tsx`・`LogAnalyticsPage.tsx`・`i18n/messages.ts`、`README.md`
- Tests: Rust の単体テスト 約 55（QueryCatalog 6、parser 12、StatsAggregator の 7 つの関数 16、pct の順位の表 8、evaluator の stats のあと・打ち切り 8、insights_session 4、README の一致 1）と Tauri 2、Vitest 約 12（色分け・文言・打ち切りの注記・権限の一致）、手で動かす測定のテスト 1（`#[ignore]`）

上流：`construction/u2-stats-complete/functional-design/`（entities.md・rules.md の BR1.1〜BR5.1・functional-spec.md の UC1〜UC4）、`inception/units-generation/unit-of-work.md`（U2）、`inception/requirements-analysis/requirements.md`（FR1.2・FR1.4・FR1.6・FR1.7・FR6.1〜FR6.3、NFR2・NFR7）、`inception/domain-design/decisions.md`（ADR-002・ADR-005）。ストーリーはないため、各手順に BR と FR を書いて追う。U1 の型（StatsCommand・AggregateSpec・CellValue・StatsAggregator・ResultColumns・ResultRow）をそのまま広げる。

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

Testing Contract の custom の順番に従う。純粋なロジック（一覧、読み取り、7 つの関数の累積、pct の順位、stats のあとの命令、打ち切り、README の一致）はテストを先に書いて失敗を確かめてから実装する。Tauri・画面の境界の層は先に実装してからテストを書く。U1 と前の作業のテストは期待値を変えずに通す。U1 で「対応していない」とした書き方（U2 の関数、stats のあとの filter・sort・limit）のテストだけは、U2 の振る舞いに合わせて直す（BR5.1）。テスト以外のコードでは `unwrap()` / `expect()` を使わない。AWS の呼び出しは増やさない。

## 手順

### 準備

- [x] **Step 1** — テストの実行の確認：unit-test-instructions.md のコマンドが今の状態で通ることを確かめ、通過の数を記録する
- [x] **Step 2** — 置き場所の用意：`crates/local-sights-core/src/query/catalog.rs` を新しく作り、`query/mod.rs` に `pub mod catalog;` を足す（中身は Step 4 から）

### 一覧（純粋なロジック：テスト先行）

- [x] **Step 3** — `catalog.rs` のテストを先に書く：`QueryVocabulary`（name・kind・supported・reserved）と `QueryCatalog` の固定の一覧。supported の命令は fields・filter・sort・limit・stats、unsupported の命令は parse・dedup・display・pattern・diff・unmask・unnest・lookup・filterIndex・source、集計の関数は 12、ほかの関数は toMillis・bin、キーワードは by・as・and・or・not・like・in・asc・desc で reserved は and・or・not・like・in の 5 つだけ、決まった項目は 4 つ、名前の重複がないこと、命令の引き方は大文字・小文字を区別しないこと。実行して失敗を確かめる（BR4.1、FR6.2）
- [x] **Step 4** — `catalog.rs` を実装し、Step 3 のテストを通す（画面へ渡す形は `serde::Serialize`、名前は camelCase）

### 計画の型と読み取り（純粋なロジック：テスト先行）

- [x] **Step 5** — `plan.rs` と `parser.rs` のテストを先に書く：
  - 関数：12 の関数名がそれぞれ読めること（BR1.1）、`sortsfirst` は Unsupported（BR1.1）。
  - pct：`pct(latency, 95)`・`pct(latency, 99.9)`・`pct(latency, 0)`・`pct(latency, 100)` が読め、百分位を（分子, 小数の桁）で持つこと、`pct(latency, 101)`・`pct(latency, -1)` は Syntax の PCT_OUT_OF_RANGE、`pct(latency)` と `count_distinct(a, b)` は引き数の数の違い、`stddev(*)` は Syntax（BR1.2）、as なしの列の名前 `pct(latency, 95)`（BR1.3）。
  - stats のあと：`stats count(*) as n by level | filter n > 10 | sort n desc | limit 5` が `StatsCommand.after` に書いた順に入ること、`` `count(*)` `` のバッククォートで列を引けること、列どうしの比較、ない列の名前は Syntax の UNKNOWN_RESULT_COLUMN、`limit` の上限は今と同じ（BR1.4）。stats のあとの `fields` は FIELDS_AFTER_STATS、2 つ目の `stats` は SECOND_STATS の Unsupported（BR1.5）。
  - 一覧からの振り分け：unsupported の命令は今と同じ問題になること、`desc`・`by` という名前の項目を裸で書いても今のとおり項目として読むこと（BR4.1）、`filter @timestamp > 5m` など前の作業のエラーが変わらないこと（BR5.1）。
  - 実行して失敗を確かめる（FR1.2・FR1.4・FR6.2・FR6.3）
- [x] **Step 6** — `plan.rs` と `parser.rs` を実装し、Step 5 のテストを通す：`AggregateFunction` を 12 に、`AggregateSpec.percentile`、`StatsCommand.after`（`AfterStatsCommand`：filter・sort・limit）、`FieldRef::Column(index)` と列の名前の引き方。`OTHER_COMMANDS`・`KEYWORDS`（reserved）・関数の振り分けを QueryCatalog から作り、U1 の暫定の表を消す。新しい問題の鍵（PCT_OUT_OF_RANGE・UNKNOWN_RESULT_COLUMN・FIELDS_AFTER_STATS・SECOND_STATS など）を `keys` に足す。U1 で U2 の関数や stats のあとの命令を「対応していない」と確かめていたテストは、U2 の振る舞いに合わせて直す（BR5.1）

### 集計（純粋なロジック：テスト先行）

- [x] **Step 7** — `stats.rs` の 7 つの関数のテストを先に書く：
  - count_distinct：値の文字が同じものを 1 つに数える（数の 200 と文字の "200" は同じ）、Missing は無視、真偽値は 1 / 0 の文字で数える（BR2.1）。
  - stddev：Welford の標本標準偏差（n − 1）、値 1 つで 0、0 個で値なし、大きな値（1e9 前後）で桁落ちしないこと（BR2.3）。
  - sortsFirst・sortsLast：min・max と同じ値、文字と数が混ざるとき（BR2.4）。
  - earliest・latest：view の順で最初・最後に見た値、Missing のイベントは飛ばす、同じ時刻なら並びの先・後（BR2.5）。
  - by ありで複数の関数を同時に使うときの列の順（U1 の BR3.3）。
  - 実行して失敗を確かめる（FR1.2・FR1.6）
- [x] **Step 8** — `stats.rs` の pct の順位の表のテストを先に書く：p = 0・7・50・99.9・100 と n = 1・100・101 の組で、k = max(1, ceil(a × n ÷ (100 × 10^d))) 番目の値になること、n = 0 は値なし、数に読めない値は数えないこと（BR2.2）。実行して失敗を確かめる（FR1.2）
- [x] **Step 9** — `stats.rs` を実装し、Step 7・8 のテストを通す：`Accumulator` に関数ごとの累積（distinct の集まり・Welford の mean と m2・pct の値の一覧・extreme・chosen）を足す。pct の順位は u128 の整数で求め、並べるのはグループごとに finish のとき 1 回

### 評価の仕上げ（純粋なロジック：テスト先行）

- [x] **Step 10** — `evaluator.rs` のテストを先に書く：
  - stats のあとの命令：filter は CellValue を FieldValue に写して今の比べ方（比較・and/or/not・like・正規表現・in、Missing はどれにも合わない）、timestamp の列は @timestamp と同じ扱い、sort は NaturalSort で同じ値は前の並びのまま、limit 0 は 0 行、limit が複数あれば書いた順にすべて（BR3.1・BR1.4）。
  - 打ち切り：limit なしで 10,001 グループなら 10,000 行で truncatedAt = 10,000 と matchedCount = 10,001、limit が残りより小さいときの truncatedAt、limit のあとの filter では数え直さない、0 行は truncatedAt なし（BR3.2、FR1.7）。
  - 止める信号：仕上げの途中（10,000 行ごと）で止まると結果を捨てる（BR3.1）。
  - stats のないクエリと U1 の stats のクエリの結果が今と同じ（BR5.1）。
  - 実行して失敗を確かめる（FR1.4・FR1.7）
- [x] **Step 11** — `evaluator.rs` を実装し、Step 10 のテストと既存のテストを通す：Aggregate の段の仕上げで `StatsCommand.after` を書いた順に適用し、10,000 行ごとに止める信号を確かめ、events と同じ判定で打ち切る。U1 の「aggregates は打ち切らない」の分岐を改める
- [ ] **Step 12** — 測定のテストを 1 つ足す（`#[ignore]`、手で `--ignored` を付けて流す）：作ったイベント 100 万件（実際の AWS は使わない）で、`stats count_distinct(@message), pct(latency, 99) by @logStream` と U1 の `stats count(*) by bin(5m)` の評価の時間を測って出し、3 倍以内であることを確かめる。満たさないときは BR2.6 の手（count_distinct を値の文字のハッシュで持つ、pct を数の配列だけにする）を試し、それでも満たさなければ記録して利用者に諮る（BR2.6、NFR2）

### 補完（中核の境界：実装してからテスト）

- [ ] **Step 13** — `insights_session.rs` を実装：`COMMAND_WORDS`・`KEYWORD_WORDS`・`FIXED_FIELD_WORDS` を QueryCatalog から作る（今の候補の文字、`toMillis(@timestamp)` などはそのまま）、集計の関数と bin を `CompletionKind::Function` の候補に足す、U1 で足した暫定の候補を消す（BR4.3・BR4.1）
- [ ] **Step 14** — `insights_session.rs` のテストを書いて実行する：候補に stats・by・as・12 の関数・bin が入る、今の候補の文字が変わらない、JSON の項目の名前が今のとおり入る、aggregates の打ち切りの見出し（truncatedAt）が画面へ渡る（BR4.3・BR3.2）

### Tauri と画面（境界の層：実装してからテスト）

- [ ] **Step 15** — `src-tauri/src/lib.rs` に `get_query_catalog` のコマンドを足す（固定の一覧を返し、AppState の鍵を取らない）。`build.rs` のコマンドの一覧、`capabilities/default.json`、`permissions/` に足す。今の権限の一致のテストの、名前を明示する一覧にこのコマンドを足し、テストを 2 つ足して `cargo test -p local-sights --lib` を実行する（BR4.2）
- [ ] **Step 16** — `src/api.ts` に `getQueryCatalog` と一覧の型を足す。`src/analytics.ts` は一覧の命令（supported と unsupported）・関数・キーワードから色分けの規則を作り、一覧が届くまでは色分けなしで動くようにする。`QueryEditor.tsx` は規則を受け取る（BR4.2）
- [ ] **Step 17** — `LogAnalyticsPage.tsx` は画面が初めて出たときに一覧を 1 回だけ取り、取れなければ色分けなしで動く。`i18n/messages.ts` の `insights.error.supportedList` を「使える命令：{commands}」の差し込みの文にし、`query.unsupported.function` を一覧に合わせて直す。新しい問題の鍵（Step 6）の文言を英語と日本語で足す（BR4.2・BR4.6 の続き、NFR9）
- [ ] **Step 18** — `QueryResultTable.tsx` に aggregates の打ち切りの注記を「行」に合わせた文言で出す（BR3.2）
- [ ] **Step 19** — Vitest のテストを書いて実行する：色分けの規則が一覧から作られる・一覧がないときは色分けなし（`analytics`・`QueryEditor`）、「使える命令」の文に supported の命令が差し込まれる・一覧がないときの文（`LogAnalyticsPage`）、aggregates の打ち切りの注記（`QueryResultTable`）、新しい文言の鍵が英語と日本語にそろう（`messages.test.ts`）、権限の一致（`commandRegistration.test.ts`）

### ドキュメント（純粋なロジック：テスト先行）

- [ ] **Step 20** — README と QueryCatalog の一致のテストを先に書く（中核のテスト。README を `include_str!` で読む）：Supported の Commands の行、Supported の Aggregate functions の行、Not supported の The other commands の行を先頭の語で見つけ、バッククォートの語から括弧と引き数を外した名前の集まりが、supported の命令・supported の集計の関数と bin・unsupported の命令と大文字・小文字まで一致すること。実行して失敗を確かめる（BR4.4、FR6.1、NFR7）
- [ ] **Step 21** — `README.md` を直して Step 20 のテストを通す：Commands の行に `stats` を足し、「Aggregate functions in `stats`」の行（12 の関数と `bin(…)`）を新しく置き、Not supported の最初の行から `stats` を外す。U1 で暫定に書き足した段落を、この表と、集計の行の並び・4 桁の丸め・既定 10,000 行の打ち切りの説明に置き換える（`toMillis` の行と表のそのほかの行の形は今のまま）

### 仕上げ

- [ ] **Step 22** — 全体の確認：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、unit-test-instructions.md のすべてのコマンドが通ることを確かめる。`cargo test --workspace` と Vitest 全体も通ることを確かめる

## 追跡（要件・決まり → 手順）

| 要件 | 決まり | 手順 |
|---|---|---|
| FR1.2（12 の関数） | BR1.1・BR1.2・BR2.1〜BR2.5 | Step 5〜9 |
| FR1.4（stats のあとの命令） | BR1.4・BR1.5・BR3.1 | Step 5・6・10・11 |
| FR1.6（仮の決まり） | BR2.1〜BR2.5 | Step 7〜9 |
| FR1.7（件数の上限） | BR3.2 | Step 10・11・14・18 |
| FR3.1（列の名前） | BR1.3 | Step 5・6 |
| FR6.1（README） | BR4.4 | Step 20・21 |
| FR6.2（一覧の一元化） | BR4.1〜BR4.3 | Step 3〜6・13〜17・19 |
| FR6.3（読み取りのエラー） | BR1.2・BR1.4・BR1.5 | Step 5・6・17 |
| NFR2（重いクエリ） | BR2.6 | Step 12 |
| NFR7 | BR4.4 | Step 20・21 |
| C5 | BR5.1 | Step 6・10・11・22 |
