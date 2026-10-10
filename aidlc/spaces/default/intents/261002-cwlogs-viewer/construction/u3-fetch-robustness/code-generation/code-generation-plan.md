# Code Generation Plan — U3 取得の作り込み（u3-fetch-robustness）

上流の成果物：`construction/u3-fetch-robustness/functional-design/`（functional-spec.md・rules.md・entities.md、レビュー 2 回とも READY）、`inception/units-generation/unit-of-work.md`（U3）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1・U2 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）に機能を足す。質問票の回答は `code-generation-questions.md`（Q1：仮想スクロールは自前で作る）。

## 1. 目的と範囲

ストリーム名の手入力をなくし、[Fetch] でロググループの中の、時間範囲に関係するストリームをすべて取得する。ストリームは DescribeLogStreams で最後のイベント時刻の新しい順に列挙し、「範囲の開始 − 1 時間」より古いストリームで打ち切る。対象のストリームを 1 つずつ順に取得し、取得したログは取得したものから逐次、時刻 → ストリーム名 → ストリームの中の順で保持ログに入れる。スロットリングと通信のエラーは倍々に待って最大 5 回再試行し、それでも失敗したストリームは失敗としてまとめ、取得できた分は表示する。画面は保持ログを持たず、表示範囲の行だけを取り寄せて描き（自前の仮想スクロール）、取得中に行が増えても一番上に見えている行を動かさない。

U3 の FR：FR4、FR4.1、FR4.2、FR4.5、FR4.7、FR4.8、FR4.10、FR4.11。NFR：NFR2、NFR3、NFR4、NFR6、NFR12、NFR13。ルール：U3 の BR1.1〜BR6.9（`U1:`・`U2:` の付いた ID は前の単位のルール）。

機能設計の再レビュー（上限の 2 回目）で出た指摘は、project.md の決まりに従って機能設計は直さず、ここで扱いを決める：

- R-09：FetchJob に listingStatus（StreamPlan の写し。StreamPlan がないときは持たない）を持たせる。BR2.1 の上限で呼ばずに失敗としたストリームも、finishedStreamCount に数え、StreamFetchOutcome（pageCount 0、retryCount 0、status Failed、最後のストリームと同じ ApiFailure）と FailedStream を作る。
- R-10：EventTimeline に、ストリームごとの「sequence → timestamp」の索引（ストリーム名ごとに sequence の順の timestamp の列。sequence はストリームごとに 0 から隙間なく振るため配列で引ける）を持たせ、(logStreamName, sequence) から timestamp を引いてから並べ替えのキー (timestamp, logStreamName, sequence) で二分探索する。
- R-11：UC8 の番号の並びは文書だけの問題で、コードには影響しない。機能設計の文書は直さない。

前の単位からの持ち越し：U1 の R-06（取得の開始時に前回の行と件数を消す）は BR5.6 として、U1 の R-08（古い名前）は U3 の rules.md の前書きで扱い済み、U2 の R-06（コマンドの順序）は BR6.7 として、この単位で実装する。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| ストリームの列挙 | `CloudWatchLogsGateway` に `describe_log_streams`（ロググループ名・`orderBy=LastEventTime`・`descending=true`・次のトークンだけを渡す）を足す | BR1.1、BR1.4。読み取り 3 API 以外を呼ぶ手段は作らない |
| 保持ログ | ソート済みの `Vec<LogEvent>` に、受けたページを並べ替えのキーの位置へマージで入れる。ストリームごとの sequence → timestamp の索引を併せて持つ | BR4.1〜BR4.4、R-10。ストリームは古い順に返るため、1 ページは挿入位置から後ろのマージで済む |
| 保持ログの共有 | `std::sync::Mutex<EventTimeline>` を、ページの追加・行の取り出し・位置の問い合わせのたびに短く握る（取得の間ずっと握らない） | 取得中も画面が行を取り寄せられるようにする（NFR3、BR4.3）。U2 の非同期のロックは BR6.7 の世代番号に置き換える |
| 再試行の待ち | 待ち時間の計算は純粋な関数（再試行の回数と乱数の値から待ち時間を返す）。実際の待ちは `tokio::time::sleep`。乱数は差し替えられる `JitterSource`（本番は `fastrand`） | BR2.1、BR2.2。テストは tokio の `test-util`（`start_paused`）で実時間を待たない |
| 中断 | `tokio::sync::watch` の中断フラグを取得の流れに渡し、待ちと各 API 呼び出しの前に確かめる | BR2.3、BR5.5。新しい依存は足さない |
| 画面への行の渡し方 | U1・U2 の `log-batch` イベント（全件を画面に送る）をやめ、コマンド `get_rows(offset, limit)`（RowWindow を返す）と `find_row_position(logStreamName, sequence)` を足す。保持件数と timelineVersion は `session-changed` に入れる | BR4.3、BR4.4、BR6.4。画面は保持ログを持たない |
| 仮想スクロール | 自前（Q1）。行の高さは固定。表示する行と描く位置は純粋な関数で求める。100 万行ぶんの高さがブラウザの要素の高さの上限を超えないよう、スクロールの高さに上限（例 1,000 万 px）を設け、超えるときは位置を比例で縮める | BR6.4、BR6.5、NFR2 |
| 操作の順序 | SessionState に connectionGeneration を持たせ、画面は最後に受けた SessionView の世代番号を `select_log_group`・`reload_log_groups`・`start_fetch` に付けて送る。世代番号が違えばコアは古い操作として捨て、画面には何も返さない | BR6.7、R-02 |
| jobId | 取得を始めるたびに増える整数（U1 の UUID の文字列から変える） | entities.md の FetchJob |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U3 で作る範囲 |
|------|----------|---------------|
| StreamPlanner | `streams/mod.rs`（選定・打ち切り・列挙の状態、純粋なロジック）、`streams/planner.rs`（ページをたどる列挙の流れ） | BR1.1〜BR1.5 |
| RetryPolicy | `retry.rs`（再試行の判断・待ち時間・続けての使い切りの数え方、純粋なロジック。待ちの実行と中断） | BR2.1〜BR2.3 |
| EventFetcher | `fetcher.rs`（ストリームごとの取得、再試行、retryCount） | BR3.1〜BR3.3 |
| EventTimeline | `timeline.rs`（並べ替えのキー、マージでの挿入、行の取り出し、位置の問い合わせ、破棄、timelineVersion）、`event.rs` | BR4.1〜BR4.5 |
| FetchCoordinator | `coordinator.rs`（流れ・結果の状態・失敗のまとめ・受け口・中断） | BR5.1〜BR5.5 |
| CloudWatchLogsGateway | `gateway/mod.rs`・`gateway/aws.rs` | DescribeLogStreams の追加 |
| AppSession | `session.rs`・`request.rs` | BR5.6、BR6.1、BR6.6、BR6.7、進み具合と失敗の一覧の状態 |
| DesktopUi | `src-tauri/src/lib.rs`、`src/components/LogTable.tsx`・`StatusLine.tsx`・`FailureList.tsx`（新規）・`FetchForm.tsx`、`src/virtualScroll.ts`（新規、純粋なロジック）、`src/hooks/useRowWindow.ts`（新規）、`src/api.ts`、`src/App.tsx`、`src/i18n/messages.ts` | BR6.2〜BR6.5、BR6.8 |
| 確認用プログラム | `crates/local-sights-core/examples/fetch_check.rs` | BR6.9 |

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
  "scope": "local-tool",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- 数値のカバレッジ下限は設けない（スコープ `local-tool` には org.md のスコープ別下限がない）。その代わり、GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。\n- テストは `cargo test` で実行し、CI でマージ前に実行する。既存のテストはすべて通った状態を保つ。"
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
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
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
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:1ba5c284be5a836c03c42abd086c6060b6df044056afea8de0055490fa1429c2",
  "contract_sha256": "sha256:12a8505da0cfbdc63db0b40dc33c162212ab7b3fb1ec6447799d4099dfd31ddf"
}
```

この契約の `ordering` をそのまま守る。純粋なロジック（ストリームの選定と打ち切り、再試行の判断と待ち時間と続けての使い切りの数え方、時刻順のマージでの挿入・行の取り出し・位置の問い合わせ、結果の状態の決め方、接続先の世代番号による古い操作の判定、画面の仮想スクロールの行と位置の計算）はテストを先に書いて失敗を確かめてから実装する。AWS 接続の層（DescribeLogStreams の AWS SDK の実装、偽物を使った列挙と取得の流れ）と GUI の層（AppSession のつなぎ、Tauri のコマンド・画面）は先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [x] `crates/local-sights-core/Cargo.toml` に `fastrand`（再試行の待ちのばらつき）を足し、dev-dependencies の `tokio` に `test-util` の機能を足す（待ちを実時間で待たないため）。ほかに新しい依存は足さない。テレメトリを送る依存は入れない（project.md Forbidden）
- [x] `src/lib.rs` に `streams`・`retry` のモジュールを宣言し、クレートの説明の「GetLogEvents だけ」を読み取り 3 API の記述に直す

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [x] U1・U2 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U3 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [x] `streams/mod.rs` のテスト：範囲と重なるストリームだけが対象になり、最初・最後のイベント時刻のどちらかを持たないストリームは対象になる（BR1.3）。lastEventTimestamp が「開始 − 1 時間」ちょうどは対象、1 ミリ秒古いと打ち切り（境界）。打ち切ったページでも、時刻を持たないストリームは位置にかかわらず対象に残り、それより後ろの時刻を持つストリームは対象にならない（BR1.2、R-01）。次のトークンがない・同じトークンで Complete（BR1.1）。エラーで Partial になり、それまでの対象が残る（BR1.5）。対象の並びは列挙で得た順
- [x] `retry.rs` のテスト：Throttled と Network は再試行し、それ以外の種類はすぐ失敗（BR2.1）。n 回目の待ちは 1・2・4・8・16 秒に（1 + ばらつき）を掛けた値で、ばらつきの値 −0.2・0・+0.2 のときの境界（BR2.1）。5 回で使い切り（BR2.1）。成功で回数を 0 から数え直す（BR2.2）。使い切りが続けて 3 ストリームで残りを打ち切り、途中の成功または使い切らない失敗で続けての数が 0 に戻る（BR2.1、R-03）
- [x] `timeline.rs` のテスト：並べ替えのキーは (timestamp, logStreamName, sequence)（BR4.1、U1:BR5.1 の置き換え）。複数のストリームのページを順に足しても、常に全体がキーの順（BR4.2）。同じ時刻は文字列の順のストリーム名、同じストリームは sequence の順。行の取り出しは offset から最大 limit 件で、offset が保持件数以上なら空、totalCount と timelineVersion が一緒に返る（BR4.3）。(logStreamName, sequence) の位置が返り、ないときはない（BR4.4、R-10）。追加と破棄で timelineVersion が増える（BR4.5、R-08）。100 万件を足して、1 回の取り出しと 1 回の位置の問い合わせがそれぞれ 10 ミリ秒以内（NFR2、BR4.4。リリースビルドで測る `#[ignore]` のテストとし、通常の `cargo test` では件数を減らして正しさだけを確かめる）
- [x] `coordinator.rs` の結果の状態のテスト：中断で Aborted、列挙が最初のページで失敗し対象 0 件で Failed、失敗したストリームか Partial で CompletedWithFailures、それ以外は Completed（対象 0 件も Completed）（BR5.2）。finishedStreamCount ≤ plannedStreamCount
- [x] `session.rs` の世代番号のテスト：接続先の変更を確定するたびに connectionGeneration が 1 増え、確認待ちのキャンセルでは増えない。古い世代を付けたロググループの選択・再読み込み・取得の開始は捨てられ（StaleGeneration）、状態が変わらない。同じ世代の操作は受け取った順に反映される（BR6.7）
- [x] `src/virtualScroll.test.ts`：スクロールの位置から表示する最初の行・行数・描く位置を求める。0 件、最後までのスクロール、行数がスクロールの高さの上限を超えるときに比例で縮めても最初と最後の行に届く。位置を保つ計算：一番上ならそのまま一番上、そうでなければ基準の行の新しい位置とずれから新しいスクロールの位置を求める（BR6.4、BR6.5）

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [x] `streams/mod.rs`・`retry.rs`・`timeline.rs`・`event.rs`・`coordinator.rs`（結果の状態の決め方）・`session.rs`（世代番号）・`src/virtualScroll.ts` を実装し、Step 3 のテストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない

### Step 5: AWS 接続の層を実装する → テストする（BR1.4）

- [x] `gateway/mod.rs`：trait に `describe_log_streams`（ロググループ名と次のトークンだけを受け、LogStream の一覧と次のトークン、または ApiFailure を返す。並び順と降順は実装の中で固定する）を足す。`GetLogEventsRequest` の log_stream_name は残す（ストリームごとに呼ぶ）。読み取り 3 API 以外を呼ぶ手段は作らない
- [x] `gateway/aws.rs`：DescribeLogStreams の実装（`orderBy=LastEventTime`、`descending=true`）。接続先の解決とクライアントの使い回しは U2 と同じ。エラーは U1 の分類と安全な詳細を使う
- [x] テスト（実装の後）：DescribeLogStreams の SDK のエラーが U1 の種類（Throttled・Network を含む）に分類され、生のメッセージが捨てられること。LogStream の時刻がないときに None になること。いずれも AWS の API は呼ばない

### Step 6: 列挙と取得の流れを実装する → 偽物の gateway でテストする（BR1.1〜BR1.5、BR2.1〜BR2.3、BR3.1〜BR3.3、BR5.1〜BR5.5）

- [x] `streams/planner.rs`：DescribeLogStreams のページをたどって StreamPlan を作る。ページごとに受け口へ列挙の途中経過（見たストリーム数と対象の数）を届ける（BR5.4、R-06）。エラーは再試行し、それでも失敗したら Partial
- [x] `fetcher.rs`：U1 の `fetch_stream` を、ストリーム名を引数で受ける形にし、エラーを再試行し（BR2.1、BR2.2）、retryCount を数え、sequence をストリームごとに 0 から振る（BR3.2）。中断を受けたら待ちを打ち切り、次の API を呼ばない（BR2.3）
- [x] `coordinator.rs`：新しい FetchJob（jobId、Running）を作り、保持ログを破棄し（BR5.6、BR4.5）、列挙 → ストリームを 1 つずつ取得 → ページごとに保持ログへ追加（保持ログのロックはページごとに短く握る）→ 結果の状態を決める（BR5.1、BR5.2）。失敗したストリームを FailedStream にまとめ（BR5.3）、使い切りが続けて 3 ストリームで残りを呼ばずに失敗とする（BR2.1、R-03、R-09）。受け口に、開始 → 列挙の途中経過 → 計画したストリーム数 → ページごとの追加（追加件数・累計件数・timelineVersion）→ ストリームごとの終了 → 終了の順に届ける（BR5.4）。中断で Aborted にし、保持ログを破棄する（BR5.5）。FetchJob に listingStatus を写す（R-09）
- [x] 偽物の gateway（`tests/support/fake_gateway.rs`）に、DescribeLogStreams の決めた応答と、ストリームごとの GetLogEvents の応答（エラーの列を含む）を足す。呼ばれた API とストリーム名を記録する
- [x] 結合テスト `crates/local-sights-core/tests/u3_fetch_flow.rs`（実装の後）：複数ストリームが時刻順に混ざる、空ページ、列挙の打ち切り（次のページを呼ばない）、列挙の途中のエラー（最初のページで Failed、2 ページ目で CompletedWithFailures）、スロットリングから回復して Completed（受け入れ条件 1）、使い切りで 1 ストリーム失敗し取得できたページが残る（受け入れ条件 2）、使い切りが 3 ストリーム続いて残りを呼ばない（受け入れ条件 3）、中断で待ちを打ち切り Aborted と保持ログの破棄、受け口に届く順序。待ちは `start_paused` で実時間を待たない
- [x] U1 の結合テスト `tests/u1_fetch_flow.rs` を、ストリーム名のない取得の要求と新しい流れに合わせて直す（U1 のルールのうち U3 が置き換えたもの以外の観点は残す）

### Step 7: AppSession を広げる → テストする（BR5.6、BR6.1、BR6.2、BR6.3、BR6.6、BR6.7）

- [x] `request.rs`：取得の要求からストリーム名をなくし、共通の検証（U1:BR1.8）からストリーム名の確認を外す（ロググループ名の確認は残す）（BR6.1）
- [x] `session.rs`：取得の開始時に前回の件数・失敗の一覧・直近の結果を消し、0 件の取得中にする（BR5.6）。進み具合（列挙中の対象の数、計画したストリーム数、終えたストリーム数、累計件数、timelineVersion）と結果（CompletedWithFailures の失敗したストリームの一覧、列挙の失敗）を持ち、SessionView に出す（BR6.2）。失敗の一覧を開く・閉じる（failureListOpen。取得し直すと閉じて中身も消える）（BR6.3）。CompletedWithFailures は phase Done（functional-spec.md の 3 章）。接続先の世代番号を持ち、操作に付いた世代番号を確かめる（BR6.7）。取得中の操作の制限は U1・U2 のまま（BR6.6）
- [x] `session.rs` のテスト（実装の後）：取得の開始で前回の件数・失敗の一覧が消え、一覧が閉じる。進み具合が SessionView に出る。CompletedWithFailures で Done と失敗の数。失敗の一覧の開閉と、取得し直しで閉じる。ストリーム名がなくても [Fetch] が押せる、ロググループがないと押せない。U1・U2 のテストをストリーム名のない形に直す

### Step 8: Tauri のつなぎを実装する

- [x] `src-tauri/src/lib.rs`：保持ログを `std::sync::Mutex` にし、取得の流れにはページの追加のたびに短く握る受け口を渡す。`log-batch` イベントをやめ、コマンド `get_rows(offset, limit)`・`find_row_position(logStreamName, sequence)`・`set_failure_list_open(open)` を足す。`select_log_group`・`reload_log_groups`・`start_fetch` に世代番号の引数を足し、古い世代は何もせずに終える（BR6.7）。接続先の変更で保持ログを破棄するとき timelineVersion を増やす（BR4.5）。ウィンドウを閉じるときに取得を中断する（BR5.5。確認ダイアログは U7）。`update_input` からストリーム名の欄をなくす。`capabilities/default.json` に新しいコマンドの権限だけを足す
- [ ] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる（判断はライブラリ側と画面側のテストで確かめる）

### Step 9: 画面を実装する → Vitest でテストする（BR6.1〜BR6.5、BR6.8）

- [x] `src/api.ts`（新しいコマンドと SessionView の型、`log-batch` の削除）、`src/hooks/useRowWindow.ts`（表示範囲の行を `get_rows` で取り寄せ、要求ごとの番号で古い応答を捨てる。timelineVersion が変わったら取り直し、位置を保つ）、`LogTable.tsx`（3 列・行の高さと列の幅は固定・メッセージは 1 行で省略・表示範囲だけを描く・矢印キー・Page Up・Page Down・Home・End）、`StatusLine.tsx`（列挙中・取得中の進み具合、件数、「N ストリームで失敗」のボタン、列挙が途中までの知らせ）、`FailureList.tsx`（失敗したストリームの名前と種類名・安全な詳細、列挙の失敗を先頭、[閉じる]・Escape、開いたときのフォーカスは [閉じる]。U1 の `useEscapeKey` を使う）、`FetchForm.tsx`（ストリーム名の欄をなくす）、`App.tsx`（世代番号を操作に付ける）、`src/i18n/messages.ts`（U3 の文言キーを英日で足す）。操作できる要素に `data-testid` を付ける
- [x] 画面のテスト（実装の後）：`LogTable.test.tsx`（表示範囲の行だけが描かれる、3 列、キーボードでのスクロール、行が増えても一番上の行が動かない）、`StatusLine.test.tsx`（列挙中・取得中・0 件・失敗の数・列挙が途中まで）、`FailureList.test.tsx`（一覧の中身、フォーカス、Escape と [閉じる]）、`FetchForm.test.tsx`・`App.test.tsx`・`messages.test.ts` の更新（ストリーム名の欄がない、世代番号が操作に付く、文言の英日のそろい）

### Step 10: 確認用プログラムを合わせる（BR6.9）

- [x] `examples/fetch_check.rs`：`--stream` をなくし、この単位の流れでロググループ全体を取得する。1 件 1 行で「UTC の時刻（ミリ秒まで）＋タブ＋ストリーム名＋タブ＋メッセージ」を時刻順に標準出力へ出し、最後に件数・ストリーム数・失敗の数を標準エラーへ出す。失敗したストリームがあれば名前と種類・安全な詳細を標準エラーへ出し、終了コード 1。引数の誤りは使い方を出して終了コード 2

### Step 11: ビルドと環境の設定

- [x] `README.md`：確認用プログラムの使い方から `--stream` をなくし、必要な IAM 権限に DescribeLogStreams を足す。手元の確認の項目に U3 の分（複数ストリームの時刻順、100 万件に近い件数でのスクロール、取得中に一番上の行が動かない、失敗の一覧、列挙中の表示、時刻を持たないストリームの並び（rules.md の「前提と手元の確認の項目」））を足す
- [ ] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 12: ドキュメントとトレーサビリティ

- [x] 公開関数に doc コメントを付ける
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json`（U3 の BR・FR・NFR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR4.1・BR1.1・BR1.4・BR3.1・BR6.1 | Step 3、Step 4、Step 5、Step 6、Step 7、Step 9 |
| FR4.2・BR1.2・BR1.3（R-01） | Step 3、Step 4、Step 6、Step 11 |
| FR4.5・NFR4・BR2.1〜BR2.3（R-03） | Step 3、Step 4、Step 6 |
| FR4.7・BR4.1・BR4.2・BR5.4 | Step 3、Step 4、Step 6、Step 9 |
| NFR2・NFR3・BR4.3・BR4.4（R-10）・BR6.4・BR6.5 | Step 3、Step 4、Step 8、Step 9 |
| FR4.8・BR6.2・BR6.6 | Step 7、Step 9 |
| FR4.10・FR4.11・BR1.5・BR3.3・BR5.2・BR5.3・BR6.3（R-09） | Step 3、Step 4、Step 6、Step 7、Step 9 |
| BR3.2 | Step 6 |
| BR4.5・BR5.5・BR5.6 | Step 3、Step 6、Step 7、Step 8 |
| BR6.7（R-02） | Step 3、Step 4、Step 7、Step 8、Step 9 |
| NFR6 | Step 5 |
| NFR12・NFR13・BR6.8 | Step 9 |
| BR6.9 | Step 10 |

## 5. 注意点

- 呼ぶ API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ（project.md Forbidden、NFR6）。
- 失敗の一覧・ステータス行・標準エラー・アプリの診断ログに出すのは、U1:BR4.3 の安全な詳細だけ（秘密の認証情報とアクセスキー ID を含まない。project.md Forbidden）。
- 自動テストは実際の AWS に接続しない。待ちは実時間で待たない。
- 100 万件の速さのテストはリリースビルドでの `#[ignore]` のテストとし、通常の `cargo test` の時間を延ばさない。画面のスクロールの体感は手元の目視で確かめる。
- 実際の AWS での確認（時刻を持たないストリームの並びを含む）と GUI の目視は、開発者本人の手元で行う。
