# Code Generation Plan — U2 接続とロググループの選択（u2-connection-selection）

上流の成果物：`construction/u2-connection-selection/functional-design/`（functional-spec.md・rules.md・entities.md、レビュー 2 回とも READY）、`inception/units-generation/unit-of-work.md`（U2）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）に機能を足す。

## 1. 目的と範囲

上部バーでプロファイル（先頭は「既定の設定（SDK に任せる）」）とリージョンを一覧から選び、左ペインに DescribeLogGroups で取ったロググループ一覧を出し、大文字・小文字を区別しない部分一致で絞り込んで 1 つ選べるようにする。表示中のログがあるときに接続を変えると確認ダイアログを出す。U1 のプロファイル名・ロググループ名の手入力はなくし、ストリーム名は U3 まで手入力のまま残す。

U2 の FR：FR1、FR1.1、FR1.3、FR1.4、FR2、FR2.1〜FR2.3。ルール：U2 の BR1.1〜BR5.2（`U1:` の付いた ID は U1 のルール）。

機能設計の再レビュー（上限の 2 回目）で出た指摘は、project.md の決まりに従って機能設計は直さず、ここで扱いを決める：

- R-11：確認用プログラムは、`--profile` を省略したら「既定の設定」（SdkDefault）、指定したらその名前のプロファイル（Named）として ConnectionProfile を作る。リージョンの引数は足さない（U2 の BR2.8 のとおり、プロファイルの既定のリージョンで接続する）。
- R-12：設定ファイルを読めなかったときの知らせ（BR1.3）には、ファイルの種類（config か credentials）だけを出し、パスも中身も出さない。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| 設定ファイルの読み取り | 自前の最小の読み取り（見出し行と `region` の行だけを見て、ほかの行は値を保持せずに捨てる） | BR1.2。AWS SDK の設定の読み取りは認証情報の値もメモリに読み込むため使わない |
| 設定ファイルの場所 | 環境変数 `AWS_CONFIG_FILE`・`AWS_SHARED_CREDENTIALS_FILE`、なければホームの `.aws/config`・`.aws/credentials`。ホームの場所は `home` クレートで得る | BR1.1 |
| リージョンの一覧 | アプリに組み込んだ静的な一覧（標準のパーティションの公開リージョン） | BR1.4。Rust の AWS SDK は公開リージョンの一覧を API として出していないため、ルールの「組み込みの一覧」を使う。ネットワークは使わない |
| 一覧の取得 | `CloudWatchLogsGateway` に `describe_log_groups`（次のトークンだけを渡す）を足す | BR3.9 |
| 接続 | 取得の要求にリージョンを足し、AWS SDK のクライアントは（プロファイル, リージョン）ごとに使い回す | BR2.8 |
| 画面とのつなぎ方 | U1 と同じく、操作は Tauri のコマンド、状態の変化は `session-changed` イベント（ロググループ一覧の状態も含める） | U1 の Q1 |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U2 で作る範囲 |
|------|----------|---------------|
| ConnectionCatalog | `catalog/mod.rs`（プロファイル一覧と既定のリージョン、純粋なロジック）、`catalog/files.rs`（設定ファイルの読み込み）、`catalog/regions.rs`（リージョンの一覧） | BR1.1〜BR1.5 |
| LogGroupBrowser | `log_groups/mod.rs`（一覧の状態・並べ替え・絞り込み、純粋なロジック）、`log_groups/listing.rs`（ページをたどる取得の流れ） | BR3.1〜BR3.10、BR4.1 |
| CloudWatchLogsGateway | `gateway/mod.rs`・`gateway/aws.rs` | DescribeLogGroups の追加、リージョンの指定、クライアントの使い回し |
| AppSession | `connection.rs`（接続の選択と確認の要否の状態遷移、純粋なロジック）、`session.rs`、`request.rs` | BR2.1〜BR2.8 |
| DesktopUi | `src-tauri/src/lib.rs`、`src/components/ConnectionBar.tsx`・`LogGroupPane.tsx`・`ConfirmDialog.tsx`、`src/components/FetchForm.tsx`（手入力欄の削除と選択中のロググループ名の表示）、`src/i18n/messages.ts` | BR5.1、BR5.2、BR3.8 の表示 |
| 確認用プログラム | `crates/local-sights-core/examples/fetch_check.rs` | R-11 の扱い |

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

この契約の `ordering` をそのまま守る。純粋なロジック（設定ファイルの中身からプロファイル一覧と既定のリージョンを作る処理、リージョンの選択肢、一覧のページの終わりの判定と並べ替え、絞り込み、接続の選択の状態遷移と確認の要否）はテストを先に書いて失敗を確かめてから実装する。AWS 接続の層（設定ファイルの読み込み、DescribeLogGroups の AWS SDK の実装、偽物を使った一覧の取得の流れ）と GUI の層（AppSession のつなぎ、Tauri のコマンド・画面）は先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [x] `crates/local-sights-core/Cargo.toml` に `home` を足す（ホームの場所を得るため）。ほかに新しい依存は足さない。テレメトリを送る依存は入れない（project.md Forbidden）
- [x] `src/lib.rs` に `catalog`・`log_groups`・`connection` のモジュールを宣言する

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [x] U1 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U2 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [x] `catalog/mod.rs` のテスト：config と credentials の中身（文字列）から、`[profile 名前]`・`[default]`・credentials の `[名前]` を集めて重複を除いた名前の昇順の一覧を作り、先頭に SdkDefault を置く（BR1.1）。認証情報の項目の値が結果に含まれない（BR1.2）。Named の既定のリージョンは config のそのセクションの region。SdkDefault の既定のリージョンは、渡した環境変数の値（AWS_REGION → AWS_DEFAULT_REGION → AWS_PROFILE のプロファイルの region → [default] の region）の順（BR1.5）。壊れた行があっても読めた分で作る
- [x] `catalog/regions.rs` のテスト：一覧がコードの昇順で空でない（BR1.4）。既定のリージョンが一覧にないときに足される（BR1.5）
- [x] `log_groups/mod.rs` のテスト：次のトークンがなければ終わり、直前に送ったトークンと同じなら終わり、違えば続ける（BR3.1）。ページを足すと名前の昇順に並ぶ（BR3.2、BR3.3）。絞り込みは前後の空白を除き、大文字・小文字を区別しない部分一致、空なら全件（BR3.7）。0 件・絞り込みで 0 件の区別（BR3.10）。途中のエラーで Partial になり取得できた分が残る（BR3.4）。listingId が違う応答と、いまの一覧がないときの応答は捨てる（BR3.6）
- [x] `connection.rs` のテスト：起動時は未選択（BR2.1）。既定のリージョンがあるプロファイルを選ぶとリージョンが入り、ないと未選択（BR2.2）。両方が決まると一覧の取得を始める指示が出る（BR2.3）。表示中のログがあると変更は確認待ちになり、ないとすぐ適用される（BR2.5）。確認待ちで「変える」なら適用、「キャンセル」なら元の選択に戻る。適用で一覧の取得が無効になる（BR2.3、BR3.6）。確認待ちの変更は、プロファイルかリージョンの少なくとも一方を持つ

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [x] `catalog/mod.rs`・`catalog/regions.rs`・`log_groups/mod.rs`・`connection.rs` を実装し、Step 3 のテストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない

### Step 5: AWS 接続の層を実装する → テストする（BR1.3、BR2.8、BR3.9）

- [x] `catalog/files.rs`：設定ファイルの場所を決めて読む（BR1.1）。ファイルがなければ 0 件で知らせなし、読めない・解釈できないなら 0 件で、ファイルの種類だけを知らせとして返す（BR1.3、R-12）。読むときに見るのは見出し行と region の行だけ（BR1.2）
- [x] `gateway/mod.rs`：trait に `describe_log_groups`（次のトークンだけを受け、ロググループの一覧と次のトークン、または ApiFailure を返す）を足す。`GetLogEventsRequest` にリージョン（省略可）を足す。読み取り 3 API 以外を呼ぶ手段は作らない（BR3.9）
- [x] `gateway/aws.rs`：DescribeLogGroups の実装。リージョンが指定されていればそのリージョンで接続し、なければ U1 と同じくプロファイルの既定のリージョンで解決する（BR2.8、U1:BR1.6）。クライアントは（プロファイル, リージョン）ごとに使い回す。エラーは U1 の分類と安全な詳細を使う
- [x] テスト（実装の後）：一時ファイルの設定ファイルで、ファイルなし・読めない（ディレクトリを指す）・正常の各場合（BR1.3）。DescribeLogGroups の SDK のエラーが U1 の種類に分類され、生のメッセージが捨てられること。リージョンの指定がある・ないときの接続先の解決。いずれも AWS の API は呼ばない（インスタンスメタデータも無効にする）

### Step 6: 一覧の取得の流れを実装する → 偽物の gateway でテストする（BR3.1〜BR3.6、BR4.1）

- [x] `log_groups/listing.rs`：新しい listingId で DescribeLogGroups のページをたどり、ページごとに受け口へ一覧の状態を届ける。エラーで止めて Partial を届ける。いまの listingId と違えば次のページを呼ばずにやめる
- [x] 偽物の gateway（`tests/support/fake_gateway.rs`）に DescribeLogGroups の決めた応答を足す
- [x] 結合テスト `crates/local-sights-core/tests/u2_log_group_listing.rs`（実装の後）：複数ページ、0 件、途中のエラー（最初のページ・2 ページ目）、同じトークンの防御、取り直しで古い取得がやめられ応答が捨てられる、AuthRequired・AccessDenied でも止まらず Partial

### Step 7: AppSession を広げる → テストする（BR2.4、BR2.6〜BR2.8、BR3.5、BR3.8）

- [x] `session.rs`・`request.rs`：接続の選択（`connection.rs`）、ロググループ一覧の状態、絞り込みの文字列、選んだロググループを SessionState に足す。接続の変更の適用で、選択とログ・件数・直近の結果を消して phase を Idle に戻す（BR2.4）。取得中と確認待ちは選択と再読み込みを受け付けない（BR2.6）。検証を、画面だけの選択の検証と、共通の検証（U1:BR1.8。ロググループ名が空でないことも残す）に分ける（BR2.7）。取得の要求に選んだプロファイルとリージョンを入れる（BR2.8）。再読み込みは選択とログを変えない（BR3.5）。表示用の形に一覧・絞り込み・選択中の名前・確認待ちを足す
- [x] `session.rs` のテスト（実装の後）：接続の変更の適用で Done・Failed から Idle、ログと件数と失敗の表示が消える。取得中と確認待ちの操作の拒否。未選択の理由と共通の検証の理由の両方。選んだプロファイル・リージョン・ロググループが取得の要求に入る。再読み込みで選択とログが変わらない。ロググループの選び直しでログが消えない

### Step 8: Tauri のつなぎを実装する

- [x] `src-tauri/src/lib.rs`：起動時に設定ファイルを読んでカタログを作る。コマンド `select_profile`・`select_region`・`confirm_connection_change`・`cancel_connection_change`・`reload_log_groups`・`update_log_group_filter`・`select_log_group` を足し、U1 の `update_input` からプロファイル名とロググループ名の欄をなくす。一覧の取得を非同期で始め、受け口が `session-changed` を送る。`capabilities/default.json` に新しいコマンドの権限だけを足す
- [x] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる（判断はライブラリ側と画面側のテストで確かめる）

### Step 9: 画面を実装する → Vitest でテストする（BR5.1、BR5.2、BR3.8 の表示、BR3.10）

- [x] `src/components/ConnectionBar.tsx`（プロファイルとリージョンの選択、未選択の状態、設定ファイルの知らせ）、`LogGroupPane.tsx`（絞り込みの欄、[再読み込み]、一覧、読み込み中・途中まで・0 件・うながし、矢印キーと Enter・スペースで選ぶ）、`ConfirmDialog.tsx`（文言と [変える]・[キャンセル]、開いたときのフォーカスは [キャンセル]、Escape で閉じる。U1 の `useEscapeKey` を使う）、`FetchForm.tsx`（プロファイル名とロググループ名の手入力欄をなくし、選択中のロググループ名を常に表示）、`App.tsx`、`src/i18n/messages.ts`（U2 の文言キーを英日で足す）。操作できる要素に `data-testid` を付ける
- [x] 画面のテスト（実装の後）：`ConnectionBar.test.tsx`、`LogGroupPane.test.tsx`、`ConfirmDialog.test.tsx`、`FetchForm.test.tsx`・`App.test.tsx`・`messages.test.ts` の更新（手入力欄がない、選択中の名前、取得中の無効化、確認ダイアログの流れ、文言の英日のそろい）

### Step 10: 確認用プログラムを合わせる（R-11）

- [x] `examples/fetch_check.rs`：`--profile` を省略したら SdkDefault、指定したらその名前の Named として取得の要求を作る。リージョンの引数は足さない。U1 の出力と終了コードは変えない

### Step 11: ビルドと環境の設定

- [x] `README.md` の手元の GUI の確認の項目に U2 の分（プロファイルとリージョンの選択、既定のリージョンがないプロファイル、一覧と絞り込み、確認ダイアログ、権限のない接続での表示）を足す
- [x] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 12: ドキュメントとトレーサビリティ

- [x] 公開関数に doc コメントを付ける
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json`（U2 の BR と FR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR1.1・BR1.1〜BR1.3 | Step 3、Step 4、Step 5、Step 9 |
| FR1.3・BR1.4・BR1.5・BR2.2 | Step 3、Step 4、Step 7、Step 9 |
| FR1.4・BR2.3・BR2.4・BR3.6 | Step 3、Step 4、Step 6、Step 7 |
| FR1・BR2.1・BR2.5 | Step 3、Step 4、Step 7、Step 9 |
| FR2.1・BR3.1〜BR3.3・BR3.10 | Step 3、Step 4、Step 6、Step 9 |
| FR2.2・BR3.7 | Step 3、Step 4、Step 9 |
| FR2.3・BR2.7・BR3.8 | Step 7、Step 9 |
| BR2.6・BR2.8・BR3.5 | Step 5、Step 7、Step 8 |
| BR3.4・BR4.1 | Step 3、Step 6 |
| BR3.9 | Step 5 |
| BR5.1・BR5.2 | Step 9 |
| R-11・R-12 | Step 5、Step 10 |

## 5. 注意点

- 呼ぶ API は DescribeLogGroups と GetLogEvents だけ（project.md Forbidden、BR3.9）。リージョンの一覧のために API を呼ばない（BR1.4）。
- 設定ファイルから認証情報の値を読み出さず、保持もせず、エラーや知らせにファイルの中身を出さない（BR1.2、BR1.3、project.md Forbidden）。
- 自動テストは実際の AWS に接続しない。設定ファイルのテストは一時ファイルだけを使い、インスタンスメタデータも無効にする。
- U1 から持ち越した R-06（取得開始時に前回の行と件数を消す）と R-08（U1 の rules.md の applies_to の名前）は U3 で扱う（U2 では扱わない）。
- 実際の AWS での確認と GUI の目視は、開発者本人の手元で行う。
