# Code Generation Plan — U3 クエリの画面の取り足しと複数のロググループ（u3-shared-store）

## Summary

- Builds: クエリの画面が取得済みの範囲を覚えて足りない分だけを取得し（ディスクキャッシュの重なる部分も使う）、50 個までのロググループに対してクエリを実行できるようにする。U2 から回した直し（条件の入れ子でアプリが落ちる不具合、補完の名前が残る）も入れる
- Touches: `crates/local-sights-core/src/cache/plan.rs`（足りない範囲の計算）・`cache/mod.rs`（重なる部分の照会）、`timeline.rs`（取り出しの操作を 1 つ足す）、`query/store.rs`・`runner.rs`・`plan.rs`・`parser.rs`・`evaluator.rs`、`insights_session.rs`・`session.rs`、`src-tauri/src/lib.rs`、`src/api.ts`・`LogGroupMultiselect.tsx`・`LogAnalyticsPage.tsx`・`QueryResultTable.tsx`・`i18n/messages.ts`
- Tests: Rust 約 50 件（範囲の計算と順番の振り直しはテスト先行、要件の FR2 の受け入れ基準 6 つを結合のテストに）と Vitest 約 15 件、メモリの測り（無視つきのテスト）、既存のテストはすべて通したまま

## 上流と前提

- 設計：`construction/u3-shared-store/functional-design/` の `functional-spec.md`（流れと状態の正本）・`rules.md`（BR1.1〜BR5.3）・`entities.md`、質問票（質問にしなかったことの表）。要件 `inception/requirements-analysis/requirements.md`（FR1.3、FR2.1〜FR2.8）、作業単位 `inception/units-generation/unit-of-work.md`（U3）。
- ログ管理の画面の保持・表示・取得・部分一致の絞り込みと、今のキャッシュの照会・書き込みの振る舞いは変えない。評価の入口の形は U1 のまま。
- 呼ぶ AWS API は今と同じ 3 つだけ。認証情報はログ・エラー・画面に出さない。自動テストは実際の AWS に接続しない。
- テストの進め方は末尾の Testing Contract のとおり（custom）。テストの量は Standard。既存のテストはすべての OS で通った状態を保ち、行カバレッジ 80% の CI の合否も下げない。

### 機能設計のレビュー（1 回目）の指摘の反映

設計書は書き換えず、この計画の手順として直す（project.md Corrections）。

| 指摘 | 直し方 | 手順 |
|---|---|---|
| R-01（Major）今の取得の流れとの接続が決まっていない | 範囲 1 回の取得ごとに、使い捨ての受け皿（新しい EventTimeline を包み、今の `TimelineStore` と `HeldEvents` を満たす）を作って今の `run_fetch_with_cache` に渡す。取得の流れの `discard` はその受け皿だけを空にするため、ほかの範囲の分は消えない。キャッシュへの書き込みも、今の流れのとおりこの受け皿から読む。取得が終わったら、受け皿のイベントを取り出し（EventTimeline に、持っているイベントをすべて渡して空になる操作を 1 つ足す。ログ管理の画面は使わない）、Completed なら新しさの境目より前を committed、後を staging に、それ以外はすべて staging に、順番を振り直して足す（BR1.2、BR1.4）。committed と staging に同じイベントを置くことはない | Step 4、Step 6、Step 7 |
| R-02（Major）入れ子の上限が and・or の連なりを数えていない | 条件の木の and・or を、2 つずつの入れ子ではなく、並び（3 つ以上の子を持てる形）で持つように変える。長い `a or b or c …` は 1 段の並びになり、評価も破棄も並びをたどる繰り返しで行うため、項の数が多くても深くならない。そのうえで、括弧・not・and と or の入れ替わりの入れ子を数え、101 段目で Syntax にする（BR5.1）。テストは、Windows のメインのスレッドと同じ 1 MB のスタックのスレッドで、10,000 文字に入るいちばん長い or の並びと、100 段ちょうどの入れ子と 101 段の入れ子を流す | Step 3 |
| R-03（Minor）メモリの確かめの条件が狭い | 測る状態を、ログ管理の画面に 100 万件、クエリの画面に 100 万件（50 個のロググループに分ける）、結果の写し 100,000 件、評価の並び、にする | Step 13 |
| R-04（Minor）Failed のあとも残りのロググループを取得し続ける | 続ける（失敗したロググループをすべて示すため）。ただし、認証情報の失敗（SSO の期限切れ・認証情報がない、今の ApiFailure の種類）は、どのロググループでも同じ結果になるため、残りのロググループを始めずに FetchFailed にする。進み具合に失敗したロググループの数を出す | Step 7、Step 10 |
| R-05（Minor）順番の振り直しの理由と索引の伸び | 理由はコードに合わせて「同じ番号を 2 回使うと、ストリームと番号で 1 つのイベントを指せなくなる」と書く。次の実行で staging を捨てるときに、ストリームごとの次の順番を committed の次の番号に戻す（staging を捨てたあとは、その番号を使うイベントがないため）。これで取り直しが続いても索引は伸びない | Step 5、Step 6 |
| R-06（Minor）重なる部分の照会での整合の確かめ | 新しい照会も、今の書き込みと同じくファイル全体を読み、今と同じ確かめ（先頭の行の形、範囲ごとのイベントの数、各イベントの形）をしてから、重なる部分のイベントだけを切り出す。読めない・壊れているときの扱いは今の照会と同じ（壊れていれば消し、どちらもその部分を AWS から取得する） | Step 4 |
| R-07（Minor）UC3 の例と新しさの基準の時刻 | 新しさの境目は、実行を始めた時刻の 5 分前とする（50 個を順に取ってもすべてのロググループで同じ境目。範囲ごとの取得の開始より早いため保守的）。キャッシュへの書き込みは今の流れのまま、範囲ごとの取得の開始で決まる。UC3 の例はテストで数値どおりに確かめる：取得済みが 10:00:00.000〜11:00:59.999 のとき、10:00:00〜12:00:00（終わりは 12:00:00.999 まで）で実行すると、足りない範囲は 11:01:00.000〜12:00:00.999 | Step 2、Step 6 |

## 手順

各手順を終えたら、その手順の箱に印を付ける。

### 準備

- [ ] **Step 1. テストの道具を確かめる**：`unit-test-instructions.md` の U3 の範囲のコマンドが今のリポジトリで動くことを確かめる。新しいクレートや画面の依存は足さない。

### 純粋なロジック（テストを先に書いてから実装する）

- [ ] **Step 2. 足りない範囲と重なりの計算（CachePlan）**：`cache/plan.rs` に、期間から取得済みの範囲を引く関数と、2 つの範囲の並びの重なりを求める関数を足す（BR1.1、BR2.1）。先にテスト（8 件ほど）：取得済みなし、すべて取得済み、前だけ・後ろだけ・真ん中が足りない、同じミリ秒の境目、終了の 999 ms、隣り合う範囲、要件の受け入れ基準の 10:00〜11:00 と 10:00〜12:00・10:15〜10:45、R-07 の数値の例。
- [ ] **Step 3. 条件の並びの形と入れ子の上限（QueryParser・QueryEvaluator）**：`plan.rs` の and・or を並びの形にし、`parser.rs` で連なりを 1 つの並びに読み、入れ子を数えて 101 段目で Syntax にする（BR5.1、R-02）。`evaluator.rs` の評価を並びをたどる形にする。U2 の読み取りと評価の結果は変えない（U2 のテストはそのまま通す）。先にテスト（6 件ほど）：1 MB のスタックのスレッドで、10,000 文字のいちばん長い or の並び・and の並びを読んで評価して捨てても落ちない、100 段ちょうどは読める、101 段は Syntax で開いた括弧の場所、`not (` の重ねも数える、and と or の混ざった優先順位は U2 のまま。
- [ ] **Step 4. キャッシュの重なる部分の照会（LogCache）**：`cache/mod.rs` に、範囲の並びを受け取り、記録範囲と重なる部分の範囲とイベントを返す照会を足す。ファイル全体を読み、今と同じ確かめをする（R-06）。今の `lookup` と `write` は変えない。テスト（実装してから、6 件ほど、一時ディレクトリを使う）：重なりなし、一部が重なる、2 つの記録範囲にまたがる、重なる部分の外のイベントを返さない、壊れたファイルは消して「使えない」を返す、読めないファイルは残して「使えない」を返す。※ ファイルに触れる境界のため、実装してからテストを書く。
- [ ] **Step 5. 順番の振り直し**：`query/store.rs` に、ロググループのストリームごとの次の順番を持ち、足すイベントに続きの番号を振る関数と、staging を捨てるときに次の順番を committed の次に戻す関数を書く（BR1.4、R-05）。先にテスト（5 件ほど）：2 つの範囲を足しても番号がぶつからない、違うロググループの同じ名前のストリームは別の列、staging を捨てて取り直しても番号が伸びない、キャッシュから足したイベントも振り直す。

### 境界の層（実装してからテストを書いて実行する）

- [ ] **Step 6. QueryStore の広げ（ロググループごとの保持）**：`query/store.rs` を、（プロファイル, リージョン, ロググループ）ごとの GroupHoldings（committed、取得済みの範囲、staging、次の順番）に広げる（BR1.1〜BR1.6）。実行を始めるときに staging を捨てて epoch を増やす、足りない範囲を返す、範囲 1 回ごとの受け皿を作る、取得が終わった受け皿を新しさの境目（実行を始めた時刻の 5 分前、R-07）で committed と staging に振り分ける（R-01）、キャッシュから読んだイベントを committed に足して範囲に加える、評価の並び（committed と staging の期間に入るイベント、BR1.5）、接続を変えたときの全部捨て（BR1.6）。`timeline.rs` に、持っているイベントをすべて渡して空になる操作を足す（R-01）。そのあとテスト（8 件ほど）：振り分け、新しさの境目より後は staging、Completed 以外は staging、次の実行で staging が捨てられて取り直される、評価の並びに同じイベントが 2 回出ない、選ばなくなったロググループの保持は残る、接続を変えると全部捨てる、ログ管理の画面の EventTimeline とは別のもの。
- [ ] **Step 7. 実行の流れ（QueryRunner）**：`query/runner.rs` の取得の段を、UC1 の 3〜8 に置き換える：staging を捨てる → ロググループごとに足りない範囲 → キャッシュの重なる部分を読む（BR2.1） → 残りを選んだ順のロググループ、時刻の順の範囲ごとに今の `run_fetch_with_cache` で取得（受け皿を渡す、BR2.2、R-01） → 振り分け → 終わりを決める（BR2.5）。認証情報の失敗なら残りのロググループを始めない（R-04）。[Cancel] は今の取得を止めて残りを始めない（BR2.4）。失敗したストリームにロググループ名を添える（BR2.6）。評価のあと結果の行を写して持つ（BR3.2）。取得のあと補完の名前を集める（BR3.3）。そのあとテスト（10 件ほど、偽のゲートウェイを使う）：要件の FR2 の受け入れ基準 6 つ（取り足し、すべて取得済みなら AWS を呼ばない、重なったイベントは 1 回だけ、2 つのロググループの同じ名前のストリーム `s1` が `@log` で見分けられる、一部失敗で取得できた分の結果と失敗したストリーム、キャッシュが切でログ管理の画面で取得していてもクエリの画面は取得してログ管理の画面は変わらない）、キャッシュが有効なら重なる部分は AWS を呼ばない、Failed のロググループがあれば評価しない、認証情報の失敗で残りを始めない、[Cancel] で残りのロググループを始めない。
- [ ] **Step 8. 結果の写しと行の読み出し**：`evaluator.rs`・`store.rs` の結果を、行の写し（イベントとロググループ名、多くて 100,000 件）で持つように変え、行の読み出しを runId で確かめる（BR3.2、BR3.4）。そのあとテスト（4 件ほど）：次の実行の取得と評価の間も前の結果の行を読める、FetchFailed・EvaluationFailed で前の結果が消える、runId が違えば空、写しの数は limit と既定の上限で切ったあとの数。
- [ ] **Step 9. クエリの画面の状態（InsightsSession・AppSession）**：ロググループを 50 個までの並びにする（BR4.1、BR4.2）、実行を始めるときに補完の名前を消す（BR5.2）、前の結果を残す（BR3.4）、ロググループごとの取得の結果と進み具合（ロググループ i / n、失敗したロググループの数、キャッシュを読んでいる間、BR2.7、R-04）を SessionView に足す。接続を変えたら選択を空に戻し、QueryStore を全部捨てる。そのあとテスト（6 件ほど）：51 個目は受け付けない、0 個と 50 個の押せる条件、補完の名前が消える、前の結果が残る、接続の変更で選択が空になる。

### Tauri のコマンドと画面（実装してからテストを書いて実行する）

- [ ] **Step 10. Tauri のコマンドと橋渡し**：`update_analytics_input` をロググループの並びにし、行の読み出しを結果の写しから行う形にする。`src/api.ts` の型を合わせる。そのあとテスト（3 件ほど）：入力の形、行の読み出しの形、コマンドの登録と許可の一致（変えていないことの確かめ）。
- [ ] **Step 11. 画面（LogGroupMultiselect・LogAnalyticsPage・QueryResultTable）**：`LogGroupMultiselect.tsx` を 50 個までの複数選択とトークンにし、上限に達したら文で示す（BR4.1）。`LogAnalyticsPage.tsx` の進み具合（ロググループ i / n、失敗の数、キャッシュを読んでいる間）と、失敗の一覧の「ロググループ名 / ストリーム名」（BR2.6）。前の結果を残す表示（BR3.4）。`i18n/messages.ts` に英語と日本語の文言。そのあとテスト（12 件ほど）：複数選択、トークンの × で外す、上限の文、押せない理由、進み具合の表示、失敗の一覧のロググループ名、前の結果が残る、FetchFailed で前の結果が消える。

### 仕上げ

- [ ] **Step 12. 全体の確かめ**：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npm run build` を通す。既存のテストとログ管理の画面の速さのテスト（10 万件 10 秒・100 万件 100 秒、`--release --ignored`）が今と同じに通ることを確かめる。
- [ ] **Step 13. メモリと速さの測り（BR5.3、R-03）**：無視つきのテストで、ログ管理の画面に 100 万件、クエリの画面に 100 万件（50 個のロググループ）、結果の写し 100,000 件を持った状態で、JSON の項目の sort を含むクエリを評価したときのピークのメモリ（OS の最大常駐量）と評価の時間を測り、`code-summary.md` に記録する。U2 の速さの測りも流し直して、遅くなっていないことを確かめる。
- [ ] **Step 14. 記録**：`code-summary.md`（U2 の `code-summary.md` の「権限ファイルはコミットしていない」は誤りで、実際はコミットに含まれていることも書く、U2 の R-05。手元の目視で、本物の Ace の色分け・補完・エラーの下線と印を確かめること、U2 の R-04）、`traceability.json`（BR1.1〜BR5.3 と要件の ID）、`source-manifest.json` を書く。

## 要件・決まりと手順の対応

| 要件・決まり | 手順 |
|---|---|
| FR2.1〜FR2.3、FR2.7（BR1.1〜BR1.6） | Step 2、Step 5、Step 6、Step 7 |
| FR2.2（キャッシュ、BR2.1） | Step 2、Step 4、Step 7 |
| FR2.4〜FR2.6、FR2.8（BR2.2〜BR2.7） | Step 7、Step 9、Step 11 |
| FR7.5・FR7.6（BR3.1〜BR3.4） | Step 8、Step 9、Step 11 |
| FR1.3・FR1.6（BR4.1・BR4.2） | Step 9、Step 11 |
| U2 から回した直し（BR5.1〜BR5.3） | Step 3、Step 9、Step 13、Step 14 |
| NFR2 | Step 13 |

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
