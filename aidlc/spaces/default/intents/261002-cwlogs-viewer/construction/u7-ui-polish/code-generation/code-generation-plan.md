# Code Generation Plan — U7 画面の仕上げ（u7-ui-polish）

上流の成果物：`construction/u7-ui-polish/functional-design/`（functional-spec.md の §6 のルール BR1.1〜BR4.5、frontend-components.md。レビュー 1 回目 NOT-READY → 修正 → 2 回目 NOT-READY（上限、Major 2・Minor 2））、`inception/units-generation/unit-of-work.md`（U7）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1〜U6 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）を仕上げる。

## 1. 目的と範囲

行の直下への全文の展開（複数・コピー可・JSON 整形なし・372 px まで・中でスクロール）と、そのための行の高さが変わる仮想スクロール、↑↓・PageUp・PageDown・Home・End・Enter・Space での行の選択と開閉、グリッドの役割。エラーの「何が起きたか」「次の行動」「詳細」の文への置き換え。取得中にウィンドウを閉じる・アプリを終えるときの確認。最小ウィンドウ 1024×640、ダークモード、英日の文言とキーボードの通し確認、診断ログは標準エラー出力だけ。

U7 の FR：FR1.5、FR4.9、FR5、FR5.2〜FR5.4、FR8、FR8.1〜FR8.3。NFR：NFR2、NFR5、NFR10〜NFR13、NFR16。ルール：functional-spec.md の BR1.1〜BR4.5（`U1:`〜`U6:` の付いた ID は前の単位のルール）。

U7 で新しく選ぶ技術はない（高さの測定は標準の `ResizeObserver`、閉じる求めは Tauri 2 の標準の API）。そのため技術の質問はせず、計画の承認だけを確認する。

### 機能設計の再レビュー（上限の 2 回目）の指摘の扱い

project.md の決まりに従い機能設計は直さず、ここで扱いを決める。

- R-11：discardGeneration の持ち主は LogView に一本化する。`LogView::clear()` の中で 1 増やし（実際に保持ログを空にしたときだけ）、`version()` と同じロックで読めるようにする。`row_positions` の答えと、SessionView（`set_timeline_version` と同じ受け取り方で AppSession が写す）の両方に載せる。増えるのは `clear()` が走るとき＝取得の開始（U3:BR5.6 の破棄。キャッシュから出すときの破棄も同じ、U6:BR3.7）、接続の変更で表示中のログを捨てるとき（U2:BR2.4）、中断（U3:BR5.5）。取得の失敗（Failed）のときは保持ログを捨てないので増えない。画面は `row_positions` の答えと SessionView のうち新しい世代を正にし、世代が変わったら展開と選択を空・一番上にする。
- R-12：縮小は U3 の `virtualScroll.ts` の正規化にそろえる。最大の仮想の上端 = H − 画面の高さ、最大のスクロール位置 = min(H, 1,000 万) − 画面の高さ とし、実際のスクロール位置 = 仮想の上端 × (最大のスクロール位置 / 最大の仮想の上端)。`rowLayout.ts` の `rowAt`・`visibleRange`・`scrollTopForAnchor` は画面の高さを入力に含める。展開がないときに U3 の `virtualScroll.test.ts` がそのまま通ることを確かめ、100 万件で End が末尾の行に届くテストを足す。
- R-13：tabIndex=0 は「選んでいる行の、スクロールする展開部分」だけに付け、ほかの展開部分は tabIndex=-1。role=grid と tabIndex=0 と aria-activedescendant は、見出しの行と行を含む 1 つの要素（一覧の外枠）に付け、見出しの行を aria-rowindex=1 としてその中に入れる。
- R-14：(1) 展開の集合を components.md の SessionState.expandedRows から画面の中に移すことは「意図した契約の変更」として `code-summary.md` に記録し、Build and Test の確認項目に入れる（承認済みの上流は書き換えない）。(2) 高さの見積もりでは、全角の文字（East Asian Width が F・W）を 2 文字ぶんで数える。

R-02 の補足：ExitRequested を受けるため、`src-tauri` の `tauri::Builder::run` を `build()` ＋ `App::run` のコールバック（`RunEvent::ExitRequested` で `api.prevent_exit()`）に変える。

### 前の作業単位から U7 に持ち越した点（U6 のコード生成のレビュー）

- U6 R-01：[Save]・[Clear cache] のファイル操作を、セッションのロックの外のブロッキング用のスレッドで行い、結果だけロックの下で反映する（コマンドを async にして `spawn_blocking`）。
- U6 R-02：書き込み中の印（LogCache か AppSession）を持ち、取得のタスクが異常終了したあとも書き込みが終わるまで設定ダイアログを開けないようにする（全削除のあとの書き戻りを防ぐ）。
- U6 R-03：キャッシュのヘッダにイベントの件数（eventCount）を足し、範囲全体を読むときと書き込みのための読み出しでは終わりで突き合わせて、行の切れ目で切れたファイルを壊れたものと見分ける。Hit の読み出しは範囲内だけを読むため、範囲内の件数（coveredRanges ごとの件数）もヘッダに持ち、読んだ件数と突き合わせる。書式の版を 2 に上げ、版 1 のファイルは知らない版として U6:BR4.1 のとおり捨てて取り直す。
- U6 R-04：保持ログからの写し取りもブロッキング用のスレッドで行う（`spawn_blocking` の中で区切りごとにロックを取って放す）。
- U6 R-06：BR3.7 の「破棄 → on_started」の順（`on_started` の時点で保持ログが空）、Hit のときの中断（Aborted・NotSaved・知らせの順）、壊れたキャッシュを取り直して readFailed と Saved が両方立つ組み合わせ、の 3 つのテストを `coordinator.rs` に足す。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| 行の位置の計算 | 純粋な関数 `src/rowLayout.ts`。展開の累積の二分探索と U3 の正規化 | BR1.4、R-12 |
| 位置の問い合わせ | Tauri のコマンド `row_positions(keys)`、ライブラリは `LogView::positions_of(&[RowKey])` を 1 回のロックで | BR1.7 |
| 破棄の世代 | `LogView` が `discard_generation` を持ち `clear()` で増やす | BR1.6、R-11 |
| 高さの測定 | `ResizeObserver`。見積もりは等幅の文字の幅（起動時に 1 回測る）と全角 2 倍 | BR1.9、R-14 |
| 終了の確認 | `on_window_event` の `CloseRequested` で `api.prevent_close()`、`RunEvent::ExitRequested` で `api.prevent_exit()`、`AppSession` の closeConfirmation、コマンド `confirm_close`・`cancel_close`、終えてよい印は `AppState` の `AtomicBool` | BR3.1〜BR3.4、R-02 |
| エラーの文 | 純粋な関数 `src/errorText.ts`、部品 `ErrorMessage` | BR2.1〜BR2.5 |
| 色 | `color-scheme: light dark` とシステムカラー（Canvas・CanvasText・Highlight・GrayText など）だけ | BR4.2 |

部品とファイルの対応：

| 部品 | ファイル | U7 で作る範囲 |
|------|----------|---------------|
| LogView | `crates/local-sights-core/src/log_view.rs`（`discard_generation`、`positions_of`） | BR1.6、BR1.7、R-11 |
| AppSession | `crates/local-sights-core/src/session.rs`（closeConfirmation、終了の判断、discardGeneration を SessionView に、U6 R-01・R-02 の書き込み中の印と設定の拒否） | BR3.1〜BR3.4、R-11、U6 R-01、R-02 |
| LogCache | `crates/local-sights-core/src/cache/{mod.rs,plan.rs}`（eventCount と範囲ごとの件数、版 2） | U6 R-03 |
| FetchCoordinator | `crates/local-sights-core/src/coordinator.rs`（写し取りをブロッキング用のスレッドに、テストの追加） | U6 R-04、R-06 |
| DesktopUi（Tauri） | `src-tauri/src/lib.rs`（`row_positions`・`confirm_close`・`cancel_close`、閉じる求めの扱い、`build()` ＋ `run`、設定のコマンドを async に）、`src-tauri/tauri.conf.json`（最小 1024×640）、`build.rs`・`capabilities/default.json` | BR1.7、BR3.1〜BR3.2、BR4.1、U6 R-01 |
| DesktopUi（画面） | `src/rowLayout.ts`（新規）・`src/errorText.ts`（新規）・`src/hooks/useRowLayout.ts`（新規）・`useRowPositions.ts`（新規）・`useRowWindow.ts`、`src/components/LogTable.tsx`・`ExpandedMessage.tsx`（新規）・`ErrorMessage.tsx`（新規）・`FetchErrorBanner.tsx`（新規）・`CloseConfirmDialog.tsx`（新規）・`StatusLine.tsx`・`LogGroupPane.tsx`・`FailureList.tsx`、`src/App.tsx`、`src/api.ts`、`src/i18n/messages.ts`、`src/styles.css` | BR1.1〜BR1.10、BR2.1〜BR2.5、BR3.3、BR4.2〜BR4.4 |

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

この契約の `ordering` をそのまま守る。純粋なロジック（`rowLayout.ts`、`errorText.ts`、展開の集合と選択の更新、`LogView::positions_of` と discardGeneration、AppSession の終了の判断、キャッシュの件数の突き合わせ）はテストを先に書いて失敗を確かめてから実装する。Tauri のつなぎ、画面の部品、ファイルの読み書きは先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [x] `src-tauri/tauri.conf.json` のウィンドウの minWidth を 1024・minHeight を 640 にし、width・height がそれ以上であることを確かめる（BR4.1）。新しい依存は足さない
- [x] 新しいファイル（`src/rowLayout.ts`・`src/errorText.ts`・フック・部品）の空の骨組みを置く

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [x] U1〜U6 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U7 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [x] `src/rowLayout.test.ts`：展開の累積と offset、rowAt との往復、展開部分の中の y はその行に属する、U3 の正規化（R-12）での縮小、100 万件と展開ありで End が末尾の行に届く、操作した行より上は動かない、アンカーからのスクロールの位置、見積もりの上限 372 px と全角 2 倍（R-14）、展開がないときに U3 と同じ答え（`virtualScroll.test.ts` の値と比べる）
- [x] `src/errorText.test.ts`：8 種類 × 3 場面、{retry} の差し込み（Fetch・Stream は [Fetch]、Listing は [再読み込み]）、起きない組み合わせは Other、詳細の有無
- [x] `src/hooks/expansionState.test.ts`（展開の集合と選択の更新の純粋な関数）：開閉、破棄の世代で空と一番上、絞り込み・追加では保つ、キーが消えたら同じ位置の行、窓にない行では Enter・Space が何もしない
- [x] `log_view.rs` のテスト：`positions_of` の絞り込みの有無・隠れた行・ない行、`clear()` で discardGeneration が増え追加では増えない（R-11）
- [x] `session.rs` のテスト：終了の判断（取得中・書き込み中・取得中でない、Pending 中のもう一度、確認中の取得の終わり、[Close] で中断と終えてよい印、[Keep fetching]）
- [x] `cache/plan.rs` のテスト：件数の突き合わせ（全体と範囲ごと）、行の切れ目で切れたファイルを壊れたとする、版 1 は知らない版（U6 R-03）

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [x] Step 3 の対象を実装し、テストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない。U3 の `virtualScroll.ts` は `rowLayout.ts` に置き換え、U3 のテストが通ることを確かめる

### Step 5: ライブラリの残りを実装する → テストする（U6 R-01〜R-04、R-06）

- [x] `cache/mod.rs`：ヘッダの eventCount と範囲ごとの件数の書き込みと突き合わせ（U6 R-03）
- [x] `coordinator.rs`：写し取りをブロッキング用のスレッドで行う（U6 R-04）。テスト 3 つを足す（U6 R-06）
- [x] `session.rs`：設定の保存・全削除を「判断と反映」と「ファイル操作」に分け、ファイル操作をロックの外で行える形にする（U6 R-01）。書き込み中の印で、異常終了のあとも書き込みが終わるまで設定ダイアログを開けない（U6 R-02）。SessionView に discardGeneration と closeConfirmation を足す
- [x] テスト（実装の後）：ファイル操作をロックの外で行う形でも R-10（U6）の順が保たれる、書き込み中の印で設定を拒む、版 1 のキャッシュを捨てて取り直す

### Step 6: Tauri のつなぎを実装する

- [x] `src-tauri/src/lib.rs`：コマンド `row_positions(keys)`（LogView の 1 回のロック）、`confirm_close`・`cancel_close`。`CloseRequested` は取得中なら `prevent_close` と Pending、取得中でなければ通す（中断をやめる）。`Builder::run` を `build()` ＋ `run` のコールバックにし、`ExitRequested` を同じく扱う。終えてよい印が立っていれば止めない。`Destroyed` では中断だけ。設定のコマンドを async にしてファイル操作を `spawn_blocking` で行う（U6 R-01）。`build.rs`・`capabilities/default.json` に新しいコマンドの権限だけを足す
- [x] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる

### Step 7: 画面を実装する → Vitest でテストする（BR1.1〜BR1.10、BR2.1〜BR2.5、BR3.3、BR4.2〜BR4.4）

- [x] `useRowLayout`・`useRowPositions`・`useRowWindow`、`LogTable`（グリッドの役割、R-13 の tabIndex と見出し行、選択とキー操作、展開、アンカー）、`ExpandedMessage`（テキストノードだけ、折り返しの CSS、372 px、スクロールするときの入力位置、Escape・Shift+Tab で戻る）、`ErrorMessage`・`FetchErrorBanner`・`StatusLine`（詳細の行を外す）・`LogGroupPane`・`FailureList`、`CloseConfirmDialog`、`App`、`api.ts`、`messages.ts`（BR2.2・BR3.3 の文言、使わなくなったキーを除く）、`styles.css`（システムカラーだけ）。操作できる要素に `data-testid` を付ける
- [x] 画面のテスト（実装の後）：`LogTable.test.tsx`、`useRowPositions.test.tsx`、`ErrorMessage.test.tsx`・`FetchErrorBanner.test.tsx`・`StatusLine.test.tsx`・`LogGroupPane.test.tsx`・`FailureList.test.tsx`、`CloseConfirmDialog.test.tsx`、`App.test.tsx`（Tab の順、closeConfirmation で出る）、`messages.test.ts`（すべてのキーの英日、正本どおり、決め打ちの文字がないことを部品の描画から確かめる）の更新

### Step 8: ビルドの設定と手元の確認

- [x] `README.md` の手元の確認の項目に U7 の分（ウィンドウの閉じる・Cmd+Q・Dock の終了を取得中・書き込み中・取得中でないときに試す、書き込み中に [Close] したあとのキャッシュと一時ファイル、ダークモードの切替、1024×640 で崩れない、100 万件で展開とスクロールの体感、行の展開とキーボード操作、エラーの文）を足す
- [x] 診断ログのファイル出力がないこと（`eprintln!` だけ）を確かめる（BR4.5）
- [x] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 9: ドキュメントとトレーサビリティ

- [x] 公開関数に doc コメントを付ける
- [x] `code-summary.md`（R-14 の契約の変更の記録を含む）・`source-manifest.json`・`traceability.json`（U7 の BR・FR・NFR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR5・FR5.2〜FR5.4・NFR2・BR1.1〜BR1.4、BR1.8、BR1.9（R-12、R-14） | Step 3、Step 4、Step 7 |
| BR1.5〜BR1.7（R-11） | Step 3、Step 4、Step 6、Step 7 |
| NFR13・BR1.10（R-13）・BR4.4 | Step 7 |
| FR1.5・FR8・FR8.1〜FR8.3・BR2.1〜BR2.5 | Step 3、Step 4、Step 7 |
| FR4.9・BR3.1〜BR3.4（R-02） | Step 3、Step 4、Step 6、Step 7、Step 8 |
| NFR11・BR4.1・BR4.2・NFR10 | Step 1、Step 7、Step 8 |
| NFR12・BR4.3 | Step 7 |
| NFR16・NFR5・BR4.5 | Step 8 |
| U6 R-01〜R-04、R-06 | Step 3、Step 5、Step 6 |

## 5. 注意点

- U7 は AWS の API を新しく呼ばない。
- 自動テストは実際の AWS に接続せず、利用者の `~/Library/Caches/` と設定用フォルダにも触れない。
- 展開部分には、ログの本文をテキストノードとしてだけ出す（HTML として解釈しない）。
- ディスクの空きが少ないため、ビルドは `CARGO_INCREMENTAL=0` で行う。
- 閉じる・終えるの実際の動き、ダークモード、1024×640、100 万件の体感は、開発者本人の手元で確かめる。
