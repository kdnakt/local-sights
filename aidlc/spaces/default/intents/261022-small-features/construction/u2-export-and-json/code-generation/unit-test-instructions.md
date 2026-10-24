# Unit Test Instructions — U2 書き出しと JSON の整形（u2-export-and-json）

## テストの枠組みと設定

- **Rust（中核）**：`cargo test`。テストは各モジュールの `#[cfg(test)] mod tests` の中。ファイルは `tempfile` の一時フォルダに書く。設定は既存のまま（`Cargo.toml` の変更は 2 つのプラグインの依存の追加だけ）。
- **Rust（アプリ）**：`cargo test -p local-sights`。`export_boundary.rs` のファイルの書き込みは一時フォルダで本物を使う。保存ダイアログとクリップボードはプラグインのため自動テストでは呼ばず、呼び出しの境界を薄く保つ。
- **画面（TypeScript / React）**：Vitest + Testing Library（`vitest.config.ts`、jsdom）。設定は既存のまま。
- 自動テストから実際の AWS には接続しない。この作業単位は AWS を呼ばない。

## この作業単位のテストの走らせ方（作業単位に絞ったコマンド）

テスト先行の最初のサイクル（Step 3.1）の前に、次のコマンドが（空の骨組みで）通ることを確かめる（Step 2）。

### Rust：CSV・Markdown の組み立てと既定のファイル名（Step 3.1〜3.5、テスト先行）

```bash
cargo test -p local-sights-core export::formatter
cargo test -p local-sights-core export::exporter
```

### Rust：AppSession・InsightsSession の書き出しの状態（Step 4.4、実装後）

```bash
cargo test -p local-sights-core export
cargo test -p local-sights-core session::tests::export
cargo test -p local-sights-core insights_session::tests::export
```

（`export` を名前に含むテストをまとめて走らせる。新しいテストの名前はすべて `export` を含める。）

### Rust：アプリ側の境界とコマンド（Step 4.8、実装後）

```bash
cargo test -p local-sights export_boundary
cargo test -p local-sights
```

### 画面：JSON の検出と整形（Step 3.6〜3.8、テスト先行）

```bash
npx vitest run src/jsonPretty.test.ts
```

### 画面：部品（Step 4.14、実装後）

```bash
npx vitest run src/components/ExportMenu.test.tsx src/components/ExportNoticeAlert.test.tsx src/components/ExpandedMessage.test.tsx
npx vitest run src/components/LogTable.test.tsx src/components/LogAnalyticsPage.test.tsx src/components/ConditionArea.test.tsx
npx vitest run src/App.test.tsx src/commandRegistration.test.ts src/i18n/messages.test.ts
```

### 全体の確認（Step 6.2。既存のテストが緑のままであることの確認）

```bash
cargo test -p local-sights-core
cargo test -p local-sights
npx vitest run
```

## 期待するカバレッジ

- 行カバレッジは全体で 80% 以上を保つ（CI：`cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`npm run test:coverage`）。
- 中核の公開関数（`write_csv`・`write_markdown`・`sanitize_file_stem`・`default_file_name`・`into_snapshot`・`render_to`・`begin_export`・`finish_export`・`can_export_logs`・`can_export`・`export_source`）には必ずテストがある。
- `jsonPretty.ts` の `findJson` と `prettyPrint` は分岐（候補の再試行、スカラーだけの配列、上限、逃がし）を網羅する。

## モック・スタブの方針

- Rust：ファイルシステムは `tempfile` で本物。「書けない」は、フォルダのはずの位置に通常のファイルを置く（既存の `settings.rs` のテストと同じ）。ダイアログとクリップボードは呼ばない（境界の関数は `AppHandle` を受け取るだけにし、純粋な部分をテストする）。
- 画面：`src/api.ts` は `App.test.tsx` の既存の `vi.mock` に `startExport`・`dismissExportNotice` を足して差し替え、`toHaveBeenCalledWith("logManagement", "csv", "file")` で確かめる。`session-changed` は既存の `push(view)` で届ける。
- Cloudscape の `ButtonDropdown` はトリガーのボタンを押して項目を選ぶ（既存の TopBar のテストの流儀）。`SegmentedControl` は `role="radiogroup"`/`radio` または button として見つけ、キー操作は `fireEvent.keyDown` に `keyCode` を付ける（既存の流儀）。

## テストデータ

- CSV / Markdown：`a,b`・`"q"`・`line1\nline2`・`x|y`・`=SUM(1)`・空の文字・日本語・絵文字を含むセル。
- ファイル名：`/aws/lambda/my-func`、`a:b*c?d"e<f>g|h`、` .name. `、空、101 文字以上、マルチバイトを含む 100 文字の境界。時刻は `2026-10-24T12:30:45Z`（Asia/Tokyo では `20261024-213045`、UTC では `20261024-123045`）。
- JSON：`{"a":1,"b":[1,2]}`、`2026-10-24T01:51:32Z abc-123 INFO {"a":1}`、`{"a":1} tail`、`{"a":1,`、`{"a":}`、`{oops} {"a":1}`、`[1234] {"event":"x"}`、`[{"a":1}]`、`{"s":"a{b\"c"}`、`{"n":1.0,"e":1e5,"z":-0,"big":12345678901234567890,"u":"é\n"}`、深さ 300 の `[[[[…]]]]`、`{` を 65 個並べた文字、1 MiB + 1 文字。
- `SessionView` / `InsightsView` の固定値は `src/test/fixtures.ts` を上書きして作る（`sessionView({ canExportLogs: true, export: { status: "Running", … } })`）。
- 秘密の値はテストデータにも入れない。
