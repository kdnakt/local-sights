# Quality Gates — マージ前の品質ゲート

上流：`memory/team.md`（Code Style・Testing Posture・Way of Working）、`construction/build-and-test/test-results.md` の Target Verification Matrix、`build-and-test-summary.md`。すべてのゲートは `.github/workflows/ci.yml` のジョブとして PR で走り、1 つでも落ちれば `main` にマージしない（ブランチ保護の必須チェック）。

## ゲートの一覧

| # | ゲート | 判定 | ジョブ | 根拠 |
|---|--------|------|--------|------|
| G1 | Rust の整形 | `cargo fmt --all --check` の差分なし | `rust-core` | team.md Code Style（必須） |
| G2 | Rust の静的検査（ライブラリ） | `cargo clippy -p local-sights-core --all-targets` がエラーなし。警告は失敗にしない | `rust-core` | team.md Code Style（`-D warnings` は付けない。deny レベルの lint はブロック） |
| G3 | ライブラリのテスト | `cargo test -p local-sights-core` がすべて成功（単体 + 結合） | `rust-core` | team.md Testing Posture（既存のテストはすべて通った状態を保つ）、TC-3 |
| G4 | 画面の型検査 | `npx tsc --noEmit` がエラーなし | `frontend` | units-generation F2 |
| G5 | 画面の静的検査と整形 | `npx eslint .` と `npx prettier --check .` が問題なし | `frontend` | units-generation F2、team.md Code Style |
| G6 | 画面のテスト | `npx vitest run` がすべて成功 | `frontend` | Testing Posture、TC-3 |
| G7 | 画面のバンドル | `npx vite build` が成功 | `frontend` | B-2 |
| G8 | npm の依存関係 | `npm audit --audit-level=high` で high 以上の脆弱性なし | `frontend` | team.md Code Style（脆弱性の検査） |
| G9 | cargo の依存関係 | `cargo deny check` が成功（脆弱性 0、許可外のライセンス 0、git・未知のレジストリ 0。重複は警告） | `cargo-deny` | team.md Code Style、Q3 |
| G10 | Rust の静的検査（ワークスペース） | `cargo clippy --workspace --all-targets` がエラーなし（Linux・macOS） | `tauri-app` | team.md Code Style、B-6 |
| G11 | Tauri アプリの組み立て | `cargo build -p local-sights` が Linux と macOS で成功 | `tauri-app` | 各単位の `unit-test-instructions.md`、B-3 |

## ゲートにしないもの（理由）

| 項目 | 扱い | 理由 |
|------|------|------|
| コードカバレッジの下限 | 設けない | team.md Testing Posture（数値の下限なし。ライブラリの公開関数にテストがあることをレビューで見る） |
| 速さの計測（`#[ignore]` のテスト、NFR1〜NFR3） | CI では走らせない | 計測機は開発者の Mac。CI のランナーの値は要件の判定に使えない（`performance-test-instructions.md`） |
| 実際の AWS に対する確認 | CI では行わない | project.md Forbidden（テストと CI から AWS に接続しない、CI に認証情報を置かない） |
| GUI の目視（NFR9〜NFR11・NFR13） | 手元で行う | 自動化しない（`build-and-test-summary.md` の残っていること） |
| clippy の警告 | 失敗にしない | team.md Code Style |
| `npm audit` の low・moderate | 失敗にしない | 偶発的な low の勧告で PR を止めないため。high・critical は止める。Dependabot の更新 PR で解消する |

## ゲートが落ちたときの扱い

- 落ちたゲートは直してから push する。ゲートを緩める・外す・テストを skip にすることで通さない（org.md Testing Posture：品質目標は弱めない。例外は人間が判断し、記録を残す）。
- `cargo deny check` の脆弱性は、まず依存の更新で直す。直せない（修正版がない）ときだけ `deny.toml` の `ignore` に ID と理由を書き、PR で説明する。
- ライセンスの許可リストに入っていないライセンスの依存を足すときは、`deny.toml` の `allow` を広げる前に、その依存が必要かを PR で説明する。
- `tauri-app` の Linux 側だけが落ちた場合（WebKitGTK のパッケージ名の変更など）は、ランナーの環境の問題として直す。macOS 側が落ちた場合は MVP の対象 OS の問題として最優先で直す。

## マージの条件（team.md Way of Working）

1. 上の G1〜G11 がすべて緑。
2. セルフレビュー済み（開発者は 1 人）。
3. スクワッシュマージ（`main` の 1 コミット = 1 Bolt / PR）。
4. ドキュメントだけの変更も同じ流れ（CI は走るが、Rust と画面のジョブはすぐ通る）。

## Build and Test の未確認項目との対応

Build and Test でこの環境では確かめられなかった B-3（Tauri アプリの組み立て）・B-6（ワークスペース全体の clippy）・B-9（`cargo-deny`）は、この CI の G9〜G11 で PR ごとに確かめる。NFR1〜NFR3・NFR9・NFR10・NFR13・NFR15 は CI では確かめられず、手元の macOS で行う（`build-and-test-summary.md`）。
