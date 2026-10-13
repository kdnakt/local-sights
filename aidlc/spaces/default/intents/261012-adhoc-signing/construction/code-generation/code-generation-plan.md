# Code Generation Plan — リリースの .app をアドホック署名する

## Summary

- Builds: Release ワークフローでのアドホック署名と署名検証ステップ、手順書の初回起動手順の書き直し、それらを機械的に確かめる Vitest のテスト
- Touches: `.github/workflows/release.yml`、`docs/install.md`、`docs/releasing.md`、`README.md`、`scripts/release/signing.ts`（新規）、`scripts/release/signing.test.ts`（新規）
- Tests: Vitest 約 12 件（純粋なチェック関数の単体テスト 6 件 + 実ファイルに対する要件ごとのテスト 6 件）。アプリのコードは変えないので `cargo test` は既存のまま

## 方針

- 対象はワークフロー定義と Markdown だけで、アプリのコード（Rust / React）は変えない。
- 要件ごとの検証は、既存の `scripts/release/versions.ts` と同じ作りにする。ファイルの中身を文字列で受け取る純粋な関数を `scripts/release/signing.ts` に置き、Vitest から (a) 合成した短い文字列で関数そのものを、(b) リポジトリの実ファイルを読んで要件を、それぞれ確かめる。YAML ライブラリは追加しない（行ベースの素朴な読み取りで足りる）。
- 純粋なロジック（チェック関数）はテストを先に書き、ワークフローと文書はそのあとで直し、最後に実ファイルのテストで要件を閉じる（Testing Contract の順序）。
- 文書は既存どおり英語で書く（FR3.6）。
- `release-dry-run.yml` と `src-tauri/tauri.conf.json` は触らない（FR1.2、FR1.3）。

## Steps

- [x] **Step 1. テストランナーの確認**（runner step）
  既存の `npx vitest run scripts/release/versions.test.ts` が通ることを確かめ、このユニットのテストコマンド `npx vitest run scripts/release/signing.test.ts` を `unit-test-instructions.md` のとおりに使えることを確認する。
  Traces: NFR2

- [x] **Step 2. チェック関数のテストを先に書く（Red）**
  `scripts/release/signing.test.ts` を新規作成し、`scripts/release/signing.ts` の次の関数を合成文字列で検証するテストを書く。この時点では関数が無いので失敗する。
  - `readTauriBuildSigningIdentity(workflowText)`：`run:` に `tauri build` を含むステップの `env:` から `APPLE_SIGNING_IDENTITY` の値を返す（ステップが無い／`env` が無い場合は `undefined`）。
  - `findSignatureCheckStep(workflowText)`：`codesign --verify` を含むステップを探し、ビルドステップの後かつ `.dmg` の存在確認ステップ（`target/release/bundle/dmg`）の前にあるかを返す。
  - `findUnsignedClaims(markdownText)`：「署名がまったくない」と読める文（`not signed`、`unsigned`。ただし `not signed with`、`not notarized`、`ad-hoc` を含む文は除く）の行番号を返す。
  - `terminalFallbackOnly(markdownText)`：`xattr` が、予備手順の見出し（`"is damaged"` を含む段落）より後にだけ現れるかを返す。
  テスト：各関数に正常系 1 件と境界 1 件（合計 6 件程度）。
  Traces: FR1.1, FR2.1, FR3.1, FR3.2

- [x] **Step 3. チェック関数を実装する（Green）**
  `scripts/release/signing.ts` を新規作成する。Node の API を使わず、`versions.ts` と同じく文字列を受け取って `Result` を返す純粋な関数として書く（Node の型剥がしで直接読める消去可能な構文のみ）。Step 2 のテストを通す。
  Traces: FR1.1, FR2.1, FR3.1, FR3.2

- [x] **Step 4. `release.yml` にアドホック署名と署名検証を入れる**
  - ビルドステップ名を「build the .dmg (ad-hoc signed)」に変え、`env:` に `APPLE_SIGNING_IDENTITY: "-"` を足す（FR1.1）。
  - ビルドステップの直後、「exactly one Apple Silicon .dmg」の前に、`target/release/bundle/macos/Local Sights.app` に対して `codesign --verify --deep --strict --verbose=2` を実行し、`codesign -dvv` の出力に `Signature=adhoc` が含まれることを確かめるステップを足す。どちらかが満たされなければ `::error::` を出して失敗させる（FR2.1、FR2.2）。
  - 冒頭コメントの「The app is not signed or notarized.」を「The app is ad-hoc signed in CI (no Apple certificate), not notarized.」の意味に改め、`NOTES` の「Unsigned macOS (Apple Silicon) build」を「Ad-hoc signed (not notarized) macOS (Apple Silicon) build」にする（FR1.4）。
  - 秘密情報は増やさない（NFR1）。`release-dry-run.yml` と `tauri.conf.json` は変えない（FR1.2、FR1.3）。
  Traces: FR1.1, FR1.2, FR1.3, FR1.4, FR2.1, FR2.2, FR2.3, NFR1, NFR3

- [x] **Step 5. 手順書と関連文書を直す（英語）**
  - `docs/install.md`「Before you install」：「not signed or notarized by Apple」を、アドホック署名はあるが Apple Developer ID の署名と公証はない、という説明に書き直す。会社のルール確認の趣旨は残す（FR3.1）。
  - `docs/install.md`「First launch」：見出しを「First launch: the "could not verify" warning」にし、macOS 15 以降は「警告で **Done** → システム設定の **Privacy & Security** で **Open Anyway** → もう一度開く」の GUI だけの流れを主手順にする。macOS 14 以前の右クリック → Open はそのまま。「is damaged and can't be opened」が出る場合は署名のない古い版か想定外のケースとして、`xattr -dr com.apple.quarantine` を節の末尾の予備手順に残す（FR3.2、FR3.3）。同じファイル内のアンカー参照（Update の節）も新しい見出しに合わせる。
  - `docs/releasing.md`：9 行目の「not signed or notarized」を同じ意味に直し、手順 7 に「初回起動で "could not verify" の警告が出て **Open Anyway** だけで開けること。"is damaged" が出たら署名の付いていないリリースの不具合として扱う」を足す。「When something fails」に署名検証ステップの失敗を 1 行足す（FR3.4、FR3.5）。
  - `README.md`：「the first-launch warning of the unsigned app」を「the first-launch warning (the app is ad-hoc signed, not notarized)」の意味に直す（FR3.4）。
  Traces: FR3.1, FR3.2, FR3.3, FR3.4, FR3.5, FR3.6

- [x] **Step 6. 実ファイルに対する要件テストを足して全体を通す**
  `scripts/release/signing.test.ts` に、リポジトリの実ファイルを読む `describe("repository")` を足す（`readFileSync(new URL("../../<path>", import.meta.url))`）。
  - `release.yml` のビルドステップの `APPLE_SIGNING_IDENTITY` が `-` である（FR1.1）。
  - `release-dry-run.yml` には `APPLE_SIGNING_IDENTITY` が無い（FR1.3）。
  - `release.yml` に署名検証ステップがあり、ビルドの後・`.dmg` 確認の前にある。`Signature=adhoc` の確認を含む（FR2.1、FR2.2）。
  - `docs/install.md`、`docs/releasing.md`、`README.md`、`release.yml` に「署名がまったくない」と読める記述が残っていない（FR3.1、FR3.4、FR1.4）。
  - `docs/install.md` の `xattr` は予備手順にだけ現れる（FR3.2、FR3.3）。
  - `docs/releasing.md` のリリース後の確認に "is damaged" の扱いがある（FR3.5）。
  その後、`npx vitest run`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit` を実行して全部通す（NFR2）。
  Traces: FR1.1, FR1.3, FR1.4, FR2.1, FR2.2, FR3.1, FR3.2, FR3.3, FR3.4, FR3.5, NFR2

- [x] **Step 7. 文書化と追跡**
  変更内容を `code-summary.md` に、要件 ID から実装・テストファイルへの対応を `traceability.json` に記録する（指揮側が作成する。開発者はこの Step の完了だけを印で示す）。
  Traces: FR1〜FR3, NFR1〜NFR3

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
  "scope": "express",
  "test_strategy": "minimal",
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
    "strategy": "minimal",
    "strategy_volume": [
      "One verifiable test per requirement at the narrowest effective level.",
      "At least one happy-path unit test per component.",
      "Unit tests are the default; a bugfix/security scope floor may require an integration or E2E regression when that is the narrowest level that reproduces the defect."
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
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:a690bda1f4080a58dab6b5642517222168d31bb94347044d2f505827f2562d61",
  "contract_sha256": "sha256:133b0fb6b9c9ac90d208c1a75da56fa396ed7daba09b0b7207b31ebe4e4bfd20"
}
```
