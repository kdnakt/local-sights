# Phase Check — Construction → Operation

**判定：PASS（利用者の判断による受け入れあり）**。Code Generation と Build and Test を終え、CI のゲートは Build and Test のコマンドを確かめる。要件の対応表は 35 件すべて OK（FR1.3 はルールセットの変更で満たされた）。品質目標 16 件のうち 1 件（NFR5.b、日本語や空白の利用者フォルダ名）は未確認で、利用者が受け入れた（`construction/build-and-test/build-and-test-questions.md` Q4 の B）。

## 1. ビルドとテスト

| 項目 | 結果 |
|------|------|
| 作業単位 | なし（ステージ単位）。Code Generation は Part A（PR #23）と Part B（PR #24）で完了し、承認済み |
| ビルド | 成功。Linux はこのコンテナで、macOS と Windows は CI（`Tauri app` の 3 OS）と Release dry run で確認 |
| テスト | Rust 549 件成功・失敗 0 件（Linux）、3 OS の CI で成功、画面 392 件成功、`signing.test.ts` 31 件成功 |
| カバレッジ | ライブラリ 97.67%、画面 90.86%（下限 80%） |
| 品質目標 | 16 件のうち Met 15 件、Unverified 1 件（NFR5.b、利用者が受け入れ）。`construction/build-and-test/build-and-test-summary.md` |
| 手元の確認 | Windows 11（起動・選択・取得・表示・SSO ログイン）と macOS（前と同じに動く）で合格。ナレーターは利用者の判断で行わず、画面のテストで判定 |

## 2. code-generation の対応表

| 対象 | `traceability.json` | 状態が OK でない行 | 対象のファイル |
|------|---------------------|--------------------|----------------|
| ステージ単位（`construction/code-generation/`） | あり | FR1.3 の `Deferred` 1 件（下の 3 で解消） | すべてある |

コード生成のレビュー（review-01、READY・advisory）の指摘は、Major 1（必須チェック）が下の 4 で解消。Minor 3 件（`publish` はタグまで動かない、ジョブ オブジェクトへの割り当ての一瞬の隙、計画のチェックの更新）は承認時に示し、受け入れ済み。

## 3. 要件の対応表

| 項目 | 結果 |
|------|------|
| 数え上げた ID | 35（requirements.md の FR の細目 29 件と NFR 6 件。user-stories は実行していない） |
| OK | 34（Build and Test の時点） |
| Deferred | 1：FR1.3（`main` のルールセットの必須チェックに `Tauri app (windows-latest)` を加える） |
| FR1.3 の解消 | 利用者が GitHub の設定画面で加えた（2026-10-17T02:06:28Z）。API で読み直し、必須チェックが 6 つになったことを確かめた。これで 35 件すべて満たされた |
| 判定 | PASS（`construction/build-and-test/cross-unit-traceability.md`） |

## 4. CI のゲートと Build and Test のコマンド

| 項目 | 結果 |
|------|------|
| Build and Test のコマンドが CI のゲートにあるか | ある（`construction/ci-pipeline/quality-gates.md` の対応表）。手元に残すのは速度測定と GUI・実際の AWS・インストーラの確認だけ |
| 必須チェック | `Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (ubuntu-latest)`、`Tauri app (macos-latest)`、`Tauri app (windows-latest)` の 6 つ（ルールセット id 24831821） |
| リリースのゲート | `release.yml` の 5 ジョブ。両 OS の成果物がそろったときだけ公開 |
| AWS への接続 | CI から接続しない。AWS の認証情報は置いていない |

## 5. 残っている項目（Operation の外）

- `v0.2.0` のリリース：`main` で Release dry run を動かし、両 OS で入れて確かめてからタグを付ける（`docs/releasing.md`）。この作業の範囲外で、利用者が行う。
- NFR5.b（日本語や空白の利用者フォルダ名）は未確認。該当する利用者から報告があれば次のパッチ版で直す。
- 時間依存の既存テスト `u5_cancel::a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss`（PR #22 由来）の安定化。

## 承認

- [ ] Construction → Operation の移行を承認する（Operation のステージはこのスコープでは実行しない）
