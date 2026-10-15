# Quality Gates — 261012-parallel-fetch

マージ前（PR）の品質ゲート。どれか 1 つでも失敗したら `main` にマージしない（team.md の Way of Working・Code Style・Testing Posture）。

## マージ前のゲートと Build and Test のコマンドの対応

| ゲート | CI の手順（ジョブ） | Build and Test で走らせたコマンド | 基準 |
|--------|---------------------|-----------------------------------|------|
| Rust の書式 | `rustfmt`（rust-core） | `cargo fmt --all --check` | 差分なし |
| Rust のリンタ | `clippy (library)`（rust-core）、`clippy (workspace)`（tauri-app） | `cargo clippy --workspace --all-targets` | deny レベルの lint がない（警告はエラー扱いにしない） |
| ライブラリのテストとカバレッジ | `tests with line coverage`（rust-core） | `cargo llvm-cov -p local-sights-core --fail-under-lines 80`、`cargo test --workspace` | 全件成功、行 80% 以上（今回 97.67%） |
| doc テスト | `doc tests (library)`（rust-core） | `cargo test -p local-sights-core --doc` | 全件成功 |
| 確認用プログラムのテスト | `example tests (library)`（rust-core、**この作業で追加**） | `cargo test -p local-sights-core --example fetch_check` | 全件成功（3 件） |
| 版の一致 | `version consistency`（frontend） | `npm run check:version` | 5 つのファイルの版がそろう |
| 型検査 | `typecheck`（frontend） | `npx tsc --noEmit` | エラーなし |
| 画面のリンタ | `eslint`（frontend） | `npx eslint .` | エラーなし |
| 画面の書式 | `prettier`（frontend） | `npx prettier --check .` | 差分なし |
| 画面のテストとカバレッジ | `vitest with line coverage`（frontend） | `npm run test:coverage` | 全件成功、行 80% 以上（今回 90.85%） |
| 画面のバンドル | `vite build`（frontend） | `npx vite build` | 成功 |
| 画面の依存関係 | `npm audit`（frontend） | `npm audit --audit-level=high` | high 以上が 0 件 |
| Rust の依存関係 | cargo-deny | `cargo deny check` | advisories・bans・licenses・sources がすべて ok |
| アプリのビルドとテスト | `build the app`・`tests (app)`（tauri-app、Ubuntu と macOS） | `cargo build -p local-sights`、`cargo test --workspace` に含まれるアプリのテスト | 成功 |

Build and Test で走らせたコマンドのうち、CI のゲートに入っていないのは次の 2 つだけで、どちらも意図して手元に残している。

- 100 万件の一覧の速度測定（`cargo test -p local-sights-core --release --lib -- --ignored`）：実行機の速さに左右されるため。
- 実際の AWS での速さの比べ：CI から AWS に接続しない決まり（project.md の Forbidden）のため。

## リリースのゲート（変更なし）

`release.yml` は、タグの形と版の一致、タグのコミットが `main` 上にあること、ライブラリと画面のテストが通ることを確かめてから `.dmg` を作って公開する。

## 必須チェックの名前

ジョブ名は変えていない。そのため、ブランチ保護の必須チェックに登録した名前とずれることはない（登録の中身はこのコンテナからは見えないため、確かめていない）。今の CI のジョブ名は次の 5 つ。

- `Rust library (fmt, clippy, test)`
- `Front end (tsc, eslint, prettier, vitest, build)`
- `cargo-deny`
- `Tauri app (ubuntu-latest)`
- `Tauri app (macos-latest)`
