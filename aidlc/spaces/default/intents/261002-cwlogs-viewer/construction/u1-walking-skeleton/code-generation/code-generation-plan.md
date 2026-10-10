# Code Generation Plan — U1 薄い一本（u1-walking-skeleton）

上流の成果物：`construction/u1-walking-skeleton/functional-design/`（functional-spec.md・rules.md・entities.md）、`inception/units-generation/unit-of-work.md`（U1）、`inception/domain-design/components.md`・`decisions.md`、`inception/requirements-analysis/requirements.md`。質問票：`code-generation-questions.md`（Q1：Tauri のコマンドとイベントでつなぐ、Q2：画面側は Vitest と React Testing Library）。

## 1. 目的と範囲

GUI を起動し、手入力した条件（プロファイル名・ロググループ名・ストリーム名・UTC の開始・終了日時）で 1 つのストリームを GetLogEvents で最後のページまで取得し、時刻とメッセージを一覧に表示する最小版を作る。あわせて、同じライブラリを使う手元用の確認用プログラムを作る。U1 の範囲外（一覧からの選択・複数ストリーム・再試行・中断・仮想スクロール・タイムゾーン切替・絞り込み・キャッシュ・行の展開）は作らない。

U1 の FR：FR1.2、FR3.1、FR3.5、FR4.3、FR4.4、FR4.6、FR5.1、FR8.3。ルール：BR1.1〜BR1.8、BR2.1〜BR2.2、BR3.1〜BR3.4、BR4.1〜BR4.5、BR5.1〜BR5.4、BR6.1〜BR6.2、BR7.1〜BR7.2。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| Rust のワークスペース | ルートの `Cargo.toml` に `crates/local-sights-core`（GUI に依存しないライブラリ）と `src-tauri`（Tauri のアプリ、クレート名 `local-sights`）を置く | ADR-001、team.md の Code Style（ライブラリと GUI の分離） |
| 画面側 | リポジトリのルートに Vite + React + TypeScript（`package.json`、`index.html`、`src/`） | Tauri 2 の標準の配置。unit-of-work の前提 |
| GUI | Tauri 2 | units-generation Q3 |
| AWS | `aws-config`・`aws-sdk-cloudwatchlogs`（1.x）、非同期は `tokio` | 認証は SDK に任せる（BR1.7） |
| 日時 | `chrono`（UTC の解釈と表示）。U4 のタイムゾーンでは `chrono-tz` などを足す前提 | 最も広く使われ、ミリ秒の計算が単純 |
| エラー型 | `thiserror` | `unwrap()` / `expect()` を使わず `Result` で返す（team.md） |
| 画面とライブラリのつなぎ方 | 操作は Tauri のコマンド（`update_input`・`start_fetch`・`get_session`）、状態の変化は `session-changed` イベント、ページごとのログのまとまりは `log-batch` イベントで送る | Q1 |
| 画面側のテスト | Vitest + React Testing Library + jsdom | Q2 |
| 画面側の静的検査 | `tsc --noEmit`・Prettier・ESLint（flat config）、`package-lock.json` をコミット。CI への組み込みは CI Pipeline ステージ | unit-of-work U1、F2 |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U1 で作る範囲 |
|------|----------|---------------|
| （共通の検証）FetchRequest | `request.rs` | BR1.1〜BR1.3、BR1.8 |
| TimeRangeModel | `time_range.rs` | BR1.2、BR2.1、BR2.2、BR5.3 の時刻の書式 |
| CloudWatchLogsGateway | `gateway/mod.rs`（trait と型）、`gateway/aws.rs`（AWS SDK の実装）、`gateway/classify.rs`（エラーの分類）、`failure.rs`（ApiFailure と伏せ字） | GetLogEvents だけ（BR3.4）、接続先の解決（BR1.5〜BR1.7）、分類（BR4.2）、安全な詳細（BR4.3） |
| EventFetcher | `paging.rs`（ページ終端の判定）、`fetcher.rs` | BR3.1〜BR3.3 |
| EventTimeline（最小版） | `timeline.rs`、`event.rs`（LogEvent と並べ替えのキー） | 追加・全件の読み出し・破棄、BR5.1 |
| FetchCoordinator（最小版） | `coordinator.rs`（FetchJob と受け口 `FetchSink`） | BR4.1、BR4.5 |
| AppSession（最小限） | `session.rs` | SessionState、BR1.4、BR5.4、phase の遷移 |
| DesktopUi（最小限） | `src/`（画面側）、`src-tauri/src/`（コマンドとイベント） | BR4.4、BR5.2、BR5.3、BR6.1、BR6.2 |
| 確認用プログラム | `crates/local-sights-core/examples/fetch_check.rs` | BR7.1、BR7.2 |

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

この契約の `ordering` をそのまま守る。純粋なロジック（共通の検証・範囲の算出・時刻の書式・ページ終端の判定・並べ替えのキー・伏せ字と安全な詳細・エラーの分類の対応表）はテストを先に書いて失敗を確かめてから実装する。AWS 接続の層（AWS SDK の実装、偽物を使った取得の流れ）と GUI の層（Tauri のコマンド・画面）は先に実装してから、その層のテストを書いて実行する。層ごとの TDD には変えない。

## 4. 手順

### Step 1: プロジェクトの骨組みと本番用の設定（FR 全体の土台、ADR-001）

- [x] ルートの `Cargo.toml`（workspace、`resolver = "2"`、members：`crates/local-sights-core`・`src-tauri`、共通の `[workspace.package]` と `[workspace.dependencies]`）
- [x] `crates/local-sights-core/Cargo.toml` と `src/lib.rs`（2 章のモジュールの宣言だけ）
- [x] `src-tauri/`：`Cargo.toml`、`build.rs`、`tauri.conf.json`（productName `local-sights`、識別子 `dev.local-sights.app`、ウィンドウ 1 枚、CSP を設定、外部への通信は許可しない）、`capabilities/default.json`（`core:default` と、自前のコマンドとイベントだけ）、`icons/`（仮のアイコン）、`src/main.rs`（起動だけ）
- [x] 画面側：`package.json`（scripts：`dev`・`build`・`tauri`・`typecheck`・`lint`・`format:check`・`test`）、`index.html`、`vite.config.ts`、`tsconfig.json`、`eslint.config.js`、`.prettierrc.json`、`.prettierignore`、`src/main.tsx`
- [x] `.gitignore` に `target/`・`node_modules/`・`dist/` を足す。`Cargo.lock` と `package-lock.json` はコミットする（team.md の Code Style、unit-of-work U1）
- [x] テレメトリやクラッシュレポートを送る依存は入れない（project.md Forbidden）

### Step 2: テストの実行環境の準備と単位を絞ったコマンドの記録

- [x] `crates/local-sights-core` で `cargo test -p local-sights-core` が（テスト 0 件で）通ることを確かめる
- [x] 画面側に `vitest`・`@testing-library/react`・`@testing-library/user-event`・`@testing-library/jest-dom`・`jsdom` を入れ、`vitest.config.ts`（environment `jsdom`、setup ファイル `src/test/setup.ts`）を置き、`npx vitest run` が動くことを確かめる
- [x] `unit-test-instructions.md` の単位を絞ったコマンドがそのまま実行できることを確かめる

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

テスト先行の対象（functional-spec.md 7 章、team.md の Testing Posture）。各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [x] `request.rs` のテスト：ロググループ名・ストリーム名の空・空白だけ（BR1.1）、プロファイル名の前後の空白の除去と空欄の扱い（BR1.5 の入力側）、日時の形式の誤りと存在しない日付（BR1.2）、開始 ≥ 終了（BR1.3）、正しい入力で検証済みの条件と TimeRange が返ること（BR1.8）、理由が文言キーの一覧で返ること
- [x] `time_range.rs` のテスト：`yyyy-mm-dd hh:mm:ss` の解釈（UTC）、開始はその秒の 0 ミリ秒（BR2.1）、終了はその秒の 999 ミリ秒（BR2.2）、GetLogEvents に渡す終了は endInstant + 1、うるう年の 2 月 29 日、ミリ秒の UTC 表示 `yyyy-mm-dd hh:mm:ss.mmm`（BR5.3）
- [x] `paging.rs` のテスト（BR3.2）：最初の応答で次のトークンあり → 続ける、最初の応答で次のトークンなし → 終わる、2 回目以降で同じトークン → 終わる、違うトークン → 続ける、空のページでは終わらない、pageCount が最後の同じトークンの応答も数えること
- [x] `event.rs`・`timeline.rs` のテスト：(timestamp, sequence) の昇順（BR5.1）、同じ timestamp では sequence 順、追加・全件の読み出し・破棄、件数で打ち切らないこと（BR3.3）
- [x] `failure.rs` のテスト（BR4.3）：AKIA／ASIA で始まる 20 文字の伏せ字、シークレットアクセスキー風の 40 文字・セッショントークン風の長い文字列の伏せ字、安全な詳細に入れてよい項目（種類・API 名・リクエスト ID・プロファイル名・ロール ARN・アカウント ID・ロググループ名・ストリーム名）だけで組み立てること、伏せ字の対象がないときは変えないこと
- [x] `gateway/classify.rs` のテスト（BR4.2 の対応表）：エラーコードと接続の状態から種類への対応（ExpiredToken・認証情報なし → AuthRequired、AccessDeniedException → AccessDenied、ThrottlingException → Throttled、タイムアウト・接続失敗 → Network、ResourceNotFoundException → NotFound、InvalidParameterException → InvalidInput、それ以外 → Other）、Throttled と Network だけ retryable

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [x] `request.rs`・`time_range.rs`・`paging.rs`・`event.rs`・`timeline.rs`・`failure.rs`・`gateway/classify.rs` を実装し、Step 3 のテストを通す
- [x] テストが通ったまま整理する（重複の除去、名前の見直し）。`unwrap()` / `expect()` はテスト以外で使わない

### Step 5: AWS 接続の層を実装する（BR1.5〜BR1.7、BR3.1、BR3.4、BR4.2、BR4.3）

- [x] `gateway/mod.rs`：trait `CloudWatchLogsGateway`（非同期の `get_log_events` だけを持つ。引数は ConnectionTarget の元になるプロファイル名・ロググループ名・ストリーム名・startTime・endTime・startFromHead・トークン、戻り値はイベントの一覧と次のトークン、または ApiFailure）。読み取り 3 API 以外を呼ぶ手段は作らない（BR3.4、project.md Forbidden）
- [x] `gateway/aws.rs`：AWS SDK の実装。最初の呼び出しの前に接続先を解決する（プロファイル名が空なら指定しない BR1.5、リージョンはプロファイルまたは SDK の既定、なければ GetLogEvents を呼ばずに RegionMissing BR1.6）。認証情報は読み出さず保存もしない（BR1.7）。SDK のエラーは `classify.rs` で分類し、安全な詳細だけを付ける（SDK の生のメッセージは出さない、BR4.3）
- [x] AWS 接続の層のテスト（実装の後）：一時ファイルの AWS 設定（`AWS_CONFIG_FILE`・`AWS_SHARED_CREDENTIALS_FILE` を一時ファイルに向け、`AWS_EC2_METADATA_DISABLED=true`）で、既定のリージョンがあるプロファイル → そのリージョン、ないプロファイル → RegionMissing、空欄 → 既定のプロファイル。いずれも AWS の API は呼ばない（project.md Forbidden）。環境変数を変えるテストは 1 つのテストの中で順に行い、並列実行で干渉しないようにする

### Step 6: 取得の流れを実装する（EventFetcher・FetchCoordinator）→ 偽物の gateway でテストする（BR3.1〜BR3.3、BR4.1、BR4.5）

- [x] `fetcher.rs`：1 ストリームを最後のページまで取得し、毎回 startTime = startInstant・endTime = endInstant + 1・startFromHead = true を渡す（BR3.1）。ページを受けるたびに API が返した順で sequence を振り、呼び出し元に渡す。ページ終端は `paging.rs` で判定（BR3.2）。件数で打ち切らない（BR3.3）。エラーで止まり、StreamFetchOutcome（Failed、それまでの件数とページ数、ApiFailure）を返す
- [x] `coordinator.rs`：trait `FetchSink`（`on_started`・`on_batch(events, total)`・`on_finished(job)`）。FetchJob を作り、EventTimeline を破棄して「開始」を届け、ページごとに EventTimeline に追加して受け口に届け、最後に FetchJob を届ける（BR4.5）。エラーでも取得できた分は残す（BR4.1）
- [x] 偽物の gateway（`crates/local-sights-core/tests/support/fake_gateway.rs`。決めた応答を順に返し、受け取った引数を記録する）
- [x] 結合テスト `crates/local-sights-core/tests/u1_fetch_flow.rs`（実装の後）：空ページを挟む複数ページ、同じトークンで終わる、次のトークンなしで終わる、途中のエラーで取得できた分が残り Failed になる、RegionMissing で GetLogEvents を呼ばない、受け口に届く順序（開始 → まとまり → 結果）、毎回の startTime・endTime・startFromHead の値、取得し直しで前回のログが破棄される

### Step 7: AppSession を実装する → テストする（BR1.4、BR1.8、BR5.4）

- [x] `session.rs`：SessionState（sessionId・phase・入力中の条件・validationErrors・直近の FetchJob の要約・件数）。入力が変わるたびに共通の検証を呼ぶ。phase が Fetching の間は入力の変更と取得の開始を受け付けない（BR1.4）。取得の開始で Fetching、Completed で Done、Failed で Failed。画面に送る表示用の形（serde でシリアライズ）を返す
- [x] `session.rs` のテスト（実装の後）：Idle で始まる、検証を通らないと取得を始められず理由が返る、Fetching 中は入力を変えられない、Completed → Done と件数、0 件の扱い（BR5.4）、Failed → Failed と種類・安全な詳細、Done・Failed から再び取得できる

### Step 8: Tauri のアプリ（DesktopUi のつなぎ）を実装する（Q1、ADR-001）

- [x] `src-tauri/src/lib.rs`：AppSession を `Mutex` で持つ状態、コマンド `get_session`・`update_input(field, value)`・`start_fetch`。`start_fetch` は AppSession の検証を通った条件で非同期に取得を始め、`FetchSink` の実装が `session-changed`（表示用の状態）と `log-batch`（ページごとの LogEvent のまとまりと累計件数）を画面に送る
- [x] AWS SDK の実装（`gateway/aws.rs`）をここで組み込む。秘密の認証情報とアクセスキー ID をログ・画面に出さない（project.md Forbidden）
- [x] このクレートは薄いつなぎだけにし、判断はライブラリ側に置く。自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる（判断はライブラリ側と画面側のテストで確かめる）

### Step 9: 画面側（DesktopUi）を実装する → Vitest でテストする（BR4.4、BR5.2〜BR5.4、BR6.1、BR6.2）

- [x] `src/i18n/messages.ts`（文言キーと英日。OS の言語が日本語なら ja、それ以外は en。画面の文字列はすべてキーから引く BR6.1）
- [x] `src/api.ts`（Tauri のコマンドとイベントの薄い包み。テストでは差し替える）
- [x] `src/App.tsx`・`src/components/FetchForm.tsx`（入力欄 5 つと [Fetch]、`<form>` の送信で Enter でも押せる BR6.2、Fetching 中は無効 BR1.4、押せない理由の表示）・`src/components/LogTable.tsx`（時刻 UTC ミリ秒まで BR5.3、メッセージは改行を空白にして 1 行、はみ出しは省略記号 BR5.2、全件を並べる）・`src/components/StatusLine.tsx`（取得中、件数と 0 件 BR5.4、エラーの種類名と安全な詳細 BR4.4）・`src/hooks/useEscapeKey.ts`（Escape でダイアログを閉じる土台 BR6.2）
- [x] 操作できる要素に `data-testid` を付ける
- [x] 画面のテスト（実装の後）：`src/components/FetchForm.test.tsx`（押せない理由、押せる条件、Fetching 中の無効化、Enter で送信）、`src/components/LogTable.test.tsx`（1 行表示、時刻の書式、全件）、`src/components/StatusLine.test.tsx`（取得中、件数、0 件、エラー表示）、`src/hooks/useEscapeKey.test.tsx`、`src/i18n/messages.test.ts`（全キーが en・ja の両方を持ち空でない、言語の選び方）

### Step 10: 確認用プログラムを作る（BR7.1、BR7.2、team.md Walking Skeleton）

- [x] `crates/local-sights-core/examples/fetch_check.rs`：引数 `--profile`（省略可）・`--log-group`・`--stream`・`--start`・`--end`（UTC）。共通の検証（BR1.8）を通らなければ使い方と理由を標準エラーに出して終了コード 2。`FetchSink` の実装が、届いたまとまりを 1 件 1 行（UTC の時刻ミリ秒まで＋タブ＋改行を空白にしたメッセージ）で標準出力に出し、最後に件数とページ数を標準エラーに出す。Failed なら種類と安全な詳細を標準エラーに出して終了コード 1
- [x] 自動テストと CI からは実行しない（cargo test は example を組み立てるだけで実行しない）。配布するアプリには含めない

### Step 11: ビルドと環境の設定

- [x] `README.md`：前提（Rust stable、Node.js 22、Tauri 2 の OS ごとの前提。Linux は `libwebkit2gtk-4.1-dev` など）、`npm ci`・`npm run tauri dev`・`npm run tauri build`・`cargo install` の手順、確認用プログラムの実行例、秘密情報を置かない注意
- [x] `cargo fmt --check`・`cargo clippy --workspace`（警告はエラーにしない）・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit` が通ることを確かめる（CI への組み込みは CI Pipeline ステージ）

### Step 12: ドキュメントとトレーサビリティ

- [x] 公開関数に doc コメントを付ける
- [ ] `code-summary.md`・`source-manifest.json`・`traceability.json`（BR1.1〜BR7.2 と U1 の FR を、実装またはテストのファイルに対応付ける）を書く

## 5. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR1.2（SDK に任せた認証）・BR1.5〜BR1.7 | Step 5、Step 8 |
| FR3.1・BR1.2・BR2.1・BR5.3 | Step 3、Step 4、Step 9 |
| FR3.5・BR2.2 | Step 3、Step 4 |
| FR3.4（U1 の最小版）・BR1.1・BR1.3・BR1.8 | Step 3、Step 4、Step 7、Step 10 |
| FR4.3・BR3.1 | Step 6 |
| FR4.4・BR3.2 | Step 3、Step 4、Step 6 |
| FR4.6・BR3.3・BR5.1 | Step 3、Step 4、Step 6、Step 9 |
| FR4.8（U1 の最小版）・BR1.4 | Step 7、Step 9 |
| FR5.1・BR5.2 | Step 9 |
| FR8.3・BR4.3 | Step 3、Step 4、Step 5 |
| BR3.4（NFR6） | Step 5 |
| BR4.1・BR4.5 | Step 6 |
| BR4.2 | Step 3、Step 4、Step 5 |
| BR4.4・BR5.4 | Step 7、Step 9 |
| BR6.1・BR6.2（NFR12・NFR13 の土台） | Step 9 |
| BR7.1・BR7.2 | Step 10 |

## 6. 注意点

- 呼ぶ API は GetLogEvents だけ（project.md Forbidden、BR3.4）。
- 自動テストは実際の AWS に接続しない。AWS 設定のテストも一時ファイルだけを読み、インスタンスメタデータも無効にする（project.md Forbidden）。
- 秘密の認証情報とアクセスキー ID を、リポジトリ・ログ・画面のエラー表示に書かない（project.md Forbidden）。
- このコンテナでは Linux 用の Tauri の前提ライブラリ（webkit2gtk など）を入れられない場合がある。その場合 `src-tauri` の組み立ては開発者の手元（GUI の目視確認と同じ場所）で確かめ、その旨を `code-summary.md` に書く。ライブラリ側と画面側のテストは影響を受けない。
- 実際の AWS での確認（確認用プログラム）と GUI の目視は、開発者の手元で本人の認証情報を使って行う（team.md Walking Skeleton）。
