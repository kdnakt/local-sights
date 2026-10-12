# Code Generation Plan — アプリから SSO ログインする

## Summary

- Builds: 認証エラーの表示に「SSO にログイン」ボタンを足し、アプリが AWS CLI の `aws sso login` をシェルなしで起動して、中央のダイアログで待機・URL・確認コード・キャンセル・5 分の時間切れを扱う。成功したら SDK のクライアントを捨てて一覧を読み直す
- Touches: crates/local-sights-core（catalog の SSO 判定、新規 sso_login.rs、session.rs、gateway/aws.rs）、src-tauri/src（新規 sso_cli.rs、lib.rs）、src/（api.ts、新規 SsoLoginButton・SsoLoginModal、LogGroupPane、FetchErrorBanner、TopBar、App、i18n）、README.md
- Tests: コアの純粋ロジック約 30（先行）、Tauri 側の CLI 起動 約 8（偽の CLI スクリプトで、実装後）、画面の部品 約 15（実装後）。本物の AWS と AWS CLI は呼ばない

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
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
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Methodology evidence**: 前回のインタビュー（261002-cwlogs-viewer の Q4 の回答 B）で、純粋なロジックはテスト先行、AWS 接続と GUI の層は実装後にテスト、という混在した進め方に決めている。今回のインタビューでも進め方そのものを変える回答はなく（Q6 は記述を CI の実態に合わせるだけ）、git の履歴にもこれと食い違う跡はない。\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- 数値のカバレッジ下限は設けない。スコープごとの下限は org.md のスコープ別の決まりに従う。その代わり、GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。\n- Rust 側のテストは `cargo test` で、画面側（フロントエンド）のテストは Vitest で実行し、どちらも CI でマージ前に実行する。既存のテストはすべて通った状態を保つ。"
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
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:f7f16d7fd154b42b8861e9286af2b827eed247c67272c213f7f8191ed5e31bbe",
  "contract_sha256": "sha256:523f8ece2eaa8d0b36b9422422321c41498301a79dd071beef6fc954bbd6b4b0"
}
```

## 上流と前提

- 要件：`inception/requirements-analysis/requirements.md`（FR1〜FR4、NFR1〜NFR7）。設計：`construction/functional-design/`（entities.md、rules.md の BR1.1〜BR4.3、functional-spec.md の UC1〜UC6 と状態遷移、frontend-components.md）。
- 作業単位は 1 つ。アプリのコードは作業ツリー直下に書き、記録ディレクトリには計画・要約・追跡だけを書く。既存のコードを直接変更する。対話部品には `data-testid` を付ける。
- 自動テストは本物の AWS にも本物の AWS CLI にも接続しない。CLI の起動のテストは、一時ディレクトリに置いた偽の CLI（決まった行を出して決まった終了コードで終わるシェルスクリプト）で行う。
- テスト以外のコードで `unwrap()`／`expect()` を使わない（team.md）。

## 機能設計レビューの指摘への対応

| 指摘 | この計画での扱い |
|---|---|
| Major：ログイン中のアプリ終了と CLI の停止 | ログイン待ちは閉じる確認の対象に入れない（確認なしで閉じてよい）。ウィンドウを閉じるとき、Dock の「終了」・Cmd+Q のとき（Tauri の `RunEvent::ExitRequested`／`Exit`）に、CLI のプロセスグループへ SIGTERM、2 秒待って残っていれば SIGKILL を送る。CLI は新しいプロセスグループで起動する（Step 6） |
| Major：URL の検証と開き方 | URL は `https://` で始まり、ホストが `.amazonaws.com` または `.awsapps.com` で終わるものだけを採る。最初に見つかった 1 つを使う。確認コードがない（PKCE 方式の新しい CLI）場合は URL だけを出す。URL を開くのはアプリ側の `open_sso_url` コマンドで、今のログインが持つ URL と一致するときだけ `/usr/bin/open` を固定パスで起動する（Step 3、Step 6） |
| Minor：クライアント破棄の API | コアの `AwsCloudWatchLogsGateway` に `clear_clients()` を足す（Step 5） |
| Minor：Starting 中のキャンセル、同時発生 | Starting でもキャンセルできる。各ログインに試行番号を付け、終わった試行や古い試行への出来事は無視する。先に届いた出来事が勝つ（Step 4） |
| Minor：再読み込みの失敗 | 成功後の一覧の読み込みが失敗した場合は、既存の一覧のエラー表示に出す（ボタンの条件も既存と同じ） |
| Minor：CLI の起動条件 | 標準入力は null、標準出力と標準エラーの両方を 1 行ずつ読む。1 行は 4096 バイトまでで切り、読んだ行は保持しない（Step 6） |
| Minor：SectionReader 拡張のテスト | Step 2 でテストを先に書く |

## 実装ステップ

### Step 1: テストランナーの確認（契約の runner_step）

- [ ] `cargo +stable test -p local-sights-core catalog` と `/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/node node_modules/vitest/vitest.mjs run src/components/LogGroupPane.test.tsx` が今の状態で通ることを確認し、unit-test-instructions.md のコマンドと照合する

### Step 2: 設定ファイルからの SSO の判定（コア、純粋ロジック：テスト先行）

- [ ] Red：`crates/local-sights-core/src/catalog/mod.rs` のテストに足す：`[profile a]` に `sso_session = x` があれば SSO、`sso_start_url = ...` でも SSO、どちらもなければ SSO でない、`[default]` の SSO、`source_profile = b` の名前を持つ、SSO の値（開始 URL など）と credentials ファイルの行は保持しない、`[sso-session x]` はプロファイルにならない（既存）、キーの前後の空白・コメント行。`cargo +stable test -p local-sights-core catalog` が落ちることを確認する
- [ ] Green：`ProfileSections` に `sso_profiles: BTreeSet<String>` と `source_profiles: BTreeMap<String, String>` を足し、`SectionReader::feed_line` で config ファイルの `sso_session`／`sso_start_url`（有無だけ、値は読まない）と `source_profile`（名前だけ）を記録する。`ConnectionCatalog` から `ProfileAuthInfo`（名前、hasSsoSettings、sourceProfile）を引けるようにする（BR1.1、BR1.2）

### Step 3: ログイン先の決定と CLI 出力の取り出し（コア、純粋ロジック：テスト先行）

- [ ] Red：新規 `crates/local-sights-core/src/sso_login.rs` のテストを書く：
  - `resolve_login_target(selector, catalog, aws_profile_env)`：Named の SSO プロファイル → そのまま・--profile 付き、SdkDefault で AWS_PROFILE なし・[default] が SSO → --profile なし、SdkDefault で AWS_PROFILE=x・x が SSO → --profile なし、AssumeRole（a → source_profile b が SSO）→ ログイン先 b・--profile b、SdkDefault で default → b → --profile b、2 段・5 段、6 段は対象外、循環は対象外、一覧にない source_profile は対象外、SSO でないプロファイルは対象外（BR1.3、BR1.4、BR2.3）
  - `cli_args(target)`：`["sso","login"]` または `["sso","login","--profile",name]`、名前が一覧にないときは作らない（BR2.2）
  - `extract_login_hints(line)`：`https://device.sso.us-east-1.amazonaws.com/` を採る、`https://my.awsapps.com/start/#/device` を採る、`https://oidc.us-east-1.amazonaws.com/authorize?...` を採る、`http://` は捨てる、`https://evil.example.com/` は捨てる、`https://amazonaws.com.evil.example/` は捨てる、確認コード `ABCD-EFGH` を採る、小文字や桁違いは捨てる、行に何もなければ何も返さない、4096 バイトを超える行は先頭だけを見る（BR3.1）
  - `is_allowed_login_url(url)`：上の URL 判定を単独で（Tauri 側の開く前の検証に使う）
  - `cargo +stable test -p local-sights-core sso_login` が落ちることを確認する
- [ ] Green：`sso_login.rs` を実装し、`lib.rs` から公開する

### Step 4: ログインの状態（コア、純粋ロジック：テスト先行）

- [ ] Red：`session.rs` のテストに足す（または `sso_login.rs` の状態の型に）：
  - 認証エラー（一覧・取得）でログイン先があれば `ssoLogin.available = true`、なければ false、認証以外のエラーでは false（BR1.5）
  - `start_sso_login()`：作業中（取得中・一覧の読み込み中・ログイン中）は拒否、そうでなければ試行番号を進めて Starting（BR2.5）
  - Starting → Waiting → Succeeded / Failed(code) / TimedOut / Cancelled、Starting → CliNotFound / Failed / Cancelled
  - URL とコードは最初の 1 つだけ保持し、終わった試行・古い試行番号への出来事は無視する
  - ログイン中はプロファイル・リージョンの選択、取得、一覧の再読み込み、設定ダイアログが拒否される（BR3.4）
  - Succeeded で一覧の再読み込みを要求する出来事を返し、取得は始めない（BR4.1）
  - `dismiss_sso_login()` で結果の状態から Idle に戻る
  - セッションの表示用データ `ssoLogin`（available、state、verificationUrl、userCode、exitCode、attemptId、profileOption）
  - `cargo +stable test -p local-sights-core session` が落ちることを確認する
- [ ] Green：`AppSession` に実装する。既存の閉じる確認の条件は変えない

### Step 5: SDK クライアントの破棄（コアの AWS 境界：実装してからテスト）

- [ ] `crates/local-sights-core/src/gateway/aws.rs` の `AwsCloudWatchLogsGateway` に `clear_clients(&self)` を足す（キャッシュを空にする）。テストを 1 つ足す（クライアントを入れた後に空になる。AWS には接続しない）

### Step 6: CLI の探索と起動（Tauri 側：実装してからテスト）

- [ ] 新規 `src-tauri/src/sso_cli.rs`：
  - `locate_cli(candidates)`：`/opt/homebrew/bin/aws`、`/usr/local/bin/aws` の順に、通常ファイルで実行権限があるものを返す（BR2.1）。候補はテストで差し替えられる引数にする
  - `spawn_login(cli_path, args)`：`std::process::Command`（シェルなし）、標準入力は null、標準出力・標準エラーはパイプ、環境変数はそのまま（BR2.4）、`process_group(0)` で新しいプロセスグループ
  - 出力の読み取り：標準出力と標準エラーをそれぞれのスレッドで 1 行ずつ読み、`extract_login_hints` に通した結果だけをコアへ渡す。行そのものは保持もログもしない（BR3.1、NFR3）
  - 時間切れ：5 分（テストでは短くできる引数）。キャンセル・時間切れ・アプリ終了で `stop_group()`：プロセスグループに SIGTERM、2 秒待って SIGKILL（`libc::killpg`。`libc` を src-tauri の直接の依存に足す）（BR3.2、BR3.3）
  - 終了コード 0 → Succeeded、それ以外 → Failed(code)、起動失敗 → Failed
- [ ] `src-tauri/src/lib.rs`：コマンド `start_sso_login`、`cancel_sso_login`、`dismiss_sso_login`、`open_sso_url(url)` を足す。`open_sso_url` は今の試行の URL と一致し `is_allowed_login_url` を満たすときだけ `/usr/bin/open` を固定パスで起動する。成功時は `clear_clients()` → 一覧の再読み込みを既存の経路で始める。`RunEvent::ExitRequested`／`Exit` とウィンドウを閉じる経路で、ログイン中なら `stop_group()` を呼ぶ。AWS_PROFILE はプロセスの環境から読んでコアに渡す
- [ ] テスト（`sso_cli.rs` のモジュール内、偽の CLI スクリプトを一時ディレクトリに作る）：探索（2 つ目だけある、どちらもない、実行権限なし）、成功（URL とコードを出して 0 で終わる → Succeeded・URL とコードが届く）、失敗（3 で終わる → Failed(3)）、時間切れ（眠り続ける → TimedOut・プロセスが残らない）、キャンセル（→ Cancelled・プロセスが残らない）、子プロセスを作るスクリプトでもグループごと止まる、許可されない URL は届かない。8 つ前後

### Step 7: 画面（React：実装してからテスト）

- [ ] `src/api.ts`：`SessionView` に `ssoLogin` を足し、`startSsoLogin()`、`cancelSsoLogin()`、`dismissSsoLogin()`、`openSsoUrl(url)` を足す。`src/test/fixtures.ts` の既定値も足す
- [ ] 新規 `src/components/SsoLoginButton.tsx`：Cloudscape の `Button`。`available` のときだけ出し、作業中・ログイン中は無効
- [ ] 新規 `src/components/SsoLoginModal.tsx`：Cloudscape の `Modal`。Starting・Waiting：案内、`Spinner` と文字の状態、URL（押すと `openSsoUrl`）、確認コード（あれば）、[キャンセル]。Waiting の間は × と Escape で閉じない。TimedOut・Failed・CliNotFound：文言、代わりの手順（`aws sso login` とプロファイルの指定）、[閉じる]。Succeeded では閉じ、右ペインの上に `Alert type="success" dismissible` を出す
- [ ] `LogGroupPane.tsx` の一覧のエラー表示と `FetchErrorBanner.tsx` のアラートに `SsoLoginButton` を置く。`TopBar.tsx` はログイン中にプロファイル・リージョン・設定を無効にする（既存の無効化の表に「ログイン中」の行を足す）。`App.tsx` でモーダルと成功の知らせを描く。焦点はモーダルを閉じたら押したボタン、なくなっていれば左ペインの再読み込みへ戻す
- [ ] `src/i18n/messages.ts`：frontend-components.md の文言表の key を英・日で足す
- [ ] テスト：`SsoLoginButton.test.tsx`（出る・出ない・無効・押すと呼ぶ、4 つ）、`SsoLoginModal.test.tsx`（待機の表示、コードなしで URL だけ、URL を押すと `openSsoUrl`、キャンセル、待機中は Escape で閉じない、時間切れ・失敗・CLI なしの文言と閉じる、6 つ）、`LogGroupPane.test.tsx`・`FetchErrorBanner.test.tsx`・`TopBar.test.tsx`・`App.test.tsx` に各 1〜2（ボタンの配置、ログイン中の無効化、成功の知らせ）、`messages.test.ts` に新しい key が両言語にあること

### Step 8: 全体の検査

- [ ] `/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/node` で prettier（--write）、eslint、tsc --noEmit、vitest run、vite build を通す
- [ ] `cargo +stable fmt --all --check`、`cargo +stable clippy --workspace --all-targets`、`cargo +stable test --workspace` を通す。`npm audit --audit-level=high` を通す
- [ ] コードに SSO・SSO-OIDC の API 呼び出しと `~/.aws/sso/cache` の読み書きがないことを `grep` で確かめ、結果を code-summary.md に書く（NFR1）

### Step 9: 文書と追跡

- [ ] `README.md`：SSO ログインの使い方、AWS CLI の探索場所、手動確認のチェックリスト（直接の SSO プロファイルと AssumeRole のプロファイルでログインできる、ブラウザが開かないときに URL で入れる、キャンセル・時間切れ・ログイン中の Cmd+Q で `aws` のプロセスが残らない）
- [ ] `construction/code-generation/code-summary.md`：変更したファイル、主な判断、計画からの逸脱、テストの数、手動確認の表
- [ ] `construction/code-generation/traceability.json`：FR1.1〜FR4.2、NFR1〜NFR7 を実装ファイルかテストファイルに対応づける

## 要件・設計との対応

| 要件 | 規則 | ステップ |
|---|---|---|
| FR1.1〜FR1.5（入口） | BR1.1〜BR1.5、BR2.5 | Step 2、3、4、7 |
| FR2.1〜FR2.6（実行と待機） | BR2.1〜BR2.5、BR3.1〜BR3.4 | Step 3、4、6、7 |
| FR3.1〜FR3.4（結果） | BR4.1、BR4.2 | Step 4、5、6、7 |
| FR4.1、FR4.2（CLI がない） | BR2.1 | Step 6、7 |
| NFR1〜NFR7 | BR4.3、BR2.2、BR3.1、BR3.3 ほか | Step 3、6、7、8 |
