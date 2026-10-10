# Code Generation Plan — U6 ディスクキャッシュ（u6-disk-cache）

上流の成果物：`construction/u6-disk-cache/functional-design/`（functional-spec.md・rules.md・entities.md、レビュー 1 回目 NOT-READY → 修正 → 2 回目 READY）、`inception/units-generation/unit-of-work.md`（U6）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1〜U5 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）に機能を足す。質問票の回答は `code-generation-questions.md` の Q1（JSON Lines）。

## 1. 目的と範囲

設定ダイアログでディスクキャッシュを有効にでき（既定は無効、設定は覚える）、全ストリームが成功した取得の結果を、取得開始の 5 分前までの部分だけ OS のキャッシュ用フォルダに保存する。同じプロファイル・リージョン・ロググループで範囲がキャッシュ済みの範囲にすっぽり入るときは、AWS を呼ばずにキャッシュから表示する。無効にして保存するか [Clear cache] で全削除する。壊れたキャッシュは捨てて取り直す。

U6 の FR：FR7、FR7.1〜FR7.7、FR7.9（FR7.8 は U6 ごと外せる構成で満たす）。NFR：NFR2、NFR5、NFR8、NFR12、NFR13、NFR15。ルール：U6 の BR1.1〜BR5.5（`U1:`〜`U5:` の付いた ID は前の単位のルール）。

### 機能設計の再レビュー（上限の 2 回目）の指摘の扱い

project.md の決まりに従い機能設計は直さず、ここで扱いを決める。

- R-10：[Save] で無効にしたときは「設定を書く → 全削除 → 両方成功したときだけ閉じる」。削除に失敗したら、cacheEnabled は無効に更新したままダイアログを開いておき、消せなかったことを出す（[Clear cache] で再試行するか、[Cancel]・Escape で閉じられる）。
- R-11：取得のタスクが結果を返さずに終わったとき（`abort_fetch_with_failure`）も、`on_finished` と同じく cacheSaving を false にし、cacheNotices を空にする。
- R-12：`run_fetch` は変えずに、FetchCoordinator に `run_fetch_with_cache` を足す。AWS から取得するときは、受け口を包む `DeferredFinishSink` で `run_fetch` の `on_finished` を受け止めて保留し、書き込みの判断と書き込みのあとに、cacheOutcome・readFailed を入れたジョブで元の受け口の `on_finished` を呼ぶ。`FetchSink` に `on_saving(job_id)` を足し（既定の中身は何もしない）、AppSession は他の進み具合と同じく現在のジョブの番号のときだけ受け付ける。Hit のときはストリーム数（planned・finished）を持たないため、ステータス行にストリーム数を出さず、件数と BR5.3 の cache.hit だけを出す。
- R-13：Hit の確かめは、ヘッダの coveredRanges の形と、読んだイベント（指定範囲のもの）の範囲・並びに対して行う。ファイルが「ない」ときは単にキャッシュなし（readFailed にしない）。権限などで「開けない・読めない」ときは readFailed として AWS から取り直すが、ファイルは消さない。消すのは壊れたと判断できたとき（解釈できない・途中で切れている・知らない版・キーが違う・確かめに通らない）だけ。

### 前の作業単位から U6 に持ち越した点（U5 のコード生成のレビュー）

- U5 R-01：絞り込みの欄の Enter は、0.3 秒待たずにすぐ絞り込む（BR5.4）。
- U5 R-02：`LogFilterInput` は、`set_log_filter` が失敗したとき、または確認ダイアログ・設定ダイアログが閉じたときに、入力欄の文字列と `SessionView.logFilter` を比べ、違えば送り直す。
- U5 R-03：走査中の `filter-progress` の間引き（100 ミリ秒）と「最後の Ready は必ず送る」の判断を、ライブラリ側の純粋な関数 `filter::should_report_progress` に移してテストする。`src-tauri` はそれを呼ぶだけにする。
- U5 R-04：承認済みの U5 の記録は書き換えず、U6 の `code-summary.md` に「U5 の記録の補足」として、U5 の traceability にないテストファイルと `src-tauri/src/lib.rs`（U5:BR1.5・BR2.5）の対応と、U5 の「計画との違い」のうち違いではなかった 1 件を書く。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| ファイルの書式 | JSON Lines。1 行目にヘッダ（formatVersion・CacheKey・coveredRanges）、2 行目以降に 1 行 1 イベント（U3:BR4.1 の順）。serde_json を本番の依存に足す（今は dev のみ） | Q1、BR4.2 |
| ファイル名 | CacheKey の SHA-256 を 16 進 64 文字にして `.cache`。一時ファイルは `.cache.tmp-<乱数>`。`sha2` クレートを足す（MIT OR Apache-2.0） | BR2.2、BR1.4 |
| 書き込み | 同じフォルダの一時ファイルに書いて `rename` で入れ替え。権限は Unix で 0600・0700（`std::os::unix::fs::PermissionsExt`） | BR3.4、BR3.6 |
| 設定ファイル | `settings.json`（`{"formatVersion":1,"cacheEnabled":bool}`）。書き方はキャッシュと同じ入れ替え | BR1.1、BR1.2 |
| 場所 | `src-tauri` の起動時に Tauri のパスの仕組み（`app_cache_dir`・`app_config_dir`）で決め、`AppSession` に渡す。渡さない既定の作り方は無効でディスクに触れない | BR1.6 |
| 読み書きのスレッド | ファイルの読み書きと合わせ込みは `tokio::task::spawn_blocking` で行う。保持ログからの写し取りは区切り（4,096 件）ごとにロックを取って放す | BR3.5、BR4.2 |
| 知らせの順 | `run_fetch_with_cache` が Hit と AWS の両方の流れを持ち、BR3.7 の順で受け口を呼ぶ | BR3.7、R-12 |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U6 で作る範囲 |
|------|----------|---------------|
| LogCache（純粋なロジック） | `cache/plan.rs`（新規。記録する範囲の算出、包含の判定、範囲のつなぎ、イベントの入れ替え、書き込む条件、sequence の振り直し、キーとファイル名、ヘッダと読んだイベントの確かめ、知らせの組み立て） | BR2.1〜BR2.4、BR3.1〜BR3.3、BR4.1、BR4.3、BR5.3 |
| LogCache（ファイル） | `cache/mod.rs`（新規。`LogCache` の読み出し・書き込み・全削除、権限）、`cache/settings.rs`（新規。設定ファイルの読み書き） | BR1.1〜BR1.4、BR1.6、BR3.4、BR3.6、BR4.1、BR4.2 |
| FetchCoordinator | `coordinator.rs`（`FetchJob` に servedFromCache・cacheOutcome・readFailed、`FetchSink::on_saving`、`DeferredFinishSink`、`run_fetch_with_cache`） | BR1.5、BR2.3、BR2.4、BR3.2、BR3.5、BR3.7、BR4.1 |
| AppSession | `session.rs`（場所を受け取る作り方、cacheEnabled・設定ダイアログ・cacheSaving・cacheNotices、ダイアログ中の拒否、保存・取消・全削除、`abort_fetch_with_failure` の後始末）。キャッシュのキーに要る値（プロファイルの kind と名前・リージョン・ロググループ）は `begin_fetch` の結果に入れる | BR1.1、BR1.3、BR1.6、BR3.7、BR5.1、BR5.3、R-10、R-11 |
| FilterEngine | `filter.rs`（`should_report_progress` を足す） | U5 R-03 |
| DesktopUi | `src-tauri/src/lib.rs`（起動時に場所を決めて渡す、コマンド `open_settings`・`cancel_settings`・`save_settings(enabled)`・`clear_cache`、取得を `run_fetch_with_cache` にする、`TauriSink::on_saving`、走査の間引きを `should_report_progress` に置き換え）、`src/components/SettingsDialog.tsx`（新規）、`ConnectionBar.tsx`（[*] ボタン）、`StatusLine.tsx`（保存中・キャッシュの知らせ、Hit のときストリーム数を出さない）、`LogFilterInput.tsx`（Enter ですぐ、失敗時とダイアログが閉じたときの送り直し）、`src/api.ts`、`src/App.tsx`、`src/i18n/messages.ts` | BR5.1〜BR5.4、U5 R-01、R-02 |

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

この契約の `ordering` をそのまま守る。純粋なロジック（`cache/plan.rs` のすべてと `filter::should_report_progress`）はテストを先に書いて失敗を確かめてから実装する。ファイルの読み書き（`cache/mod.rs`・`cache/settings.rs`）、FetchCoordinator のキャッシュの分岐、AppSession のつなぎ、Tauri のコマンド、画面は先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [ ] ワークスペースの `Cargo.toml` に `sha2` を足し、`crates/local-sights-core/Cargo.toml` の `[dependencies]` に `sha2` と `serde_json` を足す（`serde_json` は dev から本番に移す）。`deny.toml` の許可ライセンスに収まることを確かめる
- [ ] `src/lib.rs` に `cache` モジュール（`cache/mod.rs`・`cache/plan.rs`・`cache/settings.rs`）を宣言する

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [ ] U1〜U5 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U6 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [ ] `cache/plan.rs` のテスト：記録する範囲の終わりは min(終了, 開始時刻 − 300,000)、開始より前なら記録なし（BR3.1）。包含の判定は境界のミリ秒を含み、2 つの範囲にまたがれば入らない（BR2.3、BR2.4）。範囲のつなぎは重なり・隣り合い（end + 1 = start）を 1 つにし、離れていればそのまま、開始の早い順（BR3.3）。記録する範囲のイベントの入れ替えで範囲外は残り、範囲内は新しいものだけ、並びは U3:BR4.1 の順（BR3.3）。書き込む条件は Completed・失敗 0・途中で終わっていない・記録できる範囲あり・Hit でない、のすべてのときだけ（BR3.2）。sequence はストリームごとに 0 から振り直す（BR4.3）。キーは kind が違えば名前が同じでも別のハッシュ、ファイル名は 64 文字の 16 進と `.cache`、キャッシュと一時ファイルの名前の型の見分け（BR2.1、BR2.2、BR1.4）。ヘッダの範囲の形の確かめ（重なり・隣り合い・逆順・start > end を壊れたとする）と、読んだイベントの範囲・並びの確かめ（BR4.1、R-13）。知らせの組み立ては Hit・readFailed・SaveFailed から作り、readFailed と SaveFailed は両方入る（BR5.3）
- [ ] `filter.rs` の `should_report_progress` のテスト：前回から 100 ミリ秒未満なら送らない、以上なら送る、Ready は間隔によらず必ず送る、最初の 1 回は送る（U5 R-03）

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [ ] `cache/plan.rs`・`filter::should_report_progress` を実装し、Step 3 のテストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない

### Step 5: LogCache のファイルの読み書きを実装する → テストする（BR1.1〜BR1.4、BR3.4、BR3.6、BR4.1、BR4.2）

- [ ] `cache/settings.rs`：設定ファイルの読み出し（ない・読めない・解釈できない・知らない版は無効、診断ログにだけ残す）と書き込み（一時ファイルから入れ替え、0600、フォルダは 0700 で作り緩ければ直し、直せなければ誤り）
- [ ] `cache/mod.rs`：`LogCache` のヘッダだけの読み出し（1 行目）、指定範囲のイベントの読み出しと確かめ（R-13 の「ない」「読めない」「壊れた」の区別を戻り値で返す）、壊れたときの削除、書き込み（既存を読んで確かめ、通れば入れ替えてつなぎ、通らなければ作り直す。一時ファイルから入れ替え。権限）、全削除（直下の、名前の型が合う通常のファイルだけ。シンボリックリンクはたどらず消さない）。書くのは CacheKey・formatVersion・coveredRanges・CachedEvent だけで、認証情報の型を受け取る関数を持たない
- [ ] テスト（実装の後、`tempfile` の一時フォルダで）：書いて読める、範囲の外のイベントは読まない、一時ファイルから入れ替わる（途中で失敗したら前のまま）、権限 0600・0700、緩いフォルダを 0700 に直す、全削除は名前の型が合う通常のファイルだけでほかのファイルとシンボリックリンクの先は残る、壊れたファイル（途中で切れた・版が違う・キーが違う・範囲の形が違う・並びが違う）は「壊れた」になり消える、ファイルがないのは「キャッシュなし」、書き込み時に既存が壊れていれば今回の範囲だけで作り直す、設定ファイルのない・壊れた・知らない版は無効、書いて読める

### Step 6: FetchCoordinator のキャッシュの分岐を実装する → テストする（BR1.5、BR2.3、BR2.4、BR3.2、BR3.5、BR3.7、BR4.1、R-12、R-13）

- [ ] `coordinator.rs`：`FetchJob` に `served_from_cache`・`cache_outcome`（NotUsed・Hit・Saved・NotSaved・SaveFailed）・`read_failed` を足す。`FetchSink` に `on_saving(job_id)` を足す（既定の中身は何もしない）。`DeferredFinishSink` は `on_finished` だけを保留し、ほかは元の受け口に渡す。`run_fetch_with_cache` は、無効なら `run_fetch` のまま（NotUsed）。有効なら、ヘッダを読み、すっぽり入ればイベントを読んで確かめ、通れば破棄 → on_started → 1 回の追加 → on_batch（1 件以上）→ on_finished（Hit）。入らない・ない・読めない・壊れたなら、`DeferredFinishSink` で `run_fetch` を回し、保留した `on_finished` のジョブで書き込む条件を判断し、書くなら on_saving → 保持ログから区切りごとに写し取る → ブロッキング用のスレッドで書き込む → Saved / SaveFailed、書かないなら NotSaved とし、cacheOutcome・readFailed を入れて元の受け口の on_finished を呼ぶ。途中で終わったときは U3:BR5.5 のまま（NotSaved）
- [ ] テスト（実装の後、U3 の偽物の gateway と一時フォルダで）：無効なら書かず API は U3 のまま、Hit のとき API が 1 回も呼ばれず知らせが BR3.7 の順で件数が合う、0 件でも Hit、入らないとき全部取得して on_saving のあとに on_finished が来る、全部成功で書かれ次の同じ取得が Hit になる、失敗ありと途中で終わったときは書かずに既存も変わらない、記録できる範囲がないときは書かない、壊れたキャッシュは保持ログに何も足さずに取り直してファイルが消え readFailed になる、読めないだけのときは消さない、書き込みに失敗したら SaveFailed で取得結果は残る

### Step 7: AppSession を広げる → テストする（BR1.1、BR1.3、BR1.6、BR3.7、BR5.1、BR5.3、R-10、R-11、R-12）

- [ ] `session.rs`：場所（設定ファイル・キャッシュ用フォルダ）を受け取る作り方を足し、既存の作り方は場所なし（無効・ディスクに触れない）のままにする。起動時に設定を読む。SessionView に cacheEnabled・cacheDirectory・settingsDialog・cacheSaving・cacheNotices・ダイアログの知らせ（消した・消せなかった・設定を書けなかった）を足す。`open_settings`（取得中・確認待ちは拒む）、`cancel_settings`、`save_settings(enabled)`（R-10 の順）、`clear_cache`。settingsDialog が Open のあいだ、取得の開始・接続とロググループの変更・入力の変更・絞り込みの文字列の変更を拒む（U2 の確認待ちと同じ誤りのキー）。`begin_fetch` の結果にキャッシュのキーに要る値を入れる。`on_saving` は現在のジョブのときだけ cacheSaving を true に、`finish_fetch` で false にして cacheNotices を作る。`abort_fetch_with_failure` でも cacheSaving を false にし cacheNotices を空にする（R-11）。新しい取得の開始で cacheNotices を空にする
- [ ] `session.rs` のテスト（実装の後、一時フォルダで）：場所なしの作り方はディスクに触れず無効、設定ファイルの有効で起動すると有効、取得中と確認待ちは設定ダイアログを開けない、ダイアログ中は取得の開始などを拒む、有効にして保存で設定が書かれる、無効にして保存で全削除して閉じる、削除に失敗するとダイアログが開いたまま無効になる（R-10）、設定が書けないとダイアログが開いたまま前の値、古いジョブの on_saving は無視、finish_fetch で cacheNotices ができ cacheSaving が戻る、異常終了でも戻る（R-11）

### Step 8: Tauri のつなぎを実装する

- [ ] `src-tauri/src/lib.rs`：`AppState` を `setup` の中で作り、`app_cache_dir`・`app_config_dir` から場所を決めて `AppSession` に渡す。取得は `run_fetch_with_cache` で行う。`TauriSink::on_saving` は `fetch-progress` を送る。コマンド `open_settings`・`cancel_settings`・`save_settings(enabled)`・`clear_cache` を足し、変わったら `session-changed` を送る。走査の間引きを `filter::should_report_progress` に置き換える（U5 R-03）。`capabilities/default.json`・`build.rs` に新しいコマンドの権限だけを足す
- [ ] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる

### Step 9: 取得とキャッシュの結合テストを書く（実装の後）

- [ ] 結合テスト `crates/local-sights-core/tests/u6_cache_flow.rs`：偽物の gateway・`LogView`・一時フォルダで、取得 → 書き込み → 同じ範囲の取り直しが Hit で API を呼ばず、絞り込み（U5）もかかる。範囲を広げると AWS から取得して範囲がつながり、広げた範囲も Hit になる。開始時刻の 5 分前より新しい部分は記録されず、その部分を含む取り直しは AWS から取得する（FR7.4、FR7.9、BR2.3、BR3.1、BR3.3）

### Step 10: 画面を実装する → Vitest でテストする（BR5.1〜BR5.4、U5 R-01、R-02）

- [ ] `src/api.ts`（設定の 4 つのコマンド、SessionView と進み具合の新しい項目）、`src/components/SettingsDialog.tsx`（新規。チェックボックス・保存場所・注意・[Clear cache]・[Cancel]・[Save]、開いたらチェックボックスに入力位置、閉じたら [*] に戻す、Escape は [Cancel]、知らせの文字）、`ConnectionBar.tsx`（[*] ボタン、取得中・確認待ちは押せない）、`StatusLine.tsx`（cache.saving・cache.hit・cache.readFailed・cache.saveFailed、Hit のときストリーム数を出さない）、`LogFilterInput.tsx`（Enter ですぐ渡す、失敗時とダイアログが閉じたときに `logFilter` と比べて送り直す）、`src/App.tsx`、`src/i18n/messages.ts`（U6 の文言キーを英日で足す。文言は BR5.3 の正本のまま）。操作できる要素に `data-testid` を付ける
- [ ] 画面のテスト（実装の後）：`SettingsDialog.test.tsx`（開いたときの入力位置、Escape で閉じて保存しない、[Save] で保存のコマンド、[Clear cache] でその場のコマンドと知らせ、保存場所と注意が常に出る、閉じたら [*] に戻る）、`ConnectionBar.test.tsx`（[*] が取得中は押せない）、`StatusLine.test.tsx`（4 つの知らせ、readFailed と saveFailed が並ぶ、Hit でストリーム数を出さない）、`LogFilterInput.test.tsx`（Enter ですぐ渡す、失敗したら送り直す、ダイアログが閉じたら送り直す）、`App.test.tsx`、`messages.test.ts`（英日のそろい）の更新

### Step 11: ビルドの設定と手元の確認

- [ ] `README.md` の手元の確認の項目に U6 の分（有効にして取得 → 同じ条件で Hit、5 分前より新しい部分、無効にして保存で消える、[Clear cache]、壊れたファイルで取り直す、100 万件近い書き込みの時間）を足す
- [ ] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 12: ドキュメントとトレーサビリティ

- [ ] 公開関数に doc コメントを付ける
- [ ] `code-summary.md`（「U5 の記録の補足」を含む、U5 R-04）・`source-manifest.json`・`traceability.json`（U6 の BR・FR・NFR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR7.1・FR7.2・FR7.3・BR1.1・BR1.2・BR5.1・BR5.2 | Step 5、Step 7、Step 8、Step 10 |
| FR7.4・NFR15・BR2.1〜BR2.4・BR4.2・BR4.3 | Step 3、Step 4、Step 5、Step 6、Step 9 |
| FR7.5・BR3.3 | Step 3、Step 4、Step 5、Step 9 |
| FR7.6・BR1.3・BR1.4（R-10） | Step 5、Step 7、Step 10 |
| FR7.7・NFR8・BR1.5・BR1.6 | Step 6、Step 7、Step 8 |
| FR7.9・BR3.1・BR3.2・BR3.4 | Step 3、Step 4、Step 5、Step 6、Step 9 |
| NFR2・BR3.5・BR3.7（R-12） | Step 6、Step 7、Step 8 |
| NFR5・BR3.6 | Step 5 |
| BR4.1（R-13） | Step 3、Step 4、Step 5、Step 6 |
| NFR12・NFR13・BR5.3（R-11） | Step 3、Step 7、Step 10 |
| BR5.4（U5 R-01）・U5 R-02 | Step 10 |
| U5 R-03 | Step 3、Step 4、Step 8 |
| U5 R-04 | Step 12 |
| BR5.5 | 確認用プログラムは変えない |

## 5. 注意点

- U6 は AWS の API を新しく呼ばない。Hit のときは 1 回も呼ばない（FR7.4）。
- 自動テストは実際の AWS に接続せず、利用者の `~/Library/Caches/` と設定用フォルダにも触れない（テストごとの一時フォルダを渡す）。
- 認証情報はキャッシュにも設定ファイルにも書かない。書く型を CacheKey・CoveredRange・CachedEvent・設定の 1 項目に限る（project.md Forbidden）。
- ディスクの空きが少ないため、ビルドは `CARGO_INCREMENTAL=0` で行う。
- 実際の大きさでの書き込みの時間と、macOS のフォルダの場所は、開発者本人の手元で確かめる。
