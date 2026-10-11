# Code Generation Plan — リリース準備（会社 PC へのインストール）

## Summary

- Builds: 表示名 `Local Sights` への変更、版番号の検査スクリプト、タグで `.dmg` を作って公開するリリース用ワークフロー、インストール手順書とリリース手順書
- Touches: `src-tauri/tauri.conf.json`、`index.html`、`src/i18n/messages.ts`、`scripts/release/`（新規）、`package.json`、`vitest.config.ts`、`tsconfig.json`、`.github/workflows/ci.yml`、`.github/workflows/release.yml`（新規）、`docs/install.md`・`docs/releasing.md`（新規）、`README.md`
- Tests: 版番号の検査ロジックの単体テスト 12 件（Vitest、テスト先行）と、表示名の画面テスト 1 件。既存のテストはすべて通ったまま

## 対象と前提

作業単位への分割はしない（作業単位なしの 1 回の実装）。入力は `inception/requirements-analysis/requirements.md`（承認済み）と、既存のワークスペースである。

要件定義のレビューで残った指摘は、この計画で次のように決める。

| 指摘 | この計画での決定 |
|------|------------------|
| R-01（NFR4：品質の門が二択） | リリース用ワークフロー自身が、ビルドの前に `cargo test -p local-sights-core` と `npx vitest run` を実行する。`main` の CI の結果は待たない（CI が実行中でも、リリース側のテストが通らなければ公開しない）。 |
| R-02（FR3.5：同じタグでの再実行） | 同じタグの Release がすでにある場合は、`.dmg` を置き換える（`gh release upload --clobber`）。ない場合は新しく作って公開する。二重に作ることはない。 |
| R-03（`Cargo.lock` の版） | 版の検査で、`Cargo.lock` の中の `local-sights` と `local-sights-core` の版も 3 か所と同じかを調べる。手順書に「版を上げたら `cargo update -w` で `Cargo.lock` を更新する」と書く。 |
| R-04（前の版に戻したときの設定） | 保存形式の互換性はこの作業の対象外とする。手順書に、古い版で設定やキャッシュが読めないときは、キャッシュのフォルダを消して取り直す手順を書く。 |
| R-05（合否の基準がない要件） | 下の「確かめ方」の表に、要件ごとの確かめ方をまとめる。 |
| R-06（手元の Mac での起動確認） | リリース手順書の最後に「公開された `.dmg` を手元の Mac に入れて起動する」確認を入れる。 |

手順書（`docs/install.md`・`docs/releasing.md`）は、既存の README に合わせて英語で書く。日本語にしたい場合は、計画の承認のときに伝えてほしい。

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
  "input_sha256": "sha256:4a9e8205119375600f4f1dfea7f2a0812035dcd54c2d1b6528f63043a9264e7b",
  "contract_sha256": "sha256:1bf2ecb53169c83385de5c21175da261508d15336ffcdcbe3001777d493227d6"
}
```

この作業での当てはめ：版番号の検査（タグの形の判定、ファイルからの版の取り出し、食い違いの判定）は純粋なロジックなので、テストを先に書いてから実装する。表示名の変更（画面）と、ファイルを読む薄い入口・ワークフロー・手順書は、先に実装してからテストや確認を行う。

## 手順

### Step 1: 構成の土台 [FR2]

- [x] `scripts/release/` を新しく作る。純粋なロジックは `scripts/release/versions.ts`（Node の API を使わない）に置き、ファイルを読んで結果を表示する入口は `scripts/release/check-version.mjs` に分ける。
- [x] `tsconfig.json` の `include` に `scripts` を足し、`vitest.config.ts` の `include` に `scripts/**/*.test.ts` を足す（既存の `src/**/*.test.{ts,tsx}` は残す）。ESLint は既存の設定のまま `.ts` を対象にする。

### Step 2: テストの実行環境を確かめる

- [x] 既存の Vitest で、この作業のテストだけを実行するコマンドが動くことを確かめ、`unit-test-instructions.md` に記録する：`npx vitest run scripts/release/versions.test.ts src/App.title.test.tsx`（Step 3 の時点ではファイルがまだないため、まず `npx vitest run scripts/release` が「テストなし」で終わることを確かめる）。

### Step 3: 版番号の検査ロジックのテストを先に書く（失敗を確かめる）[FR2.1〜FR2.5]

- [x] `scripts/release/versions.test.ts` に次のテストを書く。`versions.ts` がまだないため、すべて失敗することを確かめて記録する。
  1. `parseTag("v0.1.0")` が `0.1.0` を返す。
  2. `parseTag` が `v1.2`、`1.2.3`、`release-1`、`v01.2.3`（先頭のゼロ）、`v1.2.3-rc.1`（今回は扱わない）を、理由つきで拒む。
  3. `readCargoWorkspaceVersion` が `Cargo.toml` の `[workspace.package]` の `version` を取り出す（他の節の `version` は拾わない）。
  4. `readCargoWorkspaceVersion` が `[workspace.package]` や `version` がないときに、分かるエラーを返す。
  5. `readCargoLockVersions` が `Cargo.lock` から `local-sights` と `local-sights-core` の版を取り出す（名前が似た別のパッケージは拾わない）。
  6. `readJsonVersion` が `package.json`・`tauri.conf.json` の `version` を取り出し、ない・文字列でないときはエラーを返す。
  7. `checkVersions` が 5 つの版（Cargo.toml・Cargo.lock の 2 つ・package.json・tauri.conf.json）がすべて同じなら成功する。
  8. `checkVersions` が 1 つでもずれていれば、ずれたファイルとその値を示して失敗する（例：`package.json: 0.1.1`）。
  9. `checkVersions` にタグを渡したとき、版がそろっていてもタグと違えば失敗する（例：タグ `v0.2.0`、版 `0.1.0`）。
  10. `checkVersions` にタグを渡し、版もタグも一致すれば成功する。
  11. `checkVersions` に形の違うタグを渡すと、版の比較の前に失敗する。
  12. 失敗の文が、秘密情報を含まない固定の形（ファイル名と版だけ）である。

### Step 4: 版番号の検査ロジックを実装する（テストを通す）[FR2.1〜FR2.5]

- [x] `scripts/release/versions.ts` に `parseTag`・`readCargoWorkspaceVersion`・`readCargoLockVersions`・`readJsonVersion`・`checkVersions` を実装する。外部のライブラリは足さない（TOML は必要な部分だけを行単位で読む）。
- [x] `npx vitest run scripts/release/versions.test.ts` がすべて通ることを確かめる。通ったあと、読みやすさを整える。

### Step 5: 検査の入口と npm のコマンドを足す [FR2.2、FR2.5]

- [x] `scripts/release/check-version.mjs` を実装する：リポジトリの 4 つのファイル（`Cargo.toml`、`Cargo.lock`、`package.json`、`src-tauri/tauri.conf.json`）を読み、`versions.ts` の `checkVersions` を呼ぶ。`--tag vX.Y.Z` を受け取ったときはタグとも比べる。成功なら版を 1 行表示して終了コード 0、失敗なら理由を標準エラーに出して終了コード 1。ファイルが読めないときもファイル名を示して 1 で終わる。
- [x] `package.json` に `"check:version": "node scripts/release/check-version.mjs"` を足す（Node 22.18 以降は `.ts` をそのまま読み込める。必要な Node の版を手順書に書く）。
- [x] 手元で `npm run check:version` が成功すること、`npm run check:version -- --tag v9.9.9` が失敗することを確かめる。

### Step 6: 表示名を `Local Sights` にする（実装してからテスト）[FR1.1〜FR1.3]

- [x] `src-tauri/tauri.conf.json` の `productName` を `Local Sights`、`app.windows[0].title` を `Local Sights` にする。`identifier` は `dev.local-sights.app` のまま変えない。
- [x] `index.html` の `<title>` と、`src/i18n/messages.ts` の `app.title`（英語・日本語とも）を `Local Sights` にする。
- [x] 表示名の画面テスト `src/App.title.test.tsx` を書く：アプリの見出しが `Local Sights` と表示される。既存の画面テストに `local-sights` の見出しを前提にしたものがあれば合わせて直す。

### Step 7: 通常の CI に版の検査を足す [FR2.5、NFR6]

- [x] `.github/workflows/ci.yml` の `frontend` ジョブに `npm run check:version` の手順を足す。ほかのジョブ・権限（`contents: read`）は変えない。

### Step 8: リリース用ワークフローを作る [FR3.1〜FR3.8、NFR1〜NFR4、NFR7]

- [x] `.github/workflows/release.yml` を作る。
  - きっかけ：`push` の `tags: ["v*"]` だけ。全体の権限は `contents: read`。同じタグで重ならないよう `concurrency` を設定する（取り消しはしない）。
  - ジョブ `verify`（ubuntu、タイムアウト 10 分）：全履歴を取得し、タグの形を確かめ（`npm run check:version -- --tag "$GITHUB_REF_NAME"`）、タグのコミットが `origin/main` に含まれることを `git merge-base --is-ancestor` で確かめる。含まれなければ理由を出して失敗する。
  - ジョブ `test`（ubuntu、タイムアウト 20 分、`needs: verify`）：`cargo test -p local-sights-core` と `npm ci` → `npx vitest run` を実行する。
  - ジョブ `build-and-publish`（`macos-latest` の Apple Silicon、タイムアウト 40 分、`needs: [verify, test]`）：このジョブだけ `contents: write`。`npm ci` → `npm run tauri build -- --bundles dmg` で `.dmg` を作り、`target/release/bundle/dmg/` に `.dmg` がちょうど 1 つあることを確かめる。そのタグの Release があれば `gh release upload "$TAG" <dmg> --clobber` で置き換え、なければ `gh release create "$TAG" <dmg> --verify-tag --generate-notes --title "Local Sights $TAG"` で作って公開する。`GITHUB_TOKEN` だけを使う。
  - 既存の CI と同じ版の actions（`actions/checkout@v7`、`actions/setup-node@v7`、`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`）を使う。AWS の認証情報や他の秘密情報は使わない。
- [x] 失敗したときは Release を作らないこと（`build-and-publish` は前のジョブがすべて成功したときだけ動き、Release の作成は `.dmg` の確認の後に置く）を、ワークフローの並びで確かめる。

### Step 9: 手順書を書く [FR4.2、FR4.3、FR5.1〜FR5.8]

- [x] `docs/install.md`（英語）：必要なもの（macOS Apple Silicon、`~/.aws/config` の準備は README を参照）、入れる前に会社のルールを確かめること、GitHub Releases からの取得、`/Applications` と `~/Applications` の 2 通りの入れ方、初回起動の警告の開き方（「システム設定」→「プライバシーとセキュリティ」→「このまま開く」、および `xattr -dr com.apple.quarantine "<path>/Local Sights.app"`）、更新、前の版への戻し方、版の確かめ方、古い版で設定やキャッシュが読めないときにキャッシュのフォルダ（`~/Library/Caches/dev.local-sights.app/`）を消す方法。秘密情報の例は書かない。
- [x] `docs/releasing.md`（英語）：3 か所の版を上げる → `cargo update -w` で `Cargo.lock` を更新 → `npm run check:version` → プルリクエストを `main` にスクワッシュマージ → `main` を取り込んでタグ `vX.Y.Z` を付けて push → ワークフローの成功と Release の公開を確かめる → 公開された `.dmg` を手元の Mac に入れて起動する。同じタグで再実行したときの扱い（`.dmg` を置き換える）と、失敗したときに Release が作られないことも書く。
- [x] `README.md`：見出しと冒頭を `Local Sights` に合わせ、「インストール」の節から `docs/install.md` へ、開発者向けの節から `docs/releasing.md` へ案内する。`cargo install` の手順は開発用として残す。

### Step 10: 整形・静的検査・テストの通しと記録

- [x] `npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`cargo fmt --all --check`、`cargo test -p local-sights-core` を実行し、すべて通ることを確かめる。
- [x] `code-summary.md` と `traceability.json` を書く。

## 確かめ方（要件ごと）

| 要件 | 確かめ方 | この工程で確かめる | 後で確かめる |
|------|----------|--------------------|--------------|
| FR1.1、FR1.2 | `tauri.conf.json` の `productName`・ウィンドウのタイトル、`src/App.title.test.tsx` | テスト・設定の確認 | 公開後の `.dmg` を手元の Mac で開く（`docs/releasing.md` の最後） |
| FR1.3 | `tauri.conf.json` の `identifier` が変わっていない | 設定の確認 | — |
| FR2.1〜FR2.5 | `scripts/release/versions.test.ts`、`npm run check:version`、CI の手順 | テスト・コマンドの実行 | — |
| FR3.1〜FR3.8 | `release.yml` の内容（きっかけ、`needs`、`main` の確認、`--clobber`、`--verify-tag`、`--bundles dmg`） | ワークフローの読み合わせ（構文の検査ができれば行う） | 本人がタグ `v0.1.0` を push したとき |
| FR4.1〜FR4.3 | `docs/install.md` の該当の節、ワークフローに署名の手順や Apple の秘密情報がない | 文書の確認 | — |
| FR5.1〜FR5.8 | `docs/install.md`・`docs/releasing.md`・`README.md` の該当の節 | 文書の確認 | 手元の Mac で手順をたどる |
| NFR1、NFR2 | `release.yml` で `contents: write` が `build-and-publish` だけにある、`secrets.` の参照が `GITHUB_TOKEN` だけ、AWS の文字列がない | ワークフローの確認（grep） | — |
| NFR3 | `npm ci` とロックファイルどおりのビルド | ワークフローの確認 | — |
| NFR4 | `build-and-publish` が `test` の成功を待つ | ワークフローの確認 | — |
| NFR5 | 自動更新・テレメトリの設定や依存を足していない | 差分の確認 | — |
| NFR6 | Step 10 のコマンドがすべて通る | コマンドの実行 | CI |
| NFR7 | 各ジョブに `timeout-minutes` がある | ワークフローの確認 | 本人のタグの push |

この作業のコンテナは Linux のため、macOS の `.dmg` のビルドはここでは実行できない。実際のビルドと公開は、マージ後に本人がタグを push したときに確かめる（要件の前提のとおり）。
