# Code Summary — リリース準備（会社 PC へのインストール）

作業単位なしの 1 回の実装。承認済みの `code-generation-plan.md` の Step 1〜10 をこの順に行った。

## 作った・変えたファイル

| ファイル | 種類 | 内容 |
|----------|------|------|
| `scripts/release/versions.ts` | 新規 | 版の検査の純粋なロジック（`parseTag`・`readCargoWorkspaceVersion`・`readCargoLockVersions`・`readJsonVersion`・`checkVersions`）。Node の API を使わず、外部のライブラリも足していない。 |
| `scripts/release/versions.test.ts` | 新規 | 上の単体テスト（計画の 1〜12 を 21 ケースで確認）。テスト先行で書いた。 |
| `scripts/release/check-version.mjs` | 新規 | 4 つのファイルを読んで `checkVersions` を呼ぶ入口。`--tag vX.Y.Z` に対応。終了コード 0＝一致、1＝不一致・形の誤り・読めないファイル、2＝引数の誤り。 |
| `src/App.title.test.tsx` | 新規 | 見出しが `Local Sights`（英語・日本語）であることと、`tauri.conf.json` の `productName`・ウィンドウのタイトル・`identifier` の確認。 |
| `.github/workflows/release.yml` | 新規 | タグ `v*` の push で `verify` → `test` → `build-and-publish`（macOS Apple Silicon）。 |
| `docs/install.md` | 新規（英語） | 会社のルールの確認、取得、`/Applications` と `~/Applications` の入れ方、初回起動の警告、版の確かめ方、更新、前の版への戻し方、キャッシュの消し方。 |
| `docs/releasing.md` | 新規（英語） | 版の上げ方、`cargo update -w`、タグの付け方、失敗したとき、同じタグでの再実行。 |
| `src-tauri/tauri.conf.json` | 変更 | `productName` とウィンドウのタイトルを `Local Sights` に。`identifier` は変えていない。 |
| `index.html` | 変更 | `<title>` を `Local Sights` に。 |
| `src/i18n/messages.ts` | 変更 | `app.title` を英語・日本語とも `Local Sights` に。 |
| `package.json` | 変更 | `"check:version"` のスクリプトを追加。依存関係は変えていない。 |
| `tsconfig.json` | 変更 | `include` に `scripts` を追加。 |
| `vitest.config.ts` | 変更 | `include` に `scripts/**/*.test.ts` を追加（既存の `src/**/*.test.{ts,tsx}` は残した）。 |
| `.github/workflows/ci.yml` | 変更 | `frontend` ジョブの `npm ci` の後に `npm run check:version` を追加。権限やほかのジョブは変えていない。 |
| `README.md` | 変更 | 見出しを `Local Sights` に。「Install」節（`docs/install.md` へ）と「Releasing」節（`docs/releasing.md` へ）を追加。`cargo install` は開発用として残した。 |

Rust のコードは変えていない。

## 主な決定

- **同じタグでの再実行（R-02、FR3.5）**：そのタグの Release があれば `gh release upload --clobber` で `.dmg` を置き換え、なければ `gh release create --verify-tag --generate-notes` で作って公開する（下書きにしない）。
- **添付ファイル名**：GitHub は添付ファイル名の空白を書き換えるため、ビルドした `Local Sights_X.Y.Z_aarch64.dmg` を空白のない `Local-Sights_X.Y.Z_aarch64.dmg` に写してからアップロードする。これで再実行時の `--clobber` が同じ名前を確実に置き換える（計画にはなかった細部。下の「計画との違い」を参照）。
- **品質の門（R-01、NFR4）**：リリース用ワークフロー自身が `cargo test -p local-sights-core` と `npx vitest run` を実行し、`build-and-publish` は `verify` と `test` の両方の成功を待つ。
- **`main` の確認（FR3.4）**：`verify` で全履歴を取得し、タグが指すコミットが `origin/main` の祖先であることを `git merge-base --is-ancestor` で確かめる。
- **Apple Silicon だけ（FR3.8）**：`build-and-publish` の最初にランナーが `arm64` であることを確かめ、`.dmg` がちょうど 1 つで名前に `aarch64` を含むことを確かめてから公開する。
- **Release の中身（FR3.6）**：自動生成のリリースノートの前に、作ったコミット・タグ・ワークフローの実行の URL を書く。
- **秘密情報（NFR1）**：使うのは `github.token`（`GITHUB_TOKEN`）だけ。`secrets.` の参照はない。タグ名は `env` 経由で渡し、シェルに直接埋め込まない。
- **版の検査の失敗の文**：ファイル名・パッケージ名・版の値だけを含む固定の文にした。JSON の構文エラーでも、ファイルの中身を引用しうるパーサーのメッセージは出さない。
- **package-lock.json**：検査の対象にはしていないが、手順書では `npm version X.Y.Z --no-git-tag-version` で `package.json` と一緒に上げるよう書いた。

## テスト

テストの進め方は Testing Contract（custom）のとおり。

- 純粋なロジック（版の検査）はテスト先行：Step 3 で `versions.test.ts` を書き、`versions.ts` がない状態で失敗することを確かめてから（赤）、Step 4 で実装して通した（緑）。
- 画面（表示名）・入口・ワークフロー・手順書は先に実装し、その後でテストや確認を行った。

実行したコマンドと結果：

| コマンド | 結果 |
|----------|------|
| `npx vitest run`（変更前のベースライン） | 成功：171 件 |
| `cargo test -p local-sights-core`（変更前のベースライン） | 成功：305 件（5 件は ignored）＋ 統合テスト 11 件 ほか |
| `npx vitest run scripts/release`（Step 2、テストファイルなし） | 想定どおり「No test files found」で終了コード 1 |
| `npx vitest run scripts/release/versions.test.ts`（Step 3、実装前） | 想定どおり失敗：`Failed to resolve import "./versions"`、Test Files 1 failed、終了コード 1 |
| `npx vitest run scripts/release/versions.test.ts`（Step 4、実装後） | 成功：21 件 |
| `npm run check:version` | 成功（0.1.0、終了コード 0） |
| `npm run check:version -- --tag v0.1.0` | 成功（終了コード 0） |
| `npm run check:version -- --tag v9.9.9` | 想定どおり失敗（タグと版の不一致、終了コード 1） |
| `npm run check:version -- --tag v1.2` | 想定どおり失敗（タグの形、終了コード 1） |
| `npm run check:version -- --bogus` | 想定どおり失敗（使い方を表示、終了コード 2） |
| `npx vitest run scripts/release/versions.test.ts src/App.title.test.tsx` | 成功：24 件 |
| `actionlint`（v1.7.7）で `release.yml` と `ci.yml` | 問題なし（shellcheck が入っていないため、`run:` の中のシェルの検査は行われていない） |
| `npx prettier --check .` | 成功（最初は `docs/releasing.md` と `versions.ts` の 2 件が整形の対象。`prettier --write` で整えた後に成功） |
| `npx eslint .` | 成功 |
| `npx tsc --noEmit` | 成功 |
| `npx vitest run` | 成功：23 ファイル・195 件（既存 171 件 ＋ 新規 24 件） |
| `cargo fmt --all --check` | 成功 |
| `cargo test -p local-sights-core` | 成功（ベースラインと同じ件数） |
| `npm audit --audit-level=high` | 成功：0 件 |

公開関数 5 つ（`parseTag`・`readCargoWorkspaceVersion`・`readCargoLockVersions`・`readJsonVersion`・`checkVersions`）にはすべてテストがある。数値のカバレッジ下限はない（team.md）ため、カバレッジは測っていない。テストはファイルの中身を文字列で渡し、実際のファイル・AWS・GitHub には接続しない。

## この工程で確かめられなかったこと

- macOS の `.dmg` のビルド（`npm run tauri build -- --bundles dmg`）と GitHub Releases への公開は、Linux のコンテナでは実行できないため行っていない。マージ後に本人がタグ `v0.1.0` を push したときに確かめる（`docs/releasing.md` の手順 6・7）。
- `check-version.mjs` の「ファイルが読めない」場合の出力は、リポジトリのファイルを消さずには試せないため、手で実行していない（コードの読み合わせのみ）。
- `docs/install.md` の初回起動の手順（macOS 15 以降の「このまま開く」、`xattr`）は、手元の Mac で本人が確かめる。

## 計画との違い

- `src/App.title.test.tsx` は、計画の「見出しの 1 件」に加えて、英語・日本語の両方の見出しと、`tauri.conf.json` の `productName`・ウィンドウのタイトル・`identifier` の確認を入れた（FR1.1〜FR1.3 の回帰を防ぐため。3 ケース）。
- `versions.test.ts` には、計画の 12 項目に加えて、`Cargo.lock` に作業領域のパッケージがないときと、JSON の誤りの補助ケースを入れた。
- リリース用ワークフローで、アップロードの前に `.dmg` を空白のない名前に写す手順と、ランナーが `arm64` であることを確かめる手順を足した（上の「主な決定」）。
- 計画の Step 2 の「`unit-test-instructions.md` に記録する」は、コマンドがすでにその文書に書かれていたため、文書は変えていない（実行結果はこの表に記録した）。
- Step 1 の `scripts/release/` は、シェルで `mkdir` せず、Step 3 で最初のファイルを書いたときに作られた。
