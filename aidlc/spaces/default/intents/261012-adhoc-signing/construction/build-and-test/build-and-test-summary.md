# Build and Test Summary — リリースの .app をアドホック署名する

上流：`construction/code-generation/code-generation-plan.md`、`unit-test-instructions.md`、`code-summary.md`。結果の詳細は `test-results.md`。

## ビルドの状態

ビルドは成功した。版の整合、画面のバンドル、整形・lint・型検査、`npm audit`、`cargo fmt`、Rust ライブラリのテストがすべて通った。`release.yml` は YAML として正しく、step の順序も要件どおり。前提と手順は `build-instructions.md`。

## 用意したテストの種類

| 種類 | ファイル | 状態 |
|---|---|---|
| 単体 | `construction/code-generation/unit-test-instructions.md` | 実行済み・24 件すべて成功（全体 320 件も成功） |
| 結合 | `integration-test-instructions.md` | 対象なし（Minimal）。ワークフローと macOS のつなぎ目は実リリースで確認 |
| 性能 | `performance-test-instructions.md` | 対象なし（性能の要件がない） |
| セキュリティ | `security-test-instructions.md` | 秘密情報を増やしていないことを確認済み |

## カバレッジの見込み

作業単位は 1 つ（作業単位を分けないワークフロー）。数値の下限はない（team.md）。要件 ID 16 件はすべて実装またはテストに対応づいている（`cross-unit-traceability.md`：PASS）。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| N/A | `nfr-requirements/`・`nfr-design/`（このワークフローになし）、`code-generation-plan.md` の Testing Contract | 数値で測る品質の目標なし（Testing Contract はカバレッジの下限を定めていない） | N/A | `test-results.md` | build-and-test | N/A |

## 準備の状態

| 観点 | 状態 |
|---|---|
| ビルドできる | はい |
| テストが通る | はい（Vitest 320 件、Rust ライブラリ 404 件） |
| 配布できる | はい（PR を作って CI が通ればマージでき、次のタグでアドホック署名付きの `.dmg` が出る）。ただし署名ステップと Gatekeeper の挙動は macOS でしか確かめられず、次のリリースで初めて実証される |

## 残っている項目

1. **実リリースでの確認**：次のタグで Release ワークフローの「the .app carries a valid ad-hoc signature」ステップが通ること、会社の Mac で初回起動が「could not verify」警告 → **Open Anyway** だけで開けること（`docs/releasing.md` 手順 7）。ダイアログの文言が違えば `docs/install.md` を直す。
2. **`cargo clippy` / `cargo deny` / `cargo test -p local-sights`**：この環境では未実行（Rust の変更なし）。CI に任せる。
3. **team.md の Deployment の文言**：「今は Apple の署名・公証をしない」はアドホック署名と矛盾しないが、「アドホック署名は行う」と明文化するかはこの作業の外で決める（`requirements.md` §7）。
