# Phase Check — Construction → Operation

対象：`aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/` 配下の成果物（U1〜U7 のコード生成、Build and Test、CI Pipeline）。このスコープ（`local-tool`）では Operation の各ステージ（4.1〜4.7）を実行しないため、この確認は Construction の締めくくりであり、ワークフロー全体の最終確認でもある。確認日：2026-10-10。

## 1. 作業単位がすべて作られ、テストされたか

| 単位 | コード生成 | レビュー（最終） | テスト（Build and Test） | 結果 |
|------|------------|------------------|--------------------------|------|
| U1 薄い一本 | 済 | READY（3 回目） | ライブラリ 162 件・結合 11 件・画面 64 件 成功 | OK |
| U2 接続とロググループの選択 | 済 | READY（3 回目） | 132 件・結合 9 件・画面 72 件 成功 | OK |
| U3 取得の作り込み | 済 | READY（3 回目） | 162 件・結合 14 件・画面 89 件 成功 | OK |
| U4 時間範囲とタイムゾーン | 済 | READY（2 回目） | 124 件・画面 69 件 成功 | OK |
| U5 絞り込み | 済 | READY（2 回目） | 104 件・結合 4 件・画面 56 件 成功 | OK |
| U6 ディスクキャッシュ | 済 | READY（2 回目） | 135 件・結合 3 件・画面 69 件 成功 | OK |
| U7 画面の仕上げ | 済 | READY（3 回目。1 回目は NOT-READY、指摘はすべて同じ単位で対応） | 145 件・画面 123 件 成功 | OK |

全体：ライブラリ 346 件・画面 171 件、失敗 0（`construction/build-and-test/test-results.md`）。

## 2. コード生成の対応表に未解決の指摘がないか

7 単位の `code-generation/traceability.json` を読んだ。`GAP`・`ORPHAN` の行はなく、`OK` の行の対応先のファイルはすべて存在する。`N/A` は U6 の FR7.8（キャッシュ全体を MVP から外してよいという選択肢）の 1 行だけ。

## 3. 単位をまたいだ FR / NFR のゲート

`construction/build-and-test/cross-unit-traceability.md`：70 件のうち 66 件が `OK`。判定は **FAIL（対応表に記入のない ID が 4 件）**。

| ID | 状況 | この確認での扱い |
|----|------|------------------|
| FR7.8 | U6 で `N/A`。選択肢であり、コードの対応先を持たない | 対応不要（記入の仕方の問題）。未解決の機能ではない |
| NFR7 外部に送らない | 対応表に ID がない。Build and Test の静的確認で満たしていることを確認（外部通信なし、CSP は IPC だけ） | 証拠あり。対応表への追記が残る |
| NFR9 macOS で動く | 対応表に ID がない。この環境では確かめられず、CI Pipeline の `tauri-app`（macos-latest）で組み立てを確かめる。起動の目視は手元 | CI で組み立てを確認する。対応表への追記が残る |
| NFR14 テストしやすい構成 | 対応表に ID がない。Build and Test で構成を確認（ライブラリと GUI の分離、trait、偽物の gateway） | 証拠あり。対応表への追記が残る |

Build and Test の承認（2026-10-10）は、この 4 件を指摘として示したうえで行われた。

## 4. CI のゲートが Build and Test のコマンドを強制しているか

| Build and Test のコマンド | CI のジョブ・ゲート |
|---------------------------|---------------------|
| `cargo fmt --all --check` | `rust-core` G1 |
| `cargo clippy -p local-sights-core --all-targets` | `rust-core` G2 |
| `cargo test -p local-sights-core`（単体 + 結合） | `rust-core` G3 |
| `npx tsc --noEmit`、`npx eslint .`、`npx prettier --check .` | `frontend` G4・G5 |
| `npx vitest run` | `frontend` G6 |
| `npx vite build` | `frontend` G7 |
| `npm audit` | `frontend` G8（high 以上） |
| `cargo deny check`（Build and Test では未実行。CI Pipeline で実行し、依存の変更と例外 1 件のあと全項目 ok） | `cargo-deny` G9 |
| `cargo clippy --workspace --all-targets`（未実行） | `tauri-app` G10（Linux・macOS） |
| `cargo build -p local-sights`（未実行） | `tauri-app` G11（Linux・macOS） |

Build and Test で実行したコマンドはすべて CI に含まれ、この環境で実行できなかった 3 つ（B-3・B-6・B-9）も CI が担う。速さの計測（`#[ignore]`）と GUI の目視は、方針どおり CI に含めない（`quality-gates.md`）。

## 5. 判定

**合格（指摘 4 件つき）。** 7 単位はすべて作られ、レビューは READY、自動テストはすべて成功、CI は Build and Test のコマンドを強制する。残る指摘は、対応表（`traceability.json`）に NFR7・NFR9・NFR14 の記入がないことと、FR7.8 の `N/A` の扱いで、いずれも機能やコードの欠けではない。追記は、コード生成のステージを閉じたあとに気づいた訂正として記録に残し（project.md Corrections）、次にコードの作業単位を開く機会に行う。

手元（macOS）で残っていること（`construction/build-and-test/build-and-test-summary.md`）：Tauri アプリの起動と README の確認項目、NFR1〜NFR3 の計測、NFR10・NFR13 の目視、Cost Explorer での料金確認、ブランチ保護・secret scanning・Dependabot security updates の設定（`construction/ci-pipeline/ci-config.md`）。

## 6. 承認

- [ ] 人間の承認（CI Pipeline のステージの承認をもって、この確認も承認とする）

## 7. 追記（2026-10-10、承認のあとの後片付け）

§3 と §5 の指摘 4 件を片付けた（`construction/build-and-test/cross-unit-traceability.md` の「追記」）。
- NFR7・NFR9 は U7 の対応表、NFR14 は U1 の対応表に追記した。
- FR7.8 は、理由つきの `N/A` を正とした。

単位をまたいだ FR / NFR のゲートは、これで PASS になった。§5 の判定は「合格（指摘なし）」と読み替える。上の §1〜§6 は、承認したときの記録としてそのまま残す。
