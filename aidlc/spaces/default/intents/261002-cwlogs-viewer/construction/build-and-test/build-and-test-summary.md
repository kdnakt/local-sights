# Build and Test Summary — ビルドとテストのまとめ（2026-10-10）

上流：U1〜U7 の `code-generation/code-generation-plan.md`（Testing Contract）・`unit-test-instructions.md`・`code-summary.md`・`traceability.json`、`inception/requirements-analysis/requirements.md`（NFR 表）、`memory/team.md`（Testing Posture・Code Style）。質問票：`build-and-test-questions.md`（Q1：手元でしか確かめられない項目は手順書だけ書き未確認で記録、Q2：速さのテストはこの環境で参考値として実行）。

## ビルドの状態と前提

- ライブラリ（`local-sights-core`）と画面（Vite + React）は、この環境でビルドでき、整形・静的検査・型検査・`npm audit` も通る。
- Tauri アプリ（`cargo build -p local-sights`）は、この環境に GDK 3・WebKitGTK がないため組み立てられない。コードの問題ではなく、macOS（MVP の対象）で組み立てる。手順は `build-instructions.md`。
- 前提：Rust 安定版 + `rustfmt`・`clippy`、Node.js 20 以上、macOS では Xcode Command Line Tools。AWS の認証情報は自動テストに要らない。

## テストの種類の棚卸し

| 種類 | 手順書 | この環境での実行 |
|------|--------|------------------|
| 単体テスト（ライブラリ・画面） | 各単位の `unit-test-instructions.md` | 実行。ライブラリ 305 件・画面 171 件成功 |
| 統合テスト | `integration-test-instructions.md` | 実行。ライブラリの結合テスト 41 件成功、`App.test.tsx` 成功 |
| 性能 | `performance-test-instructions.md` | `#[ignore]` の速さのテスト 5 件を release で実行（参考値）。GUI を通した計測は手元 |
| セキュリティ | `security-test-instructions.md` | 静的な確認とテストを実行。`cargo deny` は CI Pipeline のステージで整備 |
| ビルド | `build-instructions.md` | ライブラリ・画面は成功。Tauri アプリは手元 |

## 単位ごとのテストの範囲

数値のカバレッジ下限は設けない（team.md）。代わりに、各単位の手順書の部品別の件数の目安に対して実際の件数が足りているかを見た（各単位の `code-summary.md` の表）。

| 単位 | ライブラリ側（単位の絞り込みコマンド） | 画面側 | 結合テスト | 期待との差 |
|------|------|------|------|------|
| U1 薄い一本 | 162 件 | 5 ファイル 64 件 | `u1_fetch_flow` 11 件 | 目安以上 |
| U2 接続とロググループ | 132 件 | 6 ファイル 72 件 | `u2_log_group_listing` 9 件 | 目安以上 |
| U3 取得の作り込み | 162 件 | 7 ファイル 89 件 | `u3_fetch_flow` 14 件 | 目安以上 |
| U4 時間範囲とタイムゾーン | 124 件 | 5 ファイル 69 件 | （単体テストで境界を覆う） | 目安以上 |
| U5 絞り込み | 104 件 | 4 ファイル 56 件 | `u5_filter_flow` 4 件 | 目安以上 |
| U6 ディスクキャッシュ | 135 件 | 6 ファイル 69 件 | `u6_cache_flow` 3 件 | 目安以上 |
| U7 画面の仕上げ | 145 件 | 14 ファイル 123 件 | （`App.test.tsx` で画面全体） | 目安以上 |

モジュールが単位をまたいで重なるため、ライブラリ側の件数は合計すると 305 件を超える。

## Target Verification Matrix

確定版は `test-results.md` の同名の節にある（31 目標：Met 21、Unverified 10、Not Met 0）。要約：

| 区分 | Met | Unverified | 内容 |
|------|-----|------------|------|
| Testing Contract（TC-1〜TC-5） | 5 | 0 | 件数・結合テスト・既存のテストが通る・公開関数のテスト・AWS に接続しない |
| ビルドと検査（B-1〜B-9） | 6 | 3 | 未確認：Tauri アプリの組み立て（B-3）、ワークスペース全体の clippy（B-6）、`cargo deny`（B-9） |
| 要件の NFR（NFR1〜NFR16） | 10 | 6 | 未確認：NFR1〜NFR3（Mac での計測）、NFR9（macOS で動く）、NFR10（見た目）、NFR13（流れ全体のキーボード操作）、NFR15（Cost Explorer） |

`Not Met` はない。`Unverified` はすべて「この環境で確かめられない」もので、手元の macOS での実行と、次のステージでの道具の整備で解消する。

## 要件 ID の対応（トレーサビリティ）

`cross-unit-traceability.md`：70 件のうち 66 件が `OK` で対応先のファイルも存在。対応のない ID が 4 件（FR7.8 は選択肢で `N/A`、NFR7・NFR14 はこのステージで満たしていることを確かめた、NFR9 は手元で確かめる）。対応表への追記は次に対応表を触る機会に回す。

## 準備の状態

| 観点 | 状態 | 根拠 |
|------|------|------|
| build-ready（ビルドできる） | **ライブラリと画面は可。アプリは手元で確認待ち** | B-1・B-2・B-4・B-5・B-7 は Met、B-3・B-6 は Unverified |
| test-ready（テストできる） | **可** | 全自動テスト 517 件成功、失敗 0 |
| deployment-ready（配布できる） | **手元の確認待ち** | 配布はソースからのビルドだけ（team.md）。macOS での組み立て・起動・NFR の目視が済んでいない |

このステージの判定：**失敗（Unverified が 10 件）**。失敗の原因は生成したコードではなく確認環境のため、コード生成へ戻す候補はない。扱い（失敗を受け入れて承認に進む、または中断）は人間が決める。

## 既知の制約と残っていること

1. macOS で行うこと：`cargo build -p local-sights`、`cargo clippy --workspace --all-targets`、`npm run tauri dev` で起動して README の確認項目、NFR1〜NFR3 の計測（`performance-test-instructions.md`）、NFR10・NFR13 の目視、認証失敗時のエラー表示に秘密情報が出ないこと。
2. CI Pipeline のステージで行うこと：`cargo-deny` と `deny.toml` の整備、GitHub の secret scanning・push protection の有効化、`cargo fmt --check`・`clippy`・`cargo test`・`tsc`・`eslint`・`prettier`・`vitest` を PR で必須にする。CI に AWS の認証情報を置かない。
3. 実際の AWS での初回実行後に Cost Explorer で転送料金を確認する（NFR15）。
4. Dock からの終了は確認なしで終わる（U7 の既知の制約）。
5. 対応表に NFR7・NFR9・NFR14 を追記し、FR7.8 の `N/A` の扱いを決める（次に対応表を触る機会）。
   - 2026-10-10 に済み：U1・U7 の `traceability.json` に追記し、FR7.8 は理由つきの `N/A` を正とした（`cross-unit-traceability.md` の「追記」）。

## 前の単位の記録の訂正（U6 のコード生成レビューのあとに気づいたもの）

project.md の Corrections（レビューを記録したあとに気づいた修正は次の作業単位に回す）に従い、U6 の記録は直さずここに記す。

1. `construction/u6-disk-cache/code-generation/code-summary.md` の「まだ確かめていないこと」にある、ヘッダに件数（`eventCount`）がないこと、ロックを持ったままファイル I/O をすること、の 2 件は U7 で解消済み（書式の版 2 の `eventCount`・`rangeEventCounts`、`spawn_blocking` での写し取り。U7 の `code-summary.md` の U6 R-03・R-04）。U6 の記録はその時点の記述として残っている。
2. 同 `code-summary.md` の `cache/plan.rs` のテスト件数は 14 件と書かれているが、現在は 15 件（`#[test]` の数。U7 で 1 件足した）。
3. 同 `code-summary.md` の「最初のビルドの記録」は U6 当時の記述で、`WriteTracker`・版 2 の書式など U7 で変わった部分には触れていない。現在の姿は U7 の `code-summary.md` が正。
