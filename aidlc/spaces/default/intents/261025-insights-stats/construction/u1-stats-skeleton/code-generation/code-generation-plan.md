# Code Generation Plan — U1 薄い一本（u1-stats-skeleton）

## Summary

- Builds: `stats`（6 つの書き方・by・bin()）の読み取りと集計、形つきの QueryResult、集計の行の表示・詳細・書き出しを、中核から画面まで通す薄い一本
- Touches: `crates/local-sights-core/src/query/`（plan・parser・新規 stats.rs・evaluator・store・runner）、`insights_session.rs`、`export/exporter.rs`、`src-tauri/src/lib.rs`、`src/api.ts`・`QueryResultTable.tsx`・`LogAnalyticsPage.tsx`・`i18n/messages.ts`、`examples/compat/`・`fetch_check.rs`
- Tests: Rust の単体テスト 約 45（parser の stats と bin の字句 8、StatsAggregator 12、丸め 8、evaluator 6、insights_session 6、exporter 3、compat の型合わせ 2）と Vitest 6（QueryResultTable の 2 つの形）

上流：`construction/u1-stats-skeleton/functional-design/`（entities.md・rules.md の BR1.1〜BR5.2・functional-spec.md の UC1〜UC7）、`inception/units-generation/unit-of-work.md`（U1）、`inception/requirements-analysis/requirements.md`。要件の段の追跡（ストーリーはない）として、各手順に BR と FR を書く。

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

Testing Contract の custom の順番に従う。純粋なロジック（読み取り、集計、丸め、bin の区切り、評価の段、表示の文字）はテストを先に書いてから実装し、Tauri・画面・確認用プログラムの境界の層は先に実装してからテストを書く。既存のテストは期待値を変えずに通す（BR5.1）。テスト以外のコードでは `unwrap()` / `expect()` を使わない。

## 手順

### 準備

- [x] **Step 1** — テストの実行の確認：`cargo test -p local-sights-core --lib query::`、`cargo test -p local-sights-core --lib insights_session`、`cargo test -p local-sights-core --lib export::`、`cargo test -p local-sights --lib`、`cargo test -p local-sights-core --example compat_check`、`npx vitest run src/components/QueryResultTable.test.tsx` が今の状態で通ることを確かめ、結果を記録する（unit-test-instructions.md の基準の数）
- [x] **Step 2** — 置き場所の用意：`crates/local-sights-core/src/query/stats.rs` を新しく作り、`query/mod.rs` に `pub mod stats;` を足す（中身は Step 6 から）

### 計画の型と読み取り（純粋なロジック：テスト先行）

- [x] **Step 3** — `plan.rs` のテストを先に書く：`CommandKind::Stats(StatsCommand)` の形、`AggregateSpec`・`AggregateArgument`・`GroupKey`・`BinPeriod`、`QueryPlan::result_shape()`（stats があれば aggregates）と `aggregate_columns()`（鍵 then 集計、BR3.3）。実行して失敗を確かめる（BR3.1・BR3.3、FR3.1）
- [x] **Step 4** — `plan.rs` を実装し、Step 3 のテストを通す（`has_stats()`、`stats()` の取り出しも）
- [x] **Step 5** — `parser.rs` のテストを先に書く：`stats count(*) by bin(5m)`、`stats avg(latency) as a, max(latency) as m`、`by` なし、複数の鍵、`as` なしの列の名前（元の文の切り出し・空白をそろえる、BR1.9）、`bin(5m)`・`bin(30s)`・`bin(1d)` の字句（BR1.8）、`bin(5ms)`/`bin(1w)` は Unsupported、`bin(0m)`・`bin(1000001m)` は Syntax、`count_distinct(x)` などは Unsupported（BR1.2）、`sum(*)` は Syntax（BR1.3）、`by` のあとに何もない・閉じていない括弧・引き数の数の違い・`,` のあと何もない・`stats by level`（BR1.7）、列の名前の重複（BR1.4）、`stats` のあとの命令と 2 つ目の `stats` は Unsupported（BR1.6）、`filter @timestamp > 5m` の今のエラーは変わらない。実行して失敗を確かめる（FR1.1・FR1.3・FR2.1・FR6.3）
- [x] **Step 6** — `parser.rs` を実装し、Step 5 のテストを通す：`command()` の振り分けに `stats` を足し（`OTHER_COMMANDS` から外す）、集計・引き数・鍵・`as`・`bin(` の中を 1 語で読む字句を足す。`fields … as` の Unsupported は変えない。新しい問題の鍵を `keys` に足す

### 集計（純粋なロジック：テスト先行）

- [x] **Step 7** — `stats.rs` のテストを先に書く：`StatsAggregator::new/add/finish`。グループ化（鍵の値の文字で比べる、数の 200 と文字の "200" は同じ、Missing は 1 グループで最後、BR2.1・BR2.7）、count(*)・count(項目)（空の文字も数える、BR2.2）、sum・avg（数に読める文字は足す、真偽値と Missing は無視、足せなければ値なし、BR2.3）、min・max（NaturalSort の順、混ざるとき、Missing は無視、BR2.4）、bin のエポックの倍数（負の時刻も床、BR2.5）、by なしの 0 件は 1 行・by ありの 0 件は 0 行（BR2.6）、鍵の昇順（BR2.7）。実行して失敗を確かめる（FR1.2・FR1.3・FR1.5・FR1.6・FR2.1〜FR2.3）
- [x] **Step 8** — `stats.rs` の文字の丸めのテストを先に書く：`format_aggregate`（4 桁に四捨五入、末尾の 0 を落とす、-0 は 0、2^53 以上の整数、1e20、0.1 + 0.2、0.00005、-0.00004、inf・NaN は値なし、BR2.8）。実行して失敗を確かめる（FR1.5、A7）
- [x] **Step 9** — `stats.rs` を実装し、Step 7・8 のテストを通す：`CellValue`（missing・integer・decimal・text・timestamp）、`Accumulator`、`AggregateGroup`、`format_aggregate`

### 評価と結果の形（純粋なロジック：テスト先行）

- [x] **Step 10** — `evaluator.rs` のテストを先に書く：`filter ... | stats count(*)` が filter で残ったイベントだけを数える、Aggregate の段が 10,000 行ずつ進み進み具合が 0 のままにならない、止める信号で結果を捨てる、aggregates では既定の 10,000 件の打ち切りと Columns の段を行わない、`EvalOutput::Aggregates` の列と行、stats のない計画は今と同じ `EvalOutput::Events`。実行して失敗を確かめる（BR2.9・BR3.1・BR3.2・BR3.5、FR1.8・FR3.6）
- [x] **Step 11** — `evaluator.rs` を実装し、Step 10 のテストと既存のテストを通す：`Stage::Aggregate`、`EvalOutput` を列挙型に、`ResultColumns`（Events / Aggregates、BR3.6）、`ResultRow` を列挙型（Event / Aggregate）に、`QueryResult` に `shape`、`row_cells` は Events のまま
- [x] **Step 12** — `store.rs`・`runner.rs` の型の追従（BR3.1・BR3.5）：`copy_rows` は `ResultRow::Event` の写しを返す、`QueryRunner` は `EvalOutput::Events` なら今のとおり `copy_rows`、`Aggregates` なら行をそのまま `QueryResult` にする（流れの順番は変えない）。既存のテストを通し、aggregates の道のテストを 2 つ足す

### 表示と書き出し（中核の境界：実装してからテスト）

- [x] **Step 13** — `insights_session.rs` を実装：`ResultSummary.shape`、`display_rows` は aggregates なら `values` だけ（event・displayTime・logGroupName なし）、時刻のセルは `format_display_in_zone` で整形（BR4.1・BR4.2）、`display_row_detail` は aggregates なら列の（名前, 文字）（BR4.3）、`export_source` は形つき（BR4.4）
- [x] **Step 14** — `insights_session.rs` のテストを書いて実行する：aggregates の見出し・行・詳細、タイムゾーンの切り替えで時刻の文字だけ変わる（FR2.3）、書き出しの元、0 行、events は今と同じ（FR3.1〜FR3.4）
- [x] **Step 15** — `export/exporter.rs` を実装：`ExportSnapshotSource::LogAnalytics` の `columns` を `ResultColumns` に、aggregates の行は表と同じ文字で書く。テストを書いて実行する（CSV と Markdown の集計の行、events は今と同じ、BR4.4、FR3.4）

### Tauri と画面（境界の層：実装してからテスト）

- [x] **Step 16** — `src-tauri/src/lib.rs` を実装：`TauriQuerySink` と `get_query_rows` / `get_query_row_detail` の値の形の追従（コマンドの名前と引き数は変えない、新しいコマンドはない）。`cargo test -p local-sights --lib` を実行する
- [x] **Step 17** — `src/api.ts` を実装：`ResultSummary.shape`、`QueryRow` の `event` などを任意に
- [x] **Step 18** — `src/components/QueryResultTable.tsx`・`LogAnalyticsPage.tsx` を実装：表の部品は見出しの `shape` で分岐する。aggregates では列の名前で表示を変えない（`by @message` の鍵の列も全文、`by @timestamp` の見出しにタイムゾーンを付けない）、行の詳細は列の（名前, 文字）、列が多いときの横スクロール、集計の 0 行の表示（BR4.5、FR3.5・FR3.6）。`data-testid` を足す
- [x] **Step 19** — `src/i18n/messages.ts` に新しい文言を英語と日本語で足す：集計の 0 行、「n 行」、stats の読み取りのエラー（BR4.6、NFR9）
- [x] **Step 20** — Vitest のテストを書いて実行する（`QueryResultTable.test.tsx`）：aggregates の見出しと行、`by @message` が全文、`by @timestamp` の見出しにタイムゾーンなし、行の詳細、0 行、events は今と同じ

### 確認用プログラム（境界の層：実装してからテスト）

- [x] **Step 21** — `examples/compat/format.rs`・`examples/compat_check.rs` の型の追従：events の結果は今までどおり比べられ、aggregates の結果は「U4 で対応」と報告して比べない。テストを 2 つ足して実行する
- [x] **Step 22** — `examples/fetch_check.rs` を広げる：引き数でクエリの文を受け取り、stats の結果なら見出しの列と行（表と同じ文字）を標準出力に表で出し、取得の終わりから最終結果までの時間を出す（手元の確かめ用。AWS への呼び出しは今の取得のまま）

### 仕上げ

- [x] **Step 23** — 全体の確認：`cargo fmt --check`、`cargo clippy --workspace`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、unit-test-instructions.md のすべてのコマンドが通ることを確かめる
- [x] **Step 24** — ドキュメント：README の対応する範囲に「`stats`（U1 の関数：count・sum・avg・min・max、`by`、`bin()`）」を一時的に書き足す（一覧の一元化と表の作り直しは U2）

## 追跡（要件・決まり → 手順）

| 要件 | 決まり | 手順 |
|---|---|---|
| FR1.1 | BR1.1・BR1.4・BR1.9 | Step 5・6 |
| FR1.2（6 つの書き方） | BR1.2・BR2.2〜BR2.4 | Step 5〜9 |
| FR1.3 | BR1.3・BR2.1・BR2.6 | Step 5〜9 |
| FR1.4（U1 の範囲） | BR1.6・BR3.2 | Step 5・6・10・11 |
| FR1.5 | BR2.8・BR4.2 | Step 8・9・13・14 |
| FR1.6（仮の決まり） | BR2.1・BR2.3・BR2.4 | Step 7・9 |
| FR1.8 | BR3.5・BR5.2 | Step 10〜12 |
| FR2.1〜FR2.3 | BR1.5・BR1.8・BR2.5・BR4.2 | Step 5〜9・13・14 |
| FR3.1〜FR3.6 | BR3.1〜BR3.6・BR4.1〜BR4.6 | Step 10〜20 |
| FR6.3 | BR1.7 | Step 5・6 |
| C3・C5 | BR5.1 | Step 12・16・21・23 |
