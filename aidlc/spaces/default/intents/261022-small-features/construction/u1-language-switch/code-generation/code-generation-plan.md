# Code Generation Plan — U1 言語切替（u1-language-switch）

## Summary

- Builds: 設定ダイアログの「言語」の 3 択（OS に合わせる／日本語／English）を settings.json に保存し、起動し直さずに画面の文言を切り替える薄い一本
- Touches: `crates/local-sights-core/src/cache/settings.rs`・`session.rs`、`src-tauri/src/lib.rs`、`src/main.tsx`・`App.tsx`・`api.ts`・`i18n/messages.ts`・`components/SettingsDialog.tsx`・`test/fixtures.ts`
- Tests: 新規およそ 26 本（Rust の純粋ロジック 8 本をテスト先行、Rust の境界 6 本・画面 12 本は実装後）。既存の約 480 本は緑のまま

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
      "text": "- クエリの速さの合否の数値は、利用者の答え（作る段階で決める）に従って要件では決めなかった（NFR1） (learned 2026-10-17) \n\n- 依頼 (4) の権限のテストの固定値は依頼文の 26 ではなく今は 32 だったため、数の固定そのものをやめて 3 か所の一覧の一致だけを確かめることにした（Q8 A）。 (learned 2026-10-20)"
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
  "input_sha256": "sha256:6b791a61f9f432be6d2e64e14b35060648c7ba1feb658930c23c1bc6ff063b71",
  "contract_sha256": "sha256:1e653206b60c1801cccf7ea17a92ff67599510c6fe6d79d8aa8a845da0a0571a"
}
```

## 上流

`inception/units-generation/unit-of-work.md`（U1：薄い一本、service、S）、`inception/units-generation/unit-of-work-story-map.md`（FR1.1〜FR1.6）、`construction/u1-language-switch/functional-design/`（`entities.md`：LanguagePreference・AppSettingsFile・ResolvedLocale・SessionViewSettings、`rules.md`：BR1.1〜BR1.7、`functional-spec.md`：W1〜W4）、`inception/requirements-analysis/requirements.md`（FR1、NFR3・NFR4・NFR5・NFR6）、`inception/domain-design/decisions.md`（ADR-005）。

## 進め方の要点

- ブラウンフィールド。既存のファイルをその場で変える。新しいファイルは作らない（テストは各モジュールの中と既存の `*.test.ts(x)` に足す）。
- Testing Contract の ordering に従う。**純粋なロジック**（設定ファイルの `language` の読みと書きの判断、`resolveLocale`）は**テストを先に書いて**赤を確かめてから実装する。**境界の層**（AppSession の状態、Tauri コマンド、React の部品、`main.tsx`）は**実装してからテスト**を書いて走らせる。
- 本番コードでは `unwrap()` / `expect()` を使わない（team.md Code Style）。設定ファイルに書くのは `formatVersion`・`cacheEnabled`・`language` の 3 つだけ（NFR5、project.md Forbidden）。`formatVersion` は 1 のまま（BR1.5）。
- 対話的な UI 部品には `data-testid` を付ける（既存の命名に合わせる：`settings-dialog-language-select`）。
- コマンドの集合は変わらない（`save_settings` の引数が増えるだけ）ため、`generate_handler!`・`build.rs`・`capabilities/default.json` の 3 か所は触らない。既存の `src/commandRegistration.test.ts` が 3 か所の一致を確かめる。
- 完了の目安：`cargo fmt --check`・`cargo clippy --workspace`・`cargo test -p local-sights-core`・`cargo test -p local-sights`・`npx tsc --noEmit`・`npx eslint .`・`npx prettier --check .`・`npx vitest run` がすべて通る。

## 実装ステップ

### Step 1: 構成の確認（骨組み）

- [x] Step 1.1 変更する既存ファイルの位置と今の形を確かめる：`crates/local-sights-core/src/cache/settings.rs`（`SettingsFile`・`SettingsLoad`・`load_settings`・`save_settings`）、`crates/local-sights-core/src/session.rs`（`SettingsWork`・`SettingsWorkResult`・`SessionView`・`with_cache_location`・`begin_save_settings`・`finish_settings_work`・`view()`）、`src-tauri/src/lib.rs`（`save_settings` コマンド）、`src/api.ts`（`SessionView`・`saveSettings`）、`src/i18n/messages.ts`（`Locale`・`detectLocale`・文言）、`src/components/SettingsDialog.tsx`、`src/App.tsx`・`src/main.tsx`、`src/test/fixtures.ts`。新しいファイルや設定ファイルは要らない（新しい依存もなし）。
  - 対応：全 FR（準備）

### Step 2: テストランナーの確認と作業単位に絞ったコマンドの記録

- [x] Step 2.1 Rust のランナーを確かめる：`cargo test -p local-sights-core cache::settings` が今のテスト 5 本（Windows では 4 本）を通すことを確かめる。
- [x] Step 2.2 画面のランナーを確かめる：`npx vitest run src/i18n/messages.test.ts src/components/SettingsDialog.test.tsx` が今のテストを通すことを確かめる。
- [x] Step 2.3 作業単位に絞ったコマンドを `unit-test-instructions.md` のとおりに使う（この計画と同時に書いてある）。
  - 対応：Testing Contract の runner_step

### Step 3: 純粋なロジック — テストを先に書いてから実装する

- [x] Step 3.1 **[Red]** `cache/settings.rs` のテストを先に書く（コンパイルが通らない・失敗することを確かめる）：
  - `language_is_read_back_and_defaults_to_system`：`save_settings(path, true, LanguagePreference::Ja)` → `load_settings` が `Loaded { cache_enabled: true, language: Ja }`。`En`・`System` も同じ。ファイルの中身が `{"formatVersion":1,"cacheEnabled":true,"language":"ja"}\n` の 3 項目だけ（NFR5）。
  - `an_old_settings_file_without_language_means_system`：`{"formatVersion":1,"cacheEnabled":true}` → `Loaded { cache_enabled: true, language: System }`（BR1.4、BR1.5）。
  - `an_unknown_language_means_system_and_keeps_the_cache_setting`：`"language":"fr"`、`"language":""`、`"language":1` → `cache_enabled` は書かれた値、`language` は `System`（BR1.4）。
  - `a_file_with_unknown_fields_is_still_read`：知らない項目 `"extra":1` を無視して読む（BR1.5 の前の版との互換の裏返し）。
  - `settings_load_language_falls_back_to_system`：`SettingsLoad::Missing / Unreadable / Invalid` の `language()` は `System`、`cache_enabled()` は今のまま `false`。
  - `language_preference_round_trips_its_spelling`：`LanguagePreference` の `as_str()` と `parse` が `system` / `ja` / `en` を往復し、ほかは `None`。
  - 既存の 5 本を新しい型（`Loaded { cache_enabled, language }`・`save_settings` の 3 引数）に合わせて直す。
  - 対応：FR1.4、BR1.4、BR1.5、NFR5
- [x] Step 3.2 **[Green]** `cache/settings.rs` を実装する：`pub enum LanguagePreference { System, Ja, En }`（`Default` = `System`、`as_str()`、`FromStr` または `parse(&str) -> Option<Self>`、`serde` は `rename_all = "lowercase"` で `Serialize`/`Deserialize`）。`SettingsFile.language: Option<String>`（`#[serde(default, skip_serializing_if = "Option::is_none")]` は使わず、常に書く：読みは `Option<String>` で受け、知らない綴りは `System` に落として標準エラーに 1 行出す。書きは `Some(as_str())`）。`SettingsLoad::Loaded { cache_enabled, language }`、`SettingsLoad::language(self) -> LanguagePreference`。`save_settings(path, cache_enabled, language)`。`formatVersion` は 1 のまま。モジュールの doc コメントを更新する。
  - 対応：FR1.4、BR1.4、BR1.5
- [x] Step 3.3 **[Refactor]** Step 3.1 のテストが緑であることを確かめ、`cargo fmt` と clippy を通す。
- [x] Step 3.4 **[Red]** `src/i18n/messages.test.ts` に `resolveLocale` のテストを先に書く（失敗することを確かめる）：
  - `resolveLocale("ja", ["en-US"])` → `"ja"`、`resolveLocale("en", ["ja-JP"])` → `"en"`（設定が勝つ）。
  - `resolveLocale("system", ["ja-JP","en-US"])` → `"ja"`、`resolveLocale("system", ["en-US"])` → `"en"`、`resolveLocale("system", [])` → `"en"`（`detectLocale` と同じ）。
  - 新しい文言の鍵 `settings.language`・`settings.language.system`・`settings.language.ja`・`settings.language.en` が両方の言語にあり、`settings.language.ja` は両言語で「日本語」、`settings.language.en` は両言語で「English」（BR1.1 の固定表示名）、`settings.language.system` は en が "Follow the OS setting"、ja が「OS の設定に合わせる」。
  - 既存の「鍵の一覧」のテスト（`settings.cacheEnabled` を含む一覧）に新しい鍵を足す。
  - 対応：FR1.1、FR1.2、BR1.1、BR1.2
- [x] Step 3.5 **[Green]** `src/i18n/messages.ts` を実装する：`export type LanguagePreference = "system" | "ja" | "en"`、`export function resolveLocale(preference: LanguagePreference, languages: readonly string[]): Locale`（純粋。`system` は `detectLocale(languages)`）。4 つの文言を足す。
  - 対応：FR1.2、BR1.2
- [x] Step 3.6 **[Refactor]** Step 3.4 のテストが緑であることを確かめる。

### Step 4: 境界の層 — 実装してからテストを書く

- [x] Step 4.1 **[実装]** `session.rs`：`AppSession` に `language: LanguagePreference` を足す（初期値 `System`）。`with_cache_location` で `load_settings` の結果から `cache_enabled` と `language` の両方を取る（1 回だけ読む）。`SettingsWork::Save` に `language` を足し、`run()` で `save_settings(path, enabled, language)` を呼ぶ。`SettingsWorkResult::Saved` に `language` を足す。`begin_save_settings(enabled, language)`、`save_settings(enabled, language)`、`finish_settings_work` の `Saved` で `self.language = language`（失敗時は変えない：BR1.6）。`SessionView` に `pub language: LanguagePreference` を足し、`view()` で写す。`LanguagePreference` を `local-sights-core` の公開 API として re-export する（`lib.rs` の `pub use`、今の `SettingsNotice` と同じ流儀）。doc コメントを更新する。
  - 対応：FR1.2、FR1.3、FR1.5、FR1.6、BR1.3、BR1.4、BR1.6
- [x] Step 4.2 **[テスト]** `session.rs` のテストを書く・直す（実装後）：
  - `the_language_is_read_at_startup_and_shown_in_the_view`：`save_settings(path, false, En)` した場所で `with_cache_location` → `view().language == En`、ファイルなしは `System`。
  - `saving_the_language_changes_the_view_only_after_the_work_succeeded`：`begin_save_settings(true, Ja)` の `SettingsWork::Save { language: Ja, .. }`、`run()` → `Saved { language: Ja, .. }`、`finish_settings_work` 後に `view().language == Ja` で、ファイルも `Loaded { language: Ja, .. }`（BR1.3）。
  - `a_failed_save_keeps_the_language`：書けない場所で `save_settings(true, Ja)` → `SaveFailed`、`view().language` は前のまま（BR1.6）。
  - `the_language_is_saved_with_the_cache_setting_in_one_file`：キャッシュを無効にしつつ言語を変えても、1 つのファイルに両方が書かれ、消去は書いたあと（今の順序を保つ）。
  - 既存の `save_settings(true)` / `begin_save_settings(false)` を呼ぶテストを新しい引数（`LanguagePreference::System`）に合わせて直す。
  - 対応：BR1.3、BR1.4、BR1.6
- [x] Step 4.3 **[実装]** `src-tauri/src/lib.rs`：`save_settings(enabled: bool, language: LanguagePreference, ...)` に広げ、`begin_save_settings(enabled, language)` を渡す。コマンド名は変えない。doc コメントを更新する。
  - 対応：FR1.2、FR1.3
- [x] Step 4.4 **[テスト]** `cargo test -p local-sights` と `src/commandRegistration.test.ts` で、コマンドの集合が 3 か所で一致したままであることを確かめる（コマンドは増えないので既存のテストが緑なら足りる）。
  - 対応：制約（3 か所の一致）
- [x] Step 4.5 **[実装]** `src/api.ts`：`SessionView.language: LanguagePreference`（`messages.ts` の型を import）、`saveSettings(enabled: boolean, language: LanguagePreference)` → `invoke("save_settings", { enabled, language })`。`src/test/fixtures.ts` の `sessionView` に `language: "system"` を足す。
  - 対応：FR1.2
- [x] Step 4.6 **[実装]** `src/components/SettingsDialog.tsx`：チェックボックスの下に `FormField`（ラベル `t("settings.language")`）と Cloudscape `Select`（`data-testid="settings-dialog-language-select"`、選択肢は `system`／`ja`／`en` の順、表示名は `t("settings.language.<value>")`）を足す。状態 `language` の初期値は `session.language`。`onSave: (enabled: boolean, language: LanguagePreference) => void`。初回フォーカスはチェックボックスのまま（BR1.7）。
  - 対応：FR1.1、BR1.1、BR1.7
- [x] Step 4.7 **[テスト]** `src/components/SettingsDialog.test.tsx` を書く・直す（実装後）：
  - 開いたときの初期値が `session.language` の表示名（`"ja"` → 「日本語」）。
  - 3 つの選択肢が表示名のとおりに出る（英語の画面で "Follow the OS setting"・「日本語」・"English"、日本語の画面で「OS の設定に合わせる」・「日本語」・"English"）。
  - 選んだだけでは `onSave` が呼ばれず、［保存］で `(enabled, language)` の両方が渡る。
  - ［キャンセル］・Escape では選択が捨てられ `onSave` は呼ばれない。
  - Select がラベル「言語」で見つかり、Tab で届く（キーボード）。既存の「Tab が中に留まる」テストの回数を 1 つ増やす。
  - 既存の `onSave` の呼び出しの assert を 2 引数に直す。
  - 対応：BR1.1、BR1.7
- [x] Step 4.8 **[実装]** `src/App.tsx`・`src/main.tsx`：`AppProps` を `{ languages?: readonly string[]; platform?: string }` に変える（`locale` を受けるのをやめる。`languages` は OS の言語の並び。省略時は `navigator.languages`、テストは明示する）。`App` の中で `locale = resolveLocale(session?.language ?? "system", languages)` を `useMemo` で決め、`t = createTranslator(locale)`。`I18nProvider`（`locale` と `messages`）を `App` の返す木の最上位に移し、`useEffect` で `document.documentElement.lang = locale` を更新する。Cloudscape の文言の辞書（`all.en`/`all.ja`）の import は `main.tsx` から `App.tsx` に移す。`main.tsx` は `languages` を渡すだけにする。SessionView が届くまでは `system`（OS の言語）で描く（W1）。`handleSaveSettings` は `(enabled, language)` を `saveSettings` に渡す。`regionLabel` に渡す `locale` も同じ値。
  - 対応：FR1.2、FR1.3、FR1.5、BR1.2、BR1.3
- [x] Step 4.9 **[テスト]** `src/App.test.tsx`・`src/App.title.test.tsx` を書く・直す（実装後）：
  - 既存の `render(<App locale="en" />)` を `render(<App languages={["en-US"]} />)`、`locale="ja"` を `languages={["ja-JP"]}` に置き換える（文言の期待は変わらない）。
  - `switches the texts to Japanese when the session says ja, without losing the state`：英語の OS で開始 → ［設定］→ 「日本語」を選んで［保存］→ `saveSettings` が `(false, "ja")` で呼ばれる → `session-changed` で `language: "ja"` が届くと、見出し・ボタンが日本語になり `document.documentElement.lang === "ja"`、入力欄に入れた文字と取得済みの行の表示は残る（FR1.5）。
  - `starts in the saved language`：最初の `getSession` が `language: "en"` を返す日本語の OS で、画面が英語で描かれる（W1、W4）。
  - `a failed save leaves the language as it was`：`settingsNotice: "SaveFailed"` の view が届いても言語は変わらず、ダイアログが知らせを出す（BR1.6）。
  - 対応：FR1.2、FR1.3、FR1.5、FR1.6、BR1.2、BR1.3、BR1.6

### Step 5: 環境とビルドの設定

- [x] Step 5.1 新しい依存・設定の変更がないことを確かめる（`package.json`・`Cargo.toml`・`tauri.conf.json`・`capabilities/default.json` は触らない）。`cargo fmt --check`、`cargo clippy --workspace --all-targets`、`npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .` を通す。

### Step 6: 文書とトレーサビリティ

- [x] Step 6.1 変えた関数・型・部品の doc コメントを更新する（`settings.rs` のモジュール説明に `language` の項目、`SettingsDialog` と `App` の説明に言語の切り替え）。`docs/` の利用者向け文書はこの作業単位では変えない（U2 の書き出しと合わせて Build and Test で見直す）。
- [x] Step 6.2 `cargo test -p local-sights-core`・`cargo test -p local-sights`・`npx vitest run` を全体で走らせ、既存のテストが緑のままであることを確かめる。

## ストーリー（FR）と手順の対応

| FR / BR | 手順 |
|---|---|
| FR1.1 言語の 3 択（BR1.1） | Step 3.4〜3.5（文言）、Step 4.6〜4.7（Select） |
| FR1.2 設定 × OS → 表示（BR1.2、BR1.3） | Step 3.4〜3.5（`resolveLocale`）、Step 4.1、4.3、4.5、4.8〜4.9 |
| FR1.3 起動し直さずに切り替え（BR1.3） | Step 4.1〜4.2、4.8〜4.9 |
| FR1.4 settings.json に保存、前の版と互換（BR1.4、BR1.5） | Step 3.1〜3.3 |
| FR1.5 状態を保つ（BR1.3） | Step 4.8〜4.9 |
| FR1.6 保存失敗は SaveFailed（BR1.6） | Step 4.1〜4.2、4.9 |
| NFR6 キーボード・読み上げ（BR1.7） | Step 4.6〜4.7 |
| NFR5 秘密を書かない | Step 3.1（3 項目だけのテスト） |

## Assumptions & Open Questions

- [assumption] Cloudscape の `I18nProvider` の `locale` と `messages` を描画中に差し替えても、開いている部品は崩れない（functional-spec の仮定）。Step 4.8 で崩れる部品があれば、その部品だけ `key={locale}` で作り直し、`code-summary.md` に書く。
- [assumption] `serde_json` の既定（`deny_unknown_fields` なし）で知らない項目を無視するため、今の版が書く `language` を前の版のアプリが読んでも落ちない（BR1.5）。Step 3.1 のテストで確かめる。
