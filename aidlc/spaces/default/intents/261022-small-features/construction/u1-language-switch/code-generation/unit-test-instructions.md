# Unit Test Instructions — U1 言語切替（u1-language-switch）

## テストの枠組みと設定

- **Rust（中核）**：`cargo test`。テストは各モジュールの `#[cfg(test)] mod tests` の中にある。`tempfile` で一時フォルダを作り、設定ファイルはその中に書く。設定は既存のまま（`Cargo.toml` は変えない）。
- **Rust（アプリ）**：`cargo test -p local-sights`。コマンドの登録の一致を確かめる既存のテストがある。
- **画面（TypeScript / React）**：Vitest + Testing Library（`vitest.config.ts`、jsdom）。設定は既存のまま。
- 自動テストから実際の AWS には接続しない（team.md）。この作業単位は AWS を呼ばない。

## この作業単位のテストの走らせ方（作業単位に絞ったコマンド）

テスト先行の最初のサイクル（Step 3.1）の前に、次のコマンドが今のテストを通すことを確かめる（Step 2）。

### Rust：設定ファイルの読み書き（Step 3.1〜3.3、テスト先行）

```bash
cargo test -p local-sights-core cache::settings
```

### Rust：AppSession の設定の保存と言語（Step 4.1〜4.2、実装後）

```bash
cargo test -p local-sights-core session::tests::the_language_is_read_at_startup_and_shown_in_the_view
cargo test -p local-sights-core session::tests::saving_the_language_changes_the_view_only_after_the_work_succeeded
cargo test -p local-sights-core session::tests::a_failed_save_keeps_the_language
cargo test -p local-sights-core session::tests::the_language_is_saved_with_the_cache_setting_in_one_file
cargo test -p local-sights-core settings
```

（最後の 1 行は、名前に `settings` を含む既存の設定まわりのテストをまとめて走らせる。）

### Rust：アプリ側のコマンド（Step 4.3〜4.4、実装後）

```bash
cargo test -p local-sights
```

（アプリのテストは 3 か所のコマンド登録の一致を確かめるものだけで、作業単位に絞る必要がない大きさ。）

### 画面：純粋なロジックと文言（Step 3.4〜3.6、テスト先行）

```bash
npx vitest run src/i18n/messages.test.ts
```

### 画面：設定ダイアログ（Step 4.6〜4.7、実装後）

```bash
npx vitest run src/components/SettingsDialog.test.tsx
```

### 画面：App（Step 4.8〜4.9、実装後）

```bash
npx vitest run src/App.test.tsx src/App.title.test.tsx src/commandRegistration.test.ts
```

### 全体の確認（Step 6.2。既存のテストが緑のままであることの確認で、作業単位の確認ではない）

```bash
cargo test -p local-sights-core
cargo test -p local-sights
npx vitest run
```

## 期待するカバレッジ

- 行カバレッジは全体で 80% 以上を保つ（CI：`cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`npm run test:coverage`）。この作業単位で足す行は、上のテストで全部通る。
- 中核の公開関数（`LanguagePreference` の変換、`load_settings`、`save_settings`、`SettingsLoad::language`、`AppSession::begin_save_settings` / `finish_settings_work`）には必ずテストがある（team.md）。

## モック・スタブの方針

- Rust：ファイルシステムは `tempfile` の一時フォルダで本物を使う（既存のテストと同じ）。書けない場所は「フォルダのはずの位置に通常のファイルを置く」で作る（既存の `a_settings_file_that_cannot_be_written_is_an_error` と同じ）。
- 画面：`src/api.ts` は `App.test.tsx` の既存の `vi.mock` で差し替え、`saveSettings` の呼び出しの引数を `toHaveBeenCalledWith(false, "ja")` で確かめる。`session-changed` は既存の `push(view)` で届ける。OS の言語は `App` の `languages` prop で渡す（`navigator.languages` は触らない）。
- Cloudscape の `Select` は Testing Library でトリガーのボタンを押して選択肢を選ぶ（既存の TopBar のテストの流儀）。

## テストデータ

- 設定ファイルの中身は文字列リテラルで書く：`{"formatVersion":1,"cacheEnabled":true}`（前の版）、`{"formatVersion":1,"cacheEnabled":true,"language":"ja"}`、`{"formatVersion":1,"cacheEnabled":false,"language":"fr"}`（知らない値）、`{"formatVersion":1,"cacheEnabled":true,"extra":1}`（知らない項目）。
- `SessionView` の固定値は `src/test/fixtures.ts` の `sessionView({ language: "ja" })` のように上書きして作る。
- 秘密の値（アクセスキー・トークン）はテストデータにも入れない。
