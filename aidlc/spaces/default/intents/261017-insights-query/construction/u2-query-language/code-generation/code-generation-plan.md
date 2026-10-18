# Code Generation Plan — U2 クエリの言語をそろえる（u2-query-language）

## Summary

- Builds: クエリの言語の残り（JSON の項目、fields・sort・limit、not (…)、エラーの下線と印、可変の列と打ち切りの表示、行を開いた項目の一覧、README の対応範囲、書く手助け）と、U1 から回した 3 つの直し
- Touches: `crates/local-sights-core/src/query/`（plan・parser・fields・evaluator・runner）、`insights_session.rs`、`src-tauri/src/lib.rs`・`build.rs`・capabilities、`src/api.ts`・`QueryEditor.tsx`・`QueryResultTable.tsx`・`LogAnalyticsPage.tsx`・`i18n/messages.ts`、`README.md`
- Tests: Rust 約 60 件（部品ごとに 5〜8 件。純粋なロジックはテスト先行、コンソールで確かめた結果をそのまま例にする）と Vitest 約 25 件、速さの測り（無視つきのテスト）、既存のテストはすべて通したまま

## 上流と前提

- 設計：`construction/u2-query-language/functional-design/` の `functional-spec.md`（流れと状態の正本）・`rules.md`（BR1.1〜BR10.3）・`entities.md`。コンソールで確かめた結果は `functional-design-questions.md` の Q1a・Q1b。要件 `inception/requirements-analysis/requirements.md`、作業単位 `inception/units-generation/unit-of-work.md`（U2）。
- U1 のコード（`crates/local-sights-core/src/query/`）の上に足す。評価の入口の形（時刻順の（イベント, ロググループ名）の並び）と QueryStore は変えない。U3 の範囲（取得済みの範囲、取り足し、複数のロググループ）は作らない。
- 呼ぶ AWS API は今と同じ 3 つだけ。認証情報はログ・エラー・画面に出さない。自動テストは実際の AWS に接続しない。
- テストの進め方は末尾の Testing Contract のとおり（custom）。テストの量は Standard。既存のテストはすべての OS で通った状態を保ち、行カバレッジ 80% の CI の合否も下げない。
- Should の書く手助け（BR9.1〜BR9.3）は Step 13 にまとめた。範囲 Q9 のとおり、時間が足りなければ外してよいが、この計画では作る。

### 機能設計のレビュー（1 回目）の指摘の反映

設計書は書き換えず、この計画の手順として直す（project.md Corrections）。

| 指摘 | 直し方 | 手順 |
|---|---|---|
| R-01（Major）型の比べ方が確かめた範囲を超えて断定している | 確かめた組（Q1a の 6b・6c〜6e・6f・6g・6h）はそのままテストの例にする。確かめていない組（数字だけの文字列と数の `<`・`>`、数字でない文字列どうしの順序、真偽値、JSON の数と文字列 "200" の実値）は BR1.6 のとおり作るが、コードの説明に「仮の決まり（U4 で確かめる）」と書き、`code-summary.md` の「残っていること」に並べて U4 に引き継ぐ。6a は「文字列の項目と数の項目の比べ」で、アプリは項目どうしの比べを作らないため起きない、と説明に書く | Step 3、Step 4、Step 18 |
| R-02（Major）JSON の項目を使う評価の費用と中断 | 1 回の命令の当てはめの中で、1 つのイベントのメッセージを JSON として読むのは 1 回だけにする（命令をまたいでは持たない）。sort は、並べる行の鍵（FieldValue）を 10,000 件ずつの区切りで取り出して持ち（区切りごとに止める信号を確かめる）、並べ替えは鍵だけで行う。鍵の並べ替えそのものは止められないため、100 万件でかかる時間を Step 17 で測って記録する。列の判定（BR3.4）は、limit で切ったあとの結果の行だけを対象に、最後に 1 回だけ、10,000 件ずつの区切りで行う | Step 5、Step 6、Step 17 |
| R-03（Minor）大文字・小文字の「確かめた」の範囲 | テストの例は確かめた言葉（FILTER・LIKE・IN・Fields・Sort・Desc・Limit）にする。AND・OR・NOT・NOT LIKE・NOT IN は区別せずに読むが、仮の決まりとしてコードの説明と `code-summary.md` に書き、U4 に引き継ぐ | Step 3、Step 18 |
| R-04（Minor）`limit 0` と既定の上限 | `limit 0` は「limit による絞りなし。既定の上限 10,000 は別に効く」として作り、仮の決まりとして `code-summary.md` に書き、10,000 件を超えるロググループでの確かめを U4 に引き継ぐ | Step 6、Step 18 |
| R-05（Minor）sort の数の側・数でない側 | Number と @timestamp（ミリ秒）は数の側、Text は数字だけでも数でない側（自然順の中で数字の塊として比べる）、Boolean は文字列として数でない側、Missing は最小、として作り、`sort @timestamp` の例をテストに入れる | Step 5 |
| R-06（Minor）`a.0` が普通の入れ子として通る | 数字だけの区切り（`a.0`、`resources.0.arn`）は Unsupported（配列の要素）とする。キーが数字のオブジェクトを指したいときは、バッククォートで囲めば JSON のキーとして読む | Step 3、Step 4 |
| R-07（Minor）`not` の 2 つの場面 | 条件の先頭の `not` は次が `(` のときだけ読み、それ以外は Syntax（Q1a の 3b・4d・4e・5e をテストの例にする）。項目の直後の `not like`・`not in` は演算子として別に読む | Step 3 |
| R-08（Minor）EvaluationFailed の結果の囲み、matchedCount の定義 | EvaluationFailed では前の結果を消す（FetchFailed と同じ。違う結果を残すと取り違えるため）。「異常終了」は評価の作業が答えを返さずに終わること（スレッドの panic など）とする。matchedCount は「最後の命令まで当てはめたあと、既定の上限と最後の limit で切る前の件数」とし、途中の limit で減った分は含めない。`filter a | limit 5 | filter b` の例をテストに入れる | Step 6、Step 9 |
| R-09（Minor）名前に `.` を含むキーの見た目 | 補完の候補と、行を開いた一覧では、名前に `.`・空白・バッククォート以外の記号を含むキーをバッククォートつきで出す（BR1.3 の書き方） | Step 4、Step 12、Step 13 |

## 手順

各手順を終えたら、その手順の箱に印を付ける。

### 準備

- [ ] **Step 1. 構成**：`query/` に新しいファイルは足さず、今のファイルを広げる（plan・parser・fields・evaluator・runner）。自然順の比べ方だけは `query/natural.rs` に分ける。新しい Rust のクレートは足さない（JSON は今の `serde_json`）。画面側の新しい依存は足さない（色分けと補完は Ace の仕組みを使う）。
- [ ] **Step 2. テストの道具を確かめる**：`unit-test-instructions.md` の U2 の範囲のコマンドが今のリポジトリで動くことを確かめる。

### 純粋なロジック（テストを先に書いてから実装する）

- [ ] **Step 3. 読み取りの拡張（QueryParser）**：`plan.rs` に Command（Filter・Fields・Sort・Limit）・SortKey・JSON の FieldRef を足し、条件の木のどの部分にも Span を持たせる（BR10.3）。`parser.rs` に fields・sort・limit、`not (…)` だけの否定と項目の直後の `not like`・`not in`（BR2.4、R-07）、and > or（BR2.5）、JSON の項目の名前とバッククォート（BR1.1、BR1.3）、数字だけの区切りは Unsupported（R-06）、limit の 0〜100,000 と 100,001 以上・負の数・小数の Syntax（BR5.1）、`limit any`・`fields … as`・`display` などの Unsupported（BR2.1）を足す。エラーは種類・文言の鍵・言葉・行・列・長さを返す（BR2.2、BR2.3）。先にテスト（8 件ほど）：Q1a の 3a・3c・3d の大文字の言葉（R-03）、4a〜4c の優先順位、3b・4d・4e・5e の `not` の書き方は Syntax、`not (a and b)` は読める、`a.b` と `` `user.id` `` と `` a.`b.c` ``、`a.0` は Unsupported、`limit 100001` は Syntax、エラーの場所と長さ。
- [ ] **Step 4. JSON の項目（EventFieldReader）**：`fields.rs` に、メッセージを JSON のオブジェクトとして読み、道筋で値を取り出す関数と、値の種類（Missing・Text・Number・Boolean）を足す（BR1.2〜BR1.4）。null・オブジェクト・配列は Missing。読めない・壊れた・配列・文字列だけのメッセージは JSON の項目なし。項目の名前の収集（BR1.7、名前に `.` などを含むキーはバッククォートつき、R-09）。先にテスト（8 件ほど）：要件の受け入れ基準の `{"level":"error","http":{"status":500}}`、壊れた JSON、配列だけのメッセージ、null と入れ子のオブジェクトは Missing、真偽値、キーに `.` を含む名前、名前の収集の上限（500 個）と並び。
- [ ] **Step 5. 型の比べ方と自然順**：`evaluator.rs` に BR1.5（ない項目）と BR1.6（型の違う値）の比べ方を、`natural.rs` に BR4.2・BR4.3 の自然順（数の側・数でない側の分け方は R-05 のとおり）を書く。先にテスト（10 件ほど）：Q1a の 5c・5d・5f・5g（ない項目）、6b〜6h（型の違う値。6a は起きないことの説明だけ）、資料の自然順の例（`!:`・`#`・`*%04`・`0#`・`5A`・`111A`・`2345_`・`@`・`@_`・`A`・`A9876fghj`・`a12345hfh`・`0`・`01`・`1`・`2`・`3`）、ない値は昇順で先・降順で後ろ（5h・5i）、`sort @timestamp` は数として並ぶ。確かめていない組のテストには「仮の決まり」と名前を付ける（R-01）。
- [ ] **Step 6. 命令を順に当てはめる評価（QueryEvaluator）**：`evaluator.rs` の評価を、最初の並びを決まった並び（@timestamp の降順、同じ時刻はロググループ・ストリーム・順番の昇順、BR4.4）にしてから、命令を順に当てはめる形にする（BR3.1）。filter は 10,000 件ずつの区切りで、1 回の当てはめの中で 1 つのイベントの JSON は 1 回だけ読む（R-02）。sort は鍵を区切りごとに取り出してから鍵だけで安定に並べる（R-02）。fields の列（BR3.2、BR3.3）、limit と既定の上限 10,000（BR5.1、BR5.2、`limit 0` は R-04 のとおり）、最後に結果の行だけを対象に値のない列を外す（BR3.4、R-02）。QueryResult に列・matchedCount（R-08 の定義）・truncatedAt を足す。先にテスト（8 件ほど）：sort なしは降順（Q1b の 1d）、`sort @timestamp` は昇順（1a）、`sort … desc`、同じ値の並び、`limit 5 | filter …` の順、fields を 2 回、値のない列は出さない（5j）、既定の上限で切ると truncatedAt、`filter a | limit 5 | filter b` の matchedCount、区切りごとに止める信号で止まる。
- [ ] **Step 7. 行の値の読み出しの判断**：`evaluator.rs` の行の範囲の読み出しに、列ごとの値（BR7.1 の書き方）と、1 行の項目と値の一覧（決まった 4 つ、JSON の項目の名前の順、BR7.3、R-09）を足す。先にテスト（5 件ほど）：列の値、値のないセルは空、数と真偽値の書き方、一覧の並び、runId・epoch が違えば空。

### 境界の層（実装してからテストを書いて実行する）

- [ ] **Step 8. 実行の流れ（QueryRunner）**：`runner.rs` の評価の段を、Step 6 の評価に置き換える。区切りごとの進み具合と止める信号は U1 のまま。評価が答えを返さずに終わったら、評価の失敗として知らせる（BR10.2）。取得が終わったら、JSON の項目の名前を集める（BR1.7、Should）。そのあとテスト（6 件ほど、U1 の偽のゲートウェイを使う）：JSON の項目と sort と limit を使うクエリが最後まで進む、並べ替えの区切りの間の [Cancel] で止まる、評価の失敗が FetchFailed ではなく評価の失敗になる、名前の収集。
- [ ] **Step 9. クエリの画面の状態（InsightsSession）**：EvaluationFailed の段階を足し、前の結果を消す（BR10.2、R-08）。評価の終わりの反映で [Cancel] の印が立っていれば、結果を捨てて Cancelled にする（最後の区切りのあと・保持が 0 件のときを含む、BR10.1）。SessionView に列・件数・matchedCount・打ち切りを足す。そのあとテスト（6 件ほど）：最後の区切りのあとの [Cancel]、保持が 0 件のときの [Cancel]、EvaluationFailed で前の結果が消える、打ち切りの要約、接続を変えると EvaluationFailed から Idle に戻る。

### Tauri のコマンドと画面（実装してからテストを書いて実行する）

- [ ] **Step 10. Tauri のコマンド**：`get_query_rows` に列の値を足し、`get_query_row_detail` と `get_completion_candidates`（Should）を足す（functional-spec の 3 章）。build.rs・capabilities・generate_handler にも足し、Tauri のビルドが作る権限ファイルもリポジトリに置く。評価の失敗の知らせを段階に反映する（U1 の回復の処理が「GetLogEvents の失敗」にしないように直す、BR10.2）。そのあとテスト（4 件ほど）：コマンドの登録と許可の一致、行の値の形、行の一覧の形、評価の失敗の段階。
- [ ] **Step 11. 画面とアプリの橋渡し（ApiBridge）**：`src/api.ts` に新しいコマンドの関数と型（列、値、打ち切り、EvaluationFailed、補完の候補）を足す。
- [ ] **Step 12. 結果の表とエラーの印（QueryResultTable・QueryEditor）**：`QueryResultTable.tsx` を可変の列にし、件数と「Showing N」（BR5.3、BR7.2）、行を開いた項目と値の一覧（BR7.3、R-09）を出す。見えている行だけを読む作りは U1 のまま（BR7.4）。`QueryEditor.tsx` に、エラーの範囲の下線と行番号の横の印を出し、文を書き換えたら消す（BR6.1）。EvaluationFailed の表示（BR10.2）。`i18n/messages.ts` に英語と日本語の文言を足す。そのあとテスト（12 件ほど）：列が fields に合う、既定の列、件数と打ち切り、行を開いた一覧、値のないセル、エラーの下線と印、書き換えで消える、EvaluationFailed の文言と前の結果が消える。
- [ ] **Step 13. 書く手助け（Should：QueryEditor・LogAnalyticsPage）**：補完（命令・言葉・決まった項目・JSON の項目の名前、Ctrl+Space、BR9.1）、構文の色分け（命令・項目・文字列・数・正規表現、BR9.2）、例のクエリ（5 つほど、書きかけの文があれば置き換える前に確かめる、BR9.3）。そのあとテスト（8 件ほど。CodeEditor は U1 と同じ差し替えを使い、補完と色分けの規則は Ace に渡す前の純粋な関数で確かめる）：候補の中身、まだ取得していなければ JSON の名前を出さない、色分けの言葉の分け方、例のクエリの置き換えと確認。

### 説明と仕上げ

- [ ] **Step 14. README**：対応範囲の一覧を短く書き、今の動きと食い違う記述（「streams are read one after another」など）を直す（BR8.1）。
- [ ] **Step 15. 手元の確認用プログラム**：`fetch_check.rs` の `--query` が、fields・sort・limit と JSON の項目を使うクエリでも、列と件数と先頭の行を出すようにする。
- [ ] **Step 16. 全体の確かめ**：`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npm run build` を手元で通す。既存のテストとログ管理の画面の速さのテストが今と同じに通ることを確かめる。
- [ ] **Step 17. 速さの測り（NFR1、OQ3）**：取得済み 10 万件と 100 万件で、filter だけ、JSON の項目を使う filter、JSON の項目の sort、fields・sort・limit を合わせたクエリの評価の時間を、無視つきのテスト（`--ignored`、`--release`）で測る。並べ替えの間に [Cancel] が効くまでの遅れも測る（R-02）。結果を `code-summary.md` に記録する。合否の数値は、この結果を見て Build and Test で人と決める（決めた数値は下げない）。
- [ ] **Step 18. 記録**：`code-summary.md`（仮の決まりの一覧と U4 に引き継ぐ確かめ：R-01・R-03・R-04、速さの測りの結果を含む）、`traceability.json`（BR1.1〜BR10.3 をそれぞれの実装かテストのファイルに）、`source-manifest.json` を書く。

## 要件・決まりと手順の対応

| 要件・決まり | 手順 |
|---|---|
| FR4.1〜FR4.5（BR1.1〜BR1.6） | Step 3、Step 4、Step 5 |
| FR3.1・FR3.3〜FR3.8、FR5.2（BR2.1〜BR2.5、BR3.1〜BR3.4、BR4.1〜BR4.4、BR5.1・BR5.2） | Step 3、Step 5、Step 6 |
| FR6.1・FR6.2・FR6.4（BR2.2・BR2.3、BR6.1、BR10.3） | Step 3、Step 12 |
| FR7.1・FR7.3〜FR7.5（BR5.3、BR7.1〜BR7.4） | Step 7、Step 10、Step 12 |
| FR8.1・FR8.2（BR8.1） | Step 14 |
| FR9.1〜FR9.3（BR1.7、BR9.1〜BR9.3、Should） | Step 4、Step 8、Step 13 |
| U1 から回した直し（BR10.1〜BR10.3） | Step 3、Step 8、Step 9、Step 10 |
| NFR1・NFR2 | Step 6、Step 17 |

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
