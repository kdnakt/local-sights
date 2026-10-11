# Code Generation Plan — 画面を AWS マネジメントコンソールに寄せる

## Summary

- Builds: 既存の React/Tauri 画面を Cloudscape Design System に置き換え（黒い上部ナビゲーション、ロググループの表、コンソール風の条件エリア、欄ごとのエラー表示、アラート、ダイアログ）、日付を yyyy/mm/dd に、開始・終了を日付欄（カレンダー付き）＋時刻欄に分ける。コア側は日付の書式をスラッシュに変える
- Touches: src/（main.tsx、App.tsx、styles.css、i18n/messages.ts、新規 dateTime.ts、components/ の全部品）、package.json（Cloudscape 3 パッケージ）、crates/local-sights-core/src/time_range.rs・date_input.rs・session.rs のテスト、README.md
- Tests: コアの書式テスト約 10（先行）、画面の純粋ロジック dateTime.ts 約 10（先行）、部品テスト約 40（実装後、既存テストの更新を含む）、回帰テスト 2（書式のスラッシュ化と欄ごとのエラー表示）

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
  "scope": "bugfix",
  "test_strategy": "standard",
  "project_type": "brownfield",
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
      "Include a targeted regression for the bug or vulnerability.",
      "Keep the existing test suite green."
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
  "input_sha256": "sha256:7baa6108ef232a0197e96a65fb55a729de071d78538316ab0963fa7ca9136abc",
  "contract_sha256": "sha256:c0e9bd0a04e3db80129d20e6aa809fb05ca8a4523845e62d613c1a4b2593f709"
}
```

## 上流と前提

- 要件：`inception/requirements-analysis/requirements.md`（FR1〜FR4、NFR1〜NFR7、CN1〜CN4）。
- 画面案：`inception/refined-mockups/mockups.md`、`interaction-spec.md`（§8 コアとのやり取り、§9 文言）、`design-system-mapping.md`、`accessibility-checklist.md`。画面案のレビュー R-01〜R-07 への対応はこの計画の各ステップに織り込む（下の「レビュー対応」）。
- ゼロ Unit の 1 回の実装。アプリのコードは作業ツリー直下（`src/`、`crates/`）に書き、記録ディレクトリには計画・要約・追跡だけを書く。
- 既存のコードを直接変更する（複製ファイルは作らない）。対話部品には `data-testid` を付ける。

## レビュー対応（画面案 R-01〜R-07）

| 指摘 | この計画での扱い |
|---|---|
| R-01 時刻が空の扱い（FR1.4 との矛盾） | 画面案 Q4 の「自動で補う」を採用する。要件の FR1.4・AS1 はこの計画で置き換える（code-summary.md に明記）。空の時刻で Enter を押したとき・[Fetch] を押したときも、焦点が外れたときと同じ値（開始 00:00:00・終了 23:59:59）を先に補ってから判定する（Step 3、Step 7） |
| R-02 validationErrors と欄の対応表、表示のタイミング | `src/dateTime.ts` に key → 欄の表を持つ（Step 3）。画面側の形式判定を先に行い、コアの `startFormat`/`endFormat` が画面側の判定と食い違った場合は日付欄に出す。理由は起動直後から出す（コンソールの必須欄と同じ。選択の理由は見出し行の直下、入力の理由は欄の直下） |
| R-03 性能の測り方 | Step 9 に測定条件（Tauri 実機・100 万行・操作・合否・記録先）を書く |
| R-04 維持する既存の振る舞いの一覧 | Step 12 で code-summary.md に「維持した振る舞い」の表を書き、各部品のテストで確かめる（Step 5〜9 のテスト項目） |
| R-05 上部ナビゲーションの無効化の表 | Step 5 の表のとおり実装し、テストで確かめる |
| R-06 絞り込み 0 件の文言、Alert の Escape、無効化時の焦点 | Step 10 に文言を足す。Alert は閉じるボタンだけ（Escape は使わない）。[Fetch] が無効になるとき焦点は絞り込み欄へ移す（Step 7） |
| R-07 DatePicker のハイフン入力 | Step 1 で実機確認し、画面側の形式判定（Step 3）を DatePicker の値より先に通す |

## 実装ステップ

### Step 1: 依存の追加と動作確認（構造と設定）

- [x] `npm install @cloudscape-design/components @cloudscape-design/global-styles @cloudscape-design/design-tokens`（Apache-2.0。`package-lock.json` を更新）
- [x] `npm audit --audit-level=high` が通ることを確認する
- [ ] Tauri の実機（`npm run tauri dev`）で、DatePicker・TimeInput・TopNavigation・Modal が WKWebView で描画されることを確かめる（AS4）。DatePicker に `2024-03-01` を打ったときの振る舞いを確認し、結果を code-summary.md に書く（R-07）

### Step 2: テストランナーの確認（契約の runner_step）

- [x] `npx vitest run src/components/FetchForm.test.tsx` と `cargo test -p local-sights-core time_range` が今の状態で通ることを確認し、その出力を unit-test-instructions.md の記載と照合する

### Step 3: コアの書式変更（純粋ロジック：テスト先行）

- [x] Red：`crates/local-sights-core/src/time_range.rs` のテストを `yyyy/mm/dd hh:mm:ss` に書き換え、追加する：スラッシュの解析、ハイフンは `Format` エラー、`2024/02/30` は `Format` エラー、表示が `yyyy/mm/dd hh:mm:ss.SSS`、入力の書き戻しが `yyyy/mm/dd hh:mm:ss`、夏時間の飛び・重複（既存）、年 9999 超え（既存）。`date_input.rs` のテストも同じ書式に書き換える。`cargo test -p local-sights-core time_range date_input` が落ちることを確認する
- [x] Green：`INPUT_FORMAT` を `%Y/%m/%d %H:%M:%S`、`DISPLAY_FORMAT` を `%Y/%m/%d %H:%M:%S%.3f`、`has_input_shape` の区切りを `/` に、エラー文言を `yyyy/mm/dd hh:mm:ss` に変える。`parse_utc_seconds`（`fetch_check` の例）も同じ解析を使うので、例のドキュメントとテストも更新する
- [x] `session.rs`・`coordinator.rs`・`cache/`・`examples/` など、旧書式の文字列を使うテストと文言を `grep -rn "yyyy-mm-dd\|[0-9]-[0-9][0-9]-[0-9][0-9] [0-9][0-9]:" crates/` で洗い出し、スラッシュに揃える。`cargo test -p local-sights-core` 全体を通す
- [x] `cargo fmt --all` と `cargo clippy -p local-sights-core --all-targets` を通す

### Step 4: 画面側の純粋ロジック（テスト先行）

- [x] Red：`src/dateTime.test.ts` を書く：`splitDateTime("2024/03/01 10:00:00")` → `{date:"2024/03/01", time:"10:00:00"}`、空文字と不正な形の分割、`joinDateTime`、`completeTime("10:00","start")` → `10:00:00`、`completeTime("10:00","end")` → `10:00:59`、`completeTime("","start")` → `00:00:00`、`completeTime("","end")` → `23:59:59`、`completeTime("25:00:00",…)` は変えない、`isValidDateText`（`2024/03/01` 可、`2024-03-01` 不可、`2024/02/30` 不可、`2024/02/29` 可）、`isValidTimeText`（`hh:mm:ss` 可、`25:00:00` 不可）、`fieldOfValidationKey` の 7 key すべて（下の表）。`npx vitest run src/dateTime.test.ts` が落ちることを確認する
- [x] Green：`src/dateTime.ts` を実装する。key → 欄の表：

  | validationErrors の key | 出す欄 |
  |---|---|
  | `selection.logGroupRequired`、`validation.logGroupRequired`、`validation.logGroupTooLong` | 見出し行（選択中ロググループ）の直下 |
  | `validation.startFormat` | 開始の日付欄の直下（画面側の判定が時刻欄の誤りと言う場合は時刻欄） |
  | `validation.startNonexistentLocalTime` | 開始の時刻欄の直下 |
  | `validation.endFormat` | 終了の日付欄の直下（同上） |
  | `validation.endNonexistentLocalTime` | 終了の時刻欄の直下 |
  | `validation.rangeOrder` | 終了の日付欄の直下 |
  | `selection.profileRequired`、`selection.regionRequired` | 見出し行の直下（接続の理由） |

- [x] 画面側の形式判定の優先：日付欄・時刻欄ごとに `isValidDateText`／`isValidTimeText` を先に判定して欄の直下に出し、両方が正しいときだけコアの `startFormat`/`endFormat` を日付欄に出す（食い違いの扱い、R-02）

### Step 5: Cloudscape の基盤（画面：実装してからテスト）

- [x] `src/main.tsx`：`@cloudscape-design/global-styles/index.css` を読み込み、`I18nProvider`（`all.ja`／`all.en` を `Locale` で選ぶ）で `App` を包む。`src/hooks/useColorScheme.ts` を新規に作り、`prefers-color-scheme` の初期値と変化で `applyMode(Mode.Light|Mode.Dark)` を呼ぶ（FR4.5）
- [x] `src/styles.css`：システムカラーだけの規則を外し、レイアウト（2 ペイン、最小 1024×640、条件行の折り返し）と、ログ一覧の自前描画に残す規則をデザイントークン（`@cloudscape-design/design-tokens`）の CSS 変数で書き直す
- [x] テスト：`src/App.test.tsx` を更新し、`useColorScheme` のテスト（明→暗の切り替えで `applyMode` が呼ばれる、`matchMedia` がない環境では明るいまま）を `src/hooks/useColorScheme.test.ts` に 3 つ書く

### Step 6: 上部ナビゲーション（画面：実装してからテスト）

- [x] `src/components/TopBar.tsx` を新規に作り、`ConnectionBar.tsx` と `TimeZoneToggle.tsx` を置き換える：`TopNavigation`（identity はアプリ名、utilities はプロファイル・リージョン・タイムゾーンの `menu-dropdown`、`iconName="settings"` の `button`）。既存の `onSelectProfile`／`onSelectRegion`／`onSelectTimeZone`／`onOpenSettings` と `settingsButtonRef`（焦点の戻り先）をそのまま使う。設定ファイルを読めない通知（`catalogNotices`）は上部ナビゲーションの直下に `Alert type="warning"` で出す
- [x] 有効・無効の表（R-05）を実装する：

  | 状態 | プロファイル | リージョン | タイムゾーン | 設定 |
  |---|---|---|---|---|
  | 通常 | 有効 | 有効 | 有効 | 有効 |
  | 取得中 | 無効 | 無効 | 有効（FR1.5。日付・時刻欄は無効のまま値だけ書き換わる） | 無効 |
  | 接続変更の確認待ち | 無効 | 無効 | 無効 | 無効 |
  | 設定ダイアログ表示中 | 無効 | 無効 | 無効 | 無効（ダイアログが前面） |

- [x] `App.tsx` の `<h1>` と `ConnectionBar` を `TopBar` に置き換える。`ConnectionBar.tsx`・`TimeZoneToggle.tsx` とそのテストは削除する
- [x] テスト：`src/components/TopBar.test.tsx` に 7 つ：プロファイル選択が `onSelectProfile` を呼ぶ、リージョン選択、タイムゾーン切替、設定ボタンが `onOpenSettings` を呼ぶ、取得中に接続のドロップダウンと設定が無効でタイムゾーンは有効、確認待ちで全部無効、通知の Alert が出る

### Step 7: 左ペインのロググループ一覧（画面：実装してからテスト）

- [x] `LogGroupPane.tsx` を `Container` + `Header`（「ロググループ」）+ `TextFilter` + `Table`（`selectionType="single"`、`variant="embedded"`、列はロググループ名のみ、`loading`／`empty` を状態から組み立て、`ariaLabels` を設定）+ `Button`（再読み込み）に置き換える。一部失敗は表の上の `Alert type="warning"`、失敗は表の空の状態にエラー文言（既存 `ErrorMessage` の文言関数 `errorLines` を使う）
- [x] テスト：`LogGroupPane.test.tsx` を更新し 7 つ：絞り込みが `onFilterChange` を呼ぶ、行の選択が `onSelect` を呼ぶ、選択中の行に印が付く、読み込み中、空（0 件・検索 0 件）、一部失敗の Alert、失敗の文言、取得中は選べない

### Step 8: 条件エリア（画面：実装してからテスト）

- [x] `FetchForm.tsx` を `ConditionArea.tsx` に作り替える：`Container` + `Header`（選択中ロググループ名、未選択は「ロググループを選んでください」、`actions` に `Button variant="primary"` [取得]）、見出しの直下に選択・接続の理由（`FormField errorText` 相当の `Box color="text-status-error"`）、条件行は `Grid` で左に `LogFilterInput`（`TextFilter` に置き換え）、右に開始・終了それぞれ `FormField`（label に `{zone}`）の中に `DatePicker`（`yyyy/mm/dd`、`locale`、`i18nStrings`）と `TimeInput`（`format="hh:mm:ss"`、`use24Hour`）。1024px で収まらなければ日付・時刻を下に折り返す
- [x] 日付・時刻の振る舞い（interaction-spec §3）：画面側の draft は `date`/`time` に分け、変更のたびに `joinDateTime` でコアへ `updateInput(field, text)` を送る。時刻欄の blur で `completeTime` を適用して欄も書き換える。Enter と [取得] の押下では、先に両方の時刻欄に `completeTime` を適用し、その結果で `canFetch` が真なら `onFetch`（R-01）。コアが返した `startInput.text` は `splitDateTime` で両欄に戻す（タイムゾーン切替、FR1.5）
- [x] 欄の直下の理由は Step 4 の表で割り当てる。「取得できない理由：」の箇条書きと `form.reasons.title` を削除する。[取得] が無効になった瞬間に焦点を持っていた場合は絞り込み欄に焦点を移す（R-06）
- [x] `App.tsx` の `FetchForm` を `ConditionArea` に置き換える。`FetchForm.tsx` とそのテストは削除する
- [x] テスト：`ConditionArea.test.tsx` に 8 つ：日付欄に `2024/03/01` と打つと `onChange("startText","2024/03/01 …")`、ハイフン入力で日付欄の直下に形式の理由、`2024/02/30` で理由、時刻欄 `10:00` の blur で `10:00:00`（終了は `10:00:59`）、時刻が空のまま Enter で `00:00:00`／`23:59:59` を補って取得、`rangeOrder` が終了の日付欄の直下に出る、`logGroupRequired` が見出しの直下に出る、取得中は日付・時刻・[取得] が無効で絞り込み欄は有効。回帰テスト 1：「取得できない理由」の箇条書きが存在しない

### Step 9: エラー表示・ダイアログ・ステータス行（画面：実装してからテスト）

- [x] `FetchErrorBanner.tsx`：`Alert type="error" dismissible header={what}` に置き換える（本文は次の行動＋詳細。閉じるボタンだけ、Escape は使わない）。`App.tsx` の `command-error` も同じ `Alert`
- [x] `FailureList.tsx`：`Modal` + `Table`（列：ストリーム、何が起きたか／次の行動）+ footer の [閉じる]。Escape で閉じ、焦点はボタンに戻る
- [x] `StatusLine.tsx`：`Box` + `StatusIndicator`（loading／error／success）+ `Button variant="link"`（失敗一覧）
- [x] `ConfirmDialog.tsx`、`CloseConfirmDialog.tsx`、`SettingsDialog.tsx`：`Modal`（footer は `SpaceBetween` で右寄せ、主ボタンは右端）、`Checkbox`、`Button`。既存の焦点の戻り・Escape・[Clear cache] の即時反映は維持
- [x] テスト：各テストファイルを更新し、部品ごとに 5〜6 つ（表示内容、閉じる操作、焦点の戻り、無効化、文言）。回帰テスト 2：取得全体の失敗が 3 行構成の Alert で出て、閉じると消える

### Step 10: ログ一覧（画面：実装してからテスト。性能の判断を含む）

- [x] `LogTable.tsx` を Cloudscape の `Table` で作り直した試作を `LogTableCloudscape.tsx` として作る（列：時刻・ストリーム・メッセージ、行の押下で直下に全文、複数展開、↑↓・Enter／Space、`ariaLabels`）
- [ ] 性能測定（R-03）：Tauri の実機（開発者の Mac、`npm run tauri dev`）で、コアを通さず 100 万行のダミー（メッセージ長 80〜400 文字、JSON 混在）を `getRows` のモックから返し、次を測る。記録先は `construction/code-generation/code-summary.md` の「性能測定」節
  - 絞り込み入力から結果表示まで（合格：100 秒以内）
  - スクロール（ホイール 1 回・PageDown・End）と行展開の操作から画面が変わるまで（合格：1 秒以内、目視またはストップウォッチ）
  - 取得中を模した行の追加（10 万行ずつ）の間に、入力欄への入力が 1 秒以内に反映されること（「取得中も固まらない」）
  - メモリでアプリが落ちないこと
- [ ] 合否：いずれか 1 つでも超えたら試作を捨て、既存の `LogTable.tsx`（自前の仮想スクロール）を保って見出し・罫線・配色・余白・文字をデザイントークンで合わせる。すべて満たせば試作を `LogTable.tsx` に置き換える。どちらにしたかと数値を code-summary.md に書く（SM1 の例外の扱いを承認時に確認する）
- [x] 時刻列はコアの文字列（`yyyy/mm/dd hh:mm:ss.SSS`）をそのまま出す。表の空の状態の文言、絞り込み 0 件の文言（R-06）を出し分ける
- [x] テスト：既存の `LogTable.test.tsx`・`rowLayout.test.ts`・`virtualScroll.test.ts` を緑に保ち、空の状態と絞り込み 0 件の文言のテストを 2 つ足す

### Step 11: 文言（英・日）

- [x] `src/i18n/messages.ts`：interaction-spec §9 の key を足す（上部ナビゲーション、ロググループの検索、見出しの未選択、絞り込み欄、日付・時刻の形式の理由、日付が空、順序の誤り、表の空の状態、絞り込み 0 件）。`validation.startFormat`／`endFormat` の文言を `yyyy/mm/dd hh:mm:ss` に変える。`form.reasons.title`・`form.dateTime.placeholder` を消す。`error.what.*`／`error.next.*` は変えない
- [x] `src/i18n/messages.test.ts`（あれば更新、なければ 3 つ新規）：新しい key が両言語にある、旧書式の文字列が残っていない、`{zone}` が埋まる

### Step 12: 全体の検査と設定（環境・ビルド）

- [x] `npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build` を通す
- [x] `cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo test -p local-sights-core` を通す。`cargo-deny` は Rust の依存を足さないため影響なし
- [x] `npm audit --audit-level=high` を通す。配布物の大きさ（`vite build` の出力）を code-summary.md に記録する（要件レビュー R-05）

### Step 13: 文書と追跡

- [x] `README.md`：見た目を Cloudscape に変えたこと、日付の書式が `yyyy/mm/dd` になったこと、カレンダー入力を書く
- [x] `construction/code-generation/code-summary.md`：変更したファイル、主な判断（表の実装方針と性能の数値、DatePicker の確認結果、FR1.4／AS1 の置き換え）、維持した既存の振る舞いの表（R-04）、計画からの逸脱、テストの数
- [x] `construction/code-generation/traceability.json`：FR1.1〜FR4.6、NFR1〜NFR7 を実装ファイルかテストファイルに対応づける

### Step 14: ゲートの修正依頼への対応（利用者の指摘 4 点とレビュー R-01・R-03）

利用者の言葉：「カレンダーが狭くて全部見えない、あとSDKのに任せるとかプロファイル名が長いのでSDKデフォルトとかに。後このプロファイル設定で起動してほしい。ロググループ名の幅もう少し狭くしたいので可変にできる？」。スクリーンショットでは、開始の日付欄のカレンダーが欄＋カレンダーボタンの幅で右が切れ、曜日が「日 月 火 水 木」までしか見えない。

- [x] **14a カレンダーが切れない**：`ConditionArea.tsx` の `DatePicker` のカレンダー（ドロップダウン）が、月の 7 列すべてと「＞」ボタンまで見える幅で開くようにする。原因は `node_modules/@cloudscape-design/components/date-picker/` と `internal/components/dropdown/` の実装（ドロップダウンの幅がトリガー幅に合わされる `stretchWidth`、`expandToViewport` 時の幅計算）を読んで特定する。候補：`expandToViewport` を外して囲みの `overflow` を可視にする／日付欄の幅を広げる／ドロップダウンの最小幅を Cloudscape の設定で確保する。どれにしたかと根拠を code-summary.md に書く。合格は利用者の実機確認（README のチェックリストに「カレンダーの 7 列と月送りボタンが見える」を足す）
- [x] **14b プロファイル名を短く**：`i18n/messages.ts` の `connection.profile.sdkDefault` を「SDK デフォルト」／"SDK default" に変える。上部バーの表示は「プロファイル: SDK デフォルト」。`messages.test.ts`・`TopBar.test.tsx` の文言を更新する
- [x] **14c SDK デフォルトのプロファイルで起動する（純粋ロジック：テスト先行）**：Red：`session.rs` のテスト `starts_with_nothing_selected_and_no_list`／`starts_idle_with_reasons_and_cannot_fetch` などを「起動時に SDK デフォルトのプロファイルが選ばれ、その既定リージョンがあれば選ばれて一覧の取得が始まる、なければリージョンは未選択で一覧は空」に書き換え、`tests/u2_log_group_listing.rs` の起動の前提も直して、落ちることを確認する。Green：`Session` の生成時（`with_catalog_and_time_zones`）に `request_profile(ProfileSelector::SdkDefault, <SDK 既定のリージョン>, false)` 相当を適用する。`ConnectionState::new()` 自体は「何も選ばない」のまま（`connection.rs` のテストは変えない）。MVP の決定「起動時は何も選ばず、前回の選択も覚えない」のうち「何も選ばず」はこの指示で置き換え、「前回の選択は覚えない」は維持する（code-summary.md に明記）。画面側は `TopBar.tsx` のコメントと `App.test.tsx` の起動時の前提を更新する
- [x] **14d 左ペインの幅を可変に**：`src/components/PaneResizer.tsx` を新規に作り、左ペインと右ペインの間に縦のつまみ（`role="separator"`、`aria-orientation="vertical"`、`aria-valuenow`／`aria-valuemin`／`aria-valuemax`）を置く。マウスのドラッグと、焦点を当てて ←／→（1rem ずつ）で幅を変える。初期値は 14rem（今の 18rem より狭く）、最小 10rem、最大 40rem。幅は `localStorage`（キー `local-sights.logGroupPaneWidth`、秘密情報ではない）に覚え、次回起動で使う。`styles.css` の `.log-group-pane` の固定幅を CSS 変数（`--ls-log-group-pane-width`）に変える。テスト `PaneResizer.test.tsx` に 5 つ：初期値、ドラッグで変わる、←／→ で変わる、最小・最大で止まる、`localStorage` に保存・復元（`localStorage` が使えない環境では初期値）
- [x] **14e R-01 CSP**：`src-tauri/tauri.conf.json` の `csp` と `devCsp` に `font-src 'self' data:` を足す（Cloudscape の埋め込みフォント用。外部通信は増えない）。code-summary.md の手動確認の表と README のチェックリストに「起動時にコンソールへ CSP 違反が出ない」を加える
- [x] **14f R-03 性能ハーネスの歯止め**：`src/dev/perfRows.ts` の有効判定を `import.meta.env.DEV` でも条件づけ、本番ビルド（`vite build`）では常に無効になるようにする。テストを 1 つ足す（`vi.stubEnv("DEV", false)` 相当で `enabled` が false）。README に一文添える
- [x] **14h アプリ名を「Local Sights」に**（利用者の追加依頼：「アイコンとアプリ名をいい感じにしたい。アプリ名はLocal Sightsで。」）：`src-tauri/tauri.conf.json` の `productName` と `app.windows[0].title` を `Local Sights` に（`identifier`、Cargo のパッケージ名、npm のパッケージ名 `local-sights` は変えない）。`i18n/messages.ts` の `app.title` を `Local Sights` に（上部バーの表示名）。README の見出しも合わせる。Dock・ウィンドウ切替・ウィンドウタイトルに「Local Sights」と出ることを実機確認の表に加える
- [x] **14i アプリのアイコンを「ログを見る」ものに**（利用者の追加依頼：「アイコンはなんかログを見る感じのやつで。」）：`src-tauri/icons/source/icon.svg` を新規に作る（1024×1024。角丸の濃紺の地に、左寄せの数行のログ行（短い横線、1 行だけ強調色）と、それを覗く虫眼鏡。コンソールの配色に寄せた青系、文字は使わない）。macOS の `qlmanage -t -s 1024 -o <出力先> <svg>`（なければ `npx @resvg/resvg-js` 相当）で 1024px の PNG に描き出し、`npm run tauri icon <png>` で `src-tauri/icons/`（`icon.icns`・`icon.ico`・各 png）を作り直す。SVG と 1024px PNG はリポジトリに残す。Dock と Finder でアイコンが変わることを実機確認の表に加える
- [x] **14g 仕上げ**：code-summary.md（変更ファイル、判断、逸脱、維持した振る舞いの表に「起動時の選択」の行、テストの数）と traceability.json を更新する。`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build`、`cargo +stable fmt --all --check`、`cargo +stable clippy -p local-sights-core --all-targets`、`cargo +stable test -p local-sights-core` を通す

### Step 15: ゲートの修正依頼への対応（2 回目：利用者の指摘 6 点）

利用者の言葉：「プロファイル名は「デフォルト」に変更。ロググループ名がここまでしか小さくできない、ログ本文が見づらいから完全に閉じられるようにしたい＋ミニマムで画面幅の20%くらいまで小さくできるようにしたい。ロググループを選択してくださいが2箇所でてて変。イベントを絞り込みと日付の入力欄のy座標が揃ってない。イベントを絞り込みじゃなくてメッセージを絞り込み。アイコンの四隅が白い空白があるのを削って欲しい。」（Dock 名の質問には回答済み：`tauri dev` は Cargo のバイナリ名で起動するため。`tauri build` の `.app` では「Local Sights」になる）。スクリーンショットでは左ペインが画面幅の約 40% で止まり、それ以上狭くならない。

- [x] **15a プロファイル名を「デフォルト」に**：`i18n/messages.ts` の `connection.profile.sdkDefault` を「デフォルト」／"Default" に。上部バーの表示は「プロファイル: デフォルト」。`messages.test.ts`・`TopBar.test.tsx` を更新する
- [x] **15b 左ペインを狭められる・閉じられる**：狭くならない原因は、`.log-group-pane` に `min-width: 0` がなく、表のロググループ名（`white-space: nowrap`）の最小内容幅がペインを押し広げているため。`styles.css` で `.log-group-pane { min-width: 0; overflow: hidden }` にし、名前の列は省略（…）で収める。幅の単位を rem から「アプリ本体の幅に対する %」に変え（`usePaneWidth`：既定 25%、最小 20%、最大 50%、矢印キーは 1% 刻み、Home＝最小・End＝最大）、`localStorage` のキーは `local-sights.logGroupPaneWidthPercent` に変える（旧キーは読まない）。加えて「閉じる」状態を足す：`LogGroupPane` の見出しの `actions` に Cloudscape の `Button iconName="angle-left" variant="icon"`（aria-label「ロググループ一覧を閉じる」）、閉じると左ペインは幅 0 で非表示になり、代わりに左端に細い縦の帯（`angle-right` の `Button`、aria-label「ロググループ一覧を開く」）だけが残る。閉じた状態も `localStorage`（`local-sights.logGroupPaneCollapsed`）に覚える。つまみは閉じているときは出さない。テスト：`usePaneWidth.test.ts`（% の既定・最小・最大・保存）、`PaneResizer.test.tsx`（% 刻みに更新）、`LogGroupPane.test.tsx`／`App.test.tsx` に「閉じる → 帯だけになる → 開く → 元の幅に戻る」「閉じた状態が復元される」の 2 つ
- [x] **15c 「ロググループを選んでください」の重複をなくす**：見出し行（`ConditionArea` の `Header`）の未選択時の文言 `form.selectedLogGroup.none` を「ロググループ: 未選択」／"Log group: none" に変え、理由行（`selection.logGroupRequired`「ロググループを選んでください。」）だけが案内になるようにする。`ConditionArea.test.tsx` の期待を更新し、「同じ文言が 2 か所に出ない」テストを 1 つ足す
- [x] **15d 絞り込み欄と日付欄の高さを揃える**：絞り込み欄にも日付・時刻と同じ `Box variant="awsui-key-label"` のラベル（「メッセージを絞り込み」）を上に置き、入力欄の上端が開始・終了の入力欄と同じ行に揃うようにする（`condition-row` の `align-items: flex-start` のまま、ラベルの有無の差をなくす）。`ConditionArea.test.tsx` にラベルの存在のテストを 1 つ足す
- [x] **15e 文言「メッセージを絞り込み」**：`logFilter.label`／`logFilter.placeholder` を「メッセージを絞り込み」／"Filter messages" に。`messages.test.ts`・`LogFilterInput.test.tsx`・`ConditionArea.test.tsx` を更新する
- [x] **15f アイコンの四隅を透明に**：`qlmanage` の描き出しは透明部分が白になるため、`@resvg/resvg-js` を devDependency に追加し、`scripts/render-icon.mjs`（Node）で `src-tauri/icons/source/icon.svg` を背景透明のまま 1024×1024 の PNG に描き出す（`package.json` に `icon:render` スクリプト）。四隅のピクセルのアルファが 0 であることをスクリプト内で検証してから `tauri icon` でアイコン一式を作り直す。README の手順を更新する。`npm audit --audit-level=high` と `cargo-deny` に影響がないことを確認する
- [x] **15g 仕上げ**：code-summary.md（変更ファイル、判断、逸脱、手動確認の表に「左ペインを閉じる・開く」「アイコンの四隅が透明」「`tauri build` の .app で Dock 名が Local Sights」を追加、テストの数）と traceability.json（`GATE-7`〜`GATE-12`）を更新する。`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build`、`cargo +stable fmt --all --check`、`cargo +stable clippy -p local-sights-core --all-targets`、`cargo +stable test -p local-sights-core`、`npm audit --audit-level=high` を通す

### Step 16: ゲートの修正依頼への対応（3 回目：利用者の指摘 2 点）

利用者の言葉：「入力欄系はだいぶ良くなった。あとデフォルトの日付を埋めて欲しい。現在日を基準に開始日は昨日、終了日は今日。あと、アイコンが少しデカくない？他のアプリと比べて。」時刻の初期値は補完の規則に合わせて開始 00:00:00・終了 23:59:59（利用者が確認済み）。

- [x] **16a 日付の初期値（純粋ロジック：テスト先行）**：Red：`session.rs` に「生成時に `now` を与えると、ローカルのタイムゾーンで開始＝昨日 00:00:00、終了＝今日 23:59:59 が入力欄に入り、ロググループを選べば取得できる」のテストを足す（Tokyo と UTC、月をまたぐ日、夏時間の切り替え日、`now` を与えない生成では従来どおり空、の 5 つ）。`cargo +stable test -p local-sights-core session` が落ちることを確認する。Green：`AppSession` にビルダー `with_default_time_range(now_ms: i64) -> Self` を足し、`time_zones` のローカルゾーンで `now_ms` の日付を求め、`format_input_in_zone` の書式（`yyyy/mm/dd hh:mm:ss`）で両方の欄の文字列を作って `DateTimeInput::parse` に通し、`revalidate` する。既存の生成関数の振る舞いは変えない（テストの前提を壊さない）。`src-tauri/src/lib.rs` の生成箇所で `SystemTime::now()` をミリ秒にして渡す（`unwrap`／`expect` は使わず、取れないときは埋めない）。画面側は何も変えない（コアが持つ文字列を既存の経路で両欄に分けて表示する）。`App.test.tsx` の起動時のテストに、初期値が両欄に出ることを 1 つ足す（fixtures で `startInput.text` を与える）
- [x] **16b アイコンの大きさを macOS の規則に合わせる**：Apple のアプリアイコンは 1024px のうち中央 824×824（上下左右 100px の透明な余白）に角丸四角（角の半径 ≒ 185px）を置く規則で、今の 1024px 全面の板は他のアプリより大きく見える。`src-tauri/icons/source/icon.svg` の板を 824×824（`x=100 y=100 rx=185`）に縮め、中の図形は `<g transform="translate(100 100) scale(0.8046875)">` で同じ比率で縮める。`npm run icon:render`（四隅の透明と中央の不透明の検証つき。中央の検証座標は板の中にあるので変更不要）で描き出し、`tauri icon` でアイコン一式を作り直す（置き換えるのは tauri.conf.json が参照する 6 ファイルだけ）。`render-icon.mjs` に「余白の帯（x=50, y=512）が透明」の検証を 1 つ足す。README の「App icon」にこの寸法を書く
- [x] **16c 仕上げ**：code-summary.md（変更ファイル、判断、逸脱、手動確認の表に「起動時に昨日〜今日が入っている」「Dock で他のアプリと同じ大きさ」を追加、テストの数）と traceability.json（`GATE-13`、`GATE-14`）を更新する。`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build`、`cargo +stable fmt --all --check`、`cargo +stable clippy -p local-sights-core --all-targets`、`cargo +stable test -p local-sights-core`、`npm audit --audit-level=high` を通す

### Step 17: ゲートの修正依頼への対応（4 回目：未来の日付）

利用者の言葉：「OK、アプリ名の確認はこれで。あとカレンダーで未来の日付が選べるのはおかしいので修正して。」（`tauri dev` の Dock 名は回答済み：素の実行ファイルで起動するため。`tauri build -- --debug` の .app では「Local Sights」になる。コードの変更はしない）。

- [x] **17a 今日の日付（純粋ロジック：テスト先行）**：Red：`src/dateTime.test.ts` に `todayInZone(nowMs, zone)`（`zone` は `"Local" | "Utc"`、戻り値は `yyyy/mm/dd`）と `isFutureDate(slashed, today)` のテストを足す（Local と Utc で日付がずれる時刻、月・年またぎ、今日は未来ではない、明日は未来、不正な文字列は未来と見なさない、の 5 つ）。`npx vitest run src/dateTime.test.ts` が落ちることを確認する。Green：`src/dateTime.ts` に実装する（`Date` の `getFullYear`／`getUTCFullYear` 系で組み立て、ライブラリは足さない）
- [x] **17b カレンダーで未来の日付を選べなくする（画面：実装してからテスト）**：`ConditionArea.tsx` の開始・終了の両方の `DatePicker` に `isDateEnabled`（今日＝表示中のタイムゾーン `session.timeZone` の今日、より後の日付を無効）と、無効な日を読み上げる `i18nStrings`（Cloudscape の `todayAriaLabel` 等の既存設定があれば合わせる）を渡す。今日の基準は描画時の `Date.now()`（テストでは `vi.setSystemTime` で固定）。手入力で未来の日付を打った場合はカレンダーの制限の対象外とし、取得はそのまま可能（CloudWatch は 0 件を返すだけで害がない。利用者の依頼は「カレンダーで」のため）。テスト：`ConditionArea.test.tsx` に 3 つ：カレンダーを開くと明日のセルが無効・今日のセルが有効、終了側も同じ、タイムゾーンが UTC のとき UTC の今日が基準になる
- [x] **17c 仕上げ**：code-summary.md（変更ファイル、判断「手入力の未来日は対象外」、手動確認の表に「カレンダーで明日以降が選べない」、テストの数）と traceability.json（`GATE-15`）、README のチェックリストを更新する。`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build`、`cargo +stable fmt --all --check`、`cargo +stable clippy -p local-sights-core --all-targets`、`cargo +stable test -p local-sights-core`、`npm audit --audit-level=high` を通す

### Step 18: ゲートの修正依頼への追加（ステータス行を常に見える位置に）

利用者の言葉：「追加で、フッター付近のこのメッセージは常に見えるようにしておいてほしい（ログメッセージが増えると下に追いやられて見えなくなってしまう現状はNG」。スクリーンショットは条件エリアの枠の下端にあるステータス行（「取得中… 42/744 ストリーム、これまでに 714 件　絞り込み後 2 件 / 全 714 件」）。

- [x] **18a 原因の特定と修正（画面：実装してからテスト）**：`App.tsx` で `LogTable` と `StatusLine` は `ConditionArea`（Cloudscape の `Container`）の子として描かれる。`Container` の内側の要素は flex の列になっておらず高さが決まらないため、ログ一覧（`.log-table { flex: 1; min-height: 0 }`）が内容に合わせて伸び、ステータス行が窓の外へ押し出されていると見込む。Cloudscape の `Container` が出す DOM（`node_modules/@cloudscape-design/components/container/`）を読んで原因を確かめ、`ConditionArea` の内側（`Container` の content 領域から子の並び）を `display:flex; flex-direction:column; flex:1; min-height:0` で窓の高さに収める。ログ一覧はその残りの高さの中で自分でスクロールし、ステータス行は常にログ一覧のすぐ下（条件エリアの枠の下端）に見えるようにする。Cloudscape の内部クラス名に依存する場合は `fitHeight` プロパティなど公開 API を優先し、使えない場合だけ自前の囲み（`.condition-area-body`）で対処する。アラート（取得エラー）が出た場合もステータス行は押し出されない
- [x] **18b テスト**：jsdom は高さを計算しないため、構造のテストで確かめる：`App.test.tsx`（または `ConditionArea.test.tsx`）に 2 つ：ステータス行がログ一覧と同じ flex の列の最後の子として描かれる、ログ一覧の囲みに縮める指定（`min-height: 0` を持つクラス）が付く。行数が多いとき・取得エラーのアラートが出たときにステータス行が見えることは手動確認の表と README のチェックリストに加える（「1,000 行以上で、窓を最小 1024×640 にしてもステータス行が見える」）
- [x] **18c 仕上げ**：code-summary.md（変更ファイル、原因と判断、手動確認、テストの数）と traceability.json（`GATE-16`）を更新する。`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`npx vite build`、`cargo +stable test -p local-sights-core`、`npm audit --audit-level=high` を通す

## 要件・画面案との対応

| 要件 | ステップ |
|---|---|
| FR1.1〜FR1.8（日付と時刻の入力） | Step 3、4、8 |
| FR2.1〜FR2.4（表記） | Step 3、4、8、10、11 |
| FR3.1〜FR3.5（エラーの見せ方） | Step 4、8、9 |
| FR4.1〜FR4.6（部品、構成、ダークモード、言語） | Step 1、5、6、7、8、9、10、11 |
| NFR1（Cloudscape）、NFR2（性能）、NFR3（キーボード）、NFR4（言語）、NFR5（依存）、NFR6（回帰）、NFR7（最小ウィンドウ） | Step 1、10、5〜9 のテスト、11、1・12、2・12、5 |
