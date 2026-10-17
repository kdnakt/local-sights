# Quality Gates — 261015-windows-support

マージ前（PR）の品質ゲート。どれか 1 つでも失敗したら `main` にマージしない（team.md の Way of Working・Code Style・Testing Posture）。

## マージ前のゲートと Build and Test のコマンドの対応

| ゲート | CI の手順（ジョブ） | Build and Test で走らせたコマンド | 基準 |
|--------|---------------------|-----------------------------------|------|
| Rust の書式 | `rustfmt`（rust-core） | `cargo fmt --all --check` | 差分なし |
| Rust のリンタ | `clippy (library)`（rust-core）、`clippy (workspace)`（tauri-app、3 OS） | `cargo clippy --workspace --all-targets` | deny レベルの lint がない（警告はエラー扱いにしない） |
| ライブラリのテストとカバレッジ | `tests with line coverage`（rust-core、Linux） | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 全件成功、行 80% 以上（今回 97.67%） |
| ライブラリのテスト（macOS・Windows） | `tests (library)`（tauri-app、Linux 以外。**この作業で追加**） | `cargo test --workspace`（Linux で実行。macOS・Windows は CI） | 全件成功 |
| doc テストと確認用プログラムのテスト | `doc tests (library)`、`example tests (library)`（rust-core） | `cargo test --workspace` に含まれる | 全件成功 |
| アプリのビルドとテスト | `build the app`・`tests (app)`（tauri-app、Linux・macOS・**Windows**） | `cargo build -p local-sights`、`cargo test --workspace` | 成功。Windows では `sso_cli` の Windows のテスト 11 件とキャッシュの Windows のテスト 1 件も走る |
| 版の一致 | `version consistency`（frontend） | `npm run check:version` | 版がそろう（0.2.0） |
| 型検査 | `typecheck`（frontend） | `npx tsc --noEmit` | エラーなし |
| 画面のリンタ | `eslint`（frontend） | `npx eslint .` | エラーなし |
| 画面の書式 | `prettier`（frontend） | `npx prettier --check .` | 差分なし |
| 画面のテストとカバレッジ | `vitest with line coverage`（frontend） | `npm run test:coverage` | 全件成功、行 80% 以上（今回 90.86%）。`signing.test.ts`（リリースのワークフローと手順書の検査、31 件）を含む |
| 画面のバンドル | `vite build`（frontend） | `npx vite build` | 成功 |
| 画面の依存関係 | `npm audit`（frontend） | `npm audit --audit-level=high` | high 以上が 0 件 |
| Rust の依存関係 | cargo-deny | `cargo deny check` | advisories・bans・licenses・sources がすべて ok（`windows-sys` を含む） |
| 秘密情報 | GitGuardian（PR のチェック） | — | 成功 |

Build and Test で走らせたもののうち、CI のゲートに入っていないのは次の 2 つだけで、どちらも意図して手元に残している。

- 100 万件の一覧の速度測定（`cargo test -p local-sights-core --release --lib -- --ignored`）：実行機の速さに左右されるため。
- 手元の確認（GUI、実際の AWS、インストーラ）：CI から AWS に接続しない決まり（project.md の Forbidden）と、GUI を目で見る確認のため。

## リリースのゲート

`release.yml` は、次をすべて満たしたときだけ公開する。

1. タグが `vX.Y.Z` の形で、5 つのファイルの版と一致し、タグのコミットが `main` 上にある（`verify`）。
2. ライブラリと画面のテストが通る（`test`）。
3. `.dmg` が 1 つだけでき、中の `.app` がアドホック署名されている（`build-macos`）。
4. `x64-setup.exe` が 1 つだけできる（`build-windows`）。
5. 3 と 4 の両方の成果物がそろっている（`publish`）。

どれかが失敗すれば Release は作られない。公開後に壊れていたら、タグは動かさず次のパッチ版（`v0.2.1`）で直す（`docs/releasing.md`）。

## 必須チェックの名前

今の CI のジョブ名は次の 6 つ。ジョブを足したため、必須チェックもこの 6 つにそろえる（team.md の Way of Working）。

- `Rust library (fmt, clippy, test)`
- `Front end (tsc, eslint, prettier, vitest, build)`
- `cargo-deny`
- `Tauri app (ubuntu-latest)`
- `Tauri app (macos-latest)`
- `Tauri app (windows-latest)` ← **この作業で追加**

利用者が GitHub の設定画面からルールセットに加えた（Q1 の B、2026-10-17T02:06:28Z）。API で読み直し、上の 6 つが必須になっていることを確かめた。
