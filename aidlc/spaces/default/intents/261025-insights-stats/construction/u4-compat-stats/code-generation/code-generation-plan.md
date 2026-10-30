# Code Generation Plan — U4 互換の確かめ（u4-compat-stats）

## Summary

- Builds: 手元の確認用プログラム（compat_check）で集計の結果をコンソールと比べる仕組み（組の 5 つ目の欄の緩め、keyorder、集計の行の Row、緩めのない列で対にする比べ方、範囲内の違いと仮の決まりのまとめの報告）と、集計のクエリの組（CloudTrail の s01〜、数の組 n01〜）、U3 のレビューの持ち越し
- Touches: `crates/local-sights-core/examples/compat_check.rs`・`examples/compat/`（queries・format・compare・report）・`examples/compat_queries.txt`・新規 `examples/compat_queries_numeric.txt`、持ち越しで `src/components/QueryResultTable.tsx`・`ExportNoticeAlert.tsx`・`App.tsx`、`README.md`（手元の比べ方の手順）
- Tests: Rust の examples のテスト 約 35（組の読み方 7、集計の Row 5、小数と近似の範囲 8、対にする比べ方 6、keyorder 3、報告 5、集計の打ち切りと並べ替えの段 4）と Vitest 約 4（持ち越し）。コンソールとの比較そのものは開発者の手元で行う（自動テストと CI は AWS に接続しない）

上流：`construction/u4-compat-stats/functional-design/`（entities.md・rules.md の BR1.1〜BR3.2・functional-spec.md の UC1・UC2）、`inception/units-generation/unit-of-work.md`（U4）、`inception/requirements-analysis/requirements.md`（FR7.1〜FR7.3、NFR3・NFR4）、U1・U2 の機能設計の仮の決まり。ストーリーはないため、各手順に BR と FR を書いて追う。U2 のコード生成のレビュー R-02（値のない列への `!=`）は比べる組に入れる（Step 6）。U3 のコード生成のレビューの低い所見のうち R-02・R-05 は、決まり（レビューのあとの修正は次の作業単位に回す）に従ってこの作業単位で直す（Step 13〜15）。U3 の R-01（途中の view を作る間の store の鍵）は NFR1 の手元の計測の結果で扱いを決める（Build and Test で記録する）。

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

Testing Contract の custom の順番に従う。比べる処理（組の読み方、集計の Row、小数と近似の範囲、対にする比べ方、keyorder、報告）は純粋なロジックなので、見本のデータでテストを先に書いて失敗を確かめてから実装する。compat_check.rs の取得と評価のつなぎ（境界）と画面の持ち越しは、先に実装してからテストを書く。既存の compat のテスト（events の比べ方）は期待値を変えずに通す。自動テストと CI は AWS に接続しない。コンソールから書き出した結果はリポジトリに置かない（組のファイルにはクエリの文だけ）。テスト以外のコードでは `unwrap()` / `expect()` を使わない。

コンソールとの比較の実行（UC1・UC2 の手元の作業）と、そこで見つかった仮の決まりの違いの直し（BR3.1）は、開発者の手元の実行の結果を受けて行う。この計画では、比べるための道具と組と手順書を作り、手元の結果を受けて直すための記録の欄を用意する（Step 12）。

## 手順

### 準備

- [ ] **Step 1** — テストの実行の確認：unit-test-instructions.md のコマンドが今の状態で通ることを確かめ、通過の数を記録する

### 組の読み方（純粋なロジック：テスト先行）

- [ ] **Step 2** — `examples/compat/queries.rs` のテストを先に書く：今の 4 欄の行が今のとおり読める（`splitn(5)` に変えても同じ）、5 つ目の欄の `列の名前=decimal`・`列の名前=approximate` を `;` で区切って読む、列の名前の空白の扱い、読めない欄（種類の誤り、`=` がない、空の名前）はその行のエラー、`keyorder` の mode を読む、`# provisional: U1:BR2.1 = s03` の注記を読む（BR1.3・BR1.4・BR2.8）。実行して失敗を確かめる（FR7.1・FR7.3）
- [ ] **Step 3** — `queries.rs` を実装し、Step 2 のテストを通す：`ColumnTolerance`、`CompareMode::KeyOrder`、`ProvisionalRuleCheck` の読み取り

### 比べ方（純粋なロジック：テスト先行）

- [ ] **Step 4** — `examples/compat/format.rs` と `compare.rs` のテストを先に書く：
  - 集計の行の Row：列の名前と表と同じセルの文字、時刻のセルは `insights_timestamp` の UTC の書き方（BR2.1・BR2.7 (1)）。
  - 小数の範囲：|a − b| ≤ 0.0001 または ≤ 1e-6 × max(|a|, |b|) なら withinDecimal、同じ文字は exact、片方が数でなければ文字で比べる、範囲を超えれば different（BR2.2）。
  - 近似の範囲：整数どうしで ≤ 2% なら withinApproximate（BR2.3）。
  - 対にする比べ方：緩めのある列がなければ今の only_in のまま、緩めのある列があれば緩めのない列の (名前, 値) を鍵に対にする、同じ鍵が複数（by なしの 1 行を含む）なら緩めのある列の昇順に対にする、片側にしかない鍵は only_in（BR2.6）。
  - 順番の段・打ち切りの同点の段で decimal の列の値が範囲内なら同じ順位（BR2.2）。
  - keyorder：行の集まりのあとに鍵の列の値の並びを比べ、最初に食い違った位置を出す（BR2.8）。
  - 集計の打ち切り：sort のない集計が limit か既定の 10,000 行で切られたら Skipped、sort のある集計は同点の段（BR2.4）。
  - events の今の比べ方が変わらないこと。
  - 実行して失敗を確かめる（FR7.1）
- [ ] **Step 5** — `format.rs` と `compare.rs` を実装し、Step 4 のテストと既存のテストを通す：`CellDifference`（exact・withinDecimal・withinApproximate・different）

### 組（データ）

- [ ] **Step 6** — `examples/compat_queries.txt` に集計のクエリ s01〜 を足す（BR1.1 に挙げたもの、order・set・keyorder の使い分け、count_distinct の列に approximate）。U2 のレビュー R-02 の分として、値のない列への `!=` を確かめる `stats count(*) as n by errorCode | filter errorCode != "AccessDenied"` を足す。冒頭に、クエリの文にタブを書けないことと、`# provisional:` の対応の注記を書く（rules.md の「仮の決まりと確かめるクエリ」の表と同じ。U2:BR1.4 の `!=` も足す）。@log を鍵にするクエリと日時の文字との比較は入れない（BR3.2）。既定の 10,000 行（U2:BR3.2）は notChecked と書く
- [ ] **Step 7** — 新しい `examples/compat_queries_numeric.txt` を作る（BR1.2 のクエリ n01〜、項目の名前は仮に `latency`・`status`、小数の列に decimal、冒頭に「自分のロググループの項目の名前に書き換えて使う」の注記と、pct の百分位の小数の上限の注記）
- [ ] **Step 8** — 組のファイルが読めることのテストを足して実行する：2 つの組を `queries.rs` で読み、行のエラーがないこと、id が重ならないこと、`# provisional:` の対応のクエリの id がすべて組にあること

### 報告（純粋なロジック：テスト先行）

- [ ] **Step 9** — `examples/compat/report.rs` のテストを先に書く：クエリごとの「同じ」「同じ（小数の誤差の範囲：n セル）」「同じ（近似の範囲：n セル）」「違い」、範囲内の違いの例は最初の 5 つ、仮の決まりごとのまとめ（一致・違い・比べていない）、期間の端の違いを分けて出す、終了コード（違いがあれば 1、範囲内の違いだけなら 0）（BR2.5・BR1.4・BR2.7 (5)）。実行して失敗を確かめる
- [ ] **Step 10** — `report.rs` を実装し、Step 9 のテストを通す

### つなぎ（境界：実装してからテスト）

- [ ] **Step 11** — `examples/compat_check.rs` を実装：アプリの結果が aggregates なら集計の AppRows を作る（列の名前は `ResultColumns::Aggregates`、セルは表と同じ文字、時刻は `insights_timestamp`）、並べ替えの項目は stats のあとの sort の鍵の列の名前、`row_cap` は stats のあとの limit（なければ 10,000）、打ち切りは `matched_count` と表に出した行の数から求め、同点の段は最後の行と同じ値の行を両側から除いて件数だけ比べる、bin の鍵の列が期間の最初か最後の区切りの行は「期間の端の違い」として分ける（BR2.7）。U1 で「U4 で対応」と報告していた分岐を消す。テストを書いて `cargo test -p local-sights-core --example compat_check` を実行する（見本の結果で、集計の道・打ち切り・期間の端）
- [ ] **Step 12** — 手元の比べ方の手順を、今の手順がある `README.md` の節に書き足す（今の手順に、集計の組と数の組の選び方、`QUERIES=` と `--queries`、数の組の項目の名前の書き換え、数の項目を持つロググループがないときは `avg(toMillis(@timestamp))` の 1 本で代えること、結果の記録の仕方を足す）。U4 の記録 `code-summary.md` に、仮の決まりごとの手元の比較の結果（matches・differs・notChecked）と、differs のときに直したか諮ったかを書く表の欄を用意する（BR3.1。手元の結果は開発者が実行したあとに埋め、直しが要れば BR3.1 のとおり U4 の中で直すか諮る）

### 持ち越し（U3 のコード生成のレビュー）

- [ ] **Step 13** — `src/components/QueryResultTable.tsx`：新しい版が届いてから `findQueryRow` の答えで開いた行の位置が付け替わるまで、開いた行の詳細の取得を待つ（古い位置で別の行の詳細を取りに行かない、U3 レビュー R-02）
- [ ] **Step 14** — `src/App.tsx`・`ExportNoticeAlert.tsx`：書き出しの通知の添え書き（一部のログだけの結果）を、書き出しを始めたときの結果の stage で決めて覚え、次の実行が始まっても消えないようにする（U3 レビュー R-05）
- [ ] **Step 15** — 持ち越しの Vitest を書いて実行する：付け替えまで詳細を取りに行かない、付け替えのあとに正しい行の詳細を取る、通知の添え書きが次の実行で消えない、中断でない結果では添え書きがない

### 仕上げ

- [ ] **Step 16** — 全体の確認：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、unit-test-instructions.md のすべてのコマンド、`cargo test --workspace` と Vitest 全体が通ることを確かめる。組のファイルとリポジトリに、コンソールから書き出した結果やログの中身、アカウント ID などがないことを確かめる（NFR4・project.md Forbidden）

## 追跡（要件・決まり → 手順）

| 要件 | 決まり | 手順 |
|---|---|---|
| FR7.1（集計の結果を比べる） | BR1.3・BR2.1〜BR2.7 | Step 2〜5・9〜11 |
| FR7.2（集計のクエリの組） | BR1.1・BR1.2 | Step 6〜8 |
| FR7.3（仮の決まりの確かめと直し） | BR1.4・BR2.8・BR3.1・BR3.2 | Step 2・3・6・9・10・12 |
| NFR3・NFR4（手元で動かす、AWS に接続しない、書き出しを置かない） | — | Step 6・7・12・16 |
| U2 レビュー R-02 | — | Step 6 |
| U3 レビュー R-02・R-05 | — | Step 13〜15 |
