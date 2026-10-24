# Code Generation Plan — U2 書き出しと JSON の整形（u2-export-and-json）

## Summary

- Builds: 両画面の［エクスポート］メニュー（CSV／Markdown × 保存／コピー）と、開いた行の JSON の字句を保った整形と「整形／元のまま」の切替
- Touches: 中核に `export/` モジュール（formatter・exporter）を新設し `session.rs`・`insights_session.rs`・`lib.rs` を広げる。アプリに `export_boundary.rs` を新設し 2 つの公式プラグインを足す。画面に `jsonPretty.ts`・`ExportMenu.tsx`・`ExportNoticeAlert.tsx` を新設し `ExpandedMessage`・`ConditionArea`・`LogAnalyticsPage`・`App`・`api.ts`・`messages.ts` を広げる
- Tests: 新規およそ 65 本（純粋なロジック 33 本はテスト先行：CSV・Markdown・ファイル名・JSON の検出と整形。境界 32 本は実装後：AppSession・InsightsSession・ファイルの書き込み・メニュー・切替・知らせ）。既存の約 1,200 本は緑のまま

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

## 上流

`inception/units-generation/unit-of-work.md`（U2：service、L）、`inception/units-generation/unit-of-work-story-map.md`（FR2〜FR5）、`construction/u2-export-and-json/functional-design/`（`entities.md`、`rules.md`：BR2.1〜BR5.5、`functional-spec.md`：W1〜W7、`reviews/review-01.md`：R-01〜R-07）、`inception/requirements-analysis/requirements.md`（FR2〜FR5、NFR2〜NFR6、AS3、AS6）、`inception/domain-design/decisions.md`（ADR-001〜ADR-004、ADR-006）。

## 機能設計のレビューの所見をこの計画で引き受ける細部

機能設計は「進めてよい」の判定で、注意点が 7 つ残った。実装の細部として次のとおり決める（ルール BR2〜BR5 の範囲内の具体化）。

- **R-01（錠の中の作業を絞る）**：session と LogView の錠の中で行うのは、可否の確認・ExportJob の Running 化・exportNumber の加算・前の知らせの消去・**元の行の複製**（ログの画面は `LogView::rows(0, total)` の `LogEvent` の複製、クエリの画面は結果の `ResultRow` と列の複製）・ロググループ名とゾーンの取得だけ。**文字への変換**（`format_display_in_zone`、`row_cells`、3 列への写し）は錠を離したあと blocking スレッドで行う。錠を持つ時間は行の複製（文字のコピー）に比例する。これを `code-summary.md` と doc コメントに明記する。
- **R-02（失敗の知らせを鍵で持つ）**：`ExportNotice` の失敗側は `failure: ExportFailure`（enum：`PermissionDenied`・`NoSpace`・`NotFound`・`Other`・`Clipboard`・`Dialog`）と `path: Option<String>` で持つ。画面は `export.failure.<kind>` の鍵と引数（パス）から文を組み立てる。OS のエラーの文は標準エラーにだけ出し、画面には出さない。
- **R-03（JSON の候補の選び方）**：候補は `{` と `[` の位置を先頭から順に試す。読み切れた候補が**配列で、その中にオブジェクトも配列も 1 つも含まない**（スカラーだけの配列、例 `[1234]`）ときは候補とみなさず次へ進む。走査器は再帰を使わず明示的なスタックで書き、深さの上限 256・候補の試行回数の上限 64・メッセージの長さの上限 1 MiB を超えたら「見つからなかった」とする。「文字列の中の `{`」は、候補の位置から始める走査の中では文字列として読み飛ばされ、別の候補として試されるときは読み切れないので次に進む（この 1 文に整理して doc コメントに書く）。
- **R-04（権限）**：プラグインは Rust 側からだけ呼ぶため、`capabilities/default.json` に足すのは `allow-start-export` と `allow-dismiss-export-notice` の 2 つだけ。プラグインの WebView 権限は足さない。`@tauri-apps/plugin-*` の npm パッケージも入れない。
- **R-05（切替の操作）**：Cloudscape `SegmentedControl` を使い、キーボード（Tab で届く、矢印か Enter/Space で選べる）を実装後のテストで確かめる。切替の要素では `onKeyDown` と `onClick` の伝播を止め、行の開閉やリストの選択に伝えない。高さの計測は開いた行の内側の容れ物全体（切替＋本文）で行い、20 行の上限は本文だけにかける。切替は選択行かどうかに関わらず Tab で届く（本文の `focusable` の条件は本文だけに残す）。SegmentedControl が仮想化された一覧の中で使えないことが分かれば、2 つの Cloudscape `Button`（variant inline-link、`aria-pressed`）に替え、`code-summary.md` に逸脱として書く。
- **R-06（クエリの画面の既定名の元）**：結果を出した実行の時点の選んだロググループの先頭を使う。`InsightsRun`（または結果）に `log_group_names` を持たせ、実行の開始時に写す。結果がそれを持たないときは今の選択の先頭、それもなければ空（既定名は `export`）。
- **R-07（コマンドの形）**：`start_export` は今の `run_settings_work` と同じ「終わりまで待つ非同期コマンド」にする。受け付けの拒否（無効・二重）は `CommandError` で画面に返り、画面は既存の `reportError` で出す。処理中の状態は `session-changed` で届く。

## 進め方の要点

- ブラウンフィールド。既存のファイルはその場で変え、新しいファイルは次の 5 つだけ：`crates/local-sights-core/src/export/mod.rs`・`formatter.rs`・`exporter.rs`、`src-tauri/src/export_boundary.rs`、`src/jsonPretty.ts`・`src/jsonPretty.test.ts`、`src/components/ExportMenu.tsx`・`ExportMenu.test.tsx`、`src/components/ExportNoticeAlert.tsx`・`ExportNoticeAlert.test.tsx`。
- Testing Contract の ordering に従う。**純粋なロジック**（CSV・Markdown の組み立て、ファイル名の安全化と既定名、JSON の検出と整形）は**テストを先に書いて**赤を確かめてから実装する。**境界の層**（AppSession・InsightsSession の状態、ファイルの書き込み、Tauri コマンドとプラグイン、React の部品）は**実装してからテスト**を書いて走らせる。
- 本番コードでは `unwrap()` / `expect()` を使わない。`cfg` の OS 分岐は置かない（プラグインが吸収、NFR3）。秘密の認証情報を書かない（NFR5）。書き出す値は括りと逃がし以外に加工しない（FR4.1.4、AS3）。
- 対話的な UI 部品には `data-testid` を付ける：`export-menu`、`export-notice`、`export-notice-dismiss`、`log-table-expanded-mode`。
- コマンドが 2 つ増えるため、`generate_handler!`・`build.rs`・`capabilities/default.json` の 3 か所をそろえる（既存の `src/commandRegistration.test.ts` が確かめる）。
- 完了の目安：`cargo fmt --check`・`cargo clippy --workspace --all-targets`・`cargo test -p local-sights-core`・`cargo test -p local-sights`・`npx tsc --noEmit`・`npx eslint .`・`npx prettier --check .`・`npx vitest run` がすべて通る。`cargo deny check` が使えれば通す（使えなければ `code-summary.md` に書く）。

## 実装ステップ

### Step 1: 構成の確認（骨組み）

- [x] Step 1.1 変更するファイルの今の形を確かめる：`crates/local-sights-core/src/lib.rs`（モジュールの公開）、`session.rs`（`SessionView`・`view()`・`SettingsWork` の流儀・`phase`・`filter_state`）、`insights_session.rs`（`ResultSummary`・`QueryRow`・`display_rows`・`row_cells`・`InsightsRun`・`selected_log_groups`）、`log_view.rs`（`rows`）、`query/evaluator.rs`（`QueryResult`・`ResultRow`・`row_cells`）、`src-tauri/src/lib.rs`（`run_settings_work`・`lock`・`emit_or_log`・`run()` の builder・`generate_handler!`）、`src-tauri/build.rs`、`src-tauri/capabilities/default.json`、`src-tauri/Cargo.toml`、`Cargo.toml`（workspace.dependencies）、`src/api.ts`、`src/App.tsx`（警告の領域、`reportError`）、`src/components/ConditionArea.tsx`（絞り込みの欄の行）、`src/components/LogAnalyticsPage.tsx`（結果の Header の actions）、`src/components/ExpandedMessage.tsx`、`src/components/LogTable.tsx`（ExpandedMessage の使われ方と高さの計測）、`src/i18n/messages.ts`、`src/test/fixtures.ts`。
- [x] Step 1.2 新しいモジュールの空の骨組みを置く：`crates/local-sights-core/src/export/mod.rs`（`pub mod formatter; pub mod exporter;` と型の再公開）、`formatter.rs`、`exporter.rs`、`src-tauri/src/export_boundary.rs`、`src/jsonPretty.ts`。`lib.rs` に `pub mod export;` を足す。
  - 対応：全 FR（準備）

### Step 2: テストランナーの確認と作業単位に絞ったコマンドの記録

- [x] Step 2.1 Rust のランナーを確かめる：`cargo test -p local-sights-core export::` が（空の骨組みで）0 本で通ることを確かめる。
- [x] Step 2.2 画面のランナーを確かめる：`npx vitest run src/components/ExpandedMessage.test.tsx` が存在しなければ `npx vitest run src/components/LogTable.test.tsx` が今のテストを通すことを確かめる。
- [x] Step 2.3 作業単位に絞ったコマンドは `unit-test-instructions.md` のとおり。
  - 対応：Testing Contract の runner_step

### Step 3: 純粋なロジック — テストを先に書いてから実装する

- [x] Step 3.1 **[Red]** `export/formatter.rs` のテストを先に書く（コンパイルが通らない・失敗することを確かめる）：
  - `csv_writes_a_header_then_rows_with_crlf`（見出し 1 行＋2 行、各行 CRLF、最後も CRLF）
  - `csv_quotes_cells_with_comma_quote_or_newline_and_doubles_quotes`（`a,b` → `"a,b"`、`"q"` → `"""q"""`、改行を含むセルは括りの中にそのまま）
  - `csv_leaves_plain_cells_and_formula_like_values_untouched`（`=SUM(1)`・`+1` はそのまま、AS3）
  - `csv_with_no_rows_is_just_the_header`
  - `markdown_writes_header_separator_and_rows`（`| a | b |`、`| --- | --- |`、各行 LF、最後も LF）
  - `markdown_escapes_pipes_and_replaces_newlines_with_br`（`x|y` → `x\|y`、CRLF・LF・CR → `<br>`）
  - `markdown_empty_cell_is_a_single_space`
  - `formatter_writes_row_by_row_to_the_writer`（writer への逐次書き込み。全体を 1 つの文字に持たない）
  - 対応：FR4.2、FR4.3、FR4.1.4、BR4.2、BR4.4
- [x] Step 3.2 **[Green]** `export/formatter.rs` を実装する：`pub enum ExportFormat { Csv, Markdown }`、`pub fn write_csv(out: &mut impl Write, columns: &[String], rows: impl Iterator<Item = Vec<String>>) -> io::Result<()>`、`pub fn write_markdown(...)`、`pub fn write(out, format, columns, rows)`。セルの逃がしは小さな純粋関数（`csv_cell`、`markdown_cell`）に分ける。
- [x] Step 3.3 **[Red]** `export/exporter.rs` のファイル名のテストを先に書く：
  - `sanitize_file_stem_replaces_unsafe_characters_with_underscore`（`/aws/lambda/x:y*z?` → `_aws_lambda_x_y_z_`、制御文字も `_`）
  - `sanitize_file_stem_trims_dots_and_spaces_and_falls_back_to_export`（` .name. ` → `name`、`` → `export`、`...` → `export`）
  - `sanitize_file_stem_cuts_at_100_characters`（文字数で切る。マルチバイトの途中で切らない）
  - `default_file_name_joins_stem_time_and_extension`（`_aws_lambda_my-func_20261024-213045.csv`、Markdown は `.md`）
  - `default_file_name_writes_the_time_in_the_chosen_zone`（同じ瞬間が Local（Asia/Tokyo）と UTC で違う文字になる）
  - `default_file_name_without_a_log_group_uses_export`
  - 対応：FR4.4.1、FR4.2.4、FR4.3.3、BR4.5
- [x] Step 3.4 **[Green]** `export/exporter.rs` にファイル名の関数を実装する：`pub fn sanitize_file_stem(name: &str) -> String`、`pub fn default_file_name(log_group_name: Option<&str>, taken_at: DateTime<Utc>, zone: Tz, format: ExportFormat) -> String`（既存の `time_zone.rs` のゾーンの型と `format_display_in_zone` の流儀に合わせる）。
- [x] Step 3.5 **[Refactor]** Step 3.1〜3.4 のテストが緑であることを確かめ、`cargo fmt` と clippy を通す。
- [x] Step 3.6 **[Red]** `src/jsonPretty.test.ts` を先に書く（失敗することを確かめる）：
  - `findJson` が全体が JSON のオブジェクト・配列を見つける（prefix と suffix が空）
  - 前置き（`2026-10-24T01:51:32Z abc-123 INFO {"a":1}`）の prefix を末尾の空白まで保つ
  - 後ろに文字が続く（`{"a":1} tail`）とき suffix に残し、後ろは走査しない
  - 閉じていない `{"a":1,`、壊れた `{"a":}`、`{` だけ、は null
  - 先頭の候補が壊れていて次の候補が読み切れる（`{oops} {"a":1}` → 2 つ目）
  - スカラーだけの配列は候補にしない（`[1234] {"event":"x"}` → オブジェクトを整形）。`[{"a":1}]` と `[[1]]` は候補になる
  - 文字列の中の `{` と `"` の逃がし（`{"s":"a{b\"c"}`）を正しく読み切る
  - JSON を含まないメッセージは null、空のメッセージは null
  - 深さ 300 の入れ子は null（上限 256）、候補 65 個以上は null、1 MiB を超えるメッセージは null
  - `prettyPrint` が字句を保つ：`1.0`・`1e5`・`-0`・`12345678901234567890`・`"é\n"`・キーの順・`true`/`false`/`null`
  - `prettyPrint` の形：2 空白の字下げ、`"key": value`、要素ごとに改行、空の `{}` と `[]` はそのまま、入れ子
  - `prettyPrint` は元の空白と改行を捨てる（`{ "a" : 1 }` → 整形後は規則どおり）
  - prefix + 元の JSON + suffix が元のメッセージと等しい（`findJson` の返り値に元の JSON の範囲 `start`/`end` を含める）
  - 対応：FR5.1〜FR5.4、BR5.1、BR5.2、R-03
- [x] Step 3.7 **[Green]** `src/jsonPretty.ts` を実装する：`export interface JsonSegments { prefix: string; json: string; pretty: string; suffix: string }`、`export function findJson(message: string): JsonSegments | null`、`export function prettyPrint(json: string): string`。再帰を使わない字句の走査器（明示的なスタック）。定数 `MAX_DEPTH = 256`、`MAX_CANDIDATES = 64`、`MAX_MESSAGE_LENGTH = 1_048_576`。`JSON.parse` は使わない。
- [x] Step 3.8 **[Refactor]** Step 3.6 のテストが緑であることを確かめる。

### Step 4: 境界の層 — 実装してからテストを書く

- [x] Step 4.1 **[実装]** `export/exporter.rs` に書き出しの型と流れを実装する：`ExportScreen`・`ExportTarget`・`ExportRequest`、`ExportSnapshotSource`（錠の中で複製した元の行：ログの画面は `Vec<LogEvent>`、クエリの画面は `Vec<ResultRow>`＋列＋ゾーン）、`ExportSnapshot { columns, rows, log_group_name, taken_at }`、`ExportJob { export_number, status: ExportStatus, request, row_count }`、`ExportFailure`（R-02 の enum）、`ExportNotice`（`Saved { path, row_count }`・`Copied { row_count }`・`SaveFailed { failure, path }`・`CopyFailed { failure }`）、`ExportOutcome`（`Saved { path, row_count }`・`Copied { row_count }`・`Cancelled`・`Failed { failure, path }`）。`ExportSnapshotSource::into_snapshot(self) -> ExportSnapshot`（錠の外で文字に変える：`format_display_in_zone`、`row_cells`、3 列への写し。R-01）。`render_to(out, snapshot, format, with_bom)`。`SessionView` 向けの `ExportJobView`。
  - 対応：FR2.2、FR2.3、FR3.2、FR3.3、FR4.2.3、BR2.2、BR3.1、BR4.3、BR4.8、R-01、R-02
- [x] Step 4.2 **[実装]** `session.rs`：`export: ExportJob` を持つ（初期値 Idle）。`can_export_logs(&self) -> ExportAvailability`（BR2.3：行数は `filter_state` の一致数か全件数、`phase`、絞り込みの状態、`export.status`）。`begin_export(&mut self, request, rows_source) -> Result<ExportBegun, SessionError>`：画面に応じて可否を確かめ（クエリの画面は `insights.can_export(exporting)`）、無効なら `SessionError::ExportUnavailable`、Running なら `SessionError::ExportBusy`。有効なら Running・exportNumber+1・知らせの消去。`ExportBegun { export_number, source, default_name_parts }` を返す。`finish_export(&mut self, export_number, outcome)`（番号が違えば捨てる。Cancelled → Idle、Saved/Copied → Done と知らせ、Failed → Failed と知らせ）。`dismiss_export_notice()`。`SessionView` に `export: ExportJobView`・`export_notice: Option<ExportNotice>`・`can_export_logs: bool` を足し、`InsightsView` を作るときに `exporting` を渡す。クエリの画面の既定名のロググループは R-06 のとおり。
  - 対応：FR2.5、FR2.6、FR4.6、BR2.3、BR4.8、BR4.9
- [x] Step 4.3 **[実装]** `insights_session.rs`：`InsightsRun` に `log_group_names: Vec<String>` を足し、実行の開始時に選択を写す（R-06）。`can_export(&self, exporting: bool) -> ExportAvailability`（BR3.2）。`export_source(&self) -> Option<ExportSnapshotSource>`（結果の列と `ResultRow` の複製、ロググループ名）。`InsightsView.can_export`。`row_cells` を錠の外で呼べるように、必要なら `EventFields` と `row_cells` の可視性を `pub(crate)` に広げる。
  - 対応：FR3.1〜FR3.5、BR3.1、BR3.2、R-06
- [x] Step 4.4 **[テスト]** 中核の境界のテストを書く（実装後）：
  - `session.rs`：`export_is_unavailable_without_rows_while_fetching_while_filtering_and_while_exporting`、`begin_export_takes_a_snapshot_source_and_marks_running`、`a_second_export_while_running_is_refused_and_changes_nothing`、`finish_export_cancelled_returns_to_idle_without_a_notice`、`finish_export_saved_and_copied_make_done_with_their_notices`、`finish_export_failed_keeps_the_failure_kind_and_path`、`a_result_for_another_export_number_is_dropped`、`dismiss_export_notice_clears_it_and_a_new_export_clears_it_too`、`the_log_screen_snapshot_has_three_columns_display_times_and_full_messages`（`into_snapshot` で 3 列、絞り込み後の行だけ）。
  - `insights_session.rs`：`export_is_unavailable_without_a_result_with_zero_rows_while_running_and_while_exporting`、`the_query_snapshot_has_the_result_columns_full_messages_and_empty_cells`、`the_default_name_uses_the_first_log_group_of_the_run`。
  - `export/exporter.rs`：`render_prepends_a_bom_only_for_csv_files`。
  - 対応：BR2.1、BR2.3、BR3.1、BR3.2、BR4.3、BR4.8、BR4.9
- [x] Step 4.5 **[実装]** 依存とプラグイン：`Cargo.toml` の `[workspace.dependencies]` に `tauri-plugin-dialog = "2"` と `tauri-plugin-clipboard-manager = "2"` を足し、`src-tauri/Cargo.toml` の `[dependencies]` から参照する。`lib.rs` の `run()` の builder に `.plugin(tauri_plugin_dialog::init())` と `.plugin(tauri_plugin_clipboard_manager::init())` を足す。`Cargo.lock` を更新する（`cargo build -p local-sights` か `cargo test -p local-sights`）。`capabilities/default.json` にプラグインの権限は足さない（R-04）。
  - 対応：ADR-002、NFR3
- [x] Step 4.6 **[実装]** `src-tauri/src/export_boundary.rs`：`write_file_atomically(path, |out| render…) -> Result<(), ExportFailure>`（同じフォルダに `<名前>.tmp-<乱数 16 桁>`、書いて閉じて rename、失敗時は一時ファイルを消す。`io::ErrorKind` から `ExportFailure` へ：`PermissionDenied`→PermissionDenied、`StorageFull`/`QuotaExceeded`→NoSpace、`NotFound`→NotFound、ほかは Other。OS の文は `eprintln!` にだけ）、`ask_save_path(app, default_name, format) -> Result<Option<PathBuf>, ExportFailure>`（`tauri_plugin_dialog` の保存ダイアログ：既定名、拡張子のフィルタ `CSV`/`Markdown`。取り消しは `None`、失敗は `Dialog`）、`copy_to_clipboard(app, text) -> Result<(), ExportFailure>`（`tauri_plugin_clipboard_manager` の `write_text`、失敗は `Clipboard`）。`cfg` の分岐なし。
  - 対応：FR4.4、FR4.5、BR4.6、BR4.7
- [x] Step 4.7 **[実装]** `lib.rs` のコマンド：`start_export(screen, format, target, app, state) -> Result<(), CommandError>`（非同期、終わりまで待つ。R-07）：session の錠を取り、画面に応じて source を作る（ログの画面は LogView の錠も取って `rows(0, total)` の `LogEvent` を複製）、`begin_export`、`view()` を送って錠を離す。target が file なら `ask_save_path`；`None` なら `finish_export(Cancelled)`。それ以外は `spawn_blocking` で `into_snapshot` → `render_to`（ファイルは `write_file_atomically`、クリップボードは文字にして `copy_to_clipboard`）。スレッドが落ちたら `Failed { failure: Other }`（Interrupted 相当）。結果を `finish_export` に渡し、`view()` を送る。`dismiss_export_notice(app, state)`（同期、`change_settings` と同じ形）。`generate_handler!`・`build.rs`・`capabilities/default.json` の 3 か所に `start_export`・`dismiss_export_notice` を足す。`SessionError` の新しい値を `CommandError` の鍵（`error.exportUnavailable`・`error.exportBusy`）に対応させる。
  - 対応：FR4.4、FR4.5、FR4.6、BR4.6、BR4.7、BR4.8、R-01、R-07
- [x] Step 4.8 **[テスト]** アプリ側のテストを書く（実装後）：`export_boundary.rs` に一時フォルダでの `write_file_atomically` のテスト 4 本（書けて rename される・一時ファイルが残らない、フォルダのはずの位置に通常のファイルがあると失敗して一時ファイルが残らない、`ExportFailure` への対応、`NotFound`）。`cargo test -p local-sights` と `src/commandRegistration.test.ts` で 3 か所の一致を確かめる。
  - 対応：BR4.6、制約（3 か所の一致）
- [x] Step 4.9 **[実装]** `src/api.ts`：型 `ExportScreen`・`ExportFormat`・`ExportTarget`・`ExportStatus`・`ExportJobView`・`ExportFailure`・`ExportNotice`、`SessionView.export`・`exportNotice`・`canExportLogs`、`InsightsView.canExport`、`startExport(screen, format, target)`、`dismissExportNotice()`。`src/test/fixtures.ts` の `sessionView` と `insightsView` に新しい項目の既定値（Idle、null、false）。`src/i18n/messages.ts` に文言：`export.menu`（エクスポート／Export）、`export.item.csvFile`・`markdownFile`・`csvClipboard`・`markdownClipboard`、`export.notice.saved {path, count}`・`copied {count}`・`saveFailed {detail}`・`copyFailed {detail}`、`export.failure.PermissionDenied`・`NoSpace`・`NotFound`・`Other`・`Clipboard`・`Dialog`（`{path}` つき）、`export.notice.dismiss`、`error.exportUnavailable`・`error.exportBusy`、`expanded.mode`・`expanded.mode.pretty`・`expanded.mode.raw`。
  - 対応：FR4.1、FR4.4.4、FR4.5、BR4.9
- [x] Step 4.10 **[実装]** `src/components/ExportMenu.tsx`：`ButtonDropdown`（`data-testid="export-menu"`、`ariaLabel` = `export.menu`、項目 4 つの id は `csv-file`・`markdown-file`・`csv-clipboard`・`markdown-clipboard`）。props `disabled`・`loading`・`onSelect(format, target)`・`t`。`loading` のときも `disabled`。
- [x] Step 4.11 **[実装]** `src/components/ExportNoticeAlert.tsx`：`notice: ExportNotice | null`、`t`、`onDismiss`。Saved/Copied は `Alert type="success"` を `role="status"` の容れ物に、SaveFailed/CopyFailed は `type="error"` を `role="alert"` に。文は鍵と引数で組み立てる（R-02）。`data-testid="export-notice"`、閉じるボタンの名前 `export.notice.dismiss`。
- [x] Step 4.12 **[実装]** 配置と配線：`ConditionArea` の絞り込みの欄の行の右端に ExportMenu（`session.canExportLogs`、`session.export.status === "Running"`、`onExport`）。`LogAnalyticsPage` の結果の Header の `actions` に ExportMenu（`insights.canExport`、同じ Running、truncated の表示と `SpaceBetween` で並べる）。`App.tsx`：`handleExport(screen, format, target)` → `run(() => startExport(...))`、`handleDismissExportNotice`、両画面の警告の領域（SSO 成功の知らせの隣）に `ExportNoticeAlert`。
  - 対応：FR4.1.3、BR4.1、BR4.9
- [x] Step 4.13 **[実装]** `src/components/ExpandedMessage.tsx`：`findJson(message)` を `useMemo` で求め、見つかれば `SegmentedControl`（`data-testid="log-table-expanded-mode"`、`label` = `expanded.mode`、選択肢 `pretty`/`raw`、初期値 `pretty`）を本文の上に出し、`pretty` なら `prefix + pretty + suffix`、`raw` なら元のメッセージを 1 つの文字ノードで描く。切替の容れ物で `onKeyDown`・`onClick` の伝播を止める（R-05）。高さの計測は内側の容れ物全体（切替＋本文）で行い、20 行の上限（372 px）は本文の要素だけにかける。`LogTable.tsx` 側で ExpandedMessage に `t` を渡す（props の追加）。SegmentedControl が使えなければ R-05 の代替。
  - 対応：FR5.5〜FR5.8、BR5.3、BR5.4、BR5.5、R-05
- [x] Step 4.14 **[テスト]** 画面のテストを書く（実装後）：
  - `ExportMenu.test.tsx`（6 本）：4 項目がこの順で出る、選ぶと `(format, target)` が渡る、`disabled` で開けない、`loading` で無効と処理中、キーボードで開いて選べる、日本語の文言。
  - `ExportNoticeAlert.test.tsx`（4 本）：Saved の文にパスと行数、Copied の文に行数、SaveFailed が `role="alert"` で原因の文（鍵から）、閉じると `onDismiss`。
  - `ExpandedMessage.test.tsx`（新規、6 本）：JSON なしは切替なしで元の文字、JSON ありは初期値「整形」で整形後の文字、「元のまま」で元の文字、文字ノードだけ、切替のキー操作が行に伝わらない（`stopPropagation`）、Escape で `onLeave`（今の振る舞い）。`LogTable.test.tsx` の既存の期待（開いた行の文字）を JSON を含まないメッセージのまま緑に保つ。
  - `App.test.tsx`（4 本）：ログの画面のメニューが `canExportLogs` を映す、選ぶと `startExport("logManagement", "csv", "file")` が呼ばれる、`exportNotice` が届くと知らせが出て閉じると `dismissExportNotice`、言語を切り替えると知らせの文が切り替わる。`LogAnalyticsPage.test.tsx`（2 本）：結果の Header にメニューが出て `insights.canExport` を映す、選ぶと `startExport("logAnalytics", …)`。
  - 対応：BR4.1、BR4.9、BR5.3、BR5.4、BR5.5

### Step 5: 環境とビルドの設定

- [x] Step 5.1 `cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .` を通す。`cargo deny check` が入っていれば通す（新しい 2 つのクレートのライセンスは MIT OR Apache-2.0）。`npm audit --audit-level=high` は依存が変わらないため変化なし（`package.json` は触らない）。`Cargo.lock` の差分が 2 つのプラグインとその依存だけであることを確かめる。

### Step 6: 文書とトレーサビリティ

- [x] Step 6.1 変えた関数・型・部品の doc コメントを更新する（`export/` のモジュール説明に錠の中と外の分担（R-01）、`ExpandedMessage` に整形と切替、`lib.rs` のモジュール説明にコマンドの追加とプラグイン）。`docs/` の利用者向け文書はこの作業単位では変えず、Build and Test で U1 と合わせて見直す。
- [x] Step 6.2 `cargo test -p local-sights-core`・`cargo test -p local-sights`・`npx vitest run` を全体で走らせ、既存のテストが緑のままであることを確かめる。

## ストーリー（FR）と手順の対応

| FR / BR | 手順 |
|---|---|
| FR2.1〜FR2.3 写し（BR2.1、BR2.2） | Step 4.1、4.2、4.4、4.7 |
| FR2.4、FR3.4 上限なし | Step 3.1〜3.2（逐次書き込み）、4.1 |
| FR2.5、FR2.6 可否（BR2.3） | Step 4.2、4.4、4.10、4.12、4.14 |
| FR3.1〜FR3.3 クエリの写し（BR3.1） | Step 4.1、4.3、4.4 |
| FR3.5 可否（BR3.2） | Step 4.3、4.4、4.12、4.14 |
| FR4.1 形式と出し方・メニュー（BR4.1） | Step 4.9〜4.12、4.14 |
| FR4.2 CSV（BR4.2、BR4.3） | Step 3.1〜3.2、4.1、4.4 |
| FR4.3 Markdown（BR4.4） | Step 3.1〜3.2 |
| FR4.4 ファイル（BR4.5、BR4.6） | Step 3.3〜3.4、4.5〜4.8 |
| FR4.5 コピー（BR4.7） | Step 4.6〜4.7 |
| FR4.6 進み具合と二重の操作（BR4.8、BR4.9） | Step 4.2、4.4、4.7、4.11、4.12、4.14 |
| FR5.1〜FR5.4 JSON の検出と整形（BR5.1、BR5.2） | Step 3.6〜3.8 |
| FR5.5〜FR5.8 切替と表示（BR5.3、BR5.4） | Step 4.13〜4.14 |
| NFR6 キーボード・読み上げ（BR5.5） | Step 4.10、4.11、4.13、4.14 |
| NFR2 応答性 | Step 4.1、4.7（錠の外で変換、blocking スレッド） |
| NFR3 両 OS | Step 4.5〜4.6（プラグイン、`cfg` なし） |
| NFR5 秘密 | Step 4.6（OS の文は画面に出さない）、4.11 |

## Assumptions & Open Questions

- [assumption] `tauri-plugin-dialog` 2 の Rust API は `app.dialog().file().set_file_name(..).add_filter(..).blocking_save_file()`（または callback 版）で保存先を返し、取り消しは `None`。`tauri-plugin-clipboard-manager` 2 の Rust API は `app.clipboard().write_text(String)`。版の違いで名前が違えば、同じ役割の API に合わせ `code-summary.md` に書く。
- [assumption] `blocking_save_file` は非同期コマンドの中の `spawn_blocking` から呼べる（メインスレッドでは呼ばない）。
- [assumption] Cloudscape の `SegmentedControl` はキーボードで操作できる。できないか仮想化された一覧と衝突するときは R-05 の代替（2 つの Button）に替える。
