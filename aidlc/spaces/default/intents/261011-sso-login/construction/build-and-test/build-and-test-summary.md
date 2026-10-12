# Build and Test Summary — アプリから SSO ログインする

上流：`construction/code-generation/code-generation-plan.md`、`unit-test-instructions.md`、`code-summary.md`。結果の詳細は `test-results.md`。

## ビルドの状態

ビルドは成功した。Rust のワークスペース全体、Tauri のアプリ、画面のバンドルがビルドでき、整形・lint・型検査・`npm audit` もすべて通った。`cargo deny` はこの環境に道具がないため実行しておらず、CI に任せる。前提と手順は `build-instructions.md`。

## 用意したテストの種類

| 種類 | ファイル | 状態 |
|---|---|---|
| 単体 | `construction/code-generation/unit-test-instructions.md` | 実行済み・すべて成功 |
| 結合 | `integration-test-instructions.md` | 自動の分は実行済み・成功。コマンドの登録と実機の確認は未 |
| セキュリティ | `security-test-instructions.md` | grep とテストの分は確認済み。実機のログ確認は未 |
| 性能 | `performance-test-instructions.md` | 対象なし（性能の要件がない） |

## 作業単位ごとのカバレッジの見込み

作業単位は 1 つ（作業単位を分けないワークフロー）。数値の下限はない（team.md）。コアの公開関数にはすべてテストがある。増えたテストは、コア 43 件、Tauri 側 12 件、画面 21 件。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| N/A | `nfr-requirements/`・`nfr-design/`（このワークフローになし）、`code-generation-plan.md` の Testing Contract | 数値で測る品質の目標なし（Testing Contract はカバレッジの下限を定めていない） | N/A | `test-results.md` | build-and-test | N/A |

## 準備の状態

| 観点 | 状態 |
|---|---|
| ビルドできる | はい |
| テストが通る | はい（Rust 全体、画面 291 件） |
| 配布できる | **いいえ**。新しいコマンドが Tauri に登録されていないため、実機で機能が動かない見込み（R-01） |

## 残っている項目

1. **R-01（重大）コマンドの未登録**：`start_sso_login`・`cancel_sso_login`・`dismiss_sso_login`・`open_sso_url` を `src-tauri/build.rs` の `AppManifest::commands` と `src-tauri/capabilities/default.json` の `allow-*` に足す必要がある。コード生成の承認では受け入れたリスクとして記録されているが、このままでは機能が使えない。直すなら、`api.ts` の `invoke` 名と登録の一覧を突き合わせるテストもあわせて足すのがよい。
2. **R-02 CI の範囲**：CI に `cargo test -p local-sights` がなく、`sso_cli` の 12 件が CI で動かない。CI の tauri-app ジョブは GTK を入れてアプリをビルドしているので、そこにテストの 1 行を足せる。
3. **R-03 キャンセル直後の押し直し**：旧 CLI が止まるまでの最大 2 秒の間に押し直すと、CLI が短時間 2 つ動く。状態は壊れない。
4. **R-04 URL の許可リスト**：実装は BR3.1 の文面より厳しい（安全側）。機能設計の文面は未更新。
5. **Mac の実機での確認**：README のチェックリスト（直接の SSO、AssumeRole、URL で入る、キャンセル・時間切れ・Cmd+Q・Dock の「終了」で `aws` が残らない）。Dock の「終了」で `RunEvent::Exit` が届くかは未確認。
6. **`cargo deny`**：この環境では未実行。
