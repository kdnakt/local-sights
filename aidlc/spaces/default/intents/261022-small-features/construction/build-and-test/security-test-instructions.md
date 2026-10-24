# Security Test Instructions — アプリ内の言語切替・書き出し・JSON の整形（261022-small-features）

要件の NFR5（秘密の扱い）と project.md の Forbidden（秘密の認証情報を書かない、読み取りの 3 つの API 以外を呼ばない、CI から AWS に接続しない、テレメトリを送らない）、team.md の依存関係のチェックを対象にする。

## 1. 確かめること

| 項目 | 由来 | 確かめ方 |
|---|---|---|
| 設定ファイルに書くのは `formatVersion`・`cacheEnabled`・`language` の 3 つだけ | NFR5、BR1.5 | `cache::settings` のテスト `language_is_read_back_and_defaults_to_system`（ファイルの中身の文字そのもの） |
| 書き出しの中身は利用者が取得したログそのもので、アプリは検査も加工もしない | NFR5、FR4.1.4、AS3 | `export::formatter` のテスト `csv_leaves_plain_cells_and_formula_like_values_untouched`。式のような値（`=` で始まる）もそのまま出す。表計算ソフトで開くときの注意を README に書いた |
| 失敗の知らせに OS のエラーの文を出さない（秘密やパス以外の情報が混じらない） | NFR5、U2 レビュー R-02 | `ExportFailure` の enum と `path` だけを画面に渡し、OS の文は標準エラーにだけ出す。`export_boundary` の `export_failure_kinds_follow_the_io_error_kind` |
| 新しい API を呼ばない | Forbidden | `git diff main...HEAD -- crates src-tauri` に AWS の呼び出しの追加がない（この作業は AWS を呼ばない） |
| 外部に何も送らない | Forbidden | 足した依存（dialog・clipboard-manager）はローカルの OS の機能だけ。`git diff` に HTTP の送信の追加がない |
| プラグインの WebView 権限を広げない | U2 レビュー R-04 | `capabilities/default.json` に足したのは `allow-start-export`・`allow-dismiss-export-notice` だけ。`commandRegistration.test.ts` の「grants no permission for a command the app does not have」 |
| 一時ファイルを残さない | FR4.4.3、BR4.6 | `export_boundary` の `export_write_failure_removes_the_temporary_and_keeps_the_old_file` ほか |
| 依存関係に高い深刻度の勧告がない | team.md Code Style | `npm audit --audit-level=high`、`cargo deny check`（advisories・licenses・bans・sources） |

## 2. 流し方

```bash
cargo test -p local-sights-core cache::settings
cargo test -p local-sights-core export::formatter
cargo test -p local-sights export_boundary
npx vitest run src/commandRegistration.test.ts
npm audit --audit-level=high
cargo deny check
```

## 3. 秘密の扱い

- 自動テストのデータに秘密の値（アクセスキー・トークン）を入れない。
- 手元の確かめで保存した CSV / Markdown は、ログの中身を含むため、リポジトリにも記録にも置かない。

## 4. `npm audit` の既存の勧告

Build and Test の時点で `npm audit --audit-level=high` は high を 2 件報告した（`undici` 8.0.0〜8.12.1、`zod` < 4.6.6）。どちらも推移的な依存で、この作業では `package.json` を変えていない。扱いは `test-results.md` の 1 のとおり（`npm audit fix` による lockfile の更新をこの段で行い、CI の `npm audit` を緑に戻す）。
