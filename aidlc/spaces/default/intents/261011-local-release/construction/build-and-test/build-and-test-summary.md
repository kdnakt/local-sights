# Build and Test Summary — リリース準備

## 全体の状態

- ビルド：画面側のビルドと版の検査は成功。アプリ本体と `.dmg` のビルドは、この Linux のコンテナでは実行できない（WebKitGTK の開発用パッケージがなく、`.dmg` は macOS が必要）。CI の `tauri-app` ジョブと、タグの push 時のリリース用ワークフローで確かめる。
- テスト：実行したコマンドはすべて成功（Vitest 195/195、cargo 346/346、この作業のテスト 24/24）。
- 静的検査：Prettier・ESLint・tsc・cargo fmt・npm audit・actionlint はすべて問題なし。

## テストの種類

| 種類 | 内容 | ファイル |
|------|------|----------|
| 単体 | 版の検査ロジック 21 件、表示名 3 件 | `scripts/release/versions.test.ts`、`src/App.title.test.tsx` |
| 結合 | 入口と実ファイル、ワークフローの定義 | integration-test-instructions.md |
| 性能 | ワークフローのタイムアウトの設定 | performance-test-instructions.md |
| セキュリティ | 秘密情報・権限・送信なしの確認 | security-test-instructions.md |
| 既存 | 画面側 171 件、ライブラリ 346 件 | 既存のテスト |

## 作業ごとのカバレッジの目安

作業単位なし（1 回の実装）。数値の下限はなく、ライブラリ側にあたる `versions.ts` の公開関数にはすべてテストがある。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|-----------|--------|----------|--------|----------|--------------|---------|
| TC-STRATEGY | code-generation-plan.md › Testing Contract › obligations.strategy_volume | 部品ごとに 5〜8 件の単体テスト＋主要な境界の結合テスト | 版の検査 21 件、表示名 3 件、境界の結合確認あり | test-results.md、integration-test-instructions.md | build-and-test | Met |
| TC-SCOPE-REGRESSION | Testing Contract › obligations.scope_floor[0] | 対象の振る舞いを狙った回帰テスト | 版の不一致・タグの形の誤りのテストと入口での失敗確認 | `scripts/release/versions.test.ts`、test-results.md | build-and-test | Met |
| TC-SCOPE-GREEN | Testing Contract › obligations.scope_floor[1] | 既存のテストがすべて通る | Vitest 195/195、cargo 346/346 | test-results.md | build-and-test | Met |
| TC-TEAM-PUBLIC-FN | Testing Contract › applicable_notes（team） | `versions.ts` の公開関数すべてにテスト | 5/5 | `scripts/release/versions.test.ts` | build-and-test | Met |
| TC-TEAM-NO-AWS | Testing Contract › applicable_notes（team） | テストとワークフローに AWS の接続・認証情報がない | 0 件 | security-test-instructions.md、test-results.md | build-and-test | Met |

NFR Requirements と NFR Design の工程はこの計画に含まれないため、そこからの数値目標はない。

## 準備の状態

- ビルドの準備：できている（画面側・版の検査）。アプリ本体と `.dmg` は CI・タグの push で確かめる。
- テストの準備：できている。
- 配布の準備：リリース用ワークフローと手順書はそろった。実際の公開は、マージ後に本人がタグ `v0.1.0` を push したときに初めて行われる。

## 既知の制限・残り

- 要件の FR3.2（Release が下書きでなくすぐ公開される）は、ワークフローに実装済みだが、実際の公開はタグの push まで確かめられない（cross-unit-traceability.md の唯一の未カバー）。
- アプリ本体（`src-tauri`）のビルドは、このコンテナでは実行していない。CI の `tauri-app` ジョブで確かめる。
- `docs/install.md` の初回起動の手順（「このまま開く」、`xattr`）は、手元の Mac で確かめる必要がある。
- チームの方針（`team.md` の Deployment）は「下書きとして置く」のまま。実装は「すぐ公開」。
- `package-lock.json` の版は検査していない（手順書では `npm version` で一緒に上げる）。
