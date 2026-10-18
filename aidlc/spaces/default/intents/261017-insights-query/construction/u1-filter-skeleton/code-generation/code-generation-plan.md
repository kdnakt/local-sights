# Code Generation Plan — U1 薄い一本（u1-filter-skeleton）

## Summary

- Builds: Log Analytics の画面で filter のクエリを書いて実行し、クエリの画面専用の保持に取得してから手元で評価して結果を表に出す薄い一本（ログ管理の画面は変えない）
- Touches: 中核ライブラリに `query/`（読み取り・項目・評価・日時の文字列・QueryStore・実行の流れ）と `insights_session.rs` を足し、`session.rs`・`src-tauri/src/lib.rs`・`build.rs`・capabilities・`src/api.ts`・`App.tsx`・画面の部品 5 つ・`messages.ts`・`fetch_check.rs`・`package.json`（ace-builds）を変える
- Tests: Rust 約 55 件（部品ごとに 5〜8 件。純粋なロジックはテスト先行）と Vitest 約 30 件、既存のテストはすべて通したまま

## 上流と前提

- 設計：`construction/u1-filter-skeleton/functional-design/` の `functional-spec.md`（流れと状態の正本）・`rules.md`（BR1.1〜BR7.3）・`entities.md`。要件 `inception/requirements-analysis/requirements.md`、作業単位 `inception/units-generation/unit-of-work.md`（U1）。
- この計画の範囲は U1 だけ。JSON の項目・fields・sort・limit・エラーの下線と色分け・補完（U2）、取得済みの範囲・取り足し・複数のロググループ（U3）は作らない。
- 呼ぶ AWS API は今と同じ DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ。秘密の認証情報とアクセスキー ID はログ・エラー・画面に出さない（project.md Forbidden）。自動テストは実際の AWS に接続しない（今の FakeLogs などの偽物を使う）。
- テストの進め方は末尾の Testing Contract のとおり（custom：純粋なロジックはテスト先行、境界の層は実装してからテスト）。テストの量は Standard（部品ごとに 5〜8 件）。refactor のため新しい下限は足さないが、既存のテストはすべての OS で通った状態を保ち、行カバレッジ 80% の CI の合否も今のまま下げない。

### 機能設計のレビュー（1 回目）の指摘の反映

機能設計のレビューで残った指摘は、この計画の手順として直す。設計書（機能設計の成果物）は書き換えず、ここに直し方を書く（project.md Corrections：レビューを記録した後の修正は設計書を直さない）。

| 指摘 | 直し方 | 手順 |
|---|---|---|
| R-01（Major）[Cancel] を押しても評価に進みうる | InsightsRun に「[Cancel] が押された」印を持たせる。取得が Completed・CompletedWithFailures で終わっても、印があれば評価せずに Cancelled にする。評価中も同じ印で次の区切りで止める | Step 9、Step 10 |
| R-02（Major）取得の 1 本化が AppSession の一部の判定にしか広がっていない | AppSession に「どちらかの画面で取得中」を 1 つの関数にまとめ、今 `Phase::Fetching` だけを見ている判定（設定の画面を開く・ディスクを使う作業中・SSO のログイン・接続の変更・入力の変更・[Cancel] の押せる条件・閉じる確認で取得を止める）をすべてそれに置き換える。クエリの取得でも、キャッシュに書いている間は [Cancel] を出さない（今の cache_saving と同じ） | Step 10、Step 12 |
| R-03（Minor）parse_in_zone はハイフン区切りとミリ秒を読まない | 日時の文字列の書き方の読み取りは新しく書く。曖昧な時刻と存在しない時刻の扱いだけ、今の time_range の決まり（早い方、存在しなければ不備）をそのまま使う | Step 5 |
| R-04（Minor）実行可否の確認から段階の更新までの排他 | 読み取りと日時の変換はロックの外で済ませ、実行可否の確認・FetchOwner の取得・前の結果の消去・段階を Fetching にするまでを AppSession の 1 回の呼び出し（1 回のロック）で行う。読み取りのあとに状態が変わっていれば断る | Step 10 |
| R-05（Minor）QueryResult.rows の参照と行のキーの形 | 行のキーは QueryHoldings の中の位置（整数）とし、epoch で守る（ストリーム名の文字列を行ごとに持たない）。entities.md の「BR7.3」は BR6.3 の誤り | Step 7、Step 8 |

## 手順

各手順を終えたら、その手順の箱に印を付ける。

### 準備

- [x] **Step 1. 構成と依存関係**：`crates/local-sights-core/src/query/mod.rs` を作り、`lib.rs` に `pub mod query;` と `pub mod insights_session;` を足す（中身は以降の手順で書く）。画面側に `ace-builds`（Cloudscape の CodeEditor が使う Ace の本体。画面設計で決めた CodeEditor のため）を `package.json` と `package-lock.json` に足す。`npm audit --audit-level=high` が通ることを確かめる。新しい Rust のクレートは足さない（正規表現は今の `regex`、日時は今の `chrono`・`chrono-tz`）。
- [x] **Step 2. テストの道具を確かめる**：`unit-test-instructions.md` の U1 の範囲のコマンド（`cargo test -p local-sights-core query::` など）が、今のリポジトリで動くことを確かめる（まだテストがないため 0 件で通る）。

### 純粋なロジック（テストを先に書いてから実装する）

- [x] **Step 3. クエリの形と読み取り（QueryParser）**：`query/plan.rs` に QueryPlan・Condition・FieldRef・Literal・Span・QueryProblem（Syntax・Unsupported、言葉と場所）を置く。`query/parser.rs` に言葉の切り出しと読み取りを書く。先に `parser.rs` のテストを書いて失敗させ、それから実装する。テスト（8 件ほど）：`|` の並びと `#` の注記（BR3.1）、filter 以外の命令は Unsupported・知らない言葉は Syntax（BR3.2）、言葉の大文字・小文字（BR3.3）、優先順位と括弧・閉じていない括弧の場所（BR3.4）、文字列・数値・正規表現・値の並びと閉じていない引用符（BR3.5）、決まった 4 つ以外の項目は Unsupported（BR3.6）、toMillis(@timestamp) だけを読む（BR3.7）、=~ に文字列は Syntax・@timestamp に like は Unsupported・算術の演算子は Unsupported（BR5.4、BR5.6、BR5.7）、読めない正規表現と大きすぎる正規表現は Syntax（BR3.8）。行・列は Unicode のスカラー値で数える。
- [x] **Step 4. 決まった項目（EventFieldReader の一部）**：`query/fields.rs` に、イベントとロググループ名から @timestamp・toMillis(@timestamp)・@message・@logStream・@log の値を読む関数を書く（BR4.1）。先にテスト（5 件ほど）：それぞれの値、@log はロググループ名だけ、改行を含むメッセージ。
- [x] **Step 5. 日時の文字列をミリ秒に直す**：`query/instant.rs` に、`YYYY-MM-DD HH:MM:SS[.sss]`（選んでいるタイムゾーン）と `YYYY-MM-DDTHH:MM:SS[.sss]Z`（UTC）を読む関数を書く（BR5.9、FR5.6）。書き方の読み取りは新しく書き、曖昧な時刻・存在しない時刻の扱いは今の time_range の決まりを使う（R-03）。先にテスト（8 件ほど）：2 つの書き方、ミリ秒を省いたら 0 ms、Asia/Tokyo の例が 1756053806000 になる（要件の受け入れ基準）、夏時間で曖昧な時刻は早い方、存在しない時刻は Syntax、13 月や 25 時は Syntax、`yesterday`・`2025/08/24`・`+09:00` 付きは Syntax で受け付ける書き方を示す、Z は大文字だけ。
- [x] **Step 6. 評価（QueryEvaluator）**：`query/evaluator.rs` に、条件の木を（イベント, ロググループ名）に当てはめる判定と、時刻順の並びを 10,000 件ずつ区切って評価する入口を書く（BR5.1〜BR5.5、BR5.8、BR5.10）。区切りごとに止める信号を確かめる。先にテスト（8 件ほど）：要件の受け入れ基準の `filter @message like /ERROR/ and @message not like /timeout/`、文字列の = と順序の比べ方、文字列の項目と数値は満たさない（!= は満たす）、like の文字列は大文字・小文字を区別し (?i) で区別しない、in と not in は全体の一致、@timestamp と数値・toMillis が同じ結果（BR5.10）、filter を 2 つつないだら両方を満たすもの（BR3.2）、止める信号で途中までの結果を捨てる。
- [x] **Step 7. 結果の行の読み出しの判断**：`query/evaluator.rs` に QueryResult（runId、epoch、行のキー＝QueryHoldings の中の位置の整数、評価した件数）と、（runId, epoch, 先頭, 件数）で行の範囲を返す判断を書く（BR6.3、BR6.4、R-05）。先にテスト（5 件ほど）：一致すれば範囲の行、runId か epoch が違えば空、範囲の外は詰める、0 件。

### 境界の層（実装してからテストを書いて実行する）

- [x] **Step 8. QueryStore の最小版**：`query/store.rs` に QueryHoldings（key、EventTimeline、epoch）を持つ QueryStore を書く。今の `TimelineStore` と `HeldEvents` を持たせ、今の取得の流れの行き先にできるようにする（BR2.1、BR2.2）。実行の前の捨て方（key を変えて epoch を 1 増やす）、接続を変えたときの全部捨て（BR7.2）、評価のための（イベント, ロググループ名）の並びの貸し出し（位置で引ける形、R-05）を持つ。そのあとテスト（6 件ほど）：捨てると epoch が増える、ページを足すと時刻順、HeldEvents で範囲の写し、接続の変更で key がなくなる、ログ管理の画面の EventTimeline とは別のもの。
- [x] **Step 9. 実行の流れ（QueryRunner）**：`query/runner.rs` に、取得 → 評価の非同期の流れを書く。取得は今の `run_fetch_with_cache` を、行き先に QueryStore を渡して使う（取得の流れは変えない）。終わりの状態で分ける（Completed は評価、CompletedWithFailures は一部失敗の印を立てて評価、Failed は評価しない、Aborted は何もしない）。「[Cancel] が押された」印があれば、終わりの状態にかかわらず評価せずに Cancelled にする（R-01、BR2.7）。評価は区切りごとにロックを外して進み具合を知らせる。そのあとテスト（7 件ほど、今の FakeLogs と偽の知らせ先を使う）：成功で評価まで進む、一部のストリームの失敗で一部失敗の印と結果、一覧の失敗で FetchFailed、取得中の [Cancel] で評価しない、最後のストリームのあとに [Cancel] が届いて Completed で終わっても評価しない（R-01）、評価中の [Cancel] で途中の結果を捨てる、ログ管理の画面の EventTimeline に何も足されない（BR1.7）。
- [x] **Step 10. クエリの画面の状態と取得の 1 本化（InsightsSession・AppSession）**：`insights_session.rs` に AnalyticsInput と InsightsRun（段階、runId、[Cancel] の印、一部失敗、失敗したストリーム、結果の要約、エラー、経過時間）と、[Run query] を押せる条件と理由（BR1.5 の順、10,000 文字の上限）を書く。`session.rs` の AppSession に InsightsSession と FetchOwner を持たせ、次を足す：実行を始める 1 回の呼び出し（実行可否の確認・FetchOwner の取得・前の結果の消去・段階を Fetching、R-04）、取得の終わりと評価の終わりの反映（古い runId は捨てる、BR6.2）、[Cancel] の印（R-01）。今 `Phase::Fetching` だけを見ている判定（`can_open_settings`・`open_settings`・`is_busy_with_disk`・`is_busy_for_sso_login`・`ensure_can_change`・`can_change_connection`・`fetch_cancel_available` と、閉じる確認の取得を止める判定）を「どちらかの画面で取得中」の 1 つの関数に置き換える（R-02、BR2.3、BR2.8、BR7.3）。ログ管理の画面の [Fetch] を押せない理由に「Log Analytics でクエリを実行中」を足す。接続を変えるときの確認に「クエリの画面に結果の行がある」を足し、変えたらクエリの画面のロググループの選択と結果を消し、クエリの文と期間は残す（BR7.1、BR7.2）。SessionView にクエリの画面の状態を足す。そのあとテスト（`insights_session.rs` に 8 件ほど、`session.rs` に 8 件ほど）：押せない理由の順、空白だけと 10,000 文字超え、続けて 2 回始めても 1 本だけ（R-04）、クエリの取得中はログ管理の [Fetch]・設定の画面・SSO のログイン・接続の変更ができない（R-02）、ログ管理の取得中は [Run query] を押せない、クエリの結果だけがあるときに接続を変えると確認が出る（Q8）、接続を変えるとクエリの文と期間が残る、閉じる確認でクエリの取得も止まる、キャッシュに書いている間は [Cancel] を出さない、古い runId の知らせを捨てる。既存の session.rs のテストはそのまま通す。

### Tauri のコマンドと画面（実装してからテストを書いて実行する）

- [x] **Step 11. Tauri のコマンド**：`src-tauri/src/lib.rs` に `update_analytics_input`・`run_query`・`cancel_query`・`get_query_rows` を足し（functional-spec の 3 章）、`generate_handler!`、`build.rs` のコマンドの一覧、`capabilities/default.json` の許可にも足す。`run_query` は読み取りと日時の変換をしてから AppSession の 1 回の呼び出しで始め、QueryRunner を別の非同期の処理で走らせ、進み具合を今の `session-changed` で知らせる。止める信号の持ち手は、今のログ管理の画面の [Cancel] と同じく、始める操作の中で登録する。そのあとテスト（5 件ほど）：今ある「generate_handler と build.rs の一覧が一致する」テストに新しいコマンドが入る、capabilities にも入る、行の読み出しの形。
- [x] **Step 12. 画面とアプリの橋渡し（ApiBridge）**：`src/api.ts` に新しいコマンドの関数と、SessionView のクエリの画面の部分の型を足す。
- [x] **Step 13. ナビゲーションと画面の枠（AppNavigation・AppShell）**：`src/components/AppNavigation.tsx`（Cloudscape の SideNavigation）を作り、`App.tsx` で 2 つの画面を切り替える。起動時は Log Management。Ctrl/Cmd+1・2、閉じると [≡] だけを残し、開閉はアプリを閉じるまで覚える、切り替えたら行き先の最初の欄に焦点、画面名の読み上げ（BR1.1）。ログ管理の画面の中身は変えない。テスト（6 件ほど）：切り替え、キー、開閉、焦点、ログ管理の画面が残る。
- [x] **Step 14. Log Analytics の画面（LogAnalyticsPage・LogGroupMultiselect・QueryEditor・QueryResultTable）**：`LogAnalyticsPage.tsx` にロググループ（`LogGroupMultiselect.tsx`、U1 は 1 つだけ）・期間（今の日時の欄と同じ）・クエリ欄（`QueryEditor.tsx`、CodeEditor。Cmd/Ctrl+Enter、Tab は次の部品へ）・実行の行（[Run query]、押せない理由、[Cancel]）・結果の囲み（`QueryResultTable.tsx`、@timestamp と @message の列、行を開くとメッセージ全体と @logStream・@log、見えている行だけを読む、件数）を置く。状態ごとの表示（No results yet、処理中と進み具合、No matching events、一部失敗の警告と失敗の一覧、取得のエラー、Query cancelled、読み取りのエラーの文と行・列とカーソルの移動）を出す（BR1.2〜BR1.6、BR2.4〜BR2.9、BR3.9、BR6.1〜BR6.4）。操作できる部品には data-testid を付ける。`i18n/messages.ts` に英語と日本語の文言を足す（NFR9）。そのあとテスト（4 つの部品で合わせて 24 件ほど）：押せない理由、Cmd/Ctrl+Enter と押せないときに何もしない、実行中は欄を書き換えられない、各状態の表示、読み取りのエラーでクエリの文が残る、0 件と失敗の区別、行を開く。
- [x] **Step 15. 接続を変えるときの確認の文言**：今の確認ダイアログの文に、両方の画面のログと結果が消えることを足す（BR7.1）。テスト（2 件）：クエリの結果だけがあるときの文、ログ管理の画面だけのときの文。

### 確認用プログラム、仕上げ

- [x] **Step 16. 手元の確認用プログラム**：`crates/local-sights-core/examples/fetch_check.rs` に、クエリの文を受け取り、QueryStore に取得して評価し、件数と先頭の行を出す使い方を足す（team.md Walking Skeleton の 2）。`cargo test` は例をコンパイルするだけで実行しない。手順を `unit-test-instructions.md` に書く。
- [x] **Step 17. 全体の確かめ**：`cargo fmt --check`、`cargo clippy --workspace`、`cargo test`（ライブラリとアプリ）、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run` を手元で通す。既存のテストが 1 つも壊れていないこと、ログ管理の画面の速さのテスト（10 万件 10 秒・100 万件 100 秒）が今と同じに通ることを確かめる。
- [x] **Step 18. 記録**：`code-summary.md`、`traceability.json`（BR1.1〜BR7.3 をそれぞれの実装かテストのファイルに）、`source-manifest.json`（この作業単位で作った・変えたファイルの一覧）を書く。

## 要件・決まりと手順の対応

| 要件・決まり | 手順 |
|---|---|
| FR1.1・FR1.8（BR1.1、BR1.7） | Step 13、Step 9、Step 10 |
| FR1.2〜FR1.7（BR1.2〜BR1.6） | Step 10、Step 14 |
| FR2.1・FR2.4〜FR2.6（BR2.1〜BR2.9） | Step 8、Step 9、Step 10、Step 11、Step 14 |
| FR3.1・FR3.2・FR5.5（BR3.1〜BR3.9） | Step 3、Step 14 |
| FR4.1（BR4.1） | Step 4 |
| FR5.1〜FR5.4・FR5.6（BR5.1〜BR5.10） | Step 3、Step 5、Step 6 |
| FR6.3（BR3.9） | Step 10、Step 14 |
| FR7.2・FR7.6（BR6.1〜BR6.4） | Step 7、Step 10、Step 14 |
| A2・ADR-005・ADR-007（BR7.1〜BR7.3） | Step 10、Step 15 |
| team.md Walking Skeleton の確かめ方 | Step 16、Step 17 |

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
