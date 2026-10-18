# Code Generation Plan — U4 互換の確認用プログラム（u4-compat-check）

## Summary

- Builds: コンソールの Logs Insights の結果を CLI で書き出すスクリプトと、同じロググループ・期間でアプリの結果を作って比べる確認用プログラム（開発者の手元だけで動かし、配らない）
- Touches: `scripts/compat/insights-export.sh`（新規）、`crates/local-sights-core/examples/compat_check.rs` と `examples/compat/`（新規）、`crates/local-sights-core/examples/compat_queries.txt`（新規）、`crates/local-sights-core/Cargo.toml`（例の登録だけ）、`.gitignore`（書き出しのファイルの名前を念のため除く）、`README.md`（使い方の短い節）
- Tests: Rust 約 30 件（比べ方の部品を見本のデータでテスト先行。AWS には接続しない）、スクリプトは `bash -n` と、偽の `aws` で流すテスト、既存のテストはすべて通したまま

## 上流と前提

- 設計：`construction/u4-compat-check/functional-design/` の `functional-spec.md`（流れの正本）・`rules.md`（BR1.1〜BR3.3）・`entities.md`、質問票（Q1 = A：CLI のスクリプトで書き出す）。要件 `inception/requirements-analysis/requirements.md`（SM1、SM2、NFR3、NFR4、OQ5）、作業単位 `inception/units-generation/unit-of-work.md`（U4）。
- アプリと確認用プログラムが呼ぶ AWS API は、今と同じ DescribeLogStreams・GetLogEvents だけ（DescribeLogGroups も使わない）。Logs Insights の API（StartQuery・GetQueryResults）は、開発者が手で流すスクリプトだけが CLI で呼ぶ（U2 の console-check.sh と同じ扱い、Q1）。
- 書き出しのファイルはログの中身を含むため、リポジトリに置かない（NFR4）。自動テストは実際の AWS に接続しない。
- テストの進め方は末尾の Testing Contract のとおり（custom）。比べ方は純粋なロジックのためテスト先行、スクリプトと取得の配線は実装してからテスト。既存のテストはすべての OS で通った状態を保つ。

### 機能設計のレビュー（1 回目）の指摘の反映

設計書は書き換えず、この計画の手順として直す（project.md Corrections）。

| 指摘 | 直し方 | 手順 |
|---|---|---|
| R-01（Major）sort と limit の境目で同順位の行がまたがると、残る行が決まらない | limit（か既定の上限）で切ったクエリは、アプリの側で切る前の並びを持ち、境目の行の sort の値が切ったすぐ後ろの行と同じなら「境目の同順位」とする。境目の同順位の行（その sort の値を持つ行）は、行の集まりの比べから除き、件数だけを比べる。除いた行の数を報告する。sort のない上限で切ったクエリは、今の設計どおり Skipped（BR2.5） | Step 4 |
| R-02（Major）動かせなかった場合の扱い | 書き出しの status が Complete でない（Failed・Cancelled・Timeout・Unknown）か、書き出しのファイルがないクエリは NotCompared とする。NotCompared が 1 つでもあるか、実際に比べた本数が 0 なら、終了コード 2（比べられなかった）にする。Error（start-query が断った）は BR2.4 の断り方の比べに使う | Step 5、Step 7 |
| R-03（Minor）期間の境目の扱い | スクリプトは START・END（秒）を start-query にそのまま渡す。確認用プログラムは、アプリの取得の期間を START の 0 ms から END の 999 ms までにする（アプリの決まり）。Logs Insights が END の秒をどう含めるかは確かめていないため、アプリの結果に、START の秒か END の秒（境目の 2 秒）に入るイベントがあれば、そのクエリの報告に「境目にイベントあり」と出し、違いが出たときの見立てに使う。スクリプトの説明には、境目の秒にイベントの少ない時刻を選ぶよう書く | Step 5、Step 6 |
| R-04（Minor）p09 は JSON の数を確かめていない | p09 の目的を「数字だけの文字列（JSON の文字列）と文字列の =」に直す。CloudTrail には JSON の数の項目がほとんどないため、JSON の数の比べは確かめられないまま残ることを `code-summary.md` に書く。ほかのロググループ用のクエリの組を足せる形（`--queries` で組のファイルを選ぶ）にする | Step 2、Step 7、Step 9 |
| R-05（Minor）リポジトリの最上位と、Windows での場所の比べ方 | スクリプトは `git rev-parse --show-toplevel` で最上位を求め、`OUT` を `cd` して `pwd -P` で実際の場所にしてから、最上位の中かを比べる。確認用プログラムは、`CARGO_MANIFEST_DIR` から上へたどって `.git` のあるフォルダを最上位とし、両方を `canonicalize` してから、Windows では大文字・小文字を区別せずに前の部分を比べる。BR1.4 の「違う条件を渡したらエラー」は、確認用プログラムが条件を `settings.json` からしか読まないため起きない（条件の取り違えが起きないことの説明として扱う） | Step 3、Step 6 |
| R-06（Minor）選んだ列がすべて空のイベント | Logs Insights の行から @ptr を除いて列が 1 つもない行と、アプリの行で選んだ列がすべて値なしの行を、どちらも「列のない行」として同じに扱う。テストで確かめる | Step 4 |

## 手順

各手順を終えたら、その手順の箱に印を付ける。

### 準備

- [x] **Step 1. 構成とテストの道具**：`crates/local-sights-core/examples/compat_check.rs` と、その下のモジュール `examples/compat/`（`export.rs` 書き出しの読み込み、`format.rs` 値の書き方の合わせ、`compare.rs` 比べ方、`report.rs` 報告）を作る。`Cargo.toml` に例を登録する（新しいクレートは足さない。JSON は今の `serde_json`）。`cargo test -p local-sights-core --example compat_check` が 0 件で通ることを確かめる。

### 純粋なロジック（テストを先に書いてから実装する）

- [x] **Step 2. クエリの組の読み込み**：`compat_queries.txt` の 1 行（`id<TAB>order|set<TAB>目的<TAB>クエリ`、`#` は注記）を読む。id は英数字と - だけで重ならない。クエリの組（functional-spec の 3 章の 20 本、p09 の目的は R-04 のとおり直す）を書く。先にテスト（5 件ほど）：読める、注記と空行を飛ばす、id の重なりはエラー、id の文字の誤りはエラー、クエリの文が空はエラー。
- [x] **Step 3. 書き出しのフォルダと期間の確かめ**：書き出しのフォルダがリポジトリの外か（R-05）、`settings.json` の形、END が流した時刻の 30 分以上前か（BR1.3）を判断する関数を書く。先にテスト（5 件ほど）：リポジトリの中はエラー、リポジトリの外は通る、大文字・小文字だけ違う場所（Windows のときだけ同じとみなす）、settings.json の欠けた項目、END が近すぎる。
- [x] **Step 4. 比べ方**：`format.rs` に BR2.2 の書き方の合わせ（@timestamp は UTC の `YYYY-MM-DD HH:MM:SS.sss`、JSON の数・真偽値・文字列、値なし）、`compare.rs` に BR2.1 の列と値の多重集合の比べ、BR2.3 の並びの比べ（sort の値が同じ行どうしの順番は違いにしない）、R-01 の境目の同順位の扱い、R-06 の列のない行、BR2.4 の断り方、BR2.5 の Skipped を書く。先にテスト（12 件ほど）：同じ集まり、片方にだけある行（最初の 5 つまで）、列の名前の違い、並びの違いの番号、同順位の並びは違いにしない、境目の同順位を除いて件数だけ比べる、列のない行どうしは同じ、両方が断った・片方だけ断った、sort のない上限は Skipped、@timestamp の書き方、JSON の数と真偽値の書き方。
- [x] **Step 5. 判定と終了コード**：クエリごとの verdict（Same・Different・BothRejected・OnlyAppRejected・OnlyInsightsRejected・Skipped・NotCompared）と、全体の終了コード（0 互換、1 違いあり、2 比べられなかった。NotCompared が 1 つでもあるか比べた本数が 0 なら 2、R-02）を決める関数と、報告の 1 行の形（BR3.2、境目にイベントありの印、R-03）を書く。先にテスト（6 件ほど）：すべて Same は 0、Different があれば 1、NotCompared があれば 2、比べた本数 0 は 2、Skipped だけでは失敗にしない、報告の行に認証情報もメッセージの全文も出ない（先頭 40 文字まで）。

### 境界の層（実装してからテストを書いて実行する）

- [x] **Step 6. 確認用プログラムの配線**：`compat_check.rs` で、`--export-dir`（と `--queries`）を受け取り、settings.json を読み、フォルダと期間を確かめ、今のアプリの取得の流れ（QueryStore と取得の流れ、fetch_check と同じ作り）で期間を 1 回取得し（Completed でなければ 2 で止まる）、クエリごとにアプリの中核で評価して比べ、報告を出して終了コードで終わる。期間は START の 0 ms から END の 999 ms（R-03）。そのあとテスト（3 件ほど、偽のゲートウェイと一時フォルダで）：書き出しとアプリの結果が同じなら 0、違えば 1、取得が失敗なら 2。
- [x] **Step 7. 書き出しのスクリプト**：`scripts/compat/insights-export.sh` を書く（PROFILE・REGION・GROUP・START・END・OUT・QUERIES。U2 の console-check.sh と同じ流し方）。OUT がリポジトリの外かの確かめ（R-05）、END の確かめ（BR1.3）、settings.json、クエリごとの `<id>.json`（status・results、Error のときは断った文からリクエスト ID を除いたもの）、最後の数の要約。説明に、境目の秒にイベントの少ない時刻を選ぶことと、Logs Insights は読んだ量で課金されることを書く。そのあとテスト（3 件ほど）：`bash -n`、PATH の先頭に置いた偽の `aws`（決まった JSON を返す小さなスクリプト）で流して `<id>.json` と settings.json ができる、リポジトリの中の OUT は止まる。Windows のランナーでは bash のテストを飛ばす（スクリプトは macOS の手元で流すため）。
- [x] **Step 8. 除外と説明**：`.gitignore` に、書き出しのフォルダの名前の例（`insights-export/`）を念のため足す（BR1.2 で中には作れないが、誤って置いたときに入らないように）。README の開発者向けの節に、2 つの使い方を短く書く。

### 仕上げ

- [x] **Step 9. 全体の確かめと記録**：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`（例のテストを含む）、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run` を通す。`code-summary.md`（クエリの組、確かめられないまま残る組：JSON の数、R-04。手元で流す手順と、結果を記録すること、BR3.3）、`traceability.json`（BR1.1〜BR3.3 と要件の ID）、`source-manifest.json` を書く。

## 要件・決まりと手順の対応

| 要件・決まり | 手順 |
|---|---|
| SM1・NFR3（BR2.1〜BR2.6、BR3.1・BR3.2） | Step 2、Step 4、Step 5、Step 6 |
| SM2（BR2.4） | Step 4、Step 5 |
| NFR4・NFR5（BR1.1〜BR1.4） | Step 3、Step 6、Step 7、Step 8 |
| BR3.3（記録） | Step 9 |

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
      "text": "- クエリの速さの合否の数値は、利用者の答え（作る段階で決める）に従って要件では決めなかった（NFR1） (learned 2026-10-17)"
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
  "input_sha256": "sha256:a25545532c688efdb772fd759d07fcc6274f6b15399c5b22f94f08017c80f5fa",
  "contract_sha256": "sha256:d60a0643dd5ca9e449df554ad8bc01d40cde0d190c1f57fccbac402037684936"
}
```
